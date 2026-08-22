//! Les rattachements : la table blanche, la pose, le retrait, la lecture.
//!
//! # Ce fichier n'écrit rien hors de `media`
//!
//! Rattacher une image à une édition **n'écrit pas dans `event`** : la ligne
//! vit dans `media.attachments`, et c'est tout l'intérêt du polymorphisme
//! contrôlé. C'est ce qui referme l'obligation que B3 avait laissée sans
//! modifier une ligne de son module.
//!
//! # Les refus de la base remontent BRUTS
//!
//! `media.tg_validate_attachment()` lève ses cinq refus par `RAISE EXCEPTION`
//! **sans nom de contrainte**, et trois partagent le même `SQLSTATE`. Traduire
//! ici perdrait le `SQLSTATE`, et le service ne pourrait plus les distinguer par
//! ce qu'il a lui-même vérifié en amont. C'est le patron de `assets::ecrire`.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::attachment::AttachedMedia;
use crate::domain::rules::AttachableRoleRule;

/// Une ligne de la table blanche, telle que le service la consulte.
///
/// Le rapport attendu et sa tolérance traversent **en flottant** ici — le
/// service compare —, et **en texte** dans [`AttachableRoleRule`], que l'API
/// rend : `numeric(6,4)` n'a pas de représentant flottant exact, et un rapport
/// affiché doit l'être tel qu'il est déclaré.
#[derive(Debug, Clone)]
pub struct Regle {
    pub is_multiple: bool,
    pub allowed_mime_prefixes: Vec<String>,
    pub max_byte_size: Option<i64>,
    pub expected_aspect_ratio: Option<f64>,
    pub aspect_ratio_tolerance: f64,
}

/// La règle d'un couple (entité, rôle). `None` : le rôle n'est pas déclaré pour
/// cette entité, ou il est désactivé — et le rattachement est refusé.
pub async fn regle(
    pool: &PgPool,
    owner_schema: &str,
    owner_table: &str,
    role: &str,
) -> Result<Option<Regle>> {
    let ligne = sqlx::query!(
        r#"SELECT r.is_multiple,
                  r.allowed_mime_prefixes           AS "prefixes!",
                  r.max_byte_size,
                  r.expected_aspect_ratio::float8   AS ratio,
                  r.aspect_ratio_tolerance::float8  AS "tolerance!"
             FROM media.attachable_roles r
            WHERE r.owner_schema = $1 AND r.owner_table = $2
              AND r.role = $3::text::media.attachment_role
              AND r.is_active"#,
        owner_schema,
        owner_table,
        role
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| Regle {
        is_multiple: l.is_multiple,
        allowed_mime_prefixes: l.prefixes,
        max_byte_size: l.max_byte_size,
        expected_aspect_ratio: l.ratio,
        aspect_ratio_tolerance: l.tolerance,
    }))
}

/// Toutes les règles déclarées pour une entité — ce qu'un écran annonce au lieu
/// de le deviner.
///
/// **Les rôles inactifs en font partie**, avec leur drapeau : les masquer
/// laisserait un écran croire qu'un rôle n'a jamais existé, là où il a été
/// fermé.
pub async fn regles_de(
    pool: &PgPool,
    owner_schema: &str,
    owner_table: &str,
) -> Result<Vec<AttachableRoleRule>> {
    let lignes = sqlx::query!(
        r#"SELECT r.owner_schema, r.owner_table,
                  r.role::text  AS "role!",
                  r.label       AS "label: serde_json::Value",
                  r.is_multiple,
                  r.allowed_mime_prefixes AS "prefixes!",
                  r.max_byte_size,
                  r.expected_aspect_ratio::text  AS ratio,
                  r.aspect_ratio_tolerance::text AS "tolerance!",
                  r.is_active
             FROM media.attachable_roles r
            WHERE r.owner_schema = $1 AND r.owner_table = $2
            ORDER BY r.role"#,
        owner_schema,
        owner_table
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| AttachableRoleRule {
            owner_schema: l.owner_schema,
            owner_table: l.owner_table,
            role: l.role,
            label: l.label,
            is_multiple: l.is_multiple,
            allowed_mime_prefixes: l.prefixes,
            max_byte_size: l.max_byte_size,
            expected_aspect_ratio: l.ratio,
            aspect_ratio_tolerance: l.tolerance,
            is_active: l.is_active,
        })
        .collect())
}

/// Ce que le service écrit d'un rattachement. `is_exclusive` n'y est pas : le
/// déclencheur le pose depuis la règle, et l'écrire ici en ferait une seconde
/// définition de la multiplicité.
#[derive(Debug, Clone)]
pub struct NouveauRattachement<'a> {
    pub owner_schema: &'a str,
    pub owner_table: &'a str,
    pub owner_id: Uuid,
    pub asset_id: Uuid,
    pub role: &'a str,
    pub sort_order: i16,
    pub alt_text_override: Option<serde_json::Value>,
    pub created_by: Uuid,
}

/// Pose un rattachement. **Rend le refus de la base tel quel** : c'est le
/// service qui le nomme, d'après ce qu'il a vérifié en amont.
pub async fn poser(
    conn: &mut PgConnection,
    n: &NouveauRattachement<'_>,
) -> std::result::Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO media.attachments
               (owner_schema, owner_table, owner_id, asset_id, role,
                sort_order, alt_text_override, created_by)
           VALUES ($1, $2, $3, $4, $5::text::media.attachment_role,
                   $6, $7::jsonb::platform.i18n_text, $8)
        RETURNING id"#,
        n.owner_schema,
        n.owner_table,
        n.owner_id,
        n.asset_id,
        n.role,
        n.sort_order,
        n.alt_text_override,
        n.created_by
    )
    .fetch_one(conn)
    .await
}

/// L'entité et le rôle d'un rattachement — de quoi appliquer la garde avant de
/// le retirer.
#[derive(Debug, Clone)]
pub struct RattachementVise {
    pub owner_schema: String,
    pub owner_table: String,
    pub owner_id: Uuid,
    pub asset_id: Uuid,
}

pub async fn par_id(pool: &PgPool, attachment_id: Uuid) -> Result<Option<RattachementVise>> {
    let ligne = sqlx::query!(
        "SELECT owner_schema, owner_table, owner_id, asset_id
           FROM media.attachments WHERE id = $1",
        attachment_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| RattachementVise {
        owner_schema: l.owner_schema,
        owner_table: l.owner_table,
        owner_id: l.owner_id,
        asset_id: l.asset_id,
    }))
}

/// Retire un rattachement. **L'objet stocké n'est pas touché** : il peut servir
/// ailleurs, et la déduplication fait qu'il sert souvent ailleurs (écart n° 128).
pub async fn retirer(conn: &mut PgConnection, attachment_id: Uuid) -> Result<bool> {
    let retires = sqlx::query!("DELETE FROM media.attachments WHERE id = $1", attachment_id)
        .execute(conn)
        .await?
        .rows_affected();

    Ok(retires > 0)
}

/// Vide un rôle d'une entité. Rend le nombre de rattachements retirés — et
/// **aucun objet** n'est supprimé.
pub async fn vider_role(
    conn: &mut PgConnection,
    owner_schema: &str,
    owner_table: &str,
    owner_id: Uuid,
    role: &str,
) -> Result<u64> {
    let retires = sqlx::query!(
        "DELETE FROM media.attachments
          WHERE owner_schema = $1 AND owner_table = $2 AND owner_id = $3
            AND role = $4::text::media.attachment_role",
        owner_schema,
        owner_table,
        owner_id,
        role
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(retires)
}

/// Le rang suivant dans un rôle multiple. Zéro quand le rôle est vide.
pub async fn prochain_ordre(
    conn: &mut PgConnection,
    owner_schema: &str,
    owner_table: &str,
    owner_id: Uuid,
    role: &str,
) -> Result<i16> {
    let suivant = sqlx::query_scalar!(
        r#"SELECT COALESCE(max(sort_order) + 1, 0)::smallint AS "suivant!"
             FROM media.attachments
            WHERE owner_schema = $1 AND owner_table = $2 AND owner_id = $3
              AND role = $4::text::media.attachment_role"#,
        owner_schema,
        owner_table,
        owner_id,
        role
    )
    .fetch_one(conn)
    .await?;

    Ok(suivant)
}

/// Les médias d'une entité, dans l'ordre de tri déclaré.
///
/// # Pourquoi cette lecture ne filtre pas sur l'état servable
///
/// `media.attached_image()` ne rend que les objets `ready` : c'est **la**
/// lecture publique, celle que servent les pages de B3, B4 et B5, et FR-033 y
/// est tenue par la base.
///
/// Celle-ci sert l'écran qui **gère** les médias d'une entité. Un objet déposé
/// il y a trois secondes est parfaitement valide et pas encore servable ; le
/// masquer ferait croire que le téléversement a échoué. Il est donc rendu, avec
/// son état — et `sources`, qui vient de la même fonction que la lecture
/// publique, y est vide tant que rien n'a été fabriqué.
///
/// Un objet **supprimé**, lui, n'est pas rendu : là, il n'existe plus.
pub async fn par_entite(
    pool: &PgPool,
    owner_schema: &str,
    owner_table: &str,
    owner_id: Uuid,
    role: Option<&str>,
) -> Result<Vec<AttachedMedia>> {
    let lignes = sqlx::query!(
        r#"SELECT t.id,
                  t.role::text AS "role!",
                  t.sort_order,
                  t.asset_id,
                  media.object_url(a.bucket, a.object_key) AS "url!",
                  a.width, a.height,
                  COALESCE(t.alt_text_override, a.alt_text) AS "alt_text: serde_json::Value",
                  a.caption AS "caption: serde_json::Value",
                  a.credit,
                  media.asset_sources(a.id) AS "sources!",
                  a.status::text AS "status!"
             FROM media.attachments t
             JOIN media.assets a ON a.id = t.asset_id
            WHERE t.owner_schema = $1 AND t.owner_table = $2 AND t.owner_id = $3
              AND ($4::text IS NULL OR t.role::text = $4)
              AND a.deleted_at IS NULL
            ORDER BY t.role, t.sort_order, t.created_at"#,
        owner_schema,
        owner_table,
        owner_id,
        role
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| AttachedMedia {
            attachment_id: l.id,
            role: l.role,
            sort_order: l.sort_order,
            asset_id: l.asset_id,
            url: l.url,
            width: l.width,
            height: l.height,
            alt_text: l.alt_text,
            caption: l.caption,
            credit: l.credit,
            sources: l.sources,
            status: l.status,
        })
        .collect())
}

/// **Combien d'entités utilisent cet objet ?**
///
/// La question qui décide d'une suppression : un objet dédupliqué appartient à
/// la première organisation qui l'a déposé, et la laisser le supprimer ferait
/// disparaître l'image de la seconde (écart n° 128, R11).
pub async fn compter_usages(pool: &PgPool, asset_id: Uuid) -> Result<i64> {
    let usages = sqlx::query_scalar!(
        r#"SELECT count(*) AS "usages!" FROM media.attachments WHERE asset_id = $1"#,
        asset_id
    )
    .fetch_one(pool)
    .await?;

    Ok(usages)
}

/// **Combien d'entités utilisent cet objet.**
///
/// La déduplication traverse les propriétaires : le même fichier déposé par deux
/// organisations donne **une** ligne, appartenant à la première (écart n° 128).
/// Ce compte est ce qui empêche la première de faire disparaître l'image de la
/// seconde — et il voyage dans le refus, parce que « ce fichier est utilisé
/// ailleurs » sans chiffre ne dit pas quoi vérifier.
pub async fn compter_pour_objet(pool: &PgPool, asset_id: Uuid) -> Result<i64> {
    let compte = sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!" FROM media.attachments WHERE asset_id = $1"#,
        asset_id
    )
    .fetch_one(pool)
    .await?;

    Ok(compte)
}
