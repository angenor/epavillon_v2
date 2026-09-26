//! Ce que partagent les tests de la validation et de l'affichage par-dessus :
//! une édition importée, une autrice, une administratrice globale, les routes
//! publiques et d'administration, et le travail de publication rejoué à la main.

#![allow(dead_code, unused_macros)]

use crate::importation::Bac;
use kernel::jobs::{ClaimedJob, JobHandler};
use negotiation::jobs::publish::PublishReport;
use negotiation::state::NegotiationState;
use serde_json::Value;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

pub const ACTEUR: &str = "x-essai-acteur";

macro_rules! application {
    ($d:expr) => {
        actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new($d.bac.base.db()))
                .app_data(actix_web::web::Data::new($d.etat.clone()))
                .wrap_fn(|req, srv| {
                    use actix_web::dev::Service as _;
                    use actix_web::HttpMessage as _;
                    let acteur = req
                        .headers()
                        .get(decision::ACTEUR)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| uuid::Uuid::parse_str(v).ok());
                    let ctx = kernel::context::RequestContext::new(
                        kernel::context::RequestContext::generated_request_id(),
                        "fr",
                    );
                    req.extensions_mut().insert(match acteur {
                        Some(a) => ctx.with_actor(a),
                        None => ctx,
                    });
                    srv.call(req)
                })
                .configure(negotiation::routes)
                .configure(negotiation::admin_routes),
        )
        .await
    };
}

macro_rules! frapper {
    ($app:expr, $requete:expr $(,)?) => {{
        let reponse = actix_web::test::call_service($app, ($requete).to_request()).await;
        let statut = reponse.status();
        let etag = reponse
            .headers()
            .get(actix_web::http::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let octets = actix_web::test::read_body(reponse).await;
        (
            statut,
            serde_json::from_slice::<serde_json::Value>(&octets).unwrap_or(serde_json::Value::Null),
            etag,
        )
    }};
}

pub fn poster(acteur: Uuid, uri: &str) -> actix_web::test::TestRequest {
    actix_web::test::TestRequest::post()
        .uri(uri)
        .insert_header((ACTEUR, acteur.to_string()))
}

pub fn lire(acteur: Uuid, uri: &str) -> actix_web::test::TestRequest {
    actix_web::test::TestRequest::get()
        .uri(uri)
        .insert_header((ACTEUR, acteur.to_string()))
}

pub fn public(uri: &str) -> actix_web::test::TestRequest {
    actix_web::test::TestRequest::get().uri(uri)
}

pub struct Decor {
    pub bac: Bac,
    pub etat: NegotiationState,
    pub awa: Uuid,
    pub ifdd: Uuid,
    pub sessions: Vec<Uuid>,
}

pub async fn personne(bac: &Bac, email: &str, nom: &str, role: Option<&str>) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
         VALUES ($1::text::platform.email, $2, 'Diallo', now()) RETURNING id",
    )
    .bind(email)
    .bind(nom)
    .fetch_one(bac.pool())
    .await
    .expect("personne");
    if let Some(role) = role {
        sqlx::query(
            "INSERT INTO identity.role_assignments (person_id, role_code, scope_type)
             VALUES ($1, $2, 'global')",
        )
        .bind(id)
        .bind(role)
        .execute(bac.pool())
        .await
        .expect("rôle");
    }
    id
}

pub async fn monter() -> Decor {
    let bac = Bac::monter().await;
    bac.lire().await;
    let etat = NegotiationState::new(
        bac.base.db(),
        Arc::new(kernel::testing::test_config(bac.base.url())),
    );
    let awa = personne(&bac, "awa@example.org", "Awa", Some("negotiator")).await;
    let ifdd = personne(&bac, "ifdd@example.org", "Ines", Some("admin")).await;
    let sessions = sqlx::query_scalar(
        "SELECT id FROM negotiation.meetings WHERE event_id = $1 ORDER BY start_at, id LIMIT 4",
    )
    .bind(bac.edition)
    .fetch_all(bac.pool())
    .await
    .expect("sessions");
    Decor {
        bac,
        etat,
        awa,
        ifdd,
        sessions,
    }
}

impl Decor {
    pub async fn session(&self, cle: &str) -> Uuid {
        self.bac.session(cle).await.expect("session").id
    }

    /// Un signalement en attente, écrit comme la route l'écrirait.
    pub async fn signaler(
        &self,
        reason: &str,
        session: Uuid,
        heure: Option<&str>,
        salle: Option<&str>,
    ) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO negotiation.session_reports
                 (author_id, client_ref, event_id, meeting_id, reason, proposed_start, proposed_venue, detail)
             VALUES ($1, $2, $3, $4, $5, $6::text::timestamptz, $7, 'Affiché à l''entrée.')
             RETURNING id",
        )
        .bind(self.awa)
        .bind(Uuid::now_v7())
        .bind(self.bac.edition)
        .bind(session)
        .bind(reason)
        .bind(heure)
        .bind(salle)
        .fetch_one(self.bac.pool())
        .await
        .expect("signalement")
    }

    /// Une réunion non annoncée en attente, le jour donné.
    pub async fn non_annoncee(&self, jour: &str) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO negotiation.session_reports
                 (author_id, client_ref, event_id, reason, what, proposed_day, proposed_venue,
                  proposed_start, theme_term_id)
             SELECT $1, $2, $3, 'unannounced', 'Groupe Afrique', $4::text::date, 'Couloir B',
                    ($4 || 'T11:00:00Z')::timestamptz, t.id
               FROM reference.taxonomy_terms t
              WHERE t.taxonomy_code = 'negotiation_theme' AND t.code = 'finance'
             RETURNING id",
        )
        .bind(self.awa)
        .bind(Uuid::now_v7())
        .bind(self.bac.edition)
        .bind(jour)
        .fetch_one(self.bac.pool())
        .await
        .expect("réunion non annoncée")
    }

    /// Les publications en file, telles que le worker les prendrait.
    pub async fn travaux(&self) -> Vec<ClaimedJob> {
        let lignes: Vec<(Uuid, Value)> = sqlx::query_as(
            "SELECT id, payload FROM platform.jobs
              WHERE task = 'negotiation.report.publish' AND status = 'queued'
              ORDER BY run_at, id",
        )
        .fetch_all(self.bac.pool())
        .await
        .expect("travaux");
        lignes
            .into_iter()
            .map(|(id, payload)| ClaimedJob {
                id,
                queue: "default".into(),
                task: "negotiation.report.publish".into(),
                payload,
                attempts: 1,
                max_attempts: 5,
            })
            .collect()
    }

    pub async fn executer(&self, travail: &ClaimedJob) {
        PublishReport::new(self.bac.base.db())
            .run(travail)
            .await
            .expect("le travail de publication réussit");
    }

    /// Exécute les publications en file, puis les marque réussies.
    pub async fn publier_tout(&self) -> Vec<ClaimedJob> {
        let travaux = self.travaux().await;
        for t in &travaux {
            self.executer(t).await;
            let mut conn = self.bac.pool().acquire().await.expect("connexion");
            kernel::jobs::succeed(&mut conn, t.id)
                .await
                .expect("réussite");
        }
        travaux
    }

    /// Les événements de signalement, dans l'ordre d'émission.
    pub async fn evenements(&self) -> Vec<(String, Value)> {
        sqlx::query_as(
            "SELECT event_type, payload FROM platform.outbox_events
              WHERE event_type LIKE 'negotiation.%'
              ORDER BY occurred_at, id",
        )
        .fetch_all(self.bac.pool())
        .await
        .expect("outbox")
    }

    pub async fn publie_a(&self, id: Uuid) -> Option<OffsetDateTime> {
        sqlx::query_scalar("SELECT published_at FROM negotiation.session_reports WHERE id = $1")
            .bind(id)
            .fetch_one(self.bac.pool())
            .await
            .expect("signalement")
    }

    pub async fn garder(&self, personne: Uuid, session: Uuid) {
        sqlx::query(
            "INSERT INTO negotiation.agenda_entries (person_id, meeting_id) VALUES ($1, $2)",
        )
        .bind(personne)
        .bind(session)
        .execute(self.bac.pool())
        .await
        .expect("agenda");
    }
}

/// La session de ce nom dans la réponse publique.
pub fn dans(sessions: &Value, id: Uuid) -> &Value {
    sessions["sessions"]
        .as_array()
        .expect("sessions")
        .iter()
        .find(|s| s["id"] == serde_json::json!(id))
        .expect("session servie")
}
