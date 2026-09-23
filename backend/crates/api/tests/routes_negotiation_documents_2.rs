//! Les documents de Guide Négo à travers HTTP, suite de
//! `routes_negotiation_documents.rs` : le réservé vu par qui a l'accès et par
//! qui ne l'a pas, les favoris de deux comptes, le remplacement, le dépublié,
//! et ce que la liste des notes du back-office dit des droits de la personne.
//!
//! Les aides sont recopiées du premier fichier : un fichier de test d'intégration
//! est un crate à lui seul.

use actix_web::http::header::{ACCEPT_LANGUAGE, CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::test;
use api::state::AppState;
use kernel::context::RequestContext;
use kernel::crypto::Passwords;
use kernel::jobs::{self, JobHandler};
use kernel::storage::Entrepots;
use kernel::testing::TestDb;
use negotiation::domain::admin_documents::CorrectionNoteInput;
use negotiation::jobs::extract::ExtractDocument;
use negotiation::service::{admin_documents as admin, corrections};
use serde_json::{json, Value};
use uuid::Uuid;

const MOT_DE_PASSE: &str = "Belem2027!";
const PETIT: &[u8] = include_bytes!("../../modules/negotiation/tests/fixtures/petit.pdf");

const ADMIN: &str = "ifdd@example.org";
const EXPERT: &str = "expert@example.org";
const NEGOCIATRICE: &str = "awa.diallo@example.org";
const SANS_ACCES: &str = "lecteur@example.org";

const NOTE_PUBLIQUE: &str = "Coquille au titre de la page 1";
const NOTE_RESERVEE: &str = "Le chiffre du paragraphe 12 est celui de la version de juin";

const BIBLIOTHEQUE: &str = "/api/negotiation/documents";
const NOTES: &str = "/api/negotiation/documents/corrections";
const FAVORIS: &str = "/api/negotiation/me/bookmarks";

struct Bac {
    base: TestDb,
    etat: AppState,
    admin: Uuid,
    expert: Uuid,
}

impl Bac {
    async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = kernel::testing::test_config(base.url());
        let etat = AppState::new(base.db(), config)
            .await
            .expect("état de l'application");
        let mut bac = Self {
            base,
            etat,
            admin: Uuid::nil(),
            expert: Uuid::nil(),
        };
        bac.admin = compte(&bac, ADMIN, Some("admin")).await;
        bac.expert = compte(&bac, EXPERT, Some("expert")).await;
        compte(&bac, NEGOCIATRICE, Some("negotiator")).await;
        compte(&bac, SANS_ACCES, None).await;
        bac
    }

    fn ctx(&self, acteur: Uuid) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr").with_actor(acteur)
    }

    fn ctx_anonyme(&self) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr")
    }

    async fn telechargements(&self, document: Uuid) -> i32 {
        sqlx::query_scalar("SELECT download_count FROM negotiation.documents WHERE id = $1")
            .bind(document)
            .fetch_one(self.base.pool())
            .await
            .expect("compteur de téléchargements")
    }
}

async fn compte(bac: &Bac, email: &str, role: Option<&str>) -> Uuid {
    let empreinte = Passwords::new()
        .expect("Argon2id")
        .hash(MOT_DE_PASSE)
        .expect("empreinte");
    let person_id: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, 'Awa', 'Diallo', now())
         RETURNING id",
    )
    .bind(email)
    .fetch_one(bac.base.pool())
    .await
    .expect("insertion de la personne");
    sqlx::query(
        "INSERT INTO identity.accounts (person_id, provider, password_hash, password_changed_at)
         VALUES ($1, 'password', $2, now())",
    )
    .bind(person_id)
    .bind(&empreinte)
    .execute(bac.base.pool())
    .await
    .expect("insertion du compte");
    if let Some(role) = role {
        sqlx::query(
            "INSERT INTO identity.role_assignments (person_id, role_code, scope_type)
             VALUES ($1, $2, 'global')",
        )
        .bind(person_id)
        .bind(role)
        .execute(bac.base.pool())
        .await
        .expect("attribution du rôle");
    }
    person_id
}

fn entree(titre: &str, restreint: bool, lien: Option<&str>) -> Value {
    let mut e = json!({
        "title": { "fr": titre },
        "type": "negotiation_guide",
        "restricted": restreint,
    });
    if let Some(url) = lien {
        e["external_url"] = json!(url);
    }
    e
}

async fn brouillon(bac: &Bac, entree: Value) -> Uuid {
    let entree = serde_json::from_value(entree).expect("entrée de document");
    admin::creer(&bac.etat.negotiation, &bac.ctx(bac.admin), &entree)
        .await
        .expect("création du brouillon")
}

async fn publier(bac: &Bac, id: Uuid) {
    admin::publier(&bac.etat.negotiation, &bac.ctx(bac.admin), id)
        .await
        .expect("publication");
}

async fn publie(bac: &Bac, entree: Value) -> Uuid {
    let id = brouillon(bac, entree).await;
    publier(bac, id).await;
    id
}

async fn objet_pdf(bac: &Bac) -> Uuid {
    let bucket: String = sqlx::query_scalar(
        "SELECT value #>> '{}' FROM platform.settings WHERE key = 'media.private_bucket'",
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("bucket privé");
    let id = Uuid::now_v7();
    let cle = format!("essai/{id}.pdf");
    bac.etat
        .negotiation
        .entrepots()
        .du_bucket(&bucket)
        .put(&cle, "application/pdf", PETIT.to_vec())
        .await
        .expect("dépôt de l'objet");
    sqlx::query(
        "INSERT INTO media.assets
             (id, bucket, object_key, checksum_sha256, mime_type, byte_size, owner_person_id,
              visibility, status, scan_verdict)
         VALUES ($1, $2, $3, $4, 'application/pdf', $5, $6, 'private', 'ready', 'clean')",
    )
    .bind(id)
    .bind(&bucket)
    .bind(&cle)
    .bind(format!("{:064x}", id.as_u128()))
    .bind(PETIT.len() as i64)
    .bind(bac.admin)
    .execute(bac.base.pool())
    .await
    .expect("description de l'objet");
    id
}

async fn passer_lextraction(bac: &Bac) {
    let extracteur = ExtractDocument::new(
        bac.etat.db.clone(),
        Entrepots::new(&bac.etat.config.media),
        bac.etat.config.negotiation.pdfium_lib_path.clone(),
    );
    loop {
        let mut tx = bac
            .etat
            .db
            .write(&bac.ctx_anonyme())
            .await
            .expect("transaction");
        let travaux = jobs::claim(&mut tx, extracteur.queue(), "test-worker", 10)
            .await
            .expect("réservation");
        tx.commit().await.expect("validation");
        if travaux.is_empty() {
            break;
        }
        for travail in travaux.into_iter().filter(|t| t.task == extracteur.task()) {
            let issue = extracteur.run(&travail).await;
            let mut tx = bac
                .etat
                .db
                .write(&bac.ctx_anonyme())
                .await
                .expect("transaction");
            match &issue {
                Ok(()) => jobs::succeed(&mut tx, travail.id).await.expect("succès"),
                Err(e) => jobs::fail(&mut tx, travail.id, &e.to_string())
                    .await
                    .expect("échec"),
            }
            tx.commit().await.expect("validation");
            issue.expect("extraction du petit PDF");
        }
    }
}

/// Un brouillon dont le petit PDF est attaché et extrait : ses images existent.
async fn brouillon_extrait(bac: &Bac, titre: &str, restreint: bool) -> Uuid {
    let id = brouillon(bac, entree(titre, restreint, None)).await;
    let asset = objet_pdf(bac).await;
    admin::attacher_le_fichier(&bac.etat.negotiation, &bac.ctx(bac.admin), id, asset)
        .await
        .expect("fichier attaché");
    passer_lextraction(bac).await;
    id
}

/// Un guide public et une note réservée, publiés et extraits du même petit PDF.
async fn fichiers_publies(bac: &Bac) -> (Uuid, Uuid) {
    let guide = brouillon_extrait(bac, "Guide des négociations", false).await;
    let reserve = brouillon_extrait(bac, "Note de position", true).await;
    publier(bac, guide).await;
    publier(bac, reserve).await;
    (guide, reserve)
}

async fn poser_une_note(bac: &Bac, document: Uuid, corps: Value) {
    corrections::poser(
        &bac.etat.negotiation,
        &bac.ctx(bac.expert),
        document,
        &CorrectionNoteInput {
            page_index: 1,
            passage: None,
            body: corps,
        },
    )
    .await
    .expect("note posée");
}

macro_rules! se_connecter {
    ($app:expr, $email:expr) => {{
        let reponse = test::call_service(
            &$app,
            test::TestRequest::post()
                .uri("/api/auth/login")
                .set_json(json!({
                    "email": $email,
                    "password": MOT_DE_PASSE,
                    "remember_me": false,
                    "client": { "kind": "app", "device_id": "9f2c-appareil", "platform": "android" },
                }))
                .to_request(),
        )
        .await;
        assert_eq!(reponse.status(), StatusCode::OK);
        reponse
            .response()
            .cookies()
            .find(|c| c.name() == "epavillon_at")
            .map(|c| format!("{}={}", c.name(), c.value()))
            .expect("cookie d'accès")
    }};
}

fn get(uri: &str, cookie: Option<&str>) -> test::TestRequest {
    avec(test::TestRequest::get().uri(uri), cookie)
}

fn avec(requete: test::TestRequest, cookie: Option<&str>) -> test::TestRequest {
    match cookie {
        Some(c) => requete.insert_header(("cookie", c.to_owned())),
        None => requete,
    }
}

fn entete<B>(reponse: &actix_web::dev::ServiceResponse<B>, nom: impl AsRef<str>) -> Option<String> {
    reponse
        .headers()
        .get(nom.as_ref())
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}

fn etag_de<B>(reponse: &actix_web::dev::ServiceResponse<B>) -> String {
    entete(reponse, ETAG).expect("ETag")
}

fn cache_de<B>(reponse: &actix_web::dev::ServiceResponse<B>) -> Option<String> {
    entete(reponse, CACHE_CONTROL)
}

fn dans<'a>(corps: &'a Value, cle: &str, champ: &str, id: Uuid) -> Option<&'a Value> {
    corps[cle]
        .as_array()
        .unwrap_or_else(|| panic!("{cle} : {corps}"))
        .iter()
        .find(|d| d[champ] == json!(id))
}

fn identifiants(corps: &Value, cle: &str, champ: &str) -> Vec<Value> {
    corps[cle]
        .as_array()
        .unwrap_or_else(|| panic!("{cle} : {corps}"))
        .iter()
        .map(|d| d[champ].clone())
        .collect()
}

/// Les couples (code, libellé) d'un vocabulaire de la bibliothèque.
fn libelles(corps: &Value, vocabulaire: &str) -> Vec<(String, String)> {
    corps["vocabulary"][vocabulaire]
        .as_array()
        .unwrap_or_else(|| panic!("vocabulary.{vocabulaire} : {corps}"))
        .iter()
        .map(|t| {
            (
                t["code"].as_str().unwrap_or_default().to_owned(),
                t["label"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

async fn code_du_refus<B: actix_web::body::MessageBody>(
    reponse: actix_web::dev::ServiceResponse<B>,
) -> String {
    let corps: Value = test::read_body_json(reponse).await;
    corps["code"].as_str().unwrap_or_default().to_owned()
}

/// La session facultative arrive jusqu'au compteur : sans elle, la négociatrice
/// serait refusée comme une visiteuse.
#[actix_web::test]
async fn la_negociatrice_telecharge_le_reserve_et_revalide_son_image_en_prive() {
    let bac = Bac::monter().await;
    let (_, reserve) = fichiers_publies(&bac).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let lecteur = se_connecter!(app, SANS_ACCES);
    let negociatrice = se_connecter!(app, NEGOCIATRICE);
    let telechargement = format!("/api/negotiation/documents/{reserve}/downloads");

    let refuse = test::call_service(
        &app,
        avec(
            test::TestRequest::post().uri(&telechargement),
            Some(&lecteur),
        )
        .to_request(),
    )
    .await;
    assert_eq!(refuse.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        code_du_refus(refuse).await,
        "NEGOTIATION_DOCUMENT_RESTRICTED"
    );
    assert_eq!(bac.telechargements(reserve).await, 0);

    let compte = test::call_service(
        &app,
        avec(
            test::TestRequest::post().uri(&telechargement),
            Some(&negociatrice),
        )
        .to_request(),
    )
    .await;
    assert_eq!(compte.status(), StatusCode::NO_CONTENT);
    assert!(test::read_body(compte).await.is_empty());
    assert_eq!(bac.telechargements(reserve).await, 1);

    let image = format!("/api/negotiation/documents/{reserve}/pages/3/image");
    let servie = test::call_service(&app, get(&image, Some(&negociatrice)).to_request()).await;
    assert_eq!(servie.status(), StatusCode::OK);
    let cache = cache_de(&servie);
    assert!(
        cache
            .as_deref()
            .is_some_and(|c| c.starts_with("private, max-age=")),
        "{cache:?}"
    );
    let empreinte = etag_de(&servie);

    let inchangee = test::call_service(
        &app,
        get(&image, Some(&negociatrice))
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchangee.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(etag_de(&inchangee), empreinte);
    assert_eq!(cache_de(&inchangee), cache, "le 304 reste privé");
    assert!(test::read_body(inchangee).await.is_empty());
}

#[actix_web::test]
async fn un_compte_sans_acces_ne_recoit_pas_les_notes_dun_reserve() {
    let bac = Bac::monter().await;
    let (guide, reserve) = fichiers_publies(&bac).await;
    poser_une_note(&bac, guide, json!({ "fr": NOTE_PUBLIQUE })).await;
    poser_une_note(&bac, reserve, json!({ "fr": NOTE_RESERVEE })).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let anonyme = test::call_service(&app, get(NOTES, None).to_request()).await;
    let empreinte_anonyme = etag_de(&anonyme);

    let lecteur = se_connecter!(app, SANS_ACCES);
    let notes = test::call_service(&app, get(NOTES, Some(&lecteur)).to_request()).await;
    assert_eq!(notes.status(), StatusCode::OK);
    assert_eq!(cache_de(&notes).as_deref(), Some("private, no-cache"));
    assert_eq!(
        etag_de(&notes),
        empreinte_anonyme,
        "le compte sans accès voit ce que voit le public"
    );
    let brut = test::read_body(notes).await;
    let texte = std::str::from_utf8(&brut).expect("UTF-8");
    assert!(!texte.contains(NOTE_RESERVEE), "la note du réservé a fuité");
    let corps: Value = serde_json::from_str(texte).expect("JSON");
    assert_eq!(identifiants(&corps, "notes", "document_id"), [json!(guide)]);

    let inchangees = test::call_service(
        &app,
        get(NOTES, Some(&lecteur))
            .insert_header((IF_NONE_MATCH, empreinte_anonyme.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchangees.status(), StatusCode::NOT_MODIFIED);

    let negociatrice = se_connecter!(app, NEGOCIATRICE);
    let siennes = test::call_service(
        &app,
        get(NOTES, Some(&negociatrice))
            .insert_header((IF_NONE_MATCH, empreinte_anonyme))
            .to_request(),
    )
    .await;
    assert_eq!(siennes.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(siennes).await;
    let note = dans(&corps, "notes", "document_id", reserve).expect("la note du réservé");
    assert_eq!(note["body"], NOTE_RESERVEE);
}

/// Pour qui n'a pas l'accès, `themes_hidden` vaut vrai que le réservé porte des
/// thématiques ou non : sinon la valeur dirait s'il en a. L'adresse d'un lien
/// est son contenu (SC-007) ; son hôte reste affiché.
#[actix_web::test]
async fn un_reserve_sans_acces_masque_ses_thematiques_meme_absentes_et_tait_son_adresse() {
    let bac = Bac::monter().await;
    let url_themee = "https://unfccc.int/documents/cop30-positions";
    let url_nue = "https://unfccc.int/documents/cop30-releve";
    let mut e = entree("Note de position", true, Some(url_themee));
    e["type"] = json!("summary");
    e["themes"] = json!(["mitigation"]);
    let themee = publie(&bac, e).await;
    let nue = publie(&bac, entree("Relevé de décisions", true, Some(url_nue))).await;
    let public = publie(
        &bac,
        entree(
            "Bulletin des négociations",
            false,
            Some("https://enb.iisd.org/cop30"),
        ),
    )
    .await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let lecteur = se_connecter!(app, SANS_ACCES);
    for (qui, cookie) in [("sans compte", None), ("compte sans accès", Some(lecteur))] {
        let liste =
            test::call_service(&app, get(BIBLIOTHEQUE, cookie.as_deref()).to_request()).await;
        assert_eq!(liste.status(), StatusCode::OK, "{qui}");
        let brut = test::read_body(liste).await;
        let texte = std::str::from_utf8(&brut).expect("UTF-8");
        for fuite in ["unfccc.int/documents", "mitigation", "Atténuation"] {
            assert!(!texte.contains(fuite), "{qui} : « {fuite} » a fuité");
        }
        let corps: Value = serde_json::from_str(texte).expect("JSON");
        for id in [themee, nue] {
            let d = dans(&corps, "documents", "id", id).expect("le réservé est listé");
            assert_eq!(d["accessible"], false, "{qui}");
            assert_eq!(d["themes"], json!([]), "{qui}");
            assert_eq!(d["themes_hidden"], true, "{qui}");
            assert_eq!(d["source"], "link", "{qui}");
            assert_eq!(d["external_url"], Value::Null, "{qui}");
            assert_eq!(d["link_host"], "unfccc.int", "{qui}");
        }
        // Le type, lui, reste dit.
        let d = dans(&corps, "documents", "id", themee).expect("listé");
        assert_eq!(d["type"], "summary", "{qui}");
        assert!(libelles(&corps, "types").contains(&("summary".into(), "Résumé".into())));
        assert!(libelles(&corps, "themes").is_empty(), "{qui}");
        let d = dans(&corps, "documents", "id", public).expect("le public est listé");
        assert_eq!(d["themes_hidden"], false, "{qui}");
        assert_eq!(d["external_url"], "https://enb.iisd.org/cop30", "{qui}");
    }

    let negociatrice = se_connecter!(app, NEGOCIATRICE);
    // Témoin des chaînes cherchées plus haut : c'est ainsi que la thématique
    // paraît à qui a l'accès, dans chaque langue.
    for (langue, theme, type_) in [
        ("fr", "Atténuation", "Résumé"),
        ("en", "Mitigation", "Summary"),
    ] {
        let liste = test::call_service(
            &app,
            get(BIBLIOTHEQUE, Some(&negociatrice))
                .insert_header((ACCEPT_LANGUAGE, langue))
                .to_request(),
        )
        .await;
        let corps: Value = test::read_body_json(liste).await;
        assert_eq!(
            libelles(&corps, "themes"),
            [("mitigation".to_owned(), theme.to_owned())],
            "{langue}"
        );
        assert!(
            libelles(&corps, "types").contains(&("summary".into(), type_.into())),
            "{langue}"
        );
    }
    let liste = test::call_service(&app, get(BIBLIOTHEQUE, Some(&negociatrice)).to_request()).await;
    let corps: Value = test::read_body_json(liste).await;
    for (id, url, themes) in [
        (themee, url_themee, json!(["mitigation"])),
        (nue, url_nue, json!([])),
    ] {
        let d = dans(&corps, "documents", "id", id).expect("le réservé est listé");
        assert_eq!(d["accessible"], true);
        assert_eq!(d["themes"], themes);
        assert_eq!(d["themes_hidden"], false);
        assert_eq!(d["external_url"], url);
        assert_eq!(d["link_host"], "unfccc.int");
    }
}

#[actix_web::test]
async fn les_favoris_dun_compte_ne_paraissent_pas_chez_lautre() {
    let bac = Bac::monter().await;
    let public = publie(
        &bac,
        entree(
            "Bulletin des négociations",
            false,
            Some("https://enb.iisd.org/cop30"),
        ),
    )
    .await;
    let reserve = publie(
        &bac,
        entree("Note de position", true, Some("https://unfccc.int/cop30")),
    )
    .await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let lecteur = se_connecter!(app, SANS_ACCES);
    let negociatrice = se_connecter!(app, NEGOCIATRICE);

    // Un favori sur un réservé sans accès est permis : la liste le montre déjà.
    for (cookie, id) in [(&lecteur, reserve), (&negociatrice, public)] {
        let pose = test::call_service(
            &app,
            avec(
                test::TestRequest::put().uri(&format!("{FAVORIS}/{id}")),
                Some(cookie),
            )
            .to_request(),
        )
        .await;
        assert_eq!(pose.status(), StatusCode::NO_CONTENT);
    }

    let liste_du_lecteur =
        test::call_service(&app, get(FAVORIS, Some(&lecteur)).to_request()).await;
    let empreinte_du_lecteur = etag_de(&liste_du_lecteur);
    let corps: Value = test::read_body_json(liste_du_lecteur).await;
    assert_eq!(
        identifiants(&corps, "bookmarks", "document_id"),
        [json!(reserve)]
    );

    let liste = test::call_service(&app, get(FAVORIS, Some(&negociatrice)).to_request()).await;
    let empreinte = etag_de(&liste);
    assert_ne!(empreinte, empreinte_du_lecteur);
    let corps: Value = test::read_body_json(liste).await;
    assert_eq!(
        identifiants(&corps, "bookmarks", "document_id"),
        [json!(public)]
    );

    // Retirer le favori d'une autre ne retire que le sien, qui n'existe pas.
    let retrait = test::call_service(
        &app,
        avec(
            test::TestRequest::delete().uri(&format!("{FAVORIS}/{public}")),
            Some(&lecteur),
        )
        .to_request(),
    )
    .await;
    assert_eq!(retrait.status(), StatusCode::NO_CONTENT);
    let inchangee = test::call_service(
        &app,
        get(FAVORIS, Some(&negociatrice))
            .insert_header((IF_NONE_MATCH, empreinte))
            .to_request(),
    )
    .await;
    assert_eq!(inchangee.status(), StatusCode::NOT_MODIFIED);
}

/// Un brouillon extrait a ses images dans le bucket : le back-office les sert,
/// la route publique non, même à la négociatrice qui forge l'adresse.
#[actix_web::test]
async fn limage_dun_brouillon_se_sert_au_back_office_et_jamais_au_public() {
    let bac = Bac::monter().await;
    let brouillon = brouillon_extrait(&bac, "Note réservée en préparation", true).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let expert = se_connecter!(app, EXPERT);
    let negociatrice = se_connecter!(app, NEGOCIATRICE);

    let apercu = test::call_service(
        &app,
        get(
            &format!("/api/admin/negotiation/documents/{brouillon}/pages/3/image"),
            Some(&expert),
        )
        .to_request(),
    )
    .await;
    assert_eq!(apercu.status(), StatusCode::OK);
    assert_eq!(entete(&apercu, CONTENT_TYPE).as_deref(), Some("image/jpeg"));
    assert!(test::read_body(apercu).await.starts_with(&[0xFF, 0xD8]));

    for cookie in [None, Some(negociatrice.as_str())] {
        for chemin in ["pages/3/image", "reading"] {
            let uri = format!("/api/negotiation/documents/{brouillon}/{chemin}");
            let reponse = test::call_service(&app, get(&uri, cookie).to_request()).await;
            assert_eq!(reponse.status(), StatusCode::NOT_FOUND, "{chemin}");
            assert_eq!(
                code_du_refus(reponse).await,
                "NEGOTIATION_DOCUMENT_NOT_FOUND",
                "{chemin}"
            );
        }
    }

    // Témoin : publié, la même adresse s'ouvre à la négociatrice, donc le 404
    // tenait à la seule publication.
    publier(&bac, brouillon).await;
    let image = |index: i32| format!("/api/negotiation/documents/{brouillon}/pages/{index}/image");
    let servie = test::call_service(&app, get(&image(3), Some(&negociatrice)).to_request()).await;
    assert_eq!(servie.status(), StatusCode::OK);
    assert!(test::read_body(servie).await.starts_with(&[0xFF, 0xD8]));
    let hors_du_document =
        test::call_service(&app, get(&image(99), Some(&negociatrice)).to_request()).await;
    assert_eq!(hors_du_document.status(), StatusCode::NOT_FOUND);
    assert_eq!(code_du_refus(hors_du_document).await, "NOT_FOUND");

    // Sans l'accès, le refus passe avant l'index de page : une page absente ne
    // dit rien de plus qu'une page présente.
    let lecteur = se_connecter!(app, SANS_ACCES);
    for cookie in [None, Some(lecteur.as_str())] {
        for index in [3, 99] {
            let reponse = test::call_service(&app, get(&image(index), cookie).to_request()).await;
            assert_eq!(reponse.status(), StatusCode::FORBIDDEN, "page {index}");
            assert_eq!(
                code_du_refus(reponse).await,
                "NEGOTIATION_DOCUMENT_RESTRICTED",
                "page {index}"
            );
        }
    }
}

#[actix_web::test]
async fn un_document_depublie_quitte_la_liste_et_rend_404_partout() {
    let bac = Bac::monter().await;
    let enb = publie(
        &bac,
        entree(
            "Bulletin des négociations",
            false,
            Some("https://enb.iisd.org/cop30"),
        ),
    )
    .await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let lecteur = se_connecter!(app, SANS_ACCES);
    let lecture = format!("/api/negotiation/documents/{enb}/reading");

    let liste = test::call_service(&app, get(BIBLIOTHEQUE, None).to_request()).await;
    let corps: Value = test::read_body_json(liste).await;
    assert!(dans(&corps, "documents", "id", enb).is_some());
    let avant = test::call_service(&app, get(&lecture, None).to_request()).await;
    assert_eq!(
        code_du_refus(avant).await,
        "NEGOTIATION_DOCUMENT_NOT_READABLE"
    );

    admin::depublier(&bac.etat.negotiation, &bac.ctx(bac.admin), enb)
        .await
        .expect("dépublication");

    let liste = test::call_service(&app, get(BIBLIOTHEQUE, None).to_request()).await;
    let corps: Value = test::read_body_json(liste).await;
    assert!(
        dans(&corps, "documents", "id", enb).is_none(),
        "absent de la liste"
    );
    for requete in [
        get(&lecture, None),
        test::TestRequest::post().uri(&format!("/api/negotiation/documents/{enb}/downloads")),
        avec(
            test::TestRequest::put().uri(&format!("{FAVORIS}/{enb}")),
            Some(&lecteur),
        ),
    ] {
        let reponse = test::call_service(&app, requete.to_request()).await;
        assert_eq!(reponse.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            code_du_refus(reponse).await,
            "NEGOTIATION_DOCUMENT_NOT_FOUND"
        );
    }
}

/// `superseded_by` désigne le bout publié de la chaîne ; un remplaçant resté
/// brouillon ne remplace encore rien.
#[actix_web::test]
async fn la_bibliotheque_mene_chaque_remplace_au_bout_publie_et_ignore_un_brouillon() {
    let bac = Bac::monter().await;
    let lien = Some("https://enb.iisd.org/cop30");
    let a = publie(&bac, entree("Guide, première édition", false, lien)).await;
    let mut e = entree("Guide, deuxième édition", false, lien);
    e["supersedes_id"] = json!(a);
    let b = publie(&bac, e).await;
    let mut e = entree("Guide, troisième édition", false, lien);
    e["supersedes_id"] = json!(b);
    let c = publie(&bac, e).await;
    let mut e = entree("Guide, quatrième édition", false, lien);
    e["supersedes_id"] = json!(c);
    let d = brouillon(&bac, e).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let liste = test::call_service(&app, get(BIBLIOTHEQUE, None).to_request()).await;
    let corps: Value = test::read_body_json(liste).await;
    let bout = dans(&corps, "documents", "id", c).expect("le bout est listé");
    assert_eq!(
        bout["superseded_by"],
        Value::Null,
        "un brouillon ne remplace rien"
    );
    assert!(dans(&corps, "documents", "id", d).is_none());
    for ancien in [a, b] {
        let s = &dans(&corps, "documents", "id", ancien).expect("listé")["superseded_by"];
        assert_eq!(
            *s,
            json!({
                "id": c,
                "title": "Guide, troisième édition",
                "published_at": bout["published_at"],
                "page_count": null,
            }),
            "mène au bout de la chaîne, pas au maillon suivant"
        );
    }
}

/// La route est ouverte à qui publie ou corrige ; `can_post` et `can_withdraw`
/// suivent chacun leur permission. `admin` publie mais ne corrige pas.
#[actix_web::test]
async fn la_liste_des_notes_du_back_office_dit_qui_peut_poser_et_qui_peut_retirer() {
    let bac = Bac::monter().await;
    sqlx::query(
        "INSERT INTO identity.roles (code, label, allowed_scopes)
         VALUES ('relecteur', '{\"fr\":\"Relecteur\"}', '{global}')",
    )
    .execute(bac.base.pool())
    .await
    .expect("rôle de relecture");
    sqlx::query(
        "INSERT INTO identity.role_permissions (role_code, permission_code)
         VALUES ('relecteur', 'negotiation.correction.post')",
    )
    .execute(bac.base.pool())
    .await
    .expect("permission de poser seulement");
    compte(&bac, "relecteur@example.org", Some("relecteur")).await;
    compte(&bac, "pivot@example.org", Some("super_admin")).await;
    let (guide, _) = fichiers_publies(&bac).await;
    poser_une_note(&bac, guide, json!({ "fr": NOTE_PUBLIQUE })).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let chemin = format!("/api/admin/negotiation/documents/{guide}/corrections");

    for (email, poser, retirer) in [
        (EXPERT, true, true),
        ("pivot@example.org", true, true),
        ("relecteur@example.org", true, false),
        (ADMIN, false, false),
    ] {
        let cookie = se_connecter!(app, email);
        let reponse = test::call_service(&app, get(&chemin, Some(&cookie)).to_request()).await;
        assert_eq!(reponse.status(), StatusCode::OK, "{email}");
        let corps: Value = test::read_body_json(reponse).await;
        assert_eq!(corps["can_post"], poser, "{email}");
        assert_eq!(corps["can_withdraw"], retirer, "{email}");
        assert_eq!(identifiants(&corps, "notes", "document_id"), [json!(guide)]);
        assert_eq!(
            corps["notes"][0]["author"]["id"],
            json!(bac.expert),
            "{email}"
        );
    }

    let expert = se_connecter!(app, EXPERT);
    let inconnu = format!(
        "/api/admin/negotiation/documents/{}/corrections",
        Uuid::now_v7()
    );
    let reponse = test::call_service(&app, get(&inconnu, Some(&expert)).to_request()).await;
    assert_eq!(reponse.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        code_du_refus(reponse).await,
        "NEGOTIATION_DOCUMENT_NOT_FOUND"
    );

    let negociatrice = se_connecter!(app, NEGOCIATRICE);
    for (cookie, statut, code) in [
        (
            Some(negociatrice.as_str()),
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
        ),
        (None, StatusCode::UNAUTHORIZED, "UNAUTHENTICATED"),
    ] {
        let reponse = test::call_service(&app, get(&chemin, cookie).to_request()).await;
        assert_eq!(reponse.status(), statut);
        assert_eq!(code_du_refus(reponse).await, code);
    }
}
