//! Lecture du schéma `org` — **en lecture seule**.
//!
//! « Quelles organisations puis-je viser ? »
//!
//! **Celles qui ANIMENT au moins une activité de l'édition, et elles seules.**
//! C'est exactement le critère que `live.event_incidents()` applique à la portée
//! `organization` : un message visant une ONG ne concerne cette édition que si
//! l'ONG y porte une activité. Offrir au choix des organisations que la lecture
//! écarterait ensuite ferait publier un bandeau que personne ne verrait.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

pub struct OrganisationCible {
    pub id: Uuid,
    pub legal_name: String,
    pub acronym: Option<String>,
}

pub async fn animant_ledition(
    conn: &mut PgConnection,
    event_id: Uuid,
) -> Result<Vec<OrganisationCible>> {
    let lignes = sqlx::query!(
        r#"SELECT DISTINCT o.id, o.legal_name, o.acronym
             FROM org.organizations o
            WHERE EXISTS (
                      SELECT 1 FROM programme.sessions s
                       WHERE s.organization_id = o.id AND s.event_id = $1
                  )
            ORDER BY o.legal_name"#,
        event_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OrganisationCible {
            id: l.id,
            legal_name: l.legal_name,
            acronym: l.acronym,
        })
        .collect())
}
