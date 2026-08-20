//! Les lectures et les écritures de la fusion.
//!
//! **`org.merge_organizations()` fait tout le travail de réaffectation.** Ce
//! fichier ne le refait pas : il lit ce qu'il faut pour trancher, appelle la
//! fonction, applique les arbitrages de champ **après** elle, et relit le
//! décompte réel dans le journal.

use kernel::error::{ApiError, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::ids::OrganizationId;
use crate::domain::merge::{TransferredDomain, TransferredName};

/// Appelle la fonction de fusion du modèle.
///
/// Elle réaffecte, passe la source en `merged`, écrit le journal, marque la
/// paire de la file **et émet l'événement**. Le service n'a donc rien à faire de
/// tout cela — voir le commentaire de `service/merge.rs`.
///
/// Elle lit l'acteur dans le contexte de transaction : elle **doit** donc être
/// appelée par la porte d'écriture du noyau, faute de quoi le journal des
/// fusions n'aurait pas d'auteur.
pub async fn fusionner(
    conn: &mut PgConnection,
    source: OrganizationId,
    cible: OrganizationId,
    motif: &str,
) -> Result<OrganizationId> {
    let rendu = sqlx::query_scalar!(
        "SELECT org.merge_organizations($1, $2, $3)",
        source.as_uuid(),
        cible.as_uuid(),
        motif
    )
    .fetch_one(conn)
    .await?;

    rendu
        .map(OrganizationId)
        .ok_or_else(|| ApiError::internal("org.merge_organizations() n'a rien rendu"))
}

/// Le décompte **réel**, relu dans le journal après l'appel.
///
/// C'est la seule façon d'obtenir `rows_reassigned` : la fonction ne rend que
/// l'identifiant de la fiche survivante.
pub async fn decompte_reel(
    conn: &mut PgConnection,
    source: OrganizationId,
    cible: OrganizationId,
) -> Result<Value> {
    let rendu = sqlx::query_scalar!(
        r#"SELECT rows_reassigned AS "rows!" FROM org.merge_log
            WHERE source_id = $1 AND target_id = $2
            ORDER BY performed_at DESC LIMIT 1"#,
        source.as_uuid(),
        cible.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    Ok(rendu.unwrap_or_else(|| Value::Object(Default::default())))
}

/// Les dénominations que la source apportera, **et si la cible les a déjà**.
///
/// La comparaison porte sur le nom normalisé et le genre : c'est exactement ce
/// que fait la fonction de fusion à son étape 1.
pub async fn denominations_apportees(
    conn: &mut PgConnection,
    source: OrganizationId,
    cible: OrganizationId,
) -> Result<Vec<TransferredName>> {
    let lignes = sqlx::query!(
        r#"SELECT n.name, n.kind::text AS "kind!", n.is_confirmed,
                  EXISTS (
                      SELECT 1 FROM org.organization_names t
                       WHERE t.organization_id = $2
                         AND t.name_normalized = n.name_normalized
                         AND t.kind = n.kind
                  ) AS "already_present!"
             FROM org.organization_names n
            WHERE n.organization_id = $1
            ORDER BY n.kind, n.name"#,
        source.as_uuid(),
        cible.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| TransferredName {
            name: l.name,
            kind: l.kind,
            is_confirmed: l.is_confirmed,
            already_present: l.already_present,
        })
        .collect())
}

/// Les domaines apportés, avec leur état de vérification.
pub async fn domaines_apportes(
    conn: &mut PgConnection,
    source: OrganizationId,
    cible: OrganizationId,
) -> Result<Vec<TransferredDomain>> {
    let lignes = sqlx::query!(
        r#"SELECT d.domain, d.verified_at,
                  EXISTS (
                      SELECT 1 FROM org.organization_domains t
                       WHERE t.organization_id = $2 AND t.domain = d.domain
                  ) AS "already_present!"
             FROM org.organization_domains d
            WHERE d.organization_id = $1
            ORDER BY d.domain"#,
        source.as_uuid(),
        cible.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| TransferredDomain {
            domain: l.domain,
            verified_at: l.verified_at,
            already_present: l.already_present,
        })
        .collect())
}

/// Les dix champs comparables d'une fiche, en JSON — la forme que l'écran
/// compare.
pub async fn champs_comparables(
    conn: &mut PgConnection,
    id: OrganizationId,
) -> Result<Option<Value>> {
    let rendu = sqlx::query_scalar!(
        r#"SELECT jsonb_build_object(
                      'legal_name', o.legal_name,
                      'acronym', o.acronym,
                      'slug', o.slug::text,
                      'organization_type_code', o.organization_type_code,
                      'country_id', o.country_id,
                      'city', o.city,
                      'description', o.description::jsonb,
                      'website', o.website::text,
                      'contact_email', o.contact_email::text,
                      'contact_phone', o.contact_phone
                  ) AS "champs!"
             FROM org.organizations o WHERE o.id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    Ok(rendu)
}

/// Le nom de confirmation, **vérifié par la base**.
///
/// `platform.normalize_label` ignore la casse, les accents et la ponctuation :
/// c'est la même normalisation que partout ailleurs, et la réécrire en Rust
/// ferait diverger la comparaison d'une virgule. **Le sigle est accepté au même
/// titre que le nom légal** : quelqu'un qui tape « OSED » désigne sans ambiguïté
/// la fiche qu'il regarde.
pub async fn nom_de_confirmation_valide(
    conn: &mut PgConnection,
    source: OrganizationId,
    saisi: &str,
) -> Result<bool> {
    let valide = sqlx::query_scalar!(
        r#"SELECT (platform.normalize_label($2) IN (o.legal_name_normalized, o.acronym_normalized))
                  AS "valide!"
             FROM org.organizations o WHERE o.id = $1"#,
        source.as_uuid(),
        saisi
    )
    .fetch_optional(conn)
    .await?;

    Ok(valide.unwrap_or(false))
}

/// La fiche survivante, après arbitrage d'un champ.
///
/// Un seul ordre par champ : la liste des colonnes arbitrables est fermée
/// (`MERGE_FIELDS`), et composer un `UPDATE` dynamique pour dix colonnes
/// connues serait une seconde entorse au principe VI pour aucun gain.
pub async fn arbitrer(
    conn: &mut PgConnection,
    cible: OrganizationId,
    champ: &str,
    valeur: &Value,
) -> Result<bool> {
    let id = cible.as_uuid();
    let texte = valeur.as_str().map(str::to_owned);

    let touchees = match champ {
        "legal_name" => {
            sqlx::query!(
                "UPDATE org.organizations SET legal_name = $2 WHERE id = $1",
                id,
                texte
            )
            .execute(conn)
            .await?
        }
        "acronym" => {
            sqlx::query!(
                "UPDATE org.organizations SET acronym = $2 WHERE id = $1",
                id,
                texte
            )
            .execute(conn)
            .await?
        }
        "organization_type_code" => {
            sqlx::query!(
                "UPDATE org.organizations SET organization_type_code = $2 WHERE id = $1",
                id,
                texte
            )
            .execute(conn)
            .await?
        }
        "country_id" => {
            let pays = texte.as_deref().and_then(|s| Uuid::parse_str(s).ok());
            sqlx::query!(
                "UPDATE org.organizations SET country_id = $2 WHERE id = $1",
                id,
                pays
            )
            .execute(conn)
            .await?
        }
        "city" => {
            sqlx::query!(
                "UPDATE org.organizations SET city = $2 WHERE id = $1",
                id,
                texte
            )
            .execute(conn)
            .await?
        }
        "description" => {
            sqlx::query!(
                "UPDATE org.organizations SET description = $2::jsonb::platform.i18n_text
                  WHERE id = $1",
                id,
                valeur
            )
            .execute(conn)
            .await?
        }
        "website" => {
            sqlx::query!(
                "UPDATE org.organizations SET website = $2::text::platform.url WHERE id = $1",
                id,
                texte
            )
            .execute(conn)
            .await?
        }
        "contact_email" => {
            sqlx::query!(
                "UPDATE org.organizations SET contact_email = $2::text::platform.email
                  WHERE id = $1",
                id,
                texte
            )
            .execute(conn)
            .await?
        }
        "contact_phone" => {
            sqlx::query!(
                "UPDATE org.organizations SET contact_phone = $2 WHERE id = $1",
                id,
                texte
            )
            .execute(conn)
            .await?
        }
        // `slug` n'arrive jamais ici : le service le refuse avant, par un code
        // stable qui nomme le champ (research.md § R6).
        _ => return Ok(false),
    };

    Ok(touchees.rows_affected() == 1)
}
