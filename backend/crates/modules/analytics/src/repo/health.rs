//! `analytics.v_operational_health` et `analytics.refresh_log`.
//!
//! **Les seuils ne sont pas recalculés** : le modèle porte déjà la décision de
//! ce qui mérite attention, et la redoubler en Rust ferait deux vérités. La vue
//! est rendue telle quelle, par le **code** de l'indicateur — le libellé
//! français n'étant qu'un repli d'affichage (écart n° 45).

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;

use crate::domain::dashboard::OperationalHealthRow;

pub async fn sante(conn: &mut PgConnection) -> Result<Vec<OperationalHealthRow>> {
    let lignes = sqlx::query_as!(
        OperationalHealthRow,
        r#"SELECT code            AS "code!",
                  libelle         AS "libelle!",
                  domaine         AS "domaine!",
                  valeur          AS "valeur!",
                  seuil_attention AS "seuil_attention!",
                  seuil_critique  AS "seuil_critique!",
                  gravite         AS "gravite!",
                  detail          AS "detail!: Value",
                  mesure_le       AS "mesure_le!"
             FROM analytics.v_operational_health"#
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes)
}

/// L'âge des projections — **`max(finished_at)` SUR LES SUCCÈS**, et non la
/// dernière ligne du journal.
///
/// Une exécution partielle laisse des lignes en échec **plus récentes** que le
/// dernier succès complet : prendre la dernière ligne ferait avancer la
/// fraîcheur affichée alors que les chiffres, eux, n'ont pas bougé. C'est
/// exactement le genre de défaut qu'un journal existe pour empêcher.
///
/// `None` quand aucun rafraîchissement n'a jamais abouti — l'écran le dit alors,
/// plutôt que d'inventer une date.
pub async fn rafraichi_le(conn: &mut PgConnection) -> Result<Option<OffsetDateTime>> {
    let instant = sqlx::query_scalar!(
        r#"SELECT max(rl.finished_at) AS "instant?"
             FROM analytics.refresh_log rl
            WHERE rl.succeeded"#
    )
    .fetch_one(conn)
    .await?;

    Ok(instant)
}
