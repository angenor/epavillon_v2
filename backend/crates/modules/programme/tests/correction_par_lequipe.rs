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

/// L'équipe qui corrige le titre d'un dossier retenu corrige aussi l'affiche :
/// le titre de la séance suit, son créneau arbitré et son adresse non (FR-091).
#[tokio::test]
async fn lequipe_reporte_le_titre_sur_la_seance_et_rien_dautre() {
    use programme::domain::transitions::ProposalStatus;
    use programme::service::transition;

    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = depose(&bac, &terrain).await;
    for vers in [ProposalStatus::UnderReview, ProposalStatus::Accepted] {
        transition::tenter(&bac.state, &bac.ctx(), ProposalId(dossier), vers, None)
            .await
            .unwrap_or_else(|e| panic!("transition vers {vers:?} : {e}"));
    }
    let seance = sqlx::query!(
        r#"UPDATE programme.sessions
              SET starts_at = timestamp '2027-11-14 09:00' AT TIME ZONE 'America/Belem',
                  ends_at   = timestamp '2027-11-14 10:30' AT TIME ZONE 'America/Belem'
            WHERE proposal_id = $1
        RETURNING id, starts_at, ends_at, slug::text AS "slug!", format::text AS "format!""#,
        dossier
    )
    .fetch_one(bac.pool())
    .await
    .expect("arbitrage de la séance née de l'acceptation");

    let mut brouillon = complet(&terrain, "Atelier adaptation côtière");
    brouillon.preferred_start_at = Some("2027-11-20T16:00".to_owned());
    brouillon.format = Some("online".to_owned());
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;
    draft_write::corriger_par_lequipe(
        &bac.state,
        &bac.ctx(),
        &perimetre,
        droits.decideur,
        ProposalId(dossier),
        commun::charge(&terrain, brouillon),
    )
    .await
    .expect("correction par l'équipe");

    let apres = sqlx::query!(
        r#"SELECT title ->> 'fr' AS "title_fr!", starts_at, ends_at,
                  slug::text AS "slug!", format::text AS "format!"
             FROM programme.sessions WHERE id = $1"#,
        seance.id
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture de la séance");

    assert_eq!(apres.title_fr, "Atelier adaptation côtière");
    assert_eq!(apres.starts_at, seance.starts_at);
    assert_eq!(apres.ends_at, seance.ends_at);
    assert_eq!(apres.slug, seance.slug);
    assert_eq!(apres.format, seance.format);
}
