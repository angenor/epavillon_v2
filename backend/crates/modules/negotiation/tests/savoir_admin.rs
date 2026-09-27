//! **Le back-office du savoir**, route par route, en HTTP sur base réelle :
//! `contracts/api-admin-savoir.md`, sections FAQ, Lexique et Parcours.

mod commun;

use actix_web::http::StatusCode;
use commun::documents::{administratrice, expert};
use commun::http::{appel, frapper};
use commun::savoir::entree_faq;
use commun::{personne, Bac};
use negotiation::service::savoir_paquet::paquet;
use serde_json::{json, Value};
use uuid::Uuid;

/// Une requête du back-office, au nom de `$qui`.
macro_rules! http {
    ($app:expr, $verbe:expr, $uri:expr, $qui:expr) => {
        frapper(&$app, appel($verbe, &$uri, Some($qui), None).to_request()).await
    };
    ($app:expr, $verbe:expr, $uri:expr, $qui:expr, $corps:expr) => {
        frapper(
            &$app,
            appel($verbe, &$uri, Some($qui), Some($corps)).to_request(),
        )
        .await
    };
}

fn id(v: &Value) -> Uuid {
    serde_json::from_value(v["id"].clone()).expect("identifiant")
}

async fn aujourdhui(bac: &Bac) -> String {
    sqlx::query_scalar::<_, String>("SELECT ((now() AT TIME ZONE 'Europe/Paris')::date)::text")
        .fetch_one(bac.pool())
        .await
        .expect("date du jour")
}

async fn quelquun(bac: &Bac, email: &str, prenom: &str, nom: &str) -> Uuid {
    let p = personne(bac, email).await;
    sqlx::query("UPDATE identity.people SET first_name = $2, last_name = $3 WHERE id = $1")
        .bind(p)
        .bind(prenom)
        .bind(nom)
        .execute(bac.pool())
        .await
        .expect("nom");
    p
}

#[tokio::test]
async fn une_entree_de_faq_se_redige_se_verifie_se_publie_et_se_depublie() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let experte = expert(&bac, "experte@example.org").await;
    let voisine = entree_faq(&bac, experte, "Qui préside un groupe de contact ?", "draft").await;
    let app = crate::back_office!(bac);

    // Créer : un brouillon, sources et liées posées.
    let (statut, cree) = http!(
        app,
        "post",
        "/admin/negotiation/faq",
        ifdd,
        json!({
            "section_code": "process",
            "question": { "fr": "Qu'est-ce qu'un groupe de contact ?", "en": "What is a contact group?" },
            "sources": [
                { "external_title": "Guide IISD", "page_from": 12, "page_to": 13, "quote": "Un groupe restreint." },
                { "external_title": "Glossaire CCNUCC", "external_url": "https://unfccc.int/glossary" }
            ],
            "related_ids": [voisine]
        })
    );
    assert_eq!(statut, StatusCode::CREATED, "{cree}");
    let entree = id(&cree);
    let fiche = format!("/admin/negotiation/faq/{entree}");
    assert_eq!(
        (
            cree["status"].as_str(),
            cree["section_code"].as_str(),
            cree["answer"].is_null()
        ),
        (Some("draft"), Some("process"), true)
    );
    assert_eq!(cree["sources"].as_array().map(Vec::len), Some(2));
    assert_eq!(cree["sources"][0]["quote"], "Un groupe restreint.");
    assert_eq!(cree["related"][0]["id"], json!(voisine));
    assert_eq!(
        (cree["can_publish"].as_bool(), cree["can_review"].as_bool()),
        (Some(true), Some(false))
    );

    // La liste la trouve par trigrammes, même mal écrite, et filtre par état.
    let (statut, liste) = http!(
        app,
        "get",
        "/admin/negotiation/faq?q=groupe%20de%20contakt&status=draft",
        ifdd
    );
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(liste["entries"][0]["id"], json!(entree), "{liste}");
    let (_, publiees) = http!(app, "get", "/admin/negotiation/faq?status=published", ifdd);
    assert!(publiees["entries"]
        .as_array()
        .expect("liste")
        .iter()
        .all(|e| e["id"] != json!(entree)));

    // Modifier : sources et liées remplacées en bloc.
    let (statut, modifiee) = http!(
        app,
        "patch",
        fiche,
        ifdd,
        json!({
            "answer": { "fr": "Un groupe restreint qui négocie un point précis." },
            "sources": [{ "external_title": "Guide IIED", "section_label": "chapitre 3" }],
            "related_ids": []
        })
    );
    assert_eq!(statut, StatusCode::OK, "{modifiee}");
    assert_eq!(modifiee["sources"].as_array().map(Vec::len), Some(1));
    assert_eq!(modifiee["sources"][0]["section_label"], "chapitre 3");
    assert_eq!(modifiee["related"], json!([]));
    assert_eq!(
        modifiee["question"]["en"], "What is a contact group?",
        "un champ absent ne change rien"
    );

    // Une source à la fois document et référence extérieure est refusée.
    let (statut, refus) = http!(
        app,
        "patch",
        fiche,
        ifdd,
        json!({
            "sources": [{ "document_id": Uuid::now_v7(), "external_title": "Les deux" }]
        })
    );
    assert_eq!(
        (statut, refus["code"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("NEGOTIATION_SOURCE_TARGET_INVALID")
        )
    );
    // Liée à elle-même : refusée.
    let (_, refus) = http!(
        app,
        "patch",
        fiche,
        ifdd,
        json!({ "related_ids": [entree] })
    );
    assert_eq!(refus["code"], "NEGOTIATION_RELATED_SELF");

    // Publier sans vérification : refus en français.
    let (statut, refus) = http!(app, "post", format!("{fiche}/publish"), ifdd);
    assert_eq!(
        (statut, refus["code"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("NEGOTIATION_FAQ_UNVERIFIED")
        )
    );
    assert!(
        refus["message"]
            .as_str()
            .is_some_and(|m| m.contains("vérification")),
        "{refus}"
    );

    // Dater : l'expert, pas l'administratrice.
    let (statut, refus) = http!(app, "post", format!("{fiche}/verify"), ifdd, json!({}));
    assert_eq!(
        (statut, refus["code"].as_str()),
        (StatusCode::FORBIDDEN, Some("FORBIDDEN"))
    );
    let (statut, verifiee) = http!(app, "post", format!("{fiche}/verify"), experte, json!({}));
    assert_eq!(statut, StatusCode::OK, "{verifiee}");
    assert_eq!(verifiee["verified_on"], json!(aujourdhui(&bac).await));
    assert_eq!(verifiee["verified_by_name"], "Awa Diallo");
    assert_eq!(
        (
            verifiee["status"].as_str(),
            verifiee["can_review"].as_bool()
        ),
        (Some("draft"), Some(true))
    );

    // « À revoir » ne vaut que pour une entrée publiée.
    let (statut, _) = http!(app, "post", format!("{fiche}/to-review"), ifdd);
    assert_eq!(statut, StatusCode::CONFLICT);

    let (statut, publiee) = http!(app, "post", format!("{fiche}/publish"), ifdd);
    assert_eq!(
        (statut, publiee["status"].as_str()),
        (StatusCode::OK, Some("published")),
        "{publiee}"
    );
    assert!(publiee["first_published_at"].is_string());
    let (servi, _) = paquet(&bac.state, "fr", None).await.expect("paquet");
    assert!(
        servi.faq.iter().any(|f| f.id == entree),
        "le téléphone la reçoit"
    );

    // « À revoir », puis la vérification la republie, à la date choisie.
    let (_, a_revoir) = http!(app, "post", format!("{fiche}/to-review"), ifdd);
    assert_eq!(a_revoir["status"], "to_review");
    let (_, revue) = http!(
        app,
        "post",
        format!("{fiche}/verify"),
        experte,
        json!({ "verified_on": "2026-11-12" })
    );
    assert_eq!(
        (revue["status"].as_str(), revue["verified_on"].as_str()),
        (Some("published"), Some("2026-11-12"))
    );

    // Publiée une fois : elle ne se supprime plus, elle se dépublie.
    let (statut, refus) = http!(app, "delete", fiche, ifdd);
    assert_eq!(
        (statut, refus["code"].as_str()),
        (
            StatusCode::CONFLICT,
            Some("NEGOTIATION_KNOWLEDGE_PUBLISHED_UNDELETABLE")
        )
    );
    let avant = servi.served_at;
    let (_, depubliee) = http!(app, "post", format!("{fiche}/unpublish"), ifdd);
    assert_eq!(depubliee["status"], "draft");
    let (difference, _) = paquet(&bac.state, "fr", Some(avant))
        .await
        .expect("différence");
    assert!(
        difference.removed.faq.contains(&entree),
        "le téléphone la retire"
    );

    // Un brouillon jamais publié se supprime.
    let (statut, _) = http!(
        app,
        "delete",
        format!("/admin/negotiation/faq/{voisine}"),
        ifdd
    );
    assert_eq!(statut, StatusCode::NO_CONTENT);
    let (statut, refus) = http!(
        app,
        "get",
        format!("/admin/negotiation/faq/{voisine}"),
        ifdd
    );
    assert_eq!(
        (statut, refus["code"].as_str()),
        (StatusCode::NOT_FOUND, Some("NEGOTIATION_FAQ_NOT_FOUND"))
    );

    // Rubrique inconnue, question sans français.
    let (statut, refus) = http!(
        app,
        "post",
        "/admin/negotiation/faq",
        ifdd,
        json!({ "section_code": "nulle", "question": { "fr": "?" } })
    );
    assert_eq!(
        (statut, refus["field"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("section_code"))
    );
    let (_, refus) = http!(
        app,
        "post",
        "/admin/negotiation/faq",
        ifdd,
        json!({ "section_code": "process", "question": { "en": "?" } })
    );
    assert_eq!(refus["field"], "question");
}

#[tokio::test]
async fn la_fiche_compte_les_retours_et_montre_les_signalements_sans_aucun_auteur() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let experte = expert(&bac, "experte@example.org").await;
    let entree = entree_faq(&bac, experte, "Où retirer son badge ?", "published").await;
    let lectrices = [
        quelquun(&bac, "fatou@example.org", "Fatou", "Sow").await,
        quelquun(&bac, "ines@example.org", "Inès", "Martin").await,
        quelquun(&bac, "lea@example.org", "Léa", "Kaboré").await,
    ];
    for (qui, utile, motif) in [
        (lectrices[0], true, None),
        (lectrices[1], false, Some("too_vague")),
        (lectrices[2], false, Some("outdated")),
    ] {
        sqlx::query("INSERT INTO negotiation.faq_feedback (entry_id, person_id, helpful, missing_reason) VALUES ($1, $2, $3, $4)")
            .bind(entree).bind(qui).bind(utile).bind(motif)
            .execute(bac.pool()).await.expect("retour");
    }
    sqlx::query(
        "INSERT INTO negotiation.faq_reports (entry_id, reporter_id, client_ref, reasons, details)
         VALUES ($1, $2, $3, ARRAY['rule_changed','wrong'], 'Le lieu a changé.')",
    )
    .bind(entree)
    .bind(lectrices[2])
    .bind(Uuid::now_v7())
    .execute(bac.pool())
    .await
    .expect("signalement");

    let app = crate::back_office!(bac);
    let (statut, fiche) = http!(app, "get", format!("/admin/negotiation/faq/{entree}"), ifdd);
    assert_eq!(statut, StatusCode::OK);
    assert_eq!(
        fiche["feedback"],
        json!({ "helpful": 1, "not_helpful": 2, "too_vague": 1, "off_topic": 0, "outdated": 1 })
    );
    assert_eq!(
        fiche["reports"][0]["reasons"],
        json!(["rule_changed", "wrong"])
    );
    assert_eq!(fiche["reports"][0]["status"], "open");

    let texte = fiche.to_string();
    for qui in lectrices {
        assert!(
            !texte.contains(&qui.to_string()),
            "l'identifiant d'une lectrice est sorti"
        );
    }
    for nom in [
        "Fatou", "Sow", "Inès", "Martin", "Léa", "Kaboré", "fatou@", "lea@",
    ] {
        assert!(!texte.contains(nom), "« {nom} » est sorti de la fiche");
    }
    let (_, liste) = http!(app, "get", "/admin/negotiation/faq", ifdd);
    let ligne = liste["entries"]
        .as_array()
        .expect("liste")
        .iter()
        .find(|e| e["id"] == json!(entree))
        .cloned();
    assert_eq!(ligne.map(|l| l["open_reports"].clone()), Some(json!(1)));
}

#[tokio::test]
async fn un_terme_garde_son_slug_quoi_quon_envoie() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let experte = expert(&bac, "experte@example.org").await;
    let app = crate::back_office!(bac);

    let (statut, cree) = http!(
        app,
        "post",
        "/admin/negotiation/glossary",
        experte,
        json!({
            "slug": "pirate",
            "family_code": "meetings",
            "term": "Contact group",
            "acronym": "CG",
            "variants": ["contact groups", " ", "contact groups"],
            "translation": { "fr": "groupe de contact" },
            "definition": { "fr": "Groupe restreint chargé d'un point de l'ordre du jour." },
            "heard_in_room": "The contact group will reconvene at 3 p.m.",
            "sources": [{ "external_title": "Glossaire CCNUCC" }]
        })
    );
    assert_eq!(statut, StatusCode::CREATED, "{cree}");
    let terme = id(&cree);
    let fiche = format!("/admin/negotiation/glossary/{terme}");
    assert_eq!(
        (
            cree["slug"].as_str(),
            cree["status"].as_str(),
            cree["variants"].clone()
        ),
        (
            Some("contact-group"),
            Some("draft"),
            json!(["contact groups"])
        )
    );
    assert_eq!(
        (cree["can_publish"].as_bool(), cree["can_review"].as_bool()),
        (Some(true), Some(true))
    );

    let (statut, modifie) = http!(
        app,
        "patch",
        fiche,
        ifdd,
        json!({ "slug": "pirate", "term": "Contact group (informal)", "acronym": null })
    );
    assert_eq!(statut, StatusCode::OK, "{modifie}");
    assert_eq!(
        (
            modifie["slug"].as_str(),
            modifie["term"].as_str(),
            modifie["acronym"].is_null()
        ),
        (
            Some("contact-group"),
            Some("Contact group (informal)"),
            true
        )
    );

    let (statut, refus) = http!(
        app,
        "post",
        "/admin/negotiation/glossary",
        ifdd,
        json!({
            "family_code": "meetings", "term": "CONTACT GROUP (informal)",
            "translation": { "fr": "x" }, "definition": { "fr": "y" }
        })
    );
    assert_eq!(
        (statut, refus["code"].as_str()),
        (
            StatusCode::CONFLICT,
            Some("NEGOTIATION_GLOSSARY_SLUG_TAKEN")
        )
    );

    let (_, liste) = http!(
        app,
        "get",
        "/admin/negotiation/glossary?q=contact%20grup&family=meetings",
        ifdd
    );
    assert_eq!(liste["entries"][0]["id"], json!(terme), "{liste}");

    let (_, publie) = http!(app, "post", format!("{fiche}/publish"), ifdd);
    assert_eq!(publie["status"], "published");
    let (_, a_revoir) = http!(app, "post", format!("{fiche}/to-review"), ifdd);
    assert_eq!(a_revoir["status"], "to_review");
    let (statut, refus) = http!(app, "delete", fiche, ifdd);
    assert_eq!(
        (statut, refus["code"].as_str()),
        (
            StatusCode::CONFLICT,
            Some("NEGOTIATION_KNOWLEDGE_PUBLISHED_UNDELETABLE")
        )
    );
    let (_, depublie) = http!(app, "post", format!("{fiche}/unpublish"), ifdd);
    assert_eq!(depublie["status"], "draft");

    let (statut, refus) = http!(
        app,
        "post",
        "/admin/negotiation/glossary",
        ifdd,
        json!({
            "family_code": "meetings", "term": "Bracketed text", "translation": { "fr": "x" }
        })
    );
    assert_eq!(
        (statut, refus["field"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("definition"))
    );
    let (_, brouillon) = http!(
        app,
        "post",
        "/admin/negotiation/glossary",
        ifdd,
        json!({
            "family_code": "texts", "term": "Bracketed text", "translation": { "fr": "x" }, "definition": { "fr": "y" }
        })
    );
    let (statut, _) = http!(
        app,
        "delete",
        format!("/admin/negotiation/glossary/{}", id(&brouillon)),
        ifdd
    );
    assert_eq!(statut, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn le_parcours_se_compose_s_ordonne_et_protege_les_coches() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let experte = expert(&bac, "experte@example.org").await;
    let lectrice = personne(&bac, "lectrice@example.org").await;
    let faq = entree_faq(&bac, experte, "Où retirer son badge ?", "published").await;
    let app = crate::back_office!(bac);

    let texte = |t: &str| json!({ "fr": t });
    let (statut, p) = http!(
        app,
        "post",
        "/admin/negotiation/pathway/groups",
        ifdd,
        json!({ "label": texte("Avant de partir"), "is_published": true })
    );
    assert_eq!(statut, StatusCode::CREATED, "{p}");
    let avant = id(&p["groups"][0]);
    let (_, p) = http!(
        app,
        "post",
        "/admin/negotiation/pathway/groups",
        ifdd,
        json!({ "label": texte("Le premier jour") })
    );
    let premier_jour = id(&p["groups"][1]);
    assert_eq!(p["groups"][1]["is_published"], false);

    let (statut, p) = http!(
        app,
        "post",
        "/admin/negotiation/pathway/steps",
        ifdd,
        json!({
            "group_id": avant, "label": texte("Retirer son badge"), "is_published": true,
            "link": { "kind": "faq", "target_id": faq, "label": texte("La réponse") }
        })
    );
    assert_eq!(statut, StatusCode::CREATED, "{p}");
    let badge = &p["groups"][0]["steps"][0];
    assert_eq!(
        (
            badge["link"]["kind"].as_str(),
            badge["link"]["target_label"].as_str()
        ),
        (Some("faq"), Some("Où retirer son badge ?"))
    );
    let badge = id(badge);
    let (_, p) = http!(
        app,
        "post",
        "/admin/negotiation/pathway/steps",
        ifdd,
        json!({
            "group_id": avant, "label": texte("Lire le guide"), "detail": texte("Les chapitres 1 à 3.")
        })
    );
    let guide = id(&p["groups"][0]["steps"][1]);

    let (statut, refus) = http!(
        app,
        "post",
        "/admin/negotiation/pathway/steps",
        ifdd,
        json!({
            "group_id": avant, "label": texte("Lien faux"), "link": { "kind": "video", "target_id": faq }
        })
    );
    assert_eq!(
        (statut, refus["code"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("NEGOTIATION_PATHWAY_LINK_INVALID")
        )
    );
    let (_, refus) = http!(
        app,
        "post",
        "/admin/negotiation/pathway/steps",
        ifdd,
        json!({
            "group_id": avant, "label": texte("Page sans document"), "link": { "kind": "faq", "target_id": faq, "page": 3 }
        })
    );
    assert_eq!(refus["code"], "NEGOTIATION_PATHWAY_LINK_INVALID");

    // Ordonner : le premier jour d'abord, le guide passe dans ce groupe.
    let (statut, p) = http!(
        app,
        "put",
        "/admin/negotiation/pathway/order",
        ifdd,
        json!({
            "groups": [{ "id": premier_jour, "step_ids": [guide] }, { "id": avant, "step_ids": [badge] }]
        })
    );
    assert_eq!(statut, StatusCode::OK, "{p}");
    assert_eq!(
        (
            p["groups"][0]["id"].clone(),
            p["groups"][0]["steps"][0]["id"].clone()
        ),
        (json!(premier_jour), json!(guide))
    );
    assert_eq!(p["groups"][1]["steps"].as_array().map(Vec::len), Some(1));

    let (statut, refus) = http!(
        app,
        "delete",
        format!("/admin/negotiation/pathway/groups/{avant}"),
        ifdd
    );
    assert_eq!(
        (statut, refus["code"].as_str()),
        (
            StatusCode::CONFLICT,
            Some("NEGOTIATION_PATHWAY_GROUP_NOT_EMPTY")
        )
    );

    // Une étape cochée se dépublie, elle ne se supprime pas.
    sqlx::query("INSERT INTO negotiation.pathway_checks (person_id, step_id) VALUES ($1, $2)")
        .bind(lectrice)
        .bind(badge)
        .execute(bac.pool())
        .await
        .expect("coche");
    let (statut, refus) = http!(
        app,
        "delete",
        format!("/admin/negotiation/pathway/steps/{badge}"),
        ifdd
    );
    assert_eq!(
        (statut, refus["code"].as_str()),
        (
            StatusCode::CONFLICT,
            Some("NEGOTIATION_KNOWLEDGE_PUBLISHED_UNDELETABLE")
        )
    );
    let (_, p) = http!(
        app,
        "patch",
        format!("/admin/negotiation/pathway/steps/{badge}"),
        ifdd,
        json!({ "is_published": false, "link": null })
    );
    let etape = &p["groups"][1]["steps"][0];
    assert_eq!(
        (
            etape["is_published"].as_bool(),
            etape["link"].is_null(),
            etape["checks"].as_i64()
        ),
        (Some(false), true, Some(1))
    );
    let (servi, _) = paquet(&bac.state, "fr", None).await.expect("paquet");
    assert!(
        servi
            .pathway
            .groups
            .iter()
            .all(|g| g.steps.iter().all(|s| s.id != badge)),
        "le téléphone ne la voit plus"
    );

    let (statut, _) = http!(
        app,
        "delete",
        format!("/admin/negotiation/pathway/steps/{guide}"),
        ifdd
    );
    assert_eq!(statut, StatusCode::NO_CONTENT);
    let (statut, _) = http!(
        app,
        "delete",
        format!("/admin/negotiation/pathway/groups/{premier_jour}"),
        ifdd
    );
    assert_eq!(statut, StatusCode::NO_CONTENT);
    let (_, p) = http!(
        app,
        "patch",
        format!("/admin/negotiation/pathway/groups/{avant}"),
        experte,
        json!({ "label": texte("Avant le départ") })
    );
    assert_eq!(p["groups"][0]["label"]["fr"], "Avant le départ");
    let (_, p) = http!(app, "get", "/admin/negotiation/pathway", experte);
    assert_eq!(p["groups"].as_array().map(Vec::len), Some(1));
}
