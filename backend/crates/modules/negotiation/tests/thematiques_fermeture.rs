//! **On ferme, on ne supprime pas** — retirer une thématique pose `left_at`, la
//! ligne reste, et l'index unique partiel n'empêche pas de la reprendre plus
//! tard : une seconde ligne vivante, l'ancienne toujours fermée.

mod commun;

use commun::{codes_de, lignes_de_suivi, personne, suivre, Bac};

#[tokio::test]
async fn retirer_ferme_la_ligne_et_reprendre_en_ouvre_une_autre() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    suivre(&bac, awa, &["adaptation", "gender"], None).await.expect("deux suivis");

    let rendu = suivre(&bac, awa, &["gender"], None).await.expect("retrait");
    assert_eq!(codes_de(&rendu), ["gender"]);
    assert_eq!(
        lignes_de_suivi(&bac, awa).await,
        [("adaptation".to_owned(), true), ("gender".to_owned(), false)],
        "la ligne d'adaptation est fermée, pas supprimée"
    );

    let rendu = suivre(&bac, awa, &["adaptation", "gender"], None)
        .await
        .expect("reprendre ce qu'on avait quitté");
    assert_eq!(codes_de(&rendu), ["adaptation", "gender"]);

    let lignes = lignes_de_suivi(&bac, awa).await;
    assert_eq!(lignes.len(), 3, "trois lignes : l'historique reste lisible");
    assert_eq!(
        lignes.iter().filter(|(c, fermee)| c == "adaptation" && !fermee).count(),
        1,
        "une seule ligne vivante par thématique"
    );
    assert_eq!(
        lignes.iter().filter(|(c, fermee)| c == "adaptation" && *fermee).count(),
        1
    );
}
