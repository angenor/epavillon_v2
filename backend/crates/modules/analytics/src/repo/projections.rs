//! Les projections matérialisées — **quatre lues sur huit**.
//!
//! | Projection | Lue | Pourquoi |
//! |---|---|---|
//! | `mv_proposal_funnel` | ✅ | l'entonnoir, les dépôts, le taux d'acceptation, les activités programmées |
//! | `mv_daily_submissions` | ✅ | la courbe des dépôts et sa moyenne mobile |
//! | `mv_daily_registrations` | ✅ | la courbe des inscriptions aux activités |
//! | `mv_reviewer_workload` | ✅ | les revues en retard et l'avancement du comité |
//! | `mv_daily_signups` | ❌ | compte des **créations de compte** sur toute la plateforme : ne se ventile par aucune édition (écart n° 40) |
//! | `mv_organization_scorecard` | ❌ | aucun écran ne l'affiche |
//! | `mv_session_attendance` | ❌ | idem |
//! | `mv_content_popularity` | ❌ | idem |
//!
//! **Les quatre non lues sont rafraîchies quand même** : `refresh_all()` les
//! porte, et les en retirer serait modifier le modèle pour un gain nul.
//!
//! **Elles sont VIDES à la création.** Un test qui lit un chiffre commence donc
//! par `analytics.refresh_all(false)` — c'est une contrainte de rédaction, pas
//! un détail.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::Date;
use uuid::Uuid;

use crate::domain::figures::ProposalFunnelRow;

/// L'entonnoir de l'édition. Le grain de la projection est **(édition, appel)** :
/// le jalon n'en pose qu'un par édition, la lecture restera la même quand il y
/// en aura deux.
///
/// La ligne retenue est celle de l'appel ; à défaut, celle des propositions hors
/// appel — que la projection porte sous une clé nulle.
pub async fn entonnoir(
    conn: &mut PgConnection,
    event_id: Uuid,
    call_id: Option<Uuid>,
) -> Result<Option<ProposalFunnelRow>> {
    let ligne = sqlx::query_as!(
        ProposalFunnelRow,
        r#"SELECT f.event_id            AS "event_id!",
                  f.cle_appel           AS "cle_appel!",
                  f.call_id,
                  f.evenement,
                  f.edition_year        AS "edition_year!",
                  f.statut_evenement    AS "statut_evenement!",
                  f.code_appel,
                  f.appel,
                  f.statut_appel,
                  f.appel_ouvre_le,
                  f.appel_ferme_le,
                  f.required_reviews,
                  f.total               AS "total!",
                  f.brouillons          AS "brouillons!",
                  f.deposees            AS "deposees!",
                  f.en_attente_affectation AS "en_attente_affectation!",
                  f.en_revue            AS "en_revue!",
                  f.modifications_demandees AS "modifications_demandees!",
                  f.acceptees           AS "acceptees!",
                  f.rejetees            AS "rejetees!",
                  f.retirees            AS "retirees!",
                  f.annulees            AS "annulees!",
                  f.decidees            AS "decidees!",
                  f.en_instance         AS "en_instance!",
                  f.taux_acceptation::float8    AS "taux_acceptation?",
                  f.taux_acceptation_sur_depots::float8 AS "taux_acceptation_sur_depots?",
                  f.organisations_distinctes AS "organisations_distinctes!",
                  f.note_moyenne::float8 AS "note_moyenne?",
                  f.delai_median_decision_heures::float8 AS "delai_median_decision_heures?",
                  f.premier_depot,
                  f.dernier_depot,
                  f.sessions_programmees AS "sessions_programmees!"
             FROM analytics.mv_proposal_funnel f
            WHERE f.event_id = $1
            ORDER BY (f.call_id IS NOT DISTINCT FROM $2) DESC, f.deposees DESC
            LIMIT 1"#,
        event_id,
        call_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne)
}

/// Un jour de série, tel que les deux projections quotidiennes le rendent.
pub struct JourDeSerie {
    pub jour: Date,
    pub valeur: i64,
    pub cumul: i64,
    pub moyenne_7j: Option<f64>,
}

/// La courbe des dépôts. **Série continue, jours vides compris** : c'est garanti
/// en base, et **aucun trou n'est rebouché ici**. Un composant de courbe qui
/// trouverait un trou signalerait une requête fautive, pas une donnée manquante.
pub async fn depots_par_jour(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<JourDeSerie>> {
    let lignes = sqlx::query_as!(
        JourDeSerie,
        r#"SELECT d.jour                AS "jour!",
                  d.soumissions         AS "valeur!",
                  d.soumissions_cumulees::bigint AS "cumul!",
                  d.moyenne_mobile_7j::float8 AS "moyenne_7j?"
             FROM analytics.mv_daily_submissions d
            WHERE d.event_id = $1
            ORDER BY d.jour"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes)
}

/// La courbe des inscriptions **aux activités** — à ne pas confondre avec
/// `mv_daily_signups`, qui compte des créations de compte sur toute la
/// plateforme.
pub async fn inscriptions_par_jour(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<Vec<JourDeSerie>> {
    let lignes = sqlx::query_as!(
        JourDeSerie,
        r#"SELECT r.jour                 AS "jour!",
                  r.inscriptions         AS "valeur!",
                  r.inscriptions_cumulees::bigint AS "cumul!",
                  r.moyenne_mobile_7j::float8 AS "moyenne_7j?"
             FROM analytics.mv_daily_registrations r
            WHERE r.event_id = $1
            ORDER BY r.jour"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes)
}

/// L'avancement du comité, et **qui prend du retard**.
pub struct ChargeDuComite {
    pub revisionniste: Option<String>,
    pub propositions_assignees: i64,
    pub revues_soumises: i64,
    pub revues_en_retard: i64,
}

pub async fn charge_du_comite(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<Vec<ChargeDuComite>> {
    let lignes = sqlx::query_as!(
        ChargeDuComite,
        r#"SELECT w.revisionniste,
                  w.propositions_assignees AS "propositions_assignees!",
                  w.revues_soumises        AS "revues_soumises!",
                  w.revues_en_retard       AS "revues_en_retard!"
             FROM analytics.mv_reviewer_workload w
            WHERE w.event_id = $1
            ORDER BY w.revues_en_retard DESC, w.revisionniste"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes)
}
