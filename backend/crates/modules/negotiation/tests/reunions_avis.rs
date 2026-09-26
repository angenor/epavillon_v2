//! **Prévenir** (FR-020, US2 sc. 4, US3 sc. 2) — l'annulation et le changement
//! d'heure ou de lieu d'une réunion publiée préviennent inscrites et liste
//! d'attente ; une place obtenue prévient la promue ; l'accord éteint ne coupe
//! que le courriel ; un brouillon ne prévient personne.

#[macro_use]
mod reunions;

use actix_web::http::{Method, StatusCode};
use async_trait::async_trait;
use kernel::jobs::{ClaimedJob, JobHandler};
use kernel::mail::{MailError, Mailer, OutgoingMail};
use negotiation::jobs::change_email::SessionChangeEmail;
use negotiation::jobs::promotion_email::MeetingPromotionEmail;
use reunions::{desinscrire, inscrire, requete, Decor, LIEN};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{Duration, OffsetDateTime, UtcOffset};
use uuid::Uuid;

const CHANGEE: &str = "negotiation.francophone_meeting.changed";
const PROMUE: &str = "negotiation.meeting_registration.promoted";
const COURRIEL: &str = "negotiation.session_change_email";
const COURRIEL_PLACE: &str = "negotiation.meeting_promotion_email";

#[derive(Default)]
struct Boite(Mutex<Vec<OutgoingMail>>);

#[async_trait]
impl Mailer for Boite {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError> {
        self.0.lock().expect("boîte").push(mail.clone());
        Ok(())
    }
}

impl Boite {
    fn messages(&self) -> Vec<OutgoingMail> {
        self.0.lock().expect("boîte").clone()
    }
}

fn instant(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).expect("date")
}

/// Une concertation en ligne dans une semaine, une place.
fn corps(debut: OffsetDateTime) -> Value {
    json!({
        "edition": "cop31",
        "type": "negotiators_consultation",
        "title": { "fr": "Concertation des négociateurs", "en": "Negotiators consultation" },
        "description": null,
        "start_at": instant(debut),
        "end_at": instant(debut + Duration::hours(2)),
        "format": "online",
        "venue": null,
        "external_url": LIEN,
        "capacity": 1,
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

fn debut() -> OffsetDateTime {
    (OffsetDateTime::now_utc() + Duration::days(7))
        .replace_second(0)
        .and_then(|t| t.replace_nanosecond(0))
        .expect("instant")
}

struct Scene {
    admin: Uuid,
    reunion: Uuid,
    corps: Value,
    awa: Uuid,
    bea: Uuid,
}

/// Réunion publiée à Antalya : Awa inscrite, Béa en liste d'attente.
macro_rules! scene {
    ($d:expr, $app:expr) => {{
        let d: &Decor = $d;
        sqlx::query("UPDATE event.events SET city = 'Antalya' WHERE id = $1")
            .bind(d.edition)
            .execute(d.pool())
            .await
            .expect("ville");
        let admin = d.personne("admin@example.org", false).await;
        d.role(admin, "admin", "global", None).await;
        let c = corps(debut());
        let r = frapper!(
            $app,
            requete(Method::POST, "/admin/negotiation/meetings", Some(admin)).set_json(c.clone())
        );
        assert_eq!(r.statut, StatusCode::CREATED, "{}", r.brut);
        let reunion = Uuid::parse_str(r.corps["id"].as_str().expect("id")).expect("uuid");
        let r = frapper!(
            $app,
            requete(
                Method::POST,
                &format!("/admin/negotiation/meetings/{reunion}/publish"),
                Some(admin)
            )
        );
        assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
        let awa = d.personne("awa@example.org", true).await;
        let bea = d.personne("bea@example.org", true).await;
        frapper!($app, inscrire(Some(awa), reunion, Uuid::now_v7()));
        frapper!($app, inscrire(Some(bea), reunion, Uuid::now_v7()));
        assert_eq!(
            d.ligne(reunion, bea).await.map(|l| l.0),
            Some("waitlisted".into())
        );
        Scene {
            admin,
            reunion,
            corps: c,
            awa,
            bea,
        }
    }};
}

/// Les charges `notification` émises d'un type.
async fn avis(d: &Decor, type_: &str) -> Vec<Value> {
    sqlx::query_scalar::<_, Value>(
        "SELECT payload FROM platform.outbox_events WHERE event_type = $1 ORDER BY occurred_at, id",
    )
    .bind(type_)
    .fetch_all(d.pool())
    .await
    .expect("événements")
    .into_iter()
    .map(|p| p["notification"].clone())
    .collect()
}

fn destinataires(n: &Value) -> Vec<Uuid> {
    let mut v: Vec<Uuid> = serde_json::from_value(n["recipients"].clone()).expect("destinataires");
    v.sort();
    v
}

async fn travaux(d: &Decor, tache: &str) -> Vec<(Uuid, Value, String)> {
    sqlx::query_as(
        "SELECT id, payload, idempotency_key FROM platform.jobs
          WHERE task = $1 AND status = 'queued' ORDER BY id",
    )
    .bind(tache)
    .fetch_all(d.pool())
    .await
    .expect("travaux")
}

async fn envoyer(d: &Decor, boite: &Arc<Boite>) {
    let changement = SessionChangeEmail::new(d.base.db(), boite.clone(), "https://app.test".into());
    let place = MeetingPromotionEmail::new(d.base.db(), boite.clone(), "https://app.test".into());
    for (tache, travail) in [
        (COURRIEL, &changement as &dyn JobHandler),
        (COURRIEL_PLACE, &place as &dyn JobHandler),
    ] {
        for (id, payload, _) in travaux(d, tache).await {
            travail
                .run(&ClaimedJob {
                    id,
                    queue: "default".into(),
                    task: tache.into(),
                    payload,
                    attempts: 1,
                    max_attempts: 5,
                })
                .await
                .expect("le courriel part, ou se tait");
        }
    }
}

async fn refuser_les_courriels(d: &Decor, qui: Uuid) {
    sqlx::query(
        "INSERT INTO identity.consents (person_id, purpose, is_granted, policy_version)
         VALUES ($1, 'guide_nego_notifications', false, 'test')",
    )
    .bind(qui)
    .execute(d.pool())
    .await
    .expect("accord éteint");
}

async fn changements_importes(d: &Decor, reunion: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM negotiation.meeting_changes WHERE meeting_id = $1")
        .bind(reunion)
        .fetch_one(d.pool())
        .await
        .expect("compte")
}

fn modifier(s: &Scene, c: Value) -> actix_web::test::TestRequest {
    requete(
        Method::PUT,
        &format!("/admin/negotiation/meetings/{}", s.reunion),
        Some(s.admin),
    )
    .set_json(c)
}

fn annuler(admin: Uuid, reunion: Uuid) -> actix_web::test::TestRequest {
    requete(
        Method::POST,
        &format!("/admin/negotiation/meetings/{reunion}/cancel"),
        Some(admin),
    )
    .set_json(json!({ "reason": "Reportée à la prochaine session." }))
}

fn sans_nom(n: &Value) {
    let texte = n.to_string();
    assert!(
        !texte.contains("Diallo") && !texte.contains("Awa"),
        "{texte}"
    );
}

#[tokio::test]
async fn les_deux_types_sont_au_catalogue_et_actifs() {
    let d = Decor::monter().await;
    let actifs: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM engagement.notification_types
          WHERE code = ANY($1) AND is_active AND module_code = 'negotiation'",
    )
    .bind(vec![CHANGEE, PROMUE])
    .fetch_one(d.pool())
    .await
    .expect("catalogue");
    assert_eq!(actifs, 2, "la branche générique d'engagement les écrit");
}

#[tokio::test]
async fn lannulation_previent_inscrites_et_attente() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let s = scene!(&d, &app);

    let r = frapper!(&app, annuler(s.admin, s.reunion));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);

    let emis = avis(&d, CHANGEE).await;
    assert_eq!(emis.len(), 1);
    let n = &emis[0];
    let mut attendus = vec![s.awa, s.bea];
    attendus.sort();
    assert_eq!(destinataires(n), attendus);
    let titre = n["title"]["fr"].as_str().expect("titre");
    assert!(
        titre.starts_with("Annulée — Concertation des négociateurs"),
        "{titre}"
    );
    assert!(
        titre.contains("Reportée à la prochaine session."),
        "{titre}"
    );
    assert!(titre.contains("heure d'Antalya"), "{titre}");
    assert!(n["title"]["en"]
        .as_str()
        .expect("en")
        .starts_with("Cancelled — Negotiators consultation"));
    assert_eq!(n["body"]["fr"], "Réunions de la Francophonie");
    assert_eq!(
        n["link_path"],
        format!("/guide-nego/francophonie/reunions/{}", s.reunion)
    );
    assert!(n["group_key"]
        .as_str()
        .expect("clé")
        .starts_with(&format!("{CHANGEE}:{}:", s.reunion)));
    assert_eq!(n["replace"], true);
    sans_nom(n);

    let poses = travaux(&d, COURRIEL).await;
    assert_eq!(poses.len(), 2);
    assert!(poses
        .iter()
        .all(|(_, _, cle)| cle.starts_with(&format!("email:{}:", s.reunion))));
    let boite = Arc::new(Boite::default());
    envoyer(&d, &boite).await;
    let messages = boite.messages();
    assert_eq!(messages.len(), 2);
    for m in &messages {
        assert!(
            m.subject.starts_with("Annulée — Concertation"),
            "{}",
            m.subject
        );
        assert!(m.text.contains("Motif : Reportée"), "{}", m.text);
        assert!(
            m.text.contains("/guide-nego/francophonie/reunions/"),
            "{}",
            m.text
        );
        assert!(m.text.contains("Réunions de la Francophonie"), "{}", m.text);
    }
    assert_eq!(changements_importes(&d, s.reunion).await, 0);
}

#[tokio::test]
async fn un_changement_dheure_dit_la_nouvelle_heure() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let s = scene!(&d, &app);

    let nouveau = debut() + Duration::hours(1);
    let mut c = s.corps.clone();
    c["start_at"] = json!(instant(nouveau));
    c["end_at"] = json!(instant(nouveau + Duration::hours(2)));
    let r = frapper!(&app, modifier(&s, c));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);

    let heure = nouveau
        .to_offset(UtcOffset::from_hms(3, 0, 0).expect("décalage"))
        .format(format_description!("[hour]:[minute]"))
        .expect("heure");
    let emis = avis(&d, CHANGEE).await;
    assert_eq!(emis.len(), 1);
    let titre = emis[0]["title"]["fr"].as_str().expect("titre");
    assert!(titre.starts_with("Déplacée — "), "{titre}");
    assert!(
        titre.contains(&format!("{heure} (heure d'Antalya)")),
        "{titre}"
    );
    assert_eq!(destinataires(&emis[0]).len(), 2);
    sans_nom(&emis[0]);

    let boite = Arc::new(Boite::default());
    envoyer(&d, &boite).await;
    let messages = boite.messages();
    assert_eq!(messages.len(), 2);
    assert!(
        messages[0].subject.starts_with("Déplacée — "),
        "{}",
        messages[0].subject
    );
    assert!(
        messages[0].text.contains("heure d'Antalya"),
        "{}",
        messages[0].text
    );
    assert!(messages[0].text.contains(&heure), "{}", messages[0].text);
    assert_eq!(changements_importes(&d, s.reunion).await, 0);
}

#[tokio::test]
async fn un_changement_de_lieu_se_dit_lieu_change() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let s = scene!(&d, &app);

    let mut c = s.corps.clone();
    c["format"] = json!("hybrid");
    c["venue"] = json!("Salle Bosphore");
    let r = frapper!(&app, modifier(&s, c));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);

    let emis = avis(&d, CHANGEE).await;
    assert_eq!(emis.len(), 1);
    let titre = emis[0]["title"]["fr"].as_str().expect("titre");
    assert!(titre.starts_with("Lieu changé — "), "{titre}");
    assert!(titre.contains("Salle Bosphore et en ligne"), "{titre}");
}

#[tokio::test]
async fn la_seule_description_ne_previent_personne() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let s = scene!(&d, &app);

    let mut c = s.corps.clone();
    c["description"] = json!({ "fr": "Ordre du jour précisé." });
    c["title"] = json!({ "fr": "Concertation des négociateurs", "en": "Negotiators consultation" });
    let r = frapper!(&app, modifier(&s, c));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);

    assert!(avis(&d, CHANGEE).await.is_empty());
    assert!(travaux(&d, COURRIEL).await.is_empty());
}

#[tokio::test]
async fn se_desinscrire_previent_la_promue() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let s = scene!(&d, &app);

    let r = frapper!(&app, desinscrire(Some(s.awa), s.reunion));
    assert!(r.statut.is_success(), "{}", r.brut);

    let emis = avis(&d, PROMUE).await;
    assert_eq!(emis.len(), 1);
    let n = &emis[0];
    assert_eq!(destinataires(n), vec![s.bea]);
    let titre = n["title"]["fr"].as_str().expect("titre");
    assert!(
        titre.starts_with("Inscrite — Concertation des négociateurs"),
        "{titre}"
    );
    assert!(titre.contains("Une place s'est libérée"), "{titre}");
    assert!(titre.contains("heure d'Antalya"), "{titre}");
    assert_eq!(n["body"]["fr"], "Réunions de la Francophonie");
    assert_eq!(n["group_key"], Value::Null);
    assert_eq!(n["replace"], false);
    sans_nom(n);

    let poses = travaux(&d, COURRIEL_PLACE).await;
    assert_eq!(poses.len(), 1);
    assert_eq!(poses[0].2, format!("promotion:{}:{}", s.reunion, s.bea));
    assert!(
        avis(&d, CHANGEE).await.is_empty(),
        "une place n'est pas un changement"
    );

    let boite = Arc::new(Boite::default());
    envoyer(&d, &boite).await;
    let messages = boite.messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].to, "bea@example.org");
    assert!(
        messages[0].subject.starts_with("Inscrite — "),
        "{}",
        messages[0].subject
    );
    assert!(
        messages[0].text.contains("Une place s'est libérée"),
        "{}",
        messages[0].text
    );
}

#[tokio::test]
async fn relever_la_capacite_previent_chaque_promue() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let s = scene!(&d, &app);
    let cheikh = d.personne("cheikh@example.org", true).await;
    frapper!(&app, inscrire(Some(cheikh), s.reunion, Uuid::now_v7()));

    let mut c = s.corps.clone();
    c["capacity"] = json!(3);
    let r = frapper!(&app, modifier(&s, c));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);

    let emis = avis(&d, PROMUE).await;
    assert_eq!(emis.len(), 2);
    let mut promues: Vec<Uuid> = emis.iter().flat_map(destinataires).collect();
    promues.sort();
    let mut attendues = vec![s.bea, cheikh];
    attendues.sort();
    assert_eq!(promues, attendues);
    assert!(emis.iter().all(|n| destinataires(n).len() == 1));
    assert_eq!(travaux(&d, COURRIEL_PLACE).await.len(), 2);
    assert!(avis(&d, CHANGEE).await.is_empty());
}

#[tokio::test]
async fn laccord_eteint_garde_lavis_et_coupe_le_courriel() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let s = scene!(&d, &app);
    refuser_les_courriels(&d, s.bea).await;

    frapper!(&app, desinscrire(Some(s.awa), s.reunion));
    assert_eq!(destinataires(&avis(&d, PROMUE).await[0]), vec![s.bea]);
    let boite = Arc::new(Boite::default());
    envoyer(&d, &boite).await;
    assert!(
        boite.messages().is_empty(),
        "place obtenue : aucun courriel"
    );

    let r = frapper!(&app, annuler(s.admin, s.reunion));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
    assert_eq!(destinataires(&avis(&d, CHANGEE).await[0]), vec![s.bea]);
    let boite = Arc::new(Boite::default());
    envoyer(&d, &boite).await;
    assert!(boite.messages().is_empty(), "annulation : aucun courriel");
}

/// Un brouillon n'a pas d'inscrites : la scène en pose avant de repasser la
/// réunion en brouillon, pour prouver que le service ne regarde pas qu'elles.
#[tokio::test]
async fn un_brouillon_ne_previent_personne() {
    let d = Decor::monter().await;
    let app = administration!(d);
    let s = scene!(&d, &app);
    sqlx::query("UPDATE negotiation.meetings SET status = 'draft' WHERE id = $1")
        .bind(s.reunion)
        .execute(d.pool())
        .await
        .expect("brouillon");

    let mut c = s.corps.clone();
    c["start_at"] = json!(instant(debut() + Duration::hours(3)));
    c["end_at"] = json!(instant(debut() + Duration::hours(5)));
    c["capacity"] = json!(2);
    let r = frapper!(&app, modifier(&s, c));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
    let r = frapper!(&app, annuler(s.admin, s.reunion));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);

    assert!(avis(&d, CHANGEE).await.is_empty());
    assert!(avis(&d, PROMUE).await.is_empty());
    assert!(travaux(&d, COURRIEL).await.is_empty());
    assert!(travaux(&d, COURRIEL_PLACE).await.is_empty());
}
