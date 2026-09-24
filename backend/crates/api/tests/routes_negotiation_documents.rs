//! Les documents de Guide Négo, **à travers HTTP**.
//!
//! Ce que les tests du crate `negotiation` ne peuvent pas prouver : que les
//! routes sont montées sous `/api`, que la session facultative et la langue
//! demandée remontent jusqu'au service, que l'`ETag` et le `Cache-Control`
//! sortent sur la réponse — et sur le `304` —, et que le code d'un refus arrive
//! tel quel au client.
//!
//! Le décor passe par les services du module, comme le back-office.

use actix_web::http::header::{
    ACCEPT_LANGUAGE, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, ETAG, IF_NONE_MATCH,
};
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

const RESUME_RESERVE: &str = "Synthèse confidentielle des positions de la délégation";
const NOTE_PUBLIQUE: &str = "Coquille au titre de la page 1";
const NOTE_RESERVEE: &str = "Le chiffre du paragraphe 12 est celui de la version de juin";
const RECHERCHE: &str = "negociations%20reprennent";

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

/// Une personne qui se connecte par mot de passe, et son rôle en portée globale.
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

async fn lien_publie(bac: &Bac, titre: &str) -> Uuid {
    publie(
        bac,
        entree(titre, false, Some("https://enb.iisd.org/cop30")),
    )
    .await
}

/// Le PDF de test déposé dans le bucket privé, et décrit en base comme la
/// garde média le ferait.
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

/// Un passage du worker, avec le seul extracteur.
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

/// Un guide public et une note réservée, tous deux des fichiers publiés et
/// extraits du même petit PDF : brouillons, PDF attachés, extraction, publication.
async fn fichiers_publies(bac: &Bac) -> (Uuid, Uuid) {
    let guide = brouillon(bac, entree("Guide des négociations", false, None)).await;
    let reserve = brouillon(bac, entree("Note de position", true, None)).await;
    for id in [guide, reserve] {
        let asset = objet_pdf(bac).await;
        admin::attacher_le_fichier(&bac.etat.negotiation, &bac.ctx(bac.admin), id, asset)
            .await
            .expect("fichier attaché");
    }
    passer_lextraction(bac).await;
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

/// Une lecture, avec la session si l'on en présente une.
fn get(uri: &str, cookie: Option<&str>) -> test::TestRequest {
    let requete = test::TestRequest::get().uri(uri);
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

async fn code_du_refus<B: actix_web::body::MessageBody>(
    reponse: actix_web::dev::ServiceResponse<B>,
) -> String {
    let corps: Value = test::read_body_json(reponse).await;
    corps["code"].as_str().unwrap_or_default().to_owned()
}

#[actix_web::test]
async fn la_bibliotheque_porte_son_empreinte_privee_et_rend_304() {
    let bac = Bac::monter().await;
    let enb = lien_publie(&bac, "Bulletin des négociations").await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let liste = test::call_service(&app, get(BIBLIOTHEQUE, None).to_request()).await;
    assert_eq!(liste.status(), StatusCode::OK);
    assert_eq!(cache_de(&liste).as_deref(), Some("private, no-cache"));
    let empreinte = etag_de(&liste);
    let corps: Value = test::read_body_json(liste).await;
    let document = dans(&corps, "documents", "id", enb).expect("le lien est listé");
    assert_eq!(document["source"], "link");
    assert_eq!(document["link_host"], "enb.iisd.org");

    let inchange = test::call_service(
        &app,
        get(BIBLIOTHEQUE, None)
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(etag_de(&inchange), empreinte);
    assert_eq!(cache_de(&inchange).as_deref(), Some("private, no-cache"));
    assert!(
        test::read_body(inchange).await.is_empty(),
        "un 304 n'a pas de corps"
    );
}

/// L'empreinte suit ce que la personne voit : la copie d'une visiteuse ne vaut
/// pas pour la négociatrice, qui reçoit le résumé et les thématiques.
#[actix_web::test]
async fn la_bibliotheque_suit_la_session_et_tait_le_reserve_a_qui_na_pas_lacces() {
    let bac = Bac::monter().await;
    let mut e = entree(
        "Note de position",
        true,
        Some("https://unfccc.int/documents/cop30"),
    );
    e["summary"] = json!({ "fr": RESUME_RESERVE });
    e["themes"] = json!(["adaptation"]);
    let reserve = publie(&bac, e).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let lecteur = se_connecter!(app, SANS_ACCES);
    let mut empreintes = Vec::new();
    for (qui, cookie) in [("sans compte", None), ("compte sans accès", Some(lecteur))] {
        let liste =
            test::call_service(&app, get(BIBLIOTHEQUE, cookie.as_deref()).to_request()).await;
        assert_eq!(liste.status(), StatusCode::OK);
        assert_eq!(cache_de(&liste).as_deref(), Some("private, no-cache"));
        empreintes.push(etag_de(&liste));
        let brut = test::read_body(liste).await;
        let texte = std::str::from_utf8(&brut).expect("UTF-8");
        assert!(!texte.contains(RESUME_RESERVE), "{qui} : le résumé a fuité");
        let corps: Value = serde_json::from_str(texte).expect("JSON");
        let d = dans(&corps, "documents", "id", reserve).expect("le réservé est listé");
        assert_eq!(d["restricted"], true, "{qui}");
        assert_eq!(d["accessible"], false, "{qui}");
        assert_eq!(d["summary"], Value::Null, "{qui}");
        assert_eq!(d["themes"], json!([]), "{qui}");
        assert_eq!(d["themes_hidden"], true, "{qui}");
        assert_eq!(corps["vocabulary"]["themes"], json!([]), "{qui}");
    }

    let negociatrice = se_connecter!(app, NEGOCIATRICE);
    let liste = test::call_service(
        &app,
        get(BIBLIOTHEQUE, Some(&negociatrice))
            .insert_header((IF_NONE_MATCH, empreintes[0].clone()))
            .to_request(),
    )
    .await;
    assert_eq!(
        liste.status(),
        StatusCode::OK,
        "la copie sans accès ne vaut pas pour la négociatrice"
    );
    assert_eq!(cache_de(&liste).as_deref(), Some("private, no-cache"));
    assert!(!empreintes.contains(&etag_de(&liste)));
    let corps: Value = test::read_body_json(liste).await;
    let d = dans(&corps, "documents", "id", reserve).expect("le réservé est listé");
    assert_eq!(d["accessible"], true);
    assert_eq!(d["summary"], RESUME_RESERVE);
    assert_eq!(d["themes"], json!(["adaptation"]));
    assert_eq!(d["themes_hidden"], false);
    let themes: Vec<&str> = corps["vocabulary"]["themes"]
        .as_array()
        .expect("vocabulaire")
        .iter()
        .filter_map(|t| t["code"].as_str())
        .collect();
    assert_eq!(themes, ["adaptation"]);
}

#[actix_web::test]
async fn la_langue_demandee_resout_les_titres_et_change_lempreinte() {
    let bac = Bac::monter().await;
    let mut e = entree(
        "Guide des négociations",
        false,
        Some("https://enb.iisd.org/cop30"),
    );
    e["title"] = json!({ "fr": "Guide des négociations", "en": "Negotiations guide" });
    let guide = publie(&bac, e).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let fr = test::call_service(&app, get(BIBLIOTHEQUE, None).to_request()).await;
    let empreinte_fr = etag_de(&fr);
    let corps: Value = test::read_body_json(fr).await;
    let d = dans(&corps, "documents", "id", guide).expect("listé");
    assert_eq!(d["title"], "Guide des négociations");

    // Une copie française présentée en anglais ne doit pas rendre 304.
    let en = test::call_service(
        &app,
        get(BIBLIOTHEQUE, None)
            .insert_header((ACCEPT_LANGUAGE, "en"))
            .insert_header((IF_NONE_MATCH, empreinte_fr.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(en.status(), StatusCode::OK);
    assert_ne!(etag_de(&en), empreinte_fr);
    let corps: Value = test::read_body_json(en).await;
    let d = dans(&corps, "documents", "id", guide).expect("listé");
    assert_eq!(d["title"], "Negotiations guide");
}

#[actix_web::test]
async fn un_fichier_public_se_lit_et_se_compte_sans_session_avec_ses_empreintes_figees() {
    let bac = Bac::monter().await;
    let (guide, _) = fichiers_publies(&bac).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    // La lecture d'un public : cache partagé permis, empreinte figée.
    let chemin = format!("/api/negotiation/documents/{guide}/reading");
    let lecture = test::call_service(&app, get(&chemin, None).to_request()).await;
    assert_eq!(lecture.status(), StatusCode::OK);
    assert_eq!(cache_de(&lecture).as_deref(), Some("public, no-cache"));
    let empreinte = etag_de(&lecture);
    let corps: Value = test::read_body_json(lecture).await;
    assert_eq!(corps["id"], json!(guide));
    assert_eq!(
        (corps["has_text"].clone(), corps["large_text"].clone()),
        (json!(true), json!(true))
    );
    assert!(corps.get("mode").is_none(), "« mode » a disparu");
    assert_eq!(corps["page_count"], 4);
    assert!(
        corps["pages"]
            .as_array()
            .expect("pages")
            .iter()
            .all(|p| p.get("image").is_none()),
        "aucune image de page ne va plus au téléphone"
    );

    let inchange = test::call_service(
        &app,
        get(&chemin, None)
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(etag_de(&inchange), empreinte);
    assert_eq!(cache_de(&inchange).as_deref(), Some("public, no-cache"));
    assert!(test::read_body(inchange).await.is_empty());

    // La bibliothèque annonce l'empreinte que la lecture sert : c'est ce qui
    // dit au téléphone si sa copie est la bonne.
    let liste = test::call_service(&app, get(BIBLIOTHEQUE, None).to_request()).await;
    let corps: Value = test::read_body_json(liste).await;
    let d = dans(&corps, "documents", "id", guide).expect("listé");
    assert_eq!(d["source"], "file");
    assert_eq!(d["reading_etag"].as_str(), Some(empreinte.as_str()));

    // Le PDF, par plage, à travers toute l'application : jamais compressé.
    let fichier = format!("/api/negotiation/documents/{guide}/file");
    let morceau = test::call_service(
        &app,
        get(&fichier, None)
            .insert_header(("Range", "bytes=0-99"))
            .insert_header(("Accept-Encoding", "br, gzip"))
            .to_request(),
    )
    .await;
    assert_eq!(morceau.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        entete(&morceau, CONTENT_TYPE).as_deref(),
        Some("application/pdf")
    );
    assert_eq!(
        entete(&morceau, CONTENT_ENCODING).as_deref(),
        Some("identity")
    );
    assert_eq!(
        cache_de(&morceau).as_deref(),
        Some("private, max-age=3600, no-transform")
    );
    let empreinte_fichier = etag_de(&morceau);
    let octets = test::read_body(morceau).await;
    assert_eq!(octets.len(), 100);
    assert!(octets.starts_with(b"%PDF"), "un PDF");

    let fichier_inchange = test::call_service(
        &app,
        get(&fichier, None)
            .insert_header(("Range", "bytes=0-99"))
            .insert_header((IF_NONE_MATCH, empreinte_fichier.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(fichier_inchange.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(etag_de(&fichier_inchange), empreinte_fichier);
    assert!(test::read_body(fichier_inchange).await.is_empty());

    let hors_du_fichier = test::call_service(
        &app,
        get(&fichier, None)
            .insert_header(("Range", "bytes=999999999-"))
            .to_request(),
    )
    .await;
    assert_eq!(hors_du_fichier.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        code_du_refus(hors_du_fichier).await,
        "NEGOTIATION_DOCUMENT_RANGE_INVALID"
    );

    let route_retiree = test::call_service(
        &app,
        get(
            &format!("/api/negotiation/documents/{guide}/pages/3/image"),
            None,
        )
        .to_request(),
    )
    .await;
    assert_eq!(
        route_retiree.status(),
        StatusCode::NOT_FOUND,
        "l'image publique d'une page n'est plus servie"
    );

    // Le téléchargement se compte sans aucun compte.
    let telecharge = test::call_service(
        &app,
        test::TestRequest::post()
            .uri(&format!("/api/negotiation/documents/{guide}/downloads"))
            .to_request(),
    )
    .await;
    assert_eq!(telecharge.status(), StatusCode::NO_CONTENT);
    assert!(test::read_body(telecharge).await.is_empty());
    assert_eq!(bac.telechargements(guide).await, 1);
}

/// Une empreinte présentée ne court-circuite pas l'accès : sans lui, même la
/// copie exacte de la négociatrice reçoit un `403`, jamais un `304`.
#[actix_web::test]
async fn un_reserve_rend_403_sans_session_ou_sans_acces_et_souvre_a_la_negociatrice() {
    let bac = Bac::monter().await;
    let (_, reserve) = fichiers_publies(&bac).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let lecture = format!("/api/negotiation/documents/{reserve}/reading");
    let fichier = format!("/api/negotiation/documents/{reserve}/file");
    let telechargement = format!("/api/negotiation/documents/{reserve}/downloads");

    // La négociatrice l'ouvre, et rien ne se garde dans un cache partagé.
    let negociatrice = se_connecter!(app, NEGOCIATRICE);
    let ouverte = test::call_service(&app, get(&lecture, Some(&negociatrice)).to_request()).await;
    assert_eq!(ouverte.status(), StatusCode::OK);
    assert_eq!(cache_de(&ouverte).as_deref(), Some("private, no-cache"));
    let empreinte = etag_de(&ouverte);

    let inchangee = test::call_service(
        &app,
        get(&lecture, Some(&negociatrice))
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchangee.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(etag_de(&inchangee), empreinte);
    assert_eq!(cache_de(&inchangee).as_deref(), Some("private, no-cache"));

    let fichier_ouvert = test::call_service(
        &app,
        get(&fichier, Some(&negociatrice))
            .insert_header(("Range", "bytes=0-99"))
            .to_request(),
    )
    .await;
    assert_eq!(fichier_ouvert.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        cache_de(&fichier_ouvert).as_deref(),
        Some("private, no-store, no-transform"),
        "un réservé ne reste dans aucun cache"
    );
    let empreinte_fichier = etag_de(&fichier_ouvert);

    // Sans session : lecture, fichier et téléchargement refusés, copie présentée ou non.
    for requete in [
        get(&lecture, None),
        get(&lecture, None).insert_header((IF_NONE_MATCH, empreinte.clone())),
        get(&fichier, None).insert_header(("Range", "bytes=0-99")),
        get(&fichier, None).insert_header((IF_NONE_MATCH, empreinte_fichier.clone())),
        test::TestRequest::post().uri(&telechargement),
    ] {
        let reponse = test::call_service(&app, requete.to_request()).await;
        assert_eq!(reponse.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            code_du_refus(reponse).await,
            "NEGOTIATION_DOCUMENT_RESTRICTED"
        );
    }

    // Un compte sans l'accès négociateur n'y lit pas davantage.
    let lecteur = se_connecter!(app, SANS_ACCES);
    for requete in [
        get(&lecture, Some(&lecteur)),
        get(&fichier, Some(&lecteur)).insert_header(("Range", "bytes=0-99")),
        test::TestRequest::post()
            .uri(&telechargement)
            .insert_header(("cookie", lecteur.clone())),
    ] {
        let reponse = test::call_service(&app, requete.to_request()).await;
        assert_eq!(reponse.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            code_du_refus(reponse).await,
            "NEGOTIATION_DOCUMENT_RESTRICTED"
        );
    }
    assert_eq!(
        bac.telechargements(reserve).await,
        0,
        "un refus ne compte rien"
    );
}

#[actix_web::test]
async fn les_notes_rendent_304_suivent_la_langue_et_ne_donnent_celles_dun_reserve_qua_lacces() {
    let bac = Bac::monter().await;
    let (guide, reserve) = fichiers_publies(&bac).await;
    poser_une_note(
        &bac,
        guide,
        json!({ "fr": NOTE_PUBLIQUE, "en": "Typo in the title of page 1" }),
    )
    .await;
    poser_une_note(&bac, reserve, json!({ "fr": NOTE_RESERVEE })).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    let notes = test::call_service(&app, get(NOTES, None).to_request()).await;
    assert_eq!(notes.status(), StatusCode::OK);
    assert_eq!(cache_de(&notes).as_deref(), Some("private, no-cache"));
    let empreinte = etag_de(&notes);
    let brut = test::read_body(notes).await;
    let texte = std::str::from_utf8(&brut).expect("UTF-8");
    assert!(!texte.contains(NOTE_RESERVEE), "la note du réservé a fuité");
    let corps: Value = serde_json::from_str(texte).expect("JSON");
    let documents: Vec<&Value> = corps["notes"]
        .as_array()
        .expect("notes")
        .iter()
        .map(|n| &n["document_id"])
        .collect();
    assert_eq!(documents, [&json!(guide)]);
    assert_eq!(corps["notes"][0]["body"], NOTE_PUBLIQUE);

    let inchangees = test::call_service(
        &app,
        get(NOTES, None)
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchangees.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(etag_de(&inchangees), empreinte);
    assert_eq!(cache_de(&inchangees).as_deref(), Some("private, no-cache"));

    let en = test::call_service(
        &app,
        get(NOTES, None)
            .insert_header((ACCEPT_LANGUAGE, "en"))
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(
        en.status(),
        StatusCode::OK,
        "la copie française ne vaut pas en anglais"
    );
    let corps: Value = test::read_body_json(en).await;
    assert_eq!(corps["notes"][0]["body"], "Typo in the title of page 1");

    let negociatrice = se_connecter!(app, NEGOCIATRICE);
    let siennes = test::call_service(
        &app,
        get(NOTES, Some(&negociatrice))
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(siennes.status(), StatusCode::OK);
    assert_ne!(etag_de(&siennes), empreinte);
    let corps: Value = test::read_body_json(siennes).await;
    let note = dans(&corps, "notes", "document_id", reserve).expect("la note du réservé");
    assert_eq!(note["body"], NOTE_RESERVEE);
    assert!(dans(&corps, "notes", "document_id", guide).is_some());
}

#[actix_web::test]
async fn la_recherche_na_pas_dempreinte_et_ne_cite_un_reserve_qua_qui_a_lacces() {
    let bac = Bac::monter().await;
    let (guide, reserve) = fichiers_publies(&bac).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let chemin = format!("{BIBLIOTHEQUE}?q={RECHERCHE}");

    let lecteur = se_connecter!(app, SANS_ACCES);
    for (qui, cookie) in [("sans compte", None), ("compte sans accès", Some(lecteur))] {
        let reponse = test::call_service(&app, get(&chemin, cookie.as_deref()).to_request()).await;
        assert_eq!(reponse.status(), StatusCode::OK, "{qui}");
        assert_eq!(
            cache_de(&reponse).as_deref(),
            Some("private, no-cache"),
            "{qui}"
        );
        assert_eq!(entete(&reponse, ETAG), None, "{qui} : pas d'empreinte");
        let corps: Value = test::read_body_json(reponse).await;
        let r = dans(&corps, "hits", "document_id", reserve)
            .unwrap_or_else(|| panic!("{qui} : le réservé est nommé"));
        assert_eq!(
            *r,
            json!({ "document_id": reserve, "pages": [] }),
            "{qui} : ni page ni extrait"
        );
        let p = dans(&corps, "hits", "document_id", guide).expect("le public est trouvé");
        assert!(p["pages"][0]["excerpt"]
            .as_str()
            .is_some_and(|e| !e.is_empty()));
    }

    let negociatrice = se_connecter!(app, NEGOCIATRICE);
    let reponse = test::call_service(&app, get(&chemin, Some(&negociatrice)).to_request()).await;
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = test::read_body_json(reponse).await;
    let r = dans(&corps, "hits", "document_id", reserve).expect("le réservé est trouvé");
    assert_eq!(r["pages"][0]["index"], 1);
    assert!(r["pages"][0]["excerpt"]
        .as_str()
        .is_some_and(|e| !e.is_empty()));

    let vide = test::call_service(
        &app,
        get(&format!("{BIBLIOTHEQUE}?q=%20%20"), None).to_request(),
    )
    .await;
    assert_eq!(vide.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(code_du_refus(vide).await, "VALIDATION_FAILED");
}

#[actix_web::test]
async fn un_lien_ne_se_lit_pas_et_un_document_inconnu_ou_non_publie_est_introuvable() {
    let bac = Bac::monter().await;
    let enb = lien_publie(&bac, "Bulletin des négociations").await;
    let non_publie = brouillon(&bac, entree("Brouillon", false, None)).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;

    for chemin in ["reading", "file"] {
        let uri = format!("/api/negotiation/documents/{enb}/{chemin}");
        let lien = test::call_service(&app, get(&uri, None).to_request()).await;
        assert_eq!(lien.status(), StatusCode::CONFLICT, "{chemin}");
        assert_eq!(
            code_du_refus(lien).await,
            "NEGOTIATION_DOCUMENT_NOT_READABLE",
            "{chemin}"
        );
    }

    for id in [non_publie, Uuid::now_v7()] {
        for requete in [
            get(&format!("/api/negotiation/documents/{id}/reading"), None),
            get(&format!("/api/negotiation/documents/{id}/file"), None),
            test::TestRequest::post().uri(&format!("/api/negotiation/documents/{id}/downloads")),
        ] {
            let reponse = test::call_service(&app, requete.to_request()).await;
            assert_eq!(reponse.status(), StatusCode::NOT_FOUND);
            assert_eq!(
                code_du_refus(reponse).await,
                "NEGOTIATION_DOCUMENT_NOT_FOUND"
            );
        }
    }
}

#[actix_web::test]
async fn sans_session_les_favoris_rendent_401() {
    let bac = Bac::monter().await;
    let enb = lien_publie(&bac, "Bulletin des négociations").await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let favori = format!("{FAVORIS}/{enb}");

    for requete in [
        get(FAVORIS, None),
        test::TestRequest::put().uri(&favori),
        test::TestRequest::delete().uri(&favori),
    ] {
        let reponse = test::call_service(&app, requete.to_request()).await;
        assert_eq!(reponse.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(code_du_refus(reponse).await, "UNAUTHENTICATED");
    }

    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM negotiation.document_bookmarks")
        .fetch_one(bac.base.pool())
        .await
        .expect("compte");
    assert_eq!(n, 0);
}

#[actix_web::test]
async fn poser_et_retirer_un_favori_deux_fois_rend_204_et_la_liste_suit_avec_son_304() {
    let bac = Bac::monter().await;
    let enb = lien_publie(&bac, "Bulletin des négociations").await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let cookie = se_connecter!(app, SANS_ACCES);
    let favori = format!("{FAVORIS}/{enb}");

    for _ in 0..2 {
        let pose = test::call_service(
            &app,
            test::TestRequest::put()
                .uri(&favori)
                .insert_header(("cookie", cookie.clone()))
                .to_request(),
        )
        .await;
        assert_eq!(pose.status(), StatusCode::NO_CONTENT);
    }

    let liste = test::call_service(&app, get(FAVORIS, Some(&cookie)).to_request()).await;
    assert_eq!(liste.status(), StatusCode::OK);
    assert_eq!(cache_de(&liste).as_deref(), Some("private, no-cache"));
    let empreinte = etag_de(&liste);
    let corps: Value = test::read_body_json(liste).await;
    let favoris = corps["bookmarks"].as_array().expect("bookmarks");
    assert_eq!(favoris.len(), 1, "poser deux fois ne crée rien de plus");
    assert_eq!(favoris[0]["document_id"], json!(enb));
    assert!(favoris[0]["created_at"].is_string());

    let inchange = test::call_service(
        &app,
        get(FAVORIS, Some(&cookie))
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(inchange.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(etag_de(&inchange), empreinte);
    assert_eq!(cache_de(&inchange).as_deref(), Some("private, no-cache"));

    for _ in 0..2 {
        let retrait = test::call_service(
            &app,
            test::TestRequest::delete()
                .uri(&favori)
                .insert_header(("cookie", cookie.clone()))
                .to_request(),
        )
        .await;
        assert_eq!(retrait.status(), StatusCode::NO_CONTENT);
    }

    let vide = test::call_service(
        &app,
        get(FAVORIS, Some(&cookie))
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(
        vide.status(),
        StatusCode::OK,
        "l'empreinte a suivi le retrait"
    );
    assert_ne!(etag_de(&vide), empreinte);
    let corps: Value = test::read_body_json(vide).await;
    assert_eq!(corps["bookmarks"], json!([]));
}

/// Un brouillon n'existe pas pour le public : on ne s'y abonne pas.
#[actix_web::test]
async fn un_favori_sur_un_document_inconnu_ou_non_publie_rend_404_not_found() {
    let bac = Bac::monter().await;
    let non_publie = brouillon(&bac, entree("Brouillon", false, None)).await;
    let app = test::init_service(api::build_app(&bac.etat)).await;
    let cookie = se_connecter!(app, SANS_ACCES);

    for id in [non_publie, Uuid::now_v7()] {
        let reponse = test::call_service(
            &app,
            test::TestRequest::put()
                .uri(&format!("{FAVORIS}/{id}"))
                .insert_header(("cookie", cookie.clone()))
                .to_request(),
        )
        .await;
        assert_eq!(reponse.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            code_du_refus(reponse).await,
            "NEGOTIATION_DOCUMENT_NOT_FOUND"
        );
    }
}
