//! Les onglets d'une édition — journées, fils, lieux, salles, canaux.
//!
//! **Ces routes ne portent pas leur édition dans leur chemin**, et c'est le
//! contrat du front : `/admin/venues/{id}`, `/admin/rooms/{id}`,
//! `/admin/channels/{id}`, `/admin/tracks/{id}`. L'édition vient donc de
//! **l'ascendance de l'objet**, résolue en base avant que le périmètre soit
//! vérifié (research.md § R2).
//!
//! **L'`event_id` du corps est ignoré.** Le front l'envoie dans les charges
//! utiles d'écriture et dans les corps de suppression (`{ event_id }`) ; c'est
//! un droit *déclaré par le client*. Seule exception, et elle est vérifiée : à
//! la **création**, l'objet n'a pas encore d'ascendance — l'édition ne peut
//! venir que du corps, et elle passe alors par le même garde que les autres.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::{Perimeter, Scope};
use kernel::error::Result;
use uuid::Uuid;

use crate::domain::ids::{ChannelId, EventDayId, EventId, RoomId, TrackId, VenueId};
use crate::domain::permissions::EVENT_MANAGE;
use crate::domain::tabs::{
    DayGenerationRequest, EditionChannelPayload, EditionDayPayload, EditionRoomPayload,
    EditionTrackPayload, EditionVenuePayload,
};
use crate::routes::contexte_de;
use crate::service::{
    canal_dans_le_perimetre, channels as service_canaux, days as service_journees,
    edition_dans_le_perimetre, tracks as service_fils, venues as service_lieux, CanalCible, Cible,
};
use crate::state::EventState;

/// Les trois scopes de cet écran, remplis **une seule fois chacun**.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin/venues")
            .route("", web::post().to(creer_lieu))
            .route("/{id}", web::put().to(modifier_lieu))
            .route("/{id}", web::delete().to(supprimer_lieu)),
    )
    .service(
        web::scope("/admin/rooms")
            .route("", web::post().to(creer_salle))
            .route("/{id}", web::put().to(modifier_salle))
            .route("/{id}", web::delete().to(supprimer_salle)),
    )
    .service(
        web::scope("/admin/channels")
            .route("", web::post().to(creer_canal))
            .route("/{id}", web::put().to(modifier_canal))
            .route("/{id}", web::delete().to(supprimer_canal)),
    )
    .service(
        web::scope("/admin/tracks")
            .route("", web::post().to(creer_fil))
            .route("/{id}", web::put().to(modifier_fil))
            .route("/{id}", web::delete().to(supprimer_fil)),
    );
}

/// La permission de gérer les **événements**, sur l'édition visée. Écrite une
/// fois : neuf routes la partagent.
pub(crate) async fn autoriser(
    state: &EventState,
    perimetre: &Perimeter,
    event_id: EventId,
) -> Result<()> {
    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        EVENT_MANAGE,
        Scope::Event(event_id.as_uuid()),
    )
    .await
}

/// Le garde commun des routes paramétrées : **résoudre l'ascendance, vérifier le
/// périmètre, puis la permission**.
pub(crate) async fn garder(
    state: &EventState,
    perimetre: &Perimeter,
    cible: Cible,
) -> Result<EventId> {
    let event_id = edition_dans_le_perimetre(state.pool(), perimetre, cible).await?;
    autoriser(state, perimetre, event_id).await?;
    Ok(event_id)
}

// -----------------------------------------------------------------------------
// Lieux
// -----------------------------------------------------------------------------

/// Créer un lieu.
#[utoipa::path(
    post,
    description = "`EditionVenuePayload` → `EditionTabResult`. L'édition vient du corps **et elle est vérifiée** : à la création, l'objet n'a pas encore d'ascendance en base. La réponse porte la **composition entière recalculée**, ce qui garantit que les décomptes des cinq autres onglets restent justes.",
    path = "/admin/venues",
    tag = "Back-office — événements",
    operation_id = "admin_lieu_creer",
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer_lieu(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    corps: web::Json<EditionVenuePayload>,
) -> Result<HttpResponse> {
    let payload = corps.into_inner();
    let event_id = garder(
        &state,
        &perimetre,
        Cible::Edition(EventId::from(payload.event_id)),
    )
    .await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = service_lieux::enregistrer_lieu(&state, &ctx, event_id, None, payload).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Modifier un lieu.
#[utoipa::path(
    put,
    description = "`EditionVenuePayload` → `EditionTabResult`. **L'édition vient de l'ascendance du lieu**, jamais du corps. Écriture totale : tous les champs modifiables sont réécrits, y compris à nul.",
    path = "/admin/venues/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_lieu_modifier",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du lieu")),
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Lieu inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier_lieu(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<EditionVenuePayload>,
) -> Result<HttpResponse> {
    let id = VenueId::from(chemin.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Lieu(id)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        service_lieux::enregistrer_lieu(&state, &ctx, event_id, Some(id), corps.into_inner())
            .await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Retirer un lieu — **et ses salles avec lui**.
#[utoipa::path(
    delete,
    description = "`EditionTabResult`. Retirer un lieu emporte ses salles par cascade ; les séances qui s'y tenaient **ne disparaissent pas**, elles retournent au panneau « à placer ». `sessions_detached` les compte, **avant** l'ordre de suppression : après, le lien n'existe plus et le chiffre serait toujours zéro. Le corps de la requête est **ignoré** — l'édition vient de l'ascendance du lieu.",
    path = "/admin/venues/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_lieu_supprimer",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du lieu")),
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Lieu inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn supprimer_lieu(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = VenueId::from(chemin.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Lieu(id)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = service_lieux::supprimer_lieu(&state, &ctx, event_id, id).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

// -----------------------------------------------------------------------------
// Salles
// -----------------------------------------------------------------------------

/// Créer une salle.
#[utoipa::path(
    post,
    description = "`EditionRoomPayload` → `EditionTabResult`. L'édition vient du **lieu** désigné par la charge utile, vérifié en base : sans cela, une salle pourrait être posée dans le lieu d'une autre édition. **`is_virtual` est écrit tel quel et jamais déduit du mode de participation** — une salle virtuelle accepte les créneaux simultanés, et la déduire ferait taire le conflit de gravité haute qu'un stand unique doit signaler.",
    path = "/admin/rooms",
    tag = "Back-office — événements",
    operation_id = "admin_salle_creer",
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Lieu inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer_salle(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    corps: web::Json<EditionRoomPayload>,
) -> Result<HttpResponse> {
    let payload = corps.into_inner();
    let event_id = garder(
        &state,
        &perimetre,
        Cible::Lieu(VenueId::from(payload.venue_id)),
    )
    .await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = service_lieux::enregistrer_salle(&state, &ctx, event_id, None, payload).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Modifier une salle.
#[utoipa::path(
    put,
    description = "`EditionRoomPayload` → `EditionTabResult`. **L'édition vient de l'ascendance de la salle** — son lieu —, jamais du corps. Le lieu visé par la charge utile est vérifié : déplacer une salle d'un lieu à l'autre de la **même** édition est légitime, la déplacer ailleurs ne l'est pas.",
    path = "/admin/rooms/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_salle_modifier",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de la salle")),
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Salle inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier_salle(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<EditionRoomPayload>,
) -> Result<HttpResponse> {
    let id = RoomId::from(chemin.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Salle(id)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        service_lieux::enregistrer_salle(&state, &ctx, event_id, Some(id), corps.into_inner())
            .await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Retirer une salle.
#[utoipa::path(
    delete,
    description = "`EditionTabResult`. Les séances installées dans cette salle **retournent au panneau « à placer »** — la clé est `ON DELETE SET NULL`, aucune séance n'est perdue. `sessions_detached` les compte **avant** l'ordre de suppression.",
    path = "/admin/rooms/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_salle_supprimer",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de la salle")),
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Salle inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn supprimer_salle(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = RoomId::from(chemin.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Salle(id)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = service_lieux::supprimer_salle(&state, &ctx, event_id, id).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

// -----------------------------------------------------------------------------
// Canaux de diffusion
// -----------------------------------------------------------------------------

/// Créer un canal d'édition.
#[utoipa::path(
    post,
    description = "`EditionChannelPayload` → `EditionTabResult`. **Poser le canal par défaut retire le précédent dans la même transaction** : `ux_broadcast_channels_default` n'est pas différable, et l'ordre inverse échouerait. Le canal **général de la plateforme** forme son propre groupe et n'est jamais délogé — il sert les diffusions dont l'événement n'a pas le sien.",
    path = "/admin/channels",
    tag = "Back-office — événements",
    operation_id = "admin_canal_creer",
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer_canal(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    corps: web::Json<EditionChannelPayload>,
) -> Result<HttpResponse> {
    let payload = corps.into_inner();
    let event_id = garder(
        &state,
        &perimetre,
        Cible::Edition(EventId::from(payload.event_id)),
    )
    .await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = service_canaux::enregistrer(&state, &ctx, event_id, None, payload).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Modifier un canal — **jamais un canal général de la plateforme**.
#[utoipa::path(
    put,
    description = "`EditionChannelPayload` → `EditionTabResult`. Un canal **général de la plateforme** — sans édition — rend `platform_channel` en 200 : il sert plusieurs événements, et le modifier depuis l'un d'eux le changerait pour tous. Ce n'est ni un introuvable ni un refus de périmètre, c'est un refus que l'écran sait expliquer.",
    path = "/admin/channels/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_canal_modifier",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du canal")),
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Canal inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier_canal(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<EditionChannelPayload>,
) -> Result<HttpResponse> {
    let id = ChannelId::from(chemin.into_inner());

    let event_id = match canal_dans_le_perimetre(state.pool(), &perimetre, id).await? {
        CanalCible::Plateforme => {
            return Ok(HttpResponse::Ok().json(service_canaux::refus_de_canal_de_plateforme()));
        }
        CanalCible::Edition(event_id) => event_id,
    };
    autoriser(&state, &perimetre, event_id).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        service_canaux::enregistrer(&state, &ctx, event_id, Some(id), corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Retirer un canal — **désactivé s'il a servi, supprimé sinon**.
#[utoipa::path(
    delete,
    description = "`EditionTabResult`. **`error_code: 'deactivated'` accompagne `ok: true` et n'est PAS un refus** : le canal a servi, il est désactivé plutôt que supprimé, pour garder la trace du canal sur lequel une activité passée a été diffusée. Sans séance à son compte, il est supprimé et `error_code` reste nul. `sessions_detached` porte le nombre de séances concernées, **compté avant**.",
    path = "/admin/channels/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_canal_supprimer",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du canal")),
    responses(
        (status = 200, description = "EditionTabResult — `deactivated` est un succès", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Canal inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn supprimer_canal(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = ChannelId::from(chemin.into_inner());

    let event_id = match canal_dans_le_perimetre(state.pool(), &perimetre, id).await? {
        CanalCible::Plateforme => {
            return Ok(HttpResponse::Ok().json(service_canaux::refus_de_canal_de_plateforme()));
        }
        CanalCible::Edition(event_id) => event_id,
    };
    autoriser(&state, &perimetre, event_id).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = service_canaux::retirer(&state, &ctx, event_id, id).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

// -----------------------------------------------------------------------------
// Journées du calendrier
// -----------------------------------------------------------------------------

/// **Ce que la génération ferait, sans rien faire.**
#[utoipa::path(
    get,
    description = "`DayGenerationPlan | null` — **lecture seule : rien ne s'écrit**. Les dates de la période qui n'ont pas encore de journée, les journées **hors période avec le nombre de séances qu'elles portent**, et le nombre de journées déjà en place. Une période d'un an annonce plus de trois cents journées sans en écrire une. Ce chiffre par journée est ce qui permet à l'équipe d'arbitrer plutôt que de subir un retrait.",
    path = "/admin/events/{id}/days/plan",
    tag = "Back-office — événements",
    operation_id = "admin_journees_plan",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    responses(
        (status = 200, description = "DayGenerationPlan | null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn plan_des_journees(
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = EventId::from(chemin.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Edition(id)).await?;

    let plan = service_journees::plan(state.pool(), event_id).await?;

    Ok(HttpResponse::Ok().json(plan))
}

/// Générer le calendrier.
#[utoipa::path(
    post,
    description = "`{ remove_outside_period }` → `EditionTabResult`. **Le plan est recalculé dans la transaction d'écriture**, jamais repris du client : entre l'affichage et le clic, quelqu'un peut avoir modifié la période, et écrire d'après un état périmé reviendrait à supprimer une journée qui vient d'y entrer. **Sans le drapeau, aucune journée n'est retirée.** `sessions_detached` est compté **avant** le retrait. Le contenu éditorial des journées conservées n'est jamais écrasé.",
    path = "/admin/events/{id}/days",
    tag = "Back-office — événements",
    operation_id = "admin_journees_generer",
    params(("id" = uuid::Uuid, Path, description = "Identifiant de l'édition")),
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn generer_les_journees(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<DayGenerationRequest>,
) -> Result<HttpResponse> {
    let id = EventId::from(chemin.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Edition(id)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        service_journees::generer(&state, &ctx, event_id, corps.remove_outside_period).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Habiller une journée — contenu **éditorial** seul.
#[utoipa::path(
    put,
    description = "`EditionDayPayload` → `EditionTabResult`. Titre, adresse de page, description, mise en avant et couleur. **La date ne se modifie pas** : une journée tient sa date de la période de l'édition, et la déplacer ferait un doublon ou un trou. L'édition vient de l'adresse ; l'identifiant de la journée est vérifié comme appartenant à cette édition.",
    path = "/admin/events/{id}/days/{dayId}",
    tag = "Back-office — événements",
    operation_id = "admin_journee_habiller",
    params(
        ("id" = uuid::Uuid, Path, description = "Identifiant de l'édition"),
        ("dayId" = uuid::Uuid, Path, description = "Identifiant de la journée"),
    ),
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Journée inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn habiller_une_journee(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<(Uuid, Uuid)>,
    corps: web::Json<EditionDayPayload>,
) -> Result<HttpResponse> {
    let (_, day_id) = chemin.into_inner();
    let journee = EventDayId::from(day_id);
    // **L'édition vient de l'ascendance de la JOURNÉE**, pas du chemin : sans
    // cela, une journée d'une autre édition pourrait être habillée en passant
    // par une édition qu'on administre.
    let event_id = garder(&state, &perimetre, Cible::Journee(journee)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        service_journees::habiller(&state, &ctx, event_id, journee, corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

// -----------------------------------------------------------------------------
// Fils de programmation — les journées spéciales
// -----------------------------------------------------------------------------

/// Créer un fil.
#[utoipa::path(
    post,
    description = "`EditionTrackPayload` → `EditionTabResult`. L'édition vient du corps **et elle est vérifiée**. Le fil, ses **thématiques** et sa page publique sont écrits dans le même geste : les séparer laisserait exister un fil publié sans ses pastilles. Les thématiques passent par le référentiel partagé, avec leur libellé et leur couleur — ce sont des **données**, jamais des traductions.",
    path = "/admin/tracks",
    tag = "Back-office — événements",
    operation_id = "admin_fil_creer",
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Édition inexistante **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn creer_fil(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    corps: web::Json<EditionTrackPayload>,
) -> Result<HttpResponse> {
    let payload = corps.into_inner();
    let event_id = garder(
        &state,
        &perimetre,
        Cible::Edition(EventId::from(payload.event_id)),
    )
    .await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = service_fils::enregistrer(&state, &ctx, event_id, None, payload).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Modifier un fil.
#[utoipa::path(
    put,
    description = "`EditionTrackPayload` → `EditionTabResult`. **L'édition vient de l'ascendance du fil**, jamais du corps. L'unicité du code et de l'adresse porte sur l'**édition** : deux COP peuvent chacune avoir leur « journée finance ». Refermer puis rouvrir la page publique **n'efface pas** la date de sa première ouverture.",
    path = "/admin/tracks/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_fil_modifier",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du fil")),
    request_body = Object,
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn modifier_fil(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
    corps: web::Json<EditionTrackPayload>,
) -> Result<HttpResponse> {
    let id = TrackId::from(chemin.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Fil(id)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat =
        service_fils::enregistrer(&state, &ctx, event_id, Some(id), corps.into_inner()).await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Supprimer un fil — **la seule suppression du module qui cascade sur un
/// rattachement éditorial**.
#[utoipa::path(
    delete,
    description = "`EditionTabResult`. **Aucune séance n'est supprimée** : ce qui disparaît, ce sont les rattachements séance–fil, par cascade. `sessions_detached` compte ce travail éditorial perdu, **avant** l'ordre de suppression — après, le lien n'existe plus et le chiffre serait zéro. Le corps de la requête est ignoré.",
    path = "/admin/tracks/{id}",
    tag = "Back-office — événements",
    operation_id = "admin_fil_supprimer",
    params(("id" = uuid::Uuid, Path, description = "Identifiant du fil")),
    responses(
        (status = 200, description = "EditionTabResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission absente, ou périmètre d'administration vide", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Fil inexistant **ou hors périmètre** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn supprimer_fil(
    requete: HttpRequest,
    state: web::Data<EventState>,
    perimetre: Perimeter,
    chemin: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let id = TrackId::from(chemin.into_inner());
    let event_id = garder(&state, &perimetre, Cible::Fil(id)).await?;

    let ctx = contexte_de(&requete, perimetre.person_id);
    let resultat = service_fils::supprimer(&state, &ctx, event_id, id).await?;

    Ok(HttpResponse::Ok().json(resultat))
}
