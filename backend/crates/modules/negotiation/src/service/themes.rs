//! Lire et remplacer les thématiques suivies — **en bloc, donc idempotent, et
//! sous condition de fraîcheur**.
//!
//! # L'IDEMPOTENCE PROTÈGE DU REJEU, `If-Match` DE L'ANCIENNETÉ
//!
//! Rejouer le même corps donne le même état et n'écrit rien de plus : c'est ce
//! qui permet à une intention prise hors connexion de repartir sans crainte.
//! Mais un téléphone de retour à midi ne doit pas effacer ce que la tablette a
//! choisi à onze heures : l'empreinte reçue est comparée **dans la
//! transaction**, après le verrou, et un écart sort en `412` sans rien écrire.
//!
//! # LE VOCABULAIRE EST GARDÉ PAR LA BASE
//!
//! `tg_theme_subscriptions_check_theme` refuse tout terme d'un autre
//! vocabulaire. Le service n'en réimplémente rien : il résout les codes sous
//! `negotiation_theme` pour pouvoir **nommer le code refusé**, ce que le
//! trigger — qui ne connaît qu'un identifiant — ne peut pas dire.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use uuid::Uuid;

use crate::domain::themes::{empreinte_des_codes, MyThemes};
use crate::repo::themes;
use crate::state::NegotiationState;
use kernel::empreinte;

pub async fn mes_thematiques(state: &NegotiationState, person_id: Uuid) -> Result<MyThemes> {
    let mut conn = state.pool().acquire().await?;
    let suivis = themes::suivis(&mut conn, person_id).await?;
    Ok(MyThemes { themes: suivis })
}

pub struct Remplacement<'a> {
    pub person_id: Uuid,
    /// La liste entière ; les doublons sont réduits ici.
    pub codes: &'a [String],
    /// L'empreinte de l'état sur lequel le choix a été pris. Absente, l'écran
    /// en ligne vient de lire et n'a rien à opposer.
    pub si_correspond: Option<&'a str>,
}

pub async fn remplacer(
    state: &NegotiationState,
    ctx: &RequestContext,
    r: Remplacement<'_>,
) -> Result<MyThemes> {
    let mut codes: Vec<String> = r.codes.iter().map(|c| c.trim().to_owned()).collect();
    let mut vus = std::collections::HashSet::new();
    codes.retain(|c| !c.is_empty() && vus.insert(c.clone()));

    if codes.is_empty() {
        return Err(ApiError::new(ErrorCode::NegotiationThemesEmpty).field("codes"));
    }

    let mut tx = state.db().write(ctx).await?;
    themes::verrouiller(&mut tx, r.person_id).await?;

    let avant = themes::suivis(&mut tx, r.person_id).await?;
    if let Some(attendue) = r.si_correspond {
        let courante = empreinte_des_codes(avant.iter().map(|t| t.code.as_str()));
        if !empreinte::correspond(attendue, &courante) {
            tx.rollback().await?;
            return Err(ApiError::new(ErrorCode::NegotiationThemesStale));
        }
    }

    let termes = themes::termes(&mut tx, &codes).await?;
    if let Some(inconnu) = codes.iter().find(|c| !termes.iter().any(|t| &t.code == *c)) {
        tx.rollback().await?;
        return Err(ApiError::with_message(
            ErrorCode::NegotiationThemeUnknown,
            format!("La thématique « {inconnu} » n'existe pas."),
        )
        .field("codes"));
    }

    // Un terme retiré du vocabulaire reste à qui le suivait ; il ne se choisit
    // plus. On ne retire pas à quelqu'un ce qu'il suivait, on ne laisse pas en
    // choisir un qui n'est plus proposé.
    if let Some(retire) = termes
        .iter()
        .find(|t| !t.is_active && !avant.iter().any(|a| a.code == t.code))
    {
        tx.rollback().await?;
        return Err(ApiError::with_message(
            ErrorCode::NegotiationThemeUnknown,
            format!("La thématique « {} » n'est plus proposée.", retire.code),
        )
        .field("codes"));
    }

    let ids: Vec<Uuid> = termes.iter().map(|t| t.id).collect();
    themes::fermer_les_autres(&mut tx, r.person_id, &ids).await?;
    themes::ouvrir_les_nouveaux(&mut tx, r.person_id, &ids).await?;

    let apres = themes::suivis(&mut tx, r.person_id).await?;
    tx.commit().await?;

    Ok(MyThemes { themes: apres })
}
