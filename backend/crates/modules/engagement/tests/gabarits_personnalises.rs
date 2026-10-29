//! **Gabarits de courriel personnalisés** (29/10) : droits, présélection pour
//! les déposants, mise en page recopiée sur l'envoi, duplication, aperçu.

mod commun;

use commun::Bac;
use engagement::domain::mail_template::MailTemplatePayload;
use engagement::domain::mailing::{CreateMailingPayload, MailingCriterion, MailingPayload};
use engagement::service::{mail_templates, mailings};
use kernel::ErrorCode;
use uuid::Uuid;

fn critere(kind: &str) -> MailingCriterion {
    MailingCriterion {
        kind: kind.into(),
        proposal_statuses: vec![],
        registration_statuses: vec![],
        role_codes: vec![],
        session_id: None,
    }
}

fn gabarit(edition: Option<Uuid>, preselect: bool) -> MailTemplatePayload {
    MailTemplatePayload {
        name: "Déposants CdP31".into(),
        layout: "texte_sur_image".into(),
        title: Some("  Pavillon de la Francophonie  ".into()),
        subtitle: Some(String::new()),
        gradient: true,
        event_id: edition,
        preselect_for_submitters: preselect,
    }
}

fn envoi(edition: Uuid, kind: &str) -> CreateMailingPayload {
    CreateMailingPayload {
        event_id: edition,
        draft: MailingPayload {
            subject: Some("Objet".into()),
            body_html: Some("<p>Bonjour {{prenom}},</p>".into()),
            audience: Some(vec![critere(kind)]),
            ..MailingPayload::default()
        },
    }
}

#[tokio::test]
async fn un_gabarit_de_ledition_est_preselectionne_pour_ses_deposants() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let admin = t.administratrice;

    let g = mail_templates::creer(
        &bac.state,
        &bac.ctx(),
        admin,
        &gabarit(Some(t.edition), true),
    )
    .await
    .expect("gabarit");
    assert_eq!(g.title.as_deref(), Some("Pavillon de la Francophonie"));
    assert_eq!(g.subtitle, None);
    assert!(g.can_edit);

    // Un seul présélectionné par édition.
    let refus = mail_templates::creer(
        &bac.state,
        &bac.ctx(),
        admin,
        &gabarit(Some(t.edition), true),
    )
    .await
    .expect_err("second présélectionné");
    assert_eq!(refus.field.as_deref(), Some("preselect_for_submitters"));
    // Présélectionner suppose un événement.
    let refus = mail_templates::creer(&bac.state, &bac.ctx(), admin, &gabarit(None, true))
        .await
        .expect_err("sans événement");
    assert_eq!(
        refus.code,
        ErrorCode::Forbidden,
        "commun : la permission globale manque d'abord"
    );

    let liste = mail_templates::lister(&bac.state, admin, t.edition)
        .await
        .expect("liste");
    assert_eq!(liste.iter().map(|g| g.id).collect::<Vec<_>>(), vec![g.id]);

    // Aux déposants : présélectionné ; aux inscrits seulement : non.
    let aux_deposants = mailings::creer(
        &bac.state,
        &bac.ctx(),
        admin,
        &envoi(t.edition, "submitters"),
    )
    .await
    .expect("envoi");
    let liste = mailings::audience(&bac.state, admin, aux_deposants.id)
        .await
        .expect("audience");
    assert_eq!(liste.suggested_mail_template_id, Some(g.id));
    let aux_inscrits = mailings::creer(
        &bac.state,
        &bac.ctx(),
        admin,
        &envoi(t.edition, "registrants"),
    )
    .await
    .expect("envoi");
    let liste = mailings::audience(&bac.state, admin, aux_inscrits.id)
        .await
        .expect("audience");
    assert_eq!(liste.suggested_mail_template_id, None);

    // Choisi, il recopie sa mise en page ; un modèle par défaut le retire.
    let choisi = mailings::modifier(
        &bac.state,
        &bac.ctx(),
        admin,
        aux_deposants.id,
        &MailingPayload {
            mail_template_id: Some(g.id.to_string()),
            ..MailingPayload::default()
        },
    )
    .await
    .expect("choix");
    assert_eq!(choisi.mail_template_id, Some(g.id));
    assert_eq!(choisi.template, "texte_sur_image");

    // L'édition de l'essai n'a pas de bannière : la mise en page retombe sur
    // l'institutionnel, symbole compris.
    let apercu = mailings::apercu(&bac.state, admin, aux_deposants.id, None)
        .await
        .expect("aperçu");
    assert!(
        apercu.html.contains("de la Francophonie"),
        "{}",
        apercu.html
    );
    assert!(apercu.html.contains("data:image/png;base64,"));

    let copie = mailings::dupliquer(&bac.state, &bac.ctx(), admin, aux_deposants.id)
        .await
        .expect("duplication");
    assert_eq!(copie.mail_template_id, Some(g.id));

    let defaut = mailings::modifier(
        &bac.state,
        &bac.ctx(),
        admin,
        aux_deposants.id,
        &MailingPayload {
            template: Some("banniere".into()),
            ..MailingPayload::default()
        },
    )
    .await
    .expect("modèle par défaut");
    assert_eq!(defaut.mail_template_id, None);
    assert_eq!(defaut.template, "banniere");

    let rendu = mail_templates::apercu(&bac.state, admin, g.id, None)
        .await
        .expect("aperçu du gabarit");
    assert!(rendu.html.contains("width=\"600\""));
}

#[tokio::test]
async fn hors_de_son_edition_un_gabarit_nexiste_pas() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let g = mail_templates::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &gabarit(Some(t.edition), false),
    )
    .await
    .expect("gabarit");

    let voisine = commun::personne(&bac, "voisine@ifdd.org", "Inès", "Diop").await;
    commun::attribuer(&bac, voisine, "admin", "event", Some(Uuid::now_v7())).await;
    let refus = mail_templates::detail(&bac.state, voisine, g.id)
        .await
        .expect_err("détail refusé");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // Un gabarit inconnu (ou d'une autre édition) ne se cite pas.
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &envoi(t.edition, "registrants"),
    )
    .await
    .expect("envoi");
    let refus = mailings::modifier(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        envoi.id,
        &MailingPayload {
            mail_template_id: Some(Uuid::now_v7().to_string()),
            ..MailingPayload::default()
        },
    )
    .await
    .expect_err("gabarit inconnu");
    assert_eq!(refus.field.as_deref(), Some("mail_template_id"));
}
