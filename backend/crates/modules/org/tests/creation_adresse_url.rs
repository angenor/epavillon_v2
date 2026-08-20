//! L'adresse d'URL d'une fiche : le repli, et la collision.
//!
//! `platform.slugify` rend **NULL** quand la normalisation efface tout le nom —
//! un nom entièrement composé de ponctuation ou d'idéogrammes. La colonne est
//! `NOT NULL` et le domaine `platform.slug` refuse la chaîne vide : sans repli,
//! la création échouerait en erreur interne sur un nom que la base accepte par
//! ailleurs.

mod commun;

use commun::{pays, personne, Bac};
use org::domain::ids::PersonId;
use org::domain::organization::{CreateOrganization, CreateOrganizationOutcome};
use org::service::create;
use uuid::Uuid;

async fn creer(bac: &Bac, qui: Uuid, nom: &str, pays: Option<Uuid>) -> CreateOrganizationOutcome {
    create::create(
        &bac.state,
        &bac.ctx().with_actor(qui),
        PersonId(qui),
        CreateOrganization {
            legal_name: nom.to_owned(),
            acronym: None,
            organization_type_code: "ngo_association".to_owned(),
            country_id: pays,
            city: None,
            website: None,
            description: None,
            job_title: None,
            acknowledged_match_ids: Vec::new(),
        },
    )
    .await
    .expect("création")
}

fn adresse(issue: &CreateOrganizationOutcome) -> String {
    match issue {
        CreateOrganizationOutcome::Created { organization, .. } => organization.slug.clone(),
        autre => panic!("issue inattendue : {autre:?}"),
    }
}

/// Un nom que la normalisation efface produit **tout de même** une adresse
/// valide.
#[tokio::test]
async fn un_nom_que_la_normalisation_efface_recoit_une_adresse_de_repli() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "a.sowfall@roac-afrique.org", "Awa", "Sow Fall").await;

    // Deux caractères au moins — `organizations_legal_name_check` l'exige — mais
    // que `platform.normalize_label` réduit à rien.
    let issue = creer(&bac, awa, "《》", None).await;
    let slug = adresse(&issue);

    assert!(
        slug.starts_with("org-"),
        "le repli doit prendre le relais : {slug}"
    );

    // Et l'adresse tient le domaine `platform.slug` : la base l'a acceptée.
    let existe = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.organizations WHERE slug::text = $1"#,
        slug
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(existe, 1);
}

/// **Deux noms voisins ne se heurtent pas.** « Réseau climat » et
/// « Réseau, climat ! » produisent la même adresse normalisée : la seconde est
/// suffixée, et les deux fiches existent.
#[tokio::test]
async fn deux_noms_voisins_ne_se_heurtent_pas_sur_ladresse() {
    let bac = Bac::monter().await;
    let senegal = pays(&bac, "SEN").await;
    let burkina = pays(&bac, "BFA").await;
    let awa = personne(&bac, "a.sowfall@roac-afrique.org", "Awa", "Sow Fall").await;

    // Les deux pays diffèrent : c'est l'unicité de l'ADRESSE qu'on éprouve, pas
    // celle du nom.
    let premiere = creer(&bac, awa, "Réseau climat", Some(senegal)).await;
    let seconde = creer(&bac, awa, "Réseau, climat !", Some(burkina)).await;

    let une = adresse(&premiere);
    let autre = adresse(&seconde);

    assert_eq!(une, "reseau-climat");
    assert_ne!(une, autre, "la seconde adresse est suffixée");
    assert!(
        autre.starts_with("reseau-climat-"),
        "le suffixe garde l'adresse lisible : {autre}"
    );
}
