//! **La forme lisible servie au téléphone** : ses pages et son sommaire dans la
//! grammaire de `contracts/forme-lisible.md`, le mode « tel quel », l'empreinte
//! qui dit si la copie gardée est la bonne, et l'image d'une page — par les
//! services, sur base réelle.

mod commun;

use commun::documents::{
    administratrice, demander_lextraction, entrepots, fichier_publie, pages as pages_en_base,
    passer_lextraction, BUCKET_PRIVE,
};
use commun::Bac;
use negotiation::domain::documents::{chemin_image, DocumentReading};
use negotiation::service::{admin_documents, documents as public};
use serde_json::{json, Value};
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

/// Ce que la liste annonce de la forme lisible : son mode et son empreinte.
async fn annonce(bac: &Bac, id: Uuid) -> (Option<&'static str>, Option<String>) {
    let (bib, _) = public::bibliotheque(&bac.state, None, "fr")
        .await
        .expect("bibliothèque");
    let d = bib
        .documents
        .into_iter()
        .find(|d| d.id == id)
        .expect("document listé");
    (d.mode, d.reading_etag)
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
// Le mode recomposé
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
    assert_eq!(lecture.mode, "reflow");
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
        let origine = blocs.iter().any(est_origine);
        assert_eq!(
            page.image.as_deref(),
            origine.then(|| chemin_image(guide, *i)).as_deref(),
            "page {i} : l'image, sur les seules pages à bloc d'origine"
        );
    }
    assert!(
        lecture.pages.iter().any(|p| p.image.is_some()),
        "le petit PDF porte un tableau"
    );

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
        (Some("reflow"), Some(empreinte)),
        "la liste annonce l'empreinte que la lecture rend"
    );
}

// -----------------------------------------------------------------------------
// Le mode « tel quel »
// -----------------------------------------------------------------------------

#[tokio::test]
async fn ouvert_tel_quel_le_document_ne_se_lit_quen_images() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let (recompose, recomposee) = lire(&bac, guide).await;
    let sans_image = recompose
        .pages
        .iter()
        .find(|p| p.image.is_none())
        .expect("une page sans image en mode recomposé")
        .index;

    admin_documents::ouvrir_tel_quel(&bac.state, &bac.ctx(ifdd), guide, true)
        .await
        .expect("tel quel");

    let (tel_quel, empreinte) = lire(&bac, guide).await;
    assert_eq!(tel_quel.mode, "as_is");
    assert_eq!(tel_quel.page_count, 4);
    assert_eq!(tel_quel.outline, json!([]), "ni sommaire");
    let index: Vec<i32> = tel_quel.pages.iter().map(|p| p.index).collect();
    assert_eq!(index, [1, 2, 3, 4]);
    for (p, r) in tel_quel.pages.iter().zip(&recompose.pages) {
        assert_eq!(p.label, r.label, "page {} : son étiquette", p.index);
        assert_eq!(p.blocks, json!([]), "page {} : aucun bloc", p.index);
        assert_eq!(
            p.image.as_deref(),
            Some(chemin_image(guide, p.index).as_str()),
            "page {} : une image sur chaque page",
            p.index
        );
    }
    let servie = serde_json::to_value(&tel_quel).expect("sérialisation");
    for page in servie["pages"].as_array().unwrap() {
        let mut cles: Vec<&str> = page
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        cles.sort_unstable();
        assert_eq!(cles, ["blocks", "image", "index", "label"]);
    }

    assert_ne!(
        empreinte, recomposee,
        "une autre forme lisible : la copie gardée n'est plus la bonne"
    );
    assert_eq!(
        annonce(&bac, guide).await,
        (Some("as_is"), Some(empreinte.clone()))
    );
    let image = public::image(&bac.state, None, guide, sans_image)
        .await
        .expect("l'image d'une page sans bloc d'origine");
    assert!(image.octets.starts_with(&[0xFF, 0xD8]), "un JPEG");

    let traces = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT actor_id FROM platform.audit_log
          WHERE entity_schema = 'negotiation' AND entity_table = 'document_renditions'
            AND new_data ->> 'document_id' = $1::text
            AND 'serve_as_is' = ANY (changed_fields)
            AND (new_data ->> 'serve_as_is')::boolean",
    )
    .bind(guide)
    .fetch_all(bac.pool())
    .await
    .expect("journal d'audit");
    assert_eq!(
        traces,
        [Some(ifdd)],
        "le choix « tel quel » laisse une trace à son nom"
    );

    admin_documents::ouvrir_tel_quel(&bac.state, &bac.ctx(ifdd), guide, false)
        .await
        .expect("retour au mode recomposé");
    let (retour, reprise) = lire(&bac, guide).await;
    assert_eq!(retour.mode, "reflow");
    assert_eq!(retour.outline, recompose.outline);
    assert!(retour.pages.iter().all(|p| !blocs(p).is_empty()));
    assert_eq!(
        reprise, recomposee,
        "la même forme lisible, la même empreinte"
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
    assert_eq!(annonce(&bac, guide).await.1, Some(reextrait));
    assert_eq!(
        lire(&bac, jumeau).await.1,
        du_jumeau,
        "l'autre document ne bouge pas"
    );
}

// -----------------------------------------------------------------------------
// L'image d'une page
// -----------------------------------------------------------------------------

#[tokio::test]
async fn limage_dune_page_se_lit_dans_le_bucket_prive() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let en_base = pages_en_base(&bac, guide).await;
    let (lecture, _) = lire(&bac, guide).await;
    let origine = lecture
        .pages
        .iter()
        .find(|p| p.image.is_some())
        .expect("une page à image")
        .index;
    let autre = if origine == 1 { 2 } else { 1 };

    let image = public::image(&bac.state, None, guide, origine)
        .await
        .expect("image");
    assert!(image.octets.starts_with(&[0xFF, 0xD8]), "un JPEG");
    assert!(!image.reservee, "public : cache partagé permis");
    let cle = en_base
        .iter()
        .find(|p| p.0 == origine)
        .and_then(|p| p.2.clone())
        .expect("la clé de l'image en base");
    let stockee = entrepots(&bac)
        .du_bucket(BUCKET_PRIVE)
        .get(&cle)
        .await
        .expect("l'objet dans le bucket privé");
    assert_eq!(
        image.octets, stockee,
        "l'image de cette page, et d'aucune autre"
    );

    let voisine = public::image(&bac.state, None, guide, autre)
        .await
        .expect("image d'une autre page");
    assert_ne!(voisine.octets, image.octets);
    assert_ne!(
        voisine.empreinte, image.empreinte,
        "une empreinte par image"
    );

    let hors = public::image(&bac.state, None, guide, 99)
        .await
        .err()
        .expect("une page qui n'existe pas");
    assert_eq!(hors.code.status().as_u16(), 404);
}
