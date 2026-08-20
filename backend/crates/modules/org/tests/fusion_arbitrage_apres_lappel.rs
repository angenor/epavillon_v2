//! **Une fusion qui arbitre le nom légal aboutit.**
//!
//! Elle échouerait sur une violation d'unicité si l'ordre était inversé :
//! `ux_organizations_name_country` ne porte que sur les fiches **vivantes**, et
//! tant que la fiche absorbée l'est encore, la survivante ne peut pas reprendre
//! son nom. C'est l'écart n° 70, et c'est le champ le plus souvent arbitré.

mod commun;

use commun::{pays, perimetres, Bac};
use org::domain::ids::PersonId;
use org::domain::merge::{MergeOutcome, MergePayload, MergeSide};
use org::service::merge;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Deux fiches **dans le même pays** : c'est la condition pour que l'unicité
/// (nom normalisé, pays) puisse jouer. Sans cela, le test passerait quel que
/// soit l'ordre, et ne prouverait rien.
async fn deux_fiches_du_meme_pays(bac: &Bac) -> (Uuid, Uuid) {
    let senegal = pays(bac, "SEN").await;

    let source = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations (legal_name, slug, organization_type_code, country_id, status)
           VALUES ('Réseau ouest-africain pour le climat', 'roac'::platform.slug,
                   'ngo_association', $1, 'active')
        RETURNING id"#,
        senegal
    )
    .fetch_one(bac.pool())
    .await
    .expect("fiche source");

    let cible = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations (legal_name, slug, organization_type_code, country_id, status)
           VALUES ('ROAC Afrique', 'roac-afrique'::platform.slug,
                   'ngo_association', $1, 'active')
        RETURNING id"#,
        senegal
    )
    .fetch_one(bac.pool())
    .await
    .expect("fiche cible");

    (source, cible)
}

#[tokio::test]
async fn une_fusion_arbitrant_le_nom_legal_aboutit() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let (source, cible) = deux_fiches_du_meme_pays(&bac).await;

    let mut choix = BTreeMap::new();
    choix.insert("legal_name".to_owned(), MergeSide::Source);

    let issue = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: source,
            target_id: cible,
            pair_id: None,
            reason: "le nom complet vaut mieux que le sigle".to_owned(),
            field_choices: choix,
            confirmation_name: "Réseau ouest-africain pour le climat".to_owned(),
        },
    )
    .await
    .expect("la fusion ne doit PAS échouer sur une violation d'unicité");

    let appliques = match issue {
        MergeOutcome::Merged { fields_applied, .. } => fields_applied,
        autre => panic!("issue inattendue : {autre:?}"),
    };

    assert_eq!(appliques, vec!["legal_name".to_owned()]);

    // La survivante porte désormais le nom de la fiche absorbée.
    let survivante = org::repo::organizations::by_id(bac.pool(), cible.into())
        .await
        .expect("lecture")
        .expect("la fiche vivante");
    assert_eq!(
        survivante.legal_name,
        "Réseau ouest-africain pour le climat"
    );

    // **Effet de bord voulu** : le trigger de dénominations a fait entrer
    // l'ancien nom de la survivante dans ses variantes. Une recherche sur l'un
    // ou l'autre trouve donc la bonne fiche.
    let denominations: Vec<String> = sqlx::query_scalar!(
        "SELECT name FROM org.organization_names WHERE organization_id = $1 ORDER BY name",
        cible
    )
    .fetch_all(bac.pool())
    .await
    .expect("dénominations");

    assert!(
        denominations.contains(&"ROAC Afrique".to_owned()),
        "l'ancien nom de la survivante est conservé : {denominations:?}"
    );
    assert!(
        denominations.contains(&"Réseau ouest-africain pour le climat".to_owned()),
        "le nouveau nom aussi : {denominations:?}"
    );
}

/// **Un champ absent du dictionnaire garde la valeur de la cible.** C'est elle
/// qui survit, et l'absence de choix ne doit rien écraser.
#[tokio::test]
async fn un_champ_sans_choix_garde_la_valeur_de_la_cible() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let (source, cible) = deux_fiches_du_meme_pays(&bac).await;

    sqlx::query!(
        "UPDATE org.organizations SET city = 'Dakar' WHERE id = $1",
        source
    )
    .execute(bac.pool())
    .await
    .expect("ville côté source");
    sqlx::query!(
        "UPDATE org.organizations SET city = 'Saint-Louis' WHERE id = $1",
        cible
    )
    .execute(bac.pool())
    .await
    .expect("ville côté cible");

    merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: source,
            target_id: cible,
            pair_id: None,
            reason: "doublon".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "Réseau ouest-africain pour le climat".to_owned(),
        },
    )
    .await
    .expect("fusion");

    let survivante = org::repo::organizations::by_id(bac.pool(), cible.into())
        .await
        .expect("lecture")
        .expect("la fiche");
    assert_eq!(
        survivante.city.as_deref(),
        Some("Saint-Louis"),
        "aucun choix : la cible garde sa valeur"
    );
}
