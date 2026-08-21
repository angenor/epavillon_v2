//! Les revues : la mienne, celles des pairs, et l'avancement du comité.
//!
//! # Une revue par personne et par dossier, et l'écriture le sait
//!
//! `ux_reviews` borne le couple `(dossier, membre)`. L'enregistrement est donc
//! un `INSERT … ON CONFLICT DO UPDATE` : une personne n'a qu'une revue, qu'elle
//! l'écrive pour la première fois ou qu'elle la reprenne. Traiter le conflit
//! comme une erreur obligerait l'appelant à lire avant d'écrire, et deux
//! enregistrements automatiques qui se croiseraient rendraient un refus qui
//! n'apprend rien.
//!
//! # Le brouillon ne compte nulle part, et c'est la base qui le dit
//!
//! `refresh_proposal_score()` ne retient que les revues dont `submitted_at`
//! n'est pas nulle. Une revue en brouillon n'entre donc dans aucun agrégat —
//! et ce fichier ne la rend à aucun pair, pour la même raison : elle n'existe
//! pas encore comme avis.
//!
//! # 🔴 La consolidation n'est appelée par AUCUN déclencheur
//!
//! `programme.refresh_proposal_score()` existe, son commentaire dit « à
//! appeler après toute saisie de note », et rien ne l'appelle. Sans appel
//! explicite, la note d'un dossier, sa moyenne, son nombre de revues et son
//! élimination restent aux valeurs de la ligne : **le classement du comité est
//! faux sans qu'aucune erreur ne le signale** (écart n° 98). C'est le pire
//! défaut du module parce qu'il est muet.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::ProposalId;

/// Une revue — exactement `Review`.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct Revue {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub reviewer_id: Uuid,
    pub recommendation: String,
    pub weighted_score: Option<f64>,
    pub score_out_of_20: Option<f64>,
    pub strengths: Option<String>,
    pub weaknesses: Option<String>,
    /// **Visible du seul comité, jamais du soumissionnaire.** Aucune
    /// composition de l'espace organisation ne lit cette table.
    pub private_note: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub submitted_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

macro_rules! revue_depuis {
    ($l:expr) => {
        Revue {
            id: $l.id,
            proposal_id: $l.proposal_id,
            reviewer_id: $l.reviewer_id,
            recommendation: $l.recommendation,
            weighted_score: $l.weighted_score,
            score_out_of_20: $l.score_out_of_20,
            strengths: $l.strengths,
            weaknesses: $l.weaknesses,
            private_note: $l.private_note,
            submitted_at: $l.submitted_at,
            created_at: $l.created_at,
            updated_at: $l.updated_at,
        }
    };
}

/// Ma revue sur ce dossier, **brouillon compris** : c'est la mienne, je la
/// reprends là où je l'ai laissée.
pub async fn mienne<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    membre: Uuid,
) -> Result<Option<Revue>> {
    let ligne = sqlx::query!(
        r#"SELECT id, proposal_id, reviewer_id, recommendation,
                  weighted_score::float8, score_out_of_20::float8,
                  strengths, weaknesses, private_note, submitted_at,
                  created_at, updated_at
             FROM programme.reviews
            WHERE proposal_id = $1 AND reviewer_id = $2"#,
        dossier.as_uuid(),
        membre
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| revue_depuis!(l)))
}

/// Les revues **déposées** des autres membres du comité.
///
/// **Cette fonction n'est pas appelée quand le voile est baissé.** Elle n'a
/// pas de paramètre de masquage, et c'est délibéré : un filtre porté ici
/// laisserait croire qu'on peut l'oublier. Le voile est une décision de
/// l'appelant, prise avant l'appel (R4).
pub async fn des_pairs<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    sauf: Uuid,
) -> Result<Vec<Revue>> {
    let lignes = sqlx::query!(
        r#"SELECT id, proposal_id, reviewer_id, recommendation,
                  weighted_score::float8, score_out_of_20::float8,
                  strengths, weaknesses, private_note, submitted_at,
                  created_at, updated_at
             FROM programme.reviews
            WHERE proposal_id = $1 AND reviewer_id <> $2
              AND submitted_at IS NOT NULL
            ORDER BY submitted_at"#,
        dossier.as_uuid(),
        sauf
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| revue_depuis!(l)).collect())
}

/// **Combien de revues déposées le voile me cache.**
///
/// Compter n'ancre pas ; lire, si. C'est la seule chose que le voile laisse
/// passer, et c'est ce qui permet d'afficher « 2 revues déposées » sans en
/// montrer une seule.
pub async fn compter_deposees<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    sauf: Uuid,
) -> Result<i64> {
    let compte = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.reviews
            WHERE proposal_id = $1 AND reviewer_id <> $2
              AND submitted_at IS NOT NULL"#,
        dossier.as_uuid(),
        sauf
    )
    .fetch_one(executor)
    .await?;

    Ok(compte)
}

/// Ce qu'une écriture de revue pose, hors les notes par critère.
pub struct ChampsDeLaRevue<'a> {
    pub recommendation: &'a str,
    pub strengths: Option<&'a str>,
    pub weaknesses: Option<&'a str>,
    pub private_note: Option<&'a str>,
    /// Vrai pour un dépôt. **Une revue déposée ne redevient pas un
    /// brouillon** : la date est posée une fois et l'écriture suivante la
    /// conserve — le voile ne se rabaisse pas sur qui a déjà rendu son avis.
    pub deposer: bool,
}

/// Enregistrer ou déposer ma revue.
pub async fn enregistrer(
    conn: &mut PgConnection,
    dossier: ProposalId,
    membre: Uuid,
    champs: &ChampsDeLaRevue<'_>,
) -> Result<Revue> {
    let ligne = sqlx::query!(
        r#"INSERT INTO programme.reviews
               (proposal_id, reviewer_id, recommendation, strengths, weaknesses,
                private_note, submitted_at)
           VALUES ($1, $2, $3, $4, $5, $6,
                   CASE WHEN $7 THEN now() END)
           ON CONFLICT (proposal_id, reviewer_id) DO UPDATE
              SET recommendation = EXCLUDED.recommendation,
                  strengths      = EXCLUDED.strengths,
                  weaknesses     = EXCLUDED.weaknesses,
                  private_note   = EXCLUDED.private_note,
                  submitted_at   = COALESCE(programme.reviews.submitted_at,
                                            EXCLUDED.submitted_at)
        RETURNING id, proposal_id, reviewer_id, recommendation,
                  weighted_score::float8, score_out_of_20::float8,
                  strengths, weaknesses, private_note, submitted_at,
                  created_at, updated_at"#,
        dossier.as_uuid(),
        membre,
        champs.recommendation,
        champs.strengths,
        champs.weaknesses,
        champs.private_note,
        champs.deposer
    )
    .fetch_one(conn)
    .await?;

    Ok(revue_depuis!(ligne))
}

/// **Consolider les notes du dossier — dans la transaction de l'écriture.**
///
/// La fonction du modèle recalcule la note pondérée de chaque revue, puis la
/// moyenne du dossier, son nombre de revues **déposées** et son élimination.
/// L'appeler hors transaction laisserait une fenêtre où la revue est écrite et
/// le classement faux ; ne pas l'appeler du tout laisserait le classement faux
/// **pour toujours, et en silence** (écart n° 98).
pub async fn consolider(conn: &mut PgConnection, dossier: ProposalId) -> Result<()> {
    sqlx::query!(
        "SELECT programme.refresh_proposal_score($1)",
        dossier.as_uuid()
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Les agrégats du dossier, **relus après consolidation**.
///
/// Ils sont relus et non recalculés : l'autorité du calcul reste en base
/// (R24), et rendre une valeur calculée en Rust à côté d'une valeur écrite en
/// SQL produirait deux vérités pour le même dossier.
#[derive(Debug, Clone, serde::Serialize, ToSchema)]
pub struct Agregats {
    pub weighted_score: Option<f64>,
    pub average_score: Option<f64>,
    pub review_count: i16,
    pub is_knocked_out: bool,
}

pub async fn agregats<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Option<Agregats>> {
    let ligne = sqlx::query!(
        r#"SELECT weighted_score::float8, average_score::float8,
                  review_count, is_knocked_out
             FROM programme.proposals WHERE id = $1"#,
        dossier.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| Agregats {
        weighted_score: l.weighted_score,
        average_score: l.average_score,
        review_count: l.review_count,
        is_knocked_out: l.is_knocked_out,
    }))
}

/// L'avancement d'un membre du comité sur ce dossier, tel que la base le
/// donne. L'**état** qui s'en déduit est calculé par `domain/desk.rs` : trois
/// composants du front le calculaient chacun de leur côté, et divergeaient sur
/// le cas limite qui compte — une revue commencée n'est pas une revue rendue.
#[derive(Debug, Clone)]
pub struct LigneDAvancement {
    pub assignment: crate::repo::assignments::Affectation,
    pub review_submitted_at: Option<OffsetDateTime>,
    pub review_existe: bool,
}

/// L'avancement nominatif du comité sur ce dossier, **déports compris**.
///
/// Masquer les déportés donnerait à croire que le dossier ne leur a jamais été
/// confié, alors que la trace de leur retrait est précisément ce que la table
/// garde.
pub async fn avancement_du_comite<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<LigneDAvancement>> {
    let lignes = sqlx::query!(
        r#"SELECT ra.id, ra.proposal_id, ra.reviewer_id, ra.assigned_by,
                  ra.assigned_at, ra.due_at, ra.recused_at, ra.recusal_reason,
                  rv.submitted_at AS "review_submitted_at?",
                  (rv.id IS NOT NULL) AS "review_existe!"
             FROM programme.review_assignments ra
             LEFT JOIN programme.reviews rv
                    ON rv.proposal_id = ra.proposal_id
                   AND rv.reviewer_id = ra.reviewer_id
            WHERE ra.proposal_id = $1
            ORDER BY ra.assigned_at, ra.id"#,
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| LigneDAvancement {
            assignment: crate::repo::assignments::Affectation {
                id: l.id,
                proposal_id: l.proposal_id,
                reviewer_id: l.reviewer_id,
                assigned_by: l.assigned_by,
                assigned_at: l.assigned_at,
                due_at: l.due_at,
                recused_at: l.recused_at,
                recusal_reason: l.recusal_reason,
            },
            review_submitted_at: l.review_submitted_at,
            review_existe: l.review_existe,
        })
        .collect())
}
