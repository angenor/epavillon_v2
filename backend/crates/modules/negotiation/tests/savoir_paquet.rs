//! Le paquet du savoir, par le service, sur base réelle : ce qui sort, ce
//! qu'une différence rend, et les refus de la base traduits. Le `304` à travers
//! HTTP est dans `crates/api/tests/routes_negotiation_savoir.rs`.

mod commun;

use commun::documents::expert;
use commun::savoir::{entree_faq, etape, statut_faq, terme, vieillir, vocabulaire};
use commun::Bac;
use kernel::error::ErrorCode;
use negotiation::domain::savoir::{KnowledgeBundle, KnowledgeStatus};
use negotiation::service::savoir_paquet::{empreinte, paquet};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

async fn entier(bac: &Bac) -> KnowledgeBundle {
    paquet(&bac.state, "fr", None).await.expect("paquet").0
}

async fn depuis(bac: &Bac, since: OffsetDateTime) -> KnowledgeBundle {
    paquet(&bac.state, "fr", Some(since))
        .await
        .expect("différence")
        .0
}

fn faq_ids(p: &KnowledgeBundle) -> Vec<Uuid> {
    p.faq.iter().map(|e| e.id).collect()
}

fn lexique_ids(p: &KnowledgeBundle) -> Vec<Uuid> {
    p.glossary.iter().map(|e| e.id).collect()
}

async fn refus(bac: &Bac, sql: &'static str, id: Uuid) -> ErrorCode {
    let err = sqlx::query(sql)
        .bind(id)
        .execute(bac.pool())
        .await
        .expect_err("la base refuse");
    kernel::pg_error::translate(&err).code
}

async fn resoudre(bac: &Bac, texte: &str) -> Option<Uuid> {
    sqlx::query_scalar!("SELECT negotiation.glossary_resolve($1) AS id", texte)
        .fetch_one(bac.pool())
        .await
        .expect("résolution")
}

#[tokio::test]
async fn le_brouillon_ne_sort_jamais_et_a_revoir_sort_avec_son_statut() {
    let bac = Bac::monter().await;
    let relectrice = expert(&bac, "expert@example.org").await;
    let brouillon = entree_faq(&bac, relectrice, "Où retirer mon badge ?", "draft").await;
    let publiee = entree_faq(
        &bac,
        relectrice,
        "Qu'est-ce qu'un groupe de contact ?",
        "published",
    )
    .await;
    let a_revoir = entree_faq(&bac, relectrice, "Que faire le premier jour ?", "to_review").await;
    let terme_brouillon = terme(&bac, "Informal informal", "draft", None, &[]).await;
    let contact = terme(&bac, "Contact group", "published", None, &[]).await;

    let p = entier(&bac).await;
    assert!(p.complete);
    assert!(p.removed.faq.is_empty() && p.removed.glossary.is_empty());
    let ids = faq_ids(&p);
    assert!(!ids.contains(&brouillon));
    assert!(ids.contains(&publiee) && ids.contains(&a_revoir));
    let revue = p.faq.iter().find(|e| e.id == a_revoir).unwrap();
    assert_eq!(revue.status, KnowledgeStatus::ToReview);
    assert_eq!(revue.section_code, "first_cop");
    assert!(revue.answer.starts_with("Réponse à"));
    assert_eq!(lexique_ids(&p), vec![contact]);
    assert!(!lexique_ids(&p).contains(&terme_brouillon));
    assert_eq!(p.glossary[0].slug, "contact-group");
    assert_eq!(p.glossary[0].family_code, "meetings");

    let rubriques: Vec<&str> = p.faq_sections.iter().map(|s| s.code.as_str()).collect();
    assert_eq!(
        rubriques,
        ["first_cop", "process", "negotiating_groups", "on_site"]
    );
    assert_eq!(p.faq_sections[0].icon.as_deref(), Some("star"));
    assert_eq!(p.glossary_families.len(), 3);
}

#[tokio::test]
async fn since_rend_le_change_et_les_sorties_le_reste_entier() {
    let bac = Bac::monter().await;
    let relectrice = expert(&bac, "expert@example.org").await;
    let ancienne = entree_faq(&bac, relectrice, "Ancienne", "published").await;
    let depubliee = entree_faq(&bac, relectrice, "Dépubliée", "published").await;
    let jamais = entree_faq(&bac, relectrice, "Jamais publiée", "draft").await;
    let terme_sorti = terme(&bac, "Bracketed text", "published", None, &[]).await;
    let (_, etape_id) = etape(&bac, "Avant de partir", "Réserver l'hôtel").await;
    for (table, id) in [
        ("faq_entries", ancienne),
        ("faq_entries", depubliee),
        ("faq_entries", jamais),
        ("glossary_entries", terme_sorti),
    ] {
        vieillir(&bac, table, id, 60).await;
    }
    let since = OffsetDateTime::now_utc() - Duration::minutes(30);

    statut_faq(&bac, depubliee, "draft").await;
    sqlx::query("UPDATE negotiation.glossary_entries SET status = 'draft' WHERE id = $1")
        .bind(terme_sorti)
        .execute(bac.pool())
        .await
        .unwrap();
    let nouvelle = entree_faq(&bac, relectrice, "Nouvelle", "published").await;

    let d = depuis(&bac, since).await;
    assert!(!d.complete);
    assert_eq!(faq_ids(&d), vec![nouvelle]);
    assert_eq!(d.removed.faq, vec![depubliee]);
    assert_eq!(d.removed.glossary, vec![terme_sorti]);
    assert!(d.glossary.is_empty());
    assert_eq!(
        d.pathway.groups[0].steps[0].id, etape_id,
        "le parcours revient entier"
    );
    assert_eq!(d.faq_sections.len(), 4);
}

#[tokio::test]
async fn la_difference_remonte_de_cinq_minutes_avant_since() {
    let bac = Bac::monter().await;
    let relectrice = expert(&bac, "expert@example.org").await;
    let dans_la_marge = entree_faq(&bac, relectrice, "Dans la marge", "published").await;
    let hors_marge = entree_faq(&bac, relectrice, "Hors marge", "published").await;
    vieillir(&bac, "faq_entries", dans_la_marge, 3).await;
    vieillir(&bac, "faq_entries", hors_marge, 10).await;

    let d = depuis(&bac, OffsetDateTime::now_utc()).await;
    assert_eq!(faq_ids(&d), vec![dans_la_marge]);
}

#[tokio::test]
async fn un_enfant_modifie_fait_partir_son_parent() {
    let bac = Bac::monter().await;
    let relectrice = expert(&bac, "expert@example.org").await;
    let question = entree_faq(&bac, relectrice, "Qui parle pour le groupe ?", "published").await;
    let liee = entree_faq(&bac, relectrice, "Liée", "published").await;
    let contact = terme(&bac, "Contact group", "published", None, &[]).await;
    let informal = terme(&bac, "Informal consultations", "published", None, &[]).await;
    for (table, id) in [
        ("faq_entries", question),
        ("faq_entries", liee),
        ("glossary_entries", contact),
        ("glossary_entries", informal),
    ] {
        vieillir(&bac, table, id, 60).await;
    }
    let since = OffsetDateTime::now_utc() - Duration::minutes(30);

    sqlx::query(
        "INSERT INTO negotiation.knowledge_sources (faq_entry_id, external_title, page_from)
         VALUES ($1, 'Règlement intérieur', 12)",
    )
    .bind(question)
    .execute(bac.pool())
    .await
    .unwrap();
    sqlx::query("INSERT INTO negotiation.glossary_related (entry_id, related_id) VALUES ($1, $2)")
        .bind(contact)
        .bind(informal)
        .execute(bac.pool())
        .await
        .unwrap();

    let d = depuis(&bac, since).await;
    assert_eq!(faq_ids(&d), vec![question]);
    let source = &d.faq[0].sources[0];
    assert_eq!(
        source.external_title.as_deref(),
        Some("Règlement intérieur")
    );
    assert_eq!(source.page_from, Some(12));
    assert_eq!(lexique_ids(&d), vec![contact]);
    assert_eq!(d.glossary[0].related_ids, vec![informal]);
}

#[tokio::test]
async fn l_empreinte_tient_tant_que_rien_ne_change_et_suit_la_langue() {
    let bac = Bac::monter().await;
    let relectrice = expert(&bac, "expert@example.org").await;
    entree_faq(&bac, relectrice, "Première", "published").await;

    let (_, du_paquet) = paquet(&bac.state, "fr", None).await.unwrap();
    let avant = empreinte(&bac.state, "fr").await.unwrap();
    assert_eq!(du_paquet, avant, "le 304 compare la même empreinte");
    assert_eq!(empreinte(&bac.state, "fr").await.unwrap(), avant);
    assert_ne!(empreinte(&bac.state, "en").await.unwrap(), avant);

    entree_faq(&bac, relectrice, "Seconde", "draft").await;
    assert_ne!(empreinte(&bac.state, "fr").await.unwrap(), avant);
}

#[tokio::test]
async fn les_plus_lues_suivent_les_lectures_puis_l_ordre_editorial() {
    let bac = Bac::monter().await;
    let relectrice = expert(&bac, "expert@example.org").await;
    let a = entree_faq(&bac, relectrice, "A", "published").await;
    let b = entree_faq(&bac, relectrice, "B", "published").await;
    let c = entree_faq(&bac, relectrice, "C", "published").await;
    let brouillon = entree_faq(&bac, relectrice, "Brouillon", "draft").await;
    for (id, rang) in [(a, 1), (b, 2), (brouillon, 3)] {
        sqlx::query("UPDATE negotiation.faq_entries SET editorial_rank = $2 WHERE id = $1")
            .bind(id)
            .bind(rang as i16)
            .execute(bac.pool())
            .await
            .unwrap();
    }
    assert_eq!(entier(&bac).await.most_read, vec![a, b]);

    sqlx::query(
        "INSERT INTO negotiation.faq_reads (entry_id, day, count) VALUES
            ($1, current_date, 5), ($2, current_date - 40, 99)",
    )
    .bind(c)
    .bind(b)
    .execute(bac.pool())
    .await
    .unwrap();
    assert_eq!(entier(&bac).await.most_read, vec![c, a, b]);
}

#[tokio::test]
async fn une_entree_publiee_ne_se_supprime_pas_et_le_refus_est_traduit() {
    let bac = Bac::monter().await;
    let relectrice = expert(&bac, "expert@example.org").await;
    let publiee = entree_faq(&bac, relectrice, "Publiée", "published").await;
    statut_faq(&bac, publiee, "draft").await;
    let jamais = entree_faq(&bac, relectrice, "Jamais publiée", "draft").await;
    let publie = terme(&bac, "Contact group", "published", None, &[]).await;

    assert_eq!(
        refus(
            &bac,
            "DELETE FROM negotiation.faq_entries WHERE id = $1",
            publiee
        )
        .await,
        ErrorCode::NegotiationKnowledgePublishedUndeletable
    );
    assert_eq!(
        refus(
            &bac,
            "DELETE FROM negotiation.glossary_entries WHERE id = $1",
            publie
        )
        .await,
        ErrorCode::NegotiationKnowledgePublishedUndeletable
    );
    sqlx::query("DELETE FROM negotiation.faq_entries WHERE id = $1")
        .bind(jamais)
        .execute(bac.pool())
        .await
        .expect("un brouillon jamais publié se supprime");
}

#[tokio::test]
async fn les_autres_refus_du_savoir_sont_traduits() {
    let bac = Bac::monter().await;
    let relectrice = expert(&bac, "expert@example.org").await;
    let question = entree_faq(&bac, relectrice, "Question", "published").await;
    terme(&bac, "Contact group", "draft", None, &[]).await;
    let (groupe, _) = etape(&bac, "En salle", "Lever son drapeau").await;
    let famille = vocabulaire(&bac, "glossary_family", "meetings").await;

    let err = sqlx::query(
        r#"INSERT INTO negotiation.glossary_entries (slug, family_term_id, term, translation, definition)
           VALUES ('', $1, 'Contact  GROUP', '{"fr":"x"}', '{"fr":"y"}')"#,
    )
    .bind(famille)
    .execute(bac.pool())
    .await
    .unwrap_err();
    assert_eq!(
        kernel::pg_error::translate(&err).code,
        ErrorCode::NegotiationGlossarySlugTaken
    );
    assert_eq!(
        refus(
            &bac,
            "DELETE FROM negotiation.pathway_groups WHERE id = $1",
            groupe
        )
        .await,
        ErrorCode::NegotiationPathwayGroupNotEmpty
    );
    assert_eq!(
        refus(
            &bac,
            "INSERT INTO negotiation.faq_related (entry_id, related_id) VALUES ($1, $1)",
            question
        )
        .await,
        ErrorCode::NegotiationRelatedSelf
    );
    assert_eq!(
        refus(
            &bac,
            "UPDATE negotiation.faq_entries SET verified_by = NULL WHERE id = $1",
            question
        )
        .await,
        ErrorCode::ValidationFailed
    );
    assert_eq!(
        refus(
            &bac,
            "INSERT INTO negotiation.knowledge_sources (faq_entry_id) VALUES ($1)",
            question
        )
        .await,
        ErrorCode::NegotiationSourceTargetInvalid
    );
}

#[tokio::test]
async fn glossary_resolve_suit_terme_sigle_variante_slug() {
    let bac = Bac::monter().await;
    let contact = terme(&bac, "Contact group", "published", None, &[]).await;
    let sbi = terme(
        &bac,
        "Subsidiary Body for Implementation",
        "published",
        Some("SBI"),
        &[],
    )
    .await;
    let crochets = terme(
        &bac,
        "Bracketed text",
        "to_review",
        None,
        &["brackets", "SBI"],
    )
    .await;
    let gst = terme(&bac, "GST", "published", None, &[]).await;
    terme(&bac, "Global Stocktake", "published", Some("GST"), &[]).await;
    terme(&bac, "Draft only", "draft", None, &[]).await;

    assert_eq!(
        resoudre(&bac, "Contact Group").await,
        Some(contact),
        "1. le terme"
    );
    assert_eq!(
        resoudre(&bac, "sbi").await,
        Some(sbi),
        "2. le sigle passe avant la variante"
    );
    assert_eq!(
        resoudre(&bac, "Brackets").await,
        Some(crochets),
        "3. la variante"
    );
    assert_eq!(
        resoudre(&bac, "gst").await,
        Some(gst),
        "le terme passe avant le sigle"
    );

    sqlx::query(
        "UPDATE negotiation.glossary_entries SET term = 'Contact groups (informal)' WHERE id = $1",
    )
    .bind(contact)
    .execute(bac.pool())
    .await
    .unwrap();
    assert_eq!(
        resoudre(&bac, "contact-group").await,
        Some(contact),
        "4. le slug, jamais recalculé"
    );

    assert_eq!(
        resoudre(&bac, "Draft only").await,
        None,
        "un brouillon ne se résout pas"
    );
    assert_eq!(
        resoudre(&bac, "contct group").await,
        None,
        "jamais d'approché"
    );
}
