//! **Un domaine vérifié n'appartient qu'à une fiche** — obligation n° 3 du
//! principe X.
//!
//! `ux_organization_domains_verified` est un invariant de la base. Le code ne le
//! redouble pas : il **traduit son refus**, et le refus **nomme la fiche** qui
//! détient le domaine. Sans ce nom, « ce domaine est déjà pris » n'apprend rien
//! à qui ne sait pas par qui.

mod commun;

use commun::{perimetres, Bac};
use kernel::error::ErrorCode;
use org::domain::admin::{DomainVerification, OrganizationWriteOutcome};
use org::domain::ids::{OrganizationDomainId, OrganizationId};
use org::service::admin_write;
use uuid::Uuid;

/// L'identifiant du domaine d'une fiche.
async fn domaine_de(bac: &Bac, organisation: Uuid, domaine: &str) -> Uuid {
    sqlx::query_scalar!(
        "SELECT id FROM org.organization_domains
          WHERE organization_id = $1 AND domain = $2",
        organisation,
        domaine
    )
    .fetch_one(bac.pool())
    .await
    .expect("le domaine")
}

#[tokio::test]
async fn verifier_un_domaine_deja_verifie_ailleurs_nomme_la_fiche_qui_le_detient() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    // La fiche complète détient `osed-sahel.org` vérifié ; la jumelle porte le
    // même domaine, non vérifié. C'est exactement le signal qui les a fait
    // remonter comme doublons.
    let a_verifier = domaine_de(&bac, osed.jumelle, osed.domaine).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    let issue = admin_write::set_domain(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        &perimetre,
        OrganizationId(osed.jumelle),
        OrganizationDomainId(a_verifier),
        DomainVerification {
            organization_id: Some(osed.jumelle),
            domain_id: Some(a_verifier),
            verified: true,
            auto_join: false,
        },
    )
    .await
    .expect("le refus de la base est traduit, jamais rendu tel quel");

    match issue {
        OrganizationWriteOutcome::DomainTaken { conflict_with } => {
            assert_eq!(conflict_with.organization_id.as_uuid(), osed.complete);
            assert_eq!(
                conflict_with.legal_name,
                "Observatoire du Sahel pour l'environnement et le développement",
                "le refus NOMME la fiche : sans ce nom il est incompréhensible"
            );
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }

    // Et rien n'a été écrit : la transaction est rendue.
    let toujours_non_verifie = sqlx::query_scalar!(
        "SELECT verified_at FROM org.organization_domains WHERE id = $1",
        a_verifier
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture");
    assert!(toujours_non_verifie.is_none());
}

/// **Le rattachement automatique exige un domaine vérifié.**
/// `ck_domain_autojoin_requires_verification` le tient, et le code le traduit
/// sur le champ fautif.
#[tokio::test]
async fn le_rattachement_automatique_exige_un_domaine_verifie() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let domaine = domaine_de(&bac, osed.jumelle, osed.domaine).await;
    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    let refus = admin_write::set_domain(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        &perimetre,
        OrganizationId(osed.jumelle),
        OrganizationDomainId(domaine),
        DomainVerification {
            organization_id: Some(osed.jumelle),
            domain_id: Some(domaine),
            verified: false,
            auto_join: true,
        },
    )
    .await
    .expect_err("un rattachement automatique sans vérification est refusé");

    assert_eq!(refus.code, ErrorCode::OrgDomainVerificationRequired);
    assert_eq!(refus.field.as_deref(), Some("auto_join"));
}

/// Le chemin nominal : un domaine libre se vérifie, et le rattachement
/// automatique s'ouvre du même geste.
#[tokio::test]
async fn un_domaine_libre_se_verifie_et_ouvre_le_rattachement() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let libre = sqlx::query_scalar!(
        "INSERT INTO org.organization_domains (organization_id, domain)
         VALUES ($1, 'observatoire-sahel.bf') RETURNING id",
        osed.complete
    )
    .fetch_one(bac.pool())
    .await
    .expect("domaine libre");

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    let issue = admin_write::set_domain(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        &perimetre,
        OrganizationId(osed.complete),
        OrganizationDomainId(libre),
        DomainVerification {
            organization_id: Some(osed.complete),
            domain_id: Some(libre),
            verified: true,
            auto_join: true,
        },
    )
    .await
    .expect("vérification");

    let fiche = match issue {
        OrganizationWriteOutcome::Saved { detail } => detail,
        autre => panic!("issue inattendue : {autre:?}"),
    };

    let domaine = fiche
        .domains
        .iter()
        .find(|d| d.domain == "observatoire-sahel.bf")
        .expect("le domaine vérifié");
    assert!(domaine.verified_at.is_some());
    assert!(domaine.auto_join);
    assert_eq!(domaine.verification_method.as_deref(), Some("manual"));

    // La fiche entière revient, et elle porte le partage du domaine jumeau :
    // c'est le signal de doublon, et il se voit d'abord ici.
    let partage = fiche
        .domains
        .iter()
        .find(|d| d.domain == osed.domaine)
        .expect("le domaine partagé");
    assert_eq!(partage.shared_with.len(), 1);
    assert_eq!(
        partage.shared_with[0].organization_id.as_uuid(),
        osed.jumelle
    );
}
