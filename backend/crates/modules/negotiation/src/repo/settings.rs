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

/// Bascule le mode d'admission.
///
/// **`INSERT … ON CONFLICT` plutôt qu'`UPDATE`** : une base dont le semis n'a
/// pas été rejoué n'a pas la ligne, et un `UPDATE` y échouerait en silence —
/// l'écran dirait « enregistré » et la tentative suivante lirait toujours
/// l'ancien mode.
///
/// `updated_by` est posé explicitement : `platform.settings` ne porte pas de
/// déclencheur d'audit, et sans cette colonne la bascule serait le seul geste
/// du back-office dont on ignorerait l'auteur (FR-046).
pub async fn ecrire_le_mode(
    conn: &mut PgConnection,
    mode: AdmissionMode,
    acteur: uuid::Uuid,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO platform.settings (key, value, description, updated_by)
         VALUES ($1, to_jsonb($2::text),
                 'Mode d''admission de Guide Négo : code, approval ou code_and_approval.', $3)
         ON CONFLICT (key) DO UPDATE
             SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by",
        CLE_MODE,
        mode.as_db(),
        acteur
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Le bucket fermé au web où vivent les PDF et les images de page.
pub async fn bucket_prive(conn: &mut PgConnection) -> Result<String> {
    sqlx::query_scalar!(
        r#"SELECT (s.value #>> '{}') AS "valeur?" FROM platform.settings s
            WHERE s.key = 'media.private_bucket'"#
    )
    .fetch_optional(conn)
    .await?
    .flatten()
    .ok_or_else(|| kernel::error::ApiError::internal("réglage media.private_bucket absent"))
}
