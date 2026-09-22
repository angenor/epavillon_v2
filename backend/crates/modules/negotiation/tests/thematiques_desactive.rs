//! **Un terme désactivé** reste à qui le suivait et ne se choisit plus : on ne
//! retire pas à quelqu'un ce qu'il suivait, on ne laisse pas en choisir un qui
//! n'est plus proposé.

mod commun;

use commun::{codes_de, personne, suivre, terme_thematique, Bac};
use kernel::error::ErrorCode;

async fn desactiver(bac: &Bac, code: &str) {
    let id = terme_thematique(bac, code).await;
    sqlx::query("UPDATE reference.taxonomy_terms SET is_active = false WHERE id = $1")
        .bind(id)
        .execute(bac.pool())
        .await
        .expect("désactivation du terme");
}

#[tokio::test]
async fn deja_suivi_il_reste_accepte() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    suivre(&bac, awa, &["technology", "gender"], None)
        .await
        .expect("suivi avant le retrait du vocabulaire");
    desactiver(&bac, "technology").await;

    let rendu = suivre(&bac, awa, &["technology", "gender", "finance"], None)
        .await
        .expect("le terme suivi reste acceptable");
    assert_eq!(codes_de(&rendu), ["finance", "gender", "technology"]);
}

#[tokio::test]
async fn nouveau_il_est_refuse_en_nommant_le_code() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;
    desactiver(&bac, "technology").await;

    let refus = suivre(&bac, awa, &["gender", "technology"], None)
        .await
        .expect_err("un terme retiré ne se choisit plus");
    assert_eq!(refus.code, ErrorCode::NegotiationThemeUnknown);
    assert!(
        refus.message.contains("« technology »"),
        "{}",
        refus.message
    );
    assert!(
        refus.message.contains("plus proposée"),
        "le message distingue « retirée » d'« inconnue » : {}",
        refus.message
    );
}
