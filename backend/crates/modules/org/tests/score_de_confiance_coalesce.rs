//! **Cent approbations d'adhésion produisent un seul recalcul** (SC-014), et un
//! domaine vérifié se voit au premier rechargement.
//!
//! Le score n'est pas un invariant : rien n'est faux s'il a dix secondes de
//! retard. C'est pourquoi il vit dans un travail différé, coalescé **par
//! organisation** — et non dans un trigger qui recalculerait un agrégat de
//! quatre tables à chaque adhésion approuvée.

mod commun;

use commun::{ifdd, perimetres, personne, Bac};
use kernel::jobs::{ClaimedJob, JobHandler};
use org::domain::admin::{DomainVerification, OrganizationVerification};
use org::domain::ids::{OrganizationDomainId, OrganizationId, PersonId};
use org::jobs::trust_score::RecomputeTrustScore;
use org::service::admin_write;
use serde_json::json;
use uuid::Uuid;

async fn travaux_de_recalcul(bac: &Bac) -> Vec<String> {
    sqlx::query_scalar!(
        r#"SELECT idempotency_key AS "cle!" FROM platform.jobs
            WHERE task = 'org.trust_score.recompute' AND idempotency_key IS NOT NULL
            ORDER BY created_at"#
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de la file")
}

#[tokio::test]
async fn cent_ecritures_sur_la_meme_fiche_produisent_un_seul_recalcul() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let organisation = ifdd(&bac).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");
    let ctx = bac.ctx().with_actor(p.globale);

    // Cinquante allers-retours du sceau : cent écritures sur la même fiche.
    for tour in 0..100 {
        admin_write::set_verification(
            &bac.state,
            &ctx,
            &perimetre,
            PersonId(p.globale),
            OrganizationId(organisation),
            OrganizationVerification {
                organization_id: Some(organisation),
                verified: tour % 2 == 0,
            },
        )
        .await
        .expect("écriture");
    }

    let recalculs = travaux_de_recalcul(&bac).await;
    assert_eq!(
        recalculs.len(),
        1,
        "la clé d'unicité porte l'organisation : cent écritures, un recalcul"
    );
    assert!(recalculs[0].ends_with(&organisation.to_string()));
}

/// Deux organisations, **deux recalculs** : la coalescence porte sur la fiche,
/// pas sur le temps.
#[tokio::test]
async fn deux_organisations_produisent_deux_recalculs() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");
    let ctx = bac.ctx().with_actor(p.globale);

    for organisation in [osed.complete, osed.jumelle] {
        admin_write::set_verification(
            &bac.state,
            &ctx,
            &perimetre,
            PersonId(p.globale),
            OrganizationId(organisation),
            OrganizationVerification {
                organization_id: Some(organisation),
                verified: true,
            },
        )
        .await
        .expect("écriture");
    }

    assert_eq!(travaux_de_recalcul(&bac).await.len(), 2);
}

/// **Le recalcul n'écrit que si la valeur change**, et ce n'est pas une
/// optimisation : sans la condition, chaque passage poserait une ligne d'audit
/// et remonterait la date de dernière modification de la fiche — donc son rang
/// dans le tri « dernière activité ». L'historique se remplirait de lignes que
/// personne n'a écrites.
#[tokio::test]
async fn le_recalcul_necrit_que_si_la_valeur_change() {
    let bac = Bac::monter().await;
    let osed = commun::seed::paire_osed(&bac).await;

    let handler = RecomputeTrustScore::new(bac.db());
    let travail = |id: Uuid| ClaimedJob {
        id: Uuid::now_v7(),
        queue: "default".to_owned(),
        task: "org.trust_score.recompute".to_owned(),
        payload: json!({ "organization_id": id }),
        attempts: 0,
        max_attempts: 5,
    };

    handler
        .run(&travail(osed.complete))
        .await
        .expect("premier recalcul");

    let apres_le_premier = sqlx::query_scalar!(
        "SELECT updated_at FROM org.organizations WHERE id = $1",
        osed.complete
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture");

    let avant = lignes_daudit(&bac, osed.complete).await;

    // Deuxième passage : rien n'a changé entre-temps.
    handler
        .run(&travail(osed.complete))
        .await
        .expect("second recalcul");

    let apres_le_second = sqlx::query_scalar!(
        "SELECT updated_at FROM org.organizations WHERE id = $1",
        osed.complete
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture");

    assert_eq!(
        apres_le_premier, apres_le_second,
        "la date de dernière modification ne bouge pas : rien n'a été écrit"
    );
    assert_eq!(
        lignes_daudit(&bac, osed.complete).await,
        avant,
        "aucune ligne d'audit de plus"
    );
}

async fn lignes_daudit(bac: &Bac, organisation: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.audit_log
            WHERE entity_schema = 'org' AND entity_table = 'organizations'
              AND entity_id = $1"#,
        organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage")
}

/// **Un domaine vérifié se voit au premier rechargement** : la fiche relit les
/// quatre colonnes vives sur la table, sans attendre la projection.
#[tokio::test]
async fn un_domaine_verifie_se_voit_au_premier_rechargement() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let libre = sqlx::query_scalar!(
        "INSERT INTO org.organization_domains (organization_id, domain)
         VALUES ($1, 'sahel-observatoire.org') RETURNING id",
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
            auto_join: false,
        },
    )
    .await
    .expect("vérification");

    // La fiche rendue **immédiatement** porte le domaine vérifié : pas besoin
    // d'attendre le travail différé pour voir son propre geste.
    match issue {
        org::domain::admin::OrganizationWriteOutcome::Saved { detail } => {
            let domaine = detail
                .domains
                .iter()
                .find(|d| d.domain == "sahel-observatoire.org")
                .expect("le domaine");
            assert!(domaine.verified_at.is_some());
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }
}

/// Une adhésion approuvée met aussi le recalcul en file : le score compte les
/// membres actifs.
#[tokio::test]
async fn une_adhesion_approuvee_met_le_recalcul_en_file() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let awa = personne(&bac, "awa.diallo@ifdd.francophonie.org", "Awa", "Diallo").await;

    org::service::join::join(
        &bac.state,
        &bac.ctx().with_actor(awa),
        PersonId(awa),
        OrganizationId(organisation),
        org::domain::membership::JoinOrganization {
            job_title: Some("Chargée de projet".to_owned()),
            ..Default::default()
        },
    )
    .await
    .expect("rattachement automatique");

    let recalculs = travaux_de_recalcul(&bac).await;
    assert_eq!(recalculs.len(), 1);
}
