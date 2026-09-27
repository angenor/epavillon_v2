//! **Un administrateur d'événement ne passe aucune route du back-office des
//! documents** — `contracts/api-admin-documents.md`, SC-008, principe V.
//!
//! Les dix-sept routes sont gardées chacune par la permission de son geste —
//! publier, poser une note, la retirer — ou, pour lire, par publier **ou**
//! corriger ; toujours sur la portée globale. Le rôle `admin` porte
//! `negotiation.document.publish`, et un document peut être rattaché à une COP :
//! elles sont contrôlées dans leur source, puis **frappées une à une**. Voir
//! l'en-tête de `perimetre_url_forgee.rs`.

mod commun;

use actix_web::http::StatusCode;
use actix_web::test::TestRequest;
use commun::documents::{administratrice, entree, expert, objet_pdf, PETIT};
use commun::http::{appel, frapper, une_edition};
use commun::sources::{bloc, routes_montees, sans_commentaires, signatures};
use commun::{attribuer, personne, Bac};
use kernel::auth::Scope;
use negotiation::domain::admin_documents::CorrectionNoteInput;
use negotiation::domain::permissions::DOCUMENT_PUBLISH;
use negotiation::service::{admin_documents, corrections};
use serde_json::{json, Value};
use uuid::Uuid;

const ADMIN_DOCUMENTS: &str = include_str!("../src/routes/admin_documents.rs");

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
    (
        "put",
        "/admin/negotiation/documents/{id}/large-text",
        PUBLIER,
    ),
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
        ("put", "/admin/negotiation/documents/{id}/large-text") => Some(json!({ "choice": false })),
        ("post", "/admin/negotiation/documents/{id}/corrections") => {
            Some(json!({ "page_index": 1, "body": { "fr": "Forgée" } }))
        }
        _ => None,
    }
}

fn requete(verbe: &str, motif: &str, cibles: &Cibles, acteur: Option<Uuid>) -> TestRequest {
    appel(
        verbe,
        &chemin(motif, cibles),
        acteur,
        corps(verbe, motif, cibles),
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
    let brouillon = admin_documents::creer(&bac.state, &bac.ctx(ifdd), &e)
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

    let app = crate::back_office!(bac);

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
