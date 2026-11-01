//! Séances communes : réunir deux dossiers retenus dans un créneau, sous un
//! titre commun, puis séparer.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::{EventId, SessionId};
use programme::domain::transitions::ProposalStatus;
use programme::service::joint::{self, JointSessionPayload};
use programme::service::{public_schedule, transition};
use uuid::Uuid;

async fn retenir(bac: &Bac, dossier: Uuid) {
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .expect("l'acceptation aboutit");
}

/// Deux dossiers retenus de deux organisations, le premier placé en salle.
struct Deux {
    terrain: Terrain,
    premier: Uuid,
    second: Uuid,
    salle: Uuid,
}

async fn deux_dossiers(bac: &Bac) -> Deux {
    let terrain = commun::terrain(bac).await;
    let grille = seances::grille(bac, terrain.edition).await;

    let autre_org = commun::organisation_verifiee(bac, "Association des maires", "AIMF").await;
    let autre_personne = commun::personne(bac, "awa.sow@example.org", "Awa", "Sow").await;
    commun::adherer(bac, autre_org, autre_personne, "active").await;
    let autre = Terrain {
        edition: terrain.edition,
        appel: terrain.appel,
        organisation: autre_org,
        deposante: autre_personne,
    };

    let premier = seances::dossier_pret(
        bac,
        &terrain,
        "Villes côtières",
        "villes",
        Souhaits::default(),
    )
    .await;
    let second =
        seances::dossier_pret(bac, &autre, "Mangroves", "mangroves", Souhaits::default()).await;
    retenir(bac, premier.id).await;
    retenir(bac, second.id).await;

    let seance = seances::seances_du_dossier(bac, premier.id)
        .await
        .remove(0)
        .id;
    seances::placer(
        bac,
        terrain.edition,
        seance,
        Some(grille.salle),
        "11:30",
        "13:00",
    )
    .await
    .expect("placement");

    Deux {
        terrain,
        premier: premier.id,
        second: second.id,
        salle: grille.salle,
    }
}

fn charge(d: &Deux) -> JointSessionPayload {
    JointSessionPayload {
        event_id: d.terrain.edition,
        proposal_ids: vec![d.premier, d.second],
        title: serde_json::json!({ "fr": "Littoraux africains" }),
    }
}

async fn la_seance_commune(bac: &Bac, edition: Uuid) -> Uuid {
    sqlx::query_scalar!(
        "SELECT DISTINCT session_id FROM programme.session_proposals sp
           JOIN programme.sessions s ON s.id = sp.session_id
          WHERE s.event_id = $1",
        edition
    )
    .fetch_one(bac.pool())
    .await
    .expect("une séance commune")
}

#[tokio::test]
async fn reunir_cree_une_seance_au_creneau_du_premier_et_retire_les_autres() {
    let bac = Bac::monter().await;
    let d = deux_dossiers(&bac).await;

    let ecran = joint::reunir(
        &bac.state,
        &bac.ctx(),
        EventId(d.terrain.edition),
        charge(&d),
    )
    .await
    .expect("la réunion aboutit");

    let toutes: Vec<_> = ecran.placed.iter().chain(ecran.unplaced.iter()).collect();
    assert_eq!(toutes.len(), 1, "une seule séance reste : la commune");
    let commune = toutes[0];
    assert_eq!(
        commune.proposal_id, None,
        "la séance commune n'a pas de dossier propre"
    );
    assert_eq!(
        commune.room_id,
        Some(d.salle),
        "elle prend la salle du premier"
    );
    assert_eq!(commune.title["fr"], "Littoraux africains");
    let parties = commune.joint_proposals.as_array().expect("tableau");
    assert_eq!(parties.len(), 2);
    assert_eq!(parties[0]["proposal_id"], d.premier.to_string());
    assert_eq!(parties[1]["organization_acronym"], "AIMF");

    // Chaque intervenant et chaque porteur est rangé sous son dossier.
    let rangees = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM programme.session_speakers
                    WHERE session_id = $1 AND proposal_id IS NOT NULL) AS "intervenants!",
                  (SELECT count(*) FROM programme.session_organizations
                    WHERE session_id = $1 AND role = 'lead' AND proposal_id IS NOT NULL) AS "porteurs!""#,
        commune.id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture");
    assert_eq!(rangees.intervenants, 4, "deux intervenants par dossier");
    assert_eq!(rangees.porteurs, 2, "un porteur par dossier");
}

#[tokio::test]
async fn separer_rend_a_chaque_dossier_sa_seance() {
    let bac = Bac::monter().await;
    let d = deux_dossiers(&bac).await;
    joint::reunir(
        &bac.state,
        &bac.ctx(),
        EventId(d.terrain.edition),
        charge(&d),
    )
    .await
    .expect("réunion");
    let commune = la_seance_commune(&bac, d.terrain.edition).await;

    let ecran = joint::separer(
        &bac.state,
        &bac.ctx(),
        EventId(d.terrain.edition),
        SessionId(commune),
    )
    .await
    .expect("la séparation aboutit");

    assert_eq!(
        ecran.placed.len(),
        1,
        "le premier retrouve le créneau commun"
    );
    assert_eq!(ecran.placed[0].proposal_id, Some(d.premier));
    assert_eq!(ecran.placed[0].room_id, Some(d.salle));
    assert_eq!(ecran.unplaced.len(), 1, "le second revient au panneau");
    assert_eq!(ecran.unplaced[0].proposal_id, Some(d.second));
    assert!(ecran
        .placed
        .iter()
        .chain(ecran.unplaced.iter())
        .all(|s| s.joint_proposals.as_array().is_some_and(Vec::is_empty)));
}

#[tokio::test]
async fn reunir_ou_separer_refuse_de_perdre_des_inscrits() {
    let bac = Bac::monter().await;
    seances::activer_le_champ_par_defaut(&bac, "country").await;
    let d = deux_dossiers(&bac).await;

    let seance_du_second = seances::seances_du_dossier(&bac, d.second)
        .await
        .remove(0)
        .id;
    seances::ouvrir_les_inscriptions(&bac, seance_du_second, None, false).await;
    let inscrite = commun::personne(&bac, "fatou.ndiaye@example.org", "Fatou", "Ndiaye").await;
    seances::sinscrire(
        &bac,
        seance_du_second,
        Some(inscrite),
        seances::reponses_valides(),
    )
    .await
    .expect("inscription");

    let refus = joint::reunir(
        &bac.state,
        &bac.ctx(),
        EventId(d.terrain.edition),
        charge(&d),
    )
    .await
    .expect_err("une séance avec inscrits ne se retire pas");
    assert_eq!(refus.code, ErrorCode::JointSessionHasRegistrations);
    assert_eq!(
        seances::seances_du_dossier(&bac, d.second).await.len(),
        1,
        "rien n'a été retiré"
    );

    // Une séance commune avec inscrits ne se sépare pas non plus.
    sqlx::query!(
        "DELETE FROM programme.registrations WHERE session_id = $1",
        seance_du_second
    )
    .execute(bac.pool())
    .await
    .expect("nettoyage");
    joint::reunir(
        &bac.state,
        &bac.ctx(),
        EventId(d.terrain.edition),
        charge(&d),
    )
    .await
    .expect("réunion");
    let commune = la_seance_commune(&bac, d.terrain.edition).await;
    seances::ouvrir_les_inscriptions(&bac, commune, None, false).await;
    seances::sinscrire(&bac, commune, Some(inscrite), seances::reponses_valides())
        .await
        .expect("inscription à la séance commune");
    let refus = joint::separer(
        &bac.state,
        &bac.ctx(),
        EventId(d.terrain.edition),
        SessionId(commune),
    )
    .await
    .expect_err("séparer perdrait l'inscription");
    assert_eq!(refus.code, ErrorCode::JointSessionHasRegistrations);
}

#[tokio::test]
async fn reunir_refuse_un_dossier_seul_ou_non_retenu() {
    let bac = Bac::monter().await;
    let d = deux_dossiers(&bac).await;

    let mut seul = charge(&d);
    seul.proposal_ids = vec![d.premier, d.premier];
    let refus = joint::reunir(&bac.state, &bac.ctx(), EventId(d.terrain.edition), seul)
        .await
        .expect_err("un seul dossier ne fait pas une séance commune");
    assert_eq!(refus.code, ErrorCode::JointSessionInvalid);

    let en_evaluation = seances::dossier_pret(
        &bac,
        &d.terrain,
        "En évaluation",
        "en-evaluation",
        Souhaits::default(),
    )
    .await;
    let mut non_retenu = charge(&d);
    non_retenu.proposal_ids = vec![d.premier, en_evaluation.id];
    let refus = joint::reunir(
        &bac.state,
        &bac.ctx(),
        EventId(d.terrain.edition),
        non_retenu,
    )
    .await
    .expect_err("un dossier non retenu ne se réunit pas");
    assert_eq!(refus.code, ErrorCode::JointSessionInvalid);
}

#[tokio::test]
async fn la_presentation_du_dossier_suit_dans_la_seance_commune() {
    let bac = Bac::monter().await;
    let d = deux_dossiers(&bac).await;
    joint::reunir(
        &bac.state,
        &bac.ctx(),
        EventId(d.terrain.edition),
        charge(&d),
    )
    .await
    .expect("réunion");

    sqlx::query!(
        r#"UPDATE programme.proposals
              SET detailed_presentation = '{"fr":"<p>Présentation revue.</p>"}'::jsonb
            WHERE id = $1"#,
        d.second
    )
    .execute(bac.pool())
    .await
    .expect("correction du dossier");
    let mut conn = bac.pool().acquire().await.expect("connexion");
    programme::repo::sessions::reporter_les_textes(&mut conn, d.second)
        .await
        .expect("report des textes");

    let apres = sqlx::query!(
        r#"SELECT sp.description ->> 'fr' AS "description?",
                  s.title ->> 'fr' AS "titre_commun?"
             FROM programme.session_proposals sp
             JOIN programme.sessions s ON s.id = sp.session_id
            WHERE sp.proposal_id = $1"#,
        d.second
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture");
    assert_eq!(
        apres.description.as_deref(),
        Some("<p>Présentation revue.</p>")
    );
    assert_eq!(
        apres.titre_commun.as_deref(),
        Some("Littoraux africains"),
        "le titre commun reste celui de l'IFDD"
    );
}

#[tokio::test]
async fn la_page_publique_dune_seance_commune_porte_ses_dossiers() {
    let bac = Bac::monter().await;
    let d = deux_dossiers(&bac).await;
    joint::reunir(
        &bac.state,
        &bac.ctx(),
        EventId(d.terrain.edition),
        charge(&d),
    )
    .await
    .expect("réunion");
    let commune = la_seance_commune(&bac, d.terrain.edition).await;
    sqlx::query!(
        "UPDATE programme.sessions SET published_at = now(), status = 'scheduled' WHERE id = $1",
        commune
    )
    .execute(bac.pool())
    .await
    .expect("publication");
    let adresse = sqlx::query_scalar!(
        r#"SELECT slug::text AS "slug!" FROM programme.sessions WHERE id = $1"#,
        commune
    )
    .fetch_one(bac.pool())
    .await
    .expect("adresse");

    let detail = public_schedule::seance(bac.pool(), EventId(d.terrain.edition), &adresse)
        .await
        .expect("la page se compose");

    let ligne = detail.session.proposals.as_array().expect("tableau");
    assert_eq!(ligne.len(), 2);
    assert_eq!(ligne[0]["title"]["fr"], "Villes côtières");
    assert_eq!(ligne[1]["organization"]["acronym"], "AIMF");
    assert_eq!(detail.session.organization_acronym.as_deref(), Some("IFDD"));

    assert_eq!(detail.proposals.len(), 2);
    for partie in &detail.proposals {
        assert_eq!(partie["speakers"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            partie["organizations"].as_array().map(Vec::len),
            Some(1),
            "le co-organisateur du dossier, porteur exclu"
        );
        assert_eq!(partie["description"]["fr"], "<p>Présentation.</p>");
    }
}
