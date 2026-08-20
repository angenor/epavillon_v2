//! **La recherche répond en moins de 150 ms au 95ᵉ centile sur 5 000 fiches**
//! (SC-002).
//!
//! C'est la seule cible chiffrée du module, et elle se **mesure** avant de se
//! traiter. Le semis imite la distribution réelle des noms — beaucoup de fiches
//! partagent leurs premiers mots —, sans quoi le parcours d'index rendrait une
//! poignée de lignes et la mesure serait excellente et fausse (research.md § R3).
//!
//! **Le point d'incertitude est nommé** : la fonction du modèle pose son terme
//! normalisé dans une expression de table référencée plusieurs fois, que
//! PostgreSQL matérialise par défaut. Le terme n'est donc pas une constante au
//! moment de la planification, et l'usage de l'index GIN dépend de la capacité
//! du planificateur à paramétrer le parcours. C'est vérifiable et non devinable :
//! **l'échec rend le plan d'exécution**, pas seulement le chiffre.

mod commun;

use commun::seed::{self, FORMES_DE_RECHERCHE};
use commun::Bac;
use org::service::search::{self, SearchQuery};
use std::time::{Duration, Instant};

/// Cent recherches : dix formes, dix fois chacune.
const PASSAGES: usize = 10;
const CIBLE: Duration = Duration::from_millis(150);

#[tokio::test]
async fn la_recherche_tient_150ms_au_95e_centile_sur_5000_fiches() {
    let bac = Bac::monter().await;
    seed::referentiel_de_mesure(&bac).await;

    let semees = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM org.organizations"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage");
    assert!(
        semees >= seed::TAILLE_REFERENTIEL as i64,
        "le référentiel de mesure n'est pas semé : {semees} fiches"
    );

    // Un passage à blanc : la première recherche paie le chargement des pages
    // d'index, que la production a déjà payé depuis longtemps.
    let _ = search::similar_for_person(bac.pool(), requete(FORMES_DE_RECHERCHE[0])).await;

    let mut mesures: Vec<(Duration, &str)> =
        Vec::with_capacity(PASSAGES * FORMES_DE_RECHERCHE.len());

    for _ in 0..PASSAGES {
        for forme in FORMES_DE_RECHERCHE {
            let debut = Instant::now();
            search::similar_for_person(bac.pool(), requete(forme))
                .await
                .unwrap_or_else(|e| panic!("recherche « {forme} » : {e}"));
            mesures.push((debut.elapsed(), forme));
        }
    }

    mesures.sort_by_key(|(d, _)| *d);
    let rang = (mesures.len() as f64 * 0.95).ceil() as usize - 1;
    let (centile_95, forme_lente) = mesures[rang];
    let (pire, _) = mesures[mesures.len() - 1];

    if centile_95 > CIBLE {
        let plan = plan_dexecution(&bac, forme_lente).await;
        panic!(
            "SC-002 non tenu : 95ᵉ centile à {} ms sur {} recherches ({} fiches), \
             cible {} ms. Pire cas {} ms.\n\n\
             La forme la plus lente au 95ᵉ centile est « {forme_lente} ». \
             L'ordre des remèdes est fixé par research.md § R2 : réviser l'appel, \
             puis augmenter la statistique sur la dénomination normalisée, et \
             seulement alors proposer une modification du SQL — avec cette mesure \
             comme justification écrite.\n\n\
             Plan d'exécution :\n{plan}",
            centile_95.as_millis(),
            mesures.len(),
            semees,
            CIBLE.as_millis(),
            pire.as_millis(),
        );
    }

    let median = mesures[mesures.len() / 2].0;
    println!(
        "SC-002 tenu — médiane {} ms, 95ᵉ centile {} ms, pire cas {} ms sur {} fiches",
        median.as_millis(),
        centile_95.as_millis(),
        pire.as_millis(),
        semees
    );
}

fn requete(terme: &str) -> SearchQuery {
    SearchQuery {
        name: terme.to_owned(),
        ..Default::default()
    }
}

/// Le plan que le message d'échec porte. Il est demandé **à la même requête que
/// le dépôt exécute**, filtre compris : un plan pris sur une requête voisine
/// enverrait chercher au mauvais endroit.
async fn plan_dexecution(bac: &Bac, terme: &str) -> String {
    let lignes: Vec<String> = sqlx::query_scalar(
        "EXPLAIN (ANALYZE, BUFFERS)
         SELECT * FROM org.find_similar_organizations($1, NULL, NULL, NULL, 15)
          WHERE 'name_similarity' = ANY(match_reasons)",
    )
    .bind(terme)
    .fetch_all(bac.pool())
    .await
    .unwrap_or_else(|e| vec![format!("plan indisponible : {e}")]);

    lignes.join("\n")
}
