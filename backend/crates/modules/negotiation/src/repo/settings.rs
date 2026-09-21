//! Les deux réglages de l'admission, lus dans `platform.settings`.
//!
//! **Relus à chaque tentative, sans cache** (recherche R4, SC-002). Le coût est
//! une lecture par clé primaire sur un geste rare ; le gain est qu'une bascule
//! au back-office se voie à l'entrée suivante, sans mise en ligne.
//!
//! `platform` est le noyau partagé du principe III : lire cette table n'est pas
//! une frontière de module, et aucun découplage ne la couperait.

use kernel::error::Result;
use sqlx::postgres::PgConnection;

use crate::domain::admission::{AdmissionMode, LimiteDEssais, CLE_ESSAIS, CLE_MODE};

pub async fn mode_dadmission(conn: &mut PgConnection) -> Result<AdmissionMode> {
    let brut = sqlx::query_scalar!(
        r#"SELECT (s.value #>> '{}') AS "valeur?"
             FROM platform.settings s
            WHERE s.key = $1"#,
        CLE_MODE
    )
    .fetch_optional(conn)
    .await?
    .flatten();

    AdmissionMode::lire(brut)
}

/// La limite d'essais. Chaque champ absent retombe sur la valeur du semis :
/// un réglage tronqué à la main ne doit pas fermer l'entrée, seulement perdre
/// ce qu'il a perdu.
pub async fn limite_dessais(conn: &mut PgConnection) -> Result<LimiteDEssais> {
    let defaut = LimiteDEssais::default();

    let ligne = sqlx::query!(
        r#"SELECT (s.value ->> 'max')::integer            AS "max?",
                  (s.value ->> 'window_minutes')::integer AS "fenetre?",
                  (s.value ->> 'lock_minutes')::integer   AS "verrou?"
             FROM platform.settings s
            WHERE s.key = $1"#,
        CLE_ESSAIS
    )
    .fetch_optional(conn)
    .await?;

    let Some(ligne) = ligne else {
        return Ok(defaut);
    };

    Ok(LimiteDEssais {
        max: ligne.max.filter(|m| *m > 0).unwrap_or(defaut.max),
        fenetre_minutes: ligne
            .fenetre
            .filter(|f| *f > 0)
            .unwrap_or(defaut.fenetre_minutes),
        verrou_minutes: ligne
            .verrou
            .filter(|v| *v > 0)
            .unwrap_or(defaut.verrou_minutes),
    })
}
