//! **« Nouvelle version », par les services** : le brouillon qu'elle crée
//! reprend la fiche de l'original, sauf la version et le fichier, et un second
//! remplaçant est refusé en nommant le premier.

mod commun;

use commun::documents::{administratrice, fichier_publie, objet_pdf, passer_lextraction, PETIT};
use commun::Bac;
use kernel::error::{ApiError, ErrorCode, Result};
use negotiation::domain::admin_documents::{AdminDocument, AdminDocumentInput};
use negotiation::service::admin_documents as admin;
use serde_json::{json, Value};
use uuid::Uuid;

fn saisie(v: Value) -> AdminDocumentInput {
    serde_json::from_value(v).expect("entrée de document")
}

fn refus<T: std::fmt::Debug>(r: Result<T>) -> ApiError {
    r.expect_err("un refus était attendu")
}

async fn creer_avec(bac: &Bac, auteur: Uuid, v: Value) -> Result<Uuid> {
    admin::creer(&bac.state, &bac.ctx(auteur), &saisie(v), "fr").await
}

async fn modifier(bac: &Bac, auteur: Uuid, id: Uuid, v: Value) -> Result<()> {
    admin::modifier(&bac.state, &bac.ctx(auteur), id, &saisie(v), "fr").await
}

async fn la_fiche(bac: &Bac, personne: Uuid, id: Uuid) -> AdminDocument {
    let droits = admin::droits(&bac.state, personne).await.expect("droits");
    admin::fiche(&bac.state, &droits, id, "fr")
        .await
        .expect("fiche du document")
}

async fn attacher(bac: &Bac, auteur: Uuid, id: Uuid) -> Uuid {
    let asset = objet_pdf(bac, auteur, PETIT, "ready").await;
    admin::attacher_le_fichier(&bac.state, &bac.ctx(auteur), id, asset)
        .await
        .expect("fichier attaché");
    asset
}

async fn documents_en_base(bac: &Bac) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM negotiation.documents")
        .fetch_one(bac.pool())
        .await
        .expect("compte des documents")
}

async fn une_edition(bac: &Bac, slug: &str) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2025, jsonb_build_object('fr', 'COP30'), $1::text::platform.slug,
                   jsonb_build_object('fr', 'COP30'), 'online',
                   'America/Belem'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#,
    )
    .bind(slug)
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition")
}

/// Chaque valeur de l'original diffère du défaut de la colonne : une recopie
/// oubliée se voit.
#[tokio::test]
async fn la_nouvelle_version_reprend_la_fiche_sauf_la_version_et_le_fichier() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let cop = une_edition(&bac, "cop30-belem").await;
    let id = creer_avec(
        &bac,
        ifdd,
        json!({
            "title": { "fr": "Résumé des décisions de Belém", "en": "Summary of the Belém decisions" },
            "summary": { "fr": "Ce qui a été adopté.", "en": "What was adopted." },
            "type": "summary",
            "themes": ["mitigation", "finance"],
            "cop": cop,
            "version": "2025",
            "publisher": "OIF/IFDD",
            "locale": "en",
            "restricted": false,
            "rag_eligible": true,
        }),
    )
    .await
    .expect("original");
    attacher(&bac, ifdd, id).await;
    passer_lextraction(&bac).await;
    admin::publier(&bac.state, &bac.ctx(ifdd), id)
        .await
        .expect("publication");
    let original = la_fiche(&bac, ifdd, id).await;
    assert!(!original.restricted && original.rag_eligible);
    assert_eq!(original.locale, "en");
    assert_eq!(original.cop, Some(cop));

    let nouveau = admin::nouvelle_version(&bac.state, &bac.ctx(ifdd), id, "fr")
        .await
        .expect("nouvelle version");
    let suite = la_fiche(&bac, ifdd, nouveau).await;
    assert_eq!(suite.title, original.title);
    assert_eq!(
        suite.summary,
        Some(json!({ "fr": "Ce qui a été adopté.", "en": "What was adopted." }))
    );
    assert_eq!(suite.type_code, "summary");
    assert_eq!(suite.themes, ["mitigation", "finance"]);
    assert_eq!(suite.cop, Some(cop));
    assert_eq!(suite.publisher.as_deref(), Some("OIF/IFDD"));
    assert_eq!(suite.locale, "en");
    assert!(
        !suite.restricted,
        "la colonne vaut « réservé » par défaut : le choix « public » se recopie"
    );
    assert!(suite.rag_eligible);
    assert_eq!(suite.version, "2025 (nouvelle version)");
    assert_eq!(suite.supersedes.as_ref().map(|l| l.id), Some(id));
    assert_eq!(suite.state, "draft");
    assert!(suite.asset_id.is_none() && suite.external_url.is_none());
    assert!(suite.extraction.is_none(), "rien à extraire avant le dépôt");
    assert!(!suite.file_locked);

    let reserve = fichier_publie(&bac, ifdd, "Note réservée", true).await;
    let sa_suite = admin::nouvelle_version(&bac.state, &bac.ctx(ifdd), reserve, "fr")
        .await
        .expect("nouvelle version du réservé");
    let sa_suite = la_fiche(&bac, ifdd, sa_suite).await;
    assert!(sa_suite.restricted, "un réservé le reste");
    assert!(!sa_suite.rag_eligible);
    assert_eq!(sa_suite.locale, "fr");
    assert_eq!(sa_suite.cop, None);
}

/// Le chemin le plus probable vers un second successeur : « nouvelle version »
/// touchée deux fois sur le même document.
#[tokio::test]
async fn une_seconde_nouvelle_version_est_refusee_en_nommant_la_premiere() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let id = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let premiere = admin::nouvelle_version(&bac.state, &bac.ctx(ifdd), id, "fr")
        .await
        .expect("première nouvelle version");
    // Renommée, elle se distingue de l'original, qui porte le même titre.
    modifier(
        &bac,
        ifdd,
        premiere,
        json!({ "title": { "fr": "Guide des négociations 2026", "en": "Negotiations guide 2026" } }),
    )
    .await
    .expect("titre de la première");

    let err = refus(admin::nouvelle_version(&bac.state, &bac.ctx(ifdd), id, "fr").await);
    assert_eq!(
        err.code,
        ErrorCode::NegotiationDocumentAlreadySuperseded,
        "{err}"
    );
    assert_eq!(
        err.message,
        "Ce document est déjà remplacé par « Guide des négociations 2026 »."
    );
    assert_eq!(err.field.as_deref(), Some("supersedes_id"));
    assert_eq!(documents_en_base(&bac).await, 2, "aucun brouillon de plus");
    assert_eq!(
        la_fiche(&bac, ifdd, id).await.superseded_by.map(|l| l.id),
        Some(premiere)
    );
}
