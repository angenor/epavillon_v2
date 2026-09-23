//! **SC-007 — un document réservé ne sert rien de lui à qui n'a pas l'accès.**
//!
//! Chaque refus est essayé de trois façons : sans compte, avec un compte sans
//! accès, et par identifiant forgé. La liste le montre, sans son résumé, ses
//! thématiques ni, pour un lien, son adresse ; sa lecture, ses images et son
//! téléchargement rendent `403` ; la recherche le nomme sans le citer ; ses
//! notes restent à qui a l'accès ; le back-office reste fermé à qui ne publie
//! ni ne corrige. Chaque refus a son témoin : le même geste, servi.

mod commun;

use std::collections::BTreeSet;

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::StatusCode;
// `actix_web::test` s'importe par ses fonctions : le module entier masquerait `#[test]`.
use actix_web::test::{call_service, init_service, read_body, TestRequest};
use actix_web::{web, App, HttpMessage as _};
use commun::documents::{
    administratrice, creer, entree, expert, fichier_publie, lien_publie, negociatrice, objet_pdf,
    pages, passer_lextraction, rendu, PETIT,
};
use commun::Bac;
use kernel::context::RequestContext;
use kernel::error::{ErrorCode, Result};
use negotiation::domain::admin_documents::{AdminDocumentInput, CorrectionNoteInput};
use negotiation::domain::documents::LibraryDocument;
use negotiation::service::{admin_documents, corrections, documents as public};
use serde_json::Value;
use uuid::Uuid;

const RESUME_RESERVE: &str = "Synthèse confidentielle des positions de la délégation";
const RESUME_PUBLIC: &str = "Ce guide présente les points ouverts de la session";
const NOTE_RESERVEE: &str = "Le chiffre du paragraphe 12 est celui de la version de juin";
const NOTE_PUBLIQUE: &str = "Coquille au titre de la page 1";
const RECHERCHE: &str = "negociations reprennent";
const CHEMIN_RESERVE: &str = "/sb62/positions-confidentielles-du-groupe";
const URL_RESERVEE: &str = "https://enb.iisd.org/sb62/positions-confidentielles-du-groupe";
const URL_PUBLIQUE: &str = "https://unfccc.int/sb62/ordre-du-jour";

struct Scene {
    ifdd: Uuid,
    expert: Uuid,
    negociatrice: Uuid,
    visiteuse: Uuid,
    public: Uuid,
    reserve: Uuid,
}

impl Scene {
    /// Les regards qui ne doivent rien voir : sans compte, et un compte sans
    /// accès.
    fn sans_acces(&self) -> [(&'static str, Option<Uuid>); 2] {
        [
            ("sans compte", None),
            ("compte sans accès", Some(self.visiteuse)),
        ]
    }
}

async fn poser_resume_et_thematiques(
    bac: &Bac,
    admin: Uuid,
    id: Uuid,
    resume: &str,
    themes: &[&str],
) {
    let entree: AdminDocumentInput = serde_json::from_value(serde_json::json!({
        "summary": { "fr": resume },
        "themes": themes,
    }))
    .expect("entrée partielle");
    admin_documents::modifier(&bac.state, &bac.ctx(admin), id, &entree, "fr")
        .await
        .expect("résumé et thématiques posés");
}

/// Un guide public et un résumé réservé, tous deux des fichiers publiés et
/// extraits du même petit PDF, chacun avec son résumé et ses thématiques.
async fn scene(bac: &Bac) -> Scene {
    let ifdd = administratrice(bac, "ifdd@example.org").await;
    let expert = expert(bac, "expert@example.org").await;
    let negociatrice = negociatrice(bac, "negociatrice@example.org").await;
    let visiteuse = commun::personne(bac, "visiteuse@example.org").await;

    let public = fichier_publie(bac, ifdd, "Guide des négociations", false).await;
    let reserve = fichier_publie(bac, ifdd, "Résumé pour les décideurs", true).await;
    poser_resume_et_thematiques(bac, ifdd, public, RESUME_PUBLIC, &["mitigation"]).await;
    poser_resume_et_thematiques(
        bac,
        ifdd,
        reserve,
        RESUME_RESERVE,
        &["adaptation", "finance"],
    )
    .await;

    // Sans cette vérification, un refus pourrait tenir à un profil mal posé.
    assert!(!public::a_lacces(&bac.state, Some(visiteuse)).await.unwrap());
    assert!(public::a_lacces(&bac.state, Some(negociatrice))
        .await
        .unwrap());

    Scene {
        ifdd,
        expert,
        negociatrice,
        visiteuse,
        public,
        reserve,
    }
}

/// Le code du refus ; `ImageDePage` n'est pas `Debug`, d'où le `match`.
fn refus<T>(issue: Result<T>, quoi: &str) -> ErrorCode {
    match issue {
        Ok(_) => panic!("{quoi} : refus attendu, la réponse a été servie"),
        Err(e) => e.code,
    }
}

fn dans(documents: &[LibraryDocument], id: Uuid) -> &LibraryDocument {
    documents
        .iter()
        .find(|d| d.id == id)
        .expect("le document figure dans la bibliothèque")
}

async fn telechargements(bac: &Bac, id: Uuid) -> i32 {
    // Requête sans macro : elle n'ajoute rien au cache hors ligne de SQLx.
    sqlx::query_scalar::<_, i32>("SELECT download_count FROM negotiation.documents WHERE id = $1")
        .bind(id)
        .fetch_one(bac.pool())
        .await
        .expect("compteur")
}

async fn poser_une_note(bac: &Bac, auteur: Uuid, id: Uuid, texte: &str) {
    corrections::poser(
        &bac.state,
        &bac.ctx(auteur),
        id,
        &CorrectionNoteInput {
            page_index: 1,
            passage: None,
            body: serde_json::json!({ "fr": texte }),
        },
    )
    .await
    .expect("note posée");
}

/// Un lien réservé en brouillon : sa publication ne demande pas d'extraction.
async fn lien_en_brouillon(bac: &Bac, admin: Uuid) -> Uuid {
    let mut e = entree("Bulletin réservé en préparation", true);
    e.external_url = Some(Some(URL_RESERVEE.to_owned()));
    admin_documents::creer(&bac.state, &bac.ctx(admin), &e, "fr")
        .await
        .expect("création du lien en brouillon")
}

#[tokio::test]
async fn la_bibliotheque_montre_le_reserve_sans_son_resume_ni_ses_thematiques() {
    let bac = Bac::monter().await;
    let s = scene(&bac).await;

    let mut empreintes = Vec::new();
    for (qui, personne) in s.sans_acces() {
        let (bibliotheque, empreinte) = public::bibliotheque(&bac.state, personne, "fr")
            .await
            .expect("bibliothèque");
        empreintes.push(empreinte);

        let r = dans(&bibliotheque.documents, s.reserve);
        assert_eq!(
            r.title, "Résumé pour les décideurs",
            "{qui} : le titre reste"
        );
        assert!(r.restricted && !r.accessible, "{qui}");
        assert_eq!(r.summary, None, "{qui} : le résumé est masqué");
        assert!(r.themes.is_empty(), "{qui} : les thématiques sont masquées");
        assert!(r.themes_hidden, "{qui} : le masquage se dit");
        assert_eq!(r.source, "file");
        assert_eq!(r.page_count, Some(4), "{qui} : le nombre de pages reste");
        assert!(r.reading_bytes.is_some(), "{qui} : la taille reste");

        let p = dans(&bibliotheque.documents, s.public);
        assert!(!p.restricted && p.accessible);
        assert_eq!(p.summary.as_deref(), Some(RESUME_PUBLIC));
        assert_eq!(p.themes, ["mitigation"]);
        assert!(!p.themes_hidden);

        let themes: Vec<&str> = bibliotheque
            .vocabulary
            .themes
            .iter()
            .map(|t| t.code.as_str())
            .collect();
        assert_eq!(
            themes,
            ["mitigation"],
            "{qui} : le vocabulaire ne trahit pas les thématiques masquées"
        );

        let servi = serde_json::to_string(&bibliotheque).unwrap();
        for secret in [RESUME_RESERVE, "adaptation", "finance"] {
            assert!(!servi.contains(secret), "{qui} : « {secret} » a fuité");
        }
    }
    assert_eq!(empreintes[0], empreintes[1], "même regard, même empreinte");

    let (bibliotheque, empreinte) = public::bibliotheque(&bac.state, Some(s.negociatrice), "fr")
        .await
        .expect("bibliothèque");
    let r = dans(&bibliotheque.documents, s.reserve);
    assert!(r.restricted && r.accessible);
    assert_eq!(r.summary.as_deref(), Some(RESUME_RESERVE));
    assert_eq!(
        r.themes.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(["adaptation", "finance"])
    );
    assert!(!r.themes_hidden);
    let themes: BTreeSet<&str> = bibliotheque
        .vocabulary
        .themes
        .iter()
        .map(|t| t.code.as_str())
        .collect();
    assert_eq!(
        themes,
        BTreeSet::from(["adaptation", "finance", "mitigation"])
    );
    assert_ne!(
        empreinte, empreintes[0],
        "la copie masquée ne passe pas pour la copie entière"
    );
}

#[tokio::test]
async fn lecture_image_et_telechargement_dun_reserve_rendent_403_sans_acces() {
    let bac = Bac::monter().await;
    let s = scene(&bac).await;
    let avant = telechargements(&bac, s.reserve).await;

    for (qui, personne) in s.sans_acces() {
        assert_eq!(
            refus(public::lecture(&bac.state, personne, s.reserve).await, qui),
            ErrorCode::NegotiationDocumentRestricted,
            "{qui} : lecture"
        );
        assert_eq!(
            refus(public::image(&bac.state, personne, s.reserve, 3).await, qui),
            ErrorCode::NegotiationDocumentRestricted,
            "{qui} : image"
        );
        assert_eq!(
            refus(
                public::compter_un_telechargement(&bac.state, personne, s.reserve).await,
                qui
            ),
            ErrorCode::NegotiationDocumentRestricted,
            "{qui} : téléchargement"
        );

        // Le public reste ouvert au même regard : le refus tient au document.
        let (lecture, _, reserve) = public::lecture(&bac.state, personne, s.public)
            .await
            .expect("le public se lit");
        assert_eq!(lecture.page_count, 4);
        assert!(
            !reserve,
            "{qui} : le public se sert `Cache-Control: public`"
        );
        let image = public::image(&bac.state, personne, s.public, 3)
            .await
            .expect("l'image du public se sert");
        assert!(image.octets.starts_with(&[0xFF, 0xD8]));
        assert!(
            !image.reservee,
            "{qui} : l'image du public se sert `public`"
        );
    }
    assert_eq!(
        telechargements(&bac, s.reserve).await,
        avant,
        "un téléchargement refusé ne se compte pas"
    );

    let (lecture, _, reserve) = public::lecture(&bac.state, Some(s.negociatrice), s.reserve)
        .await
        .expect("avec l'accès, la lecture s'ouvre");
    assert_eq!(lecture.page_count, 4);
    assert!(reserve, "la route en tire `Cache-Control: private`");
    let image = public::image(&bac.state, Some(s.negociatrice), s.reserve, 3)
        .await
        .expect("avec l'accès, l'image se sert");
    assert!(image.octets.starts_with(&[0xFF, 0xD8]));
    assert!(image.reservee, "la route en tire `Cache-Control: private`");
    public::compter_un_telechargement(&bac.state, Some(s.negociatrice), s.reserve)
        .await
        .expect("avec l'accès, le téléchargement se compte");
    assert_eq!(telechargements(&bac, s.reserve).await, avant + 1);
}

#[tokio::test]
async fn un_identifiant_forge_nouvre_rien() {
    let bac = Bac::monter().await;
    let s = scene(&bac).await;

    // Un brouillon réservé, extrait : ses images existent dans le bucket, et la
    // route publique ne doit pas les servir, même avec l'accès.
    let brouillon = creer(&bac, s.ifdd, "Note réservée en préparation", true).await;
    let asset = objet_pdf(&bac, s.ifdd, PETIT, "ready").await;
    admin_documents::attacher_le_fichier(&bac.state, &bac.ctx(s.ifdd), brouillon, asset)
        .await
        .expect("fichier attaché");
    let issues = passer_lextraction(&bac).await;
    assert_eq!(issues.len(), 1, "une seule extraction, celle du brouillon");
    assert!(issues[0].is_ok(), "{issues:?}");
    // Sans ces preuves, le 404 viendrait d'un rendu absent, pas du filtre de
    // publication.
    let etat = rendu(&bac, brouillon).await;
    assert_eq!(etat.status, "ready");
    assert_eq!(etat.page_count, Some(4));
    let lignes = pages(&bac, brouillon).await;
    assert_eq!(lignes.len(), 4);
    assert!(
        lignes.iter().all(|(_, _, cle, _)| cle.is_some()),
        "chaque page a son image : {lignes:?}"
    );
    let image = public::image_de_lapercu(&bac.state, brouillon, 3)
        .await
        .expect("l'aperçu du back-office sert l'image du brouillon");
    assert!(
        image.octets.starts_with(&[0xFF, 0xD8]),
        "l'objet est dans le bucket"
    );

    let inexistant = Uuid::now_v7();
    let tous = [
        ("sans compte", None),
        ("compte sans accès", Some(s.visiteuse)),
        ("négociatrice", Some(s.negociatrice)),
    ];
    for (qui, personne) in tous {
        for (quel, id) in [("inexistant", inexistant), ("brouillon", brouillon)] {
            let contexte = format!("{qui}, identifiant {quel}");
            assert_eq!(
                refus(public::lecture(&bac.state, personne, id).await, &contexte),
                ErrorCode::NegotiationDocumentNotFound,
                "{contexte} : lecture"
            );
            assert_eq!(
                refus(public::image(&bac.state, personne, id, 3).await, &contexte),
                ErrorCode::NegotiationDocumentNotFound,
                "{contexte} : image"
            );
            assert_eq!(
                refus(
                    public::compter_un_telechargement(&bac.state, personne, id).await,
                    &contexte
                ),
                ErrorCode::NegotiationDocumentNotFound,
                "{contexte} : téléchargement"
            );
        }
    }

    // L'identifiant réel, tapé à la main par qui n'a pas l'accès : le refus
    // passe avant l'index de page, qui ne dit donc rien du document.
    for (qui, personne) in s.sans_acces() {
        for index in [1, 3, 99] {
            assert_eq!(
                refus(
                    public::image(&bac.state, personne, s.reserve, index).await,
                    qui
                ),
                ErrorCode::NegotiationDocumentRestricted,
                "{qui} : image {index}"
            );
        }
    }

    // Témoin : une fois publié, le même brouillon s'ouvre à la négociatrice —
    // le 404 tenait donc à la seule publication.
    admin_documents::publier(&bac.state, &bac.ctx(s.ifdd), brouillon)
        .await
        .expect("publication du brouillon");
    let (lecture, _, reserve) = public::lecture(&bac.state, Some(s.negociatrice), brouillon)
        .await
        .expect("publié, il se lit");
    assert_eq!(
        (lecture.id, lecture.page_count, reserve),
        (brouillon, 4, true)
    );
    public::image(&bac.state, Some(s.negociatrice), brouillon, 3)
        .await
        .expect("publié, son image se sert");
    assert_eq!(
        refus(
            public::lecture(&bac.state, Some(s.visiteuse), brouillon).await,
            "publié, sans accès"
        ),
        ErrorCode::NegotiationDocumentRestricted
    );
}

#[tokio::test]
async fn la_recherche_nomme_le_reserve_sans_le_citer() {
    let bac = Bac::monter().await;
    let s = scene(&bac).await;

    for (qui, personne) in s.sans_acces() {
        let trouve = public::rechercher(&bac.state, personne, RECHERCHE)
            .await
            .expect("recherche");
        let r = trouve
            .hits
            .iter()
            .find(|h| h.document_id == s.reserve)
            .unwrap_or_else(|| panic!("{qui} : le réservé est nommé"));
        assert!(r.pages.is_empty(), "{qui} : ni page ni extrait");
        let p = trouve
            .hits
            .iter()
            .find(|h| h.document_id == s.public)
            .expect("le public est trouvé");
        assert!(!p.pages.is_empty(), "{qui} : le public se cite");
        assert!(!p.pages[0].excerpt.is_empty());
    }

    let trouve = public::rechercher(&bac.state, Some(s.negociatrice), RECHERCHE)
        .await
        .expect("recherche");
    let r = trouve
        .hits
        .iter()
        .find(|h| h.document_id == s.reserve)
        .expect("le réservé est trouvé");
    assert!(!r.pages.is_empty(), "avec l'accès, le réservé se cite");
    assert_eq!(r.pages[0].index, 1);
    assert!(!r.pages[0].excerpt.is_empty());
}

#[tokio::test]
async fn les_notes_dun_reserve_ne_vont_qua_qui_a_lacces() {
    let bac = Bac::monter().await;
    let s = scene(&bac).await;

    for (id, texte) in [(s.reserve, NOTE_RESERVEE), (s.public, NOTE_PUBLIQUE)] {
        poser_une_note(&bac, s.expert, id, texte).await;
    }

    let mut empreintes = Vec::new();
    for (qui, personne) in s.sans_acces() {
        let (notes, empreinte) = public::notes(&bac.state, personne, "fr")
            .await
            .expect("notes");
        empreintes.push(empreinte);
        let documents: Vec<Uuid> = notes.notes.iter().map(|n| n.document_id).collect();
        assert_eq!(documents, [s.public], "{qui} : seule la note du public");
        let servi = serde_json::to_string(&notes).unwrap();
        assert!(!servi.contains(NOTE_RESERVEE), "{qui} : la note a fuité");
    }

    let (notes, empreinte) = public::notes(&bac.state, Some(s.negociatrice), "fr")
        .await
        .expect("notes");
    let documents: BTreeSet<Uuid> = notes.notes.iter().map(|n| n.document_id).collect();
    assert_eq!(documents, BTreeSet::from([s.public, s.reserve]));
    let note = notes
        .notes
        .iter()
        .find(|n| n.document_id == s.reserve)
        .unwrap();
    assert_eq!(note.body, NOTE_RESERVEE);
    assert_ne!(empreinte, empreintes[0]);
}

#[tokio::test]
async fn un_favori_sur_un_reserve_sans_acces_est_permis() {
    let bac = Bac::monter().await;
    let s = scene(&bac).await;
    let ctx = bac.ctx(s.visiteuse);

    public::poser_un_favori(&bac.state, &ctx, s.visiteuse, s.reserve)
        .await
        .expect("le favori ne dévoile rien que la liste ne montre déjà");
    let (favoris, _) = public::favoris(&bac.state, s.visiteuse)
        .await
        .expect("favoris");
    assert_eq!(
        favoris
            .bookmarks
            .iter()
            .map(|b| b.document_id)
            .collect::<Vec<_>>(),
        [s.reserve]
    );
    assert_eq!(
        refus(
            public::poser_un_favori(&bac.state, &ctx, s.visiteuse, Uuid::now_v7()).await,
            "favori forgé"
        ),
        ErrorCode::NegotiationDocumentNotFound
    );

    // L'identifiant réel d'un brouillon : « non publié » vaut « inconnu », même
    // pour qui a l'accès.
    let brouillon = lien_en_brouillon(&bac, s.ifdd).await;
    for (qui, personne) in [
        ("compte sans accès", s.visiteuse),
        ("négociatrice", s.negociatrice),
    ] {
        assert_eq!(
            refus(
                public::poser_un_favori(&bac.state, &bac.ctx(personne), personne, brouillon).await,
                qui
            ),
            ErrorCode::NegotiationDocumentNotFound,
            "{qui} : favori sur un brouillon"
        );
    }
    let (favoris, _) = public::favoris(&bac.state, s.negociatrice)
        .await
        .expect("favoris");
    assert!(favoris.bookmarks.is_empty(), "le refus n'a rien posé");

    // Témoin : publié, le même identifiant se met en favori ; dépublié, plus.
    admin_documents::publier(&bac.state, &bac.ctx(s.ifdd), brouillon)
        .await
        .expect("publication du lien");
    public::poser_un_favori(&bac.state, &ctx, s.visiteuse, brouillon)
        .await
        .expect("publié, le favori se pose");
    let (favoris, _) = public::favoris(&bac.state, s.visiteuse)
        .await
        .expect("favoris");
    assert_eq!(
        favoris
            .bookmarks
            .iter()
            .map(|b| b.document_id)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([s.reserve, brouillon])
    );
    admin_documents::depublier(&bac.state, &bac.ctx(s.ifdd), brouillon)
        .await
        .expect("dépublication");
    assert_eq!(
        refus(
            public::poser_un_favori(
                &bac.state,
                &bac.ctx(s.negociatrice),
                s.negociatrice,
                brouillon
            )
            .await,
            "favori sur un dépublié"
        ),
        ErrorCode::NegotiationDocumentNotFound
    );

    // Le favori n'ouvre pas le document.
    assert_eq!(
        refus(
            public::lecture(&bac.state, Some(s.visiteuse), s.reserve).await,
            "lecture après favori"
        ),
        ErrorCode::NegotiationDocumentRestricted
    );
}

#[tokio::test]
async fn un_reserve_sans_thematique_se_dit_masque_comme_les_autres() {
    let bac = Bac::monter().await;
    let s = scene(&bac).await;
    let fichier_nu = fichier_publie(&bac, s.ifdd, "Note sans thématique", true).await;
    let lien_nu = lien_publie(&bac, s.ifdd, "Bulletin des positions", URL_RESERVEE, true).await;
    let temoin = lien_publie(&bac, s.ifdd, "Ordre du jour", URL_PUBLIQUE, false).await;

    for (qui, personne) in s.sans_acces() {
        let (bibliotheque, _) = public::bibliotheque(&bac.state, personne, "fr")
            .await
            .expect("bibliothèque");
        let avec_themes = dans(&bibliotheque.documents, s.reserve);
        for id in [fichier_nu, lien_nu] {
            let r = dans(&bibliotheque.documents, id);
            assert!(r.restricted && !r.accessible, "{qui}");
            assert!(r.themes.is_empty(), "{qui}");
            assert!(
                r.themes_hidden,
                "{qui} : un réservé se dit masqué, qu'il porte ou non des thématiques"
            );
            assert_eq!(
                (&r.themes, r.themes_hidden),
                (&avec_themes.themes, avec_themes.themes_hidden),
                "{qui} : rien ne distingue un réservé sans thématique d'un autre"
            );
        }
        let t = dans(&bibliotheque.documents, temoin);
        assert!(
            t.themes.is_empty() && !t.themes_hidden,
            "{qui} : un public sans thématique n'a rien de masqué"
        );
    }

    let (bibliotheque, _) = public::bibliotheque(&bac.state, Some(s.negociatrice), "fr")
        .await
        .expect("bibliothèque");
    for id in [fichier_nu, lien_nu] {
        let r = dans(&bibliotheque.documents, id);
        assert!(r.accessible && r.themes.is_empty());
        assert!(!r.themes_hidden, "avec l'accès, rien n'est masqué");
    }
}

#[tokio::test]
async fn un_lien_reserve_ne_livre_son_adresse_qua_qui_a_lacces() {
    let bac = Bac::monter().await;
    let s = scene(&bac).await;
    let lien = lien_publie(&bac, s.ifdd, "Bulletin des positions", URL_RESERVEE, true).await;
    let temoin = lien_publie(&bac, s.ifdd, "Ordre du jour", URL_PUBLIQUE, false).await;
    let avant = telechargements(&bac, lien).await;

    for (qui, personne) in s.sans_acces() {
        let (bibliotheque, _) = public::bibliotheque(&bac.state, personne, "fr")
            .await
            .expect("bibliothèque");
        let r = dans(&bibliotheque.documents, lien);
        assert_eq!(r.source, "link");
        assert!(r.restricted && !r.accessible, "{qui}");
        assert_eq!(
            r.external_url, None,
            "{qui} : l'adresse d'un lien est son contenu"
        );
        assert_eq!(
            r.link_host.as_deref(),
            Some("enb.iisd.org"),
            "{qui} : l'hôte reste"
        );
        let t = dans(&bibliotheque.documents, temoin);
        assert_eq!(t.external_url.as_deref(), Some(URL_PUBLIQUE), "{qui}");
        assert_eq!(t.link_host.as_deref(), Some("unfccc.int"), "{qui}");
        let servi = serde_json::to_string(&bibliotheque).unwrap();
        assert!(!servi.contains(CHEMIN_RESERVE), "{qui} : l'adresse a fuité");

        // Le refus du réservé passe avant « un lien ne se lit pas » : le 409
        // dirait déjà que c'est un lien.
        assert_eq!(
            refus(public::lecture(&bac.state, personne, lien).await, qui),
            ErrorCode::NegotiationDocumentRestricted,
            "{qui} : lecture"
        );
        assert_eq!(
            refus(public::image(&bac.state, personne, lien, 1).await, qui),
            ErrorCode::NegotiationDocumentRestricted,
            "{qui} : image"
        );
        assert_eq!(
            refus(
                public::compter_un_telechargement(&bac.state, personne, lien).await,
                qui
            ),
            ErrorCode::NegotiationDocumentRestricted,
            "{qui} : téléchargement"
        );
        assert_eq!(
            refus(public::lecture(&bac.state, personne, temoin).await, qui),
            ErrorCode::NegotiationDocumentNotReadable,
            "{qui} : le lien public, lui, répond 409"
        );
    }
    assert_eq!(
        telechargements(&bac, lien).await,
        avant,
        "un téléchargement refusé ne se compte pas"
    );

    let (bibliotheque, _) = public::bibliotheque(&bac.state, Some(s.negociatrice), "fr")
        .await
        .expect("bibliothèque");
    let r = dans(&bibliotheque.documents, lien);
    assert!(r.accessible);
    assert_eq!(
        r.external_url.as_deref(),
        Some(URL_RESERVEE),
        "avec l'accès, l'adresse se sert"
    );
    assert_eq!(r.link_host.as_deref(), Some("enb.iisd.org"));
    assert_eq!(
        refus(
            public::lecture(&bac.state, Some(s.negociatrice), lien).await,
            "lien réservé, avec l'accès"
        ),
        ErrorCode::NegotiationDocumentNotReadable
    );
    public::compter_un_telechargement(&bac.state, Some(s.negociatrice), lien)
        .await
        .expect("avec l'accès, l'ouverture du lien se compte");
    assert_eq!(telechargements(&bac, lien).await, avant + 1);
}

/// L'en-tête qui tient lieu de session dans l'application d'essai.
const ACTEUR: &str = "x-essai-acteur";

fn lire(uri: &str, acteur: Option<Uuid>) -> TestRequest {
    let r = TestRequest::get().uri(uri);
    match acteur {
        Some(a) => r.insert_header((ACTEUR, a.to_string())),
        None => r,
    }
}

async fn frapper<S, R, B>(app: &S, requete: R) -> (StatusCode, Vec<u8>)
where
    S: Service<R, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let reponse = call_service(app, requete).await;
    let statut = reponse.status();
    (statut, read_body(reponse).await.to_vec())
}

fn code(corps: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(corps)
        .ok()
        .and_then(|v| v["code"].as_str().map(str::to_owned))
}

/// SC-007 dit « aucune API » : l'aperçu, le PDF, les images et les notes du
/// back-office servent le contenu d'un réservé sans garde propre ; seule
/// `exiger_la_lecture`, posée par la route, les ferme.
#[tokio::test]
async fn les_lectures_du_back_office_restent_fermees_a_qui_ne_publie_ni_ne_corrige() {
    let bac = Bac::monter().await;
    let s = scene(&bac).await;
    poser_une_note(&bac, s.expert, s.reserve, NOTE_RESERVEE).await;

    for (qui, personne) in [
        ("compte sans accès", s.visiteuse),
        ("négociatrice", s.negociatrice),
    ] {
        let refus = admin_documents::exiger_la_lecture(&bac.state, personne)
            .await
            .err()
            .unwrap_or_else(|| panic!("{qui} : refus attendu"));
        assert_eq!(refus.code, ErrorCode::Forbidden, "{qui}");
    }
    let d = admin_documents::exiger_la_lecture(&bac.state, s.expert)
        .await
        .expect("l'expert lit ce qu'il corrige");
    assert!(d.corriger && !d.publier);
    let d = admin_documents::exiger_la_lecture(&bac.state, s.ifdd)
        .await
        .expect("l'administratrice lit");
    assert!(d.publier);

    let app = init_service(
        App::new()
            .app_data(web::Data::new(bac.db()))
            .app_data(web::Data::new(bac.state.clone()))
            .wrap_fn(|req, srv| {
                let acteur = req
                    .headers()
                    .get(ACTEUR)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| Uuid::parse_str(v).ok());
                let ctx = RequestContext::new(RequestContext::generated_request_id(), "fr");
                req.extensions_mut().insert(match acteur {
                    Some(a) => ctx.with_actor(a),
                    None => ctx,
                });
                srv.call(req)
            })
            .configure(negotiation::admin_routes),
    )
    .await;

    let base = format!("/admin/negotiation/documents/{}", s.reserve);
    let apercu = format!("{base}/preview");
    let pdf = format!("{base}/file");
    let image = format!("{base}/pages/3/image");
    let notes = format!("{base}/corrections");

    for uri in [&apercu, &pdf, &image, &notes] {
        let (statut, corps) = frapper(&app, lire(uri, None).to_request()).await;
        assert_eq!(
            (statut, code(&corps).as_deref()),
            (StatusCode::UNAUTHORIZED, Some("UNAUTHENTICATED")),
            "{uri} sans compte"
        );
        for (qui, personne) in [
            ("compte sans accès", s.visiteuse),
            ("négociatrice", s.negociatrice),
        ] {
            let (statut, corps) = frapper(&app, lire(uri, Some(personne)).to_request()).await;
            assert_eq!(
                (statut, code(&corps).as_deref()),
                (StatusCode::FORBIDDEN, Some("FORBIDDEN")),
                "{uri} : {qui}"
            );
        }
    }

    // Témoins : qui corrige et qui publie reçoivent le contenu même.
    for (qui, personne) in [("expert", s.expert), ("administratrice", s.ifdd)] {
        let (statut, corps) = frapper(&app, lire(&apercu, Some(personne)).to_request()).await;
        assert_eq!(statut, StatusCode::OK, "{qui} : aperçu");
        let v: Value = serde_json::from_slice(&corps).expect("aperçu en JSON");
        assert_eq!(v["id"], serde_json::json!(s.reserve));
        let pages = v["pages"].as_array().expect("pages de l'aperçu");
        assert_eq!(pages.len(), 4, "{qui}");
        assert!(
            pages
                .iter()
                .any(|p| p["blocks"].as_array().is_some_and(|b| !b.is_empty())),
            "{qui} : l'aperçu porte le texte"
        );

        let (statut, corps) = frapper(&app, lire(&pdf, Some(personne)).to_request()).await;
        assert_eq!(statut, StatusCode::OK, "{qui} : PDF");
        assert_eq!(corps, PETIT, "{qui} : le PDF d'origine, octet pour octet");

        let (statut, corps) = frapper(&app, lire(&image, Some(personne)).to_request()).await;
        assert_eq!(statut, StatusCode::OK, "{qui} : image");
        assert!(corps.starts_with(&[0xFF, 0xD8]), "{qui} : un JPEG");

        let (statut, corps) = frapper(&app, lire(&notes, Some(personne)).to_request()).await;
        assert_eq!(statut, StatusCode::OK, "{qui} : notes");
        let v: Value = serde_json::from_slice(&corps).expect("notes en JSON");
        let textes: Vec<&str> = v["notes"]
            .as_array()
            .expect("liste des notes")
            .iter()
            .filter_map(|n| n["body"]["fr"].as_str())
            .collect();
        assert_eq!(textes, [NOTE_RESERVEE], "{qui}");
    }
}
