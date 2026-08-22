//! Les quotas de stockage : capacité opposable, et les trois chiffres du refus.
//!
//! # Vérifier avant d'écrire ne réimplémente rien
//!
//! `media.has_storage_capacity()` est la fonction que le modèle décrit lui-même
//! comme « contrôle opposable au téléversement » : l'appeler est son emploi
//! prévu, pas une garde parallèle. Le refus **final** reste celui de
//! `tg_enforce_quota`, et les deux sortent sous le même code stable (R14).
//!
//! # Pourquoi les trois chiffres voyagent
//!
//! « L'espace de cette organisation est atteint » ne dit pas quoi faire. Plafond,
//! consommation et reste permettent à l'écran d'afficher « il vous reste 40 Mio
//! sur 5 Gio » — et à la personne de savoir si elle doit supprimer un fichier ou
//! demander un relèvement.
//!
//! # Ce que ce fichier n'écrit jamais
//!
//! `used_bytes` et `used_files` appartiennent aux déclencheurs et à la
//! réconciliation. Les toucher ferait dériver le compteur au premier écart.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::asset::QuotaSnapshot;

/// L'organisation a-t-elle la place pour ces octets de plus ?
///
/// Une organisation nulle — un objet qui n'appartient qu'à une personne — passe
/// toujours : le quota est porté par l'organisation, et seule elle a un plafond.
pub async fn a_la_place(pool: &PgPool, organization_id: Option<Uuid>, octets: i64) -> Result<bool> {
    let place = sqlx::query_scalar!(
        r#"SELECT media.has_storage_capacity($1, $2) AS "place!""#,
        organization_id,
        octets
    )
    .fetch_one(pool)
    .await?;

    Ok(place)
}

/// Les trois chiffres du refus, avec le repli sur la ligne par défaut.
///
/// **La ligne d'une organisation n'existe pas tant qu'elle n'a rien déposé** :
/// `media.apply_quota_delta()` la crée à la volée, en héritant des plafonds par
/// défaut. Lire seulement sa ligne rendrait donc « aucun quota » là où le
/// plafond par défaut s'applique bel et bien — et le message de refus n'aurait
/// aucun chiffre à montrer.
pub async fn etat(pool: &PgPool, organization_id: Option<Uuid>) -> Result<Option<QuotaSnapshot>> {
    let ligne = sqlx::query!(
        r#"SELECT COALESCE(q.max_bytes, d.max_bytes) AS "max_bytes!",
                  COALESCE(q.used_bytes, 0)          AS "used_bytes!",
                  COALESCE(q.max_files, d.max_files) AS "max_files!",
                  COALESCE(q.used_files, 0)          AS "used_files!"
             FROM media.storage_quotas d
             LEFT JOIN media.storage_quotas q ON q.organization_id = $1
            WHERE d.organization_id IS NULL"#,
        organization_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| QuotaSnapshot {
        max_bytes: l.max_bytes,
        used_bytes: l.used_bytes,
        remaining_bytes: (l.max_bytes - l.used_bytes).max(0),
        max_files: l.max_files,
        used_files: l.used_files,
    }))
}

// -----------------------------------------------------------------------------
// Le tableau du back-office et la réconciliation
// -----------------------------------------------------------------------------

/// Une ligne de quota, **sans le nom de l'organisation** : celui-ci vit dans
/// `org`, et [`crate::repo::cross`] est le seul fichier du module qui le lise.
#[derive(Debug, Clone)]
pub struct LigneDeQuota {
    pub organization_id: Uuid,
    pub max_bytes: i64,
    pub used_bytes: i64,
    pub max_files: i32,
    pub used_files: i32,
    pub note: Option<String>,
}

/// Les quotas des organisations, **triés par proximité du plafond** : ce qui
/// demande une décision est en haut.
pub async fn tableau(pool: &PgPool) -> Result<Vec<LigneDeQuota>> {
    let lignes = sqlx::query!(
        r#"SELECT q.organization_id AS "organization_id!",
                  q.max_bytes, q.used_bytes, q.max_files, q.used_files, q.note
             FROM media.storage_quotas q
            WHERE q.organization_id IS NOT NULL
            ORDER BY q.used_bytes::float8 / q.max_bytes::float8 DESC, q.used_bytes DESC"#
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| LigneDeQuota {
            organization_id: l.organization_id,
            max_bytes: l.max_bytes,
            used_bytes: l.used_bytes,
            max_files: l.max_files,
            used_files: l.used_files,
            note: l.note,
        })
        .collect())
}

/// Relève — ou abaisse — le plafond d'une organisation.
///
/// **La ligne peut ne pas exister** : `media.apply_quota_delta()` ne la crée
/// qu'au premier dépôt. Un plafond relevé d'avance doit pouvoir être posé, et
/// les compteurs partent alors de zéro, comme la consommation.
pub async fn relever(
    conn: &mut PgConnection,
    organization_id: Uuid,
    max_bytes: i64,
    max_files: i32,
    note: Option<&str>,
) -> Result<LigneDeQuota> {
    let ligne = sqlx::query!(
        r#"INSERT INTO media.storage_quotas AS q (organization_id, max_bytes, max_files, note)
                VALUES ($1, $2, $3, $4)
           ON CONFLICT (organization_id) DO UPDATE
                  SET max_bytes = EXCLUDED.max_bytes,
                      max_files = EXCLUDED.max_files,
                      note      = EXCLUDED.note
        RETURNING q.organization_id AS "organization_id!",
                  q.max_bytes, q.used_bytes, q.max_files, q.used_files, q.note"#,
        organization_id,
        max_bytes,
        max_files,
        note
    )
    .fetch_one(conn)
    .await?;

    Ok(LigneDeQuota {
        organization_id: ligne.organization_id,
        max_bytes: ligne.max_bytes,
        used_bytes: ligne.used_bytes,
        max_files: ligne.max_files,
        used_files: ligne.used_files,
        note: ligne.note,
    })
}

/// Réaligne les compteurs incrémentaux sur la consommation réelle —
/// `media.reconcile_storage_quotas()`. Rend le nombre de lignes corrigées.
pub async fn reconcilier(conn: &mut PgConnection) -> Result<i32> {
    let corrigees =
        sqlx::query_scalar!(r#"SELECT media.reconcile_storage_quotas() AS "corrigees!""#)
            .fetch_one(conn)
            .await?;

    Ok(corrigees)
}
