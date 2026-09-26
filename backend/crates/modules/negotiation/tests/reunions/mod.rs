//! Ce que partagent les tests des réunions de la Francophonie : une édition, des
//! personnes, des réunions saisies directement en base, et l'application montée
//! sur les seules routes du module.

#![allow(dead_code)]

use kernel::testing::TestDb;
use negotiation::state::NegotiationState;
use sqlx::PgPool;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

pub const ACTEUR: &str = "x-essai-acteur";
pub const LIEN: &str = "https://visio.example.org/salle-secrete";

#[allow(unused_macros)]
macro_rules! application {
    ($d:expr) => {
        actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new($d.base.db()))
                .app_data(actix_web::web::Data::new($d.etat.clone()))
                .wrap_fn(|req, srv| {
                    use actix_web::dev::Service as _;
                    use actix_web::HttpMessage as _;
                    let acteur = req
                        .headers()
                        .get($crate::reunions::ACTEUR)
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
                .configure(negotiation::routes),
        )
        .await
    };
}

/// Le back-office du module, seul, sur le même intergiciel d'acteur.
#[allow(unused_macros)]
macro_rules! administration {
    ($d:expr) => {
        actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new($d.base.db()))
                .app_data(actix_web::web::Data::new($d.etat.clone()))
                .wrap_fn(|req, srv| {
                    use actix_web::dev::Service as _;
                    use actix_web::HttpMessage as _;
                    let acteur = req
                        .headers()
                        .get($crate::reunions::ACTEUR)
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

/// Statut, corps lu en JSON, corps brut, en-têtes utiles.
macro_rules! frapper {
    ($app:expr, $requete:expr $(,)?) => {{
        let reponse = actix_web::test::call_service($app, ($requete).to_request()).await;
        let statut = reponse.status();
        let entete = |nom: actix_web::http::header::HeaderName| {
            reponse
                .headers()
                .get(nom)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let etag = entete(actix_web::http::header::ETAG);
        let cache = entete(actix_web::http::header::CACHE_CONTROL);
        let octets = actix_web::test::read_body(reponse).await;
        $crate::reunions::Reponse {
            statut,
            corps: serde_json::from_slice(&octets).unwrap_or(serde_json::Value::Null),
            brut: String::from_utf8_lossy(&octets).into_owned(),
            etag,
            cache,
        }
    }};
}

pub struct Reponse {
    pub statut: actix_web::http::StatusCode,
    pub corps: serde_json::Value,
    pub brut: String,
    pub etag: Option<String>,
    pub cache: Option<String>,
}

pub fn requete(
    methode: actix_web::http::Method,
    uri: &str,
    acteur: Option<Uuid>,
) -> actix_web::test::TestRequest {
    let r = actix_web::test::TestRequest::default()
        .method(methode)
        .uri(uri);
    match acteur {
        Some(a) => r.insert_header((ACTEUR, a.to_string())),
        None => r,
    }
}

pub fn inscrire(
    acteur: Option<Uuid>,
    reunion: Uuid,
    client_ref: Uuid,
) -> actix_web::test::TestRequest {
    requete(
        actix_web::http::Method::PUT,
        &format!("/negotiation/me/meeting-registrations/{reunion}"),
        acteur,
    )
    .set_json(serde_json::json!({ "client_ref": client_ref }))
}

pub fn desinscrire(acteur: Option<Uuid>, reunion: Uuid) -> actix_web::test::TestRequest {
    requete(
        actix_web::http::Method::DELETE,
        &format!("/negotiation/me/meeting-registrations/{reunion}"),
        acteur,
    )
}

pub fn mes_inscriptions(acteur: Option<Uuid>) -> actix_web::test::TestRequest {
    requete(
        actix_web::http::Method::GET,
        "/negotiation/me/meeting-registrations?edition=cop31",
        acteur,
    )
}

pub struct Decor {
    pub base: TestDb,
    pub etat: NegotiationState,
    pub edition: Uuid,
}

/// Une réunion saisie. Par défaut : une concertation publiée, en ligne, dans
/// une semaine, organisée par l'IFDD, sans capacité, liste d'attente permise.
pub struct Reunion {
    pub statut: &'static str,
    pub debut: OffsetDateTime,
    pub capacite: Option<i32>,
    pub attente: bool,
    pub inscription: bool,
    pub ouverture: Option<OffsetDateTime>,
    pub fermeture: Option<OffsetDateTime>,
    pub lien: Option<&'static str>,
    pub organisation: Option<Uuid>,
}

impl Default for Reunion {
    fn default() -> Self {
        Self {
            statut: "scheduled",
            debut: OffsetDateTime::now_utc() + Duration::days(7),
            capacite: None,
            attente: true,
            inscription: true,
            ouverture: None,
            fermeture: None,
            lien: Some(LIEN),
            organisation: None,
        }
    }
}

impl Decor {
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let edition = sqlx::query_scalar::<_, Uuid>(
            r#"INSERT INTO event.events
                   (edition_year, title, slug, description, participation_mode, timezone, starts_at, ends_at)
               VALUES (2026, '{"fr":"COP31"}'::jsonb, 'cop31', '{"fr":"Description."}'::jsonb,
                       'online', 'Asia/Istanbul', '2026-11-09T00:00:00Z', '2026-11-20T00:00:00Z')
               RETURNING id"#,
        )
        .fetch_one(base.pool())
        .await
        .expect("édition");
        let etat = NegotiationState::new(
            base.db(),
            Arc::new(kernel::testing::test_config(base.url())),
        );
        Self {
            base,
            etat,
            edition,
        }
    }

    pub fn pool(&self) -> &PgPool {
        self.base.pool()
    }

    /// `negotiator` porte `negotiation.space.access` ; sans rôle, aucun accès.
    pub async fn personne(&self, email: &str, negociatrice: bool) -> Uuid {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at)
             VALUES ($1::text::platform.email, 'Awa', 'Diallo', now()) RETURNING id",
        )
        .bind(email)
        .fetch_one(self.pool())
        .await
        .expect("personne");
        if negociatrice {
            sqlx::query(
                "INSERT INTO identity.role_assignments (person_id, role_code, scope_type)
                 VALUES ($1, 'negotiator', 'global')",
            )
            .bind(id)
            .execute(self.pool())
            .await
            .expect("rôle");
        }
        id
    }

    /// Un rôle sur une portée : `("admin", "global", None)`,
    /// `("admin", "event", Some(édition))`, `("space_lead", "negotiation_space", …)`.
    pub async fn role(&self, personne: Uuid, role: &str, portee: &str, cible: Option<Uuid>) {
        sqlx::query(
            "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
             VALUES ($1, $2, $3::text::identity.scope_type, $4)",
        )
        .bind(personne)
        .bind(role)
        .bind(portee)
        .bind(cible)
        .execute(self.pool())
        .await
        .expect("rôle");
    }

    pub async fn organisation(&self, nom: &str) -> Uuid {
        sqlx::query_scalar(
            "INSERT INTO org.organizations (legal_name, slug, organization_type_code, status)
             VALUES ($1, $2::text::platform.slug, 'ngo_association', 'active') RETURNING id",
        )
        .bind(nom)
        .bind(format!("org-{}", Uuid::now_v7().simple()))
        .fetch_one(self.pool())
        .await
        .expect("organisation")
    }

    pub async fn reunion(&self, r: Reunion) -> Uuid {
        sqlx::query_scalar(
            r#"INSERT INTO negotiation.meetings
                   (space_id, kind, slug, title, start_at, end_at, timezone, format, external_url,
                    event_id, organizer_org_id, is_ifdd_organized, capacity,
                    registration_opens_at, registration_closes_at, status, cancellation_reason,
                    francophone_type_term_id, is_open_access, requires_registration, waitlist_enabled)
               VALUES ((SELECT id FROM negotiation.spaces WHERE slug::text = 'climat'),
                       'francophone_consultation', $1::text::platform.slug,
                       '{"fr":"Concertation des négociateurs","en":"Negotiators consultation"}'::jsonb,
                       $2, $2 + interval '2 hours', 'Asia/Istanbul', 'online',
                       $3::text::platform.url, $4, $5, $5 IS NULL, $6, $7, $8,
                       $9::text::negotiation.meeting_status,
                       CASE WHEN $9 = 'cancelled' THEN 'Reportée à la prochaine session.' END,
                       (SELECT id FROM reference.taxonomy_terms
                         WHERE taxonomy_code = 'francophone_meeting_type'
                           AND code = 'negotiators_consultation'),
                       true, $10, $11)
               RETURNING id"#,
        )
        .bind(format!("r-{}", Uuid::now_v7().simple()))
        .bind(r.debut)
        .bind(r.lien)
        .bind(self.edition)
        .bind(r.organisation)
        .bind(r.capacite)
        .bind(r.ouverture)
        .bind(r.fermeture)
        .bind(r.statut)
        .bind(r.inscription)
        .bind(r.attente)
        .fetch_one(self.pool())
        .await
        .expect("réunion")
    }

    /// `(statut, position)` de la ligne, ou rien.
    pub async fn ligne(&self, reunion: Uuid, personne: Uuid) -> Option<(String, Option<i32>)> {
        sqlx::query_as(
            "SELECT status::text, waitlist_position FROM negotiation.meeting_registrations
              WHERE meeting_id = $1 AND person_id = $2",
        )
        .bind(reunion)
        .bind(personne)
        .fetch_optional(self.pool())
        .await
        .expect("ligne")
    }

    pub async fn lignes(&self, reunion: Uuid) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM negotiation.meeting_registrations WHERE meeting_id = $1",
        )
        .bind(reunion)
        .fetch_one(self.pool())
        .await
        .expect("compte")
    }

    pub async fn inscrites(&self, reunion: Uuid) -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM negotiation.meeting_registrations
              WHERE meeting_id = $1 AND status = 'registered'",
        )
        .bind(reunion)
        .fetch_one(self.pool())
        .await
        .expect("compte")
    }

    /// La réunion a commencé il y a une heure.
    pub async fn commencer(&self, reunion: Uuid) {
        sqlx::query(
            "UPDATE negotiation.meetings
                SET start_at = now() - interval '1 hour', end_at = now() + interval '1 hour'
              WHERE id = $1",
        )
        .bind(reunion)
        .execute(self.pool())
        .await
        .expect("début");
    }
}
