//! Les échanges sur un dossier — **et le champ le plus dangereux de l'écran**.
//!
//! # Trois visibilités, dont une qui sort du comité
//!
//! `committee` reste entre membres du comité, `submitter` **part chez le
//! déposant**, `private` n'est lue que de son auteur. Se tromper est
//! irrattrapable : un message lu ne se retire pas.
//!
//! # Une demande de correction est FORCÉE en visibilité partagée (écart n° 99)
//!
//! Les deux colonnes sont indépendantes en base : rien n'empêche d'écrire une
//! demande de correction en visibilité « comité ». Elle bloquerait alors le
//! dossier **sans que le déposant sache pourquoi** — il verrait son dossier
//! passer en « corrections demandées » et n'aurait aucun message à lire.
//!
//! Le service ne refuse pas : il **corrige**. Refuser obligerait l'écran à
//! connaître la règle pour ne pas l'enfreindre ; corriger la tient à un seul
//! endroit.
//!
//! # Une réponse du déposant est TOUJOURS partagée, et jamais une demande
//!
//! L'organisation écrit dans un fil qui lui est adressé : sa réponse est par
//! construction destinée au comité, et une organisation ne se demande pas des
//! corrections à elle-même.
//!
//! # Ce qui est annoncé, et ce qui ne l'est pas
//!
//! `programme.comment.shared` part **sur un message partagé, et sur lui seul**.
//! Un message de comité ne sort pas du comité, par définition ; une note
//! personnelle encore moins. Émettre sur les trois enverrait au déposant l'avis
//! d'une délibération qu'il ne peut pas lire.
//!
//! Depuis le 27/10, un fil du comité **adressé à des personnes nommées** émet
//! aussi `programme.comment.addressed`, vers son cercle seulement : voir
//! `comment_circle.rs`. Un message à tout le comité reste muet.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self, DomainEvent};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::{CommentId, ProposalId};
use crate::repo::comments::{Cote, Message, NouveauMessage};
use crate::repo::{comments, proposals};
use crate::service::comment_circle::{self as circle, ProposalCommentView};
use crate::service::perimeter::{self, Acces};
use crate::state::ProgrammeState;

/// Les trois visibilités, telles que l'énumération de la base les nomme.
pub const VISIBILITE_COMITE: &str = "committee";
pub const VISIBILITE_DEPOSANT: &str = "submitter";
pub const VISIBILITE_PRIVEE: &str = "private";

/// `PostCommentPayload` — et `ReplyToCommentPayload`, qui n'en est qu'une forme
/// réduite : le contrat du front sert **deux appelants** sur la même route.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PostCommentPayload {
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    /// Ignorée quand l'auteur écrit **au nom de l'organisation** : sa réponse
    /// est toujours partagée.
    #[serde(default)]
    pub visibility: Option<String>,
    pub body: String,
    #[serde(default)]
    pub is_change_request: bool,
    /// Personnes nommées : seulement sur un message du comité qui ouvre un fil.
    /// Le fil n'est alors lu que d'elles et de son auteur.
    #[serde(default)]
    pub recipient_ids: Vec<Uuid>,
}

/// Écrire un message sur un dossier.
pub async fn ecrire(
    state: &ProgrammeState,
    ctx: &RequestContext,
    auteur: Uuid,
    dossier: ProposalId,
    payload: PostCommentPayload,
) -> Result<ProposalCommentView> {
    let (edition, acces) = perimeter::acces_au_dossier(state.pool(), auteur, dossier).await?;
    let cote = cote_de(acces);

    let corps = payload.body.trim();
    if corps.is_empty() {
        return Err(ApiError::validation(
            "Un message ne peut pas être vide.",
            "body",
        ));
    }
    let (mut visibilite, demande_de_correction) = arbitrer(acces, &payload);

    let destinataires = if payload.recipient_ids.is_empty() {
        Vec::new()
    } else {
        if acces != Acces::BackOffice
            || payload.parent_id.is_some()
            || visibilite != VISIBILITE_COMITE
        {
            return Err(ApiError::validation(
                "Seul un message du comité qui ouvre un fil peut être adressé à des personnes nommées.",
                "recipient_ids",
            ));
        }
        circle::choisir_dans_le_comite(state, dossier, edition, &payload.recipient_ids, &[auteur])
            .await?
    };

    // Le fil adressé dans lequel on répond, s'il l'est : (racine, son auteur,
    // ses destinataires).
    let mut fil_adresse = None;
    if let Some(parent) = payload.parent_id {
        exiger_le_meme_dossier(state, dossier, CommentId(parent)).await?;
        if !comments::lisible(state.pool(), parent, auteur, cote).await? {
            return Err(ApiError::not_found());
        }
        if cote == Cote::Comite {
            fil_adresse = circle::fil_adresse(state, parent).await?;
        }
    }

    // Les réponses restent dans le cercle : une réponse « partagée » sortirait
    // le fil vers l'organisation, une demande de correction aussi (elle est
    // toujours partagée, écart n° 99).
    if fil_adresse.is_some() {
        if demande_de_correction {
            return Err(ApiError::validation(
                "Une demande de correction part à l'organisation : elle ne se pose pas dans un fil adressé à des personnes nommées.",
                "is_change_request",
            ));
        }
        if visibilite == VISIBILITE_DEPOSANT {
            visibilite = VISIBILITE_COMITE;
        }
    }

    let etat = proposals::etat(state.pool(), dossier)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let mut tx = state.db().write(ctx).await?;
    let message = comments::ecrire(
        &mut tx,
        dossier,
        auteur,
        &NouveauMessage {
            parent_id: payload.parent_id,
            visibility: visibilite,
            body: corps,
            is_change_request: demande_de_correction,
        },
    )
    .await?;

    if !destinataires.is_empty() {
        let ids: Vec<Uuid> = destinataires.iter().map(|p| p.person_id).collect();
        comments::ajouter_destinataires(&mut tx, message.id, &ids, auteur).await?;
        circle::annoncer(
            &mut tx,
            dossier,
            message.id,
            auteur,
            ids,
            circle::Annonce::Adresse,
        )
        .await?;
    } else if let Some((racine_auteur, cercle)) = &fil_adresse {
        if visibilite == VISIBILITE_COMITE {
            let a_prevenir = std::iter::once(*racine_auteur)
                .chain(cercle.iter().copied())
                .filter(|p| *p != auteur)
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            circle::annoncer(
                &mut tx,
                dossier,
                message.id,
                auteur,
                a_prevenir,
                circle::Annonce::Reponse,
            )
            .await?;
        }
    }

    // **Un seul cas émet.** Voir l'en-tête du fichier.
    if visibilite == VISIBILITE_DEPOSANT {
        events::emit(
            &mut tx,
            DomainEvent {
                aggregate_schema: contracts::programme::AGGREGATE_SCHEMA,
                aggregate_type: contracts::programme::AGGREGATE_COMMENT,
                aggregate_id: message.id,
                event_type: contracts::programme::COMMENT_SHARED,
                payload: serde_json::to_value(contracts::programme::CommentShared {
                    proposal_id: dossier.as_uuid(),
                    reference_code: etat.reference_code.clone(),
                    comment_id: message.id,
                    author_id: auteur,
                    organization_id: etat.organization_id,
                    is_change_request: demande_de_correction,
                })
                .map_err(ApiError::internal)?,
            },
        )
        .await?;
    }

    tx.commit().await?;

    let mut vues = circle::composer(state, auteur, dossier, edition, cote, vec![message]).await?;
    vues.pop().ok_or_else(ApiError::not_found)
}

pub(crate) fn cote_de(acces: Acces) -> Cote {
    match acces {
        Acces::BackOffice => Cote::Comite,
        Acces::Organisation => Cote::Organisation,
    }
}

/// **Ce que l'auteur a demandé, et ce que la règle impose.**
///
/// Côté organisation : partagé, jamais une demande de correction. Côté
/// comité : la visibilité demandée, **sauf** pour une demande de correction,
/// qui est forcée en partagé.
fn arbitrer(acces: Acces, payload: &PostCommentPayload) -> (&'static str, bool) {
    if acces == Acces::Organisation {
        return (VISIBILITE_DEPOSANT, false);
    }

    if payload.is_change_request {
        return (VISIBILITE_DEPOSANT, true);
    }

    let visibilite = match payload.visibility.as_deref() {
        Some(VISIBILITE_DEPOSANT) => VISIBILITE_DEPOSANT,
        Some(VISIBILITE_PRIVEE) => VISIBILITE_PRIVEE,
        // Le défaut de la colonne, et le plus prudent : ce qui n'est pas
        // explicitement adressé au déposant ne lui part pas.
        _ => VISIBILITE_COMITE,
    };

    (visibilite, false)
}

/// Un message ne répond qu'à un message **du même dossier**.
///
/// La base ne le contraint pas : `parent_id` référence n'importe quel message.
/// Sans ce contrôle, une réponse forgée rattacherait un fil d'un dossier à
/// l'autre, et le filtrage par visibilité du second ne s'appliquerait pas au
/// premier.
async fn exiger_le_meme_dossier(
    state: &ProgrammeState,
    dossier: ProposalId,
    parent: CommentId,
) -> Result<()> {
    let message = comments::par_id(state.pool(), parent)
        .await?
        .ok_or_else(ApiError::not_found)?;

    if message.proposal_id == dossier.as_uuid() {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}

/// Le fil d'un dossier, **du côté où se tient le lecteur**, avec destinataires,
/// état de lecture et accusés.
pub async fn fil_de(
    state: &ProgrammeState,
    lecteur: Uuid,
    dossier: ProposalId,
) -> Result<Vec<ProposalCommentView>> {
    let (edition, acces) = perimeter::acces_au_dossier(state.pool(), lecteur, dossier).await?;
    let cote = cote_de(acces);
    let messages = comments::fil(state.pool(), dossier, lecteur, cote).await?;

    circle::composer(state, lecteur, dossier, edition, cote, messages).await
}

// -----------------------------------------------------------------------------
// La résolution d'une demande de correction
// -----------------------------------------------------------------------------

/// `ResolveCommentPayload`. Le verbe HTTP porte déjà le sens — `POST` pose,
/// `DELETE` retire —, et le champ `resolved` du contrat est donc redondant : on
/// le lit sans s'y fier, l'appelant restant le verbe.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ResolveCommentPayload {
    #[serde(default)]
    pub resolved: Option<bool>,
}

/// Poser ou retirer la résolution d'une demande de correction.
///
/// # Qui peut le faire n'est écrit NULLE PART dans le modèle
///
/// `resolved_by` est une simple clé étrangère vers une personne : la base
/// n'exprime aucune règle. L'écart n° 35 a été tranché en A5 — **le déposant
/// pose** (c'est lui qui sait qu'il a corrigé) et **le comité garde la main
/// pour retirer**, par permission et non par formulaire.
///
/// **Les deux gestes ne demandent donc pas le même droit** : poser est ouvert
/// aux deux côtés, retirer ne l'est qu'au comité — un déposant qui pourrait
/// retirer sa propre résolution ne changerait rien d'utile, mais un déposant
/// qui pourrait retirer celle du comité effacerait un arbitrage.
///
/// **Rien n'est émis** : l'état visible est le compteur de demandes ouvertes,
/// relu à chaque affichage.
pub async fn resoudre(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    message: CommentId,
    resolu: bool,
) -> Result<Message> {
    let ligne = comments::par_id(state.pool(), message)
        .await?
        .ok_or_else(ApiError::not_found)?;

    if !ligne.is_change_request {
        return Err(ApiError::validation(
            "Ce message n'est pas une demande de correction : il n'y a rien à résoudre.",
            "comment_id",
        ));
    }

    let dossier = ProposalId(ligne.proposal_id);
    let (_, acces) = perimeter::acces_au_dossier(state.pool(), acteur, dossier).await?;

    if !resolu && acces == Acces::Organisation {
        return Err(ApiError::with_message(
            kernel::error::ErrorCode::Forbidden,
            "Seul le comité peut rouvrir une demande de correction.",
        ));
    }

    let mut tx = state.db().write(ctx).await?;
    let mise_a_jour = sqlx::query!(
        r#"UPDATE programme.proposal_comments
              SET resolved_at = CASE WHEN $2 THEN now() END,
                  resolved_by = CASE WHEN $2 THEN $3::uuid END
            WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, proposal_id, parent_id, author_id,
                  visibility::text AS "visibility!", body, is_change_request,
                  resolved_at, resolved_by, edited_at, created_at"#,
        message.as_uuid(),
        resolu,
        acteur
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Message {
        id: mise_a_jour.id,
        proposal_id: mise_a_jour.proposal_id,
        parent_id: mise_a_jour.parent_id,
        author_id: mise_a_jour.author_id,
        visibility: mise_a_jour.visibility,
        body: mise_a_jour.body,
        is_change_request: mise_a_jour.is_change_request,
        resolved_at: mise_a_jour.resolved_at,
        resolved_by: mise_a_jour.resolved_by,
        edited_at: mise_a_jour.edited_at,
        created_at: mise_a_jour.created_at,
    })
}
