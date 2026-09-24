//! **Les lectures publiques des documents, par les services** : la
//! bibliothèque, la recherche, le remplacement, le dépublié et le lien — sur
//! base réelle. La forme lisible et l'image d'une page sont dans
//! `documents_public_lecture.rs` ; les notes, le compteur et les favoris dans
//! `documents_public_notes_favoris.rs`. Le `304` se prouve en HTTP, dans
//! `crates/api/tests/`.

mod commun;
mod fabrique;

use std::collections::HashMap;

use commun::documents::{
    administratrice, creer, entree, fichier_publie, lien_publie, objet_pdf, passer_lextraction,
    PETIT,
};
use commun::{terme_thematique, Bac};
use kernel::error::{ErrorCode, Result};
use negotiation::domain::admin_documents::AdminDocumentInput;
use negotiation::domain::documents::{DocumentLibrary, DocumentTextHits, LibraryDocument};
use negotiation::service::{admin_documents, documents as public};
use serde_json::json;
use time::macros::date;
use time::OffsetDateTime;
use uuid::Uuid;

// -----------------------------------------------------------------------------
// Les aides propres à ce lot
// -----------------------------------------------------------------------------

fn refus<T>(r: Result<T>) -> ErrorCode {
    match r {
        Ok(_) => panic!("un refus était attendu"),
        Err(e) => e.code,
    }
}

fn saisie(v: serde_json::Value) -> AdminDocumentInput {
    serde_json::from_value(v).expect("entrée de document")
}

/// Attache un PDF à un brouillon, passe l'extraction et publie.
async fn garnir_et_publier(bac: &Bac, admin: Uuid, id: Uuid, octets: &[u8]) {
    let asset = objet_pdf(bac, admin, octets, "ready").await;
    admin_documents::attacher_le_fichier(&bac.state, &bac.ctx(admin), id, asset)
        .await
        .expect("fichier attaché");
    let issues = passer_lextraction(bac).await;
    assert!(issues.iter().all(Result::is_ok), "{issues:?}");
    admin_documents::publier(&bac.state, &bac.ctx(admin), id)
        .await
        .expect("publication");
}

async fn fichier_publie_avec(bac: &Bac, admin: Uuid, e: AdminDocumentInput, pdf: &[u8]) -> Uuid {
    let id = admin_documents::creer(&bac.state, &bac.ctx(admin), &e)
        .await
        .expect("création du brouillon");
    garnir_et_publier(bac, admin, id, pdf).await;
    id
}

/// Un lien publié : sans extraction, c'est le document le moins coûteux.
async fn lien_publie_avec(bac: &Bac, admin: Uuid, e: AdminDocumentInput) -> Uuid {
    let id = admin_documents::creer(&bac.state, &bac.ctx(admin), &e)
        .await
        .expect("création du lien");
    admin_documents::publier(&bac.state, &bac.ctx(admin), id)
        .await
        .expect("publication du lien");
    id
}

/// Un lien qui remplace `ancien`, laissé en brouillon.
async fn lien_remplacant(bac: &Bac, admin: Uuid, ancien: Uuid, titre: &str) -> Uuid {
    let mut e = entree(titre, false);
    e.external_url = Some(Some(format!("https://unfccc.int/{}", Uuid::now_v7())));
    e.supersedes_id = Some(Some(ancien));
    admin_documents::creer(&bac.state, &bac.ctx(admin), &e)
        .await
        .expect("création du remplaçant")
}

/// Un fichier publié qui remplace `ancien`.
async fn remplacant_publie(bac: &Bac, admin: Uuid, ancien: Uuid, titre: &str) -> Uuid {
    let mut e = entree(titre, false);
    e.supersedes_id = Some(Some(ancien));
    fichier_publie_avec(bac, admin, e, PETIT).await
}

async fn bibliotheque(
    bac: &Bac,
    personne: Option<Uuid>,
    locale: &str,
) -> (DocumentLibrary, String) {
    public::bibliotheque(&bac.state, personne, locale)
        .await
        .expect("bibliothèque")
}

fn dans(bib: &DocumentLibrary, id: Uuid) -> Option<&LibraryDocument> {
    bib.documents.iter().find(|d| d.id == id)
}

fn remplacant(bib: &DocumentLibrary, id: Uuid) -> Option<Uuid> {
    dans(bib, id)
        .expect("document listé")
        .superseded_by
        .as_ref()
        .map(|s| s.id)
}

/// L'ordre du terme, lu dans la base et non recopié du semis.
async fn ordre_du_terme(bac: &Bac, taxonomie: &str, code: &str) -> i32 {
    sqlx::query_scalar::<_, i16>(
        "SELECT sort_order FROM reference.taxonomy_terms WHERE taxonomy_code = $1 AND code = $2",
    )
    .bind(taxonomie)
    .bind(code)
    .fetch_one(bac.pool())
    .await
    .map(i32::from)
    .expect("ordre du terme")
}

/// Une édition. Sans ville, elle est en ligne : `ck_events_physical_location`
/// n'exige la ville et le pays qu'en présentiel.
async fn edition(
    bac: &Bac,
    libelle: Option<&str>,
    acronyme: Option<&str>,
    titre: serde_json::Value,
    ville: Option<&str>,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO event.events
               (edition_label, edition_year, title, acronym, slug, description, participation_mode,
                timezone, starts_at, ends_at, country_id, city)
           VALUES ($1, 2025, $2, $3, $4::text::platform.slug, '{"fr":"Description."}'::jsonb,
                   (CASE WHEN $5::text IS NULL THEN 'online' ELSE 'in_person' END)::event.participation_mode,
                   'America/Belem', now(), now() + interval '10 days',
                   CASE WHEN $5::text IS NULL THEN NULL
                        ELSE (SELECT id FROM reference.countries ORDER BY iso2 LIMIT 1) END,
                   $5)
           RETURNING id"#,
    )
    .bind(libelle)
    .bind(titre)
    .bind(acronyme)
    .bind(format!("edition-{}", Uuid::now_v7().simple()))
    .bind(ville)
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition")
}

/// Un PDF A4, une page par texte, écrit au milieu de la page : ni en-tête ni
/// pied que l'extraction retirerait.
fn pdf_de_pages(textes: &[String]) -> Vec<u8> {
    let n = textes.len();
    let enfants: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + 2 * i)).collect();
    let mut objets = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {n} >>",
            enfants.join(" ")
        ),
        fabrique::HELVETICA.to_owned(),
    ];
    for (i, texte) in textes.iter().enumerate() {
        objets.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
             /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            5 + 2 * i
        ));
        objets.push(fabrique::flux(&format!(
            "BT /F1 11 Tf 72 420 Td ({texte}) Tj ET"
        )));
    }
    fabrique::pdf(&objets)
}

// -----------------------------------------------------------------------------
// La bibliothèque
// -----------------------------------------------------------------------------

#[tokio::test]
async fn la_bibliotheque_rend_les_documents_publies_resolus_dans_la_langue_demandee() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let resume = fichier_publie_avec(
        &bac,
        ifdd,
        saisie(json!({
            "title": { "fr": "Résumé pour les décideurs", "en": "Summary for policymakers" },
            "summary": { "fr": "Le résumé, en français seulement." },
            "type": "summary",
            "themes": ["finance", "mitigation", "adaptation"],
            "issued_on": "2025-11-10",
            "publisher": "IFDD",
        })),
        PETIT,
    )
    .await;
    let bulletin = lien_publie_avec(
        &bac,
        ifdd,
        saisie(json!({
            "title": { "fr": "Bulletin du jour" },
            "type": "bulletin",
            "external_url": "https://enb.iisd.org/cop30",
        })),
    )
    .await;
    // Son type, « negotiation_guide », n'est cité par aucun document publié.
    let brouillon = creer(&bac, ifdd, "Brouillon jamais publié", false).await;

    let avant = OffsetDateTime::now_utc();
    let (fr, _) = bibliotheque(&bac, None, "fr").await;
    let apres = OffsetDateTime::now_utc();
    assert!(
        avant <= fr.served_at && fr.served_at <= apres,
        "l'heure du serveur, au moment de la lecture"
    );
    assert!(
        dans(&fr, brouillon).is_none(),
        "un brouillon ne se liste pas"
    );
    let d = dans(&fr, resume).expect("le résumé publié est listé");
    assert_eq!(d.title, "Résumé pour les décideurs");
    assert_eq!(
        d.summary.as_deref(),
        Some("Le résumé, en français seulement.")
    );
    assert_eq!(d.type_code, "summary");
    let mut themes = d.themes.clone();
    themes.sort();
    assert_eq!(themes, ["adaptation", "finance", "mitigation"]);
    assert!(!d.themes_hidden);
    assert_eq!(d.issued_on, Some(date!(2025 - 11 - 10)));
    assert!(d.cop.is_none());
    assert_eq!(d.publisher.as_deref(), Some("IFDD"));
    assert_eq!(d.version, "1");
    assert_eq!(d.locale, "fr");
    assert!(!d.slug.is_empty());
    assert_eq!(d.source, "file");
    assert!(d.external_url.is_none() && d.link_host.is_none());
    assert!(!d.restricted && d.accessible);
    assert_eq!(d.page_count, Some(4));
    assert!(d.reading_bytes.is_some_and(|o| o > 0));
    assert!(d.has_text && d.large_text);
    assert!(d.superseded_by.is_none());
    assert!(d.reading_etag.is_some());
    let b = dans(&fr, bulletin).expect("le lien est listé");
    assert_eq!(b.type_code, "bulletin");
    assert!(b.themes.is_empty() && !b.themes_hidden);
    assert!(b.issued_on.is_none());

    let mut types: Vec<_> = fr
        .vocabulary
        .types
        .iter()
        .map(|t| t.code.as_str())
        .collect();
    types.sort_unstable();
    assert_eq!(types, ["bulletin", "summary"], "les seuls types cités");
    let mut codes: Vec<_> = fr
        .vocabulary
        .themes
        .iter()
        .map(|t| t.code.as_str())
        .collect();
    codes.sort_unstable();
    assert_eq!(
        codes,
        ["adaptation", "finance", "mitigation"],
        "les seules thématiques citées"
    );
    for t in &fr.vocabulary.types {
        assert_eq!(
            t.sort_order,
            ordre_du_terme(&bac, "document_type", &t.code).await
        );
    }
    for t in &fr.vocabulary.themes {
        assert_eq!(
            t.sort_order,
            ordre_du_terme(&bac, "negotiation_theme", &t.code).await
        );
    }
    let libelle = |bib: &DocumentLibrary, code: &str| {
        bib.vocabulary
            .types
            .iter()
            .chain(&bib.vocabulary.themes)
            .find(|t| t.code == code)
            .map(|t| t.label.clone())
            .expect("terme cité")
    };
    assert_eq!(libelle(&fr, "summary"), "Résumé");
    assert_eq!(libelle(&fr, "mitigation"), "Atténuation");

    let (en, _) = bibliotheque(&bac, None, "en").await;
    let d = dans(&en, resume).unwrap();
    assert_eq!(d.title, "Summary for policymakers");
    assert_eq!(
        d.summary.as_deref(),
        Some("Le résumé, en français seulement."),
        "sans anglais, repli sur le français"
    );
    assert_eq!(libelle(&en, "summary"), "Summary");
    assert_eq!(libelle(&en, "mitigation"), "Mitigation");
    assert_eq!(
        dans(&en, bulletin).unwrap().title,
        "Bulletin du jour",
        "un titre sans anglais se replie sur le français"
    );
}

#[tokio::test]
async fn la_bibliotheque_cite_ses_cop_par_leur_libelle_et_leur_ville() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let belem = edition(
        &bac,
        Some("COP30"),
        Some("CdP30"),
        json!({ "fr": "Conférence de Belém", "en": "Belém Conference" }),
        Some("Belém"),
    )
    .await;
    let bonn = edition(
        &bac,
        None,
        Some("SB62"),
        json!({ "fr": "Sessions de Bonn", "en": "Bonn sessions" }),
        Some("Bonn"),
    )
    .await;
    let rio = edition(
        &bac,
        None,
        None,
        json!({ "fr": "Sommet de Rio", "en": "Rio Summit" }),
        None,
    )
    .await;
    let bakou = edition(
        &bac,
        Some("COP29"),
        None,
        json!({ "fr": "Conférence de Bakou" }),
        Some("Bakou"),
    )
    .await;

    let lien_de = |titre: &'static str, cop: Option<Uuid>| {
        saisie(json!({
            "title": { "fr": titre },
            "type": "bulletin",
            "external_url": "https://enb.iisd.org/",
            "cop": cop,
        }))
    };
    let a_belem = lien_publie_avec(&bac, ifdd, lien_de("Bulletin de Belém", Some(belem))).await;
    let a_bonn = lien_publie_avec(&bac, ifdd, lien_de("Bulletin de Bonn", Some(bonn))).await;
    let a_rio = lien_publie_avec(&bac, ifdd, lien_de("Bulletin de Rio", Some(rio))).await;
    let sans = lien_publie_avec(&bac, ifdd, lien_de("Bulletin sans COP", None)).await;
    // Bakou n'est citée que par un brouillon.
    admin_documents::creer(
        &bac.state,
        &bac.ctx(ifdd),
        &lien_de("Bulletin de Bakou", Some(bakou)),
    )
    .await
    .expect("brouillon");

    let (fr, _) = bibliotheque(&bac, None, "fr").await;
    assert_eq!(dans(&fr, a_belem).unwrap().cop, Some(belem));
    assert_eq!(dans(&fr, a_bonn).unwrap().cop, Some(bonn));
    assert_eq!(dans(&fr, a_rio).unwrap().cop, Some(rio));
    assert_eq!(dans(&fr, sans).unwrap().cop, None);

    let cops = |bib: &DocumentLibrary| -> HashMap<Uuid, (String, Option<String>)> {
        bib.vocabulary
            .cops
            .iter()
            .map(|c| (c.id, (c.label.clone(), c.city.clone())))
            .collect()
    };
    let vues = cops(&fr);
    assert_eq!(
        vues.len(),
        3,
        "les seules COP citées par un document publié"
    );
    assert!(!vues.contains_key(&bakou));
    assert_eq!(
        vues[&belem],
        ("COP30".to_owned(), Some("Belém".to_owned())),
        "le libellé d'édition l'emporte sur l'acronyme"
    );
    assert_eq!(
        vues[&bonn],
        ("SB62".to_owned(), Some("Bonn".to_owned())),
        "sans libellé d'édition, l'acronyme"
    );
    assert_eq!(
        vues[&rio],
        ("Sommet de Rio".to_owned(), None),
        "sans l'un ni l'autre, le titre"
    );

    let en = cops(&bibliotheque(&bac, None, "en").await.0);
    assert_eq!(
        en[&rio].0, "Rio Summit",
        "le titre, dans la langue demandée"
    );
    assert_eq!(en[&belem].0, "COP30");
}

#[tokio::test]
async fn une_thematique_desactivee_ne_se_montre_plus() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let bulletin = lien_publie_avec(
        &bac,
        ifdd,
        saisie(json!({
            "title": { "fr": "Bulletin du jour" },
            "type": "bulletin",
            "themes": ["mitigation", "finance"],
            "external_url": "https://enb.iisd.org/cop30",
        })),
    )
    .await;
    let codes = |bib: &DocumentLibrary| {
        let mut du_document = dans(bib, bulletin).unwrap().themes.clone();
        du_document.sort();
        let mut du_vocabulaire: Vec<String> = bib
            .vocabulary
            .themes
            .iter()
            .map(|t| t.code.clone())
            .collect();
        du_vocabulaire.sort();
        (du_document, du_vocabulaire)
    };

    let (avant, _) = bibliotheque(&bac, None, "fr").await;
    assert_eq!(
        codes(&avant),
        (
            vec!["finance".to_owned(), "mitigation".to_owned()],
            vec!["finance".to_owned(), "mitigation".to_owned()]
        )
    );

    sqlx::query("UPDATE reference.taxonomy_terms SET is_active = false WHERE id = $1")
        .bind(terme_thematique(&bac, "mitigation").await)
        .execute(bac.pool())
        .await
        .expect("désactivation du terme");

    let (apres, _) = bibliotheque(&bac, None, "fr").await;
    assert_eq!(
        codes(&apres),
        (vec!["finance".to_owned()], vec!["finance".to_owned()]),
        "actives seulement, sur le document comme dans le vocabulaire"
    );
}

#[tokio::test]
async fn lempreinte_de_la_bibliotheque_ne_change_quavec_son_contenu() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;

    let (_, premiere) = bibliotheque(&bac, None, "fr").await;
    let (_, seconde) = bibliotheque(&bac, None, "fr").await;
    assert_eq!(
        premiere, seconde,
        "même empreinte à contenu égal, l'heure de service exclue"
    );

    lien_publie(
        &bac,
        ifdd,
        "Bulletin des négociations",
        "https://enb.iisd.org/cop30",
        false,
    )
    .await;
    let (_, apres_publication) = bibliotheque(&bac, None, "fr").await;
    assert_ne!(
        apres_publication, seconde,
        "une publication change l'empreinte"
    );

    admin_documents::depublier(&bac.state, &bac.ctx(ifdd), guide)
        .await
        .expect("dépublication");
    let (_, apres_depublication) = bibliotheque(&bac, None, "fr").await;
    assert_ne!(apres_depublication, apres_publication);
}

// -----------------------------------------------------------------------------
// La recherche
// -----------------------------------------------------------------------------

#[tokio::test]
async fn la_recherche_rend_la_page_et_son_extrait() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;

    let trouve = public::rechercher(&bac.state, None, "  negociations reprennent ")
        .await
        .expect("recherche");
    assert_eq!(trouve.query, "negociations reprennent");
    let hit = trouve
        .hits
        .iter()
        .find(|h| h.document_id == guide)
        .expect("le guide est trouvé");
    let page = hit.pages.first().expect("une page citée");
    assert_eq!(page.index, 1);
    assert!(!page.label.is_empty());
    assert!(
        page.excerpt.to_lowercase().contains("négociations"),
        "l'extrait cite le passage : {:?}",
        page.excerpt
    );

    let rien = public::rechercher(&bac.state, None, "zzzqqqxxx")
        .await
        .expect("recherche sans résultat");
    assert!(rien.hits.is_empty());

    assert_eq!(
        refus(public::rechercher(&bac.state, None, "   ").await),
        ErrorCode::ValidationFailed
    );
}

#[tokio::test]
async fn la_recherche_garde_cinq_pages_par_document_les_plus_pertinentes_dabord() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    // Créé le premier : si l'ordre suivait l'identifiant, il passerait devant.
    let une_fois = fichier_publie_avec(
        &bac,
        ifdd,
        entree("Note sur les zones humides", false),
        &pdf_de_pages(&["Une seule mention des tourbieres dans cette note.".to_owned()]),
    )
    .await;
    // Sept pages qui citent le mot ; les deux dernières le citent trois fois.
    let pages: Vec<String> = (1..=7)
        .map(|n| {
            if n >= 6 {
                "Tourbieres anciennes, tourbieres drainees et tourbieres restaurees.".to_owned()
            } else {
                "Les tourbieres couvrent de vastes surfaces humides du bassin.".to_owned()
            }
        })
        .collect();
    let sept_pages = fichier_publie_avec(
        &bac,
        ifdd,
        entree("Guide des tourbières", false),
        &pdf_de_pages(&pages),
    )
    .await;
    // Le guide ordinaire ne cite pas le mot.
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;

    let trouve = public::rechercher(&bac.state, None, "tourbieres")
        .await
        .expect("recherche");
    let ordre: Vec<Uuid> = trouve.hits.iter().map(|h| h.document_id).collect();
    assert_eq!(
        ordre,
        [sept_pages, une_fois],
        "le document le plus pertinent d'abord, et le guide absent"
    );
    assert!(!ordre.contains(&guide));
    let citees: Vec<i32> = trouve.hits[0].pages.iter().map(|p| p.index).collect();
    assert_eq!(
        citees,
        [6, 7, 1, 2, 3],
        "cinq pages au plus, les plus pertinentes d'abord"
    );
    assert!(trouve.hits[0]
        .pages
        .iter()
        .all(|p| p.excerpt.to_lowercase().contains("tourbieres")));
    let seule: Vec<i32> = trouve.hits[1].pages.iter().map(|p| p.index).collect();
    assert_eq!(seule, [1]);
}

// -----------------------------------------------------------------------------
// Le remplacement
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_document_remplace_par_un_brouillon_nest_pas_marque_remplace() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let ancien = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let nouveau = admin_documents::nouvelle_version(&bac.state, &bac.ctx(ifdd), ancien, "fr")
        .await
        .expect("nouvelle version");

    let (bib, _) = bibliotheque(&bac, None, "fr").await;
    assert!(
        dans(&bib, ancien).unwrap().superseded_by.is_none(),
        "un brouillon ne remplace encore rien"
    );
    assert!(dans(&bib, nouveau).is_none());

    garnir_et_publier(&bac, ifdd, nouveau, PETIT).await;
    let (bib, _) = bibliotheque(&bac, None, "fr").await;
    let s = dans(&bib, ancien)
        .unwrap()
        .superseded_by
        .as_ref()
        .expect("remplacé, une fois le successeur publié");
    assert_eq!(s.id, nouveau);
}

#[tokio::test]
async fn une_chaine_de_trois_mene_chaque_ancien_au_bout_publie() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let a = fichier_publie(&bac, ifdd, "Guide, première édition", false).await;
    let b = remplacant_publie(&bac, ifdd, a, "Guide, deuxième édition").await;
    let c = remplacant_publie(&bac, ifdd, b, "Guide, troisième édition").await;

    let (bib, _) = bibliotheque(&bac, None, "fr").await;
    let bout = dans(&bib, c).expect("le bout est listé");
    assert!(bout.superseded_by.is_none());
    for ancien in [a, b] {
        let d = dans(&bib, ancien).expect("un remplacé publié reste listé");
        let s = d.superseded_by.as_ref().expect("marqué remplacé");
        assert_eq!(s.id, c, "mène au bout de la chaîne, pas au maillon suivant");
        assert_eq!(s.title, "Guide, troisième édition");
        assert_eq!(s.published_at, bout.published_at);
        assert_eq!(s.page_count, Some(4));
    }

    let empreintes: Vec<&str> = [a, b, c]
        .iter()
        .map(|id| dans(&bib, *id).unwrap().reading_etag.as_deref().unwrap())
        .collect();
    assert!(
        empreintes[0] != empreintes[1]
            && empreintes[1] != empreintes[2]
            && empreintes[0] != empreintes[2],
        "même fichier, trois documents : trois formes lisibles à garder"
    );
}

#[tokio::test]
async fn une_chaine_sarrete_au_dernier_maillon_publie() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let a = lien_publie(
        &bac,
        ifdd,
        "Bulletin, premier",
        "https://enb.iisd.org/1",
        false,
    )
    .await;
    let b = lien_remplacant(&bac, ifdd, a, "Bulletin, deuxième").await;
    admin_documents::publier(&bac.state, &bac.ctx(ifdd), b)
        .await
        .expect("publication de B");
    let c = lien_remplacant(&bac, ifdd, b, "Bulletin, troisième").await;

    let (bib, _) = bibliotheque(&bac, None, "fr").await;
    assert_eq!(remplacant(&bib, a), Some(b), "C en brouillon : A mène à B");
    assert_eq!(remplacant(&bib, b), None, "un brouillon ne remplace rien");

    admin_documents::publier(&bac.state, &bac.ctx(ifdd), c)
        .await
        .expect("publication de C");
    let (bib, _) = bibliotheque(&bac, None, "fr").await;
    assert_eq!(remplacant(&bib, a), Some(c));
    assert_eq!(remplacant(&bib, b), Some(c));

    admin_documents::depublier(&bac.state, &bac.ctx(ifdd), c)
        .await
        .expect("dépublication de C");
    let (bib, _) = bibliotheque(&bac, None, "fr").await;
    assert!(dans(&bib, c).is_none());
    assert_eq!(remplacant(&bib, a), Some(b), "C dépublié : A mène à B");
    let bout = dans(&bib, b).unwrap();
    assert!(
        bout.superseded_by.is_none(),
        "un maillon dépublié interrompt la chaîne"
    );
    let s = dans(&bib, a).unwrap().superseded_by.as_ref().unwrap();
    assert_eq!(s.title, "Bulletin, deuxième");
    assert_eq!(s.published_at, bout.published_at);
}

// -----------------------------------------------------------------------------
// Dépublié, lien externe
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_document_depublie_quitte_la_liste_et_ne_se_lit_plus() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    public::lire_le_fichier(&bac.state, None, guide, Some("bytes=0-99"), true)
        .await
        .map(|_| ())
        .expect("publié, son fichier se sert");
    let cite = |t: &DocumentTextHits| t.hits.iter().any(|h| h.document_id == guide);
    let avant = public::rechercher(&bac.state, None, "negociations reprennent")
        .await
        .unwrap();
    assert!(cite(&avant), "publié, la recherche le trouve");

    admin_documents::depublier(&bac.state, &bac.ctx(ifdd), guide)
        .await
        .expect("dépublication");

    let (bib, _) = bibliotheque(&bac, None, "fr").await;
    assert!(dans(&bib, guide).is_none(), "absent de la liste");

    let e = public::lecture(&bac.state, None, guide)
        .await
        .expect_err("un dépublié ne se lit plus");
    assert_eq!(e.code, ErrorCode::NegotiationDocumentNotFound);
    assert_eq!(e.code.status().as_u16(), 404);
    assert_eq!(
        refus(
            public::lire_le_fichier(&bac.state, None, guide, Some("bytes=0-99"), true)
                .await
                .map(|_| ())
        ),
        ErrorCode::NegotiationDocumentNotFound,
        "ni son fichier, morceau compris"
    );

    let apres = public::rechercher(&bac.state, None, "negociations reprennent")
        .await
        .unwrap();
    assert!(!cite(&apres), "ni cité par la recherche");
}

#[tokio::test]
async fn un_lien_externe_se_liste_mais_ne_se_lit_pas_dans_lapplication() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let enb = lien_publie(
        &bac,
        ifdd,
        "Bulletin des négociations",
        "https://www.enb.iisd.org/cop30",
        false,
    )
    .await;

    let (bib, _) = bibliotheque(&bac, None, "fr").await;
    let l = dans(&bib, enb).expect("le lien est listé");
    assert_eq!(l.source, "link");
    assert_eq!(
        l.external_url.as_deref(),
        Some("https://www.enb.iisd.org/cop30")
    );
    assert_eq!(l.link_host.as_deref(), Some("enb.iisd.org"));
    assert!(l.page_count.is_none() && l.reading_bytes.is_none());
    assert!(!l.has_text && !l.large_text && l.reading_etag.is_none());

    let e = public::lecture(&bac.state, None, enb)
        .await
        .expect_err("un lien n'a pas de forme lisible");
    assert_eq!(e.code, ErrorCode::NegotiationDocumentNotReadable);
    assert_eq!(e.code.status().as_u16(), 409);
    assert_eq!(
        refus(
            public::lire_le_fichier(&bac.state, None, enb, None, true)
                .await
                .map(|_| ())
        ),
        ErrorCode::NegotiationDocumentNotReadable
    );
}
