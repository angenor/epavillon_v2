//! Le super-administrateur ouvre l'espace d'un dossier et y agit comme
//! l'organisation, à sa demande (30/10). Un administrateur ordinaire, non.

mod commun;

use commun::Bac;
use programme::domain::ids::ProposalId;
use programme::service::{draft_write, workspace};

#[tokio::test]
async fn le_super_administrateur_ouvre_et_modifie_le_dossier_sans_en_devenir_le_contact() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let brouillon = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, commun::brouillon(&terrain, "Atelier adaptation")),
    )
    .await
    .expect("brouillon de la déposante");
    let dossier = ProposalId(brouillon.proposal_id);

    let tout = commun::personne(&bac, "super@ifdd.francophonie.org", "Sam", "Super").await;
    commun::attribuer(&bac, tout, "super_admin", "global", None).await;

    let fiche = workspace::dossier(&bac.state, tout, dossier)
        .await
        .expect("pas de panne")
        .expect("le dossier s'ouvre");
    assert!(fiche.acting_for_organization, "l'écran le dit");

    let deposante = workspace::dossier(&bac.state, terrain.deposante, dossier)
        .await
        .expect("pas de panne")
        .expect("son dossier");
    assert!(!deposante.acting_for_organization);

    let mut charge = commun::charge(
        &terrain,
        commun::brouillon(&terrain, "Atelier adaptation côtière"),
    );
    charge.proposal_id = Some(brouillon.proposal_id);
    draft_write::enregistrer(&bac.state, &bac.ctx(), tout, charge)
        .await
        .expect("modifié pour l'organisation");
    let relu = commun::ligne(&bac, brouillon.proposal_id).await;
    assert_eq!(relu.title_fr, "Atelier adaptation côtière");
    assert_eq!(relu.contact_person_id, Some(terrain.deposante));
}

#[tokio::test]
async fn un_administrateur_ordinaire_reste_dehors() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = ProposalId(
        commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await,
    );

    let admin = commun::personne(&bac, "admin@ifdd.francophonie.org", "Ada", "Admin").await;
    commun::attribuer(&bac, admin, "admin", "global", None).await;

    let fiche = workspace::dossier(&bac.state, admin, dossier)
        .await
        .expect("pas de panne");
    assert!(fiche.is_none(), "même réponse qu'un dossier inexistant");
}
