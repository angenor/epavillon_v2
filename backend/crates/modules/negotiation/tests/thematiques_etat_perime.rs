//! **Le `412` sur état changé** — lire l'empreinte, modifier l'état par une
//! autre voie, rejouer avec l'ancienne : refusé, et **la base est inchangée**.
//!
//! C'est le téléphone de midi qui ne doit pas effacer le choix de la tablette
//! de onze heures.

mod commun;

use commun::{codes_de, lignes_de_suivi, mes_thematiques, personne, suivre, Bac};
use kernel::error::ErrorCode;

#[tokio::test]
async fn une_empreinte_perimee_est_refusee_sans_rien_ecrire() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    // Le téléphone lit l'état à 10:00 et retient son empreinte.
    suivre(&bac, awa, &["adaptation"], None)
        .await
        .expect("état initial");
    let empreinte_de_10h = mes_thematiques(&bac, awa).await.empreinte();

    // La tablette, à 11:00, choisit autre chose.
    suivre(&bac, awa, &["finance"], None)
        .await
        .expect("choix de la tablette");
    let lignes_de_11h = lignes_de_suivi(&bac, awa).await;

    // Le téléphone, de retour à 12:00, envoie son intention de 10:00.
    let refus = suivre(
        &bac,
        awa,
        &["adaptation", "gender"],
        Some(&empreinte_de_10h),
    )
    .await
    .expect_err("l'intention en retard doit être refusée");
    assert_eq!(refus.code, ErrorCode::NegotiationThemesStale);
    assert_eq!(refus.code.status().as_u16(), 412);

    assert_eq!(
        lignes_de_suivi(&bac, awa).await,
        lignes_de_11h,
        "aucune écriture : ni ouverture, ni fermeture"
    );
    assert_eq!(codes_de(&mes_thematiques(&bac, awa).await), ["finance"]);
}

#[tokio::test]
async fn lempreinte_courante_est_acceptee() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    suivre(&bac, awa, &["adaptation"], None)
        .await
        .expect("état initial");
    let empreinte = mes_thematiques(&bac, awa).await.empreinte();

    let rendu = suivre(&bac, awa, &["adaptation", "gender"], Some(&empreinte))
        .await
        .expect("l'empreinte est celle de l'état courant");
    assert_eq!(codes_de(&rendu), ["adaptation", "gender"]);
}
