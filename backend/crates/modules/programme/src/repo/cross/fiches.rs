//! Les lectures hors schéma **d'affichage** — celles que la fiche
//! d'évaluation compose et que rien ne décide.
//!
//! # Pourquoi elles sont à part, et pourquoi elles restent dans `cross`
//!
//! La règle du dossier parent est inchangée : **toutes** les lectures hors du
//! schéma `programme` vivent sous `cross`, pour qu'un ajout se discute au lieu
//! de se disperser. Le découpage en deux fichiers ne relâche rien — même
//! espace de noms, même inventaire — et répond au garde-fou de mille lignes de
//! `CLAUDE.md` : le fichier unique le franchissait à l'arrivée de la fiche
//! d'évaluation.
//!
//! La ligne de partage est claire : `mod.rs` porte ce qui **décide** — la
//! résolution d'ascendance, l'état de l'appel, l'adhésion, les bornes, la
//! grille —, ce fichier porte ce qui **s'affiche**. Une garde n'a pas besoin de
//! vingt-six colonnes ; un en-tête ne se contente pas de quatre.
//!
//! **Aucune ligne de ce fichier n'écrit**, comme dans `mod.rs`.

use kernel::error::Result;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::{EventId, ProposalId};

// -----------------------------------------------------------------------------
// 1 bis. L'édition entière — ce que la fiche d'évaluation affiche en en-tête
// -----------------------------------------------------------------------------

/// L'édition telle que le contrat du front la décrit — `EventEdition`.
///
/// `ContexteEdition` ne porte que ce qui **décide** (fuseau, ville, fin,
/// statut) ; celle-ci porte ce qui **s'affiche**. Les deux coexistent parce
/// qu'une garde n'a pas besoin de vingt colonnes, et qu'un en-tête ne se
/// contente pas de quatre.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct FicheEdition {
    pub id: Uuid,
    pub series_id: Option<Uuid>,
    pub edition_label: Option<String>,
    pub edition_year: i16,
    pub title: serde_json::Value,
    pub acronym: Option<String>,
    pub slug: String,
    pub description: serde_json::Value,
    pub status: String,
    pub participation_mode: String,
    pub timezone: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub has_pavilion: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub programme_published_at: Option<OffsetDateTime>,
    pub highlights: Option<serde_json::Value>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

pub async fn fiche_edition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Option<FicheEdition>> {
    let ligne = sqlx::query!(
        r#"SELECT id, series_id, edition_label, edition_year, title, acronym,
                  slug::text AS "slug!", description, status::text AS "status!",
                  participation_mode::text AS "participation_mode!",
                  timezone::text AS "timezone!", starts_at, ends_at,
                  country_id, city, address,
                  latitude::float8, longitude::float8, has_pavilion,
                  programme_published_at, highlights, created_by,
                  created_at, updated_at
             FROM event.events WHERE id = $1"#,
        event_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| FicheEdition {
        id: l.id,
        series_id: l.series_id,
        edition_label: l.edition_label,
        edition_year: l.edition_year,
        title: l.title,
        acronym: l.acronym,
        slug: l.slug,
        description: l.description,
        status: l.status,
        participation_mode: l.participation_mode,
        timezone: l.timezone,
        starts_at: l.starts_at,
        ends_at: l.ends_at,
        country_id: l.country_id,
        city: l.city,
        address: l.address,
        latitude: l.latitude,
        longitude: l.longitude,
        has_pavilion: l.has_pavilion,
        programme_published_at: l.programme_published_at,
        highlights: l.highlights,
        created_by: l.created_by,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }))
}

/// L'appel tel que le contrat le décrit — `CallForProposals`.
///
/// **`ReglesDeLAppel` reste distincte** : elle porte ce que le service
/// applique, celle-ci ce que l'écran affiche. Fusionner les deux ferait
/// charger vingt-six colonnes pour vérifier un nombre d'intervenants.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct FicheAppel {
    pub id: Uuid,
    pub event_id: Uuid,
    pub code: String,
    pub title: serde_json::Value,
    pub description: Option<serde_json::Value>,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub opens_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub closes_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub extended_until: Option<OffsetDateTime>,
    /// Les dates et heures sans fuseau traversent en `text`, patron de B3 :
    /// `time::Date` et `time::Time` n'ont pas de représentation JSON convenue,
    /// et l'heure quotidienne d'un appel se lit dans le fuseau de l'édition,
    /// jamais dans celui du lecteur.
    pub results_expected_at: Option<String>,
    pub max_proposals_per_organization: Option<i16>,
    pub requires_verified_organization: bool,
    pub min_speakers: i16,
    pub max_speakers: i16,
    pub default_duration_minutes: i16,
    pub min_duration_minutes: i16,
    pub max_duration_minutes: i16,
    pub daily_start_time: String,
    pub daily_end_time: String,
    pub allowed_formats: Vec<String>,
    pub required_reviews: i16,
    pub blind_review: bool,
    pub guidelines_url: Option<String>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

pub async fn fiche_appel<'e>(
    executor: impl PgExecutor<'e>,
    call_id: Uuid,
) -> Result<Option<FicheAppel>> {
    let ligne = sqlx::query!(
        r#"SELECT id, event_id, code, title, description, status::text AS "status!",
                  opens_at, closes_at, extended_until,
                  results_expected_at::text AS "results_expected_at_texte",
                  max_proposals_per_organization, requires_verified_organization,
                  min_speakers, max_speakers, default_duration_minutes,
                  min_duration_minutes, max_duration_minutes,
                  daily_start_time::text AS "daily_start_time!",
                  daily_end_time::text AS "daily_end_time!",
                  allowed_formats::text[] AS "allowed_formats!",
                  required_reviews, blind_review,
                  guidelines_url::text, created_by, created_at, updated_at
             FROM event.calls_for_proposals WHERE id = $1"#,
        call_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| FicheAppel {
        id: l.id,
        event_id: l.event_id,
        code: l.code,
        title: l.title,
        description: l.description,
        status: l.status,
        opens_at: l.opens_at,
        closes_at: l.closes_at,
        extended_until: l.extended_until,
        results_expected_at: l.results_expected_at_texte,
        max_proposals_per_organization: l.max_proposals_per_organization,
        requires_verified_organization: l.requires_verified_organization,
        min_speakers: l.min_speakers,
        max_speakers: l.max_speakers,
        default_duration_minutes: l.default_duration_minutes,
        min_duration_minutes: l.min_duration_minutes,
        max_duration_minutes: l.max_duration_minutes,
        daily_start_time: l.daily_start_time,
        daily_end_time: l.daily_end_time,
        allowed_formats: l.allowed_formats,
        required_reviews: l.required_reviews,
        blind_review: l.blind_review,
        guidelines_url: l.guidelines_url,
        created_by: l.created_by,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }))
}

/// La fiche d'une organisation telle que la fiche d'évaluation l'affiche —
/// `Organization`, réduit à ce que ce module montre.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct OrganisationAffichee {
    pub id: Uuid,
    pub legal_name: String,
    pub acronym: Option<String>,
    pub slug: String,
    pub organization_type_code: String,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
}

/// Les fiches d'un lot d'organisations, **en une requête**.
pub async fn organisations_affichees<'e>(
    executor: impl PgExecutor<'e>,
    ids: &[Uuid],
) -> Result<Vec<OrganisationAffichee>> {
    let lignes = sqlx::query!(
        r#"SELECT id, legal_name, acronym, slug::text AS "slug!",
                  organization_type_code, country_id, city,
                  website::text, status::text AS "status!", verified_at
             FROM org.organizations
            WHERE id = ANY($1)"#,
        ids
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OrganisationAffichee {
            id: l.id,
            legal_name: l.legal_name,
            acronym: l.acronym,
            slug: l.slug,
            organization_type_code: l.organization_type_code,
            country_id: l.country_id,
            city: l.city,
            website: l.website,
            status: l.status,
            verified_at: l.verified_at,
        })
        .collect())
}

/// **L'historique de participation, RÉDUIT aux colonnes de la fiche** (écart
/// n° 54).
///
/// La projection en porte une quarantaine — membres, articles, octets stockés,
/// score de confiance — qui appartiennent à la fiche d'organisation et n'ont
/// rien à faire dans un panneau latéral d'évaluation. Neuf suffisent à
/// répondre à la seule question que le comité se pose : cette organisation
/// est-elle une inconnue, une habituée, ou quelqu'un dont trois dossiers ont
/// déjà été écartés ?
///
/// **`ratio_acceptation` est nul et non zéro** quand rien n'a jamais été
/// déposé : la vue le pose ainsi, et un zéro se lirait « jamais retenue », ce
/// qui est un contresens.
///
/// **Ici la note obtenue sort**, contrairement à `historique_organisation` :
/// cette lecture-là sert le **comité**, l'autre sert l'organisation.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct AnteriorAffiche {
    pub organization_id: Uuid,
    pub propositions_deposees: i64,
    pub propositions_acceptees: i64,
    pub propositions_rejetees: i64,
    pub evenements_couverts: i64,
    pub sessions_realisees: i64,
    pub note_moyenne_obtenue: Option<f64>,
    pub ratio_acceptation: Option<f64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub derniere_proposition: Option<OffsetDateTime>,
}

pub async fn anteriors_affiches<'e>(
    executor: impl PgExecutor<'e>,
    ids: &[Uuid],
) -> Result<Vec<AnteriorAffiche>> {
    // `derniere_proposition` n'est pas une colonne de la projection : la vue
    // porte `derniere_activite`, qui agrège bien plus qu'un dépôt. On lit donc
    // la date du dernier dossier là où elle est vraie — sur les dossiers.
    let lignes = sqlx::query!(
        r#"SELECT s.organization_id AS "organization_id!",
                  s.propositions_deposees AS "deposees!",
                  s.propositions_acceptees AS "acceptees!",
                  s.propositions_rejetees AS "rejetees!",
                  s.evenements_couverts AS "evenements!",
                  s.sessions_realisees AS "sessions!",
                  s.note_moyenne_obtenue::float8,
                  s.ratio_acceptation::float8,
                  (SELECT max(p.created_at) FROM programme.proposals p
                    WHERE p.organization_id = s.organization_id
                      AND p.deleted_at IS NULL) AS "derniere_proposition?"
             FROM analytics.mv_organization_scorecard s
            WHERE s.organization_id = ANY($1)"#,
        ids
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| AnteriorAffiche {
            organization_id: l.organization_id,
            propositions_deposees: l.deposees,
            propositions_acceptees: l.acceptees,
            propositions_rejetees: l.rejetees,
            evenements_couverts: l.evenements,
            sessions_realisees: l.sessions,
            note_moyenne_obtenue: l.note_moyenne_obtenue,
            ratio_acceptation: l.ratio_acceptation,
            derniere_proposition: l.derniere_proposition,
        })
        .collect())
}

// -----------------------------------------------------------------------------
// 11. L'historique champ par champ
// -----------------------------------------------------------------------------

/// Une ligne de `programme.proposal_history()` — `ProposalHistoryEntry`.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct EntreeDHistorique {
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    pub actor_id: Option<Uuid>,
    pub actor_label: Option<String>,
    pub action: String,
    pub field: Option<String>,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

/// L'historique d'un dossier, **par la fonction du module**.
///
/// Elle écarte déjà les colonnes techniques — date de mise à jour, vecteur de
/// recherche, compteur de vues —, et la refaire ici ferait apparaître dans
/// l'historique une modification à chaque affichage.
pub async fn historique_du_dossier<'e>(
    executor: impl PgExecutor<'e>,
    proposal_id: ProposalId,
) -> Result<Vec<EntreeDHistorique>> {
    let lignes = sqlx::query!(
        r#"SELECT occurred_at AS "occurred_at!", actor_id, actor_label,
                  action AS "action!", field, old_value, new_value
             FROM programme.proposal_history($1)"#,
        proposal_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| EntreeDHistorique {
            occurred_at: l.occurred_at,
            actor_id: l.actor_id,
            actor_label: l.actor_label,
            action: l.action,
            field: l.field,
            old_value: l.old_value,
            new_value: l.new_value,
        })
        .collect())
}

/// Une personne **telle que la fiche d'évaluation la nomme**.
///
/// **Ni adresse électronique, ni téléphone** : le comité lit un dossier, il ne
/// démarche pas les intervenants. `FichePersonne` porte l'adresse parce que le
/// rapprochement d'un intervenant se fait par elle ; cette lecture-ci sert
/// l'affichage, et ce qui n'est pas envoyé ne peut pas fuiter.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct PersonneAffichee {
    pub id: Uuid,
    pub display_name: String,
    pub civility: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub job_title: Option<String>,
    pub primary_organization_id: Option<Uuid>,
}

/// Les personnes d'un lot, **en une requête** : les résoudre une par une est le
/// N+1 que la composition de la fiche existe pour éviter.
pub async fn personnes_affichees<'e>(
    executor: impl PgExecutor<'e>,
    ids: &[Uuid],
) -> Result<Vec<PersonneAffichee>> {
    let lignes = sqlx::query!(
        r#"SELECT id, display_name AS "display_name!", civility,
                  first_name, last_name, job_title, primary_organization_id
             FROM identity.people
            WHERE id = ANY($1)
            ORDER BY display_name"#,
        ids
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PersonneAffichee {
            id: l.id,
            display_name: l.display_name,
            civility: l.civility,
            first_name: l.first_name,
            last_name: l.last_name,
            job_title: l.job_title,
            primary_organization_id: l.primary_organization_id,
        })
        .collect())
}

// -----------------------------------------------------------------------------
// L'espace organisation — ce qu'il lit hors de son schéma
// -----------------------------------------------------------------------------

/// Une organisation **entière**, telle que son propre espace l'affiche.
///
/// `OrganisationAffichee` sert la fiche du comité et s'arrête à ce qu'on montre
/// d'un tiers ; celle-ci sert l'organisation elle-même, qui a le droit de lire
/// sa propre fiche en entier.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct FicheOrganisationComplete {
    pub id: Uuid,
    pub legal_name: String,
    pub acronym: Option<String>,
    pub slug: String,
    pub organization_type_code: String,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub description: Option<serde_json::Value>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
    pub contact_phone: Option<String>,
    pub status: String,
    pub merged_into_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub merged_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    pub trust_score: i16,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

pub async fn fiche_organisation_complete<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: Uuid,
) -> Result<Option<FicheOrganisationComplete>> {
    let ligne = sqlx::query!(
        r#"SELECT id, legal_name, acronym, slug::text AS "slug!",
                  organization_type_code, country_id, city, description,
                  website::text, contact_email::text, contact_phone,
                  status::text AS "status!", merged_into_id, merged_at,
                  verified_at, trust_score, created_by, created_at, updated_at
             FROM org.organizations WHERE id = $1"#,
        organization_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| FicheOrganisationComplete {
        id: l.id,
        legal_name: l.legal_name,
        acronym: l.acronym,
        slug: l.slug,
        organization_type_code: l.organization_type_code,
        country_id: l.country_id,
        city: l.city,
        description: l.description,
        website: l.website,
        contact_email: l.contact_email,
        contact_phone: l.contact_phone,
        status: l.status,
        merged_into_id: l.merged_into_id,
        merged_at: l.merged_at,
        verified_at: l.verified_at,
        trust_score: l.trust_score,
        created_by: l.created_by,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }))
}

/// Une ligne d'adhésion — `Membership`.
///
/// **À ne pas confondre avec `domain::ownership::Adhesion`**, qui ne porte que
/// ce qui décide : « cette personne peut-elle écrire au nom de l'organisation ? »
/// Celle-ci s'affiche, l'autre tranche.
///
/// **Les deux colonnes d'invitation portent une DIRECTION**, et les confondre
/// ferait approuver une adhésion que l'intéressé n'a jamais acceptée :
/// renseignées, l'organisation a invité et c'est la personne qui doit répondre ;
/// nulles, la personne a demandé et c'est un référent qui doit trancher.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct LigneDAdhesion {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub person_id: Uuid,
    pub role: String,
    pub status: String,
    pub is_primary: bool,
    pub job_title: Option<String>,
    pub invited_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub invited_at: Option<OffsetDateTime>,
    pub approved_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub approved_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Les adhésions d'une organisation, **révoquées exclues**.
///
/// C'est un élargissement de la lecture n° 6 — « cette personne peut-elle
/// écrire au nom de l'organisation ? » devient « qui le peut ? » —, et non une
/// porte nouvelle : le module Organisations n'expose aucune route de liste de
/// membres, et l'espace organisation la demande. À reprendre par B2 si une
/// telle route naît (écart n° 107).
pub async fn adhesions_de_lorganisation<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: Uuid,
) -> Result<Vec<LigneDAdhesion>> {
    let lignes = sqlx::query!(
        r#"SELECT m.id, m.organization_id, m.person_id, m.role::text AS "role!",
                  m.status::text AS "status!", m.is_primary, m.job_title,
                  m.invited_by, m.invited_at, m.approved_by, m.approved_at,
                  m.revoked_at, m.created_at, m.updated_at
             FROM org.memberships m
             JOIN identity.people p ON p.id = m.person_id
            WHERE m.organization_id = $1 AND m.revoked_at IS NULL
            ORDER BY (m.status = 'pending') DESC, p.display_name"#,
        organization_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| LigneDAdhesion {
            id: l.id,
            organization_id: l.organization_id,
            person_id: l.person_id,
            role: l.role,
            status: l.status,
            is_primary: l.is_primary,
            job_title: l.job_title,
            invited_by: l.invited_by,
            invited_at: l.invited_at,
            approved_by: l.approved_by,
            approved_at: l.approved_at,
            revoked_at: l.revoked_at,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}

/// **L'appel réellement ouvert de la plateforme**, et son édition.
///
/// Il y en a au plus un : `ux_calls_one_per_event` borne à un appel par
/// édition, et `event.is_call_open()` tranche par le statut **et** la fenêtre —
/// un appel marqué ouvert dont l'échéance est passée n'accepte rien.
pub async fn appel_ouvert_de_la_plateforme<'e>(
    executor: impl PgExecutor<'e>,
) -> Result<Option<(Uuid, Uuid)>> {
    let ligne = sqlx::query!(
        "SELECT id, event_id FROM event.calls_for_proposals
          WHERE event.is_call_open(id)
          ORDER BY opens_at DESC LIMIT 1"
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| (l.id, l.event_id)))
}

/// Les éditions sur lesquelles une organisation a déposé.
///
/// C'est ce que `GET /organizations/{id}/editions` rend : une organisation
/// fidèle en a plusieurs, et sa liste de dossiers les groupe — un dossier de la
/// COP30 ne se lit pas comme un dossier en cours.
pub async fn editions_de_lorganisation<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: Uuid,
) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        "SELECT DISTINCT p.event_id
           FROM programme.proposals p
          WHERE p.organization_id = $1 AND p.deleted_at IS NULL",
        organization_id
    )
    .fetch_all(executor)
    .await?;

    Ok(ids)
}
