//! La vue de pilotage du comité, **lue telle quelle**.
//!
//! # Pourquoi on ne recompose pas ses jointures
//!
//! `programme.v_proposal_dashboard` réunit onze sous-requêtes : avancement des
//! revues, classement, alertes, format, pays du porteur, thématiques,
//! co-organisateurs, membres du comité nommés, retards, prochaine échéance,
//! accusés de lecture collectifs. Elle a été étendue le 18/08 précisément pour
//! que l'écran de liste tienne en **une** requête, et son en-tête SQL le dit.
//!
//! Les recomposer ici — quatre tables de plus et les correspondances refaites —
//! reviendrait à réintroduire le défaut que l'extension de la vue a corrigé, et
//! à entretenir deux définitions du même « 2/3 ».
//!
//! # Les DEUX colonnes de titre, et pourquoi elles ne se confondent pas
//!
//! `title` est le document multilingue **brut**, du même type que
//! `programme.proposals.title` ; `title_text` est sa résolution française par
//! `platform.t()`, réservée au tri, au filtrage et à l'export.
//!
//! Une version antérieure de la vue nommait `title` la valeur déjà résolue :
//! le même nom de champ portait alors un `text` ici et un `i18n_text` sur la
//! table, et l'utilitaire de résolution du front rendait une chaîne vide
//! **sans erreur**. Les deux voyagent donc, sous deux noms et deux types.
//!
//! # Ce que la vue exclut déjà, et qu'on ne redit pas
//!
//! Les dossiers effacés (`deleted_at`). Le filtre est dans la vue ; l'ajouter
//! ici laisserait croire qu'il n'y est pas.

use kernel::error::Result;
use serde::Serialize;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::EventId;

/// Une ligne de la vue — exactement `ProposalDashboardRow`.
///
/// **Les `numeric` traversent en `float8`** : le workspace ne déclare aucune
/// caractéristique décimale pour SQLx, et le service ne calcule aucune
/// moyenne — l'autorité du calcul reste en base (R24).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LigneDePilotage {
    pub id: Uuid,
    pub reference_code: String,
    pub event_id: Uuid,
    pub call_id: Option<Uuid>,
    pub organization_id: Uuid,
    pub organization_name: String,
    /// Le document multilingue **brut**, résolu à l'affichage.
    pub title: serde_json::Value,
    /// Le même titre résolu en base. **Trier, filtrer, exporter** — jamais
    /// afficher : la liste ne pourrait plus changer de langue sans requête.
    pub title_text: Option<String>,
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub submitted_at: Option<OffsetDateTime>,
    pub weighted_score: Option<f64>,
    pub average_score: Option<f64>,
    pub is_knocked_out: bool,
    pub review_count: i16,
    pub required_reviews: Option<i16>,
    pub reviews_missing: Option<i32>,
    pub assigned_reviewers: i64,
    pub open_change_requests: i64,
    pub speaker_count: i64,
    pub event_rank: i64,
    pub format: String,
    pub activity_type_code: Option<String>,
    pub organization_acronym: Option<String>,
    pub organization_country_code: Option<String>,
    pub organization_country: Option<serde_json::Value>,
    pub co_organizer_count: i64,
    /// Pour **filtrer**. L'affichage passe par `themes`.
    pub theme_codes: Vec<String>,
    /// Pour **afficher** — libellé traduit et couleur venus de la taxonomie,
    /// où un administrateur les modifie.
    pub themes: serde_json::Value,
    pub reviewer_ids: Vec<Uuid>,
    /// Les mêmes, nommés : un « 2/3 » ne dit pas de qui on attend la troisième.
    pub reviewers: serde_json::Value,
    pub overdue_reviews: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub next_review_due_at: Option<OffsetDateTime>,
    /// **Collectif.** « Non consulté par moi » dépend du lecteur et vient de
    /// `programme.unread_proposals_for()` — voir `repo/reads.rs`.
    pub read_count: i64,
}

/// Les lignes d'une édition, **du plus haut classement au plus bas**.
///
/// L'ordre est celui du rang que la vue calcule, et non un tri du service : le
/// contrat n'expose aucun paramètre de tri (R17), et le comité cherche d'abord
/// « lesquels tiennent le haut du classement ». Le numéro de dossier départage
/// deux rangs égaux, sans quoi deux appels rendraient deux ordres.
///
/// **Aucune pagination** : quelques dizaines de dossiers par édition. Le
/// plafond est annoncé dans le quickstart, pas ignoré.
pub async fn lignes<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<LigneDePilotage>> {
    // Une vue ne porte aucune contrainte de non-nullité : SQLx rend tout
    // optionnel. Les `!` reprennent ce que les colonnes sous-jacentes
    // garantissent — jamais ce qu'on espère.
    let lignes = sqlx::query!(
        r#"SELECT id AS "id!", reference_code AS "reference_code!",
                  event_id AS "event_id!", call_id,
                  organization_id AS "organization_id!",
                  organization_name AS "organization_name!",
                  title AS "title!", title_text,
                  status::text AS "status!", submitted_at,
                  weighted_score::float8, average_score::float8,
                  is_knocked_out AS "is_knocked_out!",
                  review_count AS "review_count!",
                  required_reviews, reviews_missing,
                  assigned_reviewers AS "assigned_reviewers!",
                  open_change_requests AS "open_change_requests!",
                  speaker_count AS "speaker_count!",
                  event_rank AS "event_rank!",
                  format::text AS "format!", activity_type_code,
                  organization_acronym,
                  organization_country_code::text,
                  organization_country,
                  co_organizer_count AS "co_organizer_count!",
                  theme_codes AS "theme_codes!", themes AS "themes!",
                  reviewer_ids AS "reviewer_ids!", reviewers AS "reviewers!",
                  overdue_reviews AS "overdue_reviews!",
                  next_review_due_at,
                  read_count AS "read_count!"
             FROM programme.v_proposal_dashboard
            WHERE event_id = $1
            ORDER BY event_rank, reference_code"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| LigneDePilotage {
            id: l.id,
            reference_code: l.reference_code,
            event_id: l.event_id,
            call_id: l.call_id,
            organization_id: l.organization_id,
            organization_name: l.organization_name,
            title: l.title,
            title_text: l.title_text,
            status: l.status,
            submitted_at: l.submitted_at,
            weighted_score: l.weighted_score,
            average_score: l.average_score,
            is_knocked_out: l.is_knocked_out,
            review_count: l.review_count,
            required_reviews: l.required_reviews,
            reviews_missing: l.reviews_missing,
            assigned_reviewers: l.assigned_reviewers,
            open_change_requests: l.open_change_requests,
            speaker_count: l.speaker_count,
            event_rank: l.event_rank,
            format: l.format,
            activity_type_code: l.activity_type_code,
            organization_acronym: l.organization_acronym,
            organization_country_code: l.organization_country_code,
            organization_country: l.organization_country,
            co_organizer_count: l.co_organizer_count,
            theme_codes: l.theme_codes,
            themes: l.themes,
            reviewer_ids: l.reviewer_ids,
            reviewers: l.reviewers,
            overdue_reviews: l.overdue_reviews,
            next_review_due_at: l.next_review_due_at,
            read_count: l.read_count,
        })
        .collect())
}

/// Le **rang** d'un dossier dans son édition, tel que la vue le calcule.
///
/// On ne le recalcule pas : la ligne de `v_proposal_dashboard` porte déjà le
/// classement par note pondérée décroissante, et deux définitions du même rang
/// finiraient par diverger entre la liste et la fiche.
pub async fn rang<'e>(
    executor: impl PgExecutor<'e>,
    dossier: crate::domain::ids::ProposalId,
) -> Result<Option<i64>> {
    let rang = sqlx::query_scalar!(
        "SELECT event_rank FROM programme.v_proposal_dashboard WHERE id = $1",
        dossier.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(rang.flatten())
}
