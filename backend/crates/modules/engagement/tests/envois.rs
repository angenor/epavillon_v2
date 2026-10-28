//! **Écrire aux participants d'une édition, et garder ce qui est parti.**
//!
//! Éprouvé par le vrai chemin : service, fonction `mailing_audience()` du
//! modèle, travail d'envoi réservé sur sa file, garde d'envoi. Ce qui compte et
//! qu'aucune relecture ne remplace : la liste dédoublonnée sans égard à la
//! casse, l'adresse supprimée signalée et jamais sollicitée, la copie posée sur
//! chaque message, et le refus hors périmètre.

mod commun;

use commun::Bac;
use engagement::domain::mailing::{CreateMailingPayload, MailingCriterion, MailingPayload};
use engagement::service::mailings;
use kernel::ErrorCode;
use uuid::Uuid;

fn inscrits() -> MailingCriterion {
    MailingCriterion {
        kind: "registrants".into(),
        proposal_statuses: vec![],
        registration_statuses: vec![],
        role_codes: vec![],
        session_id: None,
    }
}

fn brouillon(edition: Uuid) -> CreateMailingPayload {
    CreateMailingPayload {
        event_id: edition,
        draft: MailingPayload {
            subject: Some("Votre séance à {{edition}}".into()),
            body_html: Some(
                "<p>Bonjour {{prenom}},</p><script>x()</script><p>À bientôt.</p>".into(),
            ),
            audience: Some(vec![inscrits()]),
            // La même personne qu'un inscrit, en majuscules : un seul message.
            extra_addresses: Some(vec!["INSCRIT0@example.org, libre@example.org".into()]),
            cc: None,
            bcc: None,
            reply_to: Some("equipe@ifdd.org".into()),
            ..MailingPayload::default()
        },
    }
}

async fn supprimer_ladresse(bac: &Bac, adresse: &str) {
    sqlx::query!(
        "INSERT INTO engagement.email_suppressions (email, reason)
         VALUES ($1::text::platform.email, 'hard_bounce')",
        adresse
    )
    .execute(bac.pool())
    .await
    .expect("suppression");
}

#[tokio::test]
async fn la_liste_est_dedoublonnee_et_ladresse_supprimee_signalee() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    supprimer_ladresse(&bac, "inscrit1@example.org").await;

    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &brouillon(t.edition),
    )
    .await
    .expect("brouillon");

    // La copie de l'équipe est proposée : l'adresse de qui écrit.
    assert_eq!(envoi.cc, vec!["admin@ifdd.org"]);
    assert!(!envoi.body_html.contains("script"), "{}", envoi.body_html);

    let liste = mailings::audience(&bac.state, t.administratrice, envoi.id)
        .await
        .expect("liste");
    // Trois inscrits et une adresse libre ; inscrit0 n'apparaît qu'une fois.
    assert_eq!(liste.total, 4);
    assert_eq!(liste.suppressed, 1);
    assert_eq!(liste.sendable, 3);
    assert_eq!(liste.without_first_name, 1);
    // Un message et une copie par destinataire.
    assert_eq!(liste.envelope_count, 6);
    let inscrit0 = liste
        .entries
        .iter()
        .find(|e| e.email == "inscrit0@example.org")
        .expect("inscrit0");
    assert_eq!(inscrit0.sources, vec!["addresses", "registrants"]);
}

#[tokio::test]
async fn lenvoi_part_par_la_file_avec_la_copie_sur_chaque_message() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    supprimer_ladresse(&bac, "inscrit1@example.org").await;

    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &brouillon(t.edition),
    )
    .await
    .expect("brouillon");
    let parti = mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("envoi");
    assert_eq!(parti.status, "queued");
    assert_eq!(parti.counts.queued, 3);
    assert_eq!(parti.counts.suppressed, 1);
    // Rien n'est parti dans la requête.
    assert_eq!(bac.boite.compte(), 0);

    for issue in commun::passer_le_worker(&bac).await {
        issue.expect("passage du worker");
    }

    let messages = bac.boite.messages();
    assert_eq!(messages.len(), 3);
    assert!(messages.iter().all(|m| m.to != "inscrit1@example.org"));
    assert!(messages
        .iter()
        .all(|m| m.subject.starts_with("Votre séance à COP31")));
    let karim = messages
        .iter()
        .find(|m| m.to == "inscrit0@example.org")
        .expect("inscrit0");
    assert!(karim.text.starts_with("Bonjour Karim,"), "{}", karim.text);
    for copies in bac.boite.copies() {
        assert_eq!(copies.cc, vec!["admin@ifdd.org"]);
        assert_eq!(copies.reply_to.as_deref(), Some("equipe@ifdd.org"));
    }

    let fini = mailings::detail(&bac.state, t.administratrice, envoi.id)
        .await
        .expect("détail");
    assert_eq!(fini.status, "sent");
    assert_eq!(fini.counts.sent, 3);
    assert!(fini.completed_at.is_some());

    // Parti, il ne se modifie plus.
    let refus = mailings::modifier(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        envoi.id,
        &MailingPayload::default(),
    )
    .await
    .expect_err("figé");
    assert_eq!(refus.code, ErrorCode::EngagementMailingNotDraft);

    // On le duplique.
    let copie = mailings::dupliquer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("duplication");
    assert_eq!(copie.status, "draft");
    assert_eq!(copie.duplicated_from, Some(envoi.id));
    assert_eq!(copie.subject, fini.subject);
}

#[tokio::test]
async fn hors_perimetre_lenvoi_nexiste_pas() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &brouillon(t.edition),
    )
    .await
    .expect("brouillon");

    // Administratrice d'une autre édition : ni la liste, ni l'envoi.
    let voisine = commun::personne(&bac, "voisine@ifdd.org", "Inès", "Diop").await;
    commun::attribuer(&bac, voisine, "admin", "event", Some(Uuid::now_v7())).await;
    let refus = mailings::ecran(&bac.state, voisine, t.edition)
        .await
        .expect_err("liste refusée");
    assert_eq!(refus.code, ErrorCode::NotFound);
    let refus = mailings::detail(&bac.state, voisine, envoi.id)
        .await
        .expect_err("détail refusé");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // Sans aucun rôle non plus.
    let refus = mailings::envoyer(&bac.state, &bac.ctx(), t.etrangere, envoi.id)
        .await
        .expect_err("envoi refusé");
    assert_eq!(refus.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn une_variable_inconnue_est_refusee_sur_le_corps() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let mut payload = brouillon(t.edition);
    payload.draft.body_html = Some("<p>Bonjour {{titre_activite}}</p>".into());
    // Le brouillon s'enregistre (on tape une variable avant de la finir) ;
    // l'envoi la refuse.
    let envoi = mailings::creer(&bac.state, &bac.ctx(), t.administratrice, &payload)
        .await
        .expect("brouillon");
    let refus = mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect_err("refus");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("body_html"));
}

/// Gabarit, valeurs de secours et bouton se mémorisent, se dupliquent et
/// s'appliquent : l'aperçu d'une personne sans compte reçoit le secours.
#[tokio::test]
async fn le_gabarit_et_le_secours_suivent_lenvoi() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &brouillon(t.edition),
    )
    .await
    .expect("brouillon");
    assert_eq!(envoi.template, "sobre");
    let modifie = mailings::modifier(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        envoi.id,
        &MailingPayload {
            template: Some("annonce".into()),
            variable_fallbacks: Some([("prenom".to_owned(), "Chère partenaire".to_owned())].into()),
            cta_label: Some("S'inscrire".into()),
            cta_url: Some("https://exemple.org/inscription".into()),
            reply_button: Some(true),
            ..MailingPayload::default()
        },
    )
    .await
    .expect("modification");
    assert_eq!(modifie.template, "annonce");

    let apercu = mailings::apercu(
        &bac.state,
        t.administratrice,
        envoi.id,
        Some("libre@example.org"),
    )
    .await
    .expect("aperçu");
    assert!(!apercu.has_account);
    assert!(
        apercu.html.contains("Bonjour Chère partenaire,"),
        "{}",
        apercu.html
    );
    assert!(
        apercu.html.contains(">S&#39;inscrire</a>"),
        "{}",
        apercu.html
    );
    assert!(apercu.html.contains("mailto:equipe@ifdd.org"));

    let liste = mailings::audience(&bac.state, t.administratrice, envoi.id)
        .await
        .expect("liste");
    assert_eq!(liste.without_account, 1);
    assert_eq!(liste.fallback_count, 1);

    let copie = mailings::dupliquer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("duplication");
    assert_eq!(copie.template, "annonce");
    assert_eq!(copie.cta_label.as_deref(), Some("S'inscrire"));
}

/// Sans limite d'envoi (décision du 28/10) : les lots de dix s'enchaînent sans
/// attente, et un seul passage du worker vide la file.
#[tokio::test]
async fn les_lots_senchainent_sans_attente() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let mut payload = brouillon(t.edition);
    payload.draft.audience = Some(vec![]);
    payload.draft.extra_addresses = Some((0..23).map(|i| format!("lot{i}@example.org")).collect());
    payload.draft.body_html = Some("<p>Bonjour,</p>".into());
    let envoi = mailings::creer(&bac.state, &bac.ctx(), t.administratrice, &payload)
        .await
        .expect("brouillon");
    mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("envoi");

    for issue in commun::passer_le_worker(&bac).await {
        issue.expect("passage du worker");
    }

    assert_eq!(bac.boite.compte(), 23);
    let fini = mailings::detail(&bac.state, t.administratrice, envoi.id)
        .await
        .expect("détail");
    assert_eq!(fini.status, "sent");
    assert_eq!(fini.counts.sent, 23);
}

/// Une pièce jointe posée par le module media (objet privé, rattachement
/// `attachment`) part avec chaque message ; une pièce encore en analyse
/// retient l'envoi.
#[tokio::test]
async fn la_piece_jointe_part_avec_chaque_message() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let envoi = mailings::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &brouillon(t.edition),
    )
    .await
    .expect("brouillon");

    let contenu = b"%PDF-1.4\n% programme\n".to_vec();
    let cle = format!("essais/{}.pdf", Uuid::now_v7());
    let asset = sqlx::query_scalar!(
        r#"INSERT INTO media.assets
               (bucket, object_key, checksum_sha256, mime_type, byte_size, original_filename,
                owner_person_id, visibility, status)
           VALUES ('epavillon-prive', $1, repeat('a', 64), 'application/pdf', $2,
                   'programme.pdf', $3, 'private', 'processing')
        RETURNING id"#,
        cle,
        contenu.len() as i64,
        t.administratrice
    )
    .fetch_one(bac.pool())
    .await
    .expect("objet");
    sqlx::query!(
        "INSERT INTO media.attachments (owner_schema, owner_table, owner_id, asset_id, role)
         VALUES ('engagement', 'mailings', $1, $2, 'attachment')",
        envoi.id,
        asset
    )
    .execute(bac.pool())
    .await
    .expect("rattachement");
    kernel::storage::Entrepots::new(&bac.config.media)
        .du_bucket("epavillon-prive")
        .put(&cle, "application/pdf", contenu.clone())
        .await
        .expect("dépôt sur le stockage");

    // En analyse : l'envoi attend.
    let refus = mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect_err("pièce pas prête");
    assert_eq!(refus.field.as_deref(), Some("attachments"));

    sqlx::query!(
        "UPDATE media.assets SET status = 'ready', scan_verdict = 'clean', scanned_at = now()
          WHERE id = $1",
        asset
    )
    .execute(bac.pool())
    .await
    .expect("analyse");
    let detail = mailings::detail(&bac.state, t.administratrice, envoi.id)
        .await
        .expect("détail");
    assert_eq!(detail.attachments.len(), 1);
    assert_eq!(detail.attachments_total_bytes, contenu.len() as i64);
    assert_eq!(detail.attachments_max_bytes, Some(10 * 1024 * 1024));

    mailings::envoyer(&bac.state, &bac.ctx(), t.administratrice, envoi.id)
        .await
        .expect("envoi");
    for issue in commun::passer_le_worker(&bac).await {
        issue.expect("passage du worker");
    }

    let extras = bac.boite.copies();
    // Trois inscrits et une adresse libre.
    assert_eq!(extras.len(), 4);
    for e in extras {
        assert_eq!(e.attachments.len(), 1);
        assert_eq!(e.attachments[0].filename, "programme.pdf");
        assert_eq!(e.attachments[0].content, contenu);
    }
}
