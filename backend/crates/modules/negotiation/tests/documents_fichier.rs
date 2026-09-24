//! **Le PDF servi par plages** (contracts/api-lecture.md) : `206` et
//! `Content-Range`, `200` entier, `416`, `304` malgré un relais qui suffixe
//! l'empreinte, `HEAD` sans corps, jamais compressé ; et le réservé refusé
//! **à chaque morceau** (SC-009) — par la route, sur base réelle.

mod commun;

use actix_web::body::MessageBody;
use actix_web::dev::{Service as _, ServiceResponse};
use actix_web::http::header::{
    HeaderMap, ACCEPT_RANGES, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE,
    CONTENT_TYPE, ETAG,
};
use actix_web::http::StatusCode;
use actix_web::test::{call_service, init_service, read_body, TestRequest};
use actix_web::{web, App, HttpMessage as _};
use commun::documents::{
    administratrice, creer, fichier_publie, negociatrice, objet_pdf, passer_lextraction, PETIT,
};
use commun::{personne, Bac};
use kernel::context::RequestContext;
use negotiation::service::admin_documents;
use serde_json::Value;
use uuid::Uuid;

/// L'en-tête qui tient lieu de session dans l'application d'essai.
const ACTEUR: &str = "x-essai-acteur";

struct Reponse {
    statut: StatusCode,
    entetes: HeaderMap,
    corps: Vec<u8>,
}

impl Reponse {
    fn entete(&self, nom: actix_web::http::header::HeaderName) -> Option<&str> {
        self.entetes.get(nom).and_then(|v| v.to_str().ok())
    }

    fn code(&self) -> Option<String> {
        serde_json::from_slice::<Value>(&self.corps)
            .ok()
            .and_then(|v| v["code"].as_str().map(str::to_owned))
    }

    /// Aucun octet du PDF n'est parti.
    fn sans_pdf(&self) -> bool {
        !self.corps.windows(4).any(|f| f == b"%PDF")
    }
}

async fn lire_la_reponse<B: MessageBody>(reponse: ServiceResponse<B>) -> Reponse {
    let statut = reponse.status();
    let entetes = reponse.headers().clone();
    Reponse {
        statut,
        entetes,
        corps: read_body(reponse).await.to_vec(),
    }
}

macro_rules! frapper {
    ($app:expr, $requete:expr $(,)?) => {
        lire_la_reponse(call_service($app, $requete.to_request()).await).await
    };
}

fn demande(id: Uuid, acteur: Option<Uuid>, plage: Option<&str>) -> TestRequest {
    let mut r = TestRequest::get()
        .uri(&format!("/negotiation/documents/{id}/file"))
        // Un navigateur annonce toujours la compression : la route doit la refuser.
        .insert_header(("Accept-Encoding", "br, gzip"));
    if let Some(a) = acteur {
        r = r.insert_header((ACTEUR, a.to_string()));
    }
    if let Some(p) = plage {
        r = r.insert_header(("Range", p));
    }
    r
}

macro_rules! application {
    ($bac:expr) => {
        init_service(
            App::new()
                .app_data(web::Data::new($bac.db()))
                .app_data(web::Data::new($bac.state.clone()))
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
                .configure(negotiation::routes),
        )
        .await
    };
}

/// Ce que toute réponse de la route porte, refus de plage compris.
fn jamais_compresse(r: &Reponse, quoi: &str) {
    assert_eq!(r.entete(CONTENT_ENCODING), Some("identity"), "{quoi}");
    assert!(
        r.entete(CACHE_CONTROL)
            .is_some_and(|c| c.contains("no-transform")),
        "{quoi} : un relais ne doit pas la transformer"
    );
    assert_eq!(r.entete(ACCEPT_RANGES), Some("bytes"), "{quoi}");
}

// -----------------------------------------------------------------------------
// Les plages
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_fichier_se_sert_par_plages_et_entier() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let app = application!(bac);
    let total = PETIT.len();

    let entier = frapper!(&app, demande(guide, None, None));
    assert_eq!(
        entier.statut,
        StatusCode::OK,
        "sans plage, le fichier entier"
    );
    assert_eq!(entier.corps, PETIT);
    assert_eq!(entier.entete(CONTENT_TYPE), Some("application/pdf"));
    assert!(entier.entete(CONTENT_RANGE).is_none());
    jamais_compresse(&entier, "entier");

    for (plage, debut, fin) in [
        ("bytes=0-99", 0, 99),
        ("bytes=100-", 100, total - 1),
        ("bytes=-50", total - 50, total - 1),
        ("bytes=10-10", 10, 10),
        ("bytes=0-99, 200-299", 0, 99),
        (
            &format!("bytes={}-99999999", total - 3),
            total - 3,
            total - 1,
        ),
    ] {
        let r = frapper!(&app, demande(guide, None, Some(plage)));
        assert_eq!(r.statut, StatusCode::PARTIAL_CONTENT, "{plage}");
        assert_eq!(
            r.entete(CONTENT_RANGE),
            Some(format!("bytes {debut}-{fin}/{total}").as_str()),
            "{plage}"
        );
        assert_eq!(
            r.corps,
            PETIT[debut..=fin],
            "{plage} : ces octets et aucun autre"
        );
        jamais_compresse(&r, plage);
    }

    let hors = frapper!(&app, demande(guide, None, Some(&format!("bytes={total}-"))));
    assert_eq!(hors.statut, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        hors.code().as_deref(),
        Some("NEGOTIATION_DOCUMENT_RANGE_INVALID")
    );
    assert_eq!(
        hors.entete(CONTENT_RANGE),
        Some(format!("bytes */{total}").as_str()),
        "le 416 dit la taille du fichier"
    );
    assert!(hors.sans_pdf());
    jamais_compresse(&hors, "416");

    let illisible = frapper!(&app, demande(guide, None, Some("octets=0-10")));
    assert_eq!(
        (illisible.statut, illisible.corps.len()),
        (StatusCode::OK, total),
        "une plage illisible s'ignore"
    );
}

#[tokio::test]
async fn lempreinte_du_fichier_rend_304_meme_suffixee_par_un_relais() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let jumeau = fichier_publie(&bac, ifdd, "Guide jumeau", false).await;
    let app = application!(bac);

    let premiere = frapper!(&app, demande(guide, None, Some("bytes=0-9")));
    let empreinte = premiere.entete(ETAG).expect("une empreinte").to_owned();
    assert!(
        empreinte.starts_with('"'),
        "une empreinte forte : {empreinte}"
    );
    let entier = frapper!(&app, demande(guide, None, None));
    assert_eq!(
        entier.entete(ETAG),
        Some(empreinte.as_str()),
        "une empreinte par fichier, plage ou non"
    );

    let noyau = empreinte.trim_matches('"');
    for presentee in [
        empreinte.clone(),
        format!("\"{noyau}-br\""),
        format!("W/\"{noyau}-gzip\""),
    ] {
        let r = frapper!(
            &app,
            demande(guide, None, Some("bytes=0-9"))
                .insert_header(("If-None-Match", presentee.as_str())),
        );
        assert_eq!(r.statut, StatusCode::NOT_MODIFIED, "{presentee}");
        assert!(r.corps.is_empty());
    }

    let autre = frapper!(
        &app,
        demande(jumeau, None, Some("bytes=0-9"))
            .insert_header(("If-None-Match", empreinte.as_str())),
    );
    assert_eq!(
        autre.statut,
        StatusCode::PARTIAL_CONTENT,
        "même fichier déposé deux fois, deux objets : pas la même empreinte"
    );
}

#[tokio::test]
async fn head_rend_les_entetes_sans_le_corps() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let app = application!(bac);
    let total = PETIT.len();

    let tete = frapper!(
        &app,
        demande(guide, None, None).method(actix_web::http::Method::HEAD),
    );
    assert_eq!(tete.statut, StatusCode::OK);
    assert!(tete.corps.is_empty(), "HEAD : aucun corps");
    assert_eq!(
        tete.entete(CONTENT_LENGTH),
        Some(total.to_string().as_str()),
        "la taille, que le lecteur lit avant la première plage"
    );
    jamais_compresse(&tete, "HEAD");

    let partielle = frapper!(
        &app,
        demande(guide, None, Some("bytes=0-99")).method(actix_web::http::Method::HEAD),
    );
    assert_eq!(partielle.statut, StatusCode::PARTIAL_CONTENT);
    assert!(partielle.corps.is_empty());
    assert_eq!(
        partielle.entete(CONTENT_RANGE),
        Some(format!("bytes 0-99/{total}").as_str())
    );
}

// -----------------------------------------------------------------------------
// Le cache
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_reserve_ne_se_garde_jamais_en_cache_un_public_si() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let aissatou = negociatrice(&bac, "aissatou@example.org").await;
    let public = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let reserve = fichier_publie(&bac, ifdd, "Note réservée", true).await;
    let app = application!(bac);

    let p = frapper!(&app, demande(public, None, Some("bytes=0-9")));
    assert_eq!(
        p.entete(CACHE_CONTROL),
        Some("private, max-age=3600, no-transform")
    );
    let r = frapper!(&app, demande(reserve, Some(aissatou), Some("bytes=0-9")));
    assert_eq!(r.statut, StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        r.entete(CACHE_CONTROL),
        Some("private, no-store, no-transform"),
        "un réservé ne reste dans aucun cache (FR-029)"
    );
}

// -----------------------------------------------------------------------------
// Le réservé, à chaque morceau
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_reserve_se_refuse_a_chaque_morceau_sans_un_octet() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let aissatou = negociatrice(&bac, "aissatou@example.org").await;
    let visiteuse = personne(&bac, "visiteuse@example.org").await;
    let reserve = fichier_publie(&bac, ifdd, "Note réservée", true).await;

    let brouillon = creer(&bac, ifdd, "Note en préparation", false).await;
    let asset = objet_pdf(&bac, ifdd, PETIT, "ready").await;
    admin_documents::attacher_le_fichier(&bac.state, &bac.ctx(ifdd), brouillon, asset)
        .await
        .expect("fichier attaché");
    let issues = passer_lextraction(&bac).await;
    assert!(issues.iter().all(Result::is_ok), "{issues:?}");
    let app = application!(bac);

    for (qui, acteur) in [
        ("sans compte", None),
        ("compte sans accès", Some(visiteuse)),
    ] {
        for plage in [
            None,
            Some("bytes=0-1023"),
            Some("bytes=-1"),
            Some("bytes=99999999-"),
        ] {
            let r = frapper!(&app, demande(reserve, acteur, plage));
            assert_eq!(
                (r.statut, r.code().as_deref()),
                (
                    StatusCode::FORBIDDEN,
                    Some("NEGOTIATION_DOCUMENT_RESTRICTED")
                ),
                "{qui}, {plage:?} : le refus passe avant la plage"
            );
            assert!(r.sans_pdf(), "{qui}, {plage:?} : jamais un octet");
        }
    }
    for acteur in [None, Some(aissatou)] {
        let r = frapper!(&app, demande(brouillon, acteur, Some("bytes=0-1023")));
        assert_eq!(
            (r.statut, r.code().as_deref()),
            (
                StatusCode::NOT_FOUND,
                Some("NEGOTIATION_DOCUMENT_NOT_FOUND")
            ),
            "adresse forgée d'un brouillon"
        );
        assert!(r.sans_pdf());
    }

    // Témoin, puis retrait entre deux morceaux : le second ne part pas.
    let premier = frapper!(&app, demande(reserve, Some(aissatou), Some("bytes=0-1023")));
    assert_eq!(premier.statut, StatusCode::PARTIAL_CONTENT, "avec l'accès");
    assert!(premier.corps.starts_with(b"%PDF"));
    admin_documents::depublier(&bac.state, &bac.ctx(ifdd), reserve)
        .await
        .expect("dépublication");
    let second = frapper!(
        &app,
        demande(reserve, Some(aissatou), Some("bytes=1024-2047")),
    );
    assert_eq!(
        (second.statut, second.code().as_deref()),
        (
            StatusCode::NOT_FOUND,
            Some("NEGOTIATION_DOCUMENT_NOT_FOUND")
        ),
        "dépublié entre deux morceaux : l'accès se relit à chaque requête"
    );
    assert!(second.sans_pdf());
}
