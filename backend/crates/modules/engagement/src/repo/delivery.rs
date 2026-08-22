//! Les deux questions posées avant tout envoi, et les deux réponses viennent de
//! la base.
//!
//! Ni l'une ni l'autre n'est réimplémentée ici : `is_channel_enabled()` porte
//! déjà le repli sur les canaux par défaut du type et le passage en force des
//! avis critiques, et `is_email_suppressed()` porte l'expiration d'une
//! suppression temporaire. Les réécrire donnerait deux réponses à la même
//! question.

use kernel::error::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// La personne accepte-t-elle ce type d'avis sur ce canal ?
///
/// Fonction **totale** : un type inconnu vaut refus. C'est écrit dans le
/// modèle, et c'est ce qui empêche une faute de frappe de faire partir un
/// courriel que personne n'a demandé.
pub async fn canal_autorise(
    pool: &PgPool,
    person_id: Uuid,
    type_code: &str,
    canal: &str,
) -> Result<bool> {
    let autorise = sqlx::query_scalar!(
        r#"SELECT engagement.is_channel_enabled(
                      $1, $2, $3::text::engagement.notification_channel
                  ) AS "autorise!""#,
        person_id,
        type_code,
        canal
    )
    .fetch_one(pool)
    .await?;

    Ok(autorise)
}

/// L'adresse est-elle hors du circuit ?
///
/// La garde d'envoi la consulte elle aussi, et c'est **volontairement en
/// double** : elle écarte le message, ce fichier écrit le **motif** sur la ligne
/// de rappel. Sans lui, l'organisation lirait « rien n'est parti » sans savoir
/// pourquoi.
pub async fn adresse_supprimee(pool: &PgPool, adresse: &str) -> Result<bool> {
    let supprimee = sqlx::query_scalar!(
        r#"SELECT engagement.is_email_suppressed($1::text::platform.email) AS "supprimee!""#,
        adresse
    )
    .fetch_one(pool)
    .await?;

    Ok(supprimee)
}
