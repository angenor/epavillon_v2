//! La machine à états, **lue** — jamais recopiée.
//!
//! # Ce que ce fichier fait, et ce qu'il ne fait pas
//!
//! `programme.proposal_transitions_allowed` porte quatorze lignes. Aucune n'est
//! écrite dans le code : ouvrir un chemin doit être une ligne de plus en base,
//! pas une relecture du service. Ce fichier **lit** la table — telle quelle
//! pour la route globale du contrat, croisée avec le lecteur pour les
//! transitions offertes —, et lit le **journal**.
//!
//! # Pourquoi les transitions offertes tiennent en une seule requête (R7)
//!
//! Quatorze règles, dont au plus quatre applicables à un état donné. Les
//! évaluer une par une ferait autant d'allers-retours pour composer un menu, et
//! appellerait la fonction d'autorisation autant de fois qu'il y a de règles.
//!
//! Surtout, **le croisement doit se faire au même instant que la lecture de
//! l'état** : deux requêtes séparées offriraient une transition depuis un état
//! déjà changé.
//!
//! # La portée est celle de l'ÉDITION du dossier, pas la portée globale
//!
//! C'est le principe V, et c'est ce qui fait qu'un responsable détaché sur un
//! webinaire ne décide pas sur la COP31.

use kernel::error::Result;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::domain::transitions::{AvailableTransition, ProposalStatus, ProposalTransitionRule};

/// La table des règles, **telle quelle** — ce que rend `GET /proposals/transitions`.
///
/// Elle est **globale et sans dossier** : l'écran s'en sert pour savoir quelles
/// actions existent et lesquelles exigent un motif, avant même d'ouvrir une
/// fiche. Les transitions offertes **pour un dossier et un lecteur** sont une
/// autre question, et une autre route (écart n° 101).
pub async fn regles<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<ProposalTransitionRule>> {
    let lignes = sqlx::query!(
        r#"SELECT from_status::text AS "depuis!", to_status::text AS "vers!",
                  required_permission, allowed_for_owner, requires_reason
             FROM programme.proposal_transitions_allowed
            ORDER BY from_status, to_status"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .filter_map(|l| {
            Some(ProposalTransitionRule {
                from_status: ProposalStatus::from_db(&l.depuis)?,
                to_status: ProposalStatus::from_db(&l.vers)?,
                required_permission: l.required_permission,
                allowed_for_owner: l.allowed_for_owner,
                requires_reason: l.requires_reason,
            })
        })
        .collect())
}

/// Les transitions **offertes à ce lecteur, sur ce dossier**, en une requête.
///
/// # Les deux voies, et pourquoi elles sont distinctes
///
/// Une règle porte deux choses indépendantes : une permission requise, et le
/// droit du **porteur** de la déclencher lui-même. Une transition est donc
/// offerte quand :
///
/// - le lecteur est **porteur** — adhésion active à l'organisation du
///   dossier — **et** la règle l'autorise (`allowed_for_owner`) ; **ou**
/// - la règle nomme une permission **et** le lecteur la détient **sur l'édition
///   du dossier**.
///
/// Les confondre casserait les deux extrémités de la table : le retrait par
/// l'organisation ne nomme **aucune** permission — le tester le rendrait
/// impossible —, et la mise en évaluation n'est **pas** ouverte au porteur —
/// s'en remettre à l'adhésion la lui offrirait.
pub async fn offertes<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    lecteur: Uuid,
) -> Result<Vec<AvailableTransition>> {
    let lignes = sqlx::query!(
        r#"SELECT r.to_status::text AS "vers!", r.requires_reason
             FROM programme.proposals p
             JOIN programme.proposal_transitions_allowed r ON r.from_status = p.status
            WHERE p.id = $1
              AND p.deleted_at IS NULL
              AND (
                    (r.allowed_for_owner AND EXISTS (
                         SELECT 1 FROM org.memberships m
                          WHERE m.organization_id = p.organization_id
                            AND m.person_id = $2
                            AND m.status = 'active'))
                 OR (r.required_permission IS NOT NULL
                     AND identity.has_permission($2, r.required_permission, 'event', p.event_id))
              )
            ORDER BY r.to_status"#,
        dossier.as_uuid(),
        lecteur
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .filter_map(|l| {
            Some(AvailableTransition {
                to_status: ProposalStatus::from_db(&l.vers)?,
                requires_reason: l.requires_reason,
            })
        })
        .collect())
}

/// Une ligne du journal — exactement `ProposalTransition`.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct LigneDeJournal {
    pub id: Uuid,
    pub proposal_id: Uuid,
    /// Nul pour la ligne d'ouverture du dossier.
    pub from_status: Option<String>,
    pub to_status: String,
    pub actor_id: Option<Uuid>,
    pub reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
}

/// Le journal d'un dossier, du plus récent au plus ancien.
///
/// **C'est lui qui porte chaque motif** (écart n° 97). La colonne
/// `decision_reason` du dossier ne garde que le **dernier** : une transition
/// suivante l'écrase, y compris quand elle n'en demande aucun — auquel cas elle
/// l'efface. Un écran qui lirait la colonne afficherait « motif de la décision »
/// sur un dossier remis en course, ou rien du tout.
pub async fn journal<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<LigneDeJournal>> {
    let lignes = sqlx::query!(
        r#"SELECT id, proposal_id, from_status::text AS "depuis?", to_status::text AS "vers!",
                  actor_id, reason, occurred_at
             FROM programme.proposal_transitions
            WHERE proposal_id = $1
            ORDER BY occurred_at DESC, id DESC"#,
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| LigneDeJournal {
            id: l.id,
            proposal_id: l.proposal_id,
            from_status: l.depuis,
            to_status: l.vers,
            actor_id: l.actor_id,
            reason: l.reason,
            occurred_at: l.occurred_at,
        })
        .collect())
}

/// La dernière ligne écrite — celle que la décision vient de produire.
pub async fn derniere<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Option<LigneDeJournal>> {
    Ok(journal(executor, dossier).await?.into_iter().next())
}
