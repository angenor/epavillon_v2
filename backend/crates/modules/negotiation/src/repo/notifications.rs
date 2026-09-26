//! L'accord « Notifications » d'« À propos » (research R10) : une ligne
//! d'`identity.consents` par bascule, la version du texte servi avec elle.
//! Écriture hors schéma, comme `programme/src/repo/consents.rs`.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// La finalité, écrite ici et jamais reçue.
pub const FINALITE: &str = "guide_nego_notifications";
const SOURCE: &str = "profile_settings";

/// Aucune ligne : allumé.
pub async fn accord(conn: &mut PgConnection, person_id: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        "SELECT is_granted FROM identity.current_consents WHERE person_id = $1 AND purpose = $2",
        person_id,
        FINALITE
    )
    .fetch_optional(conn)
    .await?
    .flatten()
    .unwrap_or(true))
}

/// `clock_timestamp()` : deux bascules d'une même transaction restent ordonnées
/// pour `current_consents`.
pub async fn consigner(
    conn: &mut PgConnection,
    person_id: Uuid,
    accorde: bool,
    version: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO identity.consents
             (person_id, purpose, is_granted, policy_version, source, recorded_at)
         VALUES ($1, $2, $3, $4, $5, clock_timestamp())",
        person_id,
        FINALITE,
        accorde,
        version,
        SOURCE
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Sérialise les bascules d'une même personne.
pub async fn verrouiller(conn: &mut PgConnection, person_id: Uuid) -> Result<()> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0)) AS verrou",
        format!("{FINALITE}:{person_id}")
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}
