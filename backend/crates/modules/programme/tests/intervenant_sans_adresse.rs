//! **Un intervenant sans adresse électronique** (D16).
//!
//! Il n'est pas une personne : `identity.people` exige une adresse, et on n'en
//! fabrique jamais. Sa ligne porte elle-même civilité, prénom et nom, et il
//! doit traverser tout le parcours : brouillon, relecture, dépôt, séance,
//! programme public.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::draft::{DraftSpeaker, ProposalDraft};
use programme::domain::ids::{EventId, ProposalId};
use programme::domain::transitions::ProposalStatus;
use programme::repo::session_parts;
use programme::service::submit::ResultatDeDepot;
use programme::service::{draft_read, draft_write, public_schedule, submit, transition};

fn sans_adresse(prenom: &str, nom: &str) -> DraftSpeaker {
    let mut intervenant = commun::intervenant("", prenom, nom);
    intervenant.civility = Some("mme".to_owned());
    intervenant.job_title = "Cheffe de délégation".to_owned();
    intervenant.organization_name = "Ministère de l'Environnement".to_owned();
    intervenant
}

fn avec_sans_adresse(terrain: &Terrain, titre: &str) -> ProposalDraft {
    let mut brouillon = commun::brouillon(terrain, titre);
    brouillon.speakers = vec![
        commun::intervenant("awa.sow@example.org", "Awa", "Sow"),
        sans_adresse("Fatou", "Diop"),
    ];
    brouillon
}

async fn personnes_nommees(bac: &Bac, prenom: &str, nom: &str) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.people
            WHERE first_name = $1 AND last_name = $2"#,
        prenom,
        nom
    )
    .fetch_one(bac.pool())
    .await
    .expect("décompte des personnes")
}

#[tokio::test]
async fn un_brouillon_garde_et_rend_un_intervenant_sans_adresse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(
            &terrain,
            avec_sans_adresse(&terrain, "Atelier sans adresse"),
        ),
    )
    .await
    .expect("un intervenant sans adresse s'enregistre");

    // Un brouillon s'écrit toutes les deux secondes : la ligne sans personne,
    // sans clé stable, ne doit pas se multiplier.
    let mut encore = commun::charge(
        &terrain,
        avec_sans_adresse(&terrain, "Atelier sans adresse"),
    );
    encore.proposal_id = Some(cree.proposal_id);
    draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, encore)
        .await
        .expect("second enregistrement");

    let lignes = sqlx::query!(
        "SELECT person_id, civility, first_name, last_name, job_title_snapshot
           FROM programme.proposal_speakers
          WHERE proposal_id = $1 AND person_id IS NULL",
        cree.proposal_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des intervenants");
    assert_eq!(
        lignes.len(),
        1,
        "une seule ligne après deux enregistrements"
    );
    assert_eq!(lignes[0].first_name.as_deref(), Some("Fatou"));
    assert_eq!(lignes[0].last_name.as_deref(), Some("Diop"));
    assert_eq!(lignes[0].civility.as_deref(), Some("mme"));
    assert_eq!(
        lignes[0].job_title_snapshot.as_deref(),
        Some("Cheffe de délégation")
    );
    assert_eq!(
        personnes_nommees(&bac, "Fatou", "Diop").await,
        0,
        "aucune personne n'est créée sans adresse"
    );

    let rouvert = draft_read::rouvrir(&bac.state, terrain.deposante, ProposalId(cree.proposal_id))
        .await
        .expect("la recomposition");
    assert_eq!(rouvert.draft.speakers.len(), 2);
    let relu = &rouvert.draft.speakers[1];
    assert_eq!(relu.person_id, None);
    assert_eq!(relu.email, "");
    assert_eq!(relu.civility.as_deref(), Some("mme"));
    assert_eq!(relu.first_name, "Fatou");
    assert_eq!(relu.last_name, "Diop");
    assert_eq!(relu.organization_name, "Ministère de l'Environnement");
    assert!(!relu.has_account);
}

#[tokio::test]
async fn un_dossier_avec_un_intervenant_sans_adresse_complet_se_depose() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let charge = commun::charge(&terrain, avec_sans_adresse(&terrain, "Dépôt sans adresse"));
    let cree = draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge.clone())
        .await
        .expect("enregistrement");

    let issue = submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(cree.proposal_id),
        charge,
    )
    .await
    .expect("dépôt");

    assert!(
        matches!(issue, ResultatDeDepot::Submitted { .. }),
        "la civilité de la ligne satisfait la recevabilité"
    );
}

#[tokio::test]
async fn un_intervenant_sans_adresse_et_sans_nom_est_refuse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut brouillon = commun::brouillon(&terrain, "Intervenant anonyme");
    brouillon.speakers = vec![sans_adresse("Fatou", "  ")];

    let refus = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, brouillon),
    )
    .await
    .expect_err("sans adresse, le nom est la seule identité");

    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("speakers"));
}

#[tokio::test]
async fn le_programme_public_nomme_un_intervenant_sans_adresse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier =
        seances::dossier_pret(&bac, &terrain, "Atelier", "atelier", Souhaits::default()).await;
    sqlx::query!(
        r#"INSERT INTO programme.proposal_speakers
               (proposal_id, role, civility, first_name, last_name,
                job_title_snapshot, organization_snapshot, sort_order)
           VALUES ($1, 'panelist', 'dr', 'Fatou', 'Diop', 'Chercheuse', 'IRD', 5)"#,
        dossier.id
    )
    .execute(bac.pool())
    .await
    .expect("intervenant sans adresse");

    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    let id = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0)
        .id;
    assert_eq!(seances::intervenants_de_la_seance(&bac, id).await.len(), 3);

    // Deux NULL ne se heurtent pas dans l'unicité : une seconde recopie ne doit
    // rien ajouter pour autant.
    let mut conn = bac.pool().acquire().await.unwrap();
    let reposes = session_parts::recopier_les_intervenants(&mut conn, dossier.id, id)
        .await
        .unwrap();
    assert_eq!(reposes, 0);

    sqlx::query!(
        "UPDATE programme.sessions SET published_at = now(), status = 'scheduled' WHERE id = $1",
        id
    )
    .execute(bac.pool())
    .await
    .expect("publication posée à la main");
    let adresse = seances::seance(&bac, id).await.slug;

    let detail = public_schedule::seance(bac.pool(), EventId(terrain.edition), &adresse)
        .await
        .expect("la séance publiée est lisible");

    assert_eq!(detail.speakers.len(), 3);
    let fatou = detail
        .speakers
        .iter()
        .find(|s| s["display_name"] == "Fatou Diop")
        .expect("l'intervenant sans adresse est nommé");
    assert!(fatou["avatar"].is_null());
    assert_eq!(fatou["job_title_snapshot"], "Chercheuse");
    assert!(fatou.get("person_id").is_none());
}
