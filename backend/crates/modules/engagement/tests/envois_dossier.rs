//! **Écrire au déposant d'un dossier, avec ses valeurs** (01/11).
//!
//! Le brouillon rattaché au dossier, les variables lues sur sa séance retenue
//! et figées à l'envoi, leur refus quand elles n'ont pas de valeur, la liste
//! des courriels du dossier et le refus hors périmètre.

mod commun;

use commun::Bac;
use engagement::domain::mailing::{
    CreateMailingPayload, CreateProposalMailingPayload, MailingPayload,
};
use engagement::service::{mail_texts, mailing_dossier, mailings};
use kernel::ErrorCode;
use uuid::Uuid;

async fn dossier_de(bac: &Bac, seance: Uuid) -> Uuid {
    sqlx::query_scalar!(
        r#"SELECT proposal_id AS "id!" FROM programme.sessions WHERE id = $1"#,
        seance
    )
    .fetch_one(bac.pool())
    .await
    .expect("dossier de la séance")
}

async fn donner_un_contact(bac: &Bac, dossier: Uuid) -> Uuid {
    let contact = commun::personne(bac, "contact@example.org", "Moussa", "Traoré").await;
    sqlx::query!(
        "UPDATE programme.proposals SET contact_person_id = $2 WHERE id = $1",
        dossier,
        contact
    )
    .execute(bac.pool())
    .await
    .expect("contact du dossier");
    contact
}

async fn donner_une_salle(bac: &Bac, edition: Uuid, seance: Uuid) {
    sqlx::query!(
        r#"WITH lieu AS (
               INSERT INTO event.venues (event_id, name)
               VALUES ($1, '{"fr":"Pavillon de la Francophonie"}'::jsonb) RETURNING id
           ), salle AS (
               INSERT INTO event.rooms (venue_id, name, code)
               SELECT id, '{"fr":"Salle Niger","en":"Niger room"}'::jsonb, 'niger' FROM lieu
               RETURNING id
           )
           UPDATE programme.sessions SET room_id = (SELECT id FROM salle) WHERE id = $2"#,
        edition,
        seance
    )
    .execute(bac.pool())
    .await
    .expect("salle de la séance");
}

fn pour(dossier: Uuid) -> CreateProposalMailingPayload {
    CreateProposalMailingPayload {
        proposal_id: dossier,
    }
}

#[tokio::test]
async fn le_brouillon_du_dossier_ecrit_au_deposant_avec_le_contact_en_copie() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let dossier = dossier_de(&bac, t.seance).await;
    donner_un_contact(&bac, dossier).await;

    let envoi = mailing_dossier::creer(&bac.state, &bac.ctx(), t.administratrice, &pour(dossier))
        .await
        .expect("brouillon du dossier");

    assert_eq!(envoi.status, "draft");
    assert_eq!(envoi.event_id, t.edition);
    assert_eq!(envoi.extra_addresses, vec!["animatrice@example.org"]);
    assert!(envoi.audience.is_empty());
    assert_eq!(envoi.cc, vec!["admin@ifdd.org", "contact@example.org"]);
    assert_eq!(envoi.subject, "Financer l'adaptation");

    let d = envoi.proposal.expect("dossier rattaché");
    assert_eq!(d.id, dossier);
    assert_eq!(d.values["activite"], "Financer l'adaptation");
    assert!(
        d.values["heures"].ends_with(", heure de Belém"),
        "{}",
        d.values["heures"]
    );
    assert!(d.values["date"].ends_with("2027") || d.values["date"].ends_with("2026"));
    // Sans salle, seule {{salle}} manque, et l'écran sait dire pourquoi.
    assert!(!d.values.contains_key("salle"));
    assert!(d.missing.expect("raison").contains("pas de salle"));
}

#[tokio::test]
async fn les_valeurs_du_dossier_se_voient_dans_lapercu_et_partent_figees() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let dossier = dossier_de(&bac, t.seance).await;
    donner_une_salle(&bac, t.edition, t.seance).await;

    let envoi = mailing_dossier::creer(&bac.state, &bac.ctx(), t.administratrice, &pour(dossier))
        .await
        .expect("brouillon du dossier");

    // Le texte type semé « Liste d'attente » se choisit, et sa trace reste.
    let texte = mail_texts::lister(&bac.state, t.administratrice, t.edition)
        .await
        .expect("textes types")
        .into_iter()
        .find(|x| x.name == "Liste d'attente")
        .expect("texte semé");
    let modifie = mailings::modifier(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        envoi.id,
        &MailingPayload {
            subject: Some("{{activite}}".into()),
            body_html: Some(
                "<p>{{prenom}} : {{activite}}, {{date}}, {{heures}}, {{salle}}.</p>".into(),
            ),
            mail_text_id: Some(texte.id.to_string()),
            ..MailingPayload::default()
        },
    )
    .await
    .expect("modification");
    assert_eq!(modifie.mail_text_id, Some(texte.id));

    let apercu = mailings::apercu(&bac.state, t.administratrice, envoi.id, None)
        .await
        .expect("aperçu");
    assert_eq!(apercu.subject, "Financer l'adaptation");
    assert!(
        apercu.html.contains("Financer l&#39;adaptation</span>"),
        "{}",
        apercu.html
    );
    assert!(
        apercu.html.contains("Salle Niger</span>"),
        "{}",
        apercu.html
    );

    mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("envoi");
    for issue in commun::passer_le_worker(&bac).await {
        issue.expect("passage du worker");
    }
    let messages = bac.boite.messages();
    assert_eq!(messages.len(), 1);
    let m = &messages[0];
    assert_eq!(m.to, "animatrice@example.org");
    assert!(
        m.text.starts_with("Awa : Financer l'adaptation, "),
        "{}",
        m.text
    );
    assert!(
        m.text.contains(", heure de Belém, Salle Niger."),
        "{}",
        m.text
    );
    // Le surlignage est celui de l'aperçu, jamais du message.
    assert!(!m
        .html
        .as_deref()
        .unwrap_or_default()
        .contains("background-color:#e0f3fb"));

    // La liste du dossier le montre, avec son destinataire et ses copies.
    let ecran = mailing_dossier::ecran(&bac.state, t.administratrice, dossier, None)
        .await
        .expect("liste du dossier");
    assert_eq!(ecran.rows.len(), 1);
    let ligne = &ecran.rows[0];
    assert_eq!(ligne.to.as_deref(), Some("animatrice@example.org"));
    assert_eq!(ligne.cc_count, 1);
    assert_eq!(ligne.mail_text_name.as_deref(), Some("Liste d'attente"));
    assert_eq!(ecran.timezone, "America/Belem");
    assert_eq!(ecran.city.as_deref(), Some("Belém"));
}

#[tokio::test]
async fn sans_seance_lapercu_et_lenvoi_le_disent() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let dossier = dossier_de(&bac, t.seance).await;
    sqlx::query!(
        "UPDATE programme.sessions SET status = 'cancelled' WHERE id = $1",
        t.seance
    )
    .execute(bac.pool())
    .await
    .expect("séance annulée");

    let envoi = mailing_dossier::creer(&bac.state, &bac.ctx(), t.administratrice, &pour(dossier))
        .await
        .expect("brouillon du dossier");
    let corps = MailingPayload {
        body_html: Some("<p>Votre créneau : {{date}}.</p>".into()),
        ..MailingPayload::default()
    };
    mailings::modifier(&bac.state, &bac.ctx(), t.administratrice, envoi.id, &corps)
        .await
        .expect("le brouillon s'enregistre");

    let refus = mailings::apercu(&bac.state, t.administratrice, envoi.id, None)
        .await
        .expect_err("aperçu refusé");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert!(
        refus.message.contains("pas encore de séance"),
        "{}",
        refus.message
    );
    let refus = mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect_err("envoi refusé");
    assert!(
        refus.message.contains("pas encore de séance"),
        "{}",
        refus.message
    );
    assert_eq!(bac.boite.compte(), 0);

    // {{activite}} seule n'attend pas de séance : la liste d'attente part.
    let corps = MailingPayload {
        body_html: Some("<p>« {{activite}} » est sur liste d'attente.</p>".into()),
        ..MailingPayload::default()
    };
    mailings::modifier(&bac.state, &bac.ctx(), t.administratrice, envoi.id, &corps)
        .await
        .expect("modification");
    mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("envoi sans séance");
}

#[tokio::test]
async fn un_envoi_groupe_refuse_les_variables_du_dossier() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &CreateMailingPayload {
            event_id: t.edition,
            draft: MailingPayload {
                subject: Some("Votre activité".into()),
                body_html: Some("<p>{{prenom}}, votre créneau : {{heures}}.</p>".into()),
                extra_addresses: Some(vec!["libre@example.org".into()]),
                ..MailingPayload::default()
            },
        },
    )
    .await
    .expect("brouillon");
    assert!(envoi.proposal.is_none());
    let refus = mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect_err("refus");
    assert_eq!(refus.field.as_deref(), Some("body_html"));
    assert!(
        refus.message.starts_with("{{heures}} ne servent"),
        "{}",
        refus.message
    );
}

#[tokio::test]
async fn hors_perimetre_le_dossier_nexiste_pas() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let dossier = dossier_de(&bac, t.seance).await;

    let voisine = commun::personne(&bac, "voisine@ifdd.org", "Inès", "Diop").await;
    commun::attribuer(&bac, voisine, "admin", "event", Some(Uuid::now_v7())).await;
    let refus = mailing_dossier::creer(&bac.state, &bac.ctx(), voisine, &pour(dossier))
        .await
        .expect_err("brouillon refusé");
    assert_eq!(refus.code, ErrorCode::NotFound);
    let refus = mailing_dossier::ecran(&bac.state, voisine, dossier, None)
        .await
        .expect_err("liste refusée");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // Un dossier nommé avec une autre édition que la sienne n'y est pas.
    let refus =
        mailing_dossier::ecran(&bac.state, t.administratrice, dossier, Some(Uuid::now_v7()))
            .await
            .expect_err("édition étrangère");
    assert_eq!(refus.code, ErrorCode::NotFound);
}
