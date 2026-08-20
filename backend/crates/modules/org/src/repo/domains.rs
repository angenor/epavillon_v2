//! Les domaines de messagerie : « ce que révèle mon adresse », et les écritures
//! du back-office.
//!
//! **Les messageries grand public sont lues en base**, jamais recopiées en
//! Rust : `org.public_email_domains` en porte vingt, et une liste en dur dans le
//! code se périmerait le jour où l'IFDD en ajoute une. Deux ONG ne sont pas la
//! même parce que leurs référents utilisent Gmail.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::domain::ids::{OrganizationDomainId, OrganizationId};
use crate::domain::membership::{EmailDomainMatch, OrganizationDomainRecord};
use crate::repo::organizations;

/// Ce que le domaine d'une adresse révèle.
///
/// **Le domaine vient de la session, jamais du client** : c'est l'appelant qui
/// le passe, et le service le lit sur la personne connectée (FR-017). Sans cela,
/// n'importe qui interrogerait l'adresse de n'importe qui.
///
/// Deux fiches peuvent déclarer le même domaine — c'est exactement le signal de
/// doublon du modèle. **La vérifiée l'emporte**, et à défaut celle qui ouvre le
/// rattachement automatique : entre deux fiches, celle que l'IFDD a reconnue est
/// la bonne réponse.
pub async fn what_email_reveals(pool: &PgPool, email: &str) -> Result<Option<EmailDomainMatch>> {
    let ligne = sqlx::query!(
        r#"WITH entree AS (
               SELECT NULLIF(platform.extract_domain($1), '') AS domaine
           ),
           utilisable AS (
               SELECT e.domaine
                 FROM entree e
                WHERE e.domaine IS NOT NULL
                  AND NOT EXISTS (
                      SELECT 1 FROM org.public_email_domains p WHERE p.domain = e.domaine
                  )
           )
           SELECT d.id                AS "domain_id!",
                  d.organization_id   AS "organization_id!",
                  d.domain            AS "domain!",
                  d.verified_at,
                  d.verification_method,
                  d.auto_join         AS "auto_join!",
                  d.created_at        AS "created_at!",
                  (SELECT count(*) FROM org.memberships m
                    WHERE m.organization_id = d.organization_id AND m.status = 'active')
                                      AS "member_count!"
             FROM org.organization_domains d
             JOIN utilisable u ON u.domaine = d.domain
             JOIN org.organizations o ON o.id = d.organization_id
            WHERE o.status IN ('candidate', 'active')
            ORDER BY (d.verified_at IS NOT NULL) DESC, d.auto_join DESC, d.created_at
            LIMIT 1"#,
        email
    )
    .fetch_optional(pool)
    .await?;

    let Some(l) = ligne else {
        return Ok(None);
    };

    let Some(organisation) = organizations::by_id(pool, OrganizationId(l.organization_id)).await?
    else {
        return Ok(None);
    };

    // La vérification `ck_domain_autojoin_requires_verification` garantit déjà
    // que le rattachement automatique implique la vérification. On teste quand
    // même les deux : une donnée importée peut précéder la contrainte.
    let can_auto_join = l.verified_at.is_some() && l.auto_join;

    Ok(Some(EmailDomainMatch {
        domain: l.domain.clone(),
        organization: Box::new(organisation),
        domain_record: OrganizationDomainRecord {
            id: l.domain_id,
            organization_id: OrganizationId(l.organization_id),
            domain: l.domain,
            verified_at: l.verified_at,
            verification_method: l.verification_method,
            auto_join: l.auto_join,
            created_at: l.created_at,
        },
        can_auto_join,
        member_count: l.member_count,
    }))
}

/// Le rattachement automatique s'applique-t-il à cette personne pour cette
/// organisation ?
///
/// La question est posée **dans la transaction du rattachement** : un domaine
/// vérifié entre l'ouverture de l'écran et le clic changerait sinon l'issue sans
/// que rien ne le relise. Et l'adresse doit être **vérifiée** — sans quoi
/// déclarer une adresse suffirait à entrer.
pub async fn auto_join_applies(
    conn: &mut PgConnection,
    organization_id: OrganizationId,
    person_id: Uuid,
) -> Result<bool> {
    let applique = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1
                 FROM identity.people p
                 JOIN org.organization_domains d
                   ON d.domain = platform.extract_domain(p.primary_email::text)
                WHERE p.id = $2
                  AND p.email_verified_at IS NOT NULL
                  AND d.organization_id = $1
                  AND d.verified_at IS NOT NULL
                  AND d.auto_join
                  AND NOT EXISTS (
                      SELECT 1 FROM org.public_email_domains g WHERE g.domain = d.domain
                  )
           ) AS "applique!""#,
        organization_id.as_uuid(),
        person_id
    )
    .fetch_one(conn)
    .await?;

    Ok(applique)
}

/// Vérifie manuellement un domaine, ou lève sa vérification, et règle le
/// rattachement automatique.
///
/// **Seule la méthode `manual` est livrée.** Le modèle porte les trois ; la
/// vérification par enregistrement DNS et par courriel appartient à un autre
/// jalon, et le contrat du front l'annonce déjà.
///
/// Les deux refus de la base sont **traduits par le service**, jamais
/// réimplémentés ici : l'unicité du domaine vérifié
/// (`ux_organization_domains_verified`) et l'exigence de vérification
/// (`ck_domain_autojoin_requires_verification`).
pub async fn set_verification(
    conn: &mut PgConnection,
    organization_id: OrganizationId,
    domain_id: OrganizationDomainId,
    verified: bool,
    auto_join: bool,
) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE org.organization_domains
            SET verified_at = CASE WHEN $3 THEN COALESCE(verified_at, now()) ELSE NULL END,
                verification_method = CASE WHEN $3 THEN 'manual' ELSE NULL END,
                auto_join = $4
          WHERE id = $2 AND organization_id = $1",
        organization_id.as_uuid(),
        domain_id.as_uuid(),
        verified,
        auto_join
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}

/// La fiche qui détient déjà ce domaine vérifié.
///
/// Sans ce nom, le refus est incompréhensible : « ce domaine est déjà pris »
/// n'apprend rien à qui ne sait pas par qui.
pub async fn holder_of_verified<'e>(
    executor: impl PgExecutor<'e>,
    domain_id: OrganizationDomainId,
) -> Result<Option<(OrganizationId, String)>> {
    let ligne = sqlx::query!(
        r#"SELECT o.id, o.legal_name
             FROM org.organization_domains d
             JOIN org.organization_domains autre ON autre.domain = d.domain
                                                AND autre.verified_at IS NOT NULL
             JOIN org.organizations o ON o.id = autre.organization_id
            WHERE d.id = $1 AND autre.id <> d.id
            LIMIT 1"#,
        domain_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| (OrganizationId(l.id), l.legal_name)))
}
