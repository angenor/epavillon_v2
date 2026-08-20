//! **Les cinq façons de désigner l'IFDD ramènent la même fiche, une seule
//! fois.**
//!
//! C'est la règle métier n° 1 du projet et le défaut n° 1 de la version 1 :
//! « certains cherchaient par nom complet tandis que d'autres par sigle », et
//! deux fiches naissaient. Le semis pose l'IFDD avec ses cinq dénominations ;
//! ce test vérifie qu'elles mènent toutes au même endroit.

mod commun;

use commun::{ifdd, Bac};
use org::service::search::{self, SearchQuery};

fn requete(terme: &str) -> SearchQuery {
    SearchQuery {
        name: terme.to_owned(),
        ..Default::default()
    }
}

#[tokio::test]
async fn les_cinq_facons_de_designer_lifdd_ramenent_la_meme_fiche() {
    let bac = Bac::monter().await;
    let attendue = ifdd(&bac).await;

    // Sigle, début du nom complet, deux lettres, traduction, ancien nom, et la
    // faute d'orthographe connue. Six entrées pour une seule fiche.
    let formes = [
        ("IFDD", "le sigle"),
        ("Institut de la Francophonie", "le début du nom complet"),
        ("in", "deux lettres"),
        ("Institute of the Francophonie", "la traduction anglaise"),
        ("IEPF", "l'ancien sigle"),
        (
            "Institut de l'énergie et de l'environnement",
            "l'ancien nom",
        ),
    ];

    for (terme, quoi) in formes {
        let resultats = search::similar_for_person(bac.pool(), requete(terme))
            .await
            .unwrap_or_else(|e| panic!("recherche « {terme} » : {e}"));

        let trouvees: Vec<_> = resultats
            .iter()
            .filter(|r| r.organization_id.as_uuid() == attendue)
            .collect();

        assert_eq!(
            trouvees.len(),
            1,
            "{quoi} — « {terme} » devait ramener l'IFDD exactement une fois, \
             et en a ramené {} sur {} résultats",
            trouvees.len(),
            resultats.len()
        );

        // La dénomination qui a déclenché la correspondance revient avec le
        // résultat : c'est ce qui permet à l'écran de dire « trouvée par son
        // sigle » plutôt que d'afficher un nom que la personne n'a pas tapé.
        assert!(
            trouvees[0].matched_name.is_some(),
            "{quoi} — la dénomination qui a déclenché la correspondance manque"
        );
    }
}

/// Une fiche ne remonte **qu'une fois**, même quand plusieurs de ses
/// dénominations correspondent. Sans le regroupement de la fonction du modèle,
/// une fiche portant à la fois un nom légal et un sigle serait proposée deux
/// fois — et l'écran donnerait à croire qu'il y a deux organisations.
#[tokio::test]
async fn une_fiche_ne_remonte_quune_fois_meme_avec_plusieurs_denominations() {
    let bac = Bac::monter().await;
    let attendue = ifdd(&bac).await;

    let resultats = search::similar_for_person(bac.pool(), requete("Institut de la Francophonie"))
        .await
        .expect("recherche");

    let occurrences = resultats
        .iter()
        .filter(|r| r.organization_id.as_uuid() == attendue)
        .count();
    assert_eq!(occurrences, 1);
}

/// Le nombre de membres accompagne le résultat : c'est ce qui fait dire « c'est
/// bien la mienne » sans recharger une table à chaque frappe. Les adhésions en
/// attente ne comptent pas — elles ne prouvent encore rien.
#[tokio::test]
async fn le_resultat_porte_de_quoi_reconnaitre_la_fiche() {
    let bac = Bac::monter().await;
    let attendue = ifdd(&bac).await;

    let resultats = search::similar_for_person(bac.pool(), requete("IFDD"))
        .await
        .expect("recherche");
    let fiche = resultats
        .iter()
        .find(|r| r.organization_id.as_uuid() == attendue)
        .expect("l'IFDD");

    assert!(fiche.verified_at.is_some(), "le sceau de l'IFDD");
    assert_eq!(
        fiche.member_count, 1,
        "le compte d'administration du semis est le seul membre actif"
    );
    assert_eq!(fiche.organization_type_code, "international_organization");
    assert!(fiche.matched_by_name(), "entrée par sa dénomination");
}
