//! Les objets stockés : écriture, lecture, recherche par empreinte.
//!
//! # Ce que ce fichier N'ÉCRIT PAS, et pourquoi
//!
//! Ni mise en file du traitement, ni émission de l'annonce de dépôt : insérer
//! une ligne dans `media.assets` déclenche `media.tg_enqueue_processing()`, qui
//! **fait déjà les deux**. Un `enqueue` de plus produirait deux traitements par
//! fichier, et le doublon ne se verrait qu'en production (FR-020).
//!
//! Ni les dimensions, ni le verdict d'analyse : ils appartiennent au travail
//! différé. Ni `deleted_at`, ni `purge_after` : `media.schedule_asset_purge()`
//! les pose. Ni les compteurs de quota : les déclencheurs les tiennent.
//!
//! # L'adresse est COMPOSÉE, jamais stockée
//!
//! `media.object_url()` la compose depuis le point d'accès courant, lu dans les
//! réglages. Aucune requête d'ici ne rend une clé nue : c'est la promesse du
//! modèle, celle qui rend une migration de stockage indolore (FR-021).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::asset::{Asset, OrphanAsset};

/// Ce que le service écrit d'un objet, et rien de plus.
#[derive(Debug, Clone)]
pub struct NouvelObjet {
    pub bucket: String,
    pub object_key: String,
    /// Calculée **pendant la réception**, jamais reçue du client sans être
    /// recalculée.
    pub checksum_sha256: String,
    pub mime_type: String,
    /// Le poids **réellement reçu**, jamais celui qui était annoncé.
    pub byte_size: i64,
    pub original_filename: Option<String>,
    pub owner_person_id: Option<Uuid>,
    pub owner_organization_id: Option<Uuid>,
    /// `media.asset_visibility`, en texte.
    pub visibility: String,
    pub alt_text: Option<serde_json::Value>,
    pub caption: Option<serde_json::Value>,
    pub credit: Option<String>,
    pub license_code: Option<String>,
}

/// Écrit la description d'un objet. **L'état posé est `uploaded`**, le seul que
/// le service pose ; tout le reste appartient au travail différé.
///
/// **Rend le refus de la base tel quel**, et non traduit : `tg_enforce_quota`
/// lève `SQLSTATE 53100`, que le service doit reconnaître pour le rendre sous
/// le même code stable que son propre contrôle préalable (R14). Une traduction
/// ici perdrait le `SQLSTATE` et ferait sortir un refus métier en erreur
/// système. C'est le patron des cinq modules livrés.
pub async fn ecrire(
    conn: &mut PgConnection,
    objet: &NouvelObjet,
) -> std::result::Result<Uuid, sqlx::Error> {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO media.assets
               (bucket, object_key, checksum_sha256, mime_type, byte_size,
                original_filename, owner_person_id, owner_organization_id,
                visibility, status, alt_text, caption, credit, license_code)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8,
                   $9::text::media.asset_visibility, 'uploaded',
                   $10::jsonb::platform.i18n_text, $11::jsonb::platform.i18n_text, $12, $13)
        RETURNING id"#,
        objet.bucket,
        objet.object_key,
        objet.checksum_sha256,
        objet.mime_type,
        objet.byte_size,
        objet.original_filename,
        objet.owner_person_id,
        objet.owner_organization_id,
        objet.visibility,
        objet.alt_text,
        objet.caption,
        objet.credit,
        objet.license_code
    )
    .fetch_one(conn)
    .await?;

    Ok(id)
}

/// L'objet **vivant** qui porte cette empreinte dans ce dépôt, s'il existe.
///
/// C'est `media.find_by_checksum()` du modèle, appelée telle quelle plutôt que
/// réécrite : elle porte déjà la condition « non supprimé », et
/// `ux_assets_checksum` est l'index qui la sert.
pub async fn par_empreinte(
    conn: &mut PgConnection,
    empreinte: &str,
    bucket: &str,
) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!("SELECT media.find_by_checksum($1, $2)", empreinte, bucket)
        .fetch_one(conn)
        .await?;

    Ok(id)
}

/// La même question, hors transaction — pour la pré-vérification, qui n'écrit
/// rien et n'a donc aucune raison d'en ouvrir une.
pub async fn par_empreinte_en_lecture(
    pool: &PgPool,
    empreinte: &str,
    bucket: &str,
) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!("SELECT media.find_by_checksum($1, $2)", empreinte, bucket)
        .fetch_one(pool)
        .await?;

    Ok(id)
}

/// Un objet et **tout ce que l'écran en lit** : l'adresse composée de l'original
/// et ses déclinaisons prêtes.
///
/// **Un objet supprimé n'est pas rendu.** Un objet en traitement, en échec ou en
/// quarantaine l'est : « en traitement » n'est pas « absent », et l'écran doit
/// pouvoir le dire ([`contracts/errors.md`](../../../../../specs/006-media-engagement/contracts/errors.md)).
pub async fn par_id(pool: &PgPool, asset_id: Uuid) -> Result<Option<Asset>> {
    let ligne = sqlx::query!(
        r#"SELECT a.id,
                  a.bucket,
                  a.object_key,
                  a.checksum_sha256,
                  a.mime_type,
                  a.byte_size,
                  a.original_filename,
                  a.width,
                  a.height,
                  a.duration_seconds::text AS duration_seconds,
                  a.owner_person_id,
                  a.owner_organization_id,
                  a.visibility::text  AS "visibility!",
                  a.status::text      AS "status!",
                  a.scan_verdict::text AS "scan_verdict!",
                  a.scan_engine,
                  a.scanned_at,
                  a.scan_details,
                  a.alt_text  AS "alt_text: serde_json::Value",
                  a.caption   AS "caption: serde_json::Value",
                  a.credit,
                  a.license_code,
                  a.deleted_at,
                  a.deleted_by,
                  a.purge_after,
                  a.purged_at,
                  a.created_at,
                  a.updated_at,
                  media.object_url(a.bucket, a.object_key) AS "url!",
                  media.asset_sources(a.id) AS "sources!"
             FROM media.assets a
            WHERE a.id = $1 AND a.deleted_at IS NULL"#,
        asset_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| Asset {
        id: l.id,
        bucket: l.bucket,
        object_key: l.object_key,
        checksum_sha256: l.checksum_sha256,
        mime_type: l.mime_type,
        byte_size: l.byte_size,
        original_filename: l.original_filename,
        width: l.width,
        height: l.height,
        duration_seconds: l.duration_seconds,
        owner_person_id: l.owner_person_id,
        owner_organization_id: l.owner_organization_id,
        visibility: l.visibility,
        status: l.status,
        scan_verdict: l.scan_verdict,
        scan_engine: l.scan_engine,
        scanned_at: l.scanned_at,
        scan_details: l.scan_details,
        alt_text: l.alt_text,
        caption: l.caption,
        credit: l.credit,
        license_code: l.license_code,
        deleted_at: l.deleted_at,
        deleted_by: l.deleted_by,
        purge_after: l.purge_after,
        purged_at: l.purged_at,
        created_at: l.created_at,
        updated_at: l.updated_at,
        url: l.url,
        sources: l.sources,
    }))
}

/// La même lecture, **dans la transaction** qui vient d'écrire — pour rendre
/// l'objet sans rouvrir une connexion, et sans risquer de le lire avant que la
/// transaction ne soit visible.
pub async fn par_id_dans(conn: &mut PgConnection, asset_id: Uuid) -> Result<Option<Asset>> {
    let ligne = sqlx::query!(
        r#"SELECT a.id, a.bucket, a.object_key, a.checksum_sha256, a.mime_type,
                  a.byte_size, a.original_filename, a.width, a.height,
                  a.duration_seconds::text AS duration_seconds,
                  a.owner_person_id, a.owner_organization_id,
                  a.visibility::text   AS "visibility!",
                  a.status::text       AS "status!",
                  a.scan_verdict::text AS "scan_verdict!",
                  a.scan_engine, a.scanned_at, a.scan_details,
                  a.alt_text AS "alt_text: serde_json::Value",
                  a.caption  AS "caption: serde_json::Value",
                  a.credit, a.license_code,
                  a.deleted_at, a.deleted_by, a.purge_after, a.purged_at,
                  a.created_at, a.updated_at,
                  media.object_url(a.bucket, a.object_key) AS "url!",
                  media.asset_sources(a.id) AS "sources!"
             FROM media.assets a
            WHERE a.id = $1 AND a.deleted_at IS NULL"#,
        asset_id
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| Asset {
        id: l.id,
        bucket: l.bucket,
        object_key: l.object_key,
        checksum_sha256: l.checksum_sha256,
        mime_type: l.mime_type,
        byte_size: l.byte_size,
        original_filename: l.original_filename,
        width: l.width,
        height: l.height,
        duration_seconds: l.duration_seconds,
        owner_person_id: l.owner_person_id,
        owner_organization_id: l.owner_organization_id,
        visibility: l.visibility,
        status: l.status,
        scan_verdict: l.scan_verdict,
        scan_engine: l.scan_engine,
        scanned_at: l.scanned_at,
        scan_details: l.scan_details,
        alt_text: l.alt_text,
        caption: l.caption,
        credit: l.credit,
        license_code: l.license_code,
        deleted_at: l.deleted_at,
        deleted_by: l.deleted_by,
        purge_after: l.purge_after,
        purged_at: l.purged_at,
        created_at: l.created_at,
        updated_at: l.updated_at,
        url: l.url,
        sources: l.sources,
    }))
}

/// Le bucket par défaut, lu dans `platform.settings` — jamais écrit en dur : le
/// modèle le déclare comme réglage précisément pour qu'il puisse changer.
pub async fn bucket_par_defaut(pool: &PgPool) -> Result<String> {
    let valeur = sqlx::query_scalar!(
        "SELECT s.value #>> '{}' FROM platform.settings s WHERE s.key = 'media.default_bucket'"
    )
    .fetch_optional(pool)
    .await?
    .flatten();

    Ok(valeur.unwrap_or_else(|| "epavillon".to_owned()))
}

/// L'instant courant de la **base**, qui date la clé d'objet.
///
/// L'horloge du processus et celle de la base peuvent différer de quelques
/// secondes ; un objet déposé le 1er du mois à minuit se rangerait alors dans le
/// mois précédent, et l'on chercherait longtemps pourquoi.
pub async fn maintenant(pool: &PgPool) -> Result<OffsetDateTime> {
    let instant = sqlx::query_scalar!(r#"SELECT now() AS "maintenant!""#)
        .fetch_one(pool)
        .await?;
    Ok(instant)
}

// -----------------------------------------------------------------------------
// Le traitement différé
//
// Ces écritures appartiennent au travail de fond, jamais au service : elles
// posent ce que le dépôt a laissé vide — dimensions, verdict d'analyse, état
// final. Chacune est **conditionnée sur l'état de départ**, ce qui rend la
// reprise sans effet quand elle a déjà eu lieu.
// -----------------------------------------------------------------------------

/// Ce dont le traitement a besoin, et rien de plus. Ce que l'écran d'avancement
/// lit, lui, est [`EtatDeTraitement`] : deux lectures pour deux questions, plutôt
/// qu'une structure qui grossit jusqu'à recopier la table.
#[derive(Debug, Clone)]
pub struct AObjetTraiter {
    pub id: Uuid,
    pub object_key: String,
    pub mime_type: String,
    pub status: String,
}

/// L'objet visé par un travail de traitement. `None` quand il a été supprimé
/// entre la mise en file et le passage du worker — ce qui n'est pas une erreur :
/// le travail n'a simplement plus d'objet.
pub async fn pour_traitement(pool: &PgPool, asset_id: Uuid) -> Result<Option<AObjetTraiter>> {
    let ligne = sqlx::query!(
        r#"SELECT id, object_key, mime_type, status::text AS "status!"
             FROM media.assets
            WHERE id = $1 AND deleted_at IS NULL"#,
        asset_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| AObjetTraiter {
        id: l.id,
        object_key: l.object_key,
        mime_type: l.mime_type,
        status: l.status,
    }))
}

/// Fait passer un objet à un état de traitement, **sans toucher aux états
/// terminaux**. Une reprise qui trouverait l'objet déjà servable ou en
/// quarantaine ne le ferait pas régresser.
pub async fn poser_etat(conn: &mut PgConnection, asset_id: Uuid, etat: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE media.assets
            SET status = $2::text::media.asset_status
          WHERE id = $1
            AND status NOT IN ('ready', 'quarantined')",
        asset_id,
        etat
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Le verdict d'analyse, son moteur et son instant.
///
/// **Le moteur est toujours écrit**, `none` compris : la trace doit dire qui a
/// regardé, ou que personne ne l'a fait (R13).
pub async fn enregistrer_analyse(
    conn: &mut PgConnection,
    asset_id: Uuid,
    verdict: &str,
    moteur: &str,
    details: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        r#"UPDATE media.assets
              SET scan_verdict = $2::text::media.scan_verdict,
                  scan_engine  = $3,
                  scanned_at   = now(),
                  scan_details = CASE WHEN $4::text IS NULL THEN NULL
                                      ELSE jsonb_build_object('message', $4::text) END
            WHERE id = $1"#,
        asset_id,
        verdict,
        moteur,
        details
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Les dimensions d'une image, ou la durée d'un média temporel.
///
/// La durée traverse **en texte** : `numeric(10,3)` n'a pas d'équivalent
/// flottant sans perte, et l'arrondi silencieux d'un `f64` ferait diverger la
/// valeur écrite de la valeur relevée.
pub async fn enregistrer_mesure(
    conn: &mut PgConnection,
    asset_id: Uuid,
    dimensions: Option<(i32, i32)>,
    duree_secondes: Option<String>,
) -> Result<()> {
    sqlx::query!(
        "UPDATE media.assets
            SET width = $2, height = $3, duration_seconds = $4::text::numeric
          WHERE id = $1",
        asset_id,
        dimensions.map(|d| d.0),
        dimensions.map(|d| d.1),
        duree_secondes
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// L'objet devient servable.
///
/// **Les deux invariants ne sont pas revérifiés ici** : `ck_assets_scan_before_ready`
/// et `ck_assets_alt_text_required` sont portés par la base, et le traitement se
/// contente de traduire leur refus en motif d'échec. Les recopier ferait une
/// seconde définition de la servabilité.
pub async fn marquer_servable(conn: &mut PgConnection, asset_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE media.assets SET status = 'ready' WHERE id = $1 AND status <> 'ready'",
        asset_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// La quarantaine : terminale, et jamais servie. `media.asset_sources()` et
/// `media.attached_image()` ne rendent que l'état servable, et
/// `tg_validate_attachment` refuse tout rattachement visant cet objet.
pub async fn mettre_en_quarantaine(conn: &mut PgConnection, asset_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE media.assets SET status = 'quarantined' WHERE id = $1",
        asset_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// L'échec **définitif** — posé seulement quand le travail a épuisé ses essais.
///
/// Le motif ne va pas sur l'objet : aucune colonne de `media.assets` ne le
/// porterait, et en ajouter une serait modifier le modèle pour une trace.
/// Il vit là où la file l'écrit déjà — `platform.jobs.last_error` — et, pour ce
/// qui touche une déclinaison, dans `media.renditions.last_error`. L'avancement
/// les rassemble.
pub async fn marquer_echec(conn: &mut PgConnection, asset_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE media.assets
            SET status = 'failed'
          WHERE id = $1 AND status NOT IN ('ready', 'quarantined')",
        asset_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

/// Le motif du dernier échec du travail de traitement de cet objet.
///
/// `platform.jobs` est une table du **noyau**, pas d'un module métier : la lire
/// n'est pas une lecture inter-modules. C'est elle qui porte le motif, parce que
/// c'est elle qui a exécuté le travail.
pub async fn motif_dechec(pool: &PgPool, asset_id: Uuid) -> Result<Option<String>> {
    let motif = sqlx::query_scalar!(
        "SELECT last_error FROM platform.jobs
          WHERE task = 'media.process_asset' AND idempotency_key = $1::text
            AND last_error IS NOT NULL
          ORDER BY created_at DESC
          LIMIT 1",
        asset_id.to_string()
    )
    .fetch_optional(pool)
    .await?
    .flatten();

    Ok(motif)
}

/// Ce que l'écran d'avancement lit d'un objet — **y compris quand il n'est pas
/// servable**. « En traitement » n'est pas « absent », et distinguer les deux
/// demande que la lecture le dise.
#[derive(Debug, Clone)]
pub struct EtatDeTraitement {
    pub asset_id: Uuid,
    pub status: String,
    pub scan_verdict: String,
    pub scan_engine: Option<String>,
    pub mime_type: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

pub async fn etat_de_traitement(pool: &PgPool, asset_id: Uuid) -> Result<Option<EtatDeTraitement>> {
    let ligne = sqlx::query!(
        r#"SELECT id,
                  status::text       AS "status!",
                  scan_verdict::text AS "scan_verdict!",
                  scan_engine, mime_type, width, height
             FROM media.assets
            WHERE id = $1 AND deleted_at IS NULL"#,
        asset_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| EtatDeTraitement {
        asset_id: l.id,
        status: l.status,
        scan_verdict: l.scan_verdict,
        scan_engine: l.scan_engine,
        mime_type: l.mime_type,
        width: l.width,
        height: l.height,
    }))
}

/// Ce que le rattachement vérifie d'un objet avant de le poser.
///
/// Les quatre contrôles de forme — type, poids, cadrage, servabilité — se font
/// contre ces valeurs, **avant** l'écriture. Non pour remplacer
/// `tg_validate_attachment`, qui reste le dernier mot, mais pour savoir lequel
/// de ses cinq refus vient de tomber : il les lève sans nom de contrainte, et
/// trois partagent le même `SQLSTATE`.
#[derive(Debug, Clone)]
pub struct ObjetARattacher {
    pub mime_type: String,
    pub byte_size: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub status: String,
}

pub async fn pour_rattachement(pool: &PgPool, asset_id: Uuid) -> Result<Option<ObjetARattacher>> {
    let ligne = sqlx::query!(
        r#"SELECT mime_type, byte_size, width, height, status::text AS "status!"
             FROM media.assets
            WHERE id = $1 AND deleted_at IS NULL"#,
        asset_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| ObjetARattacher {
        mime_type: l.mime_type,
        byte_size: l.byte_size,
        width: l.width,
        height: l.height,
        status: l.status,
    }))
}

// -----------------------------------------------------------------------------
// Les orphelins et la purge
//
// Les deux fonctions du modèle sont **appelées, jamais réécrites** :
// `find_orphan_assets()` range déjà du plus lourd au plus léger et exclut ce qui
// est rattaché ; `schedule_asset_purge()` pose la suppression logique, sa
// fenêtre de rétention, **et émet son annonce**. Un `emit` de plus ici ferait
// deux annonces pour une intention.
// -----------------------------------------------------------------------------

/// Les objets prêts que plus rien n'utilise depuis N jours, du plus lourd au
/// plus léger — `media.find_orphan_assets()`.
pub async fn orphelins(pool: &PgPool, anciennete_jours: i32) -> Result<Vec<OrphanAsset>> {
    let lignes = sqlx::query!(
        r#"SELECT asset_id AS "asset_id!", bucket AS "bucket!", object_key AS "object_key!",
                  byte_size AS "byte_size!", rendition_bytes AS "rendition_bytes!",
                  owner_organization_id, created_at AS "created_at!", age_days AS "age_days!"
             FROM media.find_orphan_assets($1)"#,
        anciennete_jours
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OrphanAsset {
            asset_id: l.asset_id,
            bucket: l.bucket,
            object_key: l.object_key,
            byte_size: l.byte_size,
            rendition_bytes: l.rendition_bytes,
            owner_organization_id: l.owner_organization_id,
            created_at: l.created_at,
            age_days: l.age_days,
        })
        .collect())
}

/// L'ancienneté minimale à partir de laquelle un objet non rattaché est proposé
/// à la purge, lue dans les réglages — jamais écrite en dur.
pub async fn anciennete_dorphelin(pool: &PgPool) -> Result<i32> {
    let valeur = sqlx::query_scalar!(
        "SELECT s.value #>> '{}' FROM platform.settings s
          WHERE s.key = 'media.orphan_retention_days'"
    )
    .fetch_optional(pool)
    .await?
    .flatten();

    Ok(valeur.and_then(|v| v.parse().ok()).unwrap_or(30))
}

/// Ce que le propriétaire lit avant de supprimer : à qui l'objet appartient.
#[derive(Debug, Clone, Copy)]
pub struct ObjetASupprimer {
    pub owner_person_id: Option<Uuid>,
    pub owner_organization_id: Option<Uuid>,
}

pub async fn pour_suppression(pool: &PgPool, asset_id: Uuid) -> Result<Option<ObjetASupprimer>> {
    let ligne = sqlx::query!(
        "SELECT owner_person_id, owner_organization_id
           FROM media.assets
          WHERE id = $1 AND deleted_at IS NULL",
        asset_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| ObjetASupprimer {
        owner_person_id: l.owner_person_id,
        owner_organization_id: l.owner_organization_id,
    }))
}

/// Suppression logique avec fenêtre de rétention — `media.schedule_asset_purge()`.
///
/// Rend l'instant à partir duquel l'objet quittera le stockage. La consommation
/// baisse **immédiatement**, dès la suppression logique : c'est le déclencheur
/// de quota qui la rend, pas la purge (FR-106).
///
/// **La fenêtre de rétention n'est pas passée** : le modèle porte sa propre
/// valeur par défaut, et la redire ici en ferait une seconde définition, que la
/// première évolution du SQL ferait diverger.
pub async fn programmer_la_purge(
    conn: &mut PgConnection,
    asset_id: Uuid,
) -> Result<Option<OffsetDateTime>> {
    sqlx::query!("SELECT media.schedule_asset_purge($1)", asset_id)
        .execute(&mut *conn)
        .await?;

    let instant = sqlx::query_scalar!(
        "SELECT purge_after FROM media.assets WHERE id = $1",
        asset_id
    )
    .fetch_optional(&mut *conn)
    .await?
    .flatten();

    Ok(instant)
}

/// Un objet dont la fenêtre de rétention est échue, et ce que son annonce de
/// disparition devra porter.
#[derive(Debug, Clone)]
pub struct ObjetAPurger {
    pub id: Uuid,
    pub bucket: String,
    pub object_key: String,
    pub byte_size: i64,
    pub rendition_bytes: i64,
    pub owner_organization_id: Option<Uuid>,
}

/// La file « à effacer du stockage », servie par `ix_assets_purgeable` : elle
/// reste minuscule quel que soit le volume historisé.
pub async fn a_purger(pool: &PgPool, limite: i64) -> Result<Vec<ObjetAPurger>> {
    let lignes = sqlx::query!(
        r#"SELECT a.id, a.bucket, a.object_key, a.byte_size, a.owner_organization_id,
                  media.rendition_bytes(a.id) AS "rendition_bytes!"
             FROM media.assets a
            WHERE a.deleted_at IS NOT NULL
              AND a.purged_at IS NULL
              AND a.purge_after IS NOT NULL
              AND a.purge_after <= now()
            ORDER BY a.purge_after
            LIMIT $1"#,
        limite
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| ObjetAPurger {
            id: l.id,
            bucket: l.bucket,
            object_key: l.object_key,
            byte_size: l.byte_size,
            rendition_bytes: l.rendition_bytes,
            owner_organization_id: l.owner_organization_id,
        })
        .collect())
}

/// L'objet a réellement quitté le stockage. Conditionné sur `purged_at IS NULL` :
/// la file est « au moins une fois », et un second passage ne doit pas réécrire
/// l'instant d'une purge déjà datée.
pub async fn marquer_purge(conn: &mut PgConnection, asset_id: Uuid) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE media.assets SET purged_at = now()
          WHERE id = $1 AND purged_at IS NULL",
        asset_id
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}
