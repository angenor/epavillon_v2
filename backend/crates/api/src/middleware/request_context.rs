//! Intergiciel de contexte de requête.
//!
//! Lit `X-Request-Id` s'il est présent, l'engendre sinon, négocie la locale,
//! pose le contexte dans la requête et dans la portée de tâche — c'est ce qui
//! permet à une réponse d'erreur de porter le même identifiant —, et le renvoie
//! dans l'en-tête de réponse.

use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{Error, HttpMessage};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use kernel::context::{RequestContext, REQUEST_ID_HEADER};
use kernel::i18n::Locales;
use std::rc::Rc;
use std::time::{Duration, Instant};
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

        Box::pin(kernel::context::scope(
            request_id,
            async move {
                let resultat = service.call(req).await;
                let statut = match &resultat {
                    Ok(res) => res.status(),
                    Err(e) => e.as_response_error().status_code(),
                };
                journaliser_fin(statut.as_u16(), debut.elapsed());
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
