//! **Les seules lectures hors du schéma `programme`, et les résolutions
//! d'ascendance.**
//!
//! # La règle qui gouverne ce fichier
//!
//! Un module lit hors de son schéma quand la question porte sur **ses propres
//! entités** ; il n'y écrit jamais depuis ici. « Dans quel fuseau ce créneau
//! se lit-il ? » est une question de ce module, même si la réponse vit dans
//! `event`.
//!
//! Cette règle est facile à énoncer et facile à enfreindre par accroissement.
//! Dispersées dans treize dépôts, ces requêtes deviennent invisibles ; réunies,
//! elles se relisent en un fichier, et **c'est ici qu'un ajout se discute**.
//! C'est le patron de B3, repris sans être réinventé.
//!
//! # Les seize lectures hors schéma autorisées
//!
//! | # | Lecture | Question de **ce** module |
//! |---|---|---|
//! | 1 | `event.events` — fuseau, ville, période, état | « dans quel fuseau ce créneau se lit-il, cette édition est-elle terminée ? » |
//! | 2 | `event.calls_for_proposals` et `effective_deadline()` | « ce dépôt est-il recevable, jusqu'à quand, combien de revues, quel aveugle, quelles bornes ? » |
//! | 3 | `event.review_criteria` et `max_weighted_score()` | « quelle grille note ce dossier, et sur combien ? » |
//! | 4 | `event.call_reviewers` | « qui siège au comité de cet appel, et quelle est sa charge ? » |
//! | 5 | `org.organizations` | « qui porte ce dossier, et est-elle vérifiée ? » |
//! | 6 | `org.memberships` | « cette personne peut-elle écrire au nom de l'organisation ? » |
//! | 7 | `identity.people` | les intervenants, les auteurs de messages, les membres du comité |
//! | 8 | `reference.taxonomy_terms`, `terms_of()`, `term_badges()` | les thématiques, pour filtrer et pour afficher |
//! | 9 | `media.assets` et `object_url()` | les pièces du dossier et leur adresse |
//! | 10 | `analytics.mv_organization_scorecard` | l'historique de participation de l'organisation porteuse |
//! | 11 | `platform.entity_history()`, par `programme.proposal_history()` | l'historique champ par champ |
//! | 12 | `event.event_days` | « quelles colonnes de jours le planificateur affiche-t-il, et à quel jour cette séance se rattache-t-elle ? » |
//! | 13 | `event.rooms` | « où cette séance est-elle installée, et occupe-t-elle le stand ? » |
//! | 14 | `event.programme_tracks` | « quelles journées spéciales peut-on lui rattacher ? » |
//! | 15 | `event.broadcast_channels` | « quel canal cette séance occupe-t-elle ? » |
//! | 16 | `reference.countries.iso2`, `reference.taxonomy_terms.code` | les valeurs admises d'une réponse « pays » ou d'un champ adossé à une taxonomie |
//!
//! Les cinq dernières sont arrivées avec B5 et vivent dans `cross/grille.rs`,
//! **dans le même espace de noms** : le découpage est un fait de fichier — le
//! garde-fou de mille lignes —, pas de frontière.
//!
//! Le périmètre d'administration ne figure pas dans cette liste : il est lu par
//! le garde du noyau (`kernel::auth`), jamais par une requête d'ici.
//!
//! **Aucune ligne de ce fichier n'écrit.** Pas un `INSERT`, pas un `UPDATE`,
//! pas un `DELETE` — c'est vérifiable d'un coup d'œil, et c'est le second
//! intérêt du regroupement. Les **trois** écritures hors schéma vivent ailleurs,
//! chacune dans son fichier : `repo/themes.rs`, `repo/people.rs` et, depuis B5,
//! `repo/consents.rs`.

use kernel::error::Result;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::eligibility::{EtatDeLAppel, EtatDeLOrganisation};
use crate::domain::ids::{CommentId, EventId, ProposalId, ReviewId};
use crate::domain::ownership::Adhesion;

// -----------------------------------------------------------------------------
// Résolution d'ascendance — TROIS niveaux, et c'est la nuance de ce module
//
// L'ordre est imposé et ne se négocie pas : **résoudre l'ascendance, PUIS
// vérifier le périmètre, PUIS agir** (research.md § R13). Vérifier d'abord
// reviendrait à croire l'édition que le client annonce.
//
// Ces lectures ne divulguent rien : elles ne rendent qu'un identifiant
// d'édition, jamais exposé, et leur absence produit le MÊME refus que l'échec
// du périmètre. Un identifiant inexistant et un identifiant hors périmètre sont
// indiscernables par la forme de la réponse (principe IX).
// -----------------------------------------------------------------------------

/// L'édition d'un dossier. **Le dossier effacé est traité comme absent** : sa
/// suppression est logique, mais rien n'y donne plus accès.
pub async fn event_id_of_proposal<'e>(
    executor: impl PgExecutor<'e>,
    proposal_id: ProposalId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT event_id FROM programme.proposals
          WHERE id = $1 AND deleted_at IS NULL",
        proposal_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// L'édition d'un message — **deux niveaux** : le message appartient à un
/// dossier, qui appartient à une édition.
pub async fn event_id_of_comment<'e>(
    executor: impl PgExecutor<'e>,
    comment_id: CommentId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT p.event_id
           FROM programme.proposal_comments c
           JOIN programme.proposals p ON p.id = c.proposal_id
          WHERE c.id = $1 AND c.deleted_at IS NULL AND p.deleted_at IS NULL",
        comment_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// L'édition d'une revue, par le même chemin.
pub async fn event_id_of_review<'e>(
    executor: impl PgExecutor<'e>,
    review_id: ReviewId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT p.event_id
           FROM programme.reviews r
           JOIN programme.proposals p ON p.id = r.proposal_id
          WHERE r.id = $1 AND p.deleted_at IS NULL",
        review_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// L'édition existe-t-elle ? Sert au cas où l'URL porte l'édition elle-même.
pub async fn event_exists<'e>(executor: impl PgExecutor<'e>, event_id: EventId) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM event.events WHERE id = $1) AS "existe!""#,
        event_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(existe)
}

// -----------------------------------------------------------------------------
// 1. L'édition — son fuseau, sa ville, son état
// -----------------------------------------------------------------------------

/// Ce que ce module a besoin de savoir d'une édition, et rien de plus.
#[derive(Debug, Clone)]
pub struct ContexteEdition {
    pub event_id: Uuid,
    /// `event.events.timezone` : **toute** date de ce module s'y lit.
    pub timezone: String,
    /// Nomme le fuseau à l'écran — « heure de Belém ».
    pub city: Option<String>,
    /// Ouverture de l'édition. **Le repli de créneau d'une séance naissante s'y
    /// appuie** quand le dossier n'a proposé aucun horaire (B5, R4).
    pub starts_at: OffsetDateTime,
    pub ends_at: OffsetDateTime,
    pub status: String,
    /// La programmation est-elle déjà publique ? Change le libellé du bouton du
    /// planificateur — et **ce module ne l'écrit jamais** : elle est posée par
    /// l'émetteur de l'annonce (B5, contracts/events.md § 3).
    pub programme_published_at: Option<OffsetDateTime>,
}

pub async fn contexte_edition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Option<ContexteEdition>> {
    let ligne = sqlx::query!(
        r#"SELECT id, timezone::text AS "timezone!", city, starts_at, ends_at,
                  status::text AS "status!", programme_published_at
             FROM event.events WHERE id = $1"#,
        event_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| ContexteEdition {
        event_id: l.id,
        timezone: l.timezone,
        city: l.city,
        starts_at: l.starts_at,
        ends_at: l.ends_at,
        status: l.status,
        programme_published_at: l.programme_published_at,
    }))
}

// -----------------------------------------------------------------------------
// 2. L'appel et ses règles
// -----------------------------------------------------------------------------

/// Les bornes que l'appel impose et **qu'aucun déclencheur ne vérifie** :
/// intervenants (écart n° 27), durée, plage horaire quotidienne, formats.
#[derive(Debug, Clone)]
pub struct ReglesDeLAppel {
    pub call_id: Uuid,
    pub event_id: Uuid,
    pub min_speakers: i16,
    pub max_speakers: i16,
    pub min_duration_minutes: i16,
    pub max_duration_minutes: i16,
    pub default_duration_minutes: i16,
    /// Heure locale de l'édition, **fin comprise** : une activité peut se
    /// terminer à l'heure de fermeture, pas après.
    pub daily_start_time: time::Time,
    pub daily_end_time: time::Time,
    pub allowed_formats: Vec<String>,
    pub required_reviews: i16,
    pub blind_review: bool,
    pub results_expected_at: Option<time::Date>,
}

pub async fn regles_de_lappel<'e>(
    executor: impl PgExecutor<'e>,
    call_id: Uuid,
) -> Result<Option<ReglesDeLAppel>> {
    let ligne = sqlx::query!(
        r#"SELECT id, event_id, min_speakers, max_speakers,
                  min_duration_minutes, max_duration_minutes, default_duration_minutes,
                  daily_start_time, daily_end_time,
                  allowed_formats::text[] AS "allowed_formats!",
                  required_reviews, blind_review, results_expected_at
             FROM event.calls_for_proposals WHERE id = $1"#,
        call_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| ReglesDeLAppel {
        call_id: l.id,
        event_id: l.event_id,
        min_speakers: l.min_speakers,
        max_speakers: l.max_speakers,
        min_duration_minutes: l.min_duration_minutes,
        max_duration_minutes: l.max_duration_minutes,
        default_duration_minutes: l.default_duration_minutes,
        daily_start_time: l.daily_start_time,
        daily_end_time: l.daily_end_time,
        allowed_formats: l.allowed_formats,
        required_reviews: l.required_reviews,
        blind_review: l.blind_review,
        results_expected_at: l.results_expected_at,
    }))
}

/// L'état de recevabilité de l'appel, **tel que le déclencheur le lit**.
///
/// L'échéance est celle d'`event.effective_deadline()` — prolongation
/// comprise —, jamais `closes_at` seule : c'est ce que le déclencheur compare,
/// et c'est ce que l'écran affiche.
pub async fn etat_de_lappel<'e>(
    executor: impl PgExecutor<'e>,
    call_id: Uuid,
) -> Result<Option<EtatDeLAppel>> {
    let ligne = sqlx::query!(
        r#"SELECT c.status::text AS "statut!", c.opens_at,
                  event.effective_deadline(c.id) AS "echeance!",
                  c.max_proposals_per_organization, c.requires_verified_organization
             FROM event.calls_for_proposals c WHERE c.id = $1"#,
        call_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| EtatDeLAppel {
        statut: l.statut,
        ouvre_le: l.opens_at,
        echeance: l.echeance,
        plafond_par_organisation: l.max_proposals_per_organization,
        exige_organisation_verifiee: l.requires_verified_organization,
    }))
}

/// L'appel **ouvert** d'une édition, s'il y en a un. Un seul par édition, les
/// appels annulés exclus (`ux_calls_one_per_event`).
pub async fn appel_de_ledition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM event.calls_for_proposals
          WHERE event_id = $1 AND status <> 'cancelled'",
        event_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id)
}

/// L'échéance effective, seule — ce que la liste du back-office affiche.
pub async fn echeance_effective<'e>(
    executor: impl PgExecutor<'e>,
    call_id: Uuid,
) -> Result<Option<OffsetDateTime>> {
    let echeance = sqlx::query_scalar!("SELECT event.effective_deadline($1)", call_id)
        .fetch_one(executor)
        .await?;

    Ok(echeance)
}

// -----------------------------------------------------------------------------
// 3. La grille d'évaluation
// -----------------------------------------------------------------------------

/// Un critère de la grille. **Les `numeric` traversent en `float8`** : le
/// workspace ne déclare aucune caractéristique décimale pour SQLx, et le
/// service ne calcule aucune moyenne — l'autorité du calcul reste en base
/// (R24).
#[derive(Debug, Clone)]
pub struct Critere {
    pub id: Uuid,
    pub code: String,
    pub label: serde_json::Value,
    pub description: Option<serde_json::Value>,
    pub max_score: f64,
    pub weight: f64,
    pub is_knockout: bool,
    pub sort_order: i16,
}

pub async fn grille_de_lappel<'e>(
    executor: impl PgExecutor<'e>,
    call_id: Uuid,
) -> Result<Vec<Critere>> {
    let lignes = sqlx::query!(
        r#"SELECT id, code, label, description,
                  max_score::float8 AS "max_score!", weight::float8 AS "weight!",
                  is_knockout, sort_order
             FROM event.review_criteria
            WHERE call_id = $1
            ORDER BY sort_order, code"#,
        call_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Critere {
            id: l.id,
            code: l.code,
            label: l.label,
            description: l.description,
            max_score: l.max_score,
            weight: l.weight,
            is_knockout: l.is_knockout,
            sort_order: l.sort_order,
        })
        .collect())
}

/// Le total pondéré maximal de la grille — le dénominateur de la note sur 20.
pub async fn note_pondere_maximale<'e>(
    executor: impl PgExecutor<'e>,
    call_id: Uuid,
) -> Result<f64> {
    let max = sqlx::query_scalar!(
        r#"SELECT event.max_weighted_score($1)::float8 AS "max!""#,
        call_id
    )
    .fetch_one(executor)
    .await?;

    Ok(max)
}

// -----------------------------------------------------------------------------
// 4. Le comité de l'appel
// -----------------------------------------------------------------------------

/// Un membre du comité, avec sa charge déclarée.
#[derive(Debug, Clone)]
pub struct MembreDuComite {
    pub person_id: Uuid,
    pub display_name: String,
    pub is_lead: bool,
    pub workload_cap: Option<i16>,
}

pub async fn comite_de_lappel<'e>(
    executor: impl PgExecutor<'e>,
    call_id: Uuid,
) -> Result<Vec<MembreDuComite>> {
    let lignes = sqlx::query!(
        r#"SELECT cr.person_id, p.display_name AS "display_name!",
                  cr.is_lead, cr.workload_cap
             FROM event.call_reviewers cr
             JOIN identity.people p ON p.id = cr.person_id
            WHERE cr.call_id = $1
            ORDER BY p.display_name"#,
        call_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| MembreDuComite {
            person_id: l.person_id,
            display_name: l.display_name,
            is_lead: l.is_lead,
            workload_cap: l.workload_cap,
        })
        .collect())
}

/// Un membre du comité **avec sa charge courante** — ce que la liste du
/// back-office propose avant de confier.
///
/// `workload_cap` est ce que l'appel déclare, `charge` ce qui est déjà confié :
/// on ne confie pas douze dossiers de plus à quelqu'un qui en porte déjà vingt,
/// et la colonne existe en base pour cela. **Le service ne refuse rien
/// au-dessus du plafond** — c'est une aide à la décision, pas une règle : la
/// base n'en porte aucune, et l'inventer ici bloquerait une répartition que
/// l'équipe assume.
#[derive(Debug, Clone)]
pub struct ChargeDuComite {
    pub person_id: Uuid,
    pub display_name: String,
    pub is_lead: bool,
    pub workload_cap: Option<i16>,
    /// Dossiers **de cet appel** encore confiés à cette personne, déports
    /// exclus — le même décompte que `assigned_reviewers` de la vue.
    pub charge: i64,
}

/// La composition du comité de l'appel, chacun avec sa charge.
///
/// **La charge est bornée à l'appel**, et non à toute la plateforme : une
/// personne peut siéger dans deux comités, et « combien lui reste-t-il à
/// évaluer ici » est la seule question que l'écran pose. Les déports sont
/// exclus, comme partout ailleurs — un dossier dont on s'est retiré n'est plus
/// une charge.
pub async fn charges_du_comite<'e>(
    executor: impl PgExecutor<'e>,
    call_id: Uuid,
) -> Result<Vec<ChargeDuComite>> {
    let lignes = sqlx::query!(
        r#"SELECT cr.person_id, p.display_name AS "display_name!",
                  cr.is_lead, cr.workload_cap,
                  (SELECT count(*)
                     FROM programme.review_assignments ra
                     JOIN programme.proposals pr ON pr.id = ra.proposal_id
                    WHERE ra.reviewer_id = cr.person_id
                      AND ra.recused_at IS NULL
                      AND pr.call_id = cr.call_id
                      AND pr.deleted_at IS NULL) AS "charge!"
             FROM event.call_reviewers cr
             JOIN identity.people p ON p.id = cr.person_id
            WHERE cr.call_id = $1
            ORDER BY p.display_name"#,
        call_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ChargeDuComite {
            person_id: l.person_id,
            display_name: l.display_name,
            is_lead: l.is_lead,
            workload_cap: l.workload_cap,
            charge: l.charge,
        })
        .collect())
}

// -----------------------------------------------------------------------------
// 5 et 6. L'organisation porteuse, et le droit d'écrire en son nom
// -----------------------------------------------------------------------------

/// La fiche minimale d'une organisation, telle que ce module l'affiche.
#[derive(Debug, Clone)]
pub struct FicheOrganisation {
    pub id: Uuid,
    pub legal_name: String,
    pub acronym: Option<String>,
    pub country_id: Option<Uuid>,
    pub verified: bool,
}

pub async fn fiche_organisation<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: Uuid,
) -> Result<Option<FicheOrganisation>> {
    let ligne = sqlx::query!(
        "SELECT id, legal_name, acronym, country_id, verified_at
           FROM org.organizations WHERE id = $1",
        organization_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| FicheOrganisation {
        id: l.id,
        legal_name: l.legal_name,
        acronym: l.acronym,
        country_id: l.country_id,
        verified: l.verified_at.is_some(),
    }))
}

/// L'adhésion d'une personne à une organisation, réduite à ce qui décide.
///
/// C'est **la** lecture qui borne l'espace organisation : une organisation
/// n'administre rien, son accès n'est pas un périmètre (R13). Ce que la réponse
/// devient est décidé par `domain/ownership.rs`, et là seulement.
pub async fn adhesion<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: Uuid,
    person_id: Uuid,
) -> Result<Option<Adhesion>> {
    let statut = sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM org.memberships
            WHERE organization_id = $1 AND person_id = $2"#,
        organization_id,
        person_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(statut.map(|s| Adhesion {
        active: s == "active",
    }))
}

/// Les organisations dont cette personne est membre **actif**. Ce que l'espace
/// organisation liste, et ce que le formulaire de dépôt propose comme porteur.
pub async fn organisations_actives<'e>(
    executor: impl PgExecutor<'e>,
    person_id: Uuid,
) -> Result<Vec<Uuid>> {
    let ids = sqlx::query_scalar!(
        "SELECT organization_id FROM org.memberships
          WHERE person_id = $1 AND status = 'active'
          ORDER BY organization_id",
        person_id
    )
    .fetch_all(executor)
    .await?;

    Ok(ids)
}

/// Le décompte des dossiers de l'organisation **sur cet appel**, tel que le
/// déclencheur de recevabilité le compte — le dossier courant exclu.
///
/// Les états exclus sont ceux du déclencheur, mot pour mot : brouillon, retiré,
/// non retenu. Compter autrement ferait diverger le contrôle préalable du
/// dernier mot, et l'écran annoncerait un plafond que la base ne tient pas.
pub async fn dossiers_comptes<'e>(
    executor: impl PgExecutor<'e>,
    call_id: Uuid,
    organization_id: Uuid,
    exclu: Option<ProposalId>,
) -> Result<EtatDeLOrganisation> {
    let ligne = sqlx::query!(
        r#"SELECT
               (SELECT count(*) FROM programme.proposals p
                 WHERE p.call_id = $1
                   AND p.organization_id = $2
                   AND p.id IS DISTINCT FROM $3
                   AND p.status NOT IN ('draft', 'withdrawn', 'rejected')
                   AND p.deleted_at IS NULL) AS "comptes!",
               (SELECT o.verified_at IS NOT NULL FROM org.organizations o
                 WHERE o.id = $2) AS "verifiee!""#,
        call_id,
        organization_id,
        exclu.map(ProposalId::as_uuid)
    )
    .fetch_one(executor)
    .await?;

    Ok(EtatDeLOrganisation {
        dossiers_comptes: ligne.comptes,
        verifiee: ligne.verifiee,
    })
}

// -----------------------------------------------------------------------------
// 7. Les personnes
// -----------------------------------------------------------------------------

/// Une personne telle que ce module l'affiche — intervenant, auteur de message,
/// membre du comité.
///
/// `has_account` commande le **verrouillage d'identité** : une personne qui
/// possède un compte détient sa propre fiche, et un déposant qui corrigerait
/// « Awa Sow Fall » en « A. Sowfall » réécrirait ce que toutes ses autres
/// participations affichent (écart n° 31).
#[derive(Debug, Clone)]
pub struct FichePersonne {
    pub id: Uuid,
    pub civility: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub job_title: Option<String>,
    pub biography: Option<serde_json::Value>,
    pub primary_organization_id: Option<Uuid>,
    pub has_account: bool,
}

pub async fn fiche_personne_par_email<'e>(
    executor: impl PgExecutor<'e>,
    email: &str,
) -> Result<Option<FichePersonne>> {
    let ligne = sqlx::query!(
        r#"SELECT p.id, p.civility, p.first_name, p.last_name,
                  p.primary_email::text AS "email!", p.job_title, p.biography,
                  p.primary_organization_id,
                  EXISTS (SELECT 1 FROM identity.accounts a WHERE a.person_id = p.id)
                      AS "has_account!"
             FROM identity.people p
            WHERE p.primary_email = $1::text::platform.email"#,
        email
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| FichePersonne {
        id: l.id,
        civility: l.civility,
        first_name: l.first_name,
        last_name: l.last_name,
        email: l.email,
        job_title: l.job_title,
        biography: l.biography,
        primary_organization_id: l.primary_organization_id,
        has_account: l.has_account,
    }))
}

/// Les personnes d'un lot, pour ne pas résoudre les noms un par un.
pub async fn fiches_personnes<'e>(
    executor: impl PgExecutor<'e>,
    ids: &[Uuid],
) -> Result<Vec<FichePersonne>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id, p.civility, p.first_name, p.last_name,
                  p.primary_email::text AS "email!", p.job_title, p.biography,
                  p.primary_organization_id,
                  EXISTS (SELECT 1 FROM identity.accounts a WHERE a.person_id = p.id)
                      AS "has_account!"
             FROM identity.people p
            WHERE p.id = ANY($1)
            ORDER BY p.display_name"#,
        ids
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| FichePersonne {
            id: l.id,
            civility: l.civility,
            first_name: l.first_name,
            last_name: l.last_name,
            email: l.email,
            job_title: l.job_title,
            biography: l.biography,
            primary_organization_id: l.primary_organization_id,
            has_account: l.has_account,
        })
        .collect())
}

// -----------------------------------------------------------------------------
// 8. Les thématiques — LECTURE seule. L'écriture vit dans `repo/themes.rs`.
// -----------------------------------------------------------------------------

/// Les codes des thématiques d'un dossier, pour **filtrer**.
pub async fn themes_du_dossier<'e>(
    executor: impl PgExecutor<'e>,
    proposal_id: ProposalId,
) -> Result<Vec<String>> {
    let codes = sqlx::query_scalar!(
        r#"SELECT reference.terms_of('programme', 'proposals', $1, 'activity_theme')
               AS "codes!""#,
        proposal_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(codes)
}

/// Les thématiques d'un dossier **prêtes à afficher** — libellé traduit et
/// couleur venus de `reference.taxonomy_terms`, où un administrateur les
/// modifie. N'exposer que les codes forcerait l'écran à recharger la taxonomie :
/// c'est ainsi que les libellés se sont retrouvés figés dans le front de la v1.
pub async fn pastilles_du_dossier<'e>(
    executor: impl PgExecutor<'e>,
    proposal_id: ProposalId,
) -> Result<serde_json::Value> {
    let pastilles = sqlx::query_scalar!(
        r#"SELECT reference.term_badges('programme', 'proposals', $1, 'activity_theme')
               AS "pastilles!""#,
        proposal_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(pastilles)
}

// -----------------------------------------------------------------------------
// 9. Les pièces — l'objet stocké et son adresse
// -----------------------------------------------------------------------------

/// L'objet stocké rattaché à une pièce. **Ce module ne pose ni ne détruit
/// l'objet** : le cycle de vie du fichier appartient à B6, qui le connaît par
/// ailleurs. Ici, on rattache et on lit l'adresse.
#[derive(Debug, Clone)]
pub struct ObjetStocke {
    pub id: Uuid,
    pub original_filename: Option<String>,
    pub mime_type: String,
    pub byte_size: i64,
    pub url: String,
}

pub async fn objet_stocke<'e>(
    executor: impl PgExecutor<'e>,
    asset_id: Uuid,
) -> Result<Option<ObjetStocke>> {
    let ligne = sqlx::query!(
        r#"SELECT a.id, a.original_filename, a.mime_type, a.byte_size,
                  media.object_url(a.bucket, a.object_key) AS "url!"
             FROM media.assets a WHERE a.id = $1 AND a.deleted_at IS NULL"#,
        asset_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| ObjetStocke {
        id: l.id,
        original_filename: l.original_filename,
        mime_type: l.mime_type,
        byte_size: l.byte_size,
        url: l.url,
    }))
}

// -----------------------------------------------------------------------------
// 10. L'historique de participation de l'organisation porteuse
// -----------------------------------------------------------------------------

/// Ce que l'espace organisation montre de son propre passé.
///
/// **Aucune note n'en sort** : la vue en porte une (`note_moyenne_obtenue`),
/// et elle n'est pas lue ici. Le déposant ne voit ni sa note ni son rang
/// (FR-077) — c'est la question n° 8 des points bloqués, dont l'option A est
/// tenue depuis A5.
#[derive(Debug, Clone)]
pub struct HistoriqueOrganisation {
    pub propositions_deposees: i64,
    pub propositions_acceptees: i64,
    pub evenements_couverts: i64,
    pub sessions_realisees: i64,
}

pub async fn historique_organisation<'e>(
    executor: impl PgExecutor<'e>,
    organization_id: Uuid,
) -> Result<Option<HistoriqueOrganisation>> {
    let ligne = sqlx::query!(
        r#"SELECT propositions_deposees AS "deposees!",
                  propositions_acceptees AS "acceptees!",
                  evenements_couverts AS "evenements!",
                  sessions_realisees AS "sessions!"
             FROM analytics.mv_organization_scorecard
            WHERE organization_id = $1"#,
        organization_id
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| HistoriqueOrganisation {
        propositions_deposees: l.deposees,
        propositions_acceptees: l.acceptees,
        evenements_couverts: l.evenements,
        sessions_realisees: l.sessions,
    }))
}

// -----------------------------------------------------------------------------
// Les lectures d'AFFICHAGE, dans le même espace de noms
//
// `cross::fiche_edition`, `cross::personnes_affichees`… s'appellent comme
// avant : le découpage est un fait de fichier, pas de frontière. Voir
// l'en-tête de `cross/fiches.rs` pour la ligne de partage.
// -----------------------------------------------------------------------------
mod fiches;

pub use fiches::*;

// -----------------------------------------------------------------------------
// Les lectures de la GRILLE, ajoutées par B5, dans le même espace de noms
//
// Jours, salles, fils, canaux, et les valeurs admises d'une réponse. Elles
// répondent toutes à une question de CE module — « où cette séance est-elle
// installée ? », « à quel fil peut-on la rattacher ? » — et n'écrivent rien.
// -----------------------------------------------------------------------------
mod grille;

pub use grille::*;
