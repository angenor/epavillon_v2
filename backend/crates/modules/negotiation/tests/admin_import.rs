//! Le back-office de l'import (FR-040, US2) : allumer pose une lecture, « Lire
//! maintenant » lit sans doubler la chaîne, éteindre coupe, le seuil vaut dès la
//! lecture suivante, un réglage incomplet est refusé, un point se rattache.

mod commun;

use actix_web::http::StatusCode;
use commun::documents::administratrice;
use commun::Bac;
use kernel::error::ApiError;
use kernel::jobs::{ClaimedJob, JobHandler};
use negotiation::domain::admin_import::UpdateOfficialImportPayload;
use negotiation::jobs::import::{ImportOfficialSessions, IMPORT_OFFICIAL_SESSIONS};
use negotiation::service::{admin_import, sessions};
use serde_json::Value;
use uuid::Uuid;

const SLUG: &str = "cop31-import";

struct Essai {
    bac: Bac,
    ifdd: Uuid,
    edition: Uuid,
}

/// Une édition sans ligne d'import : allumer doit la créer.
async fn monter() -> Essai {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let edition = sqlx::query_scalar(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode, timezone, starts_at, ends_at)
           VALUES (2026, '{"fr":"COP31"}'::jsonb, $1::text::platform.slug, '{"fr":"COP31"}'::jsonb,
                   'online', 'Asia/Istanbul', '2026-11-09T00:00:00Z', '2026-11-20T00:00:00Z')
           RETURNING id"#,
    )
    .bind(SLUG)
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition");
    Essai { bac, ifdd, edition }
}

fn reglage(allume: bool) -> UpdateOfficialImportPayload {
    UpdateOfficialImportPayload {
        enabled: allume,
        reader: "archive".into(),
        archive_name: Some("cop30/lecture-1".into()),
        archive_first_day: Some(time::macros::date!(2026 - 11 - 09)),
        live_url: None,
        time_correction_minutes: 60,
        official_programme_url: "https://unfccc.int/cop31/schedule".into(),
        // Le créneau suivant tombe à minuit UTC : aucune lecture de la chaîne
        // ne devient due pendant l'essai.
        interval_seconds: 86_400,
        missed_threshold: 3,
    }
}

impl Essai {
    async fn regler(&self, r: UpdateOfficialImportPayload) -> Result<Value, ApiError> {
        admin_import::regler(
            &self.bac.state,
            &self.bac.ctx(self.ifdd),
            self.ifdd,
            SLUG,
            r,
        )
        .await
        .map(|etat| serde_json::to_value(etat).expect("sérialisation"))
    }

    async fn lire_maintenant(&self) {
        admin_import::lire_maintenant(&self.bac.state, &self.bac.ctx(self.ifdd), SLUG)
            .await
            .expect("lecture posée");
    }

    async fn etat(&self) -> Value {
        serde_json::to_value(
            admin_import::lire(&self.bac.state, SLUG)
                .await
                .expect("état"),
        )
        .expect("sérialisation")
    }

    /// Ce que le worker ferait : chaque lecture due, dans l'ordre de sa date.
    /// Cinq secondes de marge : l'horloge de la base n'est pas celle du test.
    async fn travailler(&self) -> usize {
        let dues: Vec<(Uuid, Value)> = sqlx::query_as(
            "SELECT id, payload FROM platform.jobs
              WHERE task = $1 AND status = 'queued' AND run_at <= now() + interval '5 seconds'
              ORDER BY run_at, id",
        )
        .bind(IMPORT_OFFICIAL_SESSIONS)
        .fetch_all(self.bac.pool())
        .await
        .expect("travaux dus");
        for (id, payload) in &dues {
            let travail = ClaimedJob {
                id: *id,
                queue: "default".into(),
                task: IMPORT_OFFICIAL_SESSIONS.into(),
                payload: payload.clone(),
                attempts: 1,
                max_attempts: 5,
            };
            ImportOfficialSessions::new(self.bac.db(), false)
                .run(&travail)
                .await
                .expect("le travail d'import réussit toujours");
            sqlx::query("UPDATE platform.jobs SET status = 'succeeded' WHERE id = $1")
                .bind(id)
                .execute(self.bac.pool())
                .await
                .expect("travail clos");
        }
        dues.len()
    }

    /// Les clés en file, chaîne et lectures manuelles confondues.
    async fn en_file(&self) -> Vec<String> {
        sqlx::query_scalar(
            "SELECT idempotency_key FROM platform.jobs
              WHERE task = $1 AND status = 'queued' AND payload ->> 'event_id' = $2::text",
        )
        .bind(IMPORT_OFFICIAL_SESSIONS)
        .bind(self.edition)
        .fetch_all(self.bac.pool())
        .await
        .expect("travaux en file")
    }

    async fn servi(&self) -> bool {
        let s = sessions::de_ledition(&self.bac.state, SLUG)
            .await
            .expect("sessions publiques");
        !s.sessions.is_empty()
    }
}

#[tokio::test]
async fn allumer_cree_limport_et_pose_la_premiere_lecture() {
    let e = monter().await;
    let etat = e.etat().await;
    assert_eq!(
        (
            etat["enabled"].as_bool(),
            etat["runs"].as_array().map(Vec::len)
        ),
        (Some(false), Some(0)),
        "sans ligne, l'import se lit éteint"
    );
    assert!(etat["archives"]
        .as_array()
        .expect("jeux")
        .contains(&"cop30/lecture-1".into()));

    let etat = e.regler(reglage(true)).await.expect("allumé");
    assert_eq!(etat["enabled"], true);
    assert_eq!(e.en_file().await.len(), 1, "la première lecture est posée");

    assert_eq!(e.travailler().await, 1);
    let etat = e.etat().await;
    assert_eq!(etat["serving"], true);
    assert_eq!(etat["session_count"], 44);
    assert_eq!(etat["runs"][0]["outcome"], "success");
    assert_eq!(etat["runs"][0]["manual"], false);
    assert_eq!(e.en_file().await.len(), 1, "la chaîne a posé la suivante");
    assert!(e.servi().await);
}

#[tokio::test]
async fn deux_lectures_manuelles_font_deux_lectures_sans_doubler_la_chaine() {
    let e = monter().await;
    e.regler(reglage(true)).await.expect("allumé");
    e.travailler().await;
    let chaine = e.en_file().await;
    assert_eq!(chaine.len(), 1);

    e.lire_maintenant().await;
    e.lire_maintenant().await;
    assert_eq!(e.travailler().await, 2, "deux lectures, pas une");

    let etat = e.etat().await;
    let runs = etat["runs"].as_array().expect("journal");
    assert_eq!(runs.len(), 3);
    assert!(runs[..2].iter().all(|r| r["manual"] == true));
    assert_eq!(runs[0]["change_count"], 0, "rien n'a changé à la source");
    assert_eq!(e.en_file().await, chaine, "la chaîne reste unique");

    // Rallumer sans éteindre ne double pas davantage la chaîne.
    e.regler(reglage(true)).await.expect("réglage reposé");
    assert_eq!(e.en_file().await, chaine);
}

#[tokio::test]
async fn eteindre_coupe_laffichage_aussitot() {
    let e = monter().await;
    e.regler(reglage(true)).await.expect("allumé");
    e.travailler().await;
    assert!(e.servi().await);

    let chaine = e.en_file().await;
    let etat = e.regler(reglage(false)).await.expect("éteint");
    assert_eq!(etat["serving"], false);
    assert!(
        !e.servi().await,
        "aucune session servie, sans attendre de lecture"
    );

    // Éteint, « Lire maintenant » lit, et l'affichage reste coupé.
    e.lire_maintenant().await;
    assert_eq!(e.travailler().await, 1);
    let etat = e.etat().await;
    assert_eq!(
        (
            etat["runs"][0]["manual"].as_bool(),
            etat["runs"].as_array().map(Vec::len)
        ),
        (Some(true), Some(2))
    );
    assert!(!e.servi().await);
    assert_eq!(
        e.en_file().await,
        chaine,
        "la lecture manuelle ne replanifie rien"
    );
}

#[tokio::test]
async fn le_seuil_vaut_des_la_lecture_suivante() {
    let e = monter().await;
    e.regler(reglage(true)).await.expect("allumé");
    e.travailler().await;

    let mut r = reglage(true);
    r.archive_name = Some("cop30/injoignable".into());
    r.missed_threshold = 1;
    let etat = e.regler(r).await.expect("seuil abaissé");
    assert_eq!(
        etat["serving"], true,
        "le seuil ne coupe rien avant une lecture"
    );

    e.lire_maintenant().await;
    e.travailler().await;
    let etat = e.etat().await;
    assert_eq!(
        (etat["serving"].as_bool(), etat["missed_reads"].as_i64()),
        (Some(false), Some(1))
    );
    assert_eq!(etat["runs"][0]["outcome"], "failure");
    assert!(etat["last_error"].is_string());
}

#[tokio::test]
async fn un_reglage_incomplet_est_refuse_et_nomme_son_champ() {
    let e = monter().await;
    let mut cas = Vec::new();

    let mut r = reglage(true);
    r.reader = "live".into();
    cas.push((r, "live_url"));
    let mut r = reglage(true);
    r.archive_name = Some("cop30/inconnu".into());
    cas.push((r, "archive_name"));
    let mut r = reglage(true);
    r.interval_seconds = 30;
    cas.push((r, "interval_seconds"));
    let mut r = reglage(true);
    r.missed_threshold = 0;
    cas.push((r, "missed_threshold"));

    for (r, champ) in cas {
        let refus = e.regler(r).await.expect_err("refusé");
        assert_eq!(
            (
                refus.code.as_str(),
                refus.code.status(),
                refus.field.as_deref()
            ),
            (
                "NEGOTIATION_IMPORT_CONFIG_INVALID",
                StatusCode::BAD_REQUEST,
                Some(champ)
            )
        );
    }
    assert!(
        e.en_file().await.is_empty(),
        "aucun refus n'a posé de lecture"
    );
    let lignes: i64 =
        sqlx::query_scalar("SELECT count(*) FROM negotiation.official_imports WHERE event_id = $1")
            .bind(e.edition)
            .fetch_one(e.bac.pool())
            .await
            .expect("compte");
    assert_eq!(lignes, 0);
}

#[tokio::test]
async fn rattacher_un_point_donne_sa_thematique_aux_sessions_sans_les_toucher() {
    let e = monter().await;
    e.regler(reglage(true)).await.expect("allumé");
    e.travailler().await;

    let points = admin_import::points(&e.bac.state, SLUG)
        .await
        .expect("points");
    assert!(!points.is_empty());
    assert!(points.iter().all(|p| p.theme.is_none()));
    let point = points
        .iter()
        .find(|p| p.session_count > 0)
        .expect("un point cité");
    assert_eq!(
        e.etat().await["agenda_items_without_theme"],
        points.len() as i64
    );

    let instantane = || async {
        sqlx::query_scalar::<_, Value>(
            "SELECT jsonb_agg(to_jsonb(m) ORDER BY m.id) FROM negotiation.meetings m",
        )
        .fetch_one(e.bac.pool())
        .await
        .expect("instantané")
    };
    let avant = instantane().await;

    let ctx = e.bac.ctx(e.ifdd);
    let rattache = admin_import::rattacher(&e.bac.state, &ctx, e.ifdd, point.id, Some("finance"))
        .await
        .expect("rattaché");
    assert_eq!(rattache.theme.as_deref(), Some("finance"));
    assert!(rattache.theme_set_at.is_some());
    assert_eq!(instantane().await, avant, "aucune session n'est réécrite");

    let servies = sessions::de_ledition(&e.bac.state, SLUG)
        .await
        .expect("sessions");
    let du_point: Vec<_> = servies
        .sessions
        .iter()
        .filter(|s| s.agenda_item.as_ref().is_some_and(|a| a.code == point.code))
        .collect();
    assert!(!du_point.is_empty());
    assert!(du_point
        .iter()
        .all(|s| s.theme.as_deref() == Some("finance")));

    let refus = admin_import::rattacher(&e.bac.state, &ctx, e.ifdd, point.id, Some("astrologie"))
        .await
        .expect_err("thématique inconnue");
    assert_eq!(refus.code.as_str(), "NEGOTIATION_THEME_UNKNOWN");
    let refus = admin_import::rattacher(&e.bac.state, &ctx, e.ifdd, Uuid::now_v7(), None)
        .await
        .expect_err("point inconnu");
    assert_eq!(
        (refus.code.as_str(), refus.code.status()),
        ("NEGOTIATION_AGENDA_ITEM_UNKNOWN", StatusCode::NOT_FOUND)
    );

    let detache = admin_import::rattacher(&e.bac.state, &ctx, e.ifdd, point.id, None)
        .await
        .expect("détaché");
    assert_eq!((detache.theme, detache.theme_set_at), (None, None));
}
