//! Lire et remplacer les groupes suivis — en bloc, idempotent, sous condition
//! de fraîcheur : le patron de `service/themes.rs`, liste vide permise.

use kernel::context::RequestContext;
use kernel::empreinte;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::groups::MyGroups;
use crate::repo::groups;
use crate::state::NegotiationState;

pub async fn mes_groupes(state: &NegotiationState, person_id: Uuid) -> Result<MyGroups> {
    let mut conn = state.pool().acquire().await?;
    Ok(MyGroups::de(groups::suivis(&mut conn, person_id).await?))
}

pub struct Remplacement<'a> {
    pub person_id: Uuid,
    pub codes: &'a [String],
    pub si_correspond: Option<&'a str>,
}

pub async fn remplacer(
    state: &NegotiationState,
    ctx: &RequestContext,
    r: Remplacement<'_>,
) -> Result<MyGroups> {
    let mut codes: Vec<String> = r.codes.iter().map(|c| c.trim().to_owned()).collect();
    let mut vus = std::collections::HashSet::new();
    codes.retain(|c| !c.is_empty() && vus.insert(c.clone()));

    let mut tx = state.db().write(ctx).await?;
    groups::verrouiller(&mut tx, r.person_id).await?;

    let avant = MyGroups::de(groups::suivis(&mut tx, r.person_id).await?);
    if let Some(attendue) = r.si_correspond {
        if !empreinte::correspond(attendue, &avant.etag) {
            tx.rollback().await?;
            return Err(ApiError::new(ErrorCode::NegotiationGroupsStale));
        }
    }

    let termes = groups::termes(&mut tx, &codes).await?;
    if let Some(inconnu) = codes.iter().find(|c| !termes.iter().any(|t| &t.code == *c)) {
        tx.rollback().await?;
        return Err(ApiError::with_message(
            ErrorCode::NegotiationGroupUnknown,
            format!("Le groupe « {inconnu} » n'existe pas."),
        )
        .field("groups"));
    }

    // Un terme retiré reste à qui le suivait ; il ne se choisit plus.
    if let Some(retire) = termes
        .iter()
        .find(|t| !t.is_active && !avant.groups.contains(&t.code))
    {
        tx.rollback().await?;
        return Err(ApiError::with_message(
            ErrorCode::NegotiationGroupUnknown,
            format!("Le groupe « {} » n'est plus proposé.", retire.code),
        )
        .field("groups"));
    }

    let ids: Vec<Uuid> = termes.iter().map(|t| t.id).collect();
    groups::fermer_les_autres(&mut tx, r.person_id, &ids).await?;
    groups::ouvrir_les_nouveaux(&mut tx, r.person_id, &ids).await?;

    let apres = groups::suivis(&mut tx, r.person_id).await?;
    tx.commit().await?;
    Ok(MyGroups::de(apres))
}
