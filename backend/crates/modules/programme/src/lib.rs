//! Module `programme` — schéma PostgreSQL `programme`.
//!
//! Ne dépend que de `kernel` et de `contracts` (principe II). Les dossiers
//! déposés par les organisations, leur cycle de vie, leur évaluation par le
//! comité et les échanges qui l'accompagnent.
//!
//! **Un seul crate pour tout le schéma** : B4 y pose la partie « propositions »
//! (fichier `070`), B5 y ajoutera la partie « séances » (fichier `075`) sans en
//! créer un second (research.md § R1). Les dossiers internes sont donc nommés
//! par agrégat, jamais par « proposition ».
//!
//! **Rien n'est à semer** : `platform.modules` porte déjà l'entrée `programme`
//! (schéma `programme`, dépendant d'`event`, `org` et `identity`), posée par
//! `010_platform.sql` § 7.
//!
//! **Aucun travail différé** (R20). C'est le premier module du jalon dans ce
//! cas, et c'est un fait à vérifier plutôt qu'un oubli à constater : rien ici
//! n'a d'effet à échéance. Les rappels de revue et les avis de dépôt
//! appartiennent à B6 et se déclencheront sur les événements du service ; la
//! clôture d'un appel échu appartient à B3 et y est livrée. Ce crate n'expose
//! donc **pas** de `job_handlers()`, et le worker n'est pas modifié.

use actix_web::web::ServiceConfig;

pub mod consumers;
pub mod domain;
pub mod repo;
pub mod routes;
pub mod service;
pub mod state;

pub use state::ProgrammeState;

/// Routes propres au module. Le montage est décidé par l'API d'après
/// `platform.modules` : le module ne teste pas son propre état.
///
/// # Les chemins littéraux avant les chemins paramétrés — et à quelle condition
///
/// Actix construit une ressource par motif de chemin et retient la première
/// dont le motif correspond. **Mesuré plutôt que supposé** : quand la méthode
/// ne correspond à aucune route de cette ressource, il ne rend pas 405 — il
/// **poursuit**, et un chemin non servi finit sur la route par défaut de l'API,
/// donc en 404.
///
/// Conséquence : le risque de capture n'existe **que lorsque les méthodes se
/// recouvrent**. Aujourd'hui `/proposals/{id}` n'est servi qu'en `PUT`, et les
/// quatre chemins littéraux sont des `GET` et un `POST` : rien ne se capture,
/// quel que soit l'ordre.
///
/// **Cela changera à US4**, où `GET /proposals/{id}` arrive : `/proposals/list`,
/// `/dashboard`, `/committee`, `/transitions`, `/form-context` et `/draft`
/// entreront alors tous en concurrence avec lui, sur la même méthode. Le
/// découpage en `chemins_litteraux` / `chemins_de_dossier` est posé **avant**
/// ce moment, pour que la règle soit tenue par la structure et non par la
/// vigilance : une route ajoutée au mauvais groupe se voit à la relecture, une
/// route ajoutée à la fin d'une liste plate ne se voit pas.
pub fn routes(cfg: &mut ServiceConfig) {
    use actix_web::web;

    cfg.service(
        web::scope("/proposals")
            .configure(routes::submission::chemins_litteraux)
            .configure(routes::admin_list::chemins_litteraux)
            .configure(routes::detail::chemins_litteraux)
            .configure(routes::submission::chemins_de_dossier)
            .configure(routes::detail::chemins_de_dossier)
            .configure(routes::admin_desk::chemins_de_dossier)
            .configure(routes::workspace::chemins_de_dossier),
    );
}

/// Ce que ce module dépose sous `/people`, **sans le préfixe**.
///
/// Une seule route : la recherche d'un intervenant par son adresse. Le préfixe
/// est composé par l'API depuis B1 — deux `web::scope` du même préfixe **ne se
/// complètent pas**, Actix retient le premier et rend 404 sur les routes du
/// second, sans essayer le suivant.
pub fn people_routes(cfg: &mut ServiceConfig) {
    routes::people::configurer(cfg);
}

/// Ce que ce module dépose sous `/organizations`, **sans le préfixe**.
///
/// Deux routes : l'espace d'une organisation et ses éditions. Le préfixe
/// appartenait à un `web::scope` **unique** du module Organisations depuis B2 ;
/// il est désormais composé par l'API, sur le patron de `/people`, et
/// `org::organization_routes()` y dépose sa part. **Aucune route n'a changé de
/// chemin**, et `crates/api/tests/routes_org.rs` le prouve (research.md § R18).
pub fn organization_routes(cfg: &mut ServiceConfig) {
    routes::workspace::sous_organizations(cfg);
}

/// Ce que ce module dépose sous `/admin/planner`, **sans le préfixe**.
///
/// Le scope est composé par l'API **à partir de deux modules** : B3 y a déposé
/// le contrôle préalable et la publication, B5 y dépose l'écran du
/// planificateur. Deux `web::scope` du même préfixe **ne se complètent pas** —
/// Actix retient le premier et rend 404 sur les routes du second, et le défaut a
/// déjà coûté trois routes muettes sur vingt et une en B2.
pub fn planner_routes(cfg: &mut ServiceConfig) {
    routes::planner::configurer(cfg);
}

/// Les routes du module qui vivent sous `/sessions`, **sans le préfixe**.
///
/// Le scope était ouvert ici jusqu'à B6 ; le module Engagement y déposant
/// désormais le calendrier des rappels d'une séance et la règle qui s'y
/// applique, il est composé par l'API — même patron que `/people` depuis B1 et
/// `/organizations` depuis B4, appliqué pour la même raison : deux `web::scope`
/// du même préfixe **ne se complètent pas**, Actix retient le premier et rend
/// 404 sur les routes du second.
///
/// **Aucune route n'a changé de chemin** : l'ordre d'enregistrement est celui
/// d'avant, et `crates/api/tests/` frappe les dix-sept.
///
/// **Chemins littéraux avant chemins paramétrés.** `/sessions/conflicts` et
/// `/sessions/{id}/…` ne se recouvrent pas — le risque de capture n'existe que
/// lorsque les méthodes ET le nombre de segments coïncident —, mais l'ordre est
/// tenu par la structure plutôt que par la vigilance : une route ajoutée au
/// mauvais groupe se voit à la relecture.
pub fn session_routes(cfg: &mut ServiceConfig) {
    routes::sessions::chemins_litteraux(cfg);
    routes::sessions::chemins_de_seance(cfg);
}

/// Le scope `/registrations`, et les deux lectures publiques du programme.
pub fn registration_routes(cfg: &mut ServiceConfig) {
    use actix_web::web;

    cfg.service(
        web::scope("/registrations")
            .configure(routes::registrations::chemins_litteraux)
            .configure(routes::registrations::chemins_dinscription),
    );
}

/// La programmation publique — **aucune session exigée**.
pub fn public_schedule_routes(cfg: &mut ServiceConfig) {
    routes::public_schedule::configurer(cfg);
}

/// Ce que ce module consomme de l'outbox — **le premier consommateur du
/// dépôt**.
///
/// La garde de rejeu n'est pas ici : le relais du noyau réserve
/// `(consommateur, événement)` **avant** d'appeler `handle`, et n'appelle pas le
/// consommateur une seconde fois (research.md § R13).
pub fn event_consumers() -> Vec<std::sync::Arc<dyn kernel::events::EventConsumer>> {
    vec![std::sync::Arc::new(
        consumers::publication::PublicationConsumer,
    )]
}

/// Les deux scopes **propres à ce module** : personne d'autre n'y dépose, il
/// n'y a donc rien à composer dans l'API.
pub fn comment_routes(cfg: &mut ServiceConfig) {
    use actix_web::web;

    cfg.service(
        web::scope("/proposal-comments").configure(routes::workspace::sous_proposal_comments),
    )
    .service(web::scope("/admin/proposals").configure(routes::admin_ops::configurer));
}
