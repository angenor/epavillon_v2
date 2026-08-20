//! En-têtes CORS — le pendant, côté réponse, de `OriginCheck`.
//!
//! Le site appelle depuis un autre port avec `credentials: 'include'` : sans ces
//! en-têtes, **aucun appel de navigateur n'aboutit**. `curl` n'en a jamais eu
//! besoin, ce qui est exactement pourquoi le manque a pu vivre si longtemps.
//!
//! **La liste d'origines est celle d'`OriginCheck`**, normalisée par la même
//! fonction. Les deux doivent dire la même chose : l'une décide ce qui a le
//! droit d'écrire, l'autre ce que le navigateur a le droit de lire, et une
//! majuscule ou une barre oblique finale qui les ferait diverger produirait un
//! refus que personne ne saurait expliquer.
//!
//! **Jamais `*`.** Une origine générique est incompatible avec les cookies : le
//! navigateur refuse `Allow-Origin: *` dès que `Allow-Credentials` est vrai.
//! L'origine est donc renvoyée telle qu'annoncée, après contrôle.
//!
//! **Les réponses d'ERREUR portent les en-têtes elles aussi.** C'est le détail
//! qui se perd : sans eux, un 401 ou un 403 est masqué par le navigateur, et
//! l'écran affiche une panne réseau à la place du message français que l'API a
//! pris soin de composer. D'où la place de cet intergiciel — **le plus à
//! l'extérieur**, pour envelopper aussi ce que les autres refusent.

use actix_web::body::{BoxBody, EitherBody, MessageBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header::{
    HeaderName, HeaderValue, ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
    ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS,
    ACCESS_CONTROL_MAX_AGE, ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD, ORIGIN,
    VARY,
};
use actix_web::http::{Method, StatusCode};
use actix_web::{Error, HttpResponse};
use futures_util::future::{ready, LocalBoxFuture, Ready};
use std::collections::HashSet;
use std::rc::Rc;

use crate::middleware::origin::normaliser;

/// Ce que le contrat annonce : l'identifiant de requête voyage sur **toute**
/// réponse. Sans cette ligne, le navigateur le cache au code du site, et
/// personne ne peut le citer dans un signalement d'incident.
const EXPOSES: &str = "X-Request-Id";

const METHODES: &str = "GET, POST, PUT, DELETE, OPTIONS";

/// Dix minutes. Assez pour qu'une navigation ne repaye pas le préalable à
/// chaque appel, assez court pour qu'un changement de configuration se voie
/// dans la journée.
const DUREE_PREALABLE: &str = "600";

#[derive(Clone)]
pub struct Cors {
    autorisees: Rc<HashSet<String>>,
}

impl Cors {
    pub fn new(origines: impl IntoIterator<Item = String>) -> Self {
        Self {
            autorisees: Rc::new(origines.into_iter().map(|o| normaliser(&o)).collect()),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Cors
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = CorsService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CorsService {
            service: Rc::new(service),
            autorisees: self.autorisees.clone(),
        }))
    }
}

pub struct CorsService<S> {
    service: Rc<S>,
    autorisees: Rc<HashSet<String>>,
}

impl<S, B> Service<ServiceRequest> for CorsService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let origine = req
            .headers()
            .get(ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(normaliser)
            .filter(|o| self.autorisees.contains(o));

        // Un préalable est un OPTIONS qui annonce la méthode qu'il prépare. Il
        // ne doit atteindre aucune route : le chemin visé peut n'accepter que
        // POST, et le routeur rendrait 404 ou 405 là où le navigateur attend
        // une permission.
        let prealable = req.method() == Method::OPTIONS
            && req.headers().contains_key(ACCESS_CONTROL_REQUEST_METHOD);

        if prealable {
            let demandes = req
                .headers()
                .get(ACCESS_CONTROL_REQUEST_HEADERS)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let (requete, _) = req.into_parts();
            let reponse = reponse_prealable(origine.as_deref(), demandes.as_deref());
            return Box::pin(ready(Ok(
                ServiceResponse::new(requete, reponse).map_into_right_body()
            )));
        }

        let service = self.service.clone();
        Box::pin(async move {
            let mut reponse = service.call(req).await?;
            if let Some(origine) = origine {
                poser(reponse.headers_mut(), &origine);
            }
            // `Vary: Origin` **même sans origine reconnue** : la réponse dépend
            // de l'en-tête, et un cache partagé qui l'ignorerait servirait à
            // tout le monde la version d'un seul.
            ajouter_vary(reponse.headers_mut(), "Origin");
            Ok(reponse.map_into_left_body())
        })
    }
}

fn reponse_prealable(origine: Option<&str>, demandes: Option<&str>) -> HttpResponse {
    let mut reponse = HttpResponse::build(StatusCode::NO_CONTENT).finish();
    let entetes = reponse.headers_mut();

    if let Some(origine) = origine {
        poser(entetes, origine);
        inserer(entetes, ACCESS_CONTROL_ALLOW_METHODS, METHODES);
        inserer(entetes, ACCESS_CONTROL_MAX_AGE, DUREE_PREALABLE);
        // Les en-têtes demandés sont RENVOYÉS tels quels plutôt que comparés à
        // une liste fermée : l'origine est déjà contrôlée, et une liste écrite
        // ici échouerait en silence le jour où le site ajoute un en-tête.
        if let Some(demandes) = demandes {
            inserer(entetes, ACCESS_CONTROL_ALLOW_HEADERS, demandes);
        }
    }

    for cle in [
        "Origin",
        "Access-Control-Request-Method",
        "Access-Control-Request-Headers",
    ] {
        ajouter_vary(entetes, cle);
    }
    reponse
}

fn poser(entetes: &mut actix_web::http::header::HeaderMap, origine: &str) {
    inserer(entetes, ACCESS_CONTROL_ALLOW_ORIGIN, origine);
    inserer(entetes, ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
    inserer(entetes, ACCESS_CONTROL_EXPOSE_HEADERS, EXPOSES);
}

fn inserer(entetes: &mut actix_web::http::header::HeaderMap, cle: HeaderName, valeur: &str) {
    if let Ok(valeur) = HeaderValue::from_str(valeur) {
        entetes.insert(cle, valeur);
    }
}

fn ajouter_vary(entetes: &mut actix_web::http::header::HeaderMap, cle: &str) {
    let deja = entetes
        .get(VARY)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned();

    if deja.split(',').any(|d| d.trim().eq_ignore_ascii_case(cle)) {
        return;
    }

    let valeur = if deja.is_empty() {
        cle.to_owned()
    } else {
        format!("{deja}, {cle}")
    };
    inserer(entetes, VARY, &valeur);
}
