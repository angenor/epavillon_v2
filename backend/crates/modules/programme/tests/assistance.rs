//! **L'assistance sur toute la plateforme** (02/11) : qui la voit, les sujets
//! sans dossier, ce que chaque côté lit, la lecture et la résolution.

mod commun;

use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::{CommentId, ProposalId};
use programme::service::assistance::{
    self, MarkAssistanceReadPayload, OpenSupportSubjectPayload, ReplySupportSubjectPayload,
};
use programme::service::comments::{self, PostCommentPayload};
use uuid::Uuid;

async fn dossier_dans_letat(bac: &Bac, terrain: &Terrain, slug: &str, statut: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (call_id, event_id, organization_id, submitted_by,
                title, slug, objectives, detailed_presentation, format,
                status, submitted_at)
           VALUES ($1, $2, $3, $4, '{"fr":"Atelier"}'::jsonb, $5::text::platform.slug,
                   '{"fr":"Objectifs."}'::jsonb, '{"fr":"<p>Présentation.</p>"}'::jsonb, 'hybrid',
                   $6::text::programme.proposal_status,
                   CASE WHEN $6 = 'draft' THEN NULL ELSE now() END)
        RETURNING id"#,
        terrain.appel,
        terrain.edition,
        terrain.organisation,
        terrain.deposante,
        slug,
        statut
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier")
}

fn question(topic: &str) -> OpenSupportSubjectPayload {
    OpenSupportSubjectPayload {
        topic: topic.to_owned(),
        subject: "Impossible de changer notre logo".to_owned(),
        body: "Le bouton ne réagit pas.".to_owned(),
    }
}

fn message_partage(parent_id: Option<Uuid>, visibility: &str) -> PostCommentPayload {
    PostCommentPayload {
        parent_id,
        visibility: Some(visibility.to_owned()),
        body: "Les intervenants recevront-ils un lien ?".to_owned(),
        is_change_request: false,
        recipient_ids: Vec::new(),
        topic: Some("question".to_owned()),
        topic_other: None,
        subject: Some("Lien Zoom".to_owned()),
    }
}

#[tokio::test]
async fn un_brouillon_n_ouvre_pas_l_assistance() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    dossier_dans_letat(&bac, &terrain, "brouillon-seul", "draft").await;

    let etat = assistance::etat(&bac.state, terrain.deposante)
        .await
        .expect("état");
    assert_eq!(etat.side, None);

    let refus = assistance::ouvrir(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        question("question"),
    )
    .await
    .expect_err("pas de dossier déposé");
    assert_eq!(refus.code, ErrorCode::Forbidden);
}

#[tokio::test]
async fn un_sujet_sans_dossier_va_de_l_organisation_a_l_equipe_globale_seule() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    dossier_dans_letat(&bac, &terrain, "atelier-depose", "submitted").await;

    let equipe = commun::personne(&bac, "assistance@ifdd.test", "Aïssata", "Koné").await;
    commun::attribuer(&bac, equipe, "admin", "global", None).await;
    let detachee = commun::personne(&bac, "cop31@ifdd.test", "Binta", "Sow").await;
    commun::attribuer(&bac, detachee, "admin", "event", Some(terrain.edition)).await;

    let etat = assistance::etat(&bac.state, terrain.deposante)
        .await
        .expect("état");
    assert_eq!(etat.side, Some("organization"));

    let inconnue = assistance::ouvrir(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        question("speakers"),
    )
    .await
    .expect_err("une nature de dossier n'est pas une nature d'assistance");
    assert_eq!(inconnue.field.as_deref(), Some("topic"));

    let sujet = assistance::ouvrir(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        question("technical_issue"),
    )
    .await
    .expect("ouverture");
    assert_eq!(sujet.proposal_id, None);
    assert_eq!(sujet.visibility, "submitter");

    // L'équipe n'ouvre pas de sujet sans dossier : elle répond.
    let refus = assistance::ouvrir(&bac.state, &bac.ctx(), equipe, question("question"))
        .await
        .expect_err("l'équipe répond");
    assert_eq!(refus.code, ErrorCode::Forbidden);

    let vu = assistance::fil(&bac.state, equipe)
        .await
        .expect("fil de l'équipe");
    assert_eq!(vu.side, "team");
    let racine = vu
        .messages
        .iter()
        .find(|m| m.id == sujet.id)
        .expect("le sujet remonte");
    assert!(!racine.from_team);
    assert!(racine.organization_name.is_some());
    assert_eq!(
        assistance::etat(&bac.state, equipe)
            .await
            .expect("état")
            .unread,
        1
    );

    // Détachée sur une édition : elle répond aux dossiers, pas à la plateforme.
    let vu = assistance::fil(&bac.state, detachee)
        .await
        .expect("fil détaché");
    assert!(vu.messages.iter().all(|m| m.id != sujet.id));
    let refus = assistance::repondre(
        &bac.state,
        &bac.ctx(),
        detachee,
        sujet.id,
        ReplySupportSubjectPayload {
            body: "Je regarde.".to_owned(),
        },
    )
    .await
    .expect_err("hors de sa portée");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let reponse = assistance::repondre(
        &bac.state,
        &bac.ctx(),
        equipe,
        sujet.id,
        ReplySupportSubjectPayload {
            body: "Corrigé, réessayez.".to_owned(),
        },
    )
    .await
    .expect("réponse de l'équipe");

    let vu = assistance::fil(&bac.state, terrain.deposante)
        .await
        .expect("fil de l'organisation");
    let lue = vu
        .messages
        .iter()
        .find(|m| m.id == reponse.id)
        .expect("la réponse remonte");
    assert!(lue.from_team);
    assert_eq!(
        lue.author_name, None,
        "l'équipe signe « l'équipe » côté organisation"
    );
    assert!(!lue.is_read);
    assert_eq!(vu.proposals.len(), 1);
    assert_eq!(vu.organization_ids, vec![terrain.organisation]);

    assistance::marquer_lus(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        MarkAssistanceReadPayload {
            comment_ids: vec![reponse.id, sujet.id],
        },
    )
    .await
    .expect("lecture");
    assert_eq!(
        assistance::etat(&bac.state, terrain.deposante)
            .await
            .expect("état")
            .unread,
        0
    );

    let clos = comments::clore_sujet(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        CommentId(sujet.id),
        true,
    )
    .await
    .expect("résolu par l'organisation");
    assert!(clos.closed_at.is_some());
    assistance::repondre(
        &bac.state,
        &bac.ctx(),
        equipe,
        sujet.id,
        ReplySupportSubjectPayload {
            body: "Une précision.".to_owned(),
        },
    )
    .await
    .expect("nouvelle réponse");
    let vu = assistance::fil(&bac.state, terrain.deposante)
        .await
        .expect("relu");
    let racine = vu
        .messages
        .iter()
        .find(|m| m.id == sujet.id)
        .expect("racine");
    assert!(racine.closed_at.is_none(), "une réponse rouvre le sujet");
}

#[tokio::test]
async fn les_sujets_d_un_dossier_remontent_sans_les_notes_du_comite() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = dossier_dans_letat(&bac, &terrain, "atelier-suivi", "under_review").await;
    let detachee = commun::personne(&bac, "cop31@ifdd.test", "Binta", "Sow").await;
    commun::attribuer(&bac, detachee, "admin", "event", Some(terrain.edition)).await;

    let partage = comments::ecrire(
        &bac.state,
        &bac.ctx().with_actor(detachee),
        detachee,
        ProposalId(dossier),
        message_partage(None, "submitter"),
    )
    .await
    .expect("message partagé");
    let note = comments::ecrire(
        &bac.state,
        &bac.ctx().with_actor(detachee),
        detachee,
        ProposalId(dossier),
        message_partage(None, "committee"),
    )
    .await
    .expect("note du comité");

    for personne in [terrain.deposante, detachee] {
        let vu = assistance::fil(&bac.state, personne).await.expect("fil");
        assert!(vu
            .messages
            .iter()
            .any(|m| m.id == partage.id && m.proposal_id == Some(dossier)));
        assert!(
            vu.messages.iter().all(|m| m.id != note.id),
            "la note reste au comité"
        );
    }

    // Un sujet de dossier ne se répond pas comme un sujet sans dossier.
    let refus = assistance::repondre(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        partage.id,
        ReplySupportSubjectPayload {
            body: "Merci.".to_owned(),
        },
    )
    .await
    .expect_err("route du dossier");
    assert_eq!(refus.code, ErrorCode::NotFound);
}
