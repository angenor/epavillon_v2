//! Lecture du schéma `programme` — **en lecture seule**.
//!
//! Quatre des cinq familles d'alerte, et les deux répartitions.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

/// Un dossier déposé que personne n'a encore évalué.
///
/// **L'échéance applicable n'est PAS celle de l'appel** quand le dossier est
/// confié : c'est `min(review_assignments.due_at)` sur ses affectations non
/// déportées — ce que `v_proposal_dashboard.next_review_due_at` calcule déjà, et
/// qu'on n'a donc pas à recomposer. Un dossier **sans aucune affectation** n'a,
/// lui, que l'échéance de l'appel.
pub struct DossierSansEvaluation {
    pub proposal_id: Uuid,
    pub reference_code: String,
    pub title_text: Option<String>,
    pub echeance_applicable: Option<OffsetDateTime>,
    pub sans_revisionniste: bool,
}

/// Les dossiers `submitted` ou `under_review` **sans aucune revue**, dont
/// l'échéance applicable tombe dans moins de `jours` — **ou** qui n'ont aucun
/// révisionniste affecté, déports exclus.
///
/// Les deux conditions ne disent pas la même chose : la première est une
/// urgence de calendrier, la seconde un oubli d'affectation qui n'a pas d'heure.
/// Un dossier déposé la veille d'un appel qui ferme dans trois mois n'est
/// **pas** une alerte — c'est le fonctionnement normal.
pub async fn dossiers_sans_evaluation(
    conn: &mut PgConnection,
    event_id: Uuid,
    jours: i32,
) -> Result<Vec<DossierSansEvaluation>> {
    let lignes = sqlx::query!(
        r#"SELECT d.id                 AS "proposal_id!",
                  d.reference_code     AS "reference_code!",
                  d.title_text,
                  COALESCE(d.next_review_due_at, event.effective_deadline(d.call_id))
                                       AS "echeance_applicable?",
                  (d.assigned_reviewers = 0) AS "sans_revisionniste!"
             FROM programme.v_proposal_dashboard d
            WHERE d.event_id = $1
              AND d.status IN ('submitted', 'under_review')
              AND d.review_count = 0
              AND (
                    d.assigned_reviewers = 0
                 OR COALESCE(d.next_review_due_at, event.effective_deadline(d.call_id))
                      <= now() + make_interval(days => $2)
                  )
            ORDER BY COALESCE(d.next_review_due_at, event.effective_deadline(d.call_id))
                     NULLS LAST,
                     d.submitted_at"#,
        event_id,
        jours
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| DossierSansEvaluation {
            proposal_id: l.proposal_id,
            reference_code: l.reference_code,
            title_text: l.title_text,
            echeance_applicable: l.echeance_applicable,
            sans_revisionniste: l.sans_revisionniste,
        })
        .collect())
}

/// L'échéance la plus proche encore due sur l'édition, toutes affectations
/// confondues, déports exclus.
pub async fn prochaine_echeance_de_revue(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<Option<OffsetDateTime>> {
    let instant = sqlx::query_scalar!(
        r#"SELECT min(ra.due_at) AS "echeance?"
             FROM programme.review_assignments ra
             JOIN programme.proposals p ON p.id = ra.proposal_id AND p.deleted_at IS NULL
            WHERE p.event_id = $1 AND ra.recused_at IS NULL"#,
        event_id
    )
    .fetch_one(conn)
    .await?;

    Ok(instant)
}

/// Un chevauchement détecté. **Jamais bloquant** : la règle métier n° 2 dit
/// qu'on détecte et qu'on affiche, sans jamais refuser. La gravité vient de la
/// fonction du modèle, pas d'un jugement du code.
pub struct Chevauchement {
    pub severity: String,
    pub conflict_kind: String,
    pub subject_label: Option<String>,
    pub session_a_title: Option<String>,
    pub session_b_title: Option<String>,
}

pub async fn conflits(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<Chevauchement>> {
    let lignes = sqlx::query_as!(
        Chevauchement,
        r#"SELECT c.severity        AS "severity!",
                  c.conflict_kind   AS "conflict_kind!",
                  c.subject_label,
                  c.session_a_title,
                  c.session_b_title
             FROM programme.detect_conflicts($1) c"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes)
}

/// Une part de répartition, avant résolution du libellé.
pub struct Part {
    pub cle: String,
    pub compte: i64,
}

/// Les organisations porteuses, **par pays**. Une organisation sans pays n'entre
/// dans aucune part : l'inventer sous « autres » ferait passer une donnée
/// manquante pour une donnée rare.
pub async fn par_pays(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<Part>> {
    let lignes = sqlx::query!(
        r#"SELECT co.iso2 AS "cle!", count(DISTINCT p.organization_id) AS "compte!"
             FROM programme.proposals p
             JOIN org.organizations o    ON o.id = p.organization_id
             JOIN reference.countries co ON co.id = o.country_id
            WHERE p.event_id = $1 AND p.deleted_at IS NULL
            GROUP BY co.iso2
            ORDER BY count(DISTINCT p.organization_id) DESC, co.iso2"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Part {
            cle: l.cle,
            compte: l.compte,
        })
        .collect())
}

/// Les dossiers **par thématique**. Un dossier en porte plusieurs : la somme
/// dépasse légitimement le total, et l'écran le dit.
pub async fn par_thematique(conn: &mut PgConnection, event_id: Uuid) -> Result<Vec<Part>> {
    let lignes = sqlx::query!(
        r#"SELECT t.code AS "cle!", count(*) AS "compte!"
             FROM programme.proposals p
             JOIN reference.entity_terms et
               ON et.entity_schema = 'programme'
              AND et.entity_table  = 'proposals'
              AND et.entity_id     = p.id
             JOIN reference.taxonomy_terms t
               ON t.id = et.term_id AND t.taxonomy_code = 'activity_theme'
            WHERE p.event_id = $1 AND p.deleted_at IS NULL
            GROUP BY t.code
            ORDER BY count(*) DESC, t.code"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Part {
            cle: l.cle,
            compte: l.compte,
        })
        .collect())
}
