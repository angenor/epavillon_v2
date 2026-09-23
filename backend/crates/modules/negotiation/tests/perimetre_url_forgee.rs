//! **Un administrateur d'événement ne voit rien du back-office de Guide Négo**
//! — FR-044, SC-008, principe V.
//!
//! Les **douze** routes du back-office — sept sur les codes, trois sur les
//! demandes, deux sur le mode d'admission — exigent
//! `negotiation.space.manage` **sur la portée globale**, et rien d'autre. Un
//! identifiant forgé se refuse donc **avant toute lecture** : la garde tombe
//! sans que la route ait touché la base.
//!
//! # POURQUOI CE FICHIER LIT AUSSI LE CODE SOURCE
//!
//! Le piège que SC-008 nomme n'est pas qu'une route soit ouverte : c'est
//! qu'elle *paraisse* gardée. Le rôle `admin` porte
//! `negotiation.space.manage` et s'attribue **aussi sur un événement** ;
//! `RequiresAnyScope<SpaceManage>` compilerait, se lirait comme une garde, et
//! laisserait entrer l'administrateur d'une seule édition — alors qu'aucun
//! espace de négociation n'est rattaché à une édition.
//!
//! Monter la vraie application pour l'éprouver en HTTP demanderait à ce crate
//! une dépendance de développement vers `api`, que `cargo tree` refuse
//! (principe II, voir `frontieres.rs`). Le contrôle porte donc sur deux choses
//! qui, ensemble, ferment le cas : **ce que la base répond** à un administrateur
//! d'événement, et **quel extracteur les douze routes déclarent**.
//!
//! # LES DIX-SEPT ROUTES DES DOCUMENTS
//!
//! `contracts/api-admin-documents.md` en ajoute dix-sept, gardées chacune par
//! la permission de son geste — publier, poser une note, la retirer — ou, pour
//! lire, par publier **ou** corriger ; toujours sur la portée globale. Le même
//! piège les guette : le rôle `admin` porte `negotiation.document.publish`, et
//! un document peut être rattaché à une COP. Elles sont contrôlées dans leur
//! source, puis **frappées une à une** : le back-office du module se monte seul
//! dans une application d'essai, sans `api`, un intergiciel posant l'acteur à la
//! place de la session.
//!
//! Compter les `.route(…)` ne suffit pas : un `.service`, un scope, un
//! `default_service` ou une macro monteraient une porte hors du compte. Chaque
//! `configurer()` doit donc être **exactement** la suite de ses routes. Que qui
//! détient le geste passe chaque garde est éprouvé dans
//! `perimetre_chaque_geste_a_son_detenteur.rs`.

mod commun;

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::{Method, StatusCode};
// `actix_web::test` s'importe par ses fonctions : le module entier masquerait `#[test]`.
use actix_web::test::{call_service, init_service, read_body, TestRequest};
use actix_web::{web, App, HttpMessage as _};
use commun::documents::{administratrice, entree, expert, objet_pdf, PETIT};
use commun::{administre_guide_nego, attribuer, decor, personne, Bac};
use kernel::auth::Scope;
use kernel::context::RequestContext;
use negotiation::domain::admin_documents::CorrectionNoteInput;
use negotiation::domain::permissions::{DOCUMENT_PUBLISH, SPACE_MANAGE};
use negotiation::service::{admin_documents, corrections};
use serde_json::{json, Value};
use uuid::Uuid;

const ADMIN_CODES: &str = include_str!("../src/routes/admin_codes.rs");
const ADMIN_DEMANDES: &str = include_str!("../src/routes/admin_requests.rs");
const ADMIN_ADMISSION: &str = include_str!("../src/routes/admin_admission.rs");

/// Les douze routes, telles que `contracts/api-admin.md` les énumère.
const ROUTES: [&str; 12] = [
    "/admin/negotiation/invitation-codes",
    "/admin/negotiation/invitation-codes",
    "/admin/negotiation/invitation-codes/{id}",
    "/admin/negotiation/invitation-codes/{id}/revoke",
    "/admin/negotiation/invitation-codes/{id}/uses",
    "/admin/negotiation/invitation-codes/{id}/uses/{person_id}/revoke-access",
    "/admin/negotiation/invitation-codes/{id}/revoke-all-access",
    "/admin/negotiation/access-requests",
    "/admin/negotiation/access-requests/{id}/approve",
    "/admin/negotiation/access-requests/{id}/reject",
    "/admin/negotiation/admission",
    "/admin/negotiation/admission",
];

fn sources() -> [&'static str; 3] {
    [ADMIN_CODES, ADMIN_DEMANDES, ADMIN_ADMISSION]
}

/// Le code seul, commentaires retirés.
///
/// Les fichiers de ce module **expliquent le piège dans leur en-tête**, et y
/// nomment donc `RequiresAnyScope` et `Perimeter` pour dire de ne pas s'en
/// servir. Compter sur le texte brut ferait échouer le contrôle sur
/// l'explication qui le justifie.
fn sans_commentaires(source: &str) -> String {
    source
        .lines()
        .filter(|ligne| !ligne.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn les_douze_routes_sont_declarees() {
    let tout: String = sources().map(sans_commentaires).concat();
    for route in ROUTES {
        assert!(
            tout.contains(&format!("\"{route}\"")),
            "la route {route} n'est montée nulle part"
        );
    }

    // Douze `web::` dans les trois `configurer()` : ni plus — une treizième
    // route serait une porte que personne n'a éprouvée —, ni moins.
    let montees: usize = sources()
        .map(|source| sans_commentaires(source).matches(".route(").count())
        .iter()
        .sum();
    assert_eq!(montees, 12, "douze routes, pas une de plus");
}

#[test]
fn chaque_gestionnaire_exige_la_portee_globale() {
    for source in sources().map(sans_commentaires) {
        let gestionnaires = source.matches("pub(crate) async fn ").count();
        let gardes = source.matches("Requires<SpaceManage>").count();
        assert_eq!(
            gestionnaires, gardes,
            "chaque gestionnaire déclare sa garde, et c'est la même pour tous"
        );
    }
}

#[test]
fn aucune_route_nemprunte_une_garde_plus_large() {
    for source in sources().map(sans_commentaires) {
        // **`RequiresAnyScope` est le piège**, et il compilerait : le rôle
        // `admin` porte la permission sur un événement.
        assert!(
            !source.contains("RequiresAnyScope"),
            "« n'importe quelle portée » ouvrirait le back-office à un administrateur d'édition"
        );
        // `Perimeter` ne conviendrait pas davantage :
        // `identity.administered_events()` ne rend que des portées `event`,
        // quand un code porte `global` ou `negotiation_space`.
        assert!(
            !source.contains("Perimeter"),
            "le périmètre d'édition n'a rien à dire d'un espace de négociation"
        );
    }
}

#[tokio::test]
async fn ladministrateur_dune_edition_ne_passe_aucune_garde() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    let edition = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2027, jsonb_build_object('fr', 'COP31'),
                   'cop31-url-forgee'::text::platform.slug,
                   jsonb_build_object('fr', 'COP31'), 'online',
                   'America/Belem'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition");

    attribuer(&bac, d.admin_id, "admin", "event", Some(edition)).await;

    // La permission existe **sur son édition** : c'est ce qui rend le piège
    // crédible.
    assert!(kernel::auth::has_permission(
        bac.pool(),
        d.admin_id,
        SPACE_MANAGE,
        Scope::Event(edition)
    )
    .await
    .expect("lecture de la permission"));

    // Et elle n'existe pas sur la portée globale, la seule que la garde teste.
    assert!(!administre_guide_nego(&bac, d.admin_id).await);

    // Pas davantage sur l'espace de négociation dont il forgerait l'adresse.
    assert!(!kernel::auth::has_permission(
        bac.pool(),
        d.admin_id,
        SPACE_MANAGE,
        Scope::NegotiationSpace(d.space_id)
    )
    .await
    .expect("lecture de la permission"));
}

// ---------------------------------------------------------------------------
// Les dix-sept routes des documents et des notes
// ---------------------------------------------------------------------------

const ADMIN_DOCUMENTS: &str = include_str!("../src/routes/admin_documents.rs");
const LIB: &str = include_str!("../src/lib.rs");

const LIRE: &str = "LectureDocuments";
const PUBLIER: &str = "Requires<DocumentPublish>";
const POSER: &str = "Requires<CorrectionPost>";
const RETIRER: &str = "Requires<CorrectionWithdraw>";
const GARDES: [&str; 4] = [LIRE, PUBLIER, POSER, RETIRER];

/// Les dix-sept routes de `contracts/api-admin-documents.md`, chacune avec la
/// garde que le contrat lui donne.
const ROUTES_DOCUMENTS: [(&str, &str, &str); 17] = [
    ("get", "/admin/negotiation/documents", LIRE),
    ("post", "/admin/negotiation/documents", PUBLIER),
    ("get", "/admin/negotiation/documents/{id}", LIRE),
    ("patch", "/admin/negotiation/documents/{id}", PUBLIER),
    ("put", "/admin/negotiation/documents/{id}/file", PUBLIER),
    (
        "post",
        "/admin/negotiation/documents/{id}/extraction",
        PUBLIER,
    ),
    ("put", "/admin/negotiation/documents/{id}/as-is", PUBLIER),
    ("post", "/admin/negotiation/documents/{id}/publish", PUBLIER),
    (
        "post",
        "/admin/negotiation/documents/{id}/unpublish",
        PUBLIER,
    ),
    (
        "post",
        "/admin/negotiation/documents/{id}/new-version",
        PUBLIER,
    ),
    ("delete", "/admin/negotiation/documents/{id}", PUBLIER),
    ("get", "/admin/negotiation/documents/{id}/preview", LIRE),
    ("get", "/admin/negotiation/documents/{id}/file", LIRE),
    (
        "get",
        "/admin/negotiation/documents/{id}/pages/{index}/image",
        LIRE,
    ),
    ("get", "/admin/negotiation/documents/{id}/corrections", LIRE),
    (
        "post",
        "/admin/negotiation/documents/{id}/corrections",
        POSER,
    ),
    (
        "post",
        "/admin/negotiation/corrections/{note_id}/withdraw",
        RETIRER,
    ),
];

/// Le code sans commentaires ni blancs : `rustfmt` coupe les appels longs sur
/// plusieurs lignes.
fn compact(source: &str) -> String {
    sans_commentaires(source)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Les `.route("chemin", web::verbe().to(gestionnaire))` du fichier, en
/// `(verbe, chemin, gestionnaire)`.
fn routes_montees(source: &str) -> Vec<(String, String, String)> {
    compact(source)
        .split(".route(")
        .skip(1)
        .map(|appel| {
            let (chemin, reste) = appel
                .strip_prefix('"')
                .and_then(|a| a.split_once("\",web::"))
                .unwrap_or_else(|| panic!("route illisible : {appel:.80}"));
            let (verbe, reste) = reste
                .split_once("().to(")
                .unwrap_or_else(|| panic!("verbe illisible : {reste:.80}"));
            let gestionnaire = reste.split(')').next().unwrap_or_default();
            (verbe.to_owned(), chemin.to_owned(), gestionnaire.to_owned())
        })
        .collect()
}

/// Chaque `pub(crate) async fn`, avec sa signature — jusqu'à l'accolade du
/// corps, là où les extracteurs se déclarent.
fn signatures(source: &str) -> Vec<(String, String)> {
    sans_commentaires(source)
        .split("pub(crate) async fn ")
        .skip(1)
        .map(|reste| {
            let nom = reste.split('(').next().unwrap_or_default().trim();
            let signature = reste.split('{').next().unwrap_or_default();
            (nom.to_owned(), signature.to_owned())
        })
        .collect()
}

/// Le corps compacté du bloc qui suit `entete`. Les accolades des chaînes ne
/// comptent pas : les chemins en portent, `{id}`.
fn bloc(source: &str, entete: &str) -> String {
    let code = compact(source);
    let debut = code
        .find(entete)
        .unwrap_or_else(|| panic!("{entete} absent du fichier"))
        + entete.len();
    let reste = &code[debut..];
    let ouverture = reste.find('{').expect("accolade ouvrante");
    let (mut profondeur, mut dans_une_chaine, mut precedent) = (0, false, ' ');
    for (i, c) in reste[ouverture..].char_indices() {
        match c {
            '"' if precedent != '\\' => dans_une_chaine = !dans_une_chaine,
            '{' if !dans_une_chaine => profondeur += 1,
            '}' if !dans_une_chaine => {
                profondeur -= 1;
                if profondeur == 0 {
                    return reste[ouverture + 1..ouverture + i].to_owned();
                }
            }
            _ => {}
        }
        precedent = c;
    }
    panic!("le bloc de {entete} ne se ferme pas")
}

const CONFIGURER: &str = "pubfnconfigurer(cfg:&mutweb::ServiceConfig)";
const ADMIN_ROUTES: &str = "pubfnadmin_routes(cfg:&mutServiceConfig)";
const QUATRE_FICHIERS: &str = "routes::admin_codes::configurer(cfg);\
     routes::admin_requests::configurer(cfg);\
     routes::admin_admission::configurer(cfg);\
     routes::admin_documents::configurer(cfg);";

/// Ce qui monte une porte sans passer par `.route(…)`.
const AUTRES_MONTAGES: [&str; 7] = [
    ".service(",
    "web::resource(",
    "web::scope(",
    "default_service(",
    ".configure(",
    "web::to(",
    "external_resource(",
];
const MACROS_DE_ROUTE: [&str; 11] = [
    "get", "post", "put", "patch", "delete", "head", "options", "trace", "connect", "route",
    "routes",
];

/// Tout ce qui, dans un fichier de routes, échapperait au compte des
/// `.route(…)`. Vide : le fichier ne monte que ses routes.
fn ecarts_de_montage(source: &str) -> Vec<String> {
    let code = compact(source);
    let mut ecarts: Vec<String> = AUTRES_MONTAGES
        .iter()
        .filter(|m| code.contains(**m))
        .map(|m| (*m).to_owned())
        .collect();
    for attribut in code.split("#[").skip(1) {
        let nom = attribut.split(['(', ']']).next().unwrap_or_default();
        if MACROS_DE_ROUTE.contains(&nom.trim_start_matches("actix_web::")) {
            ecarts.push(format!("#[{nom}]"));
        }
    }
    let points_de_montage = code.matches("ServiceConfig").count();
    if points_de_montage != 1 {
        ecarts.push(format!("{points_de_montage} ServiceConfig"));
    }
    let routes: String = routes_montees(source)
        .iter()
        .map(|(v, c, g)| format!(".route(\"{c}\",web::{v}().to({g}))"))
        .collect();
    // `rustfmt` laisse une virgule finale aux appels qu'il coupe.
    let configurer = bloc(source, CONFIGURER).replace(",)", ")");
    if configurer != format!("cfg{routes};") {
        ecarts.push(format!("configurer() : {configurer}"));
    }
    ecarts
}

#[test]
fn les_dix_sept_routes_des_documents_sont_montees_et_pas_une_de_plus() {
    let montees = routes_montees(ADMIN_DOCUMENTS);
    for (verbe, chemin, _) in ROUTES_DOCUMENTS {
        let fois = montees
            .iter()
            .filter(|(v, c, _)| v == verbe && c == chemin)
            .count();
        assert_eq!(fois, 1, "{verbe} {chemin} doit être montée une fois");
    }
    assert_eq!(
        montees.len(),
        17,
        "dix-sept routes, pas une de plus : {montees:?}"
    );
}

const CONTRAT: &str =
    include_str!("../../../../../specs/011-guide-nego-documents/contracts/api-admin-documents.md");

/// Les lignes `| \`VERBE /chemin\` | garde | … |` du contrat, garde traduite.
fn routes_du_contrat() -> Vec<(String, String, &'static str)> {
    CONTRAT
        .lines()
        .filter_map(|ligne| {
            let mut cellules = ligne.split('|').map(str::trim).skip(1);
            let route = cellules.next()?.strip_prefix('`')?.strip_suffix('`')?;
            let (verbe, chemin) = route.split_once(' ').filter(|(_, c)| c.starts_with('/'))?;
            let garde = match cellules.next()? {
                "publish **ou** correction.post" => LIRE,
                "publish" => PUBLIER,
                "correction.post" => POSER,
                "correction.withdraw" => RETIRER,
                autre => panic!("garde inconnue du contrat : {autre}"),
            };
            Some((verbe.to_lowercase(), chemin.to_owned(), garde))
        })
        .collect()
}

#[test]
fn la_table_des_routes_est_celle_du_contrat() {
    let mut contrat = routes_du_contrat();
    let mut table: Vec<_> = ROUTES_DOCUMENTS
        .iter()
        .map(|(v, c, g)| ((*v).to_owned(), (*c).to_owned(), *g))
        .collect();
    contrat.sort_unstable();
    table.sort_unstable();
    assert_eq!(table, contrat, "le contrat a changé : la table le suit");
}

#[test]
fn chaque_configurer_du_back_office_ne_monte_que_ses_routes() {
    for source in [
        ADMIN_CODES,
        ADMIN_DEMANDES,
        ADMIN_ADMISSION,
        ADMIN_DOCUMENTS,
    ] {
        let ecarts = ecarts_de_montage(source);
        assert!(
            ecarts.is_empty(),
            "une porte hors du compte des routes : {ecarts:?}"
        );
    }
    assert_eq!(
        bloc(LIB, ADMIN_ROUTES),
        QUATRE_FICHIERS,
        "le back-office ne se compose que des quatre fichiers contrôlés"
    );
}

/// Le contrôle voit-il ce qu'il prétend voir ? Chaque autre façon de monter une
/// porte, greffée sur le vrai fichier, doit être relevée.
#[test]
fn le_controle_releve_chaque_autre_maniere_de_monter_une_route() {
    const ANCRE: &str = "cfg.route(";
    const CACHE: &str =
        "\nasync fn cache() -> HttpResponse {\n    HttpResponse::Ok().finish()\n}\n";
    assert_eq!(ADMIN_DOCUMENTS.matches(ANCRE).count(), 1);
    assert!(ecarts_de_montage(ADMIN_DOCUMENTS).is_empty());

    let greffes: [(&str, String); 8] = [
        (
            "cfg.service(web::resource(\"/admin/negotiation/cache\").to(cache)).route(",
            CACHE.to_owned(),
        ),
        (
            "cfg.service(web::scope(\"/admin/negotiation\").route(\"/cache\", web::get().to(cache))).route(",
            CACHE.to_owned(),
        ),
        ("cfg.default_service(web::to(cache)).route(", CACHE.to_owned()),
        ("cfg.service(cache).route(", CACHE.to_owned()),
        ("cfg.configure(autres).route(", CACHE.to_owned()),
        (
            "autres(cfg);\n    cfg.route(",
            format!("{CACHE}fn autres(cfg: &mut web::ServiceConfig) {{\n    cfg.route(\"/admin/negotiation/cache\", web::get().to(cache));\n}}\n"),
        ),
        // Une macro d'un autre crate : seul le texte exact du `configurer()` la voit.
        ("kernel::monter_les_routes!(cfg);\n    cfg.route(", String::new()),
        // Une macro d'actix, montée depuis `lib.rs` : le fichier seul la trahit.
        (
            ANCRE,
            "\n#[actix_web::get(\"/admin/negotiation/cache\")]\npub async fn cache() -> HttpResponse {\n    HttpResponse::Ok().finish()\n}\n"
                .to_owned(),
        ),
    ];
    for (greffe, ajout) in greffes {
        let mutant = ADMIN_DOCUMENTS.replacen(ANCRE, greffe, 1) + &ajout;
        assert!(
            !ecarts_de_montage(&mutant).is_empty(),
            "greffe non relevée : {greffe} {ajout}"
        );
    }

    let lib_mutante = LIB.replacen(
        "routes::admin_documents::configurer(cfg);",
        "routes::admin_documents::configurer(cfg);\n    cfg.service(routes::admin_documents::cache);",
        1,
    );
    assert_ne!(lib_mutante, LIB);
    assert_ne!(bloc(&lib_mutante, ADMIN_ROUTES), QUATRE_FICHIERS);
}

#[test]
fn chaque_route_des_codes_mene_a_un_gestionnaire_garde() {
    for source in sources() {
        let signatures = signatures(source);
        for (verbe, chemin, gestionnaire) in routes_montees(source) {
            let (_, signature) = signatures
                .iter()
                .find(|(nom, _)| *nom == gestionnaire)
                .unwrap_or_else(|| {
                    panic!("{verbe} {chemin} : {gestionnaire} n'est pas un gestionnaire du fichier")
                });
            assert!(
                signature.contains("Requires<SpaceManage>"),
                "{verbe} {chemin} ({gestionnaire}) doit exiger Requires<SpaceManage>"
            );
        }
    }
}

#[test]
fn chaque_gestionnaire_des_documents_declare_la_garde_de_son_geste() {
    let signatures = signatures(ADMIN_DOCUMENTS);
    assert_eq!(signatures.len(), 17, "un gestionnaire par route");

    for (nom, signature) in &signatures {
        let gardes: Vec<_> = GARDES.iter().filter(|g| signature.contains(*g)).collect();
        assert_eq!(
            gardes.len(),
            1,
            "{nom} déclare une garde, une seule : {gardes:?}"
        );
    }

    let montees = routes_montees(ADMIN_DOCUMENTS);
    let mut montes: Vec<&str> = montees.iter().map(|(_, _, g)| g.as_str()).collect();
    let mut declares: Vec<&str> = signatures.iter().map(|(n, _)| n.as_str()).collect();
    montes.sort_unstable();
    declares.sort_unstable();
    assert_eq!(
        montes, declares,
        "chaque route mène à un gestionnaire gardé du fichier, et chacun est monté une fois"
    );

    for (verbe, chemin, garde) in ROUTES_DOCUMENTS {
        let (_, _, gestionnaire) = montees
            .iter()
            .find(|(v, c, _)| v == verbe && c == chemin)
            .unwrap_or_else(|| panic!("{verbe} {chemin} n'est pas montée"));
        let (_, signature) = signatures
            .iter()
            .find(|(nom, _)| nom == gestionnaire)
            .unwrap_or_else(|| panic!("{gestionnaire} n'est pas un gestionnaire du fichier"));
        assert!(
            signature.contains(garde),
            "{verbe} {chemin} ({gestionnaire}) doit exiger {garde}"
        );
    }
}

#[test]
fn aucune_route_des_documents_nemprunte_une_garde_plus_large() {
    let code = sans_commentaires(ADMIN_DOCUMENTS);
    // Le rôle `admin` porte `negotiation.document.publish` sur un événement, et
    // un document peut être rattaché à une COP : chacune de ces portes laisserait
    // entrer l'administrateur d'une seule édition.
    for piege in [
        "RequiresAnyScope",
        "Perimeter",
        "_anywhere",
        "administered_events",
    ] {
        assert!(!code.contains(piege), "le back-office emprunte {piege}");
    }
    assert!(
        code.match_indices("Scope::")
            .all(|(i, _)| code[i..].starts_with("Scope::Global")),
        "toute permission lue ici l'est sur la portée globale"
    );
    // La seule lecture de permission du fichier, sans laquelle la précédente
    // serait vraie d'un fichier vide : le droit de retirer, dans `notes`.
    assert!(bloc(ADMIN_DOCUMENTS, "pub(crate)asyncfnnotes(")
        .contains("CORRECTION_WITHDRAW,kernel::auth::Scope::Global"));

    // La garde des lectures délègue au service, que `perimetre_vide_refuse.rs`
    // éprouve sur base réelle — et c'est bien l'extracteur qui l'appelle, et
    // fait tomber son refus.
    let extracteur = bloc(ADMIN_DOCUMENTS, "implFromRequestforLectureDocuments");
    let appel = extracteur
        .split_once("service::exiger_la_lecture(")
        .map(|(_, suite)| suite)
        .expect("l'extracteur des lectures appelle exiger_la_lecture");
    assert!(
        appel
            .split_once(')')
            .is_some_and(|(_, suite)| suite.starts_with(".await?")),
        "le refus d'exiger_la_lecture remonte : {appel:.80}"
    );
}

/// L'en-tête qui tient lieu de session : l'application d'essai n'a pas
/// l'intergiciel de `api`, seulement ce qu'il pose — le contexte et son acteur.
const ACTEUR: &str = "x-essai-acteur";

/// Le back-office du module, monté seul dans une application d'essai.
macro_rules! back_office {
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
                .configure(negotiation::admin_routes),
        )
        .await
    };
}

async fn une_edition(bac: &Bac, slug: &str) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2027, jsonb_build_object('fr', 'COP31'), $1::text::platform.slug,
                   jsonb_build_object('fr', 'COP31'), 'online',
                   'America/Belem'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#,
    )
    .bind(slug)
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition")
}

/// Une page posée à la main : une note n'a besoin que de son ancre, et PDFium
/// ne se lie qu'une fois par binaire de test.
async fn une_page(bac: &Bac, document: Uuid) {
    sqlx::query(
        "INSERT INTO negotiation.document_pages (document_id, page_index, label)
         VALUES ($1, 1, '1')",
    )
    .bind(document)
    .execute(bac.pool())
    .await
    .expect("page du document");
}

/// Ce que chaque adresse désigne : un document, une note, un PDF déposé.
struct Cibles {
    document: Uuid,
    note: Uuid,
    asset: Uuid,
}

impl Cibles {
    fn forgees() -> Self {
        Self {
            document: Uuid::now_v7(),
            note: Uuid::now_v7(),
            asset: Uuid::now_v7(),
        }
    }
}

fn chemin(motif: &str, cibles: &Cibles) -> String {
    motif
        .replace("{id}", &cibles.document.to_string())
        .replace("{note_id}", &cibles.note.to_string())
        .replace("{index}", "1")
}

/// Un corps bien formé pour chaque écriture : le refus doit venir de la garde,
/// pas du décodage.
fn corps(verbe: &str, motif: &str, cibles: &Cibles) -> Option<Value> {
    match (verbe, motif) {
        ("post", "/admin/negotiation/documents")
        | ("patch", "/admin/negotiation/documents/{id}") => {
            Some(json!({ "title": { "fr": "Forgé" }, "type": "negotiation_guide" }))
        }
        ("put", "/admin/negotiation/documents/{id}/file") => {
            Some(json!({ "asset_id": cibles.asset }))
        }
        ("put", "/admin/negotiation/documents/{id}/as-is") => Some(json!({ "serve_as_is": true })),
        ("post", "/admin/negotiation/documents/{id}/corrections") => {
            Some(json!({ "page_index": 1, "body": { "fr": "Forgée" } }))
        }
        _ => None,
    }
}

fn appel(verbe: &str, uri: &str, acteur: Option<Uuid>, corps: Option<Value>) -> TestRequest {
    let methode = Method::from_bytes(verbe.to_uppercase().as_bytes()).expect("verbe HTTP");
    let mut r = TestRequest::default().method(methode).uri(uri);
    if let Some(a) = acteur {
        r = r.insert_header((ACTEUR, a.to_string()));
    }
    if let Some(c) = corps {
        r = r.set_json(c);
    }
    r
}

fn requete(verbe: &str, motif: &str, cibles: &Cibles, acteur: Option<Uuid>) -> TestRequest {
    appel(
        verbe,
        &chemin(motif, cibles),
        acteur,
        corps(verbe, motif, cibles),
    )
}

async fn frapper<S, R, B>(app: &S, requete: R) -> (StatusCode, Value)
where
    S: Service<R, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let reponse = call_service(app, requete).await;
    let statut = reponse.status();
    let octets = read_body(reponse).await;
    (
        statut,
        serde_json::from_slice(&octets).unwrap_or(Value::Null),
    )
}

/// Ce qu'une écriture laisserait : documents, publiés, notes, travaux, traces.
async fn ecritures(bac: &Bac) -> (i64, i64, i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM negotiation.documents),
                (SELECT count(*) FROM negotiation.documents WHERE published_at IS NOT NULL),
                (SELECT count(*) FROM negotiation.correction_notes),
                (SELECT count(*) FROM platform.jobs),
                (SELECT count(*) FROM platform.audit_log)",
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture des compteurs")
}

#[tokio::test]
async fn chaque_route_des_documents_refuse_ladministrateur_dune_edition_meme_sur_sa_cop() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let admin_cop = personne(&bac, "admin.cop31@example.org").await;
    let edition = une_edition(&bac, "cop31-documents").await;
    attribuer(&bac, admin_cop, "admin", "event", Some(edition)).await;

    assert!(
        kernel::auth::has_permission(
            bac.pool(),
            admin_cop,
            DOCUMENT_PUBLISH,
            Scope::Event(edition)
        )
        .await
        .expect("lecture de la permission"),
        "sur son édition, il publie — c'est le piège"
    );

    // Un vrai brouillon, rattaché à SA COP : l'identifiant qu'il forgerait avec
    // le plus de vraisemblance.
    let mut e = entree("Guide de la COP31", false);
    e.cop = Some(Some(edition));
    let brouillon = admin_documents::creer(&bac.state, &bac.ctx(ifdd), &e, "fr")
        .await
        .expect("création du brouillon");
    let cop: Option<Uuid> =
        sqlx::query_scalar("SELECT event_id FROM negotiation.documents WHERE id = $1")
            .bind(brouillon)
            .fetch_one(bac.pool())
            .await
            .expect("relecture du brouillon");
    assert_eq!(cop, Some(edition), "le brouillon est bien celui de sa COP");

    // Sa page 1, une note d'experte posée dessus, un PDF déposé : au premier
    // passage, chaque identifiant de l'adresse et du corps existe.
    une_page(&bac, brouillon).await;
    let relectrice = expert(&bac, "experte@example.org").await;
    let note = corrections::poser(
        &bac.state,
        &bac.ctx(relectrice),
        brouillon,
        &CorrectionNoteInput {
            page_index: 1,
            passage: None,
            body: json!({ "fr": "Chiffre dépassé." }),
        },
    )
    .await
    .expect("note posée");
    let reelles = Cibles {
        document: brouillon,
        note: note.id,
        asset: objet_pdf(&bac, ifdd, PETIT, "ready").await,
    };

    let app = back_office!(bac);

    let avant = ecritures(&bac).await;
    for (verbe, motif, _) in ROUTES_DOCUMENTS {
        for cibles in [&reelles, &Cibles::forgees()] {
            let uri = chemin(motif, cibles);

            let (statut, corps) = frapper(
                &app,
                requete(verbe, motif, cibles, Some(admin_cop)).to_request(),
            )
            .await;
            assert_eq!(
                (statut, corps["code"].as_str()),
                (StatusCode::FORBIDDEN, Some("FORBIDDEN")),
                "{verbe} {uri} : l'administrateur de la COP31 ne passe pas"
            );

            let (statut, corps) =
                frapper(&app, requete(verbe, motif, cibles, None).to_request()).await;
            assert_eq!(
                (statut, corps["code"].as_str()),
                (StatusCode::UNAUTHORIZED, Some("UNAUTHENTICATED")),
                "{verbe} {uri} sans session"
            );
        }
    }
    assert_eq!(ecritures(&bac).await, avant, "aucun refus n'a écrit");
    let retrait: Option<Uuid> =
        sqlx::query_scalar("SELECT withdrawn_by FROM negotiation.correction_notes WHERE id = $1")
            .bind(note.id)
            .fetch_one(bac.pool())
            .await
            .expect("relecture de la note");
    assert_eq!(retrait, None, "la note visée n'a pas été retirée");

    // Témoins : la même application, avec l'administratrice de la plateforme.
    // L'adresse forgée atteint alors la lecture, et répond 404 : le 403 venait
    // donc de la garde, avant toute lecture.
    let forgees = Cibles::forgees();
    let liste = "/admin/negotiation/documents";
    let (statut, corps) = frapper(
        &app,
        requete("get", liste, &forgees, Some(ifdd)).to_request(),
    )
    .await;
    assert_eq!(statut, StatusCode::OK);
    assert!(corps["documents"]
        .as_array()
        .expect("liste des documents")
        .iter()
        .any(|d| d["id"] == json!(brouillon)));

    let fiche = "/admin/negotiation/documents/{id}";
    let (statut, corps) = frapper(
        &app,
        requete("get", fiche, &forgees, Some(ifdd)).to_request(),
    )
    .await;
    assert_eq!(
        (statut, corps["code"].as_str()),
        (
            StatusCode::NOT_FOUND,
            Some("NEGOTIATION_DOCUMENT_NOT_FOUND")
        )
    );
}
