//! **Le changement de mode prend effet à la tentative suivante** — SC-002,
//! FR-021.
//!
//! C'est le critère de sortie le plus fragile de cette étape : il suffirait
//! d'un cache de quelques secondes pour qu'un administrateur bascule sur
//! « approbation » et voie des gens entrer quand même. La valeur se relit à
//! chaque saisie, et ce fichier le mesure **sans redémarrer quoi que ce soit** :
//! le même état de module sert avant et après la bascule.

mod commun;

use commun::{a_lacces, attribuer, basculer_le_mode, creer_un_code, decor, personne, saisir, Bac};
use negotiation::domain::redeem::RedeemIssue;

#[tokio::test]
async fn la_bascule_se_voit_a_lentree_suivante_sans_redemarrage() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", Some(d.space_id), None).await;

    // Mode « code » : la première personne entre aussitôt.
    let premiere = saisir(&bac, d.person_id, &code.code, None).await;
    assert_eq!(premiere.issue, RedeemIssue::Accepted);

    basculer_le_mode(&bac, d.admin_id, "approval")
        .await
        .expect("bascule du mode");

    // **Rien n'a redémarré**, et la suivante ne passe plus : son code juste
    // ouvre une demande.
    let suivante = personne(&bac, "mariam.traore@example.org").await;
    let seconde = saisir(&bac, suivante, &code.code, None).await;

    assert_eq!(seconde.issue, RedeemIssue::PendingApproval);
    assert!(seconde.request.is_some());
    assert!(!a_lacces(&bac, suivante, Some(d.space_id)).await);
}

#[tokio::test]
async fn un_mode_inconnu_est_refuse_et_nomme_son_champ() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let refus = basculer_le_mode(&bac, d.admin_id, "libre")
        .await
        .expect_err("un mode hors des trois ne s'écrit pas");

    assert_eq!(
        refus.code,
        kernel::error::ErrorCode::NegotiationAdmissionModeInvalid
    );
    assert_eq!(
        refus.field.as_deref(),
        Some("mode"),
        "l'écran pose le refus sous le sélecteur, pas en bandeau de panne"
    );
}

#[tokio::test]
async fn les_trois_modes_sont_offerts_avec_ce_quils_produisent() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let reglage = negotiation::service::admission::lire(&bac.state)
        .await
        .expect("lecture du mode");

    assert_eq!(reglage.mode, "code", "le semis pose « code »");
    assert_eq!(reglage.options.len(), 3);

    let approbation = reglage
        .options
        .iter()
        .find(|o| o.mode == "approval")
        .expect("le mode « approbation » est offert");

    assert!(
        !approbation.offers_code,
        "en approbation seule, l'application ne propose pas la saisie d'un code (FR-022)"
    );
    assert!(approbation.needs_approval);

    let apres = basculer_le_mode(&bac, d.admin_id, "code_and_approval")
        .await
        .expect("bascule");
    assert_eq!(apres.mode, "code_and_approval");
}
