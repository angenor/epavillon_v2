//! Les préférences par type **et** par canal.
//!
//! # L'absence de ligne n'est pas un refus
//!
//! *« Absence de ligne = on retombe sur `default_channels` du type »* : personne
//! n'a rien à configurer pour que la plateforme se comporte correctement. La
//! lecture compose donc le catalogue **et** les arbitrages, jamais les seuls
//! arbitrages — une liste vide ferait croire qu'aucun avis n'est servi.
//!
//! # `is_overridable` est le champ qui compte
//!
//! Une préférence posée sur un type **critique** est enregistrée telle quelle —
//! l'API ne refuse pas —, mais `is_channel_enabled()` l'ignore. Sans ce champ,
//! l'écran afficherait un interrupteur éteint pour un avis qui part quand même,
//! et la personne croirait s'être désabonnée (FR-095).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::notification::NotificationPreferenceRow;

/// Le catalogue croisé avec les arbitrages de la personne, canal par canal.
pub async fn lister(
    pool: &PgPool,
    person_id: Uuid,
    locale: &str,
) -> Result<Vec<NotificationPreferenceRow>> {
    let lignes = sqlx::query!(
        r#"SELECT nt.code AS "type_code!",
                  platform.t(nt.label, $2) AS "label!",
                  nt.description,
                  nt.module_code,
                  nt.criticality::text AS "criticality!",
                  c.canal::text AS "channel!",
                  -- Le repli est celui du modèle : sans arbitrage, le canal est
                  -- servi s'il figure dans les canaux par défaut du type.
                  COALESCE(np.is_enabled, c.canal = ANY (nt.default_channels)) AS "is_enabled!",
                  (nt.criticality <> 'critical') AS "is_overridable!"
             FROM engagement.notification_types nt
             CROSS JOIN unnest(enum_range(NULL::engagement.notification_channel)) AS c(canal)
             LEFT JOIN engagement.notification_preferences np
                    ON np.type_code = nt.code
                   AND np.person_id = $1
                   AND np.channel   = c.canal
            WHERE nt.is_active
            ORDER BY nt.module_code, nt.code, c.canal"#,
        person_id,
        locale
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| NotificationPreferenceRow {
            type_code: l.type_code,
            label: serde_json::Value::String(l.label),
            description: l.description,
            module_code: l.module_code,
            criticality: l.criticality,
            channel: l.channel,
            is_enabled: l.is_enabled,
            is_overridable: l.is_overridable,
        })
        .collect())
}

/// Pose un arbitrage. **Même sur un type critique** : l'écriture est
/// enregistrée, et c'est la lecture qui dit qu'elle n'oppose rien. Refuser
/// laisserait l'écran sans réponse à donner.
pub async fn ecrire(
    conn: &mut PgConnection,
    person_id: Uuid,
    type_code: &str,
    canal: &str,
    actif: bool,
) -> std::result::Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO engagement.notification_preferences
             (person_id, type_code, channel, is_enabled)
         VALUES ($1, $2, $3::text::engagement.notification_channel, $4)
         ON CONFLICT (person_id, type_code, channel)
             DO UPDATE SET is_enabled = EXCLUDED.is_enabled",
        person_id,
        type_code,
        canal,
        actif
    )
    .execute(conn)
    .await?;
    Ok(())
}
