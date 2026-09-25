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
//! # LES AUTRES FICHIERS DU BACK-OFFICE
//!
//! Les documents (`perimetre_url_forgee_documents.rs`) et le savoir
//! (`perimetre_url_forgee_savoir.rs`) ont chacun leurs gardes de geste, et le
//! même piège : le rôle `admin` porte leurs permissions sur un événement. Ils
//! sont contrôlés dans leur source, puis **frappés une à une** : le back-office
//! du module se monte seul dans une application d'essai, sans `api`, un
//! intergiciel posant l'acteur à la place de la session.
//!
//! Compter les `.route(…)` ne suffit pas : un `.service`, un scope, un
//! `default_service` ou une macro monteraient une porte hors du compte. Chaque
//! `configurer()` doit donc être **exactement** la suite de ses routes. Que qui
//! détient le geste passe chaque garde est éprouvé dans
//! `perimetre_chaque_geste_a_son_detenteur.rs`.

mod commun;

use commun::sources::{bloc, ecarts_de_montage, routes_montees, sans_commentaires, signatures};
use commun::{administre_guide_nego, attribuer, decor, Bac};
use kernel::auth::Scope;
use negotiation::domain::permissions::SPACE_MANAGE;

const ADMIN_CODES: &str = include_str!("../src/routes/admin_codes.rs");
const ADMIN_DEMANDES: &str = include_str!("../src/routes/admin_requests.rs");
const ADMIN_ADMISSION: &str = include_str!("../src/routes/admin_admission.rs");
/// Même garde : l'import et l'ordre du jour (3a), éprouvés dans `perimetre_import.rs`.
const ADMIN_IMPORT: &str = include_str!("../src/routes/admin_import.rs");

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

fn sources() -> [&'static str; 4] {
    [ADMIN_CODES, ADMIN_DEMANDES, ADMIN_ADMISSION, ADMIN_IMPORT]
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
    assert_eq!(
        montees, 17,
        "douze routes et les cinq de l'import, pas une de plus"
    );
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
// Ce que le back-office monte, et rien d'autre
// ---------------------------------------------------------------------------

const ADMIN_DOCUMENTS: &str = include_str!("../src/routes/admin_documents.rs");
const ADMIN_SAVOIR: &str = include_str!("../src/routes/admin_savoir.rs");
const ADMIN_FILE: &str = include_str!("../src/routes/admin_file.rs");
const LIB: &str = include_str!("../src/lib.rs");

const ADMIN_ROUTES: &str = "pubfnadmin_routes(cfg:&mutServiceConfig)";
const FICHIERS_CONTROLES: &str = "routes::admin_codes::configurer(cfg);\
     routes::admin_requests::configurer(cfg);\
     routes::admin_admission::configurer(cfg);\
     routes::admin_documents::configurer(cfg);\
     routes::admin_savoir::configurer(cfg);\
     routes::admin_file::configurer(cfg);\
     routes::admin_import::configurer(cfg);";

#[test]
fn chaque_configurer_du_back_office_ne_monte_que_ses_routes() {
    for source in [
        ADMIN_CODES,
        ADMIN_DEMANDES,
        ADMIN_ADMISSION,
        ADMIN_DOCUMENTS,
        ADMIN_SAVOIR,
        ADMIN_FILE,
        ADMIN_IMPORT,
    ] {
        let ecarts = ecarts_de_montage(source);
        assert!(
            ecarts.is_empty(),
            "une porte hors du compte des routes : {ecarts:?}"
        );
    }
    assert_eq!(
        bloc(LIB, ADMIN_ROUTES),
        FICHIERS_CONTROLES,
        "le back-office ne se compose que des sept fichiers contrôlés"
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
        "routes::admin_savoir::configurer(cfg);",
        "routes::admin_savoir::configurer(cfg);\n    cfg.service(routes::admin_savoir::cache);",
        1,
    );
    assert_ne!(lib_mutante, LIB);
    assert_ne!(bloc(&lib_mutante, ADMIN_ROUTES), FICHIERS_CONTROLES);
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
