//! Les inscriptions — **et le verrou qui manque à la base** (écart n° 124, R19).
//!
//! # 🔴 Toute écriture commence par prendre la séance en verrou
//!
//! `tg_validate_registration()` contrôle la jauge par un `count(*)` **sans
//! verrou**. Sous `READ COMMITTED`, deux inscriptions simultanées lisent toutes
//! deux neuf places prises sur dix et insèrent toutes deux : **onze inscrits sur
//! dix places**, et cela ne se verrait que le jour de l'activité. La position en
//! liste d'attente souffre du même défaut : elle vaut le maximum en cours plus
//! un, lu sans verrou lui aussi, et **aucun index unique ne la protège**. Deux
//! personnes peuvent donc recevoir le même rang, ce qui ne se verrait jamais.
//!
//! Poser ce verrou **ne réimplémente aucun invariant** (principe VIII) : il rend
//! sûr, sous concurrence, un contrôle que la base fait déjà et fait mal.
//!
//! # Ce qu'il coûte
//!
//! Les inscriptions à une **même** séance se sérialisent. Une séance reçoit
//! quelques dizaines à quelques centaines d'inscriptions sur plusieurs semaines ;
//! deux inscriptions strictement simultanées sont déjà l'exception. Les
//! inscriptions à des séances différentes ne se gênent pas.
//!
//! # Les refus de PostgreSQL remontent BRUTS
//!
//! Le même SQLSTATE sert à deux gestes — inscrire et annuler — et seul le
//! service sait lequel il fait. La traduction lui appartient donc, comme en B4.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::ids::{RegistrationId, SessionId};

/// Ce que le service a besoin de savoir de la séance, **sous verrou**.
#[derive(Debug, Clone)]
pub struct SeanceVerrouillee {
    pub id: Uuid,
    pub event_id: Uuid,
    pub status: String,
    pub registration_required: bool,
    pub registration_opens_at: Option<time::OffsetDateTime>,
    pub registration_closes_at: Option<time::OffsetDateTime>,
    pub capacity: Option<i32>,
    pub waitlist_enabled: bool,
}

/// **Prendre la ligne de la séance en verrou**, et rendre ce qui décide de
/// l'inscription.
///
/// Le verrou tient jusqu'à la fin de la transaction : c'est ce qui sérialise le
/// contrôle de jauge du déclencheur et l'attribution du rang d'attente.
pub async fn verrouiller(
    conn: &mut PgConnection,
    session_id: SessionId,
) -> Result<Option<SeanceVerrouillee>> {
    let ligne = sqlx::query!(
        r#"SELECT id, event_id, status::text AS "status!", registration_required,
                  registration_opens_at, registration_closes_at,
                  capacity, waitlist_enabled
             FROM programme.sessions
            WHERE id = $1
              FOR UPDATE"#,
        session_id.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| SeanceVerrouillee {
        id: l.id,
        event_id: l.event_id,
        status: l.status,
        registration_required: l.registration_required,
        registration_opens_at: l.registration_opens_at,
        registration_closes_at: l.registration_closes_at,
        capacity: l.capacity,
        waitlist_enabled: l.waitlist_enabled,
    }))
}

/// L'inscription **vivante** d'une personne à une séance, s'il y en a une.
///
/// `ux_registrations_person_session` est un index **partiel** — il exclut les
/// annulations —, et c'est ce qui permet une réinscription après annulation.
pub async fn inscription_vivante(
    conn: &mut PgConnection,
    session_id: SessionId,
    person_id: Uuid,
) -> Result<Option<serde_json::Value>> {
    let ligne = sqlx::query_scalar!(
        r#"SELECT to_jsonb(r) AS "ligne!"
             FROM programme.registrations r
            WHERE r.session_id = $1 AND r.person_id = $2 AND r.status <> 'cancelled'"#,
        session_id.as_uuid(),
        person_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne)
}

/// Insérer l'inscription. **L'état obtenu est celui que la base a posé** : la
/// bascule automatique en liste d'attente est laissée faire, et le service lit
/// ce qui en est sorti (data-model § 4).
pub async fn inscrire(
    conn: &mut PgConnection,
    session_id: SessionId,
    person_id: Uuid,
    organization_id: Option<Uuid>,
    answers: &serde_json::Value,
    locale: &str,
) -> std::result::Result<serde_json::Value, sqlx::Error> {
    sqlx::query_scalar!(
        r#"WITH nouvelle AS (
               INSERT INTO programme.registrations
                   (session_id, person_id, organization_id, answers, locale)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING *
           )
           SELECT to_jsonb(nouvelle) AS "ligne!" FROM nouvelle"#,
        session_id.as_uuid(),
        person_id,
        organization_id,
        answers,
        locale
    )
    .fetch_one(conn)
    .await
}

/// Annuler une inscription, avec son motif.
///
/// Rend `None` quand l'inscription était déjà annulée : rejouer une annulation
/// ne libère aucune place, et promouvoir sur cette base ferait dépasser la jauge.
pub async fn annuler(
    conn: &mut PgConnection,
    registration_id: RegistrationId,
    motif: Option<&str>,
) -> std::result::Result<Option<(serde_json::Value, String)>, sqlx::Error> {
    let ligne = sqlx::query!(
        r#"WITH annulee AS (
               UPDATE programme.registrations
                  SET status = 'cancelled',
                      waitlist_position = NULL,
                      cancelled_at = now(),
                      cancelled_reason = $2
                WHERE id = $1 AND status <> 'cancelled'
               RETURNING *
           )
           SELECT to_jsonb(annulee) AS "ligne!",
                  (SELECT r.status::text FROM programme.registrations r WHERE r.id = $1)
                      AS "precedent!"
             FROM annulee"#,
        registration_id.as_uuid(),
        motif
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| (l.ligne, l.precedent)))
}

/// L'état d'une inscription **avant** toute écriture — qui elle vise, à qui elle
/// appartient, et si elle est vivante.
#[derive(Debug, Clone)]
pub struct EtatDInscription {
    pub session_id: Uuid,
    pub person_id: Uuid,
    pub status: String,
}

pub async fn etat<'e>(
    executor: impl PgExecutor<'e>,
    registration_id: RegistrationId,
) -> Result<Option<EtatDInscription>> {
    let ligne = sqlx::query!(
        r#"SELECT session_id, person_id, status::text AS "status!"
             FROM programme.registrations WHERE id = $1"#,
        registration_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| EtatDInscription {
        session_id: l.session_id,
        person_id: l.person_id,
        status: l.status,
    }))
}

/// Promouvoir depuis la liste d'attente — **la fonction du modèle, avec le
/// nombre que le service décide**.
///
/// Le contrôle de capacité du déclencheur ne porte que sur l'insertion, et une
/// promotion est une mise à jour : promouvoir plus que le nombre de places
/// libérées ferait dépasser la jauge **sans un mot**. Le service compte donc ce
/// qu'il libère — une annulation, une place, une promotion (R20).
pub async fn promouvoir(conn: &mut PgConnection, session_id: Uuid, combien: i32) -> Result<i64> {
    let promus = sqlx::query_scalar!(
        "SELECT programme.promote_from_waitlist($1, $2)",
        session_id,
        combien
    )
    .fetch_one(conn)
    .await?;

    Ok(promus.unwrap_or(0) as i64)
}

/// La **première présence**, écrite une seule fois par la fonction du modèle.
///
/// `record_join()` pose `COALESCE(joined_at, now())` : un second clic sur
/// « Rejoindre » ne l'écrase pas, et c'est ce qui donne un taux de présence réel.
pub async fn rejoindre(
    conn: &mut PgConnection,
    registration_id: RegistrationId,
) -> Result<Option<time::OffsetDateTime>> {
    let instant = sqlx::query_scalar!(
        "SELECT programme.record_join($1)",
        registration_id.as_uuid()
    )
    .fetch_one(conn)
    .await?;

    Ok(instant)
}

/// La liste **nominative** des inscrits d'une séance — `RegistrationRow[]`.
///
/// Elle exige `programme.registration.manage`. Le rôle de programmation ne la
/// détient pas (écart n° 119) : composer la grille et voir qui vient sont deux
/// droits distincts, et c'est une ligne de la table des droits.
pub async fn liste_nominative<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Vec<serde_json::Value>> {
    let lignes = sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
                     'registration', to_jsonb(r),
                     'person', jsonb_build_object(
                         'id', p.id,
                         'display_name', p.display_name,
                         'first_name', p.first_name,
                         'last_name', p.last_name,
                         'primary_email', p.primary_email
                     ),
                     'organization_name', o.legal_name
                 ) AS "ligne!"
             FROM programme.registrations r
             JOIN identity.people p ON p.id = r.person_id
             LEFT JOIN org.organizations o ON o.id = r.organization_id
            WHERE r.session_id = $1
            ORDER BY r.created_at"#,
        session_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// « Mes inscriptions » — **annulations comprises**, comme le contrat le dit.
pub async fn mes_inscriptions<'e>(
    executor: impl PgExecutor<'e>,
    person_id: Uuid,
) -> Result<Vec<serde_json::Value>> {
    let lignes = sqlx::query_scalar!(
        r#"SELECT to_jsonb(r) AS "ligne!"
             FROM programme.registrations r
            WHERE r.person_id = $1
            ORDER BY r.created_at DESC"#,
        person_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes)
}

/// Les trois nombres d'une séance, **pour l'organisation qui la porte**.
///
/// `registered_count` compte `registered` **et** `attended` — exactement le
/// prédicat de la vue publique et du contrôle de jauge. Trois définitions du même
/// mot produiraient trois chiffres, et c'est l'organisation qui s'en apercevrait.
pub async fn decomptes<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<(i64, i64)> {
    let ligne = sqlx::query!(
        r#"SELECT count(*) FILTER (WHERE status IN ('registered', 'attended'))
                      AS "confirmes!",
                  count(*) FILTER (WHERE status = 'waitlisted') AS "en_attente!"
             FROM programme.registrations WHERE session_id = $1"#,
        session_id.as_uuid()
    )
    .fetch_one(executor)
    .await?;

    Ok((ligne.confirmes, ligne.en_attente))
}
