//! Les déclinaisons d'un objet : écriture, avancement, reprise.
//!
//! # Poids et instant de fabrication partent ENSEMBLE
//!
//! `ck_renditions_ready_shape` refuse une ligne `ready` dont le poids ou
//! l'instant manquerait. Écrire d'abord la ligne puis la compléter produirait
//! donc un refus de la base au milieu du traitement — sur une contrainte dont
//! le message ne dirait pas que le code a procédé en deux temps. Une seule
//! écriture, avec les deux valeurs.
//!
//! # Ce que la reprise ne refait pas
//!
//! La file est « au moins une fois », jamais « exactement une fois » : un worker
//! tué entre le travail et le marquage rejoue le travail entier. Deux gardes se
//! superposent, et elles sont volontairement redondantes — [`deja_faites`] évite
//! de refabriquer et de réécrire sur le stockage, `ON CONFLICT DO NOTHING`
//! empêche la seconde ligne si deux workers passent en même temps.
//!
//! La clause `ON CONFLICT` est **sans cible** : deux index uniques protègent
//! cette table — `ux_renditions` sur (objet, code, format) et
//! `ux_renditions_object_key` sur la clé de stockage —, et nommer le premier
//! laisserait le second sortir en erreur.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

/// Une déclinaison **fabriquée** : elle n'est écrite qu'une fois les octets
/// déposés sur le stockage.
#[derive(Debug, Clone)]
pub struct DeclinaisonPrete {
    pub asset_id: Uuid,
    pub variant_code: String,
    /// `media.rendition_format`, en texte — le patron des cinq modules livrés.
    pub format: String,
    pub width: i32,
    pub height: i32,
    pub object_key: String,
    pub byte_size: i64,
}

/// Écrit une déclinaison prête. Rend `false` quand elle existait déjà — une
/// reprise, et non une erreur.
pub async fn ecrire_prete(conn: &mut PgConnection, d: &DeclinaisonPrete) -> Result<bool> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO media.renditions
               (asset_id, variant_code, format, width, height, object_key,
                byte_size, status, generated_at)
           VALUES ($1, $2, $3::text::media.rendition_format, $4, $5, $6,
                   $7, 'ready', now())
           ON CONFLICT DO NOTHING
        RETURNING id"#,
        d.asset_id,
        d.variant_code,
        d.format,
        d.width,
        d.height,
        d.object_key,
        d.byte_size
    )
    .fetch_optional(conn)
    .await?;

    Ok(id.is_some())
}

/// Enregistre l'échec **définitif** d'une déclinaison, avec son motif.
///
/// Sans cette ligne, une déclinaison en échec et une déclinaison jamais demandée
/// se ressembleraient : l'écran d'avancement dirait « deux sur trois » sans
/// pouvoir dire pourquoi la troisième manque (FR-032).
pub async fn ecrire_echec(
    conn: &mut PgConnection,
    asset_id: Uuid,
    variant_code: &str,
    format: &str,
    object_key: &str,
    motif: &str,
) -> Result<()> {
    sqlx::query!(
        r#"INSERT INTO media.renditions
               (asset_id, variant_code, format, object_key, status, last_error)
           VALUES ($1, $2, $3::text::media.rendition_format, $4, 'failed', $5)
           ON CONFLICT DO NOTHING"#,
        asset_id,
        variant_code,
        format,
        object_key,
        motif
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Les couples (code, format) **déjà écrits**, quel que soit leur état.
///
/// L'état n'entre pas dans la question : `ux_renditions` porte sur le couple
/// seul, et une ligne en échec occupe la place aussi sûrement qu'une ligne
/// prête.
pub async fn deja_faites(conn: &mut PgConnection, asset_id: Uuid) -> Result<Vec<(String, String)>> {
    let lignes = sqlx::query!(
        r#"SELECT variant_code, format::text AS "format!"
             FROM media.renditions
            WHERE asset_id = $1"#,
        asset_id
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| (l.variant_code, l.format))
        .collect())
}

/// Le nombre de déclinaisons **prêtes** — le numérateur de l'avancement.
pub async fn compter_pretes(pool: &PgPool, asset_id: Uuid) -> Result<i64> {
    let compte = sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!"
             FROM media.renditions
            WHERE asset_id = $1 AND status = 'ready'"#,
        asset_id
    )
    .fetch_one(pool)
    .await?;

    Ok(compte)
}

/// Le motif du dernier échec de déclinaison, s'il y en a un.
pub async fn dernier_echec(pool: &PgPool, asset_id: Uuid) -> Result<Option<String>> {
    let motif = sqlx::query_scalar!(
        "SELECT last_error FROM media.renditions
          WHERE asset_id = $1 AND status = 'failed' AND last_error IS NOT NULL
          ORDER BY created_at DESC
          LIMIT 1",
        asset_id
    )
    .fetch_optional(pool)
    .await?
    .flatten();

    Ok(motif)
}

/// Les clés de stockage des déclinaisons d'un objet, **tous états confondus**.
///
/// Une ligne en échec porte elle aussi une clé — le service a pu la déposer
/// avant que l'écriture ne tombe. La purge la vise donc comme les autres :
/// supprimer ce qui n'existe pas est un succès, et laisser un octet derrière ne
/// l'est pas.
pub async fn cles_de(conn: &mut PgConnection, asset_id: Uuid) -> Result<Vec<String>> {
    let cles = sqlx::query_scalar!(
        "SELECT object_key FROM media.renditions WHERE asset_id = $1",
        asset_id
    )
    .fetch_all(conn)
    .await?;

    Ok(cles)
}

/// Efface les lignes de déclinaison d'un objet purgé.
///
/// **Sans effet sur les compteurs de quota** : `tg_track_storage_usage` sort
/// tout de suite quand l'objet porteur est déjà supprimé, et le bloc entier a
/// été rendu au moment de la suppression logique. Les garder dirait qu'une
/// déclinaison est prête à une clé qui n'existe plus.
pub async fn effacer_de(conn: &mut PgConnection, asset_id: Uuid) -> Result<u64> {
    let effacees = sqlx::query!("DELETE FROM media.renditions WHERE asset_id = $1", asset_id)
        .execute(conn)
        .await?
        .rows_affected();

    Ok(effacees)
}
