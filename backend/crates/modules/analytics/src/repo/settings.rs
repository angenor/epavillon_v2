//! `platform.settings` — **hors `cross/`**.
//!
//! Le principe III nomme `platform` comme noyau partagé : le ranger dans
//! `cross/` ferait perdre au dossier son sens, qui est de lister exactement les
//! frontières à trancher le jour où le module deviendrait un service autonome.
//! Aucun découplage ne couperait la table des réglages.

use kernel::error::Result;
use sqlx::postgres::PgConnection;

/// Le seuil au-delà duquel un dossier sans évaluation cesse d'être une alerte.
///
/// **Il vit en base et non dans le code** (écart n° 43, ouvert le 17/08) : c'est
/// une règle d'exploitation que l'IFDD ajuste d'une COP à l'autre, sans
/// redéploiement — et l'écrire côté serveur aurait seulement déplacé la dette
/// que le site portait.
///
/// Le repli sur 21 vaut pour une base dont le semis n'aurait pas été rechargé :
/// il rend l'écran lisible plutôt que vide, et le fait sur la valeur même que
/// `130_analytics.sql` déclare.
pub const SEUIL_PAR_DEFAUT: i32 = 21;

pub async fn jours_avant_alerte(conn: &mut PgConnection) -> Result<i32> {
    let valeur = sqlx::query_scalar!(
        r#"SELECT (s.value #>> '{}') AS "valeur?"
             FROM platform.settings s
            WHERE s.key = 'analytics.review_alert_days'"#
    )
    .fetch_optional(conn)
    .await?
    .flatten()
    .and_then(|v| v.parse::<i32>().ok())
    .unwrap_or(SEUIL_PAR_DEFAUT);

    Ok(valeur)
}
