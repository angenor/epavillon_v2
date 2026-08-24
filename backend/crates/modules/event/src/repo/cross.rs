//! **Les seules lectures hors du schéma `event`, et les résolutions
//! d'ascendance.**
//!
//! ## La règle qui gouverne ce fichier
//!
//! Un module lit hors de son schéma quand la question porte sur **ses propres
//! entités** ; il n'y écrit jamais. « Combien de dossiers cette édition
//! a-t-elle reçus ? » est une question de ce module, même si la réponse vit
//! dans `programme`. « Rendre ces séances publiques » n'en est pas une : c'est
//! un effet dans un autre module, et il passe par l'outbox (research.md § R10).
//!
//! Cette règle est facile à énoncer et facile à enfreindre par accroissement.
//! Dispersées dans huit dépôts, ces requêtes deviennent invisibles ; réunies,
//! elles se relisent en un fichier, et **c'est ici qu'un ajout se discute**.
//!
//! ## Les neuf lectures hors schéma autorisées
//!
//! Chacune arrive avec l'histoire qui la rend nécessaire ; aucune n'existe
//! encore ici tant que sa phase n'est pas écrite.
//!
//! | # | Lecture | Question de **ce** module |
//! |---|---|---|
//! | 1 | `programme` — dossiers déposés par édition, brouillons exclus | « combien de dossiers cette édition a-t-elle reçus ? » |
//! | 2 | `programme` — séances par édition, et séances placées en salle | « où en est le placement de cette édition ? » |
//! | 3 | `programme` — séances par journée, salle, lieu, canal | « que détacherait ce retrait ? » (R8) |
//! | 4 | `programme` — rattachements séance–fil | « combien de séances ce fil porte-t-il ? » |
//! | 5 | `programme` — notes posées par critère | « ce critère peut-il être supprimé ? » (R9) |
//! | 6 | `programme` — `publication_readiness()` et le prédicat des séances à publier | « cette édition peut-elle être publiée, et combien de séances ? » (R10) |
//! | 7 | `programme` — dossiers confiés et revues rendues par membre, et `v_edition_stats` | « quelle est la charge de ce membre, quel volume publie cette édition ? » |
//! | 8 | `identity` — personnes assignables, candidats au comité, `has_permission` sur l'édition | « qui peut curer un fil, qui peut siéger, et détient-il bien le droit d'évaluer ? » |
//! | 9 | `reference` et `media` — `terms_of`, `term_badges`, pays, langues, `attached_image` | thématiques d'un fil, listes du formulaire, les trois déclinaisons d'une image |
//!
//! Le périmètre d'administration ne figure pas dans cette liste : il est lu par
//! le garde du noyau (`kernel::auth`), jamais par une requête d'ici.
//!
//! **Aucune ligne de ce fichier n'écrit.** Pas un `INSERT`, pas un `UPDATE`,
//! pas un `DELETE` — c'est vérifiable d'un coup d'œil, et c'est le second
//! intérêt du regroupement. Le contrôle mécanique est dans `quickstart.md`.

use kernel::error::Result;
use sqlx::PgExecutor;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::edition::CountryOption;
use crate::domain::ids::{CallId, ChannelId, EventDayId, EventId, RoomId, TrackId, VenueId};

// -----------------------------------------------------------------------------
// Résolution d'ascendance
//
// Six routes du back-office sont paramétrées par l'identifiant d'un ENFANT de
// l'édition. L'ordre est imposé et ne se négocie pas : **résoudre l'ascendance,
// puis vérifier le périmètre, puis agir** (research.md § R2).
//
// Ces lectures ne divulguent rien : elles ne rendent qu'un identifiant
// d'édition, jamais exposé, et leur absence produit le MÊME refus que l'échec
// du périmètre. Un identifiant inexistant et un identifiant hors périmètre sont
// indiscernables par la forme de la réponse (principe IX).
//
// Elles restent dans le schéma `event` — elles sont ici parce que c'est le
// fichier de la frontière, pas parce qu'elles la franchissent.
// -----------------------------------------------------------------------------

/// L'édition d'un fil de programmation.
pub async fn event_id_of_track<'e>(
    executor: impl PgExecutor<'e>,
    track_id: TrackId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT event_id FROM event.programme_tracks WHERE id = $1",
        track_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// L'édition d'un lieu.
pub async fn event_id_of_venue<'e>(
    executor: impl PgExecutor<'e>,
    venue_id: VenueId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT event_id FROM event.venues WHERE id = $1",
        venue_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// L'édition d'une salle — **par son lieu** : `event.rooms` ne porte pas
/// l'édition, et la déduire autrement demanderait de la faire voyager dans
/// l'URL, donc de la croire.
pub async fn event_id_of_room<'e>(
    executor: impl PgExecutor<'e>,
    room_id: RoomId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT v.event_id
           FROM event.rooms r
           JOIN event.venues v ON v.id = r.venue_id
          WHERE r.id = $1",
        room_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// L'édition d'un canal de diffusion.
///
/// **Elle peut être absente sans que le canal le soit** : un canal général de
/// la plateforme porte `event_id IS NULL`. Le double `Option` est donc porteur
/// de sens — « ce canal n'existe pas » et « ce canal n'appartient à aucune
/// édition » ne se règlent pas de la même façon, le second valant refus
/// `platform_channel` et non refus de périmètre.
pub async fn event_id_of_channel<'e>(
    executor: impl PgExecutor<'e>,
    channel_id: ChannelId,
) -> Result<Option<Option<EventId>>> {
    let ligne = sqlx::query_scalar!(
        "SELECT event_id FROM event.broadcast_channels WHERE id = $1",
        channel_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|id| id.map(EventId::from)))
}

/// L'édition d'un appel à propositions.
pub async fn event_id_of_call<'e>(
    executor: impl PgExecutor<'e>,
    call_id: CallId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT event_id FROM event.calls_for_proposals WHERE id = $1",
        call_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// L'édition d'une journée du calendrier.
pub async fn event_id_of_day<'e>(
    executor: impl PgExecutor<'e>,
    day_id: EventDayId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT event_id FROM event.event_days WHERE id = $1",
        day_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// L'édition existe-t-elle ? Le pendant des six résolutions pour les routes
/// paramétrées par l'édition elle-même, afin que le refus soit posé au même
/// endroit et prenne la même forme.
pub async fn event_exists<'e>(executor: impl PgExecutor<'e>, event_id: EventId) -> Result<bool> {
    let existe: Option<Uuid> = sqlx::query_scalar!(
        "SELECT id FROM event.events WHERE id = $1",
        event_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(existe.is_some())
}

// -----------------------------------------------------------------------------
// Décomptes d'une ligne de liste — lectures n° 1, 2 et 3 de l'inventaire
// -----------------------------------------------------------------------------

/// Ce qu'une ligne de la liste affiche et qui n'est **aucune colonne** de
/// `event.events`.
#[derive(Debug, Clone, Default)]
pub struct Decomptes {
    /// Dossiers **déposés**, brouillons exclus.
    pub proposal_count: i64,
    /// Séances de l'édition, placées ou non.
    pub session_count: i64,
    /// Séances effectivement **placées en salle** : le reste attend un créneau.
    pub scheduled_session_count: i64,
    /// Journées du calendrier déjà créées.
    pub day_count: i64,
}

/// Les quatre décomptes d'une édition, **en une requête**.
///
/// **Les brouillons sont exclus des dossiers**, à l'inverse de la liste des
/// propositions qui les montre : la colonne répond à « combien de dossiers cette
/// édition a-t-elle *reçus* ? », et un brouillon n'a rien été reçu.
///
/// **Les sous-requêtes se comportent comme des jointures par la gauche** : elles
/// rendent zéro plutôt que rien, et une édition sans aucun dossier reste
/// visible dans sa propre liste. C'est le piège classique du décompte joint —
/// une jointure interne fait disparaître exactement les lignes qu'on cherchait.
///
/// `programme.proposals` porte `event_id` en dénormalisation assumée du modèle :
/// on s'en sert plutôt que de repasser par l'appel, parce que l'édition reste
/// connue même si l'appel disparaît.
pub async fn decomptes<'e>(executor: impl PgExecutor<'e>, event_id: EventId) -> Result<Decomptes> {
    let ligne = sqlx::query!(
        r#"SELECT
             (SELECT count(*) FROM programme.proposals p
               WHERE p.event_id = $1 AND p.status <> 'draft')          AS "proposal_count!",
             (SELECT count(*) FROM programme.sessions s
               WHERE s.event_id = $1)                                   AS "session_count!",
             (SELECT count(*) FROM programme.sessions s
               WHERE s.event_id = $1 AND s.room_id IS NOT NULL)         AS "scheduled_session_count!",
             (SELECT count(*) FROM event.event_days d
               WHERE d.event_id = $1)                                   AS "day_count!""#,
        event_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(Decomptes {
        proposal_count: ligne.proposal_count,
        session_count: ligne.session_count,
        scheduled_session_count: ligne.scheduled_session_count,
        day_count: ligne.day_count,
    })
}

// -----------------------------------------------------------------------------
// Listes du formulaire — lecture n° 9 de l'inventaire
// -----------------------------------------------------------------------------

/// Les pays ACTIFS du référentiel, triés par leur nom français.
///
/// `is_active` n'est pas décoratif : un pays retiré du référentiel — parce qu'il
/// a changé de nom, ou n'existe plus — reste en base pour que les fiches
/// anciennes continuent de le nommer, mais il ne se propose plus au choix.
pub async fn pays_du_referentiel<'e>(executor: impl PgExecutor<'e>) -> Result<Vec<CountryOption>> {
    let lignes = sqlx::query!(
        r#"SELECT id, name, iso2 FROM reference.countries WHERE is_active ORDER BY name->>'fr'"#
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| CountryOption {
            id: l.id,
            name: l.name,
            iso2: l.iso2,
        })
        .collect())
}

/// Les décomptes de **plusieurs** éditions, en une requête.
///
/// La liste du back-office en demande autant qu'elle a de lignes : les prendre
/// une par une ferait une requête par édition — le défaut que B2 a payé sur sa
/// propre liste. Une édition absente de la réponse n'existe pas ; une édition
/// sans rien à compter y figure à zéro, parce que `unnest` porte la liste et que
/// les sous-requêtes se comportent comme des jointures par la gauche.
pub async fn decomptes_par_edition<'e>(
    executor: impl PgExecutor<'e>,
    editions: &[Uuid],
) -> Result<HashMap<Uuid, Decomptes>> {
    if editions.is_empty() {
        return Ok(HashMap::new());
    }

    let lignes = sqlx::query!(
        r#"SELECT e.id AS "id!",
             (SELECT count(*) FROM programme.proposals p
               WHERE p.event_id = e.id AND p.status <> 'draft')      AS "proposal_count!",
             (SELECT count(*) FROM programme.sessions s
               WHERE s.event_id = e.id)                               AS "session_count!",
             (SELECT count(*) FROM programme.sessions s
               WHERE s.event_id = e.id AND s.room_id IS NOT NULL)     AS "scheduled_session_count!",
             (SELECT count(*) FROM event.event_days d
               WHERE d.event_id = e.id)                               AS "day_count!"
           FROM unnest($1::uuid[]) AS e(id)"#,
        editions
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| {
            (
                l.id,
                Decomptes {
                    proposal_count: l.proposal_count,
                    session_count: l.session_count,
                    scheduled_session_count: l.scheduled_session_count,
                    day_count: l.day_count,
                },
            )
        })
        .collect())
}

// -----------------------------------------------------------------------------
// Décomptes des onglets — lectures n° 3 et 4 de l'inventaire
//
// « Que détacherait ce retrait ? » est une question de ce module : toutes ces
// clés étrangères sont `ON DELETE SET NULL`, la séance survit et perd son
// rattachement. L'écran doit pouvoir l'annoncer AVANT (research.md § R8).
// -----------------------------------------------------------------------------

/// Séances par journée du calendrier.
pub async fn seances_par_journee<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<HashMap<Uuid, i64>> {
    let lignes = sqlx::query!(
        r#"SELECT s.event_day_id AS "journee!", count(*) AS "n!"
             FROM programme.sessions s
            WHERE s.event_id = $1 AND s.event_day_id IS NOT NULL
            GROUP BY s.event_day_id"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.journee, l.n)).collect())
}

/// Séances rattachées par fil. Le rattachement est **explicite** — une décision
/// éditoriale prise au planificateur —, jamais déduit des dates.
pub async fn seances_par_fil<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<HashMap<Uuid, i64>> {
    let lignes = sqlx::query!(
        r#"SELECT st.track_id AS "fil!", count(*) AS "n!"
             FROM programme.session_tracks st
             JOIN event.programme_tracks t ON t.id = st.track_id
            WHERE t.event_id = $1
            GROUP BY st.track_id"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.fil, l.n)).collect())
}

/// Séances placées par salle : ce qu'un retrait de salle déplacerait.
pub async fn seances_par_salle<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<HashMap<Uuid, i64>> {
    let lignes = sqlx::query!(
        r#"SELECT s.room_id AS "salle!", count(*) AS "n!"
             FROM programme.sessions s
            WHERE s.event_id = $1 AND s.room_id IS NOT NULL
            GROUP BY s.room_id"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.salle, l.n)).collect())
}

/// Séances diffusées par canal.
///
/// **Le filtre porte sur le canal et non sur l'édition** : un canal général de
/// la plateforme sert plusieurs éditions, et ne compter que les séances de
/// celle-ci ferait annoncer « aucune séance » à qui s'apprête à le désactiver.
pub async fn seances_par_canal<'e>(
    executor: impl PgExecutor<'e>,
    canaux: &[Uuid],
) -> Result<HashMap<Uuid, i64>> {
    if canaux.is_empty() {
        return Ok(HashMap::new());
    }

    let lignes = sqlx::query!(
        r#"SELECT s.broadcast_channel_id AS "canal!", count(*) AS "n!"
             FROM programme.sessions s
            WHERE s.broadcast_channel_id = ANY($1::uuid[])
            GROUP BY s.broadcast_channel_id"#,
        canaux
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.canal, l.n)).collect())
}

// -----------------------------------------------------------------------------
// L'appel et sa grille — lectures n° 1 et 5 de l'inventaire
// -----------------------------------------------------------------------------

/// Dossiers **déposés** sur un appel, brouillons exclus.
pub async fn dossiers_de_l_appel<'e>(
    executor: impl PgExecutor<'e>,
    call_id: CallId,
) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.proposals
            WHERE call_id = $1 AND status <> 'draft'"#,
        call_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(n)
}

/// Notes déjà posées, **par critère**.
///
/// C'est le chiffre qui interdira le retrait d'un critère :
/// `xmod_fk_review_scores_criterion` est `ON DELETE CASCADE`, et supprimer la
/// ligne effacerait sans un mot l'argumentaire d'une décision de sélection
/// (research.md § R9).
pub async fn notes_par_critere<'e>(
    executor: impl PgExecutor<'e>,
    call_id: CallId,
) -> Result<HashMap<Uuid, i64>> {
    let lignes = sqlx::query!(
        r#"SELECT sc.criterion_id AS "critere!", count(*) AS "n!"
             FROM programme.review_scores sc
             JOIN event.review_criteria rc ON rc.id = sc.criterion_id
            WHERE rc.call_id = $1
            GROUP BY sc.criterion_id"#,
        call_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.critere, l.n)).collect())
}

// -----------------------------------------------------------------------------
// Personnes — lectures n° 7 et 8 de l'inventaire
//
// **`programme.review.write` est ici un CRITÈRE DE LISTE, pas un garde.** Il ne
// protège aucune route de ce module et n'a donc rien à faire dans
// `domain/permissions.rs`, qui déclare les trois permissions consommées par
// l'autorisation. Il répond à « cette personne peut-elle réellement évaluer ? »,
// et l'écran se contente de le DIRE : siéger au comité n'accorde rien.
// -----------------------------------------------------------------------------

/// Permission d'évaluer, telle que `030_identity.sql` § 8 la nomme. Consommée
/// comme une **donnée** : c'est le critère des deux listes de personnes.
pub const REVIEW_WRITE: &str = "programme.review.write";

/// Les noms d'affichage de quelques personnes. Sert à résoudre le responsable
/// d'un fil : l'écran n'affiche pas un identifiant.
pub async fn noms_de_personnes<'e>(
    executor: impl PgExecutor<'e>,
    personnes: &[Uuid],
) -> Result<HashMap<Uuid, String>> {
    if personnes.is_empty() {
        return Ok(HashMap::new());
    }

    let lignes = sqlx::query!(
        r#"SELECT id, display_name AS "display_name!"
             FROM identity.people WHERE id = ANY($1::uuid[])"#,
        personnes
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.id, l.display_name)).collect())
}

/// La composition du comité, **résolue** : nom, adresse, organisation de
/// rattachement, charge confiée, revues rendues, et détention **effective** de
/// la permission d'évaluer sur cette édition.
///
/// Les dossiers confiés excluent les **déports** : un membre qui s'est retiré
/// d'un dossier ne le porte plus, et le compter reviendrait à surestimer sa
/// charge au moment d'en répartir une nouvelle.
///
/// `has_review_permission` ne fait que **dire** : ajouter quelqu'un au comité
/// n'accorde aucun droit, et l'écran ne doit pas laisser croire l'inverse.
pub async fn comite_resolu<'e>(
    executor: impl PgExecutor<'e>,
    call_id: CallId,
    event_id: EventId,
) -> Result<HashMap<Uuid, PersonneDuComite>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id,
                  p.display_name AS "full_name!",
                  p.primary_email::text AS "email!",
                  o.legal_name AS "organization_name?",
                  (SELECT count(*) FROM programme.review_assignments ra
                    JOIN programme.proposals pr ON pr.id = ra.proposal_id
                   WHERE ra.reviewer_id = p.id
                     AND ra.recused_at IS NULL
                     AND pr.call_id = $1)                       AS "assigned_count!",
                  (SELECT count(*) FROM programme.reviews rv
                    JOIN programme.proposals pr ON pr.id = rv.proposal_id
                   WHERE rv.reviewer_id = p.id
                     AND rv.submitted_at IS NOT NULL
                     AND pr.call_id = $1)                       AS "submitted_count!",
                  identity.has_permission(p.id, 'programme.review.write',
                                          'event'::identity.scope_type, $2)
                      AS "has_review_permission!"
             FROM event.call_reviewers cr
             JOIN identity.people p ON p.id = cr.person_id
             LEFT JOIN org.organizations o ON o.id = p.primary_organization_id
            WHERE cr.call_id = $1"#,
        call_id.as_uuid(),
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| {
            (
                l.id,
                PersonneDuComite {
                    full_name: l.full_name,
                    email: l.email,
                    organization_name: l.organization_name,
                    assigned_count: l.assigned_count,
                    submitted_count: l.submitted_count,
                    has_review_permission: l.has_review_permission,
                },
            )
        })
        .collect())
}

/// Ce que le comité affiche d'un siège, et qui ne vient pas de `event`.
#[derive(Debug, Clone)]
pub struct PersonneDuComite {
    pub full_name: String,
    pub email: String,
    pub organization_name: Option<String>,
    pub assigned_count: i64,
    pub submitted_count: i64,
    pub has_review_permission: bool,
}

/// Les personnes que l'équipe peut **désigner** — responsable d'un fil, membre
/// du comité.
///
/// **Le critère est une permission, jamais un nom de rôle** (principe V) :
/// quiconque détient l'une des permissions données, globalement ou sur cette
/// édition, en fait partie. Une liste de rôles écrite en dur laisserait de côté
/// le premier rôle ajouté au catalogue.
pub async fn personnes_assignables<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
    permissions: &[&str],
) -> Result<Vec<Candidat>> {
    let codes: Vec<String> = permissions.iter().map(|c| (*c).to_owned()).collect();

    let lignes = sqlx::query!(
        r#"SELECT DISTINCT p.id,
                  p.display_name AS "full_name!",
                  p.primary_email::text AS "email!",
                  o.legal_name AS "organization_name?",
                  identity.has_permission(p.id, 'programme.review.write',
                                          'event'::identity.scope_type, $1)
                      AS "has_review_permission!"
             FROM identity.people p
             JOIN identity.role_assignments ra ON ra.person_id = p.id
             JOIN identity.role_permissions rp ON rp.role_code = ra.role_code
             LEFT JOIN org.organizations o ON o.id = p.primary_organization_id
            WHERE p.status = 'active'
              AND rp.permission_code = ANY($2::text[])
              AND ra.revoked_at IS NULL
              AND ra.valid_from <= now()
              AND (ra.valid_until IS NULL OR ra.valid_until > now())
              AND (ra.scope_type = 'global'
                   OR (ra.scope_type = 'event' AND ra.scope_id = $1))
            ORDER BY p.display_name"#,
        event_id.as_uuid(),
        &codes
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Candidat {
            person_id: l.id,
            full_name: l.full_name,
            email: l.email,
            organization_name: l.organization_name,
            has_review_permission: l.has_review_permission,
        })
        .collect())
}

/// Une personne désignable, telle que l'écran la propose.
#[derive(Debug, Clone)]
pub struct Candidat {
    pub person_id: Uuid,
    pub full_name: String,
    pub email: String,
    pub organization_name: Option<String>,
    pub has_review_permission: bool,
}

// -----------------------------------------------------------------------------
// Référentiel et médias — lecture n° 9 de l'inventaire
// -----------------------------------------------------------------------------

/// Les **trois déclinaisons** d'une édition, telles que `media.attached_image()`
/// les rend pour les rôles `banner` (32:9), `cover` (16:9) et `thumbnail` (1:1).
///
/// **Les trois clés sont toujours présentes**, à `null` tant que rien n'a été
/// téléversé pour ce rôle : la boucle d'affichage n'a alors aucune garde à
/// écrire. La fonction du modèle est appelée plutôt que sa jointure recopiée —
/// le rattachement est polymorphe, et trois vues qui le déroulent à la main sont
/// trois occasions de diverger.
pub async fn images_de_l_edition<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<serde_json::Value> {
    let images = sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
                     'banner',    media.attached_image('event', 'events', $1, 'banner'),
                     'cover',     media.attached_image('event', 'events', $1, 'cover'),
                     'thumbnail', media.attached_image('event', 'events', $1, 'thumbnail')
                  ) AS "images!""#,
        event_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(images)
}

/// Les thématiques d'un fil, **prêtes à afficher** — code, libellé, couleur,
/// icône —, par `reference.term_badges()`.
///
/// Ce sont des **données**, pas des traductions : un administrateur les modifie
/// depuis le back-office, et les recopier dans un fichier i18n est exactement le
/// défaut n° 1 de la v1.
pub async fn themes_des_fils<'e>(
    executor: impl PgExecutor<'e>,
    fils: &[Uuid],
) -> Result<HashMap<Uuid, serde_json::Value>> {
    if fils.is_empty() {
        return Ok(HashMap::new());
    }

    let lignes = sqlx::query!(
        r#"SELECT t.id AS "id!",
                  reference.term_badges('event', 'programme_tracks', t.id, 'activity_theme')
                      AS "themes!"
             FROM unnest($1::uuid[]) AS t(id)"#,
        fils
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.id, l.themes)).collect())
}

/// Les thématiques dont on peut habiller un fil.
pub async fn themes_disponibles<'e>(executor: impl PgExecutor<'e>) -> Result<serde_json::Value> {
    let themes = sqlx::query_scalar!(
        r#"SELECT COALESCE(
                    jsonb_agg(jsonb_build_object(
                        'code', code, 'label', label, 'color', color_hex, 'icon', icon)
                      ORDER BY sort_order, code),
                    '[]'::jsonb) AS "themes!"
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = 'activity_theme' AND is_active"#
    )
    .fetch_one(executor)
    .await?;

    Ok(themes)
}

// -----------------------------------------------------------------------------
// Ce qu'un retrait détacherait — lecture n° 3 de l'inventaire
//
// **Chacune se prend AVANT l'ordre de suppression** (research.md § R8). Les clés
// sont `ON DELETE SET NULL` (journée, salle) ou `CASCADE` (rattachement à un
// fil) : après l'ordre, le lien n'existe plus, le décompte rendrait ZÉRO, et
// l'écran annoncerait sereinement qu'il n'a rien détaché.
// -----------------------------------------------------------------------------

/// Séances installées dans une salle.
pub async fn seances_de_la_salle<'e>(
    executor: impl PgExecutor<'e>,
    room_id: RoomId,
) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.sessions WHERE room_id = $1"#,
        room_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(n)
}

/// Séances installées dans **l'une quelconque des salles** d'un lieu.
///
/// Retirer un lieu emporte ses salles par cascade : le chiffre annoncé doit
/// couvrir toutes ses salles, pas seulement celles que l'écran affiche.
pub async fn seances_du_lieu<'e>(executor: impl PgExecutor<'e>, venue_id: VenueId) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!"
             FROM programme.sessions s
             JOIN event.rooms r ON r.id = s.room_id
            WHERE r.venue_id = $1"#,
        venue_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(n)
}

/// Séances diffusées sur un canal.
///
/// **Le filtre porte sur le canal et non sur l'édition** : un canal général de
/// la plateforme sert plusieurs éditions, et ne compter que les séances de
/// celle-ci ferait annoncer « aucune séance » à qui s'apprête à le désactiver.
pub async fn seances_du_canal<'e>(
    executor: impl PgExecutor<'e>,
    channel_id: ChannelId,
) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!"
             FROM programme.sessions WHERE broadcast_channel_id = $1"#,
        channel_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(n)
}

/// Séances rattachées à une journée du calendrier.
pub async fn seances_de_la_journee<'e>(
    executor: impl PgExecutor<'e>,
    day_id: EventDayId,
) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.sessions WHERE event_day_id = $1"#,
        day_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(n)
}

/// **Rattachements** séance–fil, et non séances : supprimer un fil ne supprime
/// aucune séance, il défait un travail éditorial. C'est ce travail-là que
/// l'écran chiffre avant de confirmer.
pub async fn rattachements_du_fil<'e>(
    executor: impl PgExecutor<'e>,
    track_id: TrackId,
) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.session_tracks WHERE track_id = $1"#,
        track_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok(n)
}

/// Les personnes qui **existent** parmi celles qu'on veut faire siéger.
///
/// Sans ce contrôle, `xmod_fk_call_reviewers_person` refuserait — mais elle
/// refuserait **l'enregistrement entier**, et l'écran ne saurait pas laquelle
/// des huit lignes est en cause. Le service compare et nomme.
///
/// **Les personnes anonymisées ne comptent pas** : leur ligne subsiste par
/// obligation d'historique, mais désigner quelqu'un qui a exercé son droit à
/// l'effacement n'aurait aucun sens.
pub async fn personnes_existantes<'e>(
    executor: impl PgExecutor<'e>,
    personnes: &[Uuid],
) -> Result<Vec<Uuid>> {
    if personnes.is_empty() {
        return Ok(Vec::new());
    }

    let existantes = sqlx::query_scalar!(
        "SELECT id FROM identity.people
          WHERE id = ANY($1::uuid[]) AND status <> 'anonymized'",
        personnes
    )
    .fetch_all(executor)
    .await?;

    Ok(existantes)
}

// -----------------------------------------------------------------------------
// La publication de la programmation — lecture n° 6 de l'inventaire
// -----------------------------------------------------------------------------

/// Un point du contrôle préalable, tel que `programme.publication_readiness()`
/// le rend — `PublicationReadinessIssue`.
///
/// **`occurs_at` est un INSTANT, pas un texte.** La première version du modèle
/// glissait un intervalle brut dans `detail`, qui s'affichait tel quel à
/// l'écran : une chaîne figée en base ne peut ni se traduire, ni se situer dans
/// le fuseau de l'édition, alors que la règle du projet l'exige de toute date
/// affichée.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PointDeControle {
    pub severity: String,
    pub issue: String,
    pub detail: Option<String>,
    pub session_id: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub occurs_at: Option<time::OffsetDateTime>,
}

impl PointDeControle {
    /// **Seule la gravité bloquante retient la publication.** Les
    /// avertissements l'accompagnent sans l'empêcher : une séance sans
    /// intervenant déclaré mérite d'être signalée, pas d'interdire tout un
    /// programme.
    pub fn est_bloquant(&self) -> bool {
        self.severity == "blocking"
    }
}

/// Le contrôle préalable d'une édition — **la fonction du modèle, appelée**.
///
/// Elle réunit les conflits détectés et les manques : séance sans créneau
/// valide, séance sans lieu ni précision de lieu, diffusion sans canal,
/// intervenant absent. La réécrire ici ferait une seconde définition de « prêt à
/// publier », et c'est la seconde qui finit par se tromper.
pub async fn controle_de_publication<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<Vec<PointDeControle>> {
    let lignes = sqlx::query!(
        r#"SELECT severity AS "severity!", issue AS "issue!", detail,
                  session_id, occurs_at
             FROM programme.publication_readiness($1)
            ORDER BY (severity = 'blocking') DESC, occurs_at NULLS LAST"#,
        event_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| PointDeControle {
            severity: l.severity,
            issue: l.issue,
            detail: l.detail,
            session_id: l.session_id,
            occurs_at: l.occurs_at,
        })
        .collect())
}

/// **Le prédicat exact des séances que la publication désigne**, et leur
/// décompte.
///
/// Il est écrit **une fois**, ici, et voyage ensuite dans la charge utile de
/// l'annonce : le consommateur de B5 publie ce prédicat et pas un autre. Le
/// chiffre annoncé et l'effet obtenu viennent ainsi du même raisonnement, sous
/// l'instantané de la même transaction (research.md § R10).
pub const STATUTS_A_PUBLIER: [&str; 2] = ["planned", "scheduled"];

pub async fn seances_a_publier<'e>(
    executor: impl PgExecutor<'e>,
    event_id: EventId,
) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!"
             FROM programme.sessions
            WHERE event_id = $1
              AND status::text = ANY($2::text[])
              AND published_at IS NULL"#,
        event_id.as_uuid(),
        &STATUTS_A_PUBLIER.map(str::to_owned)
    )
    .fetch_one(executor)
    .await?;

    Ok(n)
}
