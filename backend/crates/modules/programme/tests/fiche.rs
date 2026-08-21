//! **Le dossier lu, et ce que chaque voie d'accès en voit.**
//!
//! Deux voies mènent à un dossier — adhésion active à l'organisation
//! porteuse, ou lecture générale dans le périmètre —, et elles ne rendent
//! **pas** la même chose. FR-077 interdit qu'une note atteigne le déposant, et
//! le contrat du front décrit pourtant le dossier comme la ligne de table,
//! agrégats d'évaluation compris (écart n° 104).
//!
//! Le masquage est **à la source** : ce qui n'est pas envoyé ne peut pas
//! fuiter, et un filtrage à l'affichage devrait être refait dans chaque écran,
//! chaque courriel et chaque export — le premier oubli étant la fuite.

mod commun;

use commun::Bac;
use kernel::error::ErrorCode;
use programme::domain::ids::ProposalId;
use programme::service::detail;
use uuid::Uuid;

/// Poser une note consolidée sur un dossier, comme la consolidation le ferait.
async fn noter(bac: &Bac, dossier: Uuid) {
    sqlx::query!(
        "UPDATE programme.proposals
            SET average_score = 16.50, weighted_score = 82.00,
                review_count = 3, is_knocked_out = true
          WHERE id = $1",
        dossier
    )
    .execute(bac.pool())
    .await
    .expect("pose des agrégats");
}

#[tokio::test]
async fn la_voie_de_lorganisation_ne_voit_ni_note_ni_elimination() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;
    noter(&bac, dossier).await;

    let vue_deposante = detail::dossier(&bac.state, droits.deposante, ProposalId(dossier))
        .await
        .expect("la déposante accède à son dossier");

    assert_eq!(vue_deposante.average_score, None);
    assert_eq!(vue_deposante.weighted_score, None);
    assert!(!vue_deposante.is_knocked_out);
    // Le nombre de revues déposées n'est ni une note ni un rang : l'espace
    // organisation affiche l'avancement de l'instruction.
    assert_eq!(vue_deposante.review_count, 3);

    let vue_comite = detail::dossier(&bac.state, droits.decideur, ProposalId(dossier))
        .await
        .expect("le décideur accède au dossier par le périmètre");

    assert_eq!(vue_comite.average_score, Some(16.5));
    assert_eq!(vue_comite.weighted_score, Some(82.0));
    assert!(vue_comite.is_knocked_out);
}

/// **Les deux voies sont distinctes, et le refus est unique.**
#[tokio::test]
async fn hors_des_deux_voies_le_refus_est_celui_dun_inexistant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;
    let quidam = commun::personne(&bac, "quidam@example.org", "Quid", "Am").await;

    let etranger = detail::dossier(&bac.state, quidam, ProposalId(dossier))
        .await
        .expect_err("ni membre, ni administrateur : le dossier n'existe pas pour lui");
    let inexistant = detail::dossier(&bac.state, quidam, ProposalId(Uuid::now_v7()))
        .await
        .expect_err("un dossier inexistant se refuse");

    assert_eq!(etranger.code, inexistant.code);
    assert_eq!(etranger.message, inexistant.message);
    assert_eq!(etranger.code, ErrorCode::NotFound);
}

/// L'espace organisation liste **tous** ses dossiers, brouillons compris, sans
/// aucun périmètre : une organisation n'administre rien.
#[tokio::test]
async fn lorganisation_liste_ses_dossiers_sans_perimetre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    commun::dossier(&bac, &terrain, "Premier dossier", "premier-dossier").await;
    let second = commun::dossier(&bac, &terrain, "Second dossier", "second-dossier").await;

    let fiches = detail::de_lorganisation(&bac.state, terrain.deposante, terrain.organisation)
        .await
        .expect("la déposante liste les dossiers de son organisation");
    assert_eq!(fiches.len(), 2);
    assert!(fiches.iter().all(|f| f.average_score.is_none()));

    // Une personne étrangère à l'organisation et sans droit de lecture reçoit
    // le refus d'une ressource inexistante — pas une liste vide.
    let quidam = commun::personne(&bac, "quidam@example.org", "Quid", "Am").await;
    let refus = detail::de_lorganisation(&bac.state, quidam, terrain.organisation)
        .await
        .expect_err("une organisation étrangère se refuse");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let _ = second;
}
