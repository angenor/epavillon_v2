//! **La forme lisible servie au téléphone** : ses pages et son sommaire dans la
//! grammaire de `contracts/forme-lisible.md`, le choix « Texte agrandi », et
//! l'empreinte qui dit si la copie gardée est la bonne — par les services, sur
//! base réelle. Le PDF lui-même a ses tests dans `documents_fichier.rs`.

mod commun;

use commun::documents::{
    administratrice, demander_lextraction, fichier_publie, passer_lextraction,
};
use commun::Bac;
use negotiation::domain::documents::DocumentReading;
use negotiation::service::{admin_documents, documents as public};
use serde_json::Value;
use uuid::Uuid;

// -----------------------------------------------------------------------------
// Les aides propres à ce lot
// -----------------------------------------------------------------------------

async fn lire(bac: &Bac, id: Uuid) -> (DocumentReading, String) {
    let (lecture, empreinte, _) = public::lecture(&bac.state, None, id)
        .await
        .expect("lecture");
    (lecture, empreinte)
}

/// Ce que la liste annonce de la forme lisible : texte, « Texte agrandi », empreinte.
async fn annonce(bac: &Bac, id: Uuid) -> (bool, bool, Option<String>) {
    let (bib, _) = public::bibliotheque(&bac.state, None, "fr")
        .await
        .expect("bibliothèque");
    let d = bib
        .documents
        .into_iter()
        .find(|d| d.id == id)
        .expect("document listé");
    (d.has_text, d.large_text, d.reading_etag)
}

async fn choisir(bac: &Bac, admin: Uuid, id: Uuid, choix: Option<bool>) {
    admin_documents::choisir_le_texte_agrandi(&bac.state, &bac.ctx(admin), id, choix)
        .await
        .expect("choix « Texte agrandi »");
}

/// Ce que la lecture offre : texte, « Texte agrandi ».
async fn modes(bac: &Bac, id: Uuid) -> (bool, bool) {
    let (lecture, _) = lire(bac, id).await;
    (lecture.has_text, lecture.large_text)
}

/// Le texte de chaque page, tel que la recherche l'indexe.
async fn textes_en_base(bac: &Bac, id: Uuid) -> Vec<(i32, String)> {
    sqlx::query_as::<_, (i32, String)>(
        "SELECT page_index, plain_text FROM negotiation.document_pages
          WHERE document_id = $1 ORDER BY page_index",
    )
    .bind(id)
    .fetch_all(bac.pool())
    .await
    .expect("textes des pages")
}

async fn fichier_du_document(bac: &Bac, id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT asset_id FROM negotiation.documents WHERE id = $1")
        .bind(id)
        .fetch_one(bac.pool())
        .await
        .expect("fichier du document")
}

fn a_plat(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn blocs(page: &negotiation::domain::documents::ReadingPage) -> &Vec<Value> {
    page.blocks.as_array().expect("les blocs, en tableau")
}

fn est_origine(bloc: &Value) -> bool {
    bloc["kind"] == "origin"
}

/// Le texte d'un bloc : ses segments, ou le texte replié d'un bloc d'origine.
fn texte_du_bloc(bloc: &Value) -> String {
    let segments = if est_origine(bloc) {
        &bloc["text"]
    } else {
        &bloc["spans"]
    };
    segments
        .as_array()
        .map(|s| s.iter().filter_map(|s| s["text"].as_str()).collect())
        .unwrap_or_default()
}

/// Un bloc de la grammaire close : un type connu, ses champs, et des segments
/// dont `term` implique `italic`.
fn bloc_conforme(bloc: &Value) -> bool {
    let segments = |v: &Value| {
        v.as_array().is_some_and(|s| {
            !s.is_empty()
                && s.iter()
                    .all(|s| s["text"].is_string() && (s["term"] != true || s["italic"] == true))
        })
    };
    match bloc["kind"].as_str() {
        Some("heading") => {
            (1..=3).contains(&bloc["level"].as_u64().unwrap_or(0)) && segments(&bloc["spans"])
        }
        Some("paragraph") => segments(&bloc["spans"]),
        Some("list_item") => {
            bloc["depth"].as_u64().is_some_and(|d| d <= 2)
                && bloc["marker"].is_string()
                && segments(&bloc["spans"])
        }
        Some("note") => bloc["mark"].is_string() && segments(&bloc["spans"]),
        Some("origin") => matches!(bloc["reason"].as_str(), Some("table" | "figure")),
        _ => false,
    }
}

/// Les entrées du sommaire, à plat : titre, niveau, page.
fn entrees(sommaire: &Value, acc: &mut Vec<(String, u64, i64)>) {
    for e in sommaire.as_array().expect("le sommaire, en tableau") {
        acc.push((
            e["title"].as_str().expect("un titre").to_owned(),
            e["level"].as_u64().expect("un niveau"),
            e["page_index"].as_i64().expect("une page"),
        ));
        entrees(&e["children"], acc);
    }
}

// -----------------------------------------------------------------------------
// La forme lisible
// -----------------------------------------------------------------------------

#[tokio::test]
async fn la_lecture_rend_la_forme_lisible_et_une_empreinte_figee() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;

    let (lecture, empreinte, reserve) = public::lecture(&bac.state, None, guide)
        .await
        .expect("lecture");
    assert!(!reserve);
    assert_eq!(lecture.id, guide);
    assert_eq!(lecture.version, "1");
    assert!(lecture.has_text, "le petit PDF a du texte");
    assert!(lecture.large_text, "et son verdict le dit recomposable");
    assert_eq!(lecture.page_count, 4);
    let index: Vec<i32> = lecture.pages.iter().map(|p| p.index).collect();
    assert_eq!(index, [1, 2, 3, 4], "des pages contiguës de 1 à page_count");
    assert!(lecture.pages.iter().all(|p| !p.label.is_empty()));

    let textes = textes_en_base(&bac, guide).await;
    assert_eq!(textes.len(), 4);
    for (page, (i, texte)) in lecture.pages.iter().zip(&textes) {
        assert_eq!(page.index, *i);
        let blocs = blocs(page);
        assert!(!blocs.is_empty(), "page {i} : des blocs");
        assert!(
            blocs.iter().all(bloc_conforme),
            "page {i} : la grammaire close — {blocs:?}"
        );
        assert!(!texte.trim().is_empty(), "page {i} : du texte");
        let affiche: Vec<String> = blocs
            .iter()
            .map(texte_du_bloc)
            .filter(|t| !t.is_empty())
            .collect();
        assert_eq!(
            a_plat(&affiche.join(" ")),
            a_plat(texte),
            "page {i} : rien ne se cherche qui ne s'affiche pas"
        );
    }
    assert!(
        lecture
            .pages
            .iter()
            .any(|p| blocs(p).iter().any(est_origine)),
        "le petit PDF porte un tableau"
    );
    let servie = serde_json::to_value(&lecture).expect("sérialisation");
    for page in servie["pages"].as_array().unwrap() {
        let mut cles: Vec<&str> = page
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        cles.sort_unstable();
        assert_eq!(
            cles,
            ["blocks", "index", "label"],
            "aucune image de page ne va plus au téléphone"
        );
    }
    assert!(servie.get("mode").is_none(), "« mode » a disparu");

    let mut sommaire = Vec::new();
    entrees(&lecture.outline, &mut sommaire);
    assert!(!sommaire.is_empty(), "le petit PDF porte deux titres");
    for (titre, niveau, page) in &sommaire {
        assert!((1..=3).contains(niveau), "{titre} : niveau {niveau}");
        assert!(
            (1..=i64::from(lecture.page_count)).contains(page),
            "{titre} : page {page} hors du document"
        );
        let texte = &textes[(*page - 1) as usize].1;
        assert!(
            a_plat(texte)
                .to_lowercase()
                .contains(&a_plat(titre).to_lowercase()),
            "« {titre} » renvoie à la page qui le porte, pas à la {page} : {texte:?}"
        );
    }

    let (_, encore) = lire(&bac, guide).await;
    assert_eq!(encore, empreinte, "figée tant que rien ne change");
    assert_eq!(
        annonce(&bac, guide).await,
        (true, true, Some(empreinte)),
        "la liste annonce ce que la lecture rend"
    );
}

// -----------------------------------------------------------------------------
// Le choix « Texte agrandi »
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_choix_texte_agrandi_suit_le_verdict_puis_le_choix() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let (recompose, du_verdict) = lire(&bac, guide).await;
    assert_eq!(
        modes(&bac, guide).await,
        (true, true),
        "verdict vrai, sans choix"
    );

    choisir(&bac, ifdd, guide, Some(false)).await;
    let (retire, empreinte) = lire(&bac, guide).await;
    assert_eq!(
        (retire.has_text, retire.large_text),
        (true, false),
        "choix contraire au verdict"
    );
    assert_eq!(retire.pages.len(), recompose.pages.len());
    for (p, r) in retire.pages.iter().zip(&recompose.pages) {
        assert_eq!(
            p.blocks, r.blocks,
            "page {} : le texte reste, pour la recherche",
            p.index
        );
    }
    assert_eq!(retire.outline, recompose.outline, "le sommaire reste");
    assert_ne!(empreinte, du_verdict, "la copie gardée n'est plus la bonne");
    assert_eq!(annonce(&bac, guide).await, (true, false, Some(empreinte)));

    choisir(&bac, ifdd, guide, None).await;
    let (_, reprise) = lire(&bac, guide).await;
    assert_eq!(modes(&bac, guide).await, (true, true));
    assert_eq!(reprise, du_verdict, "rendue au verdict : la même empreinte");

    sqlx::query(
        "UPDATE negotiation.document_renditions SET is_reflowable = false WHERE document_id = $1",
    )
    .bind(guide)
    .execute(bac.pool())
    .await
    .expect("verdict faux");
    assert_eq!(
        modes(&bac, guide).await,
        (true, false),
        "verdict faux, sans choix"
    );
    choisir(&bac, ifdd, guide, Some(true)).await;
    assert_eq!(
        modes(&bac, guide).await,
        (true, true),
        "le choix l'emporte sur le verdict"
    );

    sqlx::query("UPDATE negotiation.document_pages SET plain_text = '' WHERE document_id = $1")
        .bind(guide)
        .execute(bac.pool())
        .await
        .expect("pages sans texte");
    assert_eq!(
        modes(&bac, guide).await,
        (false, false),
        "sans texte, ni recherche ni « Texte agrandi », quel que soit le choix"
    );

    let traces = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT actor_id FROM platform.audit_log
          WHERE entity_schema = 'negotiation' AND entity_table = 'document_renditions'
            AND new_data ->> 'document_id' = $1::text
            AND 'large_text_choice' = ANY (changed_fields)",
    )
    .bind(guide)
    .fetch_all(bac.pool())
    .await
    .expect("journal d'audit");
    assert_eq!(
        traces,
        [Some(ifdd); 3],
        "chaque choix « Texte agrandi » laisse une trace à son nom"
    );
}

// -----------------------------------------------------------------------------
// L'empreinte de lecture
// -----------------------------------------------------------------------------

#[tokio::test]
async fn lempreinte_de_lecture_distingue_les_documents_et_suit_lextraction() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let jumeau = fichier_publie(&bac, ifdd, "Guide jumeau", false).await;

    let (_, du_guide) = lire(&bac, guide).await;
    let (_, du_jumeau) = lire(&bac, jumeau).await;
    assert_ne!(
        du_guide, du_jumeau,
        "même fichier, deux documents : deux copies à garder"
    );

    let fichier = fichier_du_document(&bac, guide).await;
    demander_lextraction(&bac, guide, fichier, ifdd).await;
    let issues = passer_lextraction(&bac).await;
    assert!(
        !issues.is_empty() && issues.iter().all(Result::is_ok),
        "{issues:?}"
    );

    let (_, reextrait) = lire(&bac, guide).await;
    assert_ne!(
        reextrait, du_guide,
        "une nouvelle extraction change l'empreinte"
    );
    assert_eq!(annonce(&bac, guide).await.2, Some(reextrait));
    assert_eq!(
        lire(&bac, jumeau).await.1,
        du_jumeau,
        "l'autre document ne bouge pas"
    );
}
