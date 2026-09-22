//! **Le rejeu identique** — deux `PUT` du même corps donnent le même état et
//! **aucune ligne d'audit de plus**. C'est ce qui permet à une intention prise
//! hors connexion de repartir sans crainte.

mod commun;

use commun::{codes_de, lignes_de_suivi, personne, suivre, traces_de_suivi, Bac};

#[tokio::test]
async fn rejouer_le_meme_corps_necrit_rien() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    let premier = suivre(&bac, awa, &["adaptation", "gender"], None)
        .await
        .expect("premier envoi");
    let lignes = lignes_de_suivi(&bac, awa).await;
    let traces = traces_de_suivi(&bac, awa).await;
    assert_eq!(traces.len(), 2, "deux ouvertures, deux traces");

    let second = suivre(&bac, awa, &["gender", "adaptation"], None)
        .await
        .expect("second envoi, même corps dans un autre ordre");

    assert_eq!(codes_de(&second), codes_de(&premier));
    assert_eq!(second.empreinte(), premier.empreinte());
    assert_eq!(
        second.themes[0].followed_at, premier.themes[0].followed_at,
        "un suivi inchangé garde sa date"
    );
    assert_eq!(lignes_de_suivi(&bac, awa).await, lignes, "aucune ligne de plus");
    assert_eq!(
        traces_de_suivi(&bac, awa).await,
        traces,
        "aucune trace d'audit de plus : rien n'a été écrit"
    );
}
