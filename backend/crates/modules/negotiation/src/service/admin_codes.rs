//! Le back-office des codes : créer, lister, révoquer, retirer.
//!
//! # CE QUE CE FICHIER NE FAIT PAS
//!
//! Il **ne teste aucune permission** : la garde est l'extracteur de route,
//! `Requires<SpaceManage>`, qui exige la portée globale. La poser ici en plus
//! donnerait deux points de décision, et le jour où l'un changerait, l'autre
//! mentirait.
//!
//! Il **ne vérifie pas non plus** qu'un espace existe, qu'un quota est
//! atteignable ou qu'une période est cohérente : `ck_invitation_codes_*` et la
//! clé étrangère de l'espace s'en chargent, et l'API traduit leur refus
//! (principe VIII).
//!
//! # RÉVOQUER N'EST PAS RETIRER
//!
//! Deux gestes, deux routes, deux écrans (ADR-006). Une fuite de code se
//! referme en le révoquant — le réseau déjà entré garde son accès —, et un abus
//! se sanctionne en retirant un accès — le code du groupe reste valable. Les
//! confondre ferait de chaque fuite une exclusion collective.

use contracts::negotiation as evenements;
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self, DomainEvent};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::admin::{
    CreateInvitationCodePayload, InvitationCodeDetail, InvitationCodeListScreen, InvitationCodeRow,
    InvitationCodeUsesScreen, RevokeAllAccessResult,
};
use crate::domain::code;
use crate::repo::codes::{self, Filtre, NouveauCode};
use crate::repo::uses::{self, AccesRetire};
use crate::state::NegotiationState;

/// Combien de tirages avant d'abandonner.
///
/// L'alphabet retenu donne vingt-sept milliards de codes de sept caractères :
/// cinq collisions d'affilée ne signalent pas un manque de place mais une panne
/// de l'aléa, et il vaut mieux le dire que boucler.
const TIRAGES: u8 = 5;

pub async fn liste(
    state: &NegotiationState,
    filtre: &Filtre<'_>,
    locale: &str,
) -> Result<InvitationCodeListScreen> {
    let mut conn = state.pool().acquire().await?;

    let (rows, total) = codes::lister(&mut conn, filtre, locale).await?;
    let spaces = codes::espaces(&mut conn, locale).await?;
    let networks = codes::reseaux(&mut conn, locale).await?;

    Ok(InvitationCodeListScreen {
        rows,
        total,
        spaces,
        networks,
    })
}

/// La fiche d'un code. `None` : il n'existe pas.
pub async fn fiche(
    state: &NegotiationState,
    code_id: Uuid,
    locale: &str,
) -> Result<Option<InvitationCodeDetail>> {
    let mut conn = state.pool().acquire().await?;

    let Some(code) = codes::fiche(&mut conn, code_id, locale).await? else {
        return Ok(None);
    };
    let granted_uses = uses::acces_ouverts(&mut conn, code_id).await?;

    Ok(Some(InvitationCodeDetail { code, granted_uses }))
}

pub async fn usages(
    state: &NegotiationState,
    code_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<InvitationCodeUsesScreen> {
    let mut conn = state.pool().acquire().await?;
    let (rows, total, granted_uses) = uses::du_code(&mut conn, code_id, limit, offset).await?;

    Ok(InvitationCodeUsesScreen {
        rows,
        total,
        granted_uses,
    })
}

/// Crée un code. Il est **engendré**, jamais choisi : laisser un administrateur
/// l'écrire produirait des codes devinables — « COP31 », « IFDD2026 » — sur une
/// porte que rien d'autre ne protège.
pub async fn creer(
    state: &NegotiationState,
    ctx: &RequestContext,
    acteur: Uuid,
    charge: &CreateInvitationCodePayload,
    locale: &str,
) -> Result<InvitationCodeRow> {
    let libelle = charge.label.trim();
    if libelle.is_empty() {
        return Err(ApiError::validation(
            "Donnez un libellé au code : c'est ce qui permettra de le retrouver.",
            "label",
        ));
    }

    let (scope_type, space_id) = charge.scope.as_db();

    let mut tx = state.db().write(ctx).await?;

    let network_term_id =
        match charge.grants_network.as_deref() {
            Some(reseau) => Some(codes::terme_de_reseau(&mut tx, reseau).await?.ok_or_else(
                || {
                    ApiError::validation(
                        "Ce réseau n'existe pas dans le vocabulaire des réseaux de négociation.",
                        "grants_network",
                    )
                },
            )?),
            None => None,
        };

    let mut engendre = None;
    for _ in 0..TIRAGES {
        let candidat = code::engendrer();
        let insere = codes::creer(
            &mut tx,
            NouveauCode {
                code: &candidat,
                label: libelle,
                scope_type,
                space_id,
                grants_network_term_id: network_term_id,
                max_uses: charge.max_uses,
                valid_from: charge.valid_from,
                valid_until: charge.valid_until,
                created_by: acteur,
            },
        )
        .await?;

        if let Some(id) = insere {
            engendre = Some(id);
            break;
        }
    }

    let code_id = engendre.ok_or_else(|| {
        ApiError::internal("aucun code libre après cinq tirages : l'aléa est en panne")
    })?;

    let fiche = codes::fiche(&mut tx, code_id, locale)
        .await?
        .ok_or_else(|| ApiError::internal("code introuvable juste après avoir été créé"))?;

    tx.commit().await?;

    Ok(fiche)
}

/// Révoque le code. **Ne retire aucun accès déjà accordé** (ADR-006, FR-039) :
/// il cesse seulement d'ouvrir, dès la tentative suivante.
///
/// `None` : le code n'existe pas. Déjà révoqué, la fiche revient telle quelle —
/// ni seconde date, ni second auteur.
pub async fn revoquer(
    state: &NegotiationState,
    ctx: &RequestContext,
    acteur: Uuid,
    code_id: Uuid,
    motif: Option<&str>,
    locale: &str,
) -> Result<Option<InvitationCodeRow>> {
    let mut tx = state.db().write(ctx).await?;

    if let Some(revocation) = codes::revoquer(&mut tx, code_id, acteur, motif).await? {
        let charge = serde_json::to_value(evenements::InvitationCodeRevoked {
            invitation_code_id: code_id,
            scope_type: revocation.scope_type,
            space_id: revocation.space_id,
            reason: motif.map(str::to_owned),
            granted_uses: revocation.granted_uses,
        })
        .map_err(ApiError::internal)?;

        events::emit(
            &mut tx,
            DomainEvent {
                aggregate_schema: evenements::AGGREGATE_SCHEMA,
                aggregate_type: evenements::AGGREGATE_INVITATION_CODE,
                aggregate_id: code_id,
                event_type: evenements::INVITATION_CODE_REVOKED,
                payload: charge,
            },
        )
        .await?;
    }

    let fiche = codes::fiche(&mut tx, code_id, locale).await?;
    tx.commit().await?;

    Ok(fiche)
}

/// Retire l'accès d'une personne entrée par ce code.
///
/// `false` : elle n'avait pas d'accès en cours sur cette portée. L'écran le dit
/// sans crier à l'erreur — deux administrateurs peuvent cliquer à la seconde
/// près.
pub async fn retirer_un_acces(
    state: &NegotiationState,
    ctx: &RequestContext,
    acteur: Uuid,
    code_id: Uuid,
    person_id: Uuid,
    motif: Option<&str>,
) -> Result<bool> {
    let mut tx = state.db().write(ctx).await?;

    let retire = uses::retirer(&mut tx, code_id, person_id, acteur, motif).await?;
    if let Some(retire) = &retire {
        emettre_le_retrait(&mut tx, retire, evenements::RevocationCause::Removed, motif).await?;
    }

    tx.commit().await?;

    Ok(retire.is_some())
}

/// Retire en un geste les accès de **toutes** les personnes entrées par ce
/// code. La cause portée par l'événement les distingue d'un retrait
/// individuel : celui-ci suit un code compromis.
pub async fn retirer_tous_les_acces(
    state: &NegotiationState,
    ctx: &RequestContext,
    acteur: Uuid,
    code_id: Uuid,
    motif: Option<&str>,
) -> Result<RevokeAllAccessResult> {
    let mut tx = state.db().write(ctx).await?;

    let retires = uses::retirer_tous(&mut tx, code_id, acteur, motif).await?;
    for retire in &retires {
        emettre_le_retrait(
            &mut tx,
            retire,
            evenements::RevocationCause::CodeCompromised,
            motif,
        )
        .await?;
    }

    tx.commit().await?;

    Ok(RevokeAllAccessResult {
        revoked: retires.len() as i64,
    })
}

/// L'événement part **dans la transaction du retrait** : un accès retiré dont
/// l'annonce ne partirait pas laisserait les contenus réservés en place sur le
/// téléphone, ce qu'ADR-006 interdit.
async fn emettre_le_retrait(
    conn: &mut PgConnection,
    retire: &AccesRetire,
    cause: evenements::RevocationCause,
    motif: Option<&str>,
) -> Result<()> {
    let charge = serde_json::to_value(evenements::SpaceAccessRevoked {
        person_id: retire.person_id,
        scope_type: retire.scope_type.clone(),
        space_id: retire.space_id,
        cause,
        reason: motif.map(str::to_owned),
    })
    .map_err(ApiError::internal)?;

    events::emit(
        conn,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_SPACE_ACCESS,
            aggregate_id: retire.role_assignment_id,
            event_type: evenements::SPACE_ACCESS_REVOKED,
            payload: charge,
        },
    )
    .await?;

    Ok(())
}
