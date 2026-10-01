//! **Guide Négo, étape 5 : l'API publique du programme s'enrichit par ajouts.**
//!
//! Le site lit ces routes : tout ce qu'elles rendaient est encore rendu, à
//! l'identique, sauf `attended`, `confirmed_at` et `person_id` des intervenants
//! du détail public (fuite fermée le 26/09). Rien du dossier n'est servi, hors
//! la langue.

mod commun;

use std::collections::BTreeSet;

use actix_web::http::header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH};
use actix_web::http::StatusCode;
use actix_web::{test as atest, web, App};
use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use programme::domain::ids::{EventId, SessionId};
use programme::domain::transitions::ProposalStatus;
use programme::repo::session_parts;
use programme::service::{public_schedule, transition};
use serde_json::Value;
use uuid::Uuid;

const CLES_DAVANT: [&str; 28] = [
    "id",
    "event_id",
    "event_day_id",
    "proposal_id",
    "slug",
    "title",
    "summary",
    "starts_at",
    "ends_at",
    "timezone",
    "format",
    "status",
    "room_id",
    "room_name",
    "organization_id",
    "organization_name",
    "organization_acronym",
    "organization_country_code",
    "organization_country",
    "is_streamed",
    "broadcast_channel_id",
    "capacity",
    "tracks",
    "cover",
    "temporal_state",
    "registered_count",
    "theme_codes",
    "themes",
];

const CLES_AJOUTEES: [&str; 11] = [
    "waitlist_enabled",
    "registration_required",
    "registration_opens_at",
    "registration_closes_at",
    "waitlisted_count",
    "listing_changed_at",
    "language_codes",
    "replay_url",
    "replay_duration_seconds",
    "organization_logo",
    "organization_type_code",
];

const RETIREES_DU_DETAIL: [&str; 3] = ["attended", "confirmed_at", "person_id"];

async fn seance_publiee(bac: &Bac, terrain: &Terrain, slug: &str) -> (Uuid, Uuid) {
    let grille = seances::grille(bac, terrain.edition).await;
    let dossier = seances::dossier_pret(bac, terrain, slug, slug, Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    let id = seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id;
    seances::placer(
        bac,
        terrain.edition,
        id,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE programme.sessions SET published_at = now(), status = 'scheduled' WHERE id = $1",
    )
    .bind(id)
    .execute(bac.pool())
    .await
    .expect("publication");
    (id, dossier.id)
}

async fn programmation(bac: &Bac, terrain: &Terrain) -> Vec<Value> {
    public_schedule::programmation(bac.pool(), Some(EventId(terrain.edition)), None)
        .await
        .unwrap()
        .into_iter()
        .map(|l| serde_json::to_value(l).unwrap())
        .collect()
}

fn cles(objet: &Value) -> BTreeSet<String> {
    objet.as_object().unwrap().keys().cloned().collect()
}

fn toutes_les_cles(valeur: &Value, acc: &mut Vec<String>) {
    match valeur {
        Value::Object(o) => {
            for (k, v) in o {
                acc.push(k.clone());
                toutes_les_cles(v, acc);
            }
        }
        Value::Array(a) => a.iter().for_each(|v| toutes_les_cles(v, acc)),
        _ => {}
    }
}

async fn flux(bac: &Bac, session_id: Uuid, colonnes: &str, valeurs: &str) -> Uuid {
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "INSERT INTO live.streams (session_id, {colonnes}) VALUES ($1, {valeurs}) RETURNING id"
    )))
    .bind(session_id)
    .fetch_one(bac.pool())
    .await
    .expect("flux posé")
}

#[tokio::test]
async fn les_cles_davant_restent_et_les_nouvelles_sajoutent() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let (id, _) = seance_publiee(&bac, &terrain, "atelier-cles").await;

    let lignes = programmation(&bac, &terrain).await;
    assert_eq!(lignes.len(), 1);
    let attendues: BTreeSet<String> = CLES_DAVANT
        .iter()
        .chain(CLES_AJOUTEES.iter())
        .map(|c| (*c).to_owned())
        .collect();
    assert_eq!(
        cles(&lignes[0]),
        attendues,
        "que des ajouts, rien de retiré"
    );
    assert_eq!(lignes[0]["language_codes"], serde_json::json!(["fr"]));
    assert_eq!(lignes[0]["waitlisted_count"], 0);
    assert_eq!(lignes[0]["replay_url"], Value::Null);

    // Le détail : chaque intervenant et chaque organisation rendent ce qu'ils
    // rendaient (la lecture d'avant est celle que garde le back-office), moins
    // les trois champs retirés, plus leurs noms.
    let detail = public_schedule::seance(bac.pool(), EventId(terrain.edition), "atelier-cles")
        .await
        .unwrap();
    let detail = serde_json::to_value(detail).unwrap();
    assert_eq!(cles(&detail["session"]), attendues);

    let avant = session_parts::intervenants(bac.pool(), SessionId::from(id))
        .await
        .unwrap();
    let apres = detail["speakers"].as_array().unwrap();
    assert_eq!(apres.len(), 2);
    assert_eq!(avant.len(), apres.len());
    for (a, p) in avant.iter().zip(apres) {
        for (cle, valeur) in a.as_object().unwrap() {
            if RETIREES_DU_DETAIL.contains(&cle.as_str()) {
                assert!(p.get(cle).is_none(), "{cle} n'est plus servi au public");
            } else {
                assert_eq!(p.get(cle), Some(valeur), "{cle} rendu à l'identique");
            }
        }
    }
    let noms: Vec<&str> = apres
        .iter()
        .map(|s| s["display_name"].as_str().unwrap())
        .collect();
    assert_eq!(noms, ["Moussa Ba", "Léa Martin"]);

    let avant = session_parts::organisations(bac.pool(), SessionId::from(id))
        .await
        .unwrap();
    let apres = detail["organizations"].as_array().unwrap();
    assert_eq!(avant.len(), 2);
    for (a, p) in avant.iter().zip(apres) {
        for (cle, valeur) in a.as_object().unwrap() {
            assert_eq!(p.get(cle), Some(valeur), "{cle} rendu à l'identique");
        }
        for cle in ["name", "acronym", "country_code", "country"] {
            assert!(p.get(cle).is_some(), "{cle} ajouté");
        }
    }
    assert_eq!(apres[0]["name"], "Institut de la Francophonie");
    assert_eq!(apres[0]["acronym"], "IFDD");
}

#[tokio::test]
async fn le_public_ne_recoit_ni_presence_ni_coordonnee_ni_piece() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seance_publiee(&bac, &terrain, "atelier-prive").await;

    let detail = public_schedule::seance(bac.pool(), EventId(terrain.edition), "atelier-prive")
        .await
        .unwrap();
    let detail = serde_json::to_value(detail).unwrap();
    for intervenant in detail["speakers"].as_array().unwrap() {
        for cle in RETIREES_DU_DETAIL {
            assert!(intervenant.get(cle).is_none(), "{cle} servi au public");
        }
    }

    let mut vues = Vec::new();
    toutes_les_cles(&detail, &mut vues);
    for ligne in programmation(&bac, &terrain).await {
        toutes_les_cles(&ligne, &mut vues);
    }
    for cle in &vues {
        for interdit in [
            "email",
            "phone",
            "address",
            "document",
            "attachment",
            "score",
            "objectives",
        ] {
            assert!(!cle.contains(interdit), "« {cle} » ne doit pas être servi");
        }
    }
    let texte = detail.to_string();
    assert!(!texte.contains("@example.org"), "aucun courriel servi");
    assert!(!texte.contains("Pas le matin"), "rien du dossier");
}

#[tokio::test]
async fn deux_rediffusions_ne_font_quune_ligne() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let (id, _) = seance_publiee(&bac, &terrain, "atelier-rediffusion").await;
    assert_eq!(programmation(&bac, &terrain).await.len(), 1);

    let asset: Uuid = sqlx::query_scalar(
        "INSERT INTO media.assets
             (object_key, original_filename, mime_type, byte_size, checksum_sha256,
              status, owner_organization_id, scan_verdict, scanned_at, duration_seconds)
         VALUES ('rediffusions/atelier.mp4', 'atelier.mp4', 'video/mp4', 9000000,
                 repeat('b', 64), 'ready', $1, 'clean', now(), 3125.4)
         RETURNING id",
    )
    .bind(terrain.organisation)
    .fetch_one(bac.pool())
    .await
    .expect("enregistrement archivé");

    // Écartées : annulée, pas encore disponible.
    flux(
        &bac,
        id,
        "kind, status, watch_url, is_primary",
        "'replay', 'cancelled', 'https://video.example/annulee', true",
    )
    .await;
    flux(
        &bac,
        id,
        "kind, watch_url, is_primary, replay_available_at",
        "'replay', 'https://video.example/demain', true, now() + interval '1 day'",
    )
    .await;
    // Deux disponibles : la principale l'emporte.
    flux(
        &bac,
        id,
        "kind, replay_url, watch_url, is_primary, replay_available_at",
        "'replay', 'https://video.example/secondaire', 'https://video.example/s', false, now() - interval '1 hour'",
    )
    .await;
    let principale = flux(
        &bac,
        id,
        "kind, watch_url, is_primary",
        "'replay', 'https://video.example/principale', true",
    )
    .await;
    sqlx::query("UPDATE live.streams SET recording_asset_id = $1 WHERE id = $2")
        .bind(asset)
        .bind(principale)
        .execute(bac.pool())
        .await
        .expect("enregistrement rattaché");

    let lignes = programmation(&bac, &terrain).await;
    assert_eq!(lignes.len(), 1, "autant de lignes qu'avant");
    assert_eq!(lignes[0]["replay_url"], "https://video.example/principale");
    assert_eq!(lignes[0]["replay_duration_seconds"], 3125);
}

#[tokio::test]
async fn un_direct_devenu_consultable_donne_sa_duree_de_diffusion() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let (id, _) = seance_publiee(&bac, &terrain, "atelier-direct").await;

    flux(
        &bac,
        id,
        "kind, status, watch_url, started_at, ended_at, is_primary",
        "'live', 'ended', 'https://video.example/direct', now() - interval '2 hours', now() - interval '68 minutes', true",
    )
    .await;
    assert_eq!(
        programmation(&bac, &terrain).await[0]["replay_url"],
        Value::Null,
        "un direct sans replay_url n'est pas une rediffusion"
    );

    sqlx::query(
        "UPDATE live.streams SET replay_url = 'https://video.example/replay' WHERE session_id = $1",
    )
    .bind(id)
    .execute(bac.pool())
    .await
    .unwrap();
    let ligne = programmation(&bac, &terrain).await.remove(0);
    assert_eq!(ligne["replay_url"], "https://video.example/replay");
    assert_eq!(ligne["replay_duration_seconds"], 52 * 60);
}

#[tokio::test]
async fn langue_nulle_sans_dossier_et_liste_dattente_comptee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let (id, _) = seance_publiee(&bac, &terrain, "atelier-attente").await;

    sqlx::query(
        "UPDATE programme.sessions
            SET capacity = 1, waitlist_enabled = true, registration_required = true,
                registration_closes_at = now() + interval '1 day'
          WHERE id = $1",
    )
    .bind(id)
    .execute(bac.pool())
    .await
    .unwrap();
    for (courriel, prenom) in [("a@inscrits.test", "Awa"), ("b@inscrits.test", "Binta")] {
        let personne = commun::personne(&bac, courriel, prenom, "Sow").await;
        sqlx::query("INSERT INTO programme.registrations (session_id, person_id) VALUES ($1, $2)")
            .bind(id)
            .bind(personne)
            .execute(bac.pool())
            .await
            .expect("inscription");
    }

    let ligne = programmation(&bac, &terrain).await.remove(0);
    assert_eq!(ligne["registered_count"], 1);
    assert_eq!(ligne["waitlisted_count"], 1);
    assert_eq!(ligne["waitlist_enabled"], true);
    assert_eq!(ligne["registration_required"], true);
    assert!(ligne["registration_closes_at"].is_string());
    assert!(ligne["listing_changed_at"].is_string());

    sqlx::query("UPDATE programme.sessions SET proposal_id = NULL WHERE id = $1")
        .bind(id)
        .execute(bac.pool())
        .await
        .expect("séance sans dossier");
    let ligne = programmation(&bac, &terrain).await.remove(0);
    assert_eq!(ligne["language_codes"], Value::Null);
}

#[tokio::test]
async fn la_programmation_rend_304_sur_son_empreinte() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seance_publiee(&bac, &terrain, "atelier-empreinte").await;

    let app = atest::init_service(
        App::new()
            .app_data(web::Data::new(bac.state.clone()))
            .configure(programme::routes::public_schedule::configurer),
    )
    .await;
    let uri = format!("/schedule?event_id={}", terrain.edition);

    let premiere =
        atest::call_service(&app, atest::TestRequest::get().uri(&uri).to_request()).await;
    assert_eq!(premiere.status(), StatusCode::OK);
    let empreinte = premiere
        .headers()
        .get(ETAG)
        .expect("ETag")
        .to_str()
        .unwrap()
        .to_owned();
    let corps: Value = atest::read_body_json(premiere).await;
    assert_eq!(corps.as_array().map(Vec::len), Some(1));

    let seconde = atest::call_service(
        &app,
        atest::TestRequest::get()
            .uri(&uri)
            .insert_header((IF_NONE_MATCH, empreinte.clone()))
            .to_request(),
    )
    .await;
    assert_eq!(seconde.status(), StatusCode::NOT_MODIFIED);

    let autre = atest::call_service(
        &app,
        atest::TestRequest::get()
            .uri(&uri)
            .insert_header((IF_NONE_MATCH, "\"00000000000000000000000000000000\""))
            .to_request(),
    )
    .await;
    assert_eq!(autre.status(), StatusCode::OK);
}

#[tokio::test]
async fn mes_inscriptions_se_revalident_en_prive() {
    let corps = serde_json::json!([{ "id": "une-inscription" }]);
    let sans = atest::TestRequest::get().to_http_request();
    let reponse = programme::routes::json_revalide(&sans, &corps, "private, no-cache").unwrap();
    assert_eq!(reponse.status(), StatusCode::OK);
    assert_eq!(
        reponse.headers().get(CACHE_CONTROL).unwrap(),
        "private, no-cache"
    );
    let empreinte = reponse
        .headers()
        .get(ETAG)
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let avec = atest::TestRequest::get()
        .insert_header((IF_NONE_MATCH, format!("W/{empreinte}")))
        .to_http_request();
    let reponse = programme::routes::json_revalide(&avec, &corps, "private, no-cache").unwrap();
    assert_eq!(reponse.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(
        reponse.headers().get(CACHE_CONTROL).unwrap(),
        "private, no-cache"
    );
}
