//! **L'équipe corrige un dossier déposé**, sans être membre de son organisation.

mod commun;

use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::draft::ProposalDraft;
use programme::domain::ids::ProposalId;
use programme::service::{draft_write, submit};
use uuid::Uuid;

fn complet(terrain: &Terrain, titre: &str) -> ProposalDraft {
    let mut brouillon = commun::brouillon(terrain, titre);
    brouillon.speakers = vec![commun::intervenant("awa.sow@example.org", "Awa", "Sow")];
    brouillon
}

async fn depose(bac: &Bac, terrain: &Terrain) -> Uuid {
    let ligne = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(terrain, complet(terrain, "Atelier adaptation")),
    )
    .await
    .expect("enregistrement");
    submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(ligne.proposal_id),
        commun::charge(terrain, complet(terrain, "Atelier adaptation")),
    )
    .await
    .expect("dépôt");
    ligne.proposal_id
}

#[tokio::test]
async fn ladministration_corrige_sans_changer_letat_ni_le_contact() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = depose(&bac, &terrain).await;
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;

    let ligne = draft_write::corriger_par_lequipe(
        &bac.state,
        &bac.ctx(),
        &perimetre,
        droits.decideur,
        ProposalId(dossier),
        commun::charge(&terrain, complet(&terrain, "Atelier adaptation côtière")),
    )
    .await
    .expect("correction par l'équipe");

    assert_eq!(ligne.status, "submitted");
    let relu = commun::ligne(&bac, dossier).await;
    assert_eq!(relu.title_fr, "Atelier adaptation côtière");
    assert_eq!(relu.contact_person_id, Some(terrain.deposante));
}

#[tokio::test]
async fn le_comite_sans_la_permission_est_refuse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = depose(&bac, &terrain).await;
    let perimetre = commun::perimetre_de(&bac, droits.noteur).await;

    let refus = draft_write::corriger_par_lequipe(
        &bac.state,
        &bac.ctx(),
        &perimetre,
        droits.noteur,
        ProposalId(dossier),
        commun::charge(&terrain, complet(&terrain, "Autre titre")),
    )
    .await
    .expect_err("un membre du comité ne corrige pas le contenu");
    assert_eq!(refus.code, ErrorCode::Forbidden);
}

#[tokio::test]
async fn un_brouillon_reste_a_son_organisation() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let brouillon = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, complet(&terrain, "En cours")),
    )
    .await
    .expect("brouillon");
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;

    let refus = draft_write::corriger_par_lequipe(
        &bac.state,
        &bac.ctx(),
        &perimetre,
        droits.decideur,
        ProposalId(brouillon.proposal_id),
        commun::charge(&terrain, complet(&terrain, "Corrigé")),
    )
    .await
    .expect_err("un brouillon se refuse");
    assert_eq!(refus.code, ErrorCode::ProposalNotEditable);
}

#[tokio::test]
async fn un_dossier_depose_garde_ses_thematiques() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = depose(&bac, &terrain).await;
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;

    let mut vide = complet(&terrain, "Atelier adaptation");
    vide.theme_codes.clear();
    let refus = draft_write::corriger_par_lequipe(
        &bac.state,
        &bac.ctx(),
        &perimetre,
        droits.decideur,
        ProposalId(dossier),
        commun::charge(&terrain, vide),
    )
    .await
    .expect_err("sans thématique, le dossier ne serait plus complet");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
}

#[tokio::test]
async fn un_intervenant_sans_civilite_ne_se_depose_pas() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let mut brouillon = complet(&terrain, "Atelier incomplet");
    brouillon.speakers[0].civility = None;
    let ligne = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, brouillon.clone()),
    )
    .await
    .expect("un brouillon incomplet s'enregistre");

    let refus = submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(ligne.proposal_id),
        commun::charge(&terrain, brouillon),
    )
    .await
    .expect_err("la base refuse le dépôt");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("speakers"));
    assert_eq!(commun::ligne(&bac, ligne.proposal_id).await.status, "draft");
}

#[tokio::test]
async fn lequipe_ne_laisse_pas_un_intervenant_sans_organisation() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = depose(&bac, &terrain).await;
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;

    let mut brouillon = complet(&terrain, "Atelier adaptation");
    brouillon.speakers[0].organization_name.clear();
    let refus = draft_write::corriger_par_lequipe(
        &bac.state,
        &bac.ctx(),
        &perimetre,
        droits.decideur,
        ProposalId(dossier),
        commun::charge(&terrain, brouillon),
    )
    .await
    .expect_err("un intervenant sans organisation se refuse");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
}
