//! **Le chemin nominal** — un `PUT` de deux codes ouvre deux suivis, et le `GET`
//! les rend avec leur empreinte, calculée sur les codes triés.

mod commun;

use commun::{codes_de, mes_thematiques, personne, suivre, Bac};
use negotiation::domain::themes::empreinte_des_codes;

#[tokio::test]
async fn deux_codes_ouvrent_deux_suivis_et_se_relisent() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    let avant = mes_thematiques(&bac, awa).await;
    assert!(
        avant.themes.is_empty(),
        "sans suivi : une liste vide, jamais une erreur"
    );

    let rendu = suivre(&bac, awa, &["gender", "adaptation"], None)
        .await
        .expect("remplacement");
    assert_eq!(
        codes_de(&rendu),
        ["adaptation", "gender"],
        "rendus dans l'ordre du vocabulaire, pas dans celui de la saisie"
    );

    let relu = mes_thematiques(&bac, awa).await;
    assert_eq!(codes_de(&relu), codes_de(&rendu));
    assert_eq!(
        relu.empreinte(),
        empreinte_des_codes(["adaptation", "gender"]),
        "l'empreinte dit l'état : les codes triés, rien d'autre"
    );
    assert_ne!(relu.empreinte(), avant.empreinte());
}

#[tokio::test]
async fn les_doublons_recus_sont_reduits() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    let rendu = suivre(&bac, awa, &["finance", "finance", " finance "], None)
        .await
        .expect("remplacement");
    assert_eq!(codes_de(&rendu), ["finance"]);
}
