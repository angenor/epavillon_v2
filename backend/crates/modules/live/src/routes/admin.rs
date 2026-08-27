//! Le back-office des messages d'incident — **sept routes plates**.
//!
//! # DEUX RÈGLES D'ORDRE, ET AUCUNE N'EST SUPPOSÉE
//!
//! **1. Jamais un `web::scope("/admin")`.** Le préfixe est partagé — la vitrine
//! vient de B8, le planificateur de B5, les règles de rappel de B6 — et deux
//! scopes du même préfixe **ne se complètent pas** : Actix retient le premier
//! dont le préfixe correspond et rend 404 si la route n'y figure pas. Un scope
//! ici rendrait muettes les routes des autres modules.
//!
//! **2. Les chemins littéraux AVANT les chemins de dossier.**
//! `/admin/incidents/overrun-template` et `/admin/incidents/{id}` sont toutes
//! deux en `GET` : déclarée après, la littérale serait lue comme un identifiant,
//! et le raccourci « Signaler un débordement » rendrait « message introuvable ».
//! Les deux blocs ci-dessous portent cette règle par la structure, comme le
//! module `programme`.
//!
//! # CE QUE CHAQUE ÉCRITURE REND
//!
//! `IncidentWriteResult`, **toujours, et en 200** — `forbidden` et `not_found`
//! compris. Le contrat du site range **dix issues sous un seul discriminant** et
//! l'écran les traduit une par une sous le champ concerné ; répondre 403 à
//! celles-là ferait **lever le client** là où il attend un message posé dans son
//! formulaire.
//!
//! **Ce qui reste en HTTP, c'est le périmètre** : il ne figure pas au contrat et
//! ne doit rien révéler. Périmètre vide → 403 ; édition hors périmètre → 404,
//! **jamais 403** — un identifiant hors périmètre se refuse comme un identifiant
//! inexistant.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::Perimeter;
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::payload::{
    CreateIncidentPayload, UnpublishIncidentPayload, UpdateIncidentPayload,
};
use crate::service::{list, write};
use crate::state::LiveState;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    chemins_litteraux(cfg);
    chemins_de_dossier(cfg);
}

/// **Déclarés en premier.** Voir la règle d'ordre en tête de fichier.
fn chemins_litteraux(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/incidents", web::get().to(lister))
        .route(
            "/admin/incidents/overrun-template",
            web::get().to(gabarit_de_debordement),
        )
        .route("/admin/incidents", web::post().to(creer));
}

/// Tout ce qui porte un identifiant. **Déclarés après**, sans exception.
fn chemins_de_dossier(cfg: &mut web::ServiceConfig) {
    cfg.route("/admin/incidents/{id}", web::get().to(relire))
        .route("/admin/incidents/{id}", web::put().to(corriger))
        .route("/admin/incidents/{id}/publish", web::post().to(publier))
        .route("/admin/incidents/{id}/publish", web::delete().to(depublier));
}

#[derive(Debug, Deserialize)]
pub struct EcranQuery {
    pub event_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct GabaritQuery {
    pub session_id: Uuid,
}

// ---------------------------------------------------------------------------
// Lectures
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    description = "`IncidentListScreen` — tout l'écran d'une édition **en une réponse et un instant** : l'en-tête (titre, fuseau, ville), les lignes rendues par `live.event_incidents()` **dans l'ordre où elle les rend** (actifs, programmés, brouillons, historique ; gravité décroissante à état égal), le poste de direct, les compteurs par état, les natures d'incident et les cibles visables.\n\n**Aucune permission n'est exigée** : lire les messages d'une édition qu'on administre n'est pas un privilège — un bandeau publié est de toute façon public. Ce qui est gardé, c'est le périmètre.\n\n**Les cinq portées remontent**, la portée `organization` comprise dès lors que l'organisation anime une activité de l'édition, et un message `global` apparaît sur **chaque** édition administrée : une équipe qui pilote un pavillon doit savoir qu'un bandeau d'entretien le couvre.",
    path = "/admin/incidents",
    tag = "Back-office — messages d'incident",
    operation_id = "admin_incidents_lister",
    params(("event_id" = Uuid, Query, description = "Édition dont on veut l'écran")),
    responses(
        (status = 200, description = "IncidentListScreen", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lister(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    query: web::Query<EcranQuery>,
) -> Result<HttpResponse> {
    let ecran = list::ecran(
        &state,
        &perimetre,
        query.event_id,
        &crate::routes::locale_de(&requete),
    )
    .await?;
    Ok(HttpResponse::Ok().json(ecran))
}

#[utoipa::path(
    get,
    description = "`OverrunTemplate` — de quoi pré-remplir le formulaire depuis le raccourci « Signaler un débordement » du planificateur, sans une saisie pendant que la salle attend : l'activité, son titre **résolu**, son créneau et son édition.\n\n**`title` est ici résolu et non brut**, à la différence du reste : c'est une valeur de pré-remplissage de champ, que le site pose telle quelle. Le site lit cette route par `callOrNull` — un 404 y est une réponse, pas une panne.\n\n**Cette route est déclarée AVANT `/admin/incidents/{id}`**, toutes deux étant en `GET`.",
    path = "/admin/incidents/overrun-template",
    tag = "Back-office — messages d'incident",
    operation_id = "admin_incidents_gabarit_debordement",
    params(("session_id" = Uuid, Query, description = "Activité qui déborde")),
    responses(
        (status = 200, description = "OverrunTemplate", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Activité inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn gabarit_de_debordement(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    query: web::Query<GabaritQuery>,
) -> Result<HttpResponse> {
    let gabarit = list::gabarit_de_debordement(
        &state,
        &perimetre,
        query.session_id,
        &crate::routes::locale_de(&requete),
    )
    .await?;
    Ok(HttpResponse::Ok().json(gabarit))
}

#[utoipa::path(
    get,
    description = "`ManagedIncident` — un message, pour le relire et le corriger.\n\n**L'édition d'un message se CALCULE, elle ne se lit pas** : pour les portées `session`, `event_day` et `organization`, la ligne ne porte aucune colonne d'édition. La route retrouve donc le message **par `live.event_incidents()`** sur les éditions du périmètre, ce qui rend le contrôle et la lecture indissociables.\n\nLe site la lit par `callOrNull`.",
    path = "/admin/incidents/{id}",
    tag = "Back-office — messages d'incident",
    operation_id = "admin_incidents_relire",
    params(("id" = Uuid, Path, description = "Identifiant du message")),
    responses(
        (status = 200, description = "ManagedIncident", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Message inexistant **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn relire(
    state: web::Data<LiveState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let ligne = list::relire(&state, &perimetre, chemin.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ligne))
}

// ---------------------------------------------------------------------------
// Écritures
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    description = "`CreateIncidentPayload` → `IncidentWriteResult`, **toujours en 200**. Rédiger, et publier dans le même geste si `publish` est vrai.\n\n**`granted` n'existe pas** : le site l'envoyait pour rejouer l'autorisation sur des données d'exemple, et un client qui déclare ses droits n'est pas un contrôle d'accès. **`from_event_id` reste** — c'est l'édition depuis laquelle on agit, donc l'ancre du contrôle de périmètre.\n\n**Les dix issues sortent en 200**, `forbidden` et `not_found` compris : le contrat du site les nomme et l'écran les traduit champ par champ. L'autorisation se vérifie sur la **portée visée** — l'édition de la cible, ou la portée globale pour un message `global`.",
    path = "/admin/incidents",
    tag = "Back-office — messages d'incident",
    operation_id = "admin_incidents_creer",
    responses(
        (status = 200, description = "IncidentWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "`from_event_id` inexistant **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn creer(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    corps: web::Json<CreateIncidentPayload>,
) -> Result<HttpResponse> {
    let corps = corps.into_inner();
    let resultat = write::creer(
        &state,
        &crate::routes::contexte_de(&requete, perimetre.person_id),
        &perimetre,
        corps.from_event_id,
        &corps.incident,
    )
    .await?;
    Ok(HttpResponse::Ok().json(resultat))
}

#[utoipa::path(
    put,
    description = "`UpdateIncidentPayload` → `IncidentWriteResult`. Corriger.\n\n**La portée peut changer, et l'autorisation se vérifie sur celle d'ARRIVÉE** : déplacer un message d'une édition vers la portée globale exige la permission globale.\n\n**Republier efface la dépublication** — instant, auteur, motif —, exactement comme le fait `live.publish_incident()`. Le comportement n'est pas recomposé : la fonction est appelée.",
    path = "/admin/incidents/{id}",
    tag = "Back-office — messages d'incident",
    operation_id = "admin_incidents_corriger",
    params(("id" = Uuid, Path, description = "Identifiant du message")),
    responses(
        (status = 200, description = "IncidentWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "`from_event_id` inexistant **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn corriger(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<UpdateIncidentPayload>,
) -> Result<HttpResponse> {
    let id = chemin.into_inner();
    let corps = corps.into_inner();
    let resultat = write::corriger(
        &state,
        &crate::routes::contexte_de(&requete, perimetre.person_id),
        &perimetre,
        corps.from_event_id,
        id,
        &corps.incident,
    )
    .await?;
    Ok(HttpResponse::Ok().json(resultat))
}

#[utoipa::path(
    post,
    description = "`IncidentWriteResult` (`published`). Publier un brouillon depuis la ligne de liste, ou rétablir un message retiré.\n\nAppelle `live.publish_incident(id)` : la fonction horodate, attribue depuis la session, efface le retrait **et émet** `live.incident.published`. **Le service n'émet rien** — un `emit_event` ajouté ici doublerait chaque ligne d'outbox.",
    path = "/admin/incidents/{id}/publish",
    tag = "Back-office — messages d'incident",
    operation_id = "admin_incidents_publier",
    params(("id" = Uuid, Path, description = "Identifiant du message")),
    responses(
        (status = 200, description = "IncidentWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn publier(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let resultat = write::publier(
        &state,
        &crate::routes::contexte_de(&requete, perimetre.person_id),
        &perimetre,
        chemin.into_inner(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(resultat))
}

#[utoipa::path(
    delete,
    description = "`UnpublishIncidentPayload` → `IncidentWriteResult` (`unpublished`). Retirer un bandeau, avec un motif. **Ce n'est pas une suppression** : la ligne demeure — instant, auteur, motif — et reparaît à l'historique de la liste.\n\n**Un `DELETE` porteur d'un corps, et c'est délibéré** : le chemin est celui de la publication, le verbe dit qu'on la retire, et le motif accompagne le geste.\n\nAppelle `live.unpublish_incident(id, motif)`, qui lève sur un message jamais publié ; le service **traduit** la levée en issue `not_published` plutôt que de rejouer la condition en amont.",
    path = "/admin/incidents/{id}/publish",
    tag = "Back-office — messages d'incident",
    operation_id = "admin_incidents_depublier",
    params(("id" = Uuid, Path, description = "Identifiant du message")),
    responses(
        (status = 200, description = "IncidentWriteResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn depublier(
    state: web::Data<LiveState>,
    requete: HttpRequest,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: Option<web::Json<UnpublishIncidentPayload>>,
) -> Result<HttpResponse> {
    let motif = corps.and_then(|c| c.into_inner().reason);
    let resultat = write::depublier(
        &state,
        &crate::routes::contexte_de(&requete, perimetre.person_id),
        &perimetre,
        chemin.into_inner(),
        motif.as_deref(),
    )
    .await?;
    Ok(HttpResponse::Ok().json(resultat))
}
