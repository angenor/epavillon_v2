//! Les essais de code, **comptés par personne**.
//!
//! # L'APPAREIL EST UNE INFORMATION, JAMAIS UNE BORNE
//!
//! `device_id` vient du corps de la requête : il se forge. Compter « par
//! personne **et** par appareil » suffirait à changer d'identifiant à chaque
//! essai pour que la limite ne limite rien. La fonction de comptage du modèle,
//! `invitation_attempts_recent(person, window)`, **ne prend délibérément pas
//! d'appareil en argument** : ce qui n'est pas passé ne peut pas être
//! contourné. L'appareil reste enregistré parce qu'il aide un administrateur à
//! lire une série d'échecs.
//!
//! # LE CODE ESSAYÉ N'EST JAMAIS ÉCRIT
//!
//! Ni en clair ni en empreinte. Un essai raté peut être le vrai code d'un autre
//! espace, et cette table se lit au back-office.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// Combien d'essais ont **consommé une tentative** sur la fenêtre.
///
/// Les essais acceptés n'y sont pas, et ceux que la limite a déjà refusés non
/// plus : sans cela, chaque appui sur le bouton pendant le verrou ferait
/// repartir la fenêtre, et le quart d'heure annoncé à l'écran ne finirait
/// jamais.
pub async fn recents(
    conn: &mut PgConnection,
    person_id: Uuid,
    fenetre_minutes: i32,
) -> Result<i32> {
    let compte = sqlx::query_scalar!(
        r#"SELECT negotiation.invitation_attempts_recent(
                      $1, make_interval(mins => $2)) AS "compte!""#,
        person_id,
        fenetre_minutes
    )
    .fetch_one(conn)
    .await?;

    Ok(compte)
}

/// Un essai, avec son issue. L'appareil est gardé tel qu'il s'est déclaré.
pub async fn enregistrer(
    conn: &mut PgConnection,
    person_id: Uuid,
    device_id: Option<&str>,
    outcome: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO negotiation.invitation_code_attempts (person_id, device_id, outcome)
         VALUES ($1, $2, $3::text::negotiation.invitation_attempt_outcome)",
        person_id,
        device_id,
        outcome
    )
    .execute(conn)
    .await?;

    Ok(())
}
