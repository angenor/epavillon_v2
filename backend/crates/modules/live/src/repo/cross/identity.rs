//! Lecture de `identity.people`, en lecture seule : qui entre dans la salle
//! d'une séance en entrée libre, sans inscription pour le dire.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

pub struct Personne {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub display_name: String,
}

pub async fn personne(conn: &mut PgConnection, person_id: Uuid) -> Result<Option<Personne>> {
    let ligne = sqlx::query_as!(
        Personne,
        r#"SELECT primary_email::text AS "email!", first_name, last_name,
                  display_name AS "display_name!"
             FROM identity.people WHERE id = $1"#,
        person_id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne)
}
