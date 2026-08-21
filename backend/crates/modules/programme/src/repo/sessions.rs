//! Les séances — lecture, naissance, créneau, diffusion.
//!
//! # La première question de toute route paramétrée
//!
//! **De quelle édition cette séance relève-t-elle ?** Elle est posée ici, et
//! elle précède toute vérification de périmètre : croire l'édition annoncée par
//! le client reviendrait à lui laisser choisir son propre droit. Le contrat du
//! front envoie encore `event_id` dans ses charges utiles ; il est ignoré,
//! comme `actorId` l'a été en B1.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::ids::{EventId, RegistrationId, SessionId};

/// L'édition d'une séance.
///
/// Ne rend qu'un identifiant, jamais exposé : son absence produit le **même**
/// refus que l'échec du périmètre, et une URL forgée ne dit donc pas si la
/// séance existe (principe IX).
pub async fn event_id_of_session<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT event_id FROM programme.sessions WHERE id = $1",
        session_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// L'édition d'une inscription — **deux niveaux** : l'inscription appartient à
/// une séance, qui appartient à une édition.
pub async fn event_id_of_registration<'e>(
    executor: impl PgExecutor<'e>,
    registration_id: RegistrationId,
) -> Result<Option<EventId>> {
    let id = sqlx::query_scalar!(
        "SELECT s.event_id
           FROM programme.registrations r
           JOIN programme.sessions s ON s.id = r.session_id
          WHERE r.id = $1",
        registration_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(id.map(EventId::from))
}

/// Une séance, telle que la base la porte. C'est la forme que le contrat du
/// front nomme `Session`, rendue par l'espace organisation et la page publique.
pub async fn seance<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Option<serde_json::Value>> {
    let ligne = sqlx::query_scalar!(
        r#"SELECT to_jsonb(s) - 'search_vector' AS "seance!"
             FROM programme.sessions s WHERE s.id = $1"#,
        session_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne)
}

/// Ce qu'une écriture du planificateur a besoin de savoir avant d'agir.
#[derive(Debug, Clone)]
pub struct EtatDeSeance {
    pub id: Uuid,
    pub event_id: Uuid,
    pub room_id: Option<Uuid>,
    pub event_day_id: Option<Uuid>,
    pub is_streamed: bool,
    pub broadcast_channel_id: Option<Uuid>,
    pub status: String,
}

pub async fn etat<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Option<EtatDeSeance>> {
    let ligne = sqlx::query!(
        r#"SELECT id, event_id, room_id, event_day_id, is_streamed,
                  broadcast_channel_id, status::text AS "status!"
             FROM programme.sessions WHERE id = $1"#,
        session_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| EtatDeSeance {
        id: l.id,
        event_id: l.event_id,
        room_id: l.room_id,
        event_day_id: l.event_day_id,
        is_streamed: l.is_streamed,
        broadcast_channel_id: l.broadcast_channel_id,
        status: l.status,
    }))
}

/// Une adresse d'URL est-elle déjà prise dans cette édition ?
///
/// `ux_sessions_slug` porte l'unicité ; cette lecture sert au suffixe, qui se
/// pose **sur collision** et jamais par comptage préalable (R7).
pub async fn adresse_prise(conn: &mut PgConnection, event_id: EventId, slug: &str) -> Result<bool> {
    let prise = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM programme.sessions
                WHERE event_id = $1 AND slug = $2::text::platform.slug
           ) AS "prise!""#,
        event_id.as_uuid(),
        slug
    )
    .fetch_one(conn)
    .await?;

    Ok(prise)
}

// -----------------------------------------------------------------------------
// La naissance — une insertion, et l'idempotence portée par une contrainte
// -----------------------------------------------------------------------------

/// D'où vient le début, tel que la requête doit le composer.
///
/// **La conversion se fait en base** : une heure murale posée sur un jour civil
/// dans le fuseau d'une édition demande la base de fuseaux de PostgreSQL.
/// L'écrire en Rust demanderait une base de fuseaux — c'est le patron de B4, et
/// c'est ce qui a fait tomber le formulaire du front sur `Europe/Geneva`.
pub struct CreneauNaissant {
    /// Le créneau souhaité par l'organisation, quand il existe.
    pub souhaite: Option<time::OffsetDateTime>,
    /// L'heure d'ouverture quotidienne de l'appel, en texte — le repli se pose
    /// alors sur le **premier jour** de l'édition.
    pub heure_de_lappel: Option<String>,
    pub duree_minutes: i32,
}

/// Créer la séance de rang `rang` d'un dossier retenu.
///
/// # L'idempotence tombe d'une contrainte, jamais d'un décompte
///
/// `ux_sessions_proposal_sequence UNIQUE (proposal_id, sequence_number)` existe.
/// Une acceptation rejouée — un dossier remis en évaluation puis retenu de
/// nouveau, une action groupée passée deux fois — ne double donc aucune séance.
/// **On ne compte pas avant d'insérer** : compter puis insérer est une course,
/// la contrainte n'en est pas une (R6).
///
/// Rend `None` quand la séance existait déjà.
pub async fn creer(
    conn: &mut PgConnection,
    proposal_id: Uuid,
    rang: i16,
    slug: &str,
    creneau: &CreneauNaissant,
) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, proposal_id, organization_id, sequence_number,
                title, slug, summary, format, timezone, starts_at, ends_at)
           SELECT p.event_id, p.id, p.organization_id, $2,
                  p.title, $3::text::platform.slug, p.summary, p.format, e.timezone,
                  debut.instant,
                  debut.instant + make_interval(mins => $6)
             FROM programme.proposals p
             JOIN event.events e ON e.id = p.event_id
             CROSS JOIN LATERAL (
                 SELECT COALESCE(
                     $4::timestamptz,
                     CASE WHEN $5::text IS NOT NULL THEN
                         (((e.starts_at AT TIME ZONE e.timezone)::date::text)
                          || ' ' || $5::text)::timestamp AT TIME ZONE e.timezone
                     END,
                     e.starts_at
                 ) AS instant
             ) AS debut
            WHERE p.id = $1
           ON CONFLICT (proposal_id, sequence_number) DO NOTHING
        RETURNING id"#,
        proposal_id,
        rang,
        slug,
        creneau.souhaite,
        creneau.heure_de_lappel,
        creneau.duree_minutes
    )
    .fetch_optional(conn)
    .await?;

    Ok(id)
}

/// Ce qu'un dossier retenu apporte à ses séances.
#[derive(Debug, Clone)]
pub struct DossierARetenir {
    pub id: Uuid,
    pub event_id: Uuid,
    pub call_id: Option<Uuid>,
    /// Base de l'adresse d'URL : celle du dossier, déjà normalisée par
    /// `platform.slugify()` au dépôt. La renormaliser ici produirait deux
    /// normalisations divergentes du même texte (R7).
    pub slug: String,
    pub requested_sessions: i16,
    pub preferred_start_at: Option<time::OffsetDateTime>,
    pub duration_minutes: Option<i16>,
}

pub async fn dossier_a_retenir(
    conn: &mut PgConnection,
    proposal_id: Uuid,
) -> Result<Option<DossierARetenir>> {
    let ligne = sqlx::query!(
        r#"SELECT id, event_id, call_id, slug::text AS "slug!",
                  requested_sessions, preferred_start_at, duration_minutes
             FROM programme.proposals
            WHERE id = $1 AND deleted_at IS NULL"#,
        proposal_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| DossierARetenir {
        id: l.id,
        event_id: l.event_id,
        call_id: l.call_id,
        slug: l.slug,
        requested_sessions: l.requested_sessions,
        preferred_start_at: l.preferred_start_at,
        duration_minutes: l.duration_minutes,
    }))
}

/// L'heure d'ouverture quotidienne et la durée par défaut d'un appel, en une
/// lecture — les deux seules règles dont la naissance a besoin.
pub async fn regles_de_naissance(
    conn: &mut PgConnection,
    call_id: Uuid,
) -> Result<Option<(time::Time, i16)>> {
    let ligne = sqlx::query!(
        "SELECT daily_start_time, default_duration_minutes
           FROM event.calls_for_proposals WHERE id = $1",
        call_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| (l.daily_start_time, l.default_duration_minutes)))
}

// -----------------------------------------------------------------------------
// Les écritures du planificateur
//
// **Elles rendent l'erreur brute de PostgreSQL**, et non une erreur d'API : la
// traduction appartient au service, qui sait quel geste il est en train de faire
// — c'est la règle de B4, et elle vaut deux fois ici, où le même SQLSTATE sert à
// deux refus différents.
// -----------------------------------------------------------------------------

/// Le créneau d'une séance : sa salle, son début, sa fin, sa journée.
pub struct Creneau {
    /// Nul : la séance retourne au panneau « à placer ». **Ce n'est pas une
    /// suppression** — la séance existe, son créneau reste.
    pub room_id: Option<Uuid>,
    pub starts_at: time::OffsetDateTime,
    pub ends_at: time::OffsetDateTime,
    /// Fournie, elle est écrite telle quelle. **Absente, elle est remise à nul**
    /// pour que le déclencheur la redéduise — voir ci-dessous.
    pub event_day_id: Option<Uuid>,
}

/// Écrire le créneau, **et remettre la journée à nul quand elle n'est pas
/// fournie** (écart n° 113, R9).
///
/// # Pourquoi cette mise à nul n'est pas un détail
///
/// `tg_sessions_derive_fields()` ne déduit la journée que lorsque la colonne est
/// **nulle**. Une séance déjà rattachée qu'on déplace du 12 au 14 novembre
/// **resterait rattachée au 12**, en silence — et déplacer est le geste le plus
/// fréquent de tout l'écran. La programmation publique et le calendrier du
/// back-office rangeraient alors la séance au mauvais jour, et rien ne le
/// signalerait.
///
/// Recalculer la journée en Rust reproduirait une requête que la base porte
/// (`day_date = (starts_at AT TIME ZONE e.timezone)::date`) et rouvrirait la
/// question du fuseau ; modifier le déclencheur serait plus propre encore, et
/// demanderait de toucher au modèle.
pub async fn ecrire_le_creneau(
    conn: &mut PgConnection,
    session_id: SessionId,
    creneau: Creneau,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE programme.sessions
            SET room_id = $2, starts_at = $3, ends_at = $4, event_day_id = $5
          WHERE id = $1",
        session_id.as_uuid(),
        creneau.room_id,
        creneau.starts_at,
        creneau.ends_at,
        creneau.event_day_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Marquer une séance diffusée, **avec le canal choisi tel quel**.
///
/// Le canal par défaut est laissé au déclencheur : il ne le pose que lorsque la
/// colonne est nulle, il complète et n'écrase jamais (R8). Retirer la diffusion
/// efface le canal — c'est la base qui le fait, et c'est pour cela que le service
/// refuse un canal désigné dans ce cas.
pub async fn ecrire_la_diffusion(
    conn: &mut PgConnection,
    session_id: SessionId,
    diffusee: bool,
    canal: Option<Uuid>,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE programme.sessions
            SET is_streamed = $2, broadcast_channel_id = $3
          WHERE id = $1",
        session_id.as_uuid(),
        diffusee,
        canal
    )
    .execute(conn)
    .await?;

    Ok(())
}

// -----------------------------------------------------------------------------
// Ce que l'organisation voit de ses séances — TROIS NOMBRES, JAMAIS UN NOM
// -----------------------------------------------------------------------------

/// Les séances d'un dossier, avec leur salle et leurs trois nombres —
/// `TrackedSession[]` (écarts n° 36 et n° 108).
///
/// **Aucun nom d'inscrit n'entre dans cette lecture**, et c'est le point : une
/// organisation sait combien de personnes viendront, jamais qui. Le filtrage est
/// à la source, pas dans l'écran — la v1 renvoyait des données internes dans une
/// réponse JSON que l'interface n'affichait pas.
///
/// `registered_count` compte `registered` **et** `attended`, exactement comme
/// `v_public_schedule` et comme le contrôle de jauge : trois définitions du même
/// mot produiraient trois chiffres.
pub async fn seances_suivies<'e>(
    executor: impl PgExecutor<'e>,
    proposal_id: Uuid,
) -> Result<Vec<crate::domain::sessions::TrackedSession>> {
    let lignes = sqlx::query!(
        r#"SELECT to_jsonb(s) - 'search_vector' AS "session!",
                  to_jsonb(r) AS "room?",
                  s.capacity::int4 AS "capacity?",
                  (SELECT count(*) FROM programme.registrations rg
                    WHERE rg.session_id = s.id
                      AND rg.status IN ('registered', 'attended')) AS "confirmes!",
                  (SELECT count(*) FROM programme.registrations rg
                    WHERE rg.session_id = s.id AND rg.status = 'waitlisted')
                      AS "en_attente!"
             FROM programme.sessions s
             LEFT JOIN event.rooms r ON r.id = s.room_id
            WHERE s.proposal_id = $1
            ORDER BY s.sequence_number, s.starts_at"#,
        proposal_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| crate::domain::sessions::TrackedSession {
            session: l.session,
            room: l.room,
            registered_count: l.confirmes,
            waitlisted_count: l.en_attente,
            capacity: l.capacity,
            // **Vide jusqu'à B6, jamais absente** : le champ existe au contrat,
            // et le supprimer ferait échouer l'écran (écart n° 108).
            reminders: Vec::new(),
        })
        .collect())
}

/// Les séances **terminées sans compte rendu** d'un dossier — ce que
/// l'organisation doit encore fournir.
///
/// Une séance est terminée quand sa fin est passée ; le compte rendu manque
/// quand `report_submitted_at` est nul. Aucun écran ne l'écrit encore (écart
/// n° 122), mais l'**action** est servie : c'est ce que l'organisation seule peut
/// débloquer, et c'est le critère d'entrée dans le bloc.
pub async fn comptes_rendus_manquants<'e>(
    executor: impl PgExecutor<'e>,
    proposal_id: Uuid,
) -> Result<Vec<(Uuid, serde_json::Value, time::OffsetDateTime)>> {
    let lignes = sqlx::query!(
        "SELECT id, title, ends_at
           FROM programme.sessions
          WHERE proposal_id = $1
            AND ends_at < now()
            AND status <> 'cancelled'
            AND report_submitted_at IS NULL
          ORDER BY ends_at",
        proposal_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| (l.id, l.title, l.ends_at))
        .collect())
}
