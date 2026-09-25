//! **Un administrateur d'événement ne passe aucune route du back-office du
//! savoir** — `contracts/api-admin-savoir.md`, principe V.
//!
//! Les vingt-cinq routes de la FAQ, du lexique et du parcours sont gardées par
//! la permission de leur geste — publier, ou dater une vérification — ou, pour
//! lire, par l'une **ou** l'autre ; toujours sur la portée globale. Le rôle
//! `admin` porte `negotiation.knowledge.publish` et s'attribue aussi sur un
//! événement : chaque route est contrôlée dans sa source, puis frappée.

mod commun;

use actix_web::http::StatusCode;
use actix_web::test::TestRequest;
use commun::documents::{administratrice, expert};
use commun::http::{appel, frapper, une_edition};
use commun::savoir::{entree_faq, etape, terme};
use commun::sources::{bloc, routes_montees, sans_commentaires, signatures};
use commun::{attribuer, personne, Bac};
use kernel::auth::Scope;
use negotiation::domain::permissions::KNOWLEDGE_PUBLISH;
use serde_json::{json, Value};
use uuid::Uuid;

const ADMIN_SAVOIR: &str = include_str!("../src/routes/admin_savoir.rs");
const CONTRAT: &str =
    include_str!("../../../../../specs/013-guide-nego-faq-lexique/contracts/api-admin-savoir.md");

const LIRE: &str = "LectureSavoir";
const PUBLIER: &str = "Requires<KnowledgePublish>";
const VERIFIER: &str = "Requires<KnowledgeReview>";
const GARDES: [&str; 3] = [LIRE, PUBLIER, VERIFIER];

const ROUTES_SAVOIR: [(&str, &str, &str); 25] = [
    ("get", "/admin/negotiation/faq", LIRE),
    ("post", "/admin/negotiation/faq", PUBLIER),
    ("get", "/admin/negotiation/faq/{id}", LIRE),
    ("patch", "/admin/negotiation/faq/{id}", PUBLIER),
    ("delete", "/admin/negotiation/faq/{id}", PUBLIER),
    ("post", "/admin/negotiation/faq/{id}/verify", VERIFIER),
    ("post", "/admin/negotiation/faq/{id}/publish", PUBLIER),
    ("post", "/admin/negotiation/faq/{id}/to-review", PUBLIER),
    ("post", "/admin/negotiation/faq/{id}/unpublish", PUBLIER),
    ("get", "/admin/negotiation/glossary", LIRE),
    ("post", "/admin/negotiation/glossary", PUBLIER),
    ("get", "/admin/negotiation/glossary/{id}", LIRE),
    ("patch", "/admin/negotiation/glossary/{id}", PUBLIER),
    ("delete", "/admin/negotiation/glossary/{id}", PUBLIER),
    ("post", "/admin/negotiation/glossary/{id}/publish", PUBLIER),
    (
        "post",
        "/admin/negotiation/glossary/{id}/to-review",
        PUBLIER,
    ),
    (
        "post",
        "/admin/negotiation/glossary/{id}/unpublish",
        PUBLIER,
    ),
    ("get", "/admin/negotiation/pathway", LIRE),
    ("post", "/admin/negotiation/pathway/groups", PUBLIER),
    (
        "patch",
        "/admin/negotiation/pathway/groups/{group}",
        PUBLIER,
    ),
    (
        "delete",
        "/admin/negotiation/pathway/groups/{group}",
        PUBLIER,
    ),
    ("post", "/admin/negotiation/pathway/steps", PUBLIER),
    ("patch", "/admin/negotiation/pathway/steps/{step}", PUBLIER),
    ("delete", "/admin/negotiation/pathway/steps/{step}", PUBLIER),
    ("put", "/admin/negotiation/pathway/order", PUBLIER),
];

/// Le chemin tel que le fichier de routes l'écrit : `{id}` partout.
fn monte(chemin: &str) -> String {
    chemin.replace("{group}", "{id}").replace("{step}", "{id}")
}

#[test]
fn les_vingt_cinq_routes_du_savoir_sont_montees_et_pas_une_de_plus() {
    let montees = routes_montees(ADMIN_SAVOIR);
    for (verbe, chemin, _) in ROUTES_SAVOIR {
        let chemin = monte(chemin);
        let fois = montees
            .iter()
            .filter(|(v, c, _)| v == verbe && *c == chemin)
            .count();
        assert_eq!(fois, 1, "{verbe} {chemin} doit être montée une fois");
    }
    assert_eq!(montees.len(), 25, "vingt-cinq routes : {montees:?}");
}

/// Les lignes de la table FAQ du contrat, garde traduite. Le lexique et le
/// parcours y sont écrits en prose : la table ci-dessus en tient lieu.
#[test]
fn la_table_de_la_faq_est_celle_du_contrat() {
    let mut contrat: Vec<(String, String, &str)> = CONTRAT
        .lines()
        .filter_map(|ligne| {
            let mut cellules = ligne.split('|').map(str::trim).skip(1);
            let route = cellules.next()?.strip_prefix('`')?.strip_suffix('`')?;
            let (verbe, chemin) = route.split_once(' ')?;
            if !chemin.starts_with("/admin/negotiation/faq") {
                return None;
            }
            let garde = match cellules.next()? {
                "publish ou review" => LIRE,
                "publish" => PUBLIER,
                "review" => VERIFIER,
                autre => panic!("garde inconnue du contrat : {autre}"),
            };
            Some((verbe.to_lowercase(), chemin.to_owned(), garde))
        })
        .collect();
    let mut table: Vec<_> = ROUTES_SAVOIR
        .iter()
        .filter(|(_, c, _)| c.starts_with("/admin/negotiation/faq"))
        .map(|(v, c, g)| ((*v).to_owned(), (*c).to_owned(), *g))
        .collect();
    contrat.sort_unstable();
    table.sort_unstable();
    assert_eq!(table, contrat);
}

#[test]
fn chaque_gestionnaire_du_savoir_declare_la_garde_de_son_geste() {
    let signatures = signatures(ADMIN_SAVOIR);
    assert_eq!(signatures.len(), 25, "un gestionnaire par route");
    for (nom, signature) in &signatures {
        let gardes: Vec<_> = GARDES.iter().filter(|g| signature.contains(*g)).collect();
        assert_eq!(
            gardes.len(),
            1,
            "{nom} déclare une garde, une seule : {gardes:?}"
        );
    }
    let montees = routes_montees(ADMIN_SAVOIR);
    for (verbe, chemin, garde) in ROUTES_SAVOIR {
        let chemin = monte(chemin);
        let (_, _, gestionnaire) = montees
            .iter()
            .find(|(v, c, _)| v == verbe && *c == chemin)
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
fn aucune_route_du_savoir_nemprunte_une_garde_plus_large() {
    let code = sans_commentaires(ADMIN_SAVOIR);
    for piege in [
        "RequiresAnyScope",
        "Perimeter",
        "_anywhere",
        "administered_events",
        "Scope::",
    ] {
        assert!(!code.contains(piege), "le back-office emprunte {piege}");
    }
    let extracteur = bloc(ADMIN_SAVOIR, "implFromRequestforLectureSavoir");
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

struct Cibles {
    faq: Uuid,
    terme: Uuid,
    groupe: Uuid,
    etape: Uuid,
}

impl Cibles {
    fn forgees() -> Self {
        Self {
            faq: Uuid::now_v7(),
            terme: Uuid::now_v7(),
            groupe: Uuid::now_v7(),
            etape: Uuid::now_v7(),
        }
    }
}

fn chemin(motif: &str, c: &Cibles) -> String {
    let id = if motif.contains("/glossary/") {
        c.terme
    } else {
        c.faq
    };
    motif
        .replace("{id}", &id.to_string())
        .replace("{group}", &c.groupe.to_string())
        .replace("{step}", &c.etape.to_string())
}

fn corps(verbe: &str, motif: &str, c: &Cibles) -> Option<Value> {
    let texte = json!({ "fr": "Forgé" });
    match (verbe, motif) {
        ("post" | "patch", "/admin/negotiation/faq" | "/admin/negotiation/faq/{id}") => {
            Some(json!({ "section_code": "first_cop", "question": texte }))
        }
        ("post" | "patch", "/admin/negotiation/glossary" | "/admin/negotiation/glossary/{id}") => {
            Some(json!({ "family_code": "meetings", "term": "forged term",
                         "translation": texte, "definition": texte }))
        }
        ("post", "/admin/negotiation/faq/{id}/verify") => Some(json!({})),
        ("post" | "patch", m) if m.starts_with("/admin/negotiation/pathway/groups") => {
            Some(json!({ "label": texte, "is_published": true }))
        }
        ("post" | "patch", m) if m.starts_with("/admin/negotiation/pathway/steps") => {
            Some(json!({ "group_id": c.groupe, "label": texte }))
        }
        ("put", _) => Some(json!({ "groups": [{ "id": c.groupe, "step_ids": [c.etape] }] })),
        _ => None,
    }
}

fn requete(verbe: &str, motif: &str, c: &Cibles, acteur: Option<Uuid>) -> TestRequest {
    appel(verbe, &chemin(motif, c), acteur, corps(verbe, motif, c))
}

/// Ce qu'une écriture laisserait : entrées, publiées, étapes, traces.
async fn ecritures(bac: &Bac) -> (i64, i64, i64, i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM negotiation.faq_entries),
                (SELECT count(*) FROM negotiation.faq_entries WHERE status <> 'draft'),
                (SELECT count(*) FROM negotiation.glossary_entries),
                (SELECT count(*) FROM negotiation.pathway_steps),
                (SELECT count(*) FROM platform.audit_log)",
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture des compteurs")
}

#[tokio::test]
async fn chaque_route_du_savoir_refuse_ladministrateur_dune_edition_et_le_compte_sans_role() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let experte = expert(&bac, "experte@example.org").await;
    let admin_cop = personne(&bac, "admin.cop31@example.org").await;
    let sans_role = personne(&bac, "sans.role@example.org").await;
    let edition = une_edition(&bac, "cop31-savoir").await;
    attribuer(&bac, admin_cop, "admin", "event", Some(edition)).await;
    assert!(
        kernel::auth::has_permission(
            bac.pool(),
            admin_cop,
            KNOWLEDGE_PUBLISH,
            Scope::Event(edition)
        )
        .await
        .expect("lecture de la permission"),
        "sur son édition, il publie — c'est le piège"
    );

    let (groupe, etape_id) = etape(&bac, "Avant de partir", "Lire le guide").await;
    let reelles = Cibles {
        faq: entree_faq(
            &bac,
            experte,
            "Qu'est-ce qu'un groupe de contact ?",
            "draft",
        )
        .await,
        terme: terme(&bac, "contact group", "draft", None, &[]).await,
        groupe,
        etape: etape_id,
    };

    let app = crate::back_office!(bac);
    let avant = ecritures(&bac).await;
    for (verbe, motif, _) in ROUTES_SAVOIR {
        for cibles in [&reelles, &Cibles::forgees()] {
            let uri = chemin(motif, cibles);
            for qui in [admin_cop, sans_role] {
                let (statut, corps) =
                    frapper(&app, requete(verbe, motif, cibles, Some(qui)).to_request()).await;
                assert_eq!(
                    (statut, corps["code"].as_str()),
                    (StatusCode::FORBIDDEN, Some("FORBIDDEN")),
                    "{verbe} {uri} : {qui} ne passe pas"
                );
            }
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

    // Témoin : l'administratrice de la plateforme atteint la lecture, et
    // l'adresse forgée y rend 404 — le 403 venait donc de la garde.
    let forgees = Cibles::forgees();
    let (statut, corps) = frapper(
        &app,
        requete("get", "/admin/negotiation/faq/{id}", &forgees, Some(ifdd)).to_request(),
    )
    .await;
    assert_eq!(
        (statut, corps["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("NEGOTIATION_FAQ_NOT_FOUND"))
    );
    let (statut, corps) = frapper(
        &app,
        requete("get", "/admin/negotiation/faq", &forgees, Some(ifdd)).to_request(),
    )
    .await;
    assert_eq!(statut, StatusCode::OK);
    assert!(corps["entries"]
        .as_array()
        .expect("liste de la FAQ")
        .iter()
        .any(|e| e["id"] == json!(reelles.faq)));
}

// ---------------------------------------------------------------------------
// La file des experts : `Requires<KnowledgeReview>`, et rien d'autre
// ---------------------------------------------------------------------------

const ADMIN_FILE: &str = include_str!("../src/routes/admin_file.rs");

const ROUTES_FILE: [(&str, &str); 2] = [
    ("get", "/admin/negotiation/queue"),
    ("post", "/admin/negotiation/queue/reports/{id}/close"),
];

#[test]
fn chaque_route_de_la_file_exige_de_verifier() {
    let montees = routes_montees(ADMIN_FILE);
    assert_eq!(montees.len(), ROUTES_FILE.len(), "{montees:?}");
    let signatures = signatures(ADMIN_FILE);
    for (verbe, chemin) in ROUTES_FILE {
        let (_, _, gestionnaire) = montees
            .iter()
            .find(|(v, c, _)| v == verbe && c == chemin)
            .unwrap_or_else(|| panic!("{verbe} {chemin} n'est pas montée"));
        let (_, signature) = signatures
            .iter()
            .find(|(nom, _)| nom == gestionnaire)
            .expect("gestionnaire du fichier");
        assert!(
            signature.contains(VERIFIER)
                && !signature.contains(PUBLIER)
                && !signature.contains(LIRE),
            "{verbe} {chemin} doit exiger {VERIFIER}, seul"
        );
    }
    let code = sans_commentaires(ADMIN_FILE);
    for piege in ["RequiresAnyScope", "Perimeter", "_anywhere", "Scope::"] {
        assert!(!code.contains(piege), "la file emprunte {piege}");
    }
}

#[tokio::test]
async fn la_file_refuse_ladministrateur_dune_edition_la_publieuse_et_le_compte_sans_role() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let experte = expert(&bac, "experte@example.org").await;
    let admin_cop = personne(&bac, "admin.cop31@example.org").await;
    let sans_role = personne(&bac, "sans.role@example.org").await;
    let edition = une_edition(&bac, "cop31-file").await;
    attribuer(&bac, admin_cop, "admin", "event", Some(edition)).await;

    let faq = entree_faq(&bac, experte, "Qui préside ?", "published").await;
    let reel: Uuid = sqlx::query_scalar(
        "INSERT INTO negotiation.faq_reports (entry_id, reporter_id, client_ref, reasons)
         VALUES ($1, $2, gen_random_uuid(), ARRAY['wrong']) RETURNING id",
    )
    .bind(faq)
    .bind(sans_role)
    .fetch_one(bac.pool())
    .await
    .expect("signalement");

    let app = crate::back_office!(bac);
    for (verbe, motif) in ROUTES_FILE {
        for id in [reel, Uuid::now_v7()] {
            let uri = motif.replace("{id}", &id.to_string());
            let corps = (verbe == "post").then(|| json!({ "outcome": "dismissed" }));
            for qui in [admin_cop, ifdd, sans_role] {
                let (statut, r) = frapper(
                    &app,
                    appel(verbe, &uri, Some(qui), corps.clone()).to_request(),
                )
                .await;
                assert_eq!(
                    (statut, r["code"].as_str()),
                    (StatusCode::FORBIDDEN, Some("FORBIDDEN")),
                    "{verbe} {uri} : {qui} ne passe pas"
                );
            }
            let (statut, _) = frapper(&app, appel(verbe, &uri, None, corps).to_request()).await;
            assert_eq!(
                statut,
                StatusCode::UNAUTHORIZED,
                "{verbe} {uri} sans session"
            );
        }
    }
    let ouvert: String =
        sqlx::query_scalar("SELECT status::text FROM negotiation.faq_reports WHERE id = $1")
            .bind(reel)
            .fetch_one(bac.pool())
            .await
            .unwrap();
    assert_eq!(ouvert, "open", "aucun refus n'a clos");
}
