//! **L'espace organisation : ce qu'il montre, et surtout ce qu'il ne montre
//! pas.**
//!
//! Le test central de ce fichier ne vérifie pas des champs : il **balaie la
//! charge utile entière**. C'est la seule forme qui prouve quelque chose ici —
//! vérifier que `average_score` est nul laisse passer la note qui arriverait
//! par un champ ajouté demain, et c'est exactement le défaut que FR-076 et
//! FR-077 existent pour empêcher.

mod commun;

use commun::Bac;
use kernel::error::ErrorCode;
use programme::domain::ids::{CommentId, ProposalId};
use programme::service::comments::{self, PostCommentPayload};
use programme::service::workspace;
use uuid::Uuid;

/// Un membre du comité qui note et écrit.
async fn membre_du_comite(bac: &Bac, edition: Uuid, appel: Uuid) -> Uuid {
    let personne = commun::personne(bac, "noteuse@ifdd.francophonie.org", "Nour", "Comite").await;
    commun::attribuer(bac, personne, "reviewer", "event", Some(edition)).await;
    sqlx::query!(
        "INSERT INTO event.call_reviewers (call_id, person_id) VALUES ($1, $2)",
        appel,
        personne
    )
    .execute(bac.pool())
    .await
    .expect("inscription au comité");
    personne
}

/// Poser une note consolidée et un rang, comme le comité les produirait.
async fn noter(bac: &Bac, dossier: Uuid) {
    sqlx::query!(
        "UPDATE programme.proposals
            SET average_score = 17.25, weighted_score = 86.00,
                review_count = 3, is_knocked_out = true
          WHERE id = $1",
        dossier
    )
    .execute(bac.pool())
    .await
    .expect("pose des agrégats");
}

// -----------------------------------------------------------------------------
// T124 — le balayage de la charge utile entière
// -----------------------------------------------------------------------------

/// **Rien du comité n'entre dans l'espace du déposant.**
///
/// Ni note, ni note pondérée, ni élimination, ni note personnelle, ni
/// délibération. Le test cherche les **valeurs** dans la réponse sérialisée :
/// un champ ajouté demain qui les porterait le ferait tomber, ce qu'une
/// assertion champ par champ ne ferait pas.
#[tokio::test]
async fn lespace_du_deposant_ne_porte_rien_du_comite() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let noteuse = membre_du_comite(&bac, terrain.edition, terrain.appel).await;

    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;
    commun::dossier(&bac, &terrain, "Second dossier", "second-dossier").await;
    noter(&bac, dossier).await;

    // Une délibération interne et une note personnelle, que rien ne doit sortir.
    for (visibilite, corps) in [
        ("committee", "Délibération : dossier trop mince."),
        ("private", "Ma note à moi, à ne jamais montrer."),
        ("submitter", "Merci de préciser le format attendu."),
    ] {
        comments::ecrire(
            &bac.state,
            &bac.ctx(),
            noteuse,
            ProposalId(dossier),
            PostCommentPayload {
                parent_id: None,
                visibility: Some(visibilite.to_owned()),
                body: corps.to_owned(),
                is_change_request: false,
            },
        )
        .await
        .expect("écriture du message");
    }

    let espace = workspace::espace(&bac.state, terrain.deposante, terrain.organisation)
        .await
        .expect("pas de panne")
        .expect("la déposante ouvre l'espace de son organisation");
    let fichier = workspace::dossier(&bac.state, terrain.deposante, ProposalId(dossier))
        .await
        .expect("pas de panne")
        .expect("elle ouvre son dossier");

    let charge = format!(
        "{}{}",
        serde_json::to_string(&espace).expect("sérialisation de l'espace"),
        serde_json::to_string(&fichier).expect("sérialisation du dossier")
    );

    for interdit in [
        "17.25",
        "86.0",
        "Délibération",
        "Ma note à moi",
        "Nour",
        "noteuse@ifdd.francophonie.org",
    ] {
        assert!(
            !charge.contains(interdit),
            "« {interdit} » ne doit pas franchir l'espace organisation"
        );
    }

    assert_eq!(espace.proposals.len(), 2);
    for suivi in &espace.proposals {
        assert_eq!(suivi.proposal.average_score, None);
        assert_eq!(suivi.proposal.weighted_score, None);
        assert!(!suivi.proposal.is_knocked_out);
        // Les séances appartiennent à B5 : la liste est **vide**, jamais
        // absente — un champ absent ferait échouer l'écran.
        assert!(suivi.sessions.is_empty());
    }

    // Le journal, lui, est bien là : c'est de lui que la frise se compose.
    let suivi = espace
        .proposals
        .iter()
        .find(|s| s.proposal.id == dossier)
        .expect("le dossier noté");
    assert_eq!(
        suivi.transitions.len(),
        1,
        "la ligne d'ouverture du dossier"
    );
}

// -----------------------------------------------------------------------------
// T125 — trois visibilités, un seul message rendu
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_deposant_ne_recoit_que_ce_qui_lui_est_adresse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let noteuse = membre_du_comite(&bac, terrain.edition, terrain.appel).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;

    for visibilite in ["committee", "private", "submitter"] {
        comments::ecrire(
            &bac.state,
            &bac.ctx(),
            noteuse,
            ProposalId(dossier),
            PostCommentPayload {
                parent_id: None,
                visibility: Some(visibilite.to_owned()),
                body: format!("Message {visibilite}."),
                is_change_request: false,
            },
        )
        .await
        .expect("écriture");
    }

    let fichier = workspace::dossier(&bac.state, terrain.deposante, ProposalId(dossier))
        .await
        .expect("pas de panne")
        .expect("le dossier du déposant");

    assert_eq!(
        fichier.comments.len(),
        1,
        "un seul des trois lui est adressé"
    );
    assert_eq!(fichier.comments[0].visibility, "submitter");
    // **Et son auteur n'est pas nommé** : seuls les membres de l'organisation
    // porteuse le sont (écart n° 109). L'écran affiche un libellé neutre.
    assert!(fichier.participants.is_empty());
}

// -----------------------------------------------------------------------------
// T126 — la résolution posée, retirée, et le compteur qui suit
// -----------------------------------------------------------------------------

/// **Le déposant pose, le comité retire** (écart n° 35).
///
/// Et le compteur de demandes ouvertes suit les deux gestes : c'est lui que
/// l'écran crie, et il est relu à chaque affichage — voilà pourquoi la
/// résolution n'émet aucun événement.
#[tokio::test]
async fn la_resolution_se_pose_et_se_retire_et_le_compteur_suit() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let noteuse = membre_du_comite(&bac, terrain.edition, terrain.appel).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;

    let demande = comments::ecrire(
        &bac.state,
        &bac.ctx(),
        noteuse,
        ProposalId(dossier),
        PostCommentPayload {
            parent_id: None,
            visibility: Some("committee".to_owned()),
            body: "Le résumé ne dit pas ce que l'atelier produit.".to_owned(),
            is_change_request: true,
        },
    )
    .await
    .expect("la demande de correction");
    assert_eq!(demande.visibility, "submitter", "forcée en partagé");

    async fn ouvertes(bac: &Bac, lecteur: Uuid, dossier: Uuid) -> i64 {
        workspace::dossier(&bac.state, lecteur, ProposalId(dossier))
            .await
            .expect("pas de panne")
            .expect("le dossier")
            .tracking
            .open_change_requests
    }

    assert_eq!(ouvertes(&bac, terrain.deposante, dossier).await, 1);

    // **Le déposant pose** : c'est lui qui sait qu'il a corrigé.
    let resolue = comments::resoudre(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        CommentId(demande.id),
        true,
    )
    .await
    .expect("la déposante marque la demande résolue");
    assert!(resolue.resolved_at.is_some());
    assert_eq!(resolue.resolved_by, Some(terrain.deposante));
    assert_eq!(ouvertes(&bac, terrain.deposante, dossier).await, 0);

    // **Le déposant ne retire pas** : ce serait effacer un arbitrage.
    let refus = comments::resoudre(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        CommentId(demande.id),
        false,
    )
    .await
    .expect_err("le déposant ne rouvre pas une demande");
    assert_eq!(refus.code, ErrorCode::Forbidden);

    // **Le comité, si.**
    let rouverte = comments::resoudre(
        &bac.state,
        &bac.ctx(),
        noteuse,
        CommentId(demande.id),
        false,
    )
    .await
    .expect("le comité rouvre la demande");
    assert!(rouverte.resolved_at.is_none());
    assert!(rouverte.resolved_by.is_none());
    assert_eq!(ouvertes(&bac, terrain.deposante, dossier).await, 1);

    // Rien n'a été annoncé par ces deux gestes : le seul événement du fil est
    // celui du message partagé.
    let emis = commun::evenements_emis(&bac, demande.id).await;
    assert_eq!(emis, vec!["programme.comment.shared".to_owned()]);
}

// -----------------------------------------------------------------------------
// T127 — une personne étrangère à l'organisation
// -----------------------------------------------------------------------------

/// **L'adhésion active, et rien d'autre.**
///
/// Un administrateur de l'édition n'entre pas dans l'espace d'une organisation
/// dont il n'est pas membre : il a la fiche du comité pour cela. Et le refus
/// est celui d'une ressource inexistante.
#[tokio::test]
async fn ladhesion_active_est_le_seul_droit_dentree() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;

    // **Le refus n'est pas une panne** : il rend `None`, que la route sérialise
    // en `null`. Un statut d'erreur faisait afficher « une erreur est survenue »
    // sur les trois écrans de l'espace, là où il faut lire « vous n'avez pas
    // d'espace ici ». L'indiscernabilité voulue est intacte — étrangère et
    // inexistante rendent la même chose, c'est-à-dire rien.
    let quidam = commun::personne(&bac, "quidam@example.org", "Quid", "Am").await;
    let refus = workspace::espace(&bac.state, quidam, terrain.organisation)
        .await
        .expect("une personne étrangère n'est pas une panne");
    let inexistante = workspace::espace(&bac.state, quidam, Uuid::now_v7())
        .await
        .expect("une organisation inexistante n'est pas une panne");
    assert!(refus.is_none());
    assert!(inexistante.is_none());

    // **Un administrateur de l'édition n'y entre pas non plus.**
    let droits = commun::droits(&bac, &terrain).await;
    assert!(
        workspace::espace(&bac.state, droits.decideur, terrain.organisation)
            .await
            .expect("pas de panne")
            .is_none(),
        "le périmètre d'administration n'ouvre pas l'espace organisation"
    );
    assert!(
        workspace::dossier(&bac.state, droits.decideur, ProposalId(dossier))
            .await
            .expect("pas de panne")
            .is_none(),
        "ni le dossier vu par son déposant"
    );
    assert!(
        workspace::editions(&bac.state, droits.decideur, terrain.organisation)
            .await
            .expect("pas de panne")
            .is_none(),
        "ni la liste de ses éditions — et `None`, jamais une liste vide : « aucun          dossier » et « ce n'est pas votre espace » ne se confondent pas"
    );

    // **Une adhésion en attente ne suffit pas** : « active » est le mot du
    // modèle, et un membre invité qui n'a pas répondu n'écrit rien.
    let invitee = commun::personne(&bac, "invitee@example.org", "Ines", "Vitee").await;
    commun::adherer(&bac, terrain.organisation, invitee, "pending").await;
    assert!(
        workspace::espace(&bac.state, invitee, terrain.organisation)
            .await
            .expect("pas de panne")
            .is_none(),
        "une adhésion en attente n'ouvre rien"
    );
}

/// Les éditions et le bloc « ce qui attend une action » se composent sur ce que
/// l'organisation a réellement fait.
#[tokio::test]
async fn les_editions_et_les_actions_se_composent_sur_les_dossiers() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let noteuse = membre_du_comite(&bac, terrain.edition, terrain.appel).await;
    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;

    comments::ecrire(
        &bac.state,
        &bac.ctx(),
        noteuse,
        ProposalId(dossier),
        PostCommentPayload {
            parent_id: None,
            visibility: None,
            body: "Le résumé est à revoir.".to_owned(),
            is_change_request: true,
        },
    )
    .await
    .expect("la demande de correction");

    let editions = workspace::editions(&bac.state, terrain.deposante, terrain.organisation)
        .await
        .expect("pas de panne")
        .expect("les éditions");
    assert_eq!(editions.len(), 1);
    assert_eq!(editions[0].id, terrain.edition);

    let espace = workspace::espace(&bac.state, terrain.deposante, terrain.organisation)
        .await
        .expect("pas de panne")
        .expect("l'espace");

    let natures: Vec<&str> = espace.actions.iter().map(|a| a.kind.as_str()).collect();
    assert!(
        natures.contains(&"changes_requested"),
        "une correction demandée attend une action de l'organisation — reçu : {natures:?}"
    );
    assert!(
        natures.contains(&"draft_before_deadline"),
        "un brouillon avant l'échéance aussi — reçu : {natures:?}"
    );

    // L'appel ouvert est mis en avant : c'est la seule chose utile à montrer à
    // qui n'a rien déposé.
    assert!(espace.open_call.is_some());
    assert_eq!(
        espace.call_edition.as_ref().map(|e| e.id),
        Some(terrain.edition)
    );

    // Et la personne connectée y lit sa propre adhésion.
    assert_eq!(espace.membership.person_id, terrain.deposante);
    assert_eq!(espace.membership.status, "active");
    // La noteuse siège au comité de l'appel ; elle n'est pas membre de
    // l'organisation, et n'a donc rien à faire dans sa liste de membres.
    assert_eq!(espace.members.len(), 1);
    assert_eq!(espace.members[0].membership.person_id, terrain.deposante);
    assert!(!espace.members[0].is_invitation);
}
