//! Le calendrier des rappels d'une séance, **lu par la fonction du modèle**.
//!
//! # Une fonction, deux lecteurs, une seule agrégation
//!
//! `engagement.session_reminder_schedule()` est appelée ici **et** par la
//! composition de l'espace organisation, côté `programme`. Écrites séparément,
//! les deux agrégations divergeraient au premier ajustement de la consolidation
//! — et la divergence serait silencieuse : un nombre de destinataires faux
//! ressemble à un nombre juste (FR-052).
//!
//! # Une fonction qui rend une TABLE ne porte aucune contrainte de nullité
//!
//! SQLx type donc **toutes** ses colonnes en `Option`, y compris celles que la
//! requête ne peut pas rendre nulles. Les annoter une à une est la leçon de B3 :
//! sans cela, la structure porterait sept `Option` et l'écran devrait deviner
//! lesquelles sont de vraies absences. Seuls `skip_reason` et `sent_at` en
//! sont : le premier ne sort que sur un groupe mort, le second tant que rien
//! n'est parti.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::reminder::ReminderSlot;

/// Une ligne par (décalage, canal), rangée du décalage le plus lointain au plus
/// proche — l'ordre que la fonction impose elle-même (FR-050).
pub async fn calendrier(pool: &PgPool, session_id: Uuid) -> Result<Vec<ReminderSlot>> {
    let lignes = sqlx::query!(
        r#"SELECT offset_minutes  AS "offset_minutes!",
                  channel         AS "channel!",
                  scheduled_for   AS "scheduled_for!",
                  status          AS "status!",
                  recipient_count AS "recipient_count!",
                  skip_reason,
                  sent_at
             FROM engagement.session_reminder_schedule($1)"#,
        session_id
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ReminderSlot {
            offset_before: l.offset_minutes,
            channel: l.channel,
            scheduled_for: l.scheduled_for,
            status: l.status,
            recipient_count: l.recipient_count,
            skip_reason: l.skip_reason,
            sent_at: l.sent_at,
        })
        .collect())
}

// -----------------------------------------------------------------------------
// Matérialisation, réactivation, décalage, annulation
// -----------------------------------------------------------------------------

/// **La matérialisation passe par la fonction du modèle, et rien d'autre.**
///
/// Elle insère les rappels manquants, **met un travail par rappel en file** et
/// **émet** `engagement.reminders.scheduled`. Le service ne redouble ni l'un ni
/// l'autre : il produirait deux courriels par rappel, et le doublon ne se
/// verrait qu'en production.
pub async fn materialiser(conn: &mut PgConnection, session_id: Uuid) -> Result<i32> {
    let crees = sqlx::query_scalar!(
        r#"SELECT engagement.schedule_session_reminders($1) AS "crees!""#,
        session_id
    )
    .fetch_one(conn)
    .await?;

    Ok(crees)
}

/// **Réactive les rappels d'une inscription reprise** — le cas que la clé
/// d'unicité rend piégeux.
///
/// `ux_scheduled_reminders_once` porte sur (séance, personne, canal, décalage)
/// **sans condition d'état** : la ligne annulée existe toujours, et
/// `ON CONFLICT DO NOTHING` ne la ressuscite pas. Sans cette remise à l'état
/// d'attente, qui se désiste puis revient ne recevrait **plus jamais rien** —
/// sans erreur, sans trace (R21).
///
/// **`job_id` est conservé, et c'est délibéré.** Annuler un rappel ne
/// décommande pas son travail : celui-ci est toujours en file, et il partira à
/// l'heure dite. La condition « l'instant est encore devant » garantit qu'il
/// n'a pas encore tourné — un travail déjà passé aurait un instant derrière
/// nous. Le lui rendre à `NULL` ferait tenter une seconde mise en file que
/// `ux_jobs_idempotency` absorberait, et la ligne resterait sans travail.
pub async fn reactiver(
    conn: &mut PgConnection,
    session_id: Uuid,
    person_id: Option<Uuid>,
) -> Result<u64> {
    let reprises = sqlx::query!(
        "UPDATE engagement.scheduled_reminders
            SET status = 'pending', skip_reason = NULL
          WHERE session_id = $1
            AND ($2::uuid IS NULL OR person_id = $2)
            AND status = 'cancelled'
            AND scheduled_for > now()",
        session_id,
        person_id
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(reprises)
}

/// Les rappels **déplacés** d'un créneau qui bouge, et les travaux qu'il faut
/// déplacer avec eux.
pub struct Deplacement {
    pub deplaces: u64,
    /// Les travaux encore en file qui portent ces rappels. Leur échéance vit
    /// dans `platform.jobs`, que ce module n'écrit pas.
    pub job_ids: Vec<Uuid>,
    /// Les rappels dont l'instant est passé sous le nez : écartés, jamais
    /// envoyés en rattrapage.
    pub depasses: u64,
}

/// **Déplace les instants d'envoi, il ne les recrée pas.**
///
/// Recréer se heurterait à la clé d'unicité, qui ne porte pas l'instant : les
/// lignes existantes resteraient à l'ancienne heure et rien ne le dirait. Le
/// décalage est le même pour tous — quand un créneau bouge, tous les rappels
/// qui en dépendent bougent d'autant.
pub async fn decaler(
    conn: &mut PgConnection,
    session_id: Uuid,
    secondes: f64,
    motif_depasse: &str,
) -> Result<Deplacement> {
    let lignes = sqlx::query!(
        r#"UPDATE engagement.scheduled_reminders
              SET scheduled_for = scheduled_for + make_interval(secs => $2)
            WHERE session_id = $1 AND status IN ('pending', 'queued')
        RETURNING id, job_id"#,
        session_id,
        secondes
    )
    .fetch_all(&mut *conn)
    .await?;

    let job_ids: Vec<Uuid> = lignes.iter().filter_map(|l| l.job_id).collect();

    // Un créneau qui recule peut faire passer un décalage derrière nous. On ne
    // rattrape pas : c'est la règle que la fonction du modèle applique déjà à la
    // création, et l'appliquer ici évite qu'un report d'une heure réveille tout
    // le monde d'un coup.
    let depasses = sqlx::query!(
        "UPDATE engagement.scheduled_reminders
            SET status = 'skipped', skip_reason = $2
          WHERE session_id = $1
            AND status IN ('pending', 'queued')
            AND scheduled_for <= now()",
        session_id,
        motif_depasse
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();

    Ok(Deplacement {
        deplaces: lignes.len() as u64,
        job_ids,
        depasses,
    })
}

/// Annule les rappels **encore à traiter** d'une séance, ou d'une personne sur
/// cette séance, **avec leur motif**.
///
/// Les rappels déjà partis ne sont pas touchés : ils sont partis, et le dire
/// autrement serait faux. Le motif est ce qui distingue une annulation d'un
/// oubli — sans lui, l'organisation lirait « rien n'est parti » sans savoir
/// pourquoi (FR-065).
pub async fn annuler(
    conn: &mut PgConnection,
    session_id: Uuid,
    person_id: Option<Uuid>,
    motif: &str,
) -> Result<u64> {
    let annules = sqlx::query!(
        "UPDATE engagement.scheduled_reminders
            SET status = 'cancelled', skip_reason = $3
          WHERE session_id = $1
            AND ($2::uuid IS NULL OR person_id = $2)
            AND status IN ('pending', 'queued')",
        session_id,
        person_id,
        motif
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(annules)
}

// -----------------------------------------------------------------------------
// Ce que le travail d'envoi lit et écrit
// -----------------------------------------------------------------------------

/// Un rappel à traiter, avec ce qu'il faut pour décider s'il part.
#[derive(Debug, Clone)]
pub struct RappelATraiter {
    pub id: Uuid,
    pub session_id: Uuid,
    pub person_id: Uuid,
    pub channel: String,
    pub offset_minutes: i32,
    pub status: String,
    /// Le type de notification de la règle qui l'a produit. Replié sur le
    /// défaut du modèle quand la règle a été supprimée — `rule_id` est
    /// `ON DELETE SET NULL`, et un rappel orphelin doit tout de même savoir
    /// quel avis il porte.
    pub type_code: String,
    pub template_id: Option<Uuid>,
}

pub async fn a_traiter(pool: &PgPool, reminder_id: Uuid) -> Result<Option<RappelATraiter>> {
    let ligne = sqlx::query!(
        r#"SELECT sr.id, sr.session_id, sr.person_id,
                  sr.channel::text AS "channel!",
                  round(extract(epoch FROM sr.offset_before) / 60)::int AS "offset_minutes!",
                  sr.status::text AS "status!",
                  COALESCE(r.type_code, 'programme.session.reminder') AS "type_code!",
                  r.template_id
             FROM engagement.scheduled_reminders sr
             LEFT JOIN engagement.reminder_rules r ON r.id = sr.rule_id
            WHERE sr.id = $1"#,
        reminder_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| RappelATraiter {
        id: l.id,
        session_id: l.session_id,
        person_id: l.person_id,
        channel: l.channel,
        offset_minutes: l.offset_minutes,
        status: l.status,
        type_code: l.type_code,
        template_id: l.template_id,
    }))
}

/// **Marque le rappel parti AVANT l'envoi, et rend faux si un autre l'a déjà
/// fait.**
///
/// L'ordre est le cœur de « une fois, et une seule ». La file est « au moins une
/// fois » : un worker tué entre l'envoi et son marquage rejoue le travail
/// entier. Marquer après l'envoi produirait alors **deux courriels** — le défaut
/// exact que la v1 avait, et que la clé d'unicité du modèle ne rattrape pas,
/// puisqu'elle interdit deux lignes, pas deux envois sur la même ligne.
///
/// Le prix est écrit : un worker tué **entre** ce marquage et l'envoi perd un
/// courriel. Un rappel manquant se voit et se rejoue à la main ; un rappel
/// envoyé deux fois est irrattrapable. [`rendre_a_la_file`] limite la perte aux
/// morts brutales, les échecs annoncés étant rendus.
pub async fn marquer_parti(conn: &mut PgConnection, reminder_id: Uuid) -> Result<bool> {
    let pris = sqlx::query!(
        "UPDATE engagement.scheduled_reminders
            SET status = 'sent', sent_at = now(), skip_reason = NULL
          WHERE id = $1 AND status IN ('pending', 'queued')",
        reminder_id
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(pris == 1)
}

/// Rend le rappel à la file après un échec d'expédition **annoncé**. Le travail
/// se reprend ; sans ce retour, il trouverait la ligne déjà partie et n'enverrait
/// jamais rien.
pub async fn rendre_a_la_file(conn: &mut PgConnection, reminder_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE engagement.scheduled_reminders
            SET status = 'queued', sent_at = NULL
          WHERE id = $1 AND status = 'sent'",
        reminder_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// **Écarte un rappel avec son motif, jamais en silence** (FR-065).
///
/// Une adresse supprimée, un canal coupé, un canal sans expédition : trois
/// raisons différentes de ne rien envoyer, et l'organisation qui lit son
/// calendrier a droit à laquelle.
pub async fn marquer_ecarte(conn: &mut PgConnection, reminder_id: Uuid, motif: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE engagement.scheduled_reminders
            SET status = 'skipped', skip_reason = $2
          WHERE id = $1 AND status IN ('pending', 'queued')",
        reminder_id,
        motif
    )
    .execute(conn)
    .await?;
    Ok(())
}
