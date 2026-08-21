//! Confier un dossier, en retirer la charge, lire qui l'évalue.
//!
//! # Le déport n'est pas une suppression, et cela commande tout ce fichier
//!
//! `recused_at` et `recusal_reason` gardent la trace d'une déclaration
//! d'impartialité : un membre du comité a dit avoir un lien avec
//! l'organisation porteuse et s'est retiré. Effacer la ligne effacerait la
//! déclaration, et **réattribuer le dossier à la même personne la contredirait
//! en silence**.
//!
//! Conséquence : `ux_review_assignments` étant unique sur `(dossier, membre)`,
//! une réattribution après déport se heurterait à la contrainte. Le service ne
//! la tente pas — il **écarte**, et nomme la raison (`recused`).
//!
//! # Ce fichier n'émet rien
//!
//! `programme.review.assigned` est émis par le service, **une fois par
//! dossier**, dans la même transaction. Le mettre ici le rendrait invisible à
//! qui lit le service, et ferait émettre la lecture d'une affectation.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::ProposalId;

/// Une affectation, telle que ce module la manipule.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Affectation {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub reviewer_id: Uuid,
    pub assigned_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub assigned_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub recused_at: Option<OffsetDateTime>,
    pub recusal_reason: Option<String>,
}

/// L'affectation d'une personne sur un dossier, **déportée comprise**.
///
/// C'est elle qui distingue les deux écarts de l'action groupée : une ligne
/// sans déport est « déjà confié », une ligne déportée est « déporté ». Les
/// confondre effacerait la nuance que l'écran affiche.
pub async fn affectation<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    membre: Uuid,
) -> Result<Option<Affectation>> {
    let ligne = sqlx::query!(
        "SELECT id, proposal_id, reviewer_id, assigned_by, assigned_at,
                due_at, recused_at, recusal_reason
           FROM programme.review_assignments
          WHERE proposal_id = $1 AND reviewer_id = $2",
        dossier.as_uuid(),
        membre
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| Affectation {
        id: l.id,
        proposal_id: l.proposal_id,
        reviewer_id: l.reviewer_id,
        assigned_by: l.assigned_by,
        assigned_at: l.assigned_at,
        due_at: l.due_at,
        recused_at: l.recused_at,
        recusal_reason: l.recusal_reason,
    }))
}

/// Toutes les affectations d'un dossier, **déports compris**.
///
/// La fiche du comité montre qui s'est déporté : le masquer donnerait à croire
/// que le dossier n'a jamais été confié à cette personne.
pub async fn du_dossier<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<Affectation>> {
    let lignes = sqlx::query!(
        "SELECT id, proposal_id, reviewer_id, assigned_by, assigned_at,
                due_at, recused_at, recusal_reason
           FROM programme.review_assignments
          WHERE proposal_id = $1
          ORDER BY assigned_at, id",
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Affectation {
            id: l.id,
            proposal_id: l.proposal_id,
            reviewer_id: l.reviewer_id,
            assigned_by: l.assigned_by,
            assigned_at: l.assigned_at,
            due_at: l.due_at,
            recused_at: l.recused_at,
            recusal_reason: l.recusal_reason,
        })
        .collect())
}

/// Confier un dossier à un membre du comité.
///
/// **`ON CONFLICT DO NOTHING` reste le dernier mot** : le service a déjà lu
/// l'affectation et écarté le déjà confié, mais deux actions groupées lancées
/// en même temps se croiseraient entre la lecture et l'écriture. Rendre `None`
/// plutôt qu'une erreur laisse l'appelant porter l'écart, sans avorter les
/// onze autres dossiers de la sélection.
pub async fn confier(
    conn: &mut PgConnection,
    dossier: ProposalId,
    membre: Uuid,
    par: Uuid,
    echeance: Option<OffsetDateTime>,
) -> Result<Option<Affectation>> {
    let ligne = sqlx::query!(
        "INSERT INTO programme.review_assignments
             (proposal_id, reviewer_id, assigned_by, due_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (proposal_id, reviewer_id) DO NOTHING
         RETURNING id, proposal_id, reviewer_id, assigned_by, assigned_at,
                   due_at, recused_at, recusal_reason",
        dossier.as_uuid(),
        membre,
        par,
        echeance
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| Affectation {
        id: l.id,
        proposal_id: l.proposal_id,
        reviewer_id: l.reviewer_id,
        assigned_by: l.assigned_by,
        assigned_at: l.assigned_at,
        due_at: l.due_at,
        recused_at: l.recused_at,
        recusal_reason: l.recusal_reason,
    }))
}

/// Retirer une affectation — **jamais un déport**.
///
/// Retirer, c'est corriger une répartition : le dossier n'aurait pas dû être
/// confié à cette personne. Se déporter, c'est déclarer un lien, et cela
/// s'écrit (US4). La condition `recused_at IS NULL` interdit qu'un retrait
/// efface une déclaration d'impartialité ; il rend alors « rien retiré ».
pub async fn retirer(conn: &mut PgConnection, dossier: ProposalId, membre: Uuid) -> Result<bool> {
    let effacees = sqlx::query!(
        "DELETE FROM programme.review_assignments
          WHERE proposal_id = $1 AND reviewer_id = $2 AND recused_at IS NULL",
        dossier.as_uuid(),
        membre
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(effacees == 1)
}
