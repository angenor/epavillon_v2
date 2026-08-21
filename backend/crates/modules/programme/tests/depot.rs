//! **Une organisation dépose son dossier** — du premier enregistrement à la
//! confirmation, avec le même numéro de bout en bout.
//!
//! Les tests appellent les **services**, pas les routes : le montage est
//! éprouvé dans `crates/api/tests/`, et une dépendance de développement vers
//! `api` ferait apparaître l'arête que le jalon interdit.

mod commun;

use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::ProposalId;
use programme::service::submit::ResultatDeDepot;
use programme::service::{draft_write, submit};

/// Un dossier prêt à être déposé : deux intervenants, dans les bornes de
/// l'appel par défaut (1 à 10).
fn complet(terrain: &Terrain, titre: &str) -> programme::domain::draft::ProposalDraft {
    let mut brouillon = commun::brouillon(terrain, titre);
    brouillon.speakers = vec![
        commun::intervenant("awa.sow@example.org", "Awa", "Sow"),
        commun::intervenant("karim.ilboudo@example.org", "Karim", "Ilboudo"),
    ];
    brouillon
}

// -----------------------------------------------------------------------------
// T052 — le parcours nominal, et le numéro qui ne change jamais
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_parcours_nominal_garde_le_meme_numero() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Premier enregistrement : c'est lui qui crée la ligne et attribue le
    // numéro. L'écran peut l'annoncer dès la première frappe.
    let premier = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, complet(&terrain, "Atelier adaptation")),
    )
    .await
    .expect("premier enregistrement");

    assert_eq!(premier.status, "draft");
    assert!(!premier.reference_code.is_empty());

    // Deux enregistrements de plus : le brouillon s'écrit toutes les deux
    // secondes, et rien ne doit bouger sinon la date.
    for titre in ["Atelier adaptation côtière", "Atelier adaptation et eau"] {
        let mut charge = commun::charge(&terrain, complet(&terrain, titre));
        charge.proposal_id = Some(premier.proposal_id);
        let suivant = draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge)
            .await
            .expect("enregistrement suivant");
        assert_eq!(suivant.proposal_id, premier.proposal_id);
        assert_eq!(suivant.reference_code, premier.reference_code);
        assert_eq!(suivant.status, "draft");
    }

    let depot = submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(premier.proposal_id),
        commun::charge(&terrain, complet(&terrain, "Atelier adaptation et eau")),
    )
    .await
    .expect("dépôt");

    match depot {
        ResultatDeDepot::Submitted {
            reference_code,
            required_reviews,
            results_expected_at,
            ..
        } => {
            // **Le même numéro de bout en bout** : le déclencheur l'attribue à
            // l'insertion, pas au dépôt.
            assert_eq!(reference_code, premier.reference_code);
            // Lus sur l'appel, jamais inventés.
            assert_eq!(required_reviews, 2);
            assert_eq!(results_expected_at.as_deref(), Some("2027-09-15"));
        }
        autre => panic!("attendu submitted, reçu {autre:?}"),
    }

    let ligne = commun::ligne(&bac, premier.proposal_id).await;
    assert_eq!(ligne.status, "submitted");
    // Le contact du dossier est le déposant par défaut — règle explicite du
    // service, la colonne étant nullable et rien ne la remplissant (écart n° 30).
    assert_eq!(ligne.contact_person_id, Some(terrain.deposante));
}

// -----------------------------------------------------------------------------
// T053 — le titre vide, et les homonymes (écart n° 95)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_premier_enregistrement_aboutit_avec_un_titre_vide() {
    // C'est le cas du tout premier enregistrement automatique : le formulaire
    // commence par les organisations, et le titre n'a pas encore été touché.
    // `platform.slugify('')` rend NULL et `platform.i18n_text` refuse un
    // français vide — sans repli, la fonctionnalité entière ne démarre pas.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let mut vide = commun::brouillon(&terrain, "");
    vide.objectives = String::new();
    vide.detailed_presentation = String::new();
    vide.summary = String::new();
    vide.target_audiences = Vec::new();
    vide.theme_codes = Vec::new();
    vide.activity_type_code = None;
    vide.format = None;

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, vide),
    )
    .await
    .expect("un brouillon sans titre doit être créé");

    let ligne = commun::ligne(&bac, cree.proposal_id).await;
    assert!(
        !ligne.slug.is_empty(),
        "l'adresse d'URL ne peut pas être vide"
    );
    assert_eq!(ligne.slug, programme::domain::slug::REPLI);
    assert_eq!(ligne.status, "draft");
    assert!(!cree.reference_code.is_empty());
}

#[tokio::test]
async fn deux_dossiers_homonymes_recoivent_deux_adresses() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let premier = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, commun::brouillon(&terrain, "Atelier adaptation")),
    )
    .await
    .expect("premier dossier");

    // Exactement le même titre, dans la même édition : `ux_proposals_slug`
    // refuserait la seconde insertion si le service ne suffixait pas.
    let second = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, commun::brouillon(&terrain, "Atelier adaptation")),
    )
    .await
    .expect("le second dossier homonyme doit aboutir");

    let a = commun::ligne(&bac, premier.proposal_id).await;
    let b = commun::ligne(&bac, second.proposal_id).await;
    assert_eq!(a.slug, "atelier-adaptation");
    assert_eq!(b.slug, "atelier-adaptation-2");
    assert_ne!(a.reference_code, b.reference_code);
}

// -----------------------------------------------------------------------------
// T054 — le dossier naît en brouillon (écart n° 96)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_etat_demande_par_le_client_nest_jamais_honore() {
    // Le garde d'état n'est posé que sur la MISE À JOUR de `status` : une
    // insertion lui échappe, et un dossier pourrait naître « retenu ». Le
    // service ne lit donc jamais l'état demandé — ici envoyé en trop dans la
    // charge utile, comme le ferait un client bricolé.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let brouillon = commun::brouillon(&terrain, "Dossier forcé");
    let mut charge = serde_json::to_value(commun::charge(&terrain, brouillon))
        .expect("sérialisation de la charge utile");
    charge["status"] = serde_json::json!("accepted");
    charge["draft"]["status"] = serde_json::json!("accepted");

    let charge: programme::domain::draft::SaveDraftPayload =
        serde_json::from_value(charge).expect("la charge utile reste lisible");

    let cree = draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge)
        .await
        .expect("création");

    assert_eq!(cree.status, "draft");
    assert_eq!(commun::ligne(&bac, cree.proposal_id).await.status, "draft");

    // Et le journal du modèle n'a qu'une ligne : l'ouverture du dossier.
    let journal = commun::journal(&bac, cree.proposal_id).await;
    assert_eq!(journal, vec![(None, "draft".to_owned())]);
}

// -----------------------------------------------------------------------------
// T055 — les trois refus de recevabilité, chacun portant sa valeur (R9)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_appel_clos_rend_son_echeance_et_non_une_erreur() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, complet(&terrain, "Trop tard")),
    )
    .await
    .expect("création");

    // L'échéance tombe entre le chargement de la page et le clic : c'est
    // exactement le cas que le contrat prévoit.
    commun::fermer_lappel(&bac, terrain.appel).await;

    let issue = submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(cree.proposal_id),
        commun::charge(&terrain, complet(&terrain, "Trop tard")),
    )
    .await
    .expect("un appel clos est une RÉPONSE, pas une erreur");

    match issue {
        // La valeur est ce qui compte : l'écran doit dire QUAND l'appel a
        // fermé, ce dont l'organisation a précisément besoin.
        ResultatDeDepot::CallClosed { deadline } => {
            assert!(deadline < time::OffsetDateTime::now_utc());
        }
        autre => panic!("attendu call_closed, reçu {autre:?}"),
    }

    assert_eq!(commun::ligne(&bac, cree.proposal_id).await.status, "draft");
}

#[tokio::test]
async fn le_plafond_atteint_rend_sa_valeur() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    commun::plafonner(&bac, terrain.appel, 1).await;

    for titre in ["Premier dossier", "Second dossier"] {
        let cree = draft_write::enregistrer(
            &bac.state,
            &bac.ctx(),
            terrain.deposante,
            commun::charge(&terrain, complet(&terrain, titre)),
        )
        .await
        .expect("création");

        let issue = submit::deposer(
            &bac.state,
            &bac.ctx(),
            terrain.deposante,
            ProposalId(cree.proposal_id),
            commun::charge(&terrain, complet(&terrain, titre)),
        )
        .await
        .expect("le plafond est une RÉPONSE");

        match (titre, issue) {
            ("Premier dossier", ResultatDeDepot::Submitted { .. }) => {}
            ("Second dossier", ResultatDeDepot::QuotaReached { max }) => assert_eq!(max, 1),
            (t, autre) => panic!("issue inattendue pour {t} : {autre:?}"),
        }
    }
}

#[tokio::test]
async fn une_organisation_non_verifiee_est_refusee_quand_lappel_lexige() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    sqlx::query!(
        "UPDATE event.calls_for_proposals SET requires_verified_organization = true WHERE id = $1",
        terrain.appel
    )
    .execute(bac.pool())
    .await
    .expect("l'appel exige désormais une organisation vérifiée");
    sqlx::query!(
        "UPDATE org.organizations SET verified_at = NULL WHERE id = $1",
        terrain.organisation
    )
    .execute(bac.pool())
    .await
    .expect("l'organisation n'est plus vérifiée");

    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, complet(&terrain, "Sans sceau")),
    )
    .await
    .expect("création");

    let issue = submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(cree.proposal_id),
        commun::charge(&terrain, complet(&terrain, "Sans sceau")),
    )
    .await
    .expect("le troisième refus est une réponse lui aussi");

    assert!(matches!(issue, ResultatDeDepot::OrganizationNotVerified));
}

// -----------------------------------------------------------------------------
// T056 — les bornes de l'appel, chacune sur son champ (écart n° 27)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn les_bornes_dintervenants_sappliquent_au_depot_et_pas_avant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Un brouillon SANS intervenant s'enregistre : la saisie se construit
    // intervenant par intervenant, et refuser le premier enregistrement
    // rendrait le formulaire inutilisable.
    let cree = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, commun::brouillon(&terrain, "Sans intervenant")),
    )
    .await
    .expect("un brouillon sans intervenant s'enregistre");

    // Au dépôt, la borne basse de l'appel s'applique — et aucun déclencheur ne
    // la vérifie.
    let refus = submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(cree.proposal_id),
        commun::charge(&terrain, commun::brouillon(&terrain, "Sans intervenant")),
    )
    .await
    .expect_err("l'appel demande au moins un intervenant");

    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("speakers"));
}

#[tokio::test]
async fn la_duree_hors_bornes_de_lappel_est_refusee_sur_son_champ() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // La colonne accepte 15 à 600 ; l'appel, lui, borne à 45–150 par défaut.
    // Ce sont des règles de CAMPAGNE, et ce sont elles qui refusent en premier.
    let mut trop_court = complet(&terrain, "Trop court");
    trop_court.duration_minutes = Some(20);

    let refus = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, trop_court),
    )
    .await
    .expect_err("vingt minutes est sous la borne de l'appel");

    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("duration_minutes"));
    assert!(refus.message.contains("45"), "{}", refus.message);
}

#[tokio::test]
async fn la_plage_horaire_quotidienne_est_refusee_fin_comprise() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Le stand ouvre de 9 h à 17 h. Une activité de 60 minutes commençant à
    // 16:30 finirait à 17:30 : elle ne tient pas.
    let mut tardif = complet(&terrain, "Trop tard dans la journée");
    tardif.preferred_start_at = Some("2027-11-12T16:30".to_owned());
    tardif.duration_minutes = Some(60);

    let refus = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, tardif),
    )
    .await
    .expect_err("l'activité déborderait la fermeture du stand");
    assert_eq!(refus.field.as_deref(), Some("preferred_start_at"));

    // **Fin comprise** : la même activité terminée pile à 17:00 est acceptée.
    let mut juste = complet(&terrain, "Juste à la fermeture");
    juste.preferred_start_at = Some("2027-11-12T16:00".to_owned());
    juste.duration_minutes = Some(60);

    draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, juste),
    )
    .await
    .expect("une activité qui se termine à l'heure de fermeture tient");
}

#[tokio::test]
async fn un_format_hors_de_ceux_de_lappel_est_refuse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    sqlx::query!(
        "UPDATE event.calls_for_proposals
            SET allowed_formats = ARRAY['online']::event.participation_mode[]
          WHERE id = $1",
        terrain.appel
    )
    .execute(bac.pool())
    .await
    .expect("l'appel n'accepte plus que le distanciel");

    let mut presentiel = complet(&terrain, "En présentiel");
    presentiel.format = Some("in_person".to_owned());

    let refus = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, presentiel),
    )
    .await
    .expect_err("un cycle de webinaires ne reçoit pas de séance en présentiel");

    assert_eq!(refus.field.as_deref(), Some("format"));
}
