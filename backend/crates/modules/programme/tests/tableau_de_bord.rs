//! Le tableau de bord du dossier (01/11) : ce que la fiche lit au-delà de
//! l'évaluation, et les deux rattachements (porteuse, déposant).

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::ProposalId;
use programme::domain::transitions::ProposalStatus;
use programme::service::rattachement::{ChangeLeadOrganizationPayload, ChangeSubmitterPayload};
use programme::service::{desk, rattachement, transition};
use uuid::Uuid;

/// Un dossier retenu, sa séance placée dans l'auditorium et rattachée au fil.
async fn dossier_place(bac: &Bac, terrain: &Terrain) -> (Uuid, Uuid, seances::Grille) {
    let grille = seances::grille(bac, terrain.edition).await;
    let dossier = seances::dossier_pret(
        bac,
        terrain,
        "Villes côtières",
        "villes-cotieres",
        Souhaits::default(),
    )
    .await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .expect("l'acceptation aboutit");
    let seance = seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id;
    seances::placer_le(
        bac,
        terrain.edition,
        seance,
        Some(grille.salle),
        "2027-11-14",
        "10:00",
        "11:30",
    )
    .await
    .expect("le placement aboutit");
    seances::rattacher(bac, terrain.edition, seance, vec![grille.fil], None)
        .await
        .expect("le rattachement aboutit");
    (dossier.id, seance, grille)
}

/// Un dossier d'une autre édition, pour la même organisation.
async fn dossier_ailleurs(bac: &Bac, terrain: &Terrain, edition: Uuid) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (event_id, organization_id, submitted_by, title, slug, objectives,
                detailed_presentation, format)
           VALUES ($1, $2, $3, '{"fr":"Dossier du PACO"}'::jsonb,
                   'dossier-du-paco'::platform.slug, '{"fr":"Objectifs."}'::jsonb,
                   '{"fr":"<p>Présentation.</p>"}'::jsonb, 'online')
        RETURNING id"#,
        edition,
        terrain.organisation,
        terrain.deposante
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier d'une autre édition")
}

#[tokio::test]
async fn la_fiche_porte_le_creneau_retenu_les_adresses_et_les_autres_dossiers_du_perimetre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let (dossier, seance, grille) = dossier_place(&bac, &terrain).await;

    seances::ouvrir_les_inscriptions(&bac, seance, None, false).await;
    let inscrite = commun::personne(&bac, "inscrite@example.org", "Ina", "Scrite").await;
    seances::sinscrire(&bac, seance, Some(inscrite), seances::reponses_valides())
        .await
        .expect("l'inscription aboutit");

    let voisin = commun::dossier(&bac, &terrain, "Dossier voisin", "dossier-voisin").await;
    let secondaire = commun::edition_secondaire(&bac).await;
    let lointain = dossier_ailleurs(&bac, &terrain, secondaire).await;

    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;
    let fiche = desk::ouvrir(&bac.state, &bac.ctx(), &perimetre, ProposalId(dossier))
        .await
        .expect("le décideur ouvre la fiche");
    let tableau = &fiche.dashboard;

    assert!(
        fiche.permissions.can_schedule,
        "l'administrateur de l'édition place les séances"
    );
    assert_eq!(tableau.sessions.len(), 1);
    let retenue = &tableau.sessions[0];
    assert_eq!(retenue.id, seance);
    assert_eq!(retenue.room_id, Some(grille.salle));
    assert!(retenue.room_name.is_some());
    assert_eq!(
        retenue.tracks.iter().map(|j| j.id).collect::<Vec<_>>(),
        vec![grille.fil]
    );
    assert_eq!(retenue.registration_count, 1);
    assert_eq!(tableau.counts.registrations, 1);
    assert_eq!(tableau.counts.speakers, 2);

    let deposante = tableau.submitter.as_ref().expect("la déposante est jointe");
    assert_eq!(deposante.person_id, terrain.deposante);
    assert_eq!(deposante.email, "aicha.diallo@example.org");
    assert!(
        fiche.speakers.iter().all(|i| i.email.is_some()),
        "chaque intervenant porte son adresse"
    );

    let autres: Vec<Uuid> = tableau.other_proposals.iter().map(|a| a.id).collect();
    assert_eq!(
        autres,
        vec![voisin],
        "le dossier d'une édition hors périmètre ne sort pas (règle 8)"
    );
    let autre = &tableau.other_proposals[0];
    assert_eq!(
        autre.submitter.as_ref().map(|p| p.email.as_str()),
        Some("aicha.diallo@example.org")
    );
    assert!(!autres.contains(&lointain));
}

#[tokio::test]
async fn changer_de_porteuse_garde_les_coorganisatrices_et_deplace_les_seances() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let (dossier, seance, _) = dossier_place(&bac, &terrain).await;
    let avant = commun::associations(&bac, dossier).await;
    let coorganisatrice = avant
        .iter()
        .find(|(_, role)| role == "co_organizer")
        .map(|(id, _)| *id)
        .expect("le dossier a une co-organisatrice");

    let nouvelle =
        commun::organisation_verifiee(&bac, "Villes durables d'Afrique de l'Ouest", "VDAO").await;
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;
    let fiche = rattachement::changer_de_porteuse(
        &bac.state,
        &bac.ctx().with_actor(droits.decideur),
        &perimetre,
        ProposalId(dossier),
        ChangeLeadOrganizationPayload {
            organization_id: nouvelle,
        },
    )
    .await
    .expect("le rattachement aboutit");
    assert_eq!(fiche.organization_id, nouvelle);

    let apres = commun::associations(&bac, dossier).await;
    assert!(apres.contains(&(nouvelle, "lead".to_owned())));
    assert!(apres.contains(&(coorganisatrice, "co_organizer".to_owned())));
    assert!(!apres.iter().any(|(id, _)| *id == terrain.organisation));

    let porteuse_de_la_seance = sqlx::query_scalar!(
        "SELECT organization_id FROM programme.sessions WHERE id = $1",
        seance
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la séance");
    assert_eq!(porteuse_de_la_seance, Some(nouvelle));

    let refus = rattachement::changer_de_porteuse(
        &bac.state,
        &bac.ctx().with_actor(droits.decideur),
        &perimetre,
        ProposalId(dossier),
        ChangeLeadOrganizationPayload {
            organization_id: Uuid::now_v7(),
        },
    )
    .await
    .expect_err("une organisation inconnue se refuse");
    assert_eq!(refus.code, ErrorCode::ProposalUnknownReference);
}

#[tokio::test]
async fn changer_de_deposant_et_les_refus() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let (dossier, _, _) = dossier_place(&bac, &terrain).await;
    let releve = commun::personne(&bac, "releve@example.org", "Rémi", "Lève").await;

    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;
    let fiche = rattachement::changer_de_deposant(
        &bac.state,
        &bac.ctx().with_actor(droits.decideur),
        &perimetre,
        ProposalId(dossier),
        ChangeSubmitterPayload { person_id: releve },
    )
    .await
    .expect("le changement de déposant aboutit");
    assert_eq!(fiche.submitted_by, releve);

    // Le noteur lit le dossier mais ne le corrige pas.
    let noteur = commun::perimetre_de(&bac, droits.noteur).await;
    let interdit = rattachement::changer_de_deposant(
        &bac.state,
        &bac.ctx().with_actor(droits.noteur),
        &noteur,
        ProposalId(dossier),
        ChangeSubmitterPayload {
            person_id: terrain.deposante,
        },
    )
    .await
    .expect_err("sans programme.proposal.edit, refus");
    assert_eq!(interdit.code, ErrorCode::Forbidden);

    // Hors périmètre : indiscernable d'un dossier inexistant (règle 8).
    let secondaire = commun::edition_secondaire(&bac).await;
    let ailleurs = commun::personne(&bac, "ailleurs@ifdd.francophonie.org", "Al", "Leurs").await;
    commun::attribuer(&bac, ailleurs, "admin", "event", Some(secondaire)).await;
    let etranger = commun::perimetre_de(&bac, ailleurs).await;
    let hors = rattachement::changer_de_porteuse(
        &bac.state,
        &bac.ctx().with_actor(ailleurs),
        &etranger,
        ProposalId(dossier),
        ChangeLeadOrganizationPayload {
            organization_id: terrain.organisation,
        },
    )
    .await
    .expect_err("hors périmètre, refus");
    assert_eq!(hors.code, ErrorCode::NotFound);
    let desk_hors = desk::ouvrir(&bac.state, &bac.ctx(), &etranger, ProposalId(dossier))
        .await
        .expect_err("la fiche hors périmètre se refuse aussi");
    assert_eq!(desk_hors.code, ErrorCode::NotFound);
}
