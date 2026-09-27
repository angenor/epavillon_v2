//! La grille de critères est facultative, et un critère n'a plus de code à
//! saisir : la base le reçoit dérivé de son libellé.

mod commun;

use commun::{auteur, formulaire_appel, Bac};
use event::domain::ids::{CallId, EventId};
use event::repo::public;
use event::service::call as service_appel;
use serde_json::json;

#[tokio::test]
async fn une_grille_eteinte_accepte_une_charge_vide_et_garde_les_criteres() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let mut ouverture = formulaire_appel(editions.cop31, "cop31");
    ouverture.criteria = vec![
        commun::critere("relevance", 2.0),
        commun::critere("impact", 1.5),
    ];
    let cree = service_appel::creer(&bac.state, &bac.ctx(), acteur, cop31, ouverture)
        .await
        .expect("création");
    assert!(cree.ok, "{:?}", cree.errors);
    let appel = cree.call.expect("l'appel créé");

    let mut extinction = formulaire_appel(editions.cop31, "cop31");
    extinction.uses_scoring_grid = false;
    extinction.required_reviews = None;
    extinction.criteria.clear();

    let modifie = service_appel::modifier(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        CallId::from(appel.id),
        extinction,
    )
    .await
    .expect("modification");
    assert!(modifie.ok, "{:?}", modifie.errors);

    let relu = modifie.call.expect("l'appel relu");
    assert!(!relu.uses_scoring_grid);
    assert_eq!(relu.required_reviews, None);
    assert_eq!(
        relu.allowed_formats,
        vec!["in_person".to_owned(), "online".to_owned()]
    );
    assert_eq!(
        commun::grille_en_base(&bac, appel.id).await.len(),
        2,
        "les critères saisis restent en base"
    );

    let servi = public::appel(bac.pool(), cop31)
        .await
        .expect("lecture publique")
        .expect("l'appel");
    assert!(!servi.uses_scoring_grid);
    assert!(
        servi.criteria.is_empty(),
        "le public ne voit pas une grille éteinte"
    );
}

#[tokio::test]
async fn un_critere_sans_code_recoit_celui_de_son_libelle() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    let mut nouveau = commun::critere("", 1.0);
    nouveau.label = json!({ "fr": "Qualité des intervenants" });
    p.criteria = vec![commun::critere("relevance", 2.0), nouveau];

    let cree = service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        EventId::from(editions.cop31),
        p,
    )
    .await
    .expect("création");
    assert!(cree.ok, "{:?}", cree.errors);

    let codes: Vec<String> = commun::grille_en_base(&bac, cree.call.expect("appel").id)
        .await
        .into_iter()
        .map(|(code, _, _)| code)
        .collect();
    assert!(
        codes.contains(&"qualite_des_intervenants".to_owned()),
        "{codes:?}"
    );
}
