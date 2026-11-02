//! Intergiciel de contexte de requête.
//!
//! Lit `X-Request-Id` s'il est présent, l'engendre sinon, négocie la locale,
//! pose le contexte dans la requête et dans la portée de tâche — c'est ce qui
//! permet à une réponse d'erreur de porter le même identifiant —, et le renvoie
//! dans l'en-tête de réponse.
//!
//! En fin de requête, il pose dans `platform.ops_log` ce qui mérite d'être relu
//! (`classer`), par le canal de [`OpsLog`] : jamais d'attente, jamais d'échec.

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{web, Error, HttpMessage};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use kernel::context::{RequestContext, REQUEST_ID_HEADER};
use kernel::error::ApiError;
use kernel::i18n::Locales;
use kernel::ops_log::{HttpOutcome, OpsEntry, OpsKind, OpsLevel, OpsLog};
use std::rc::Rc;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use tracing::field::Empty;
use tracing::{Instrument, Level, Span};

/// Borne large : un identifiant venu du client ne doit ni gonfler les journaux
/// ni transporter autre chose que des caractères d'en-tête.
const LONGUEUR_MAX: usize = 128;

const REQUETE_LENTE: Duration = Duration::from_secs(1);

/// Une ligne par requête noierait les journaux en pleine COP : seules les
/// pannes, les lenteurs et les refus d'accès ou de débit méritent d'être lus.
fn niveau_de_fin(statut: u16, duree: Duration) -> Option<Level> {
    if statut >= 500 || duree > REQUETE_LENTE {
        Some(Level::WARN)
    } else if matches!(statut, 401 | 403 | 429) {
        Some(Level::INFO)
    } else {
        None
    }
}

/// Ce qui entre dans `platform.ops_log`. Un refus ordinaire (400, 404 d'un
/// visiteur, 409 ou 422 hors back-office) est le fonctionnement normal du site ;
/// le retenir noierait les vraies pannes.
fn classer(
    statut: u16,
    duree: Duration,
    route: &str,
    connecte: bool,
) -> Option<(OpsKind, OpsLevel)> {
    let back_office = route
        .strip_prefix(crate::PREFIXE)
        .unwrap_or(route)
        .starts_with("/admin");
    if statut >= 500 {
        Some((OpsKind::HttpError, OpsLevel::Error))
    } else if matches!(statut, 401 | 403 | 429)
        || (back_office && matches!(statut, 409 | 422))
        || (connecte && statut == 404)
    {
        Some((OpsKind::HttpRefused, OpsLevel::Info))
    } else if statut < 400 && duree > REQUETE_LENTE {
        Some((OpsKind::HttpSlow, OpsLevel::Warn))
    } else {
        None
    }
}

fn journaliser_fin(statut: u16, duree: Duration) {
    let duree_ms = u64::try_from(duree.as_millis()).unwrap_or(u64::MAX);
    match niveau_de_fin(statut, duree) {
        Some(Level::WARN) if statut >= 500 => {
            tracing::warn!(statut, duree_ms, "requête en échec")
        }
        Some(Level::WARN) => tracing::warn!(statut, duree_ms, "requête lente"),
        Some(_) => tracing::info!(statut, duree_ms, "requête refusée"),
        None => {}
    }
    Span::current()
        .record("statut", statut)
        .record("duree_ms", duree_ms);
}

pub struct RequestContextMiddleware {
    locales: Locales,
}

impl RequestContextMiddleware {
    pub fn new(locales: Locales) -> Self {
        Self { locales }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestContextMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestContextService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestContextService {
            service: Rc::new(service),
            locales: self.locales.clone(),
        }))
    }
}

pub struct RequestContextService<S> {
    service: Rc<S>,
    locales: Locales,
}

impl<S, B> Service<ServiceRequest> for RequestContextService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request_id = req
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.trim())
            .filter(|v| {
                !v.is_empty()
                    && v.len() <= LONGUEUR_MAX
                    && v.chars().all(|c| c.is_ascii_graphic() || c == ' ')
            })
            .map(str::to_owned)
            .unwrap_or_else(RequestContext::generated_request_id);

        let locale = self.locales.negotiate(
            req.headers()
                .get(actix_web::http::header::ACCEPT_LANGUAGE)
                .and_then(|v| v.to_str().ok()),
        );

        req.extensions_mut()
            .insert(RequestContext::new(request_id.clone(), locale));

        let service = self.service.clone();
        let entete = request_id.clone();

        // Le modèle de route, jamais le chemin reçu : des jetons passent dans
        // certains chemins. La table de routage répond avant même le routage.
        let route = req
            .match_pattern()
            .unwrap_or_else(|| "(aucune route)".to_owned());
        // Sans span, l'exportateur OTLP n'a rien à exporter : la couche
        // OpenTelemetry ignore tout événement hors span, et Jaeger reste vide.
        let span = tracing::info_span!(
            "http",
            methode = %req.method(),
            route = %route,
            request_id = %request_id,
            statut = Empty,
            duree_ms = Empty,
        );
        let debut = Instant::now();
        let recu_le = OffsetDateTime::now_utc();
        let journal = req.app_data::<web::Data<OpsLog>>().cloned();
        let methode = req.method().as_str().to_owned();
        let identifiant = request_id.clone();

        Box::pin(kernel::context::scope(
            request_id,
            async move {
                let resultat = service.call(req).await;
                let statut = match &resultat {
                    Ok(res) => res.status(),
                    Err(e) => e.as_response_error().status_code(),
                };
                let duree = debut.elapsed();
                journaliser_fin(statut.as_u16(), duree);
                if let Some(journal) = journal {
                    let acteur = resultat.as_ref().ok().and_then(|res| {
                        res.request()
                            .extensions()
                            .get::<RequestContext>()
                            .and_then(|ctx| ctx.actor_id)
                    });
                    if let Some((kind, level)) =
                        classer(statut.as_u16(), duree, &route, acteur.is_some())
                    {
                        let erreur = match &resultat {
                            Ok(res) => res
                                .response()
                                .error()
                                .and_then(|e| e.as_error::<ApiError>()),
                            Err(e) => e.as_error::<ApiError>(),
                        };
                        journal.record(OpsEntry::http(
                            kind,
                            level,
                            HttpOutcome {
                                occurred_at: recu_le,
                                request_id: &identifiant,
                                actor_id: acteur,
                                method: &methode,
                                route: &route,
                                status: statut.as_u16(),
                                duration: duree,
                                error_code: erreur.map(|e| e.code.as_str()),
                                detail: erreur.and_then(|e| e.detail.as_deref()),
                            },
                        ));
                    }
                }
                let mut res = resultat?;
                if let Ok(valeur) = HeaderValue::from_str(&entete) {
                    res.headers_mut()
                        .insert(HeaderName::from_static("x-request-id"), valeur);
                }
                Ok(res)
            }
            .instrument(span),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAPIDE: Duration = Duration::from_millis(40);
    const LENT: Duration = Duration::from_millis(1500);

    #[test]
    fn succes_rapide_ne_dit_rien() {
        for statut in [200, 201, 204, 304, 400, 404, 409, 422] {
            assert_eq!(niveau_de_fin(statut, RAPIDE), None, "{statut}");
        }
    }

    #[test]
    fn panne_ou_lenteur_avertit() {
        assert_eq!(niveau_de_fin(500, RAPIDE), Some(Level::WARN));
        assert_eq!(niveau_de_fin(503, RAPIDE), Some(Level::WARN));
        assert_eq!(niveau_de_fin(200, LENT), Some(Level::WARN));
        assert_eq!(niveau_de_fin(403, LENT), Some(Level::WARN));
        assert_eq!(niveau_de_fin(200, REQUETE_LENTE), None);
    }

    #[test]
    fn classement_du_journal_d_exploitation() {
        let c = |statut, duree, route, connecte| {
            classer(statut, duree, route, connecte).map(|(k, _)| k)
        };
        assert_eq!(c(500, RAPIDE, "/api/x", false), Some(OpsKind::HttpError));
        assert_eq!(c(503, LENT, "/api/x", false), Some(OpsKind::HttpError));
        for statut in [401, 403, 429] {
            assert_eq!(
                c(statut, RAPIDE, "/api/x", false),
                Some(OpsKind::HttpRefused)
            );
        }
        assert_eq!(
            c(409, RAPIDE, "/api/admin/proposals/{id}", true),
            Some(OpsKind::HttpRefused)
        );
        assert_eq!(
            c(422, RAPIDE, "/api/admin/x", true),
            Some(OpsKind::HttpRefused)
        );
        assert_eq!(c(409, RAPIDE, "/api/proposals/{id}", true), None);
        assert_eq!(c(404, RAPIDE, "/api/x", false), None);
        assert_eq!(c(404, RAPIDE, "/api/x", true), Some(OpsKind::HttpRefused));
        assert_eq!(c(400, LENT, "/api/x", true), None);
        assert_eq!(c(200, LENT, "/api/x", false), Some(OpsKind::HttpSlow));
        assert_eq!(c(200, RAPIDE, "/api/x", true), None);
    }

    #[test]
    fn refus_d_acces_ou_de_debit_informe() {
        for statut in [401, 403, 429] {
            assert_eq!(niveau_de_fin(statut, RAPIDE), Some(Level::INFO), "{statut}");
        }
    }

    #[derive(Clone, Default)]
    struct Tampon(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for Tampon {
        fn write(&mut self, octets: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(octets);
            Ok(octets.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[actix_web::test]
    async fn journal_porte_le_modele_de_route_jamais_le_chemin_recu() {
        use actix_web::{test, web, App, HttpResponse};

        let tampon = Tampon::default();
        let ecrivain = tampon.clone();
        let abonne = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(move || ecrivain.clone())
            .finish();
        let _garde = tracing::subscriber::set_default(abonne);

        let app = test::init_service(
            App::new()
                .wrap(RequestContextMiddleware::new(Locales::fixes(&["fr"], "fr")))
                .route(
                    "/invitations/{jeton}",
                    web::get().to(|| async { HttpResponse::InternalServerError().finish() }),
                )
                .route("/ok", web::get().to(HttpResponse::Ok)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/invitations/jeton-secret?code=code-secret")
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), 500);
        let req = test::TestRequest::get().uri("/ok").to_request();
        assert_eq!(test::call_service(&app, req).await.status(), 200);

        let journal = String::from_utf8(tampon.0.lock().unwrap().clone()).unwrap();
        assert_eq!(journal.lines().count(), 1, "{journal}");
        assert!(journal.contains(" WARN "), "{journal}");
        assert!(journal.contains("route=/invitations/{jeton}"), "{journal}");
        assert!(journal.contains("statut=500"), "{journal}");
        assert!(!journal.contains("secret"), "{journal}");
    }
}
