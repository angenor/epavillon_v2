//! **Les textes types** (01/11) : semés, rangés, gardés comme les gabarits.

mod commun;

use commun::Bac;
use engagement::domain::mail_text::MailTextPayload;
use engagement::service::mail_texts;
use kernel::ErrorCode;
use uuid::Uuid;

fn texte(event_id: Option<Uuid>) -> MailTextPayload {
    MailTextPayload {
        name: "  Rappel du formulaire  ".into(),
        category: Some(" ".into()),
        subject: "Votre formulaire pour {{edition}}".into(),
        body_html: "<p>Bonjour {{prenom}},</p><script>x()</script>".into(),
        sort_order: 5,
        event_id,
    }
}

#[tokio::test]
async fn les_neuf_textes_de_la_v1_sont_semes_communs_et_ranges() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let liste = mail_texts::lister(&bac.state, t.administratrice, t.edition)
        .await
        .expect("liste");
    assert_eq!(liste.len(), 9);
    assert!(liste.iter().all(|x| x.event_id.is_none()));
    // Communs : l'administratrice d'une édition les choisit sans les modifier.
    assert!(liste.iter().all(|x| !x.can_edit));
    assert_eq!(liste[0].category.as_deref(), Some("Journées spéciales"));
    // Aucune date limite ni « CdP31 » en dur.
    for x in &liste {
        assert!(!x.subject.contains("CdP31"), "{}", x.subject);
        assert!(!x.body_html.contains("2026"), "{}", x.name);
    }
}

#[tokio::test]
async fn un_texte_de_ledition_se_cree_assaini_et_un_commun_demande_la_permission_globale() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;

    let cree = mail_texts::creer(
        &bac.state,
        &bac.ctx(),
        t.administratrice,
        &texte(Some(t.edition)),
    )
    .await
    .expect("création");
    assert_eq!(cree.name, "Rappel du formulaire");
    assert_eq!(cree.category, None);
    assert!(!cree.body_html.contains("script"), "{}", cree.body_html);
    assert!(cree.can_edit);

    let refus = mail_texts::creer(&bac.state, &bac.ctx(), t.administratrice, &texte(None))
        .await
        .expect_err("commun sans la permission globale");
    assert_eq!(refus.code, ErrorCode::Forbidden);

    let redactrice = commun::redactrice(&bac).await;
    mail_texts::creer(&bac.state, &bac.ctx(), redactrice, &texte(None))
        .await
        .expect("commun avec la permission globale");

    // Hors périmètre, le texte de l'édition n'existe pas.
    let voisine = commun::personne(&bac, "voisine@ifdd.org", "Inès", "Diop").await;
    commun::attribuer(&bac, voisine, "admin", "event", Some(Uuid::now_v7())).await;
    let refus = mail_texts::detail(&bac.state, voisine, cree.id)
        .await
        .expect_err("hors périmètre");
    assert_eq!(refus.code, ErrorCode::NotFound);

    mail_texts::supprimer(&bac.state, &bac.ctx(), t.administratrice, cree.id)
        .await
        .expect("suppression");
}
