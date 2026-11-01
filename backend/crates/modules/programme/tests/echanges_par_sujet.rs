//! **Les échanges d'un dossier sont des sujets** (01/11) : un titre et une
//! nature à l'ouverture, un état résolu que les deux côtés posent, et une
//! réponse qui rouvre le sujet.

mod commun;

use commun::Bac;
use kernel::error::ErrorCode;
use programme::domain::ids::{CommentId, ProposalId};
use programme::service::comments::{self, PostCommentPayload};
use uuid::Uuid;

fn charge(parent_id: Option<Uuid>, subject: Option<&str>) -> PostCommentPayload {
    PostCommentPayload {
        parent_id,
        visibility: None,
        body: "Mme Diallo ne pourra pas venir.".to_owned(),
        is_change_request: false,
        recipient_ids: Vec::new(),
        topic: Some("speakers".to_owned()),
        topic_other: None,
        subject: subject.map(str::to_owned),
    }
}

#[tokio::test]
async fn un_sujet_s_ouvre_avec_son_titre_se_resout_et_se_rouvre_par_une_reponse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = ProposalId(
        commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await,
    );
    let ctx = bac.ctx().with_actor(terrain.deposante);

    let sans_titre = comments::ecrire(
        &bac.state,
        &ctx,
        terrain.deposante,
        dossier,
        charge(None, None),
    )
    .await
    .expect_err("l'organisation titre son sujet");
    assert_eq!(sans_titre.code, ErrorCode::ValidationFailed);
    assert_eq!(sans_titre.field.as_deref(), Some("subject"));

    let sujet = comments::ecrire(
        &bac.state,
        &ctx,
        terrain.deposante,
        dossier,
        charge(None, Some("  Remplacer une intervenante  ")),
    )
    .await
    .expect("ouverture du sujet");
    assert_eq!(sujet.subject.as_deref(), Some("Remplacer une intervenante"));

    // La réponse ne porte ni titre ni nature : elle hérite du sujet.
    let reponse = comments::ecrire(
        &bac.state,
        &ctx,
        terrain.deposante,
        dossier,
        charge(Some(sujet.id), Some("Ignoré")),
    )
    .await
    .expect("réponse");
    assert_eq!(reponse.subject, None);
    assert_eq!(reponse.topic, None);

    let refus = comments::clore_sujet(
        &bac.state,
        &ctx,
        terrain.deposante,
        CommentId(reponse.id),
        true,
    )
    .await
    .expect_err("une réponse n'ouvre pas de sujet");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);

    let clos = comments::clore_sujet(
        &bac.state,
        &ctx,
        terrain.deposante,
        CommentId(sujet.id),
        true,
    )
    .await
    .expect("sujet résolu par l'organisation");
    assert!(clos.closed_at.is_some());
    assert_eq!(clos.closed_by, Some(terrain.deposante));
    assert!(
        clos.resolved_at.is_none(),
        "rien à voir avec une demande de correction"
    );

    comments::ecrire(
        &bac.state,
        &ctx,
        terrain.deposante,
        dossier,
        charge(Some(reponse.id), None),
    )
    .await
    .expect("nouvelle réponse, sur une réponse");
    let rouvert = sqlx::query!(
        "SELECT closed_at, closed_by FROM programme.proposal_comments WHERE id = $1",
        sujet.id
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture");
    assert!(
        rouvert.closed_at.is_none(),
        "écrire dans un sujet résolu le rouvre"
    );
    assert!(rouvert.closed_by.is_none());

    comments::clore_sujet(
        &bac.state,
        &ctx,
        terrain.deposante,
        CommentId(sujet.id),
        true,
    )
    .await
    .expect("de nouveau résolu");
    let rouvert = comments::clore_sujet(
        &bac.state,
        &ctx,
        terrain.deposante,
        CommentId(sujet.id),
        false,
    )
    .await
    .expect("rouvert à la main");
    assert!(rouvert.closed_at.is_none());
}
