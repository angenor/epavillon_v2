//! La nature d'un échange : exigée de l'organisation quand elle ouvre un fil,
//! « Autre » avec son intitulé, jamais sur une réponse.

mod commun;

use commun::Bac;
use programme::domain::ids::ProposalId;
use programme::service::comments::{self, PostCommentPayload};
use uuid::Uuid;

fn message(
    parent_id: Option<Uuid>,
    topic: Option<&str>,
    topic_other: Option<&str>,
) -> PostCommentPayload {
    PostCommentPayload {
        parent_id,
        visibility: None,
        body: "Le lien de connexion ne s'ouvre pas.".to_owned(),
        is_change_request: false,
        recipient_ids: Vec::new(),
        topic: topic.map(str::to_owned),
        topic_other: topic_other.map(str::to_owned),
    }
}

#[tokio::test]
async fn lorganisation_nomme_la_nature_du_fil_quelle_ouvre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = ProposalId(
        commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await,
    );
    let ctx = bac.ctx();
    let ecrire = |payload| comments::ecrire(&bac.state, &ctx, terrain.deposante, dossier, payload);

    let sans = ecrire(message(None, None, None))
        .await
        .expect_err("fil sans nature");
    assert_eq!(sans.field.as_deref(), Some("topic"));

    let inconnue = ecrire(message(None, Some("meteo"), None))
        .await
        .expect_err("nature inconnue");
    assert_eq!(inconnue.field.as_deref(), Some("topic"));

    let autre_vide = ecrire(message(None, Some("other"), Some("  ")))
        .await
        .expect_err("« Autre » sans intitulé");
    assert_eq!(autre_vide.field.as_deref(), Some("topic_other"));

    let technique = ecrire(message(None, Some("technical_issue"), Some("ignoré")))
        .await
        .expect("problème technique");
    assert_eq!(technique.topic.as_deref(), Some("technical_issue"));
    assert_eq!(
        technique.topic_other, None,
        "l'intitulé libre n'accompagne que « Autre »"
    );

    let autre = ecrire(message(None, Some("other"), Some(" Visa pour Antalya ")))
        .await
        .expect("autre nature");
    assert_eq!(autre.topic.as_deref(), Some("other"));
    assert_eq!(autre.topic_other.as_deref(), Some("Visa pour Antalya"));

    let reponse = ecrire(message(Some(technique.id), Some("complaint"), None))
        .await
        .expect("une réponse");
    assert_eq!(reponse.topic, None, "la réponse hérite de la racine");
}
