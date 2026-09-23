//! **Les notes de correction** : l'expert lit ce qu'il corrige, pose une note
//! et la retire, sans jamais publier ni modifier ; l'administratrice publie et
//! ne corrige pas. Une note retirée reste en base, datée, et quitte la lecture
//! publique.
//!
//! PDFium ne se lie qu'une fois par processus, et chaque extracteur du décor le
//! relie : une seule extraction tient donc dans ce binaire de test. Tout ce qui
//! demande des pages extraites vit dans le parcours de l'expert ; le reste
//! travaille sur un lien, ou sur un brouillon dont les pages sont posées en SQL.

mod commun;

use commun::documents::{
    administratrice, brouillon, expert, fichier_publie, lien_publie, negociatrice,
};
use commun::{attribuer, personne, traces, Bac};
use kernel::auth::{has_permission, Scope};
use kernel::error::ErrorCode;
use negotiation::domain::admin_documents::{AdminCorrectionNote, CorrectionNoteInput, PersonLink};
use negotiation::domain::documents::CorrectionNote;
use negotiation::domain::permissions::{CORRECTION_POST, CORRECTION_WITHDRAW, DOCUMENT_PUBLISH};
use negotiation::service::{admin_documents, corrections, documents as public};
use serde_json::{json, Value};
use time::OffsetDateTime;
use uuid::Uuid;

async fn peut(bac: &Bac, personne: Uuid, permission: &str) -> bool {
    has_permission(bac.pool(), personne, permission, Scope::Global)
        .await
        .expect("lecture de la permission")
}

/// Un texte qui diffère d'une langue à l'autre : un repli sur le français se
/// verrait.
fn corps() -> Value {
    json!({ "fr": "Ce chiffre date de la COP29.", "en": "Outdated figure." })
}

fn note_sur(page_index: i32, passage: Option<&str>, body: Value) -> CorrectionNoteInput {
    CorrectionNoteInput {
        page_index,
        passage: passage.map(str::to_owned),
        body,
    }
}

fn note(page_index: i32, body: Value) -> CorrectionNoteInput {
    note_sur(page_index, Some("  les négociations reprennent  "), body)
}

async fn poser(bac: &Bac, auteur: Uuid, document: Uuid, page: i32) -> AdminCorrectionNote {
    corrections::poser(&bac.state, &bac.ctx(auteur), document, &note(page, corps()))
        .await
        .expect("note posée")
}

fn qui(p: &Option<PersonLink>) -> Option<(Uuid, &str)> {
    p.as_ref().map(|p| (p.id, p.name.as_str()))
}

struct LigneDeNote {
    passage: Option<String>,
    body: Value,
    created_at: OffsetDateTime,
    withdrawn_at: Option<OffsetDateTime>,
    withdrawn_by: Option<Uuid>,
}

/// La note telle qu'elle est en base, hors de tout service.
async fn en_base(bac: &Bac, id: Uuid) -> Option<LigneDeNote> {
    sqlx::query!(
        r#"SELECT passage, body AS "body!: Value", created_at, withdrawn_at, withdrawn_by
             FROM negotiation.correction_notes WHERE id = $1"#,
        id
    )
    .fetch_optional(bac.pool())
    .await
    .expect("lecture de la note")
    .map(|l| LigneDeNote {
        passage: l.passage,
        body: l.body,
        created_at: l.created_at,
        withdrawn_at: l.withdrawn_at,
        withdrawn_by: l.withdrawn_by,
    })
}

async fn nombre_de_notes(bac: &Bac, document: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM negotiation.correction_notes WHERE document_id = $1"#,
        document
    )
    .fetch_one(bac.pool())
    .await
    .expect("compte des notes")
}

async fn note_publique(bac: &Bac, langue: &str, id: Uuid) -> Option<CorrectionNote> {
    let (liste, _) = public::notes(&bac.state, None, langue)
        .await
        .expect("notes publiques");
    liste.notes.into_iter().find(|n| n.id == id)
}

/// L'horloge de la base : c'est elle qui date la pose et le retrait.
async fn horloge(bac: &Bac) -> OffsetDateTime {
    sqlx::query_scalar!(r#"SELECT clock_timestamp() AS "t!""#)
        .fetch_one(bac.pool())
        .await
        .expect("horloge de la base")
}

/// `commun::personne` nomme tout le monde « Awa Diallo » : un retireur renommé
/// ne se confond plus avec l'auteur.
async fn renommer(bac: &Bac, id: Uuid, prenom: &str, nom: &str) {
    sqlx::query!(
        "UPDATE identity.people SET first_name = $2, last_name = $3 WHERE id = $1",
        id,
        prenom,
        nom
    )
    .execute(bac.pool())
    .await
    .expect("renommage");
}

/// Des pages posées à la main, sans extraction : de quoi ancrer une note.
async fn pages_a_la_main(bac: &Bac, document: Uuid, nombre: i32) {
    sqlx::query!(
        "INSERT INTO negotiation.document_pages (document_id, page_index, label)
         SELECT $1, i, i::text FROM generate_series(1, $2::int) AS i",
        document,
        nombre
    )
    .execute(bac.pool())
    .await
    .expect("pages posées à la main");
}

#[tokio::test]
async fn lexpert_lit_pose_et_retire_une_note_sans_pouvoir_publier_ni_modifier() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let relecteur = expert(&bac, "expert@example.org").await;
    let second = expert(&bac, "second.expert@example.org").await;
    renommer(&bac, second, "Moussa", "Traoré").await;

    // Ce qu'il peut, et ce qu'il ne peut pas.
    let droits = admin_documents::exiger_la_lecture(&bac.state, relecteur)
        .await
        .expect("l'expert lit le back-office des documents");
    assert!(droits.corriger);
    assert!(!droits.publier, "l'expert ne publie pas");
    assert!(
        !peut(&bac, relecteur, DOCUMENT_PUBLISH).await,
        "ni publier ni modifier : les deux passent par negotiation.document.publish"
    );
    assert!(peut(&bac, relecteur, CORRECTION_POST).await);
    assert!(peut(&bac, relecteur, CORRECTION_WITHDRAW).await);

    // Il lit la liste, la fiche et l'aperçu.
    let liste = admin_documents::liste(&bac.state, &droits, "fr")
        .await
        .expect("liste");
    assert!(liste.documents.iter().any(|d| d.id == guide));
    assert!(!liste.can_publish && liste.can_correct);
    let fiche = admin_documents::fiche(&bac.state, &droits, guide, "fr")
        .await
        .expect("fiche");
    assert!(!fiche.can_publish && fiche.can_correct);
    let apercu = admin_documents::apercu(&bac.state, guide)
        .await
        .expect("aperçu");
    assert_eq!(
        apercu.pages.len(),
        4,
        "l'expert voit chaque page du document"
    );
    let page = apercu.pages[0].index;
    let derniere = apercu.pages.iter().map(|p| p.index).max().unwrap_or(0);

    // Une page hors du document : refusée, rien n'est écrit.
    let refus = corrections::poser(
        &bac.state,
        &bac.ctx(relecteur),
        guide,
        &note(derniere + 1, json!({ "fr": "Hors du document." })),
    )
    .await
    .unwrap_err();
    assert_eq!(refus.code, ErrorCode::NegotiationCorrectionPageUnknown);
    assert_eq!(refus.field.as_deref(), Some("page_index"));

    // Sans texte français, sur une page qui existe : le refus porte sur body.
    for body in [
        json!({ "en": "English only." }),
        json!({ "fr": "   ", "en": "Blank French." }),
        json!({ "fr": 42 }),
        json!("Une chaîne nue"),
    ] {
        let refus = corrections::poser(
            &bac.state,
            &bac.ctx(relecteur),
            guide,
            &note(page, body.clone()),
        )
        .await
        .unwrap_err();
        assert_eq!(refus.code, ErrorCode::ValidationFailed, "{body}");
        assert_eq!(refus.field.as_deref(), Some("body"), "{body}");
    }
    assert_eq!(nombre_de_notes(&bac, guide).await, 0, "aucun refus n'écrit");

    // Il pose une note : elle revient telle que posée, à son nom, datée par la base.
    let avant = horloge(&bac).await;
    let posee = poser(&bac, relecteur, guide, page).await;
    let apres = horloge(&bac).await;
    assert_eq!(posee.document_id, guide);
    assert_eq!(posee.page_index, page);
    assert_eq!(
        posee.body,
        corps(),
        "le français et l'anglais tels que posés"
    );
    assert_eq!(
        posee.passage.as_deref(),
        Some("les négociations reprennent")
    );
    assert_eq!(posee.author.id, relecteur);
    assert_eq!(posee.author.name, "Awa Diallo");
    assert!(
        avant <= posee.posted_at && posee.posted_at <= apres,
        "{avant} ≤ {} ≤ {apres}",
        posee.posted_at
    );
    assert!(posee.withdrawn_at.is_none() && posee.withdrawn_by.is_none());
    let ligne = en_base(&bac, posee.id).await.expect("note en base");
    assert_eq!(ligne.created_at, posee.posted_at);
    assert_eq!(ligne.body, corps());
    assert_eq!(
        ligne.passage.as_deref(),
        Some("les négociations reprennent")
    );

    // La lecture publique la montre, dans la langue demandée.
    for (langue, texte) in [
        ("fr", "Ce chiffre date de la COP29."),
        ("en", "Outdated figure."),
    ] {
        let publique = note_publique(&bac, langue, posee.id)
            .await
            .expect("la lecture publique montre la note");
        assert_eq!(publique.body, texte, "{langue}");
        assert_eq!(publique.document_id, guide);
        assert_eq!(publique.page_index, page);
        assert_eq!(
            publique.passage.as_deref(),
            Some("les négociations reprennent")
        );
        assert_eq!(publique.author_name, "Awa Diallo");
        assert_eq!(publique.posted_at, posee.posted_at);
    }

    // Un autre expert la retire : elle reste en base, datée, et quitte la
    // lecture publique. Le délai sépare la date du retrait de celle de la pose.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let une_fois = corrections::retirer(&bac.state, &bac.ctx(second), posee.id)
        .await
        .expect("retrait");
    let retrait = une_fois.withdrawn_at.expect("un retrait se date");
    assert!(retrait > posee.posted_at, "{retrait} > {}", posee.posted_at);
    assert_eq!(qui(&une_fois.withdrawn_by), Some((second, "Moussa Traoré")));
    assert_eq!(
        une_fois.author.id, relecteur,
        "l'auteur reste celui de la pose"
    );
    assert_eq!(
        une_fois.posted_at, posee.posted_at,
        "le retrait ne redate pas la pose"
    );
    assert_eq!(une_fois.body, corps());
    let ligne = en_base(&bac, posee.id)
        .await
        .expect("une note retirée n'est jamais supprimée");
    assert_eq!(ligne.withdrawn_at, Some(retrait));
    assert_eq!(ligne.withdrawn_by, Some(second));
    assert_eq!(ligne.created_at, posee.posted_at);
    for langue in ["fr", "en"] {
        assert!(
            note_publique(&bac, langue, posee.id).await.is_none(),
            "une note retirée quitte la lecture publique ({langue})"
        );
    }

    // Un second retrait, par l'auteur et plus tard, ne change rien.
    // Le délai garantit qu'un second now() différerait, s'il était écrit.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let deux_fois = corrections::retirer(&bac.state, &bac.ctx(relecteur), posee.id)
        .await
        .expect("un second retrait n'est pas une erreur");
    assert_eq!(deux_fois.withdrawn_at, Some(retrait));
    assert_eq!(
        qui(&deux_fois.withdrawn_by),
        Some((second, "Moussa Traoré")),
        "le premier retrait fait foi"
    );
    assert_eq!(deux_fois.posted_at, posee.posted_at);
    let ligne = en_base(&bac, posee.id).await.expect("note en base");
    assert_eq!(ligne.withdrawn_at, Some(retrait));
    assert_eq!(ligne.withdrawn_by, Some(second));
    assert_eq!(ligne.created_at, posee.posted_at);

    // Le back-office garde la note retirée : son texte, ses auteurs, ses dates.
    let back_office = corrections::du_document(&bac.state, guide, true, true)
        .await
        .expect("notes du document");
    assert_eq!(back_office.notes.len(), 1);
    let vue = &back_office.notes[0];
    assert_eq!(vue.id, posee.id);
    assert_eq!(vue.document_id, guide);
    assert_eq!(vue.page_index, page);
    assert_eq!(vue.body, corps());
    assert_eq!(vue.passage.as_deref(), Some("les négociations reprennent"));
    assert_eq!(
        (vue.author.id, vue.author.name.as_str()),
        (relecteur, "Awa Diallo")
    );
    assert_eq!(
        vue.posted_at, posee.posted_at,
        "la pose garde sa première date"
    );
    assert_eq!(
        vue.withdrawn_at,
        Some(retrait),
        "le retrait garde sa première date"
    );
    assert_eq!(qui(&vue.withdrawn_by), Some((second, "Moussa Traoré")));

    // L'audit porte l'auteur de la pose et celui du retrait, et rien d'autre.
    let sur_la_note = traces(&bac, "correction_notes", posee.id).await;
    let poses: Vec<_> = sur_la_note.iter().filter(|(a, _)| a == "insert").collect();
    let retraits: Vec<_> = sur_la_note.iter().filter(|(a, _)| a == "update").collect();
    assert_eq!(
        poses.iter().map(|(_, acteur)| *acteur).collect::<Vec<_>>(),
        vec![Some(relecteur)],
        "la pose porte son auteur : {sur_la_note:?}"
    );
    assert_eq!(
        retraits
            .iter()
            .map(|(_, acteur)| *acteur)
            .collect::<Vec<_>>(),
        vec![Some(second)],
        "le retrait porte qui l'a décidé ; le second ne change rien, donc ne trace rien : \
         {sur_la_note:?}"
    );
    assert!(
        sur_la_note.iter().all(|(action, _)| action != "delete"),
        "une note ne se supprime jamais : {sur_la_note:?}"
    );
}

#[tokio::test]
async fn un_passage_vide_ou_blanc_vaut_pour_toute_la_page() {
    let bac = Bac::monter().await;
    let relecteur = expert(&bac, "expert@example.org").await;
    let doc = brouillon(&bac, "passage-vide").await;
    pages_a_la_main(&bac, doc, 2).await;
    let francais_seul = json!({ "fr": "Toute la page est dépassée." });

    for passage in [None, Some(""), Some("   "), Some(" \t\n ")] {
        let posee = corrections::poser(
            &bac.state,
            &bac.ctx(relecteur),
            doc,
            &note_sur(2, passage, francais_seul.clone()),
        )
        .await
        .expect("note posée");
        assert_eq!(posee.passage, None, "{passage:?}");
        assert_eq!(
            posee.body, francais_seul,
            "sans anglais posé, aucun n'est inventé"
        );
        let ligne = en_base(&bac, posee.id).await.expect("note en base");
        assert_eq!(ligne.passage, None, "{passage:?} : NULL en base");
    }

    let citee = corrections::poser(
        &bac.state,
        &bac.ctx(relecteur),
        doc,
        &note_sur(
            2,
            Some("\n  pertes et préjudices \t"),
            francais_seul.clone(),
        ),
    )
    .await
    .expect("note citant un passage");
    assert_eq!(citee.passage.as_deref(), Some("pertes et préjudices"));
    assert_eq!(
        en_base(&bac, citee.id)
            .await
            .expect("note en base")
            .passage
            .as_deref(),
        Some("pertes et préjudices")
    );
}

#[tokio::test]
async fn ladministratrice_publie_mais_ne_pose_ni_ne_retire_de_note() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let enb = lien_publie(
        &bac,
        ifdd,
        "Bulletin des négociations",
        "https://enb.iisd.org/cop30",
        false,
    )
    .await;

    let droits = admin_documents::exiger_la_lecture(&bac.state, ifdd)
        .await
        .expect("publier suffit à lire le back-office");
    assert!(droits.publier);
    assert!(!droits.corriger, "corriger le fond relève de l'expert");
    assert!(peut(&bac, ifdd, DOCUMENT_PUBLISH).await);
    assert!(!peut(&bac, ifdd, CORRECTION_POST).await);
    assert!(!peut(&bac, ifdd, CORRECTION_WITHDRAW).await);

    let liste = admin_documents::liste(&bac.state, &droits, "fr")
        .await
        .expect("elle lit la liste");
    let ligne = liste
        .documents
        .iter()
        .find(|d| d.id == enb)
        .expect("son lien est listé");
    assert_eq!(ligne.state, "published");
    assert!(liste.can_publish && !liste.can_correct);

    // Elle lit les notes des experts, vivantes et retirées, avec leurs auteurs.
    // can_post et can_withdraw se calculent dans la route : éprouvés en HTTP.
    let relecteur = expert(&bac, "expert@example.org").await;
    let doc = brouillon(&bac, "notes-lues-par-ladmin").await;
    pages_a_la_main(&bac, doc, 2).await;
    let vivante = poser(&bac, relecteur, doc, 1).await;
    let retiree = poser(&bac, relecteur, doc, 2).await;
    corrections::retirer(&bac.state, &bac.ctx(relecteur), retiree.id)
        .await
        .expect("retrait");

    let notes = corrections::du_document(&bac.state, doc, droits.corriger, false)
        .await
        .expect("elle lit les notes");
    assert_eq!(notes.notes.len(), 2, "vivantes et retirées");
    let lue = |id: Uuid| {
        notes
            .notes
            .iter()
            .find(|n| n.id == id)
            .expect("la note est listée")
    };
    let v = lue(vivante.id);
    assert_eq!((v.page_index, v.author.id), (1, relecteur));
    assert_eq!(v.body, corps());
    assert!(v.withdrawn_at.is_none() && v.withdrawn_by.is_none());
    let r = lue(retiree.id);
    assert_eq!((r.page_index, r.author.id), (2, relecteur));
    assert!(r.withdrawn_at.is_some());
    assert_eq!(r.withdrawn_by.as_ref().map(|p| p.id), Some(relecteur));
}

#[tokio::test]
async fn le_super_administrateur_detient_les_deux_permissions_de_correction() {
    let bac = Bac::monter().await;
    let racine = personne(&bac, "racine@example.org").await;
    attribuer(&bac, racine, "super_admin", "global", None).await;
    let sans_role = personne(&bac, "sans.role@example.org").await;

    for permission in [CORRECTION_POST, CORRECTION_WITHDRAW, DOCUMENT_PUBLISH] {
        assert!(peut(&bac, racine, permission).await, "{permission}");
        assert!(!peut(&bac, sans_role, permission).await, "{permission}");
    }
    let droits = admin_documents::exiger_la_lecture(&bac.state, racine)
        .await
        .expect("le super administrateur lit le back-office");
    assert!(droits.publier && droits.corriger);
}

#[tokio::test]
async fn une_negociatrice_ne_lit_pas_le_back_office_des_documents() {
    let bac = Bac::monter().await;
    let awa = negociatrice(&bac, "awa@example.org").await;

    let refus = admin_documents::exiger_la_lecture(&bac.state, awa)
        .await
        .err()
        .expect("ni publier ni corriger : refusée");
    assert_eq!(refus.code, ErrorCode::Forbidden);
}

#[tokio::test]
async fn un_document_sans_forme_lisible_na_aucune_page_ou_poser_une_note() {
    let bac = Bac::monter().await;
    let relecteur = expert(&bac, "expert@example.org").await;
    let sans_pages = brouillon(&bac, "brouillon-sans-extraction").await;

    let refus = corrections::poser(
        &bac.state,
        &bac.ctx(relecteur),
        sans_pages,
        &note(1, json!({ "fr": "Rien n'est encore extrait." })),
    )
    .await
    .unwrap_err();
    assert_eq!(refus.code, ErrorCode::NegotiationCorrectionPageUnknown);
    assert_eq!(refus.field.as_deref(), Some("page_index"));
    assert_eq!(nombre_de_notes(&bac, sans_pages).await, 0);

    // Ses pages posées, la même note passe.
    pages_a_la_main(&bac, sans_pages, 1).await;
    let posee = corrections::poser(
        &bac.state,
        &bac.ctx(relecteur),
        sans_pages,
        &note(1, json!({ "fr": "Rien n'est encore extrait." })),
    )
    .await
    .expect("la page existe désormais");
    assert_eq!(posee.page_index, 1);
    assert_eq!(nombre_de_notes(&bac, sans_pages).await, 1);
}

#[tokio::test]
async fn une_note_sur_un_document_inconnu_est_introuvable() {
    let bac = Bac::monter().await;
    let relecteur = expert(&bac, "expert@example.org").await;
    let inconnu = Uuid::now_v7();

    let refus = corrections::poser(
        &bac.state,
        &bac.ctx(relecteur),
        inconnu,
        &note(1, json!({ "fr": "Aucun document." })),
    )
    .await
    .unwrap_err();
    assert_eq!(refus.code, ErrorCode::NegotiationDocumentNotFound);

    let refus = corrections::du_document(&bac.state, inconnu, true, true)
        .await
        .expect_err("aucune liste pour un document inconnu");
    assert_eq!(refus.code, ErrorCode::NegotiationDocumentNotFound);
}

#[tokio::test]
async fn retirer_une_note_inconnue_est_introuvable() {
    let bac = Bac::monter().await;
    let relecteur = expert(&bac, "expert@example.org").await;

    let refus = corrections::retirer(&bac.state, &bac.ctx(relecteur), Uuid::now_v7())
        .await
        .unwrap_err();
    assert_eq!(refus.code, ErrorCode::NotFound);
}
