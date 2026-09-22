//! **Sans `If-Match`** — accepté, quel que soit l'état : l'écran en ligne vient
//! de lire et n'a rien à opposer. Le refus de fraîcheur ne vaut que pour une
//! intention qui présente une empreinte.

mod commun;

use commun::{codes_de, personne, suivre, Bac};

#[tokio::test]
async fn sans_empreinte_le_remplacement_passe_toujours() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    suivre(&bac, awa, &["adaptation"], None).await.expect("premier");
    suivre(&bac, awa, &["finance"], None).await.expect("changé ailleurs");

    let rendu = suivre(&bac, awa, &["gender"], None)
        .await
        .expect("sans If-Match, rien n'est opposé");
    assert_eq!(codes_de(&rendu), ["gender"]);
}
