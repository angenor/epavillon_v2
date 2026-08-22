//! Assemblage de l'application HTTP.
//!
//! Le binaire ne fait que charger la configuration et lancer le serveur ; le
//! montage vit ici pour qu'un test d'intégration puisse dresser **la même**
//! application, avec ses intergiciels et son préfixe, sur une base jetable. Un
//! test qui remonterait son propre assemblage n'éprouverait pas celui-ci.

pub mod middleware;
pub mod modules;
pub mod openapi;
pub mod routes;
pub mod state;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::{web, App, Error, HttpResponse};
use kernel::error::ApiError;

use crate::middleware::auth::SessionResolver;
use crate::middleware::cors::Cors;
use crate::middleware::origin::OriginCheck;
use crate::middleware::request_context::RequestContextMiddleware;
use crate::state::AppState;

/// Préfixe prescrit par `.env.example` pour le raccordement du site.
pub const PREFIXE: &str = "/api";

/// Garde-fou d'entrée. Large pour un formulaire, étroit pour un envoi de
/// fichier — qui ne passera jamais par du JSON de toute façon.
const LIMITE_CORPS: usize = 1024 * 1024;

pub fn build_app(
    etat: &AppState,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = Error,
    >,
> {
    // `/ready` et non `/health` : le contrat réserve `/health` aux chiffres
    // d'exploitation, protégés par une permission, et confie la vivacité anonyme
    // à `/ready`. Prendre le nom protégé casserait la sonde le jour où `/health`
    // arrive — il est arrivé, et les deux vivent côte à côte dans `routes/health.rs`.
    let mut portee = web::scope(PREFIXE).configure(routes::health::configurer);

    // Servie partout sauf en production, où le document décrirait la totalité
    // de la surface d'appel à qui sonde le port.
    if etat.config.api_docs_enabled {
        let documentation = openapi::document(&etat.modules);
        portee = portee.configure(|cfg| openapi::configurer(cfg, &documentation));
    }

    // Un module absent de `platform.modules`, ou marqué `disabled`, n'a
    // simplement pas de routes : ses chemins rendent 404.
    if etat.modules.is_mounted("identity") {
        portee = portee.configure(identity::routes);
    }
    if etat.modules.is_mounted("org") {
        portee = portee.configure(org::routes);
    }
    if etat.modules.is_mounted("event") {
        portee = portee.configure(event::routes);
    }
    if etat.modules.is_mounted("programme") {
        portee = portee.configure(programme::routes);
        // `/proposal-comments` n'appartient qu'à ce module : aucun autre n'y
        // dépose, il n'y a donc rien à composer ici. `/registrations` est dans
        // le même cas depuis B5, et les deux lectures publiques du programme
        // n'exigent aucune session. **`/sessions` n'y est plus** : le module
        // Engagement y dépose depuis B6, et le préfixe est composé plus bas.
        portee = portee.configure(programme::comment_routes);
        portee = portee.configure(programme::registration_routes);
        portee = portee.configure(programme::public_schedule_routes);
    }

    // Les quatre préfixes du module Média n'appartiennent qu'à lui : aucun
    // autre module n'y dépose, il n'y a donc rien à composer plus bas.
    if etat.modules.is_mounted("media") {
        portee = portee.configure(media::routes);
    }
    if etat.modules.is_mounted("engagement") {
        portee = portee.configure(engagement::routes);
        // **La porte d'ingestion des retours de courriel n'est montée que si son
        // jeton est configuré** : sans secret, elle rend 404 comme un module
        // éteint. Le contrôle d'origine la laisse passer d'elle-même — le site
        // n'est pas un navigateur, et n'annonce donc aucune origine (R30).
        let jeton_dingestion = etat.config.mail.webhook_token.is_some();
        portee = portee.configure(move |cfg| engagement::internal_routes(cfg, jeton_dingestion));
    }

    // **Le scope `/people` est composé ici, et une seule fois.** Trois modules y
    // déposent des routes depuis B4 — l'identité pour les personnes, les
    // organisations pour leurs adhésions, les propositions pour la recherche
    // d'un intervenant par son adresse —, et deux `web::scope` du même préfixe
    // **ne se complètent pas** : Actix retient le premier dont le préfixe
    // correspond et rend 404 si la route n'y figure pas, sans essayer le
    // suivant. Le défaut s'est produit sur `/organizations` avant d'être vu ici.
    let identite = etat.modules.is_mounted("identity");
    let organisations = etat.modules.is_mounted("org");
    let propositions = etat.modules.is_mounted("programme");
    if identite || organisations || propositions {
        portee = portee.service(web::scope("/people").configure(move |cfg| {
            if identite {
                identity::people_routes(cfg);
            }
            if organisations {
                org::people_routes(cfg);
            }
            if propositions {
                programme::people_routes(cfg);
            }
        }));
    }

    // **Le scope `/organizations` est composé ici depuis B4, et une seule
    // fois.** Il appartenait au module Organisations, qui l'ouvrait lui-même ;
    // le module Propositions y déposant l'espace d'une organisation et ses
    // éditions, le laisser là aurait rendu ces deux routes **muettes** — c'est
    // exactement le défaut qui a coûté trois routes sur vingt et une en B2, et
    // le reproduire alors qu'il est raconté six lignes plus haut serait
    // difficile à défendre.
    //
    // **Aucune route n'a changé de chemin**, et l'ordre d'enregistrement est
    // celui d'avant : `org` d'abord, `programme` ensuite.
    if organisations || propositions {
        portee = portee.service(web::scope("/organizations").configure(move |cfg| {
            if organisations {
                org::organization_routes(cfg);
            }
            if propositions {
                programme::organization_routes(cfg);
            }
        }));
    }

    // **Le scope `/sessions` est composé ici depuis B6, et une seule fois.**
    //
    // Il appartenait au module Programmation, qui l'ouvrait lui-même depuis B5 ;
    // le module Engagement y déposant le calendrier des rappels d'une séance et
    // la règle qui s'y applique, le laisser là aurait rendu ces deux routes
    // **muettes** — le défaut qui a coûté trois routes sur vingt et une en B2,
    // puis a failli se reproduire sur `/organizations` en B4.
    //
    // **Aucune route de B5 ne change de chemin**, et l'ordre d'enregistrement
    // est celui d'avant : `programme` d'abord, `engagement` ensuite.
    let engagement_monte = etat.modules.is_mounted("engagement");
    if propositions || engagement_monte {
        portee = portee.service(web::scope("/sessions").configure(move |cfg| {
            if propositions {
                programme::session_routes(cfg);
            }
            if engagement_monte {
                engagement::session_routes(cfg);
            }
        }));
    }

    // **Le scope `/admin/planner` est composé ici, et une seule fois** — même
    // motif que `/people` ci-dessus, écrit AVANT que le défaut ne se reproduise.
    // Il porte désormais les DEUX modules : `event` y dépose le contrôle
    // préalable et la publication (B3), `programme` l'écran du planificateur
    // (B5). Deux `web::scope` du même préfixe ne se complètent pas : Actix
    // retient le premier et rend 404 sur les routes du second.
    let evenements = etat.modules.is_mounted("event");
    if evenements || propositions {
        portee = portee.service(web::scope("/admin/planner").configure(move |cfg| {
            if evenements {
                event::planner_routes(cfg);
            }
            if propositions {
                programme::planner_routes(cfg);
            }
        }));
    }

    App::new()
        .app_data(web::Data::new(etat.db.clone()))
        .app_data(web::Data::from(etat.config.clone()))
        .app_data(web::Data::from(etat.passwords.clone()))
        .app_data(web::Data::from(etat.mailer.clone()))
        .app_data(web::Data::new(etat.locales.clone()))
        .app_data(web::Data::new(etat.modules.clone()))
        .app_data(web::Data::new(etat.identity.clone()))
        .app_data(web::Data::new(etat.org.clone()))
        .app_data(web::Data::new(etat.event.clone()))
        .app_data(web::Data::new(etat.programme.clone()))
        .app_data(web::Data::new(etat.media.clone()))
        .app_data(web::Data::new(etat.engagement.clone()))
        .app_data(web::Data::from(etat.identity.token_codec()))
        .app_data(web::Data::new(etat.clone()))
        .app_data(corps_json())
        .wrap(SessionResolver)
        .wrap(OriginCheck::new(etat.allowed_origins.clone()))
        .wrap(RequestContextMiddleware::new(etat.locales.clone()))
        // **Le plus à l'extérieur**, et ce n'est pas indifférent : il enveloppe
        // aussi les refus des trois autres. Sans cela, le navigateur masque un
        // 401 ou un 403 au code du site, qui affiche une panne réseau à la
        // place du message que l'API a composé.
        .wrap(Cors::new(etat.allowed_origins.clone()))
        .service(portee)
        // Un chemin inconnu rend un corps d'erreur du catalogue, pas la réponse
        // vide d'Actix : le principe IX ne souffre pas d'exception parce que la
        // route n'existe pas.
        .default_service(web::to(|| async {
            Err::<HttpResponse, ApiError>(ApiError::not_found())
        }))
}

/// Sans cela, un champ manquant sortirait en 400 avec le texte anglais de serde
/// — sans code stable, sans message français, sans identifiant de requête.
fn corps_json() -> web::JsonConfig {
    web::JsonConfig::default()
        .limit(LIMITE_CORPS)
        .error_handler(|erreur, _| ApiError::from(&erreur).into())
}
