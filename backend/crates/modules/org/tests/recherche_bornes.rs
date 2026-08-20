//! Les bornes de la recherche : le terme trop court, la limite forgée, et la
//! fiche absorbée.

mod commun;

use commun::{ifdd, Bac};
use org::domain::search::MAX_LIMIT;
use org::service::search::{self, SearchQuery};

fn requete(terme: &str) -> SearchQuery {
    SearchQuery {
        name: terme.to_owned(),
        ..Default::default()
    }
}

/// **Une liste vide, pas une erreur.** Le front ne demande jamais un terme d'un
/// signe — son anti-rebond ne part qu'à deux —, et le garde existe pour qu'un
/// appel forgé ne balaie pas la table (FR-013).
#[tokio::test]
async fn un_seul_caractere_rend_une_liste_vide_et_non_une_erreur() {
    let bac = Bac::monter().await;

    for terme in ["i", " ", "", "  a  ".trim_end()] {
        let resultats = search::similar_for_person(bac.pool(), requete(terme))
            .await
            .unwrap_or_else(|e| panic!("« {terme} » ne doit pas produire d'erreur : {e}"));
        assert!(resultats.is_empty(), "« {terme} » a rendu des résultats");
    }

    // Deux caractères passent, et trouvent.
    let resultats = search::similar_for_person(bac.pool(), requete("in"))
        .await
        .expect("recherche à deux caractères");
    assert!(!resultats.is_empty());
}

/// Une limite forgée retombe au maximum. Sans ce bornage, `limit=100000`
/// deviendrait un export du référentiel par une route ouverte à toute session.
#[tokio::test]
async fn une_limite_forgee_est_ramenee_au_maximum() {
    let bac = Bac::monter().await;
    commun::seed::referentiel_de_mesure(&bac).await;

    let resultats = search::similar_for_person(
        bac.pool(),
        SearchQuery {
            name: "réseau".to_owned(),
            limit: Some(100_000),
            ..Default::default()
        },
    )
    .await
    .expect("recherche à limite forgée");

    assert_eq!(resultats.len(), MAX_LIMIT as usize);
}

/// **Une fiche absorbée ne remonte jamais, et son ancien nom mène à la
/// vivante.** C'est la promesse de la fusion, et elle tient parce que
/// `merge_organizations()` déplace les dénominations avant de clore la fiche.
#[tokio::test]
async fn une_fiche_absorbee_ne_remonte_pas_et_son_nom_mene_a_la_vivante() {
    let bac = Bac::monter().await;
    let osed = commun::seed::paire_osed(&bac).await;
    let survivante = osed.complete;
    let absorbee = osed.jumelle;

    // Avant la fusion, les deux fiches se trouvent par leur nom.
    let avant = search::similar_for_person(bac.pool(), requete("OSED Sahel"))
        .await
        .expect("recherche avant fusion");
    assert!(avant
        .iter()
        .any(|r| r.organization_id.as_uuid() == absorbee));

    let db = bac.db();
    let mut tx = db.write(&bac.ctx()).await.expect("transaction");
    sqlx::query_scalar!(
        "SELECT org.merge_organizations($1, $2, 'fusion de contrôle')",
        absorbee,
        survivante
    )
    .fetch_one(&mut *tx)
    .await
    .expect("fusion");
    tx.commit().await.expect("validation");

    let apres = search::similar_for_person(bac.pool(), requete("OSED Sahel"))
        .await
        .expect("recherche après fusion");

    assert!(
        !apres
            .iter()
            .any(|r| r.organization_id.as_uuid() == absorbee),
        "la fiche absorbée ne doit plus remonter"
    );
    assert!(
        apres
            .iter()
            .any(|r| r.organization_id.as_uuid() == survivante),
        "l'ancien nom doit mener à la fiche vivante"
    );

    // La lecture par identifiant, elle, rend toujours la fiche absorbée : les
    // adresses déjà diffusées continuent de mener quelque part.
    let fiche = org::repo::organizations::by_id(bac.pool(), absorbee.into())
        .await
        .expect("lecture")
        .expect("la fiche absorbée survit");
    assert_eq!(
        fiche.merged_into_id.map(|id| id.as_uuid()),
        Some(survivante)
    );
}

/// La lecture ouverte ne rend que des fiches vivantes, et l'IFDD en est.
#[tokio::test]
async fn la_liste_ouverte_ecarte_les_fiches_absorbees() {
    let bac = Bac::monter().await;
    let osed = commun::seed::paire_osed(&bac).await;
    let attendue = ifdd(&bac).await;

    let db = bac.db();
    let mut tx = db.write(&bac.ctx()).await.expect("transaction");
    sqlx::query_scalar!(
        "SELECT org.merge_organizations($1, $2, NULL)",
        osed.jumelle,
        osed.complete
    )
    .fetch_one(&mut *tx)
    .await
    .expect("fusion");
    tx.commit().await.expect("validation");

    let fiches = org::repo::organizations::list(bac.pool(), 200, 0)
        .await
        .expect("liste");

    let ids: Vec<_> = fiches.iter().map(|f| f.id.as_uuid()).collect();
    assert!(ids.contains(&attendue));
    assert!(
        !ids.contains(&osed.jumelle),
        "la fiche absorbée est écartée"
    );
}
