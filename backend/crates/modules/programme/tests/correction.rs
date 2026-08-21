//! **Corriger un dossier déposé, et le renvoyer — sans toucher à la séance.**
//!
//! Quatre choses s'y prouvent :
//!
//! - **la recomposition, champ à champ**, et notamment l'heure murale : un
//!   créneau saisi à 14:30 à Belém doit se rouvrir à 14:30, quel que soit le
//!   fuseau d'où l'on regarde — le test le vérifie depuis un autre fuseau ;
//! - **le renvoi sur appel clos aboutit**, et le même dossier par la route de
//!   dépôt est refusé : la fenêtre ne borne que le premier dépôt ;
//! - **corriger ne change pas l'état** : un dossier en évaluation ne repart pas
//!   au comité parce qu'on a rectifié une faute de frappe ;
//! - **corriger un dossier retenu laisse sa séance strictement inchangée** —
//!   créneau, salle, capacité. C'est le test qui coûterait le plus cher à ne
//!   pas avoir : déplacer une séance à laquelle quarante personnes se sont
//!   inscrites ne se rattrape pas.

mod commun;

use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::ProposalId;
use programme::service::{draft_read, draft_write, resubmit, submit, transition};
use uuid::Uuid;

/// Une charge d'enregistrement complète : l'appel exige au moins un
/// intervenant, et un renvoi réécrit le dossier entier.
fn charge_avec_intervenant(
    terrain: &Terrain,
    titre: &str,
) -> programme::domain::draft::SaveDraftPayload {
    let mut brouillon = commun::brouillon(terrain, titre);
    brouillon.speakers = vec![commun::intervenant("awa.sow@example.org", "Awa", "Sow")];
    commun::charge(terrain, brouillon)
}

/// Un dossier déposé par le service, avec tout ce que le formulaire écrit.
async fn dossier_complet(bac: &Bac, terrain: &Terrain, titre: &str) -> Uuid {
    let mut brouillon = commun::brouillon(terrain, titre);
    brouillon.preferred_start_at = Some("2027-11-12T14:30".to_owned());
    brouillon.duration_minutes = Some(90);
    brouillon.summary = "Un résumé.".to_owned();
    brouillon.expected_outcomes = "Des résultats.".to_owned();
    brouillon.target_audiences = vec!["Ministères".to_owned(), "ONG".to_owned()];
    brouillon.scheduling_constraints = "Pas le matin.".to_owned();
    brouillon.speakers = vec![commun::intervenant("awa.sow@example.org", "Awa", "Sow")];
    brouillon.theme_codes = vec!["adaptation".to_owned()];

    let charge = commun::charge(terrain, brouillon);
    let ligne = draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge.clone())
        .await
        .expect("enregistrement du brouillon");

    submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(ligne.proposal_id),
        charge,
    )
    .await
    .expect("dépôt");

    ligne.proposal_id
}

// -----------------------------------------------------------------------------
// T134 — la recomposition, champ à champ
// -----------------------------------------------------------------------------

/// **Le créneau se rouvre à l'heure où il a été saisi, d'où qu'on regarde.**
///
/// La conversion se fait en base dans le fuseau de l'**édition**. Belém est à
/// trois heures derrière l'UTC : si la recomposition lisait l'instant sans le
/// convertir, elle rendrait 17:30, et si elle le convertissait dans le fuseau
/// du serveur, elle rendrait autre chose encore. C'est ce décalage qui rend le
/// test discriminant.
#[tokio::test]
async fn la_recomposition_rend_le_dossier_tel_quil_a_ete_saisi() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = dossier_complet(&bac, &terrain, "Atelier adaptation").await;

    let rouvert = draft_read::rouvrir(&bac.state, terrain.deposante, ProposalId(dossier))
        .await
        .expect("la recomposition");

    assert_eq!(rouvert.proposal_id, dossier);
    assert_eq!(rouvert.status, "submitted");
    assert_eq!(rouvert.draft.draft.title, "Atelier adaptation");
    assert_eq!(rouvert.draft.draft.summary, "Un résumé.");
    assert_eq!(rouvert.draft.draft.expected_outcomes, "Des résultats.");
    assert_eq!(
        rouvert.draft.draft.target_audiences,
        vec!["Ministères".to_owned(), "ONG".to_owned()]
    );
    assert_eq!(rouvert.draft.draft.scheduling_constraints, "Pas le matin.");
    assert_eq!(rouvert.draft.draft.duration_minutes, Some(90));
    assert_eq!(
        rouvert.draft.draft.theme_codes,
        vec!["adaptation".to_owned()]
    );
    assert_eq!(
        rouvert.draft.draft.organization_id,
        Some(terrain.organisation)
    );

    // **L'heure murale, et rien d'autre.**
    assert_eq!(
        rouvert.draft.draft.preferred_start_at.as_deref(),
        Some("2027-11-12T14:30"),
        "le créneau se rouvre dans le fuseau de l'édition, pas en UTC"
    );

    // Et l'instant stocké est bien 17:30 UTC : c'est la seconde lecture qui
    // prouve que la conversion a eu lieu dans les deux sens.
    let utc = sqlx::query_scalar!(
        r#"SELECT to_char(preferred_start_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI')
             FROM programme.proposals WHERE id = $1"#,
        dossier
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture de l'instant");
    assert_eq!(utc.as_deref(), Some("2027-11-12T17:30"));

    // L'intervenant porte son verrouillage d'identité.
    assert_eq!(rouvert.draft.speakers.len(), 1);
    let intervenant = &rouvert.draft.speakers[0];
    assert_eq!(intervenant.email, "awa.sow@example.org");
    assert_eq!(intervenant.first_name, "Awa");
    assert!(
        !intervenant.has_account,
        "cette personne n'a pas de compte : son identité reste modifiable"
    );

    commun::donner_un_compte(&bac, intervenant.person_id.expect("la personne")).await;
    let rouvert = draft_read::rouvrir(&bac.state, terrain.deposante, ProposalId(dossier))
        .await
        .expect("la recomposition");
    assert!(
        rouvert.draft.speakers[0].has_account,
        "dès qu'un compte existe, l'identité est verrouillée"
    );
}

/// **Les textes provisoires ne reviennent jamais au formulaire** (écart
/// n° 102).
#[tokio::test]
async fn les_textes_provisoires_sont_effaces_a_la_recomposition() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Le tout premier enregistrement automatique : le formulaire commence par
    // les organisations, aucun texte n'a encore été tapé.
    let mut brouillon = commun::brouillon(&terrain, "");
    brouillon.objectives = String::new();
    brouillon.detailed_presentation = String::new();
    let ligne = draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, brouillon),
    )
    .await
    .expect("le premier enregistrement");

    // En base, les replis sont bien là — sans quoi la ligne n'existerait pas.
    let en_base = sqlx::query_scalar!(
        "SELECT title ->> 'fr' FROM programme.proposals WHERE id = $1",
        ligne.proposal_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture du titre");
    assert_eq!(en_base.as_deref(), Some("Dossier sans titre"));

    // …et le formulaire ne les voit pas.
    let rouvert = draft_read::rouvrir(&bac.state, terrain.deposante, ProposalId(ligne.proposal_id))
        .await
        .expect("la recomposition");
    assert_eq!(rouvert.draft.draft.title, "");
    assert_eq!(rouvert.draft.draft.objectives, "");
    assert_eq!(rouvert.draft.draft.detailed_presentation, "");
}

// -----------------------------------------------------------------------------
// T135 et T136 — le renvoi, la fenêtre et le plafond
// -----------------------------------------------------------------------------

/// **Un renvoi sur appel clos aboutit ; le même dossier par la route de dépôt
/// est refusé** (écart n° 38).
#[tokio::test]
async fn le_renvoi_franchit_la_cloture_que_le_depot_ne_franchit_pas() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = dossier_complet(&bac, &terrain, "Atelier adaptation").await;
    let id = ProposalId(dossier);

    // Le comité demande une correction, puis l'appel ferme.
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        id,
        programme::domain::transitions::ProposalStatus::ChangesRequested,
        Some("Le résumé est à revoir."),
    )
    .await
    .expect("demande de correction");
    let _ = droits;
    commun::fermer_lappel(&bac, terrain.appel).await;

    let charge = charge_avec_intervenant(&terrain, "Atelier adaptation");

    // **Par la route de dépôt : refusé**, et pas parce que l'appel est clos —
    // parce que ce dossier n'est plus un brouillon.
    let refus = submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        id,
        charge.clone(),
    )
    .await
    .expect_err("un dossier corrigé ne se dépose pas une seconde fois");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);

    // **Par la route de renvoi : accepté**, appel clos ou non.
    let resultat = resubmit::renvoyer(&bac.state, &bac.ctx(), terrain.deposante, id, charge)
        .await
        .expect("le renvoi aboutit");
    assert!(
        matches!(resultat, submit::ResultatDeDepot::Submitted { .. }),
        "la fenêtre ne borne que le premier dépôt — reçu : {resultat:?}"
    );

    let etat = commun::ligne(&bac, dossier).await;
    assert_eq!(etat.status, "submitted");
}

/// **Le plafond, lui, s'applique au renvoi** : il compte les dossiers en
/// course, et un renvoi en remet un.
#[tokio::test]
async fn le_plafond_refuse_aussi_un_renvoi_excedentaire() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let a_renvoyer = dossier_complet(&bac, &terrain, "Premier dossier").await;
    dossier_complet(&bac, &terrain, "Second dossier").await;

    transition::tenter(
        &bac.state,
        &bac.ctx(),
        ProposalId(a_renvoyer),
        programme::domain::transitions::ProposalStatus::ChangesRequested,
        Some("À revoir."),
    )
    .await
    .expect("demande de correction");

    // Un seul dossier admis par organisation : le second est déjà en course.
    commun::plafonner(&bac, terrain.appel, 1).await;

    let charge = charge_avec_intervenant(&terrain, "Premier dossier");
    let resultat = resubmit::renvoyer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(a_renvoyer),
        charge,
    )
    .await
    .expect("l'appel aboutit, le refus est une réponse");

    assert!(
        matches!(resultat, submit::ResultatDeDepot::QuotaReached { max: 1 }),
        "le plafond s'applique au renvoi — reçu : {resultat:?}"
    );
}

// -----------------------------------------------------------------------------
// T137 — corriger n'est pas déposer
// -----------------------------------------------------------------------------

/// **Un dossier en évaluation ne repart pas au comité parce qu'on a rectifié
/// une faute de frappe.**
#[tokio::test]
async fn corriger_un_dossier_en_evaluation_ne_change_pas_son_etat() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = dossier_complet(&bac, &terrain, "Atelier adaptation").await;
    let id = ProposalId(dossier);

    transition::tenter(
        &bac.state,
        &bac.ctx(),
        id,
        programme::domain::transitions::ProposalStatus::UnderReview,
        None,
    )
    .await
    .expect("mise en évaluation");

    let avant = commun::journal(&bac, dossier).await.len();

    let mut brouillon = commun::brouillon(&terrain, "Atelier adaptation côtière");
    brouillon.speakers = vec![commun::intervenant("awa.sow@example.org", "Awa", "Sow")];
    let mut charge = commun::charge(&terrain, brouillon);
    charge.proposal_id = Some(dossier);

    draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge)
        .await
        .expect("la correction");

    let apres = commun::ligne(&bac, dossier).await;
    assert_eq!(apres.status, "under_review", "l'état n'a pas bougé");
    assert_eq!(
        commun::journal(&bac, dossier).await.len(),
        avant,
        "aucune ligne de journal n'a été écrite"
    );

    let rouvert = draft_read::rouvrir(&bac.state, terrain.deposante, id)
        .await
        .expect("la recomposition");
    assert_eq!(rouvert.draft.draft.title, "Atelier adaptation côtière");
}

/// **Une édition terminée ferme la modification** — arbitrage du 17/08.
#[tokio::test]
async fn une_edition_terminee_ferme_la_modification() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = dossier_complet(&bac, &terrain, "Atelier adaptation").await;

    sqlx::query!(
        "UPDATE event.events
            SET starts_at = now() - interval '10 days',
                ends_at = now() - interval '3 days'
          WHERE id = $1",
        terrain.edition
    )
    .execute(bac.pool())
    .await
    .expect("l'édition est terminée");

    let mut charge = commun::charge(&terrain, commun::brouillon(&terrain, "Trop tard"));
    charge.proposal_id = Some(dossier);

    let refus = draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge)
        .await
        .expect_err("une édition terminée ne se corrige plus");
    assert_eq!(refus.code, ErrorCode::ProposalNotEditable);
}

// -----------------------------------------------------------------------------
// T138 — la séance reste strictement inchangée
// -----------------------------------------------------------------------------

/// **Corriger un dossier retenu ne déplace pas sa séance.**
///
/// Une séance retenue a un créneau **arbitré** par l'IFDD, une salle attribuée,
/// des inscrits prévenus. Recopier dessus le créneau *souhaité* d'un dossier
/// corrigé déplacerait une séance à laquelle des gens se sont inscrits, sans
/// que personne l'ait demandé (FR-091).
#[tokio::test]
async fn corriger_un_dossier_retenu_laisse_sa_seance_inchangee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = dossier_complet(&bac, &terrain, "Atelier adaptation").await;
    let id = ProposalId(dossier);

    for vers in [
        programme::domain::transitions::ProposalStatus::UnderReview,
        programme::domain::transitions::ProposalStatus::Accepted,
    ] {
        transition::tenter(&bac.state, &bac.ctx(), id, vers, None)
            .await
            .unwrap_or_else(|e| panic!("transition vers {vers:?} : {e}"));
    }

    // La séance, programmée par l'IFDD sur un créneau ARBITRÉ — différent du
    // créneau souhaité, comme il l'est presque toujours.
    let seance = sqlx::query!(
        r#"INSERT INTO programme.sessions
               (event_id, proposal_id, organization_id, title, slug, format,
                starts_at, ends_at, timezone, capacity)
           VALUES ($1, $2, $3,
                   '{"fr":"Atelier adaptation"}'::jsonb,
                   'seance-atelier-adaptation'::platform.slug,
                   'hybrid',
                   timestamp '2027-11-14 09:00' AT TIME ZONE 'America/Belem',
                   timestamp '2027-11-14 10:30' AT TIME ZONE 'America/Belem',
                   'America/Belem'::platform.timezone_name, 40)
        RETURNING id, starts_at, ends_at, title, capacity, format::text AS "format!""#,
        terrain.edition,
        dossier,
        terrain.organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("programmation de la séance");

    // La correction change le titre, le créneau souhaité, le format et la durée.
    let mut brouillon = commun::brouillon(&terrain, "Atelier adaptation — version corrigée");
    brouillon.preferred_start_at = Some("2027-11-20T16:00".to_owned());
    brouillon.duration_minutes = Some(45);
    brouillon.format = Some("online".to_owned());
    brouillon.speakers = vec![commun::intervenant("awa.sow@example.org", "Awa", "Sow")];
    let mut charge = commun::charge(&terrain, brouillon);
    charge.proposal_id = Some(dossier);

    draft_write::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, charge)
        .await
        .expect("la correction d'un dossier retenu est permise");

    let apres = sqlx::query!(
        r#"SELECT id, starts_at, ends_at, title, capacity, format::text AS "format!",
                  room_id, status::text AS "status!"
             FROM programme.sessions WHERE id = $1"#,
        seance.id
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture de la séance");

    assert_eq!(
        apres.starts_at, seance.starts_at,
        "le créneau n'a pas bougé"
    );
    assert_eq!(apres.ends_at, seance.ends_at);
    assert_eq!(apres.title, seance.title, "le titre de la séance non plus");
    assert_eq!(apres.capacity, seance.capacity);
    assert_eq!(apres.format, seance.format);
    assert_eq!(apres.status, "planned");

    // Et le dossier, lui, a bien changé.
    let rouvert = draft_read::rouvrir(&bac.state, terrain.deposante, id)
        .await
        .expect("la recomposition");
    assert_eq!(
        rouvert.draft.draft.title,
        "Atelier adaptation — version corrigée"
    );
}
