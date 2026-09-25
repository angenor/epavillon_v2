//! Les lectures de la FAQ : un compteur par entrée et par jour de Paris, sans
//! auteur ni appareil (R12).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// Un brouillon, ou une entrée inconnue, ne se compte pas : rien n'est écrit.
pub async fn compter(conn: &mut PgConnection, entry_id: Uuid) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.faq_reads (entry_id, day)
         SELECT id, (now() AT TIME ZONE 'Europe/Paris')::date
           FROM negotiation.faq_entries
          WHERE id = $1 AND status <> 'draft'
         ON CONFLICT (entry_id, day) DO UPDATE SET count = negotiation.faq_reads.count + 1",
        entry_id
    )
    .execute(conn)
    .await?;
    Ok(())
}
