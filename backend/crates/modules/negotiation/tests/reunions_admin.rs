//! **Le back-office des réunions** (US3) — la garde sur la portée globale,
//! ce que pose le serveur, la publication et ses refus nommés, l'annulation,
//! le lien au Pavillon, la capacité relevée qui promeut, les chevauchements
//! acceptés (FR-019).

#[macro_use]
mod reunions;

use actix_web::http::{Method, StatusCode};
use reunions::{inscrire, requete, Decor, LIEN};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

fn instant(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).expect("date")
}

fn corps(nature: Option<&str>, format: &str) -> Value {
    let debut = OffsetDateTime::now_utc() + Duration::days(7);
    json!({
        "edition": "cop31",
        "type": nature,
        "title": { "fr": "Atelier préparatoire", "en": "Preparatory workshop" },
        "description": null,
        "start_at": instant(debut),
        "end_at": instant(debut + Duration::hours(2)),
        "format": format,
        "venue": null,
        "external_url": null,
        "capacity": null,
        "waitlist_enabled": true,
        "requires_registration": true,
        "registration_opens_at": null,
        "registration_closes_at": null,
        "open_access": true,
        "access_audience": null,
        "is_ifdd_organized": true,
        "organizer_org_id": null
    })
}

async fn administratrice(d: &Decor) -> Uuid {
    let id = d.personne("admin@example.org", false).await;
    d.role(id, "admin", "global", None).await;
    id
}

async fn espace_climat(d: &Decor) -> Uuid {
    sqlx::query_scalar("SELECT id FROM negotiation.spaces WHERE slug::text = 'climat'")
        .fetch_one(d.pool())
        .await
        .expect("espace")
}

async fn activite(d: &Decor, edition: Uuid, titre: &str) -> Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO programme.sessions (event_id, title, slug, status, starts_at, ends_at, format, timezone)
           VALUES ($1, jsonb_build_object('fr', $2::text), $3::text::platform.slug, 'scheduled',
                   now() + interval '8 days', now() + interval '8 days 1 hour', 'in_person',
                   (SELECT e.timezone FROM event.events e WHERE e.id = $1))
           RETURNING id"#,
    )
    .bind(edition)
    .bind(titre)
    .bind(format!("a-{}", Uuid::now_v7().simple()))
    .fetch_one(d.pool())
    .await
    .expect("activité")
}

#[tokio::test]
async fn seule_la_portee_globale_passe_la_garde() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let admin = administratrice(&d).await;
    let sans = d.personne("sans@example.org", true).await;
    let dedition = d.personne("edition@example.org", false).await;
    d.role(dedition, "admin", "event", Some(d.edition)).await;
    let animatrice = d.personne("lead@example.org", false).await;
    d.role(
        animatrice,
        "space_lead",
        "negotiation_space",
        Some(espace_climat(&d).await),
    )
    .await;

    let creee = frapper!(
        &app,
        requete(Method::POST, "/admin/negotiation/meetings", Some(admin))
            .set_json(corps(Some("preparatory_workshop"), "online"))
    );
    assert_eq!(creee.statut, StatusCode::CREATED, "{}", creee.brut);
    let id = creee.corps["id"].as_str().expect("id").to_owned();

    // L'adresse forgée vise une réunion qui existe : la garde tombe avant.
    let routes = [
        (
            Method::GET,
            "/admin/negotiation/meetings?edition=cop31".to_owned(),
            None,
        ),
        (
            Method::POST,
            "/admin/negotiation/meetings".to_owned(),
            Some(corps(Some("preparatory_workshop"), "online")),
        ),
        (
            Method::GET,
            format!("/admin/negotiation/meetings/{id}"),
            None,
        ),
        (
            Method::PUT,
            format!("/admin/negotiation/meetings/{id}"),
            Some(corps(Some("preparatory_workshop"), "online")),
        ),
        (
            Method::POST,
            format!("/admin/negotiation/meetings/{id}/publish"),
            None,
        ),
        (
            Method::POST,
            format!("/admin/negotiation/meetings/{id}/cancel"),
            Some(json!({ "reason": "Forgée." })),
        ),
        (
            Method::PUT,
            format!("/admin/negotiation/meetings/{id}/pavilion"),
            Some(json!({ "pavilion_session_id": null })),
        ),
        (
            Method::GET,
            format!("/admin/negotiation/meetings/{id}/registrations"),
            None,
        ),
        (
            Method::GET,
            "/admin/negotiation/pavilion-activities?edition=cop31".to_owned(),
            None,
        ),
    ];
    for (acteur, attendu) in [
        (Some(sans), StatusCode::FORBIDDEN),
        (Some(dedition), StatusCode::FORBIDDEN),
        (Some(animatrice), StatusCode::FORBIDDEN),
        (None, StatusCode::UNAUTHORIZED),
    ] {
        for (methode, uri, charge) in &routes {
            let r = requete(methode.clone(), uri, acteur);
            let r = match charge {
                Some(c) => r.set_json(c),
                None => r,
            };
            let reponse = frapper!(&app, r);
            assert_eq!(
                reponse.statut, attendu,
                "{methode} {uri} : {}",
                reponse.brut
            );
        }
    }

    let statut: String =
        sqlx::query_scalar("SELECT status::text FROM negotiation.meetings WHERE id = $1::uuid")
            .bind(&id)
            .fetch_one(d.pool())
            .await
            .expect("statut");
    assert_eq!(statut, "draft", "aucune porte forgée n'a écrit");
    let lues = frapper!(
        &app,
        requete(
            Method::GET,
            "/admin/negotiation/meetings?edition=cop31",
            Some(admin)
        )
    );
    assert_eq!(lues.corps["meetings"].as_array().map(Vec::len), Some(1));
}

#[tokio::test]
async fn la_saisie_pose_ce_que_le_serveur_decide() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let admin = administratrice(&d).await;

    let r = frapper!(
        &app,
        requete(Method::POST, "/admin/negotiation/meetings", Some(admin))
            .set_json(corps(Some("preparatory_workshop"), "online"))
    );
    assert_eq!(r.statut, StatusCode::CREATED, "{}", r.brut);
    assert_eq!(r.corps["status"], "draft");
    assert_eq!(r.corps["edition"], "cop31");
    assert_eq!(r.corps["timezone"], "Asia/Istanbul");
    assert_eq!(r.corps["type"]["code"], "preparatory_workshop");
    assert_eq!(r.corps["organizer"], "IFDD");
    assert!(r.corps["slug"]
        .as_str()
        .is_some_and(|s| s.starts_with("cop31-preparatory-workshop-")));

    let (kind, espace, evenement, auteur): (String, Uuid, Uuid, Uuid) = sqlx::query_as(
        "SELECT kind::text, space_id, event_id, created_by FROM negotiation.meetings
          WHERE id = $1::uuid",
    )
    .bind(r.corps["id"].as_str())
    .fetch_one(d.pool())
    .await
    .expect("réunion");
    assert_eq!(kind, "preparatory_workshop");
    assert_eq!(espace, espace_climat(&d).await);
    assert_eq!(evenement, d.edition);
    assert_eq!(auteur, admin);

    // Une concertation prend l'autre `kind`, et une organisation son nom.
    let org = d.organisation("Réseau Climat Sahel").await;
    let mut c = corps(Some("ministerial_consultation"), "onsite");
    c["is_ifdd_organized"] = json!(false);
    c["organizer_org_id"] = json!(org);
    c["open_access"] = json!(false);
    c["access_audience"] = json!({ "fr": "Ministres et chefs de délégation" });
    let r = frapper!(
        &app,
        requete(Method::POST, "/admin/negotiation/meetings", Some(admin)).set_json(c)
    );
    assert_eq!(r.statut, StatusCode::CREATED, "{}", r.brut);
    assert_eq!(r.corps["organizer"], "Réseau Climat Sahel");
    assert_eq!(r.corps["open_access"], false);
    let kind: String =
        sqlx::query_scalar("SELECT kind::text FROM negotiation.meetings WHERE id = $1::uuid")
            .bind(r.corps["id"].as_str())
            .fetch_one(d.pool())
            .await
            .expect("kind");
    assert_eq!(kind, "francophone_consultation");

    // Accès limité sans public : le champ est nommé.
    let mut c = corps(Some("negotiators_consultation"), "online");
    c["open_access"] = json!(false);
    let r = frapper!(
        &app,
        requete(Method::POST, "/admin/negotiation/meetings", Some(admin)).set_json(c)
    );
    assert_eq!(r.statut, StatusCode::BAD_REQUEST);
    assert_eq!(r.corps["code"], "NEGOTIATION_MEETING_INVALID");
    assert_eq!(r.corps["field"], "access_audience");
}

macro_rules! creer {
    ($app:expr, $admin:expr, $c:expr) => {{
        let r = frapper!(
            $app,
            requete(Method::POST, "/admin/negotiation/meetings", Some($admin)).set_json($c)
        );
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.brut);
        r.corps["id"].as_str().expect("id").to_owned()
    }};
}

#[tokio::test]
async fn publier_nomme_le_champ_qui_manque() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let admin = administratrice(&d).await;
    let publier = |id: &str| {
        requete(
            Method::POST,
            &format!("/admin/negotiation/meetings/{id}/publish"),
            Some(admin),
        )
    };

    for (c, champ) in [
        (
            corps(Some("preparatory_workshop"), "online"),
            "external_url",
        ),
        (corps(Some("preparatory_workshop"), "onsite"), "venue"),
        (
            {
                let mut c = corps(None, "online");
                c["external_url"] = json!(LIEN);
                c
            },
            "type",
        ),
    ] {
        let id = creer!(&app, admin, c);
        let r = frapper!(&app, publier(&id));
        assert_eq!(r.statut, StatusCode::BAD_REQUEST, "{}", r.brut);
        assert_eq!(r.corps["code"], "NEGOTIATION_MEETING_INVALID");
        assert_eq!(r.corps["field"], champ);
    }

    let mut c = corps(Some("preparatory_workshop"), "online");
    c["external_url"] = json!(LIEN);
    let id = creer!(&app, admin, c);
    let publiques = frapper!(
        &app,
        requete(Method::GET, "/negotiation/meetings?edition=cop31", None)
    );
    assert_eq!(
        publiques.corps["meetings"],
        json!([]),
        "un brouillon ne se lit pas"
    );

    let r = frapper!(&app, publier(&id));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
    assert_eq!(r.corps["status"], "scheduled");
    let publiques = frapper!(
        &app,
        requete(Method::GET, "/negotiation/meetings?edition=cop31", None)
    );
    assert_eq!(publiques.corps["meetings"][0]["id"], json!(id));
}

#[tokio::test]
async fn annuler_demande_un_motif() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let admin = administratrice(&d).await;
    let mut c = corps(Some("negotiators_consultation"), "online");
    c["external_url"] = json!(LIEN);
    let id = creer!(&app, admin, c);
    let annuler = |motif: &str| {
        requete(
            Method::POST,
            &format!("/admin/negotiation/meetings/{id}/cancel"),
            Some(admin),
        )
        .set_json(json!({ "reason": motif }))
    };

    let r = frapper!(&app, annuler("  "));
    assert_eq!(r.statut, StatusCode::BAD_REQUEST);
    assert_eq!(r.corps["field"], "reason");

    let r = frapper!(&app, annuler("Reportée à la prochaine session."));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
    assert_eq!(r.corps["status"], "cancelled");
    assert_eq!(
        r.corps["cancellation_reason"],
        "Reportée à la prochaine session."
    );

    let r = frapper!(
        &app,
        requete(
            Method::POST,
            &format!("/admin/negotiation/meetings/{id}/publish"),
            Some(admin)
        )
    );
    assert_eq!(
        r.statut,
        StatusCode::CONFLICT,
        "une annulée ne se republie pas"
    );
}

async fn ecritures_dans_programme(d: &Decor) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM platform.audit_log WHERE entity_schema = 'programme'")
        .fetch_one(d.pool())
        .await
        .expect("audit")
}

#[tokio::test]
async fn le_lien_au_pavillon_reste_dans_ledition_et_tombe_avec_lactivite() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let admin = administratrice(&d).await;
    let ici = activite(&d, d.edition, "Journée de la Francophonie").await;
    let ailleurs_edition: Uuid = sqlx::query_scalar(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode, timezone, starts_at, ends_at)
           VALUES (2027, '{"fr":"COP32"}'::jsonb, 'cop32', '{"fr":"Description."}'::jsonb,
                   'online', 'UTC', '2027-11-09T00:00:00Z', '2027-11-20T00:00:00Z')
           RETURNING id"#,
    )
    .fetch_one(d.pool())
    .await
    .expect("édition");
    let ailleurs = activite(&d, ailleurs_edition, "Autre COP").await;
    let id = creer!(&app, admin, corps(Some("preparatory_workshop"), "online"));

    let options = frapper!(
        &app,
        requete(
            Method::GET,
            "/admin/negotiation/pavilion-activities?edition=cop31",
            Some(admin)
        )
    );
    assert_eq!(options.corps.as_array().map(Vec::len), Some(1));
    assert_eq!(options.corps[0]["id"], json!(ici));

    let avant = ecritures_dans_programme(&d).await;
    let lier = |activite: Option<Uuid>| {
        requete(
            Method::PUT,
            &format!("/admin/negotiation/meetings/{id}/pavilion"),
            Some(admin),
        )
        .set_json(json!({ "pavilion_session_id": activite }))
    };
    let r = frapper!(&app, lier(Some(ici)));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
    assert_eq!(r.corps["pavilion_session_id"], json!(ici));

    let r = frapper!(&app, lier(Some(ailleurs)));
    assert_eq!(r.statut, StatusCode::BAD_REQUEST);
    assert_eq!(r.corps["field"], "pavilion_session_id");

    assert_eq!(
        ecritures_dans_programme(&d).await,
        avant,
        "aucune écriture dans programme"
    );

    sqlx::query("DELETE FROM programme.sessions WHERE id = $1")
        .bind(ici)
        .execute(d.pool())
        .await
        .expect("suppression");
    let r = frapper!(
        &app,
        requete(
            Method::GET,
            &format!("/admin/negotiation/meetings/{id}"),
            Some(admin)
        )
    );
    assert_eq!(r.corps["pavilion_session_id"], Value::Null, "le lien tombe");
}

#[tokio::test]
async fn relever_la_capacite_promeut_la_liste_dattente() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let admin = administratrice(&d).await;
    let awa = d.personne("awa@example.org", true).await;
    let bea = d.personne("bea@example.org", true).await;
    let mut c = corps(Some("negotiators_consultation"), "online");
    c["external_url"] = json!(LIEN);
    c["capacity"] = json!(1);
    let id = creer!(&app, admin, c.clone());
    frapper!(
        &app,
        requete(
            Method::POST,
            &format!("/admin/negotiation/meetings/{id}/publish"),
            Some(admin)
        )
    );
    let reunion = Uuid::parse_str(&id).expect("uuid");
    frapper!(&app, inscrire(Some(awa), reunion, Uuid::now_v7()));
    frapper!(&app, inscrire(Some(bea), reunion, Uuid::now_v7()));

    let inscrites = |app_id: &str| {
        requete(
            Method::GET,
            &format!("/admin/negotiation/meetings/{app_id}/registrations"),
            Some(admin),
        )
    };
    let r = frapper!(&app, inscrites(&id));
    assert_eq!(r.corps["registered"][0]["person_id"], json!(awa));
    assert_eq!(r.corps["waitlisted"][0]["person_id"], json!(bea));
    assert_eq!(r.corps["waitlisted"][0]["waitlist_position"], 1);
    assert_eq!(r.corps["registered"][0]["name"], "Awa Diallo");

    c["capacity"] = json!(2);
    let r = frapper!(
        &app,
        requete(
            Method::PUT,
            &format!("/admin/negotiation/meetings/{id}"),
            Some(admin)
        )
        .set_json(c)
    );
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
    assert_eq!(r.corps["registered_count"], 2);
    assert_eq!(r.corps["waitlisted_count"], 0);
    assert_eq!(
        d.ligne(reunion, bea).await,
        Some(("registered".to_owned(), None))
    );
}

#[tokio::test]
async fn deux_reunions_qui_se_chevauchent_sont_acceptees() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let admin = administratrice(&d).await;
    let mut c = corps(Some("preparatory_workshop"), "online");
    c["external_url"] = json!(LIEN);
    for _ in 0..2 {
        let id = creer!(&app, admin, c.clone());
        let r = frapper!(
            &app,
            requete(
                Method::POST,
                &format!("/admin/negotiation/meetings/{id}/publish"),
                Some(admin)
            )
        );
        assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
    }
    let r = frapper!(
        &app,
        requete(Method::GET, "/negotiation/meetings?edition=cop31", None)
    );
    assert_eq!(r.corps["meetings"].as_array().map(Vec::len), Some(2));
}
