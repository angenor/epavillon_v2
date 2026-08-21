//! Le dossier lui-même : sa création, son enregistrement, et le contexte que
//! le formulaire lit avant d'afficher quoi que ce soit.
//!
//! # Deux choses que la base ne fait pas, et que ce fichier fait
//!
//! **L'adresse d'URL.** `slug` est obligatoire et unique par édition, et le
//! contrat du formulaire ne la porte pas : le client ne peut pas la calculer,
//! il ignore les autres dossiers de l'édition. Elle est dérivée par
//! `platform.slugify()` — **en base**, pour que la normalisation reste celle de
//! toute la plateforme — puis repliée et **suffixée sur collision** (R5, écarts
//! n° 95 et n° 96).
//!
//! **La création TOUJOURS en brouillon.** Le garde d'état n'est posé que sur la
//! mise à jour de `status` : une insertion échappe à la machine, et un dossier
//! pourrait naître « retenu » (écart n° 96). L'état demandé par le client n'est
//! donc jamais lu à la création.
//!
//! # Le réessai passe par un point de reprise, et il le faut
//!
//! Une violation d'unicité **avorte la transaction** en PostgreSQL : sans
//! `SAVEPOINT`, le premier homonyme rendrait la transaction inutilisable et le
//! réessai échouerait sur « current transaction is aborted ». Chaque tentative
//! ouvre donc une transaction imbriquée, que SQLx traduit en point de reprise.

use kernel::error::{ApiError, Result};
use kernel::pg_error;
use sqlx::postgres::PgConnection;
use sqlx::{Connection, PgExecutor};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::{EventId, ProposalId};
use crate::domain::slug;

/// Ce qu'un enregistrement écrit, hors ce qui identifie le dossier.
///
/// **Le créneau voyage en heure murale**, découpé en date et heure, avec le
/// fuseau de l'édition : la conversion en instant se fait en base (R6).
pub struct ChampsDuDossier {
    pub title: serde_json::Value,
    pub summary: Option<serde_json::Value>,
    pub objectives: serde_json::Value,
    pub detailed_presentation: serde_json::Value,
    pub expected_outcomes: Option<serde_json::Value>,
    pub target_audiences: Vec<serde_json::Value>,
    pub format: String,
    pub activity_type_code: Option<String>,
    pub language_codes: Vec<String>,
    pub country_id: Option<Uuid>,
    /// `(date, heure)` en heure locale de l'édition, ou rien.
    pub creneau: Option<(String, String)>,
    /// `event.events.timezone` — lu sur l'édition du dossier, jamais sur la
    /// requête.
    pub fuseau: String,
    pub duration_minutes: Option<i16>,
    pub requested_sessions: i16,
    pub scheduling_constraints: Option<String>,
    pub contact_person_id: Option<Uuid>,
}

/// Ce qu'il faut de plus pour créer.
pub struct NouveauDossier {
    pub call_id: Uuid,
    pub event_id: EventId,
    pub organization_id: Uuid,
    pub submitted_by: Uuid,
    /// Le titre français **brut**, tel que le déposant l'a tapé : c'est lui que
    /// `platform.slugify()` normalise, pas le document multilingue.
    pub titre_brut: String,
}

/// Ce que rend un enregistrement — exactement `SaveDraftResult`.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Enregistrement {
    pub proposal_id: Uuid,
    /// Attribué par `tg_assign_reference_code` **à l'insertion**, donc dès le
    /// brouillon, et jamais réutilisé : le dossier porte le même numéro avant
    /// et après son dépôt.
    pub reference_code: String,
    #[serde(with = "time::serde::rfc3339")]
    pub saved_at: OffsetDateTime,
    pub status: String,
}

/// Créer un dossier — **toujours en brouillon**, avec son adresse dérivée.
pub async fn creer(
    conn: &mut PgConnection,
    nouveau: &NouveauDossier,
    champs: &ChampsDuDossier,
) -> Result<Enregistrement> {
    let base = base_dadresse(&mut *conn, &nouveau.titre_brut).await?;

    for tentative in 0..slug::TENTATIVES_MAX {
        let adresse = slug::tentative(&base, tentative);
        let mut point_de_reprise = conn.begin().await?;

        match inserer(&mut point_de_reprise, nouveau, champs, &adresse).await {
            Ok(ligne) => {
                point_de_reprise.commit().await?;
                return Ok(ligne);
            }
            Err(erreur) if collision_dadresse(&erreur) => {
                point_de_reprise.rollback().await?;
            }
            Err(erreur) => {
                point_de_reprise.rollback().await?;
                return Err(pg_error::translate(&erreur));
            }
        }
    }

    Err(ApiError::internal(format!(
        "adresse d'URL introuvable après {} tentatives sur « {base} »",
        slug::TENTATIVES_MAX
    )))
}

/// Mettre à jour un dossier **sans toucher à son état** : corriger n'est pas
/// déposer, et un dossier en évaluation ne repart pas au comité parce qu'on a
/// rectifié une faute de frappe.
///
/// **L'adresse suit le titre tant que le dossier est en brouillon, et se fige
/// au dépôt** : une adresse déjà communiquée ne doit pas changer sous une
/// correction de titre.
pub async fn mettre_a_jour(
    conn: &mut PgConnection,
    dossier: ProposalId,
    champs: &ChampsDuDossier,
    titre_brut: &str,
    refaire_ladresse: bool,
) -> Result<Enregistrement> {
    if !refaire_ladresse {
        return ecrire(&mut *conn, dossier, champs, None)
            .await
            .map_err(|e| pg_error::translate(&e));
    }

    let base = base_dadresse(&mut *conn, titre_brut).await?;

    for tentative in 0..slug::TENTATIVES_MAX {
        let adresse = slug::tentative(&base, tentative);
        let mut point_de_reprise = conn.begin().await?;

        match ecrire(&mut point_de_reprise, dossier, champs, Some(&adresse)).await {
            Ok(ligne) => {
                point_de_reprise.commit().await?;
                return Ok(ligne);
            }
            Err(erreur) if collision_dadresse(&erreur) => {
                point_de_reprise.rollback().await?;
            }
            Err(erreur) => {
                point_de_reprise.rollback().await?;
                return Err(pg_error::translate(&erreur));
            }
        }
    }

    Err(ApiError::internal(format!(
        "adresse d'URL introuvable après {} tentatives sur « {base} »",
        slug::TENTATIVES_MAX
    )))
}

/// Ce que le dossier porte et que le service doit connaître **avant** d'écrire.
pub struct EtatDuDossier {
    pub id: Uuid,
    pub reference_code: String,
    pub event_id: Uuid,
    pub call_id: Option<Uuid>,
    pub organization_id: Uuid,
    pub submitted_by: Uuid,
    pub status: String,
}

pub async fn etat<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Option<EtatDuDossier>> {
    let ligne = sqlx::query!(
        r#"SELECT id, reference_code, event_id, call_id, organization_id, submitted_by,
                  status::text AS "status!"
             FROM programme.proposals
            WHERE id = $1 AND deleted_at IS NULL"#,
        dossier.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| EtatDuDossier {
        id: l.id,
        reference_code: l.reference_code,
        event_id: l.event_id,
        call_id: l.call_id,
        organization_id: l.organization_id,
        submitted_by: l.submitted_by,
        status: l.status,
    }))
}

/// Le brouillon en cours d'une personne, pour reprendre où elle s'est arrêtée.
///
/// **Le plus récent, et un seul** : rien n'interdit à une personne d'avoir deux
/// brouillons — deux dossiers pour deux organisations —, et le contrat n'en
/// rend qu'un. Prendre le plus récent est le seul choix qui corresponde à
/// « reprendre où elle s'est arrêtée ».
pub async fn brouillon_en_cours<'e>(
    executor: impl PgExecutor<'e>,
    personne: Uuid,
) -> Result<Option<Enregistrement>> {
    let ligne = sqlx::query!(
        r#"SELECT id, reference_code, updated_at, status::text AS "status!"
             FROM programme.proposals
            WHERE submitted_by = $1 AND status = 'draft' AND deleted_at IS NULL
            ORDER BY updated_at DESC
            LIMIT 1"#,
        personne
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| Enregistrement {
        proposal_id: l.id,
        reference_code: l.reference_code,
        saved_at: l.updated_at,
        status: l.status,
    }))
}

/// Ce que l'écran charge avant d'afficher quoi que ce soit — `ProposalFormContext`.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct ContexteDuFormulaire {
    /// Nul quand aucune édition n'ouvre de dépôt : l'écran l'annonce et
    /// s'arrête.
    pub call_id: Option<Uuid>,
    pub event_id: Option<Uuid>,
    /// Dossiers déjà comptés dans le plafond, **ce brouillon exclu**.
    pub counted_proposals: i64,
}

/// L'appel réellement ouvert de la plateforme, et le décompte du plafond.
///
/// **Il y en a au plus un** : `ux_calls_one_per_event` borne à un appel par
/// édition, et le formulaire ne choisit pas son édition — il dépose sur celle
/// qui reçoit. `event.is_call_open()` tranche par le **statut ET la fenêtre** :
/// un appel marqué ouvert dont l'échéance est passée n'accepte rien, et le dire
/// avant sept étapes de saisie vaut mieux qu'après.
pub async fn contexte_du_formulaire<'e>(
    executor: impl PgExecutor<'e>,
    organisations: &[Uuid],
    brouillon_exclu: Option<ProposalId>,
) -> Result<ContexteDuFormulaire> {
    let ligne = sqlx::query!(
        r#"WITH ouvert AS (
               SELECT c.id, c.event_id
                 FROM event.calls_for_proposals c
                WHERE event.is_call_open(c.id)
                ORDER BY c.opens_at DESC
                LIMIT 1
           )
           SELECT o.id AS "call_id?", o.event_id AS "event_id?",
                  (SELECT count(*) FROM programme.proposals p
                    WHERE p.call_id = o.id
                      AND p.organization_id = ANY($1)
                      AND p.id IS DISTINCT FROM $2
                      AND p.status NOT IN ('draft', 'withdrawn', 'rejected')
                      AND p.deleted_at IS NULL) AS "comptes!"
             FROM ouvert o"#,
        organisations,
        brouillon_exclu.map(ProposalId::as_uuid)
    )
    .fetch_optional(executor)
    .await?;

    Ok(match ligne {
        None => ContexteDuFormulaire {
            call_id: None,
            event_id: None,
            counted_proposals: 0,
        },
        Some(l) => ContexteDuFormulaire {
            call_id: l.call_id,
            event_id: l.event_id,
            counted_proposals: l.comptes,
        },
    })
}

// -----------------------------------------------------------------------------
// Ce qui n'a pas à sortir de ce fichier
// -----------------------------------------------------------------------------

/// La base d'adresse : `platform.slugify()` **en base**, repliée par le domaine.
///
/// La normalisation reste celle de toute la plateforme — accents enlevés,
/// ponctuation retirée, selon les règles de PostgreSQL. La réécrire en Rust
/// produirait deux normalisations divergentes du même texte.
async fn base_dadresse(conn: &mut PgConnection, titre: &str) -> Result<String> {
    let brut = sqlx::query_scalar!("SELECT platform.slugify($1)", titre)
        .fetch_one(conn)
        .await?;

    Ok(slug::base(brut.as_deref()))
}

fn collision_dadresse(erreur: &sqlx::Error) -> bool {
    pg_error::sqlstate(erreur).as_deref() == Some("23505")
        && pg_error::constraint(erreur) == Some("ux_proposals_slug")
}

/// L'insertion, **sans `status`** : la colonne garde son défaut `'draft'`.
async fn inserer(
    conn: &mut PgConnection,
    nouveau: &NouveauDossier,
    champs: &ChampsDuDossier,
    adresse: &str,
) -> std::result::Result<Enregistrement, sqlx::Error> {
    let (date, heure) = découper(champs);

    let ligne = sqlx::query!(
        r#"WITH creneau AS (
               SELECT CASE WHEN $17::text IS NULL THEN NULL
                           ELSE ($17 || ' ' || $18)::timestamp AT TIME ZONE $19 END AS debut
           )
           INSERT INTO programme.proposals
               (call_id, event_id, organization_id, submitted_by, contact_person_id,
                title, slug, summary, objectives, detailed_presentation, expected_outcomes,
                target_audiences, format, activity_type_code, language_codes, country_id,
                preferred_start_at, preferred_end_at,
                duration_minutes, requested_sessions, scheduling_constraints)
           SELECT $1, $2, $3, $4, $5,
                  $6::jsonb::platform.i18n_text, $7::text::platform.slug,
                  $8::jsonb::platform.i18n_text, $9::jsonb::platform.i18n_text,
                  $10::jsonb::platform.i18n_text, $11::jsonb::platform.i18n_text,
                  $12::jsonb[]::platform.i18n_text[],
                  $13::text::event.participation_mode, $14, $15, $16,
                  c.debut, c.debut + make_interval(mins => $20::int),
                  $20, $21, $22
             FROM creneau c
        RETURNING id, reference_code, updated_at, status::text AS "status!""#,
        nouveau.call_id,
        nouveau.event_id.as_uuid(),
        nouveau.organization_id,
        nouveau.submitted_by,
        champs.contact_person_id,
        champs.title,
        adresse,
        champs.summary,
        champs.objectives,
        champs.detailed_presentation,
        champs.expected_outcomes,
        &champs.target_audiences,
        champs.format,
        champs.activity_type_code,
        &champs.language_codes,
        champs.country_id,
        date,
        heure,
        champs.fuseau,
        champs.duration_minutes.map(i32::from),
        champs.requested_sessions,
        champs.scheduling_constraints,
    )
    .fetch_one(conn)
    .await?;

    Ok(Enregistrement {
        proposal_id: ligne.id,
        reference_code: ligne.reference_code,
        saved_at: ligne.updated_at,
        status: ligne.status,
    })
}

/// La mise à jour, **sans `status`** : la colonne n'est pas dans le `SET`, et
/// c'est ce qui garantit que le garde d'état n'est pas réveillé.
async fn ecrire(
    conn: &mut PgConnection,
    dossier: ProposalId,
    champs: &ChampsDuDossier,
    adresse: Option<&str>,
) -> std::result::Result<Enregistrement, sqlx::Error> {
    let (date, heure) = découper(champs);

    let ligne = sqlx::query!(
        r#"WITH creneau AS (
               SELECT CASE WHEN $16::text IS NULL THEN NULL
                           ELSE ($16 || ' ' || $17)::timestamp AT TIME ZONE $18 END AS debut
           )
           UPDATE programme.proposals p
              SET contact_person_id = $2,
                  title = $3::jsonb::platform.i18n_text,
                  slug = COALESCE($4::text::platform.slug, p.slug),
                  summary = $5::jsonb::platform.i18n_text,
                  objectives = $6::jsonb::platform.i18n_text,
                  detailed_presentation = $7::jsonb::platform.i18n_text,
                  expected_outcomes = $8::jsonb::platform.i18n_text,
                  target_audiences = $9::jsonb[]::platform.i18n_text[],
                  format = $10::text::event.participation_mode,
                  activity_type_code = $11,
                  language_codes = $12,
                  country_id = $13,
                  preferred_start_at = c.debut,
                  preferred_end_at = c.debut + make_interval(mins => $19::int),
                  duration_minutes = $19,
                  requested_sessions = $14,
                  scheduling_constraints = $15
             FROM creneau c
            WHERE p.id = $1 AND p.deleted_at IS NULL
        RETURNING p.id, p.reference_code, p.updated_at, p.status::text AS "status!""#,
        dossier.as_uuid(),
        champs.contact_person_id,
        champs.title,
        adresse,
        champs.summary,
        champs.objectives,
        champs.detailed_presentation,
        champs.expected_outcomes,
        &champs.target_audiences,
        champs.format,
        champs.activity_type_code,
        &champs.language_codes,
        champs.country_id,
        champs.requested_sessions,
        champs.scheduling_constraints,
        date,
        heure,
        champs.fuseau,
        champs.duration_minutes.map(i32::from),
    )
    .fetch_one(conn)
    .await?;

    Ok(Enregistrement {
        proposal_id: ligne.id,
        reference_code: ligne.reference_code,
        saved_at: ligne.updated_at,
        status: ligne.status,
    })
}

fn découper(champs: &ChampsDuDossier) -> (Option<&str>, Option<&str>) {
    match &champs.creneau {
        Some((date, heure)) => (Some(date.as_str()), Some(heure.as_str())),
        None => (None, None),
    }
}

// -----------------------------------------------------------------------------
// La fiche complète — `Proposal`, telle que la table la porte
// -----------------------------------------------------------------------------

/// Le dossier entier. **La colonne de recherche est omise** : `search_vector`
/// est engendrée, aucune route de recherche plein texte n'est au contrat, et
/// un `tsvector` n'a rien à faire dans une charge utile JSON.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Fiche {
    pub id: Uuid,
    pub reference_code: String,
    pub call_id: Option<Uuid>,
    pub event_id: Uuid,
    pub organization_id: Uuid,
    pub submitted_by: Uuid,
    pub contact_person_id: Option<Uuid>,
    pub title: serde_json::Value,
    pub slug: String,
    pub summary: Option<serde_json::Value>,
    pub objectives: serde_json::Value,
    pub detailed_presentation: serde_json::Value,
    pub expected_outcomes: Option<serde_json::Value>,
    pub target_audiences: Vec<serde_json::Value>,
    pub format: String,
    pub activity_type_code: Option<String>,
    pub language_codes: Vec<String>,
    pub country_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub preferred_start_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub preferred_end_at: Option<OffsetDateTime>,
    pub duration_minutes: Option<i16>,
    pub requested_sessions: i16,
    pub scheduling_constraints: Option<String>,
    pub status: String,
    #[serde(with = "time::serde::rfc3339::option")]
    pub submitted_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub decided_at: Option<OffsetDateTime>,
    /// **Le motif de la DERNIÈRE transition, et rien de plus.** Une transition
    /// suivante l'écrase, et une transition sans motif l'efface (écart n° 97) :
    /// un écran qui cherche « pourquoi ce dossier a-t-il été renvoyé » lit le
    /// journal, pas cette colonne.
    pub decision_reason: Option<String>,
    pub decided_by: Option<Uuid>,
    pub average_score: Option<f64>,
    pub weighted_score: Option<f64>,
    pub review_count: i16,
    pub is_knocked_out: bool,
    pub view_count: i32,
    #[serde(with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
    pub deleted_by: Option<Uuid>,
    pub deleted_reason: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Un dossier, **effacé exclu**. La suppression est logique, mais rien n'y
/// donne plus accès : un dossier effacé se refuse comme un inexistant.
///
/// La liste de colonnes est écrite deux fois — ici et dans
/// `de_lorganisation` — parce que `sqlx::query!` exige une chaîne littérale
/// pour vérifier la requête à la compilation. C'est le prix de la
/// vérification, et il est explicite plutôt que contourné.
pub async fn fiche<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Option<Fiche>> {
    let ligne = sqlx::query!(
        r#"SELECT id, reference_code, call_id, event_id, organization_id, submitted_by,
                  contact_person_id, title, slug::text AS "slug!", summary, objectives,
                  detailed_presentation, expected_outcomes,
                  target_audiences::jsonb[] AS "target_audiences!",
                  format::text AS "format!", activity_type_code, language_codes,
                  country_id, preferred_start_at, preferred_end_at, duration_minutes,
                  requested_sessions, scheduling_constraints,
                  status::text AS "status!", submitted_at, decided_at, decision_reason,
                  decided_by, average_score::float8, weighted_score::float8,
                  review_count, is_knocked_out, view_count,
                  deleted_at, deleted_by, deleted_reason, created_at, updated_at
             FROM programme.proposals
            WHERE id = $1 AND deleted_at IS NULL"#,
        dossier.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| Fiche {
        id: l.id,
        reference_code: l.reference_code,
        call_id: l.call_id,
        event_id: l.event_id,
        organization_id: l.organization_id,
        submitted_by: l.submitted_by,
        contact_person_id: l.contact_person_id,
        title: l.title,
        slug: l.slug,
        summary: l.summary,
        objectives: l.objectives,
        detailed_presentation: l.detailed_presentation,
        expected_outcomes: l.expected_outcomes,
        target_audiences: l.target_audiences,
        format: l.format,
        activity_type_code: l.activity_type_code,
        language_codes: l.language_codes,
        country_id: l.country_id,
        preferred_start_at: l.preferred_start_at,
        preferred_end_at: l.preferred_end_at,
        duration_minutes: l.duration_minutes,
        requested_sessions: l.requested_sessions,
        scheduling_constraints: l.scheduling_constraints,
        status: l.status,
        submitted_at: l.submitted_at,
        decided_at: l.decided_at,
        decision_reason: l.decision_reason,
        decided_by: l.decided_by,
        average_score: l.average_score,
        weighted_score: l.weighted_score,
        review_count: l.review_count,
        is_knocked_out: l.is_knocked_out,
        view_count: l.view_count,
        deleted_at: l.deleted_at,
        deleted_by: l.deleted_by,
        deleted_reason: l.deleted_reason,
        created_at: l.created_at,
        updated_at: l.updated_at,
    }))
}

/// Les dossiers d'une organisation **porteuse**, brouillons compris.
///
/// `organization_id` désigne le porteur principal ; une organisation
/// simplement co-organisatrice ne figure pas ici, et c'est la colonne que le
/// modèle tient en cohérence avec la ligne de rôle `lead`.
///
/// `editions` borne la lecture au périmètre d'administration quand l'accès
/// vient du back-office. Il est **nul** pour un membre de l'organisation, qui
/// voit alors tous les dossiers de la sienne : une organisation n'administre
/// rien, son accès n'est pas un périmètre.
pub async fn de_lorganisation<'e>(
    executor: impl PgExecutor<'e>,
    organisation: Uuid,
    editions: Option<&[Uuid]>,
) -> Result<Vec<Fiche>> {
    let lignes = sqlx::query!(
        r#"SELECT id, reference_code, call_id, event_id, organization_id, submitted_by,
                  contact_person_id, title, slug::text AS "slug!", summary, objectives,
                  detailed_presentation, expected_outcomes,
                  target_audiences::jsonb[] AS "target_audiences!",
                  format::text AS "format!", activity_type_code, language_codes,
                  country_id, preferred_start_at, preferred_end_at, duration_minutes,
                  requested_sessions, scheduling_constraints,
                  status::text AS "status!", submitted_at, decided_at, decision_reason,
                  decided_by, average_score::float8, weighted_score::float8,
                  review_count, is_knocked_out, view_count,
                  deleted_at, deleted_by, deleted_reason, created_at, updated_at
             FROM programme.proposals
            WHERE organization_id = $1
              AND deleted_at IS NULL
              AND ($2::uuid[] IS NULL OR event_id = ANY($2::uuid[]))
            ORDER BY created_at DESC, reference_code"#,
        organisation,
        editions
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Fiche {
            id: l.id,
            reference_code: l.reference_code,
            call_id: l.call_id,
            event_id: l.event_id,
            organization_id: l.organization_id,
            submitted_by: l.submitted_by,
            contact_person_id: l.contact_person_id,
            title: l.title,
            slug: l.slug,
            summary: l.summary,
            objectives: l.objectives,
            detailed_presentation: l.detailed_presentation,
            expected_outcomes: l.expected_outcomes,
            target_audiences: l.target_audiences,
            format: l.format,
            activity_type_code: l.activity_type_code,
            language_codes: l.language_codes,
            country_id: l.country_id,
            preferred_start_at: l.preferred_start_at,
            preferred_end_at: l.preferred_end_at,
            duration_minutes: l.duration_minutes,
            requested_sessions: l.requested_sessions,
            scheduling_constraints: l.scheduling_constraints,
            status: l.status,
            submitted_at: l.submitted_at,
            decided_at: l.decided_at,
            decision_reason: l.decision_reason,
            decided_by: l.decided_by,
            average_score: l.average_score,
            weighted_score: l.weighted_score,
            review_count: l.review_count,
            is_knocked_out: l.is_knocked_out,
            view_count: l.view_count,
            deleted_at: l.deleted_at,
            deleted_by: l.deleted_by,
            deleted_reason: l.deleted_reason,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}
