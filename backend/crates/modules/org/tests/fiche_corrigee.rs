//! **L'équipe corrige la fiche d'une organisation** (31/10) : nom, sigle, type,
//! pays, site. L'ancien nom reste trouvable (règle n° 1), un homonyme du même
//! pays est nommé, un type inconnu est refusé.

mod commun;

use commun::{evenements_emis, perimetres, Bac};
use kernel::auth::AdminScope;
use kernel::error::ErrorCode;
use org::domain::admin::{OrganizationProfile, OrganizationWriteOutcome};
use org::domain::ids::OrganizationId;
use org::service::admin_write;
use uuid::Uuid;

async fn equipe(bac: &Bac) -> (Uuid, AdminScope) {
    let p = perimetres(bac).await;
    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");
    (p.globale, perimetre)
}

async fn fiche(bac: &Bac, id: Uuid) -> (String, Option<String>, String, Option<Uuid>) {
    let l = sqlx::query!(
        "SELECT legal_name, acronym, organization_type_code, country_id FROM org.organizations WHERE id = $1",
        id
    )
    .fetch_one(bac.pool())
    .await
    .expect("la fiche");
    (
        l.legal_name,
        l.acronym,
        l.organization_type_code,
        l.country_id,
    )
}

async fn denominations(bac: &Bac, id: Uuid) -> Vec<(String, String)> {
    sqlx::query!(
        r#"SELECT kind::text AS "kind!", name FROM org.organization_names WHERE organization_id = $1 ORDER BY kind, name"#,
        id
    )
    .fetch_all(bac.pool())
    .await
    .expect("dénominations")
    .into_iter()
    .map(|l| (l.kind, l.name))
    .collect()
}

#[tokio::test]
async fn renommer_garde_l_ancien_nom_et_l_ancien_sigle_cherchables() {
    let bac = Bac::monter().await;
    let (admin, perimetre) = equipe(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;
    let (ancien_nom, ancien_sigle, type_code, pays) = fiche(&bac, osed.complete).await;

    let issue = admin_write::set_profile(
        &bac.state,
        &bac.ctx().with_actor(admin),
        &perimetre,
        OrganizationId(osed.complete),
        OrganizationProfile {
            legal_name: "  Observatoire sahélien de l'environnement durable  ".to_owned(),
            acronym: Some("OSEDD".to_owned()),
            acronym_well_known: None,
            organization_type_code: type_code,
            country_id: pays,
            website: Some("https://osedd.example.org".to_owned()),
        },
    )
    .await
    .expect("correction");
    assert!(
        matches!(issue, OrganizationWriteOutcome::Saved { .. }),
        "{issue:?}"
    );

    let (nom, sigle, _, _) = fiche(&bac, osed.complete).await;
    assert_eq!(nom, "Observatoire sahélien de l'environnement durable");
    assert_eq!(sigle.as_deref(), Some("OSEDD"));

    let noms = denominations(&bac, osed.complete).await;
    assert!(
        noms.contains(&("former".to_owned(), ancien_nom.clone())),
        "{noms:?}"
    );
    if let Some(s) = ancien_sigle {
        assert!(noms.contains(&("former".to_owned(), s)), "{noms:?}");
    }
    assert_eq!(
        noms.iter().filter(|(k, _)| k == "legal").count(),
        1,
        "un seul nom légal : {noms:?}"
    );

    let trouvee = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.find_similar_organizations($1, NULL, NULL, NULL, 10) WHERE organization_id = $2"#,
        ancien_nom,
        osed.complete
    )
    .fetch_one(bac.pool())
    .await
    .expect("recherche");
    assert_eq!(trouvee, 1, "l'ancien nom ramène la fiche");

    assert!(evenements_emis(&bac, osed.complete)
        .await
        .contains(&"org.organization.updated".to_owned()));
}

#[tokio::test]
async fn un_homonyme_du_meme_pays_est_nomme_et_un_type_inconnu_refuse() {
    let bac = Bac::monter().await;
    let (admin, perimetre) = equipe(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;
    let (nom_complete, _, type_code, pays) = fiche(&bac, osed.complete).await;
    let (_, _, _, pays_jumelle) = fiche(&bac, osed.jumelle).await;

    let issue = admin_write::set_profile(
        &bac.state,
        &bac.ctx().with_actor(admin),
        &perimetre,
        OrganizationId(osed.jumelle),
        OrganizationProfile {
            legal_name: nom_complete.clone(),
            acronym: None,
            acronym_well_known: None,
            organization_type_code: type_code.clone(),
            country_id: pays,
            website: None,
        },
    )
    .await
    .expect("le refus de la base est traduit");
    match issue {
        OrganizationWriteOutcome::NameTaken { conflict_with } => {
            assert_eq!(conflict_with.organization_id, OrganizationId(osed.complete));
        }
        autre => panic!("homonyme attendu : {autre:?}"),
    }
    assert_eq!(
        fiche(&bac, osed.jumelle).await.3,
        pays_jumelle,
        "rien n'a changé"
    );

    let refus = admin_write::set_profile(
        &bac.state,
        &bac.ctx().with_actor(admin),
        &perimetre,
        OrganizationId(osed.jumelle),
        OrganizationProfile {
            legal_name: "Nom libre".to_owned(),
            acronym: None,
            acronym_well_known: None,
            organization_type_code: "type_qui_n_existe_pas".to_owned(),
            country_id: None,
            website: None,
        },
    )
    .await
    .expect_err("type inconnu");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
}
