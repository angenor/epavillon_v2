//! **Sur la même requête, les deux lectures rendent des résultats différents.**
//!
//! C'est l'écart n° 23, et c'est le cœur du module (SC-003, SC-004). La lecture
//! destinée à une personne répond à « ce que j'ai tapé, est-ce que ça existe
//! déjà ? » ; celle destinée à la revue des doublons répond à « qu'est-ce qui
//! pourrait être la même entité ? ». Le domaine de l'appelant n'a pas le même
//! poids dans les deux, et c'est délibéré.

mod commun;

use commun::seed::{self, DOMAINE_OSED};
use commun::Bac;
use org::service::search::{self, SearchQuery};

/// Un terme qui n'a **aucun mot commun** avec les fiches OSED : seul le domaine
/// peut les faire entrer.
const SANS_RAPPORT: &str = "Coopérative maraîchère de Bobo";

fn requete_avec_domaine(terme: &str) -> SearchQuery {
    SearchQuery {
        name: terme.to_owned(),
        email: Some(format!("b.ouedraogo@{DOMAINE_OSED}")),
        ..Default::default()
    }
}

#[tokio::test]
async fn la_lecture_dutilisateur_ecarte_ce_que_la_revue_retient() {
    let bac = Bac::monter().await;
    let osed = seed::paire_osed(&bac).await;

    let pour_la_personne =
        search::similar_for_person(bac.pool(), requete_avec_domaine(SANS_RAPPORT))
            .await
            .expect("lecture destinée à une personne");

    let pour_la_revue = search::similar_for_review(bac.pool(), requete_avec_domaine(SANS_RAPPORT))
        .await
        .expect("lecture destinée à la revue");

    assert!(
        pour_la_personne.is_empty(),
        "chercher « {SANS_RAPPORT} » ne doit pas ramener l'organisation du domaine \
         de la personne : un bandeau la lui propose déjà nommément. Rendu : {:?}",
        pour_la_personne
            .iter()
            .map(|r| &r.legal_name)
            .collect::<Vec<_>>()
    );

    let ids: Vec<_> = pour_la_revue
        .iter()
        .map(|r| r.organization_id.as_uuid())
        .collect();
    assert!(
        ids.contains(&osed.complete) && ids.contains(&osed.jumelle),
        "les deux fiches déclarant {DOMAINE_OSED} sont la même maison : la revue \
         doit les voir. Rendu : {ids:?}"
    );

    // Et elles sont entrées par le domaine, pas par le nom.
    for r in &pour_la_revue {
        assert!(
            r.match_reasons.iter().any(|m| m == "shared_domain"),
            "« {} » n'est pas entrée par le domaine",
            r.legal_name
        );
        assert!(
            !r.matched_by_name(),
            "« {} » n'aurait pas dû ressembler au terme",
            r.legal_name
        );
    }
}

/// Le domaine **continue de hisser** une fiche qui correspond aussi par le nom :
/// il ne fait plus entrer une fiche sans rapport, il n'a pas cessé de compter
/// (FR-007). Retirer le bonus aurait été l'autre façon de régler l'écart n° 23,
/// et elle aurait coûté le signal le plus fiable du modèle.
#[tokio::test]
async fn le_domaine_hisse_encore_la_fiche_qui_correspond_aussi_par_le_nom() {
    let bac = Bac::monter().await;
    seed::paire_osed(&bac).await;

    let avec =
        search::similar_for_person(bac.pool(), requete_avec_domaine("Observatoire du Sahel"))
            .await
            .expect("recherche avec le domaine");

    let sans = search::similar_for_person(
        bac.pool(),
        SearchQuery {
            name: "Observatoire du Sahel".to_owned(),
            ..Default::default()
        },
    )
    .await
    .expect("recherche sans le domaine");

    let score_avec = avec.first().expect("un résultat").score;
    let score_sans = sans.first().expect("un résultat").score;

    assert!(
        score_avec > score_sans,
        "le domaine doit hisser la fiche : {score_avec} n'est pas supérieur à {score_sans}"
    );
}

/// La lecture de revue rend **au plus** la limite demandée, la filtrée aussi :
/// la sur-lecture ne doit pas déborder du contrat.
#[tokio::test]
async fn la_sur_lecture_ne_deborde_pas_de_la_limite_demandee() {
    let bac = Bac::monter().await;
    seed::paire_osed(&bac).await;

    let resultats = search::similar_for_person(
        bac.pool(),
        SearchQuery {
            name: "observatoire".to_owned(),
            limit: Some(1),
            ..Default::default()
        },
    )
    .await
    .expect("recherche bornée à un résultat");

    assert_eq!(resultats.len(), 1);
}
