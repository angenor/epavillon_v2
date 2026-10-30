//! La programmation publique — **deux routes, aucune session exigée**.
//!
//! Elles ne prennent aucun périmètre et ne gardent rien : une programmation
//! publiée est publique. C'est le pendant, côté séances, des éditions publiques
//! de B3.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::error::Result;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::EventId;
use crate::service::public_schedule;
use crate::state::ProgrammeState;

#[derive(Debug, Deserialize)]
pub struct EditionDemandee {
    /// **Facultative.** Absente, ce sont les séances à venir de toutes les
    /// éditions — ce que compose l'accueil du site, qui n'a pas d'édition à
    /// nommer.
    event_id: Option<Uuid>,
    limit: Option<i64>,
}

/// Plafond de la lecture sans édition. Assez large pour que l'accueil compose
/// sa liste, assez étroit pour qu'une page publique ne puisse pas demander la
/// programmation entière de toutes les COP.
const LIMITE_PAR_DEFAUT: i64 = 50;
const LIMITE_MAX: i64 = 200;

/// Les deux chemins publics. **Ils ne vivent pas sous `/sessions`** : le premier
/// est `/schedule`, le second passe par l'édition et l'adresse d'URL de la
/// séance — c'est ainsi que le site les écrit.
pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/schedule", web::get().to(programmation))
        .route("/events/{event_id}/sessions/{slug}", web::get().to(seance))
        .route("/events/{event_slug}/tracks/{track_slug}", web::get().to(page_du_fil));
}

/// La page publique d'un fil : journée spéciale ou atelier préparatoire.
#[utoipa::path(
    get,
    description = "`PublicTrackPage`, en une réponse et **sans session** : l'édition (fuseau de référence compris), le fil (titres, accroche, dates, couleur, thématiques), son lieu (celui du fil, à défaut le pavillon ; `is_off_site` quand il se tient ailleurs, avec adresse, lien de carte, secteur, consigne et coordonnées), la photo du bandeau (`AttachedImage`), les documents (`AttachedDocument[]`), l'encart mis en avant et son visuel, les publics cibles, les partenaires (logo, rôle) et la mention de soutien, les parties (heures lues sur leur contenu), le **déroulé** fusionnant moments sans activité (`kind: moment`) et activités **publiées** (`kind: activity` : ligne de `PublicScheduleRow`, intervenants, organisations avec logo, temps fort), trié par heure, les heures de la journée (`starts_at`, `ends_at`, `opens_at` pour l'accueil qui précède) et les autres fils publiés de l'édition (`PublicTrackSummary[]`, la page lue marquée). **Un fil non publié, une édition en brouillon ou annulée et une adresse inconnue rendent le même 404.**",
    path = "/events/{event_slug}/tracks/{track_slug}",
    tag = "Programmation publique",
    operation_id = "programmation_page_du_fil",
    params(
        ("event_slug" = String, Path, description = "Adresse d'URL de l'édition"),
        ("track_slug" = String, Path, description = "Adresse d'URL du fil"),
    ),
    responses(
        (status = 200, description = "PublicTrackPage", body = Object),
        (status = 404, description = "Adresse inconnue ou fil non publié, indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn page_du_fil(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    chemin: web::Path<(String, String)>,
) -> Result<HttpResponse> {
    let (edition, fil) = chemin.into_inner();
    let page = crate::service::track_page::page(state.pool(), &edition, &fil).await?;

    crate::routes::json_revalide(&requete, &page, "public, no-cache")
}

/// La programmation d'une édition.
#[utoipa::path(
    get,
    description = "`PublicScheduleRow[]` — `programme.v_public_schedule`, **telle quelle**, et **sans session**. Une ligne = un bloc du calendrier : salle, organisation avec son sigle et son pays, journées spéciales, thématiques avec libellé et couleur, image de couverture — celle de la séance, **à défaut celle du dossier d'origine** —, état temporel calculé en base, nombre d'inscrits. Une édition dont le programme n'est pas paru rend une liste **vide**, jamais une erreur. **`event_id` est facultative** : absente, ce sont les séances `upcoming` et `ongoing` de TOUTES les éditions, dans l'ordre du temps — ce que compose l'accueil, qui n'a pas d'édition à nommer. La lecture est alors plafonnée.\n\nChaque ligne porte aussi les conditions d'inscription (`waitlist_enabled`, `registration_required`, `registration_opens_at`, `registration_closes_at`, `waitlisted_count`), `listing_changed_at`, `language_codes` — **la seule donnée tirée du dossier**, nulle sans dossier — et **une rediffusion au plus** (`replay_url`, `replay_duration_seconds`), seulement disponible. Enfin le logo de l'organisation porteuse (`organization_logo`, même forme que `cover`) et son type (`organization_type_code`, qui fait afficher le drapeau pour `public_national_institution`).\n\n`ETag` sur le corps ; **304** sur `If-None-Match`.",
    path = "/schedule",
    tag = "Programmation publique",
    operation_id = "programmation_publique",
    params(
        ("event_id" = Option<Uuid>, Query, description = "Édition dont on lit le programme. Absente : les séances à venir de toutes les éditions"),
        ("limit" = Option<i64>, Query, description = "Plafond du nombre de lignes (défaut 50, maximum 200)"),
    ),
    responses(
        (status = 200, description = "PublicScheduleRow[]", body = Object),
        (status = 304, description = "Rien n'a changé depuis l'empreinte présentée"),
    )
)]
pub(crate) async fn programmation(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    demande: web::Query<EditionDemandee>,
) -> Result<HttpResponse> {
    // Le plafond ne s'applique qu'à la lecture SANS édition : la programmation
    // d'une édition se rend entière, c'est un calendrier qu'on affiche en bloc.
    let limite = match (demande.event_id, demande.limit) {
        (Some(_), aucune) => aucune,
        (None, choisie) => Some(choisie.unwrap_or(LIMITE_PAR_DEFAUT).clamp(1, LIMITE_MAX)),
    };
    let lignes =
        public_schedule::programmation(state.pool(), demande.event_id.map(EventId), limite).await?;

    crate::routes::json_revalide(&requete, &lignes, "public, no-cache")
}

/// Le détail d'une séance publiée.
#[utoipa::path(
    get,
    description = "`PublicSessionDetail` : `{ session, description, allows_questions, is_recorded, cancelled_reason, speakers, organizations, gallery }` : la séance **publiée** désignée par son adresse d'URL dans son édition (une ligne de `PublicScheduleRow`), sa description longue (texte multilingue, nulle si absente), si elle prend des questions du public et si elle est enregistrée, le motif de son annulation (texte multilingue, nul si elle ne l'est pas), ses intervenants (`PublicSessionSpeaker[]` : nom d'affichage, fonction, organisation, biographie, photo `avatar`, **ni identifiant de personne, ni adresse, ni confirmation, ni présence**), ses organisations (`SessionOrganization[]`, avec `name`, `acronym`, `country_code`, `country`) et sa galerie (`AttachedImage[]`, photos **prêtes et publiques** seulement, dans l'ordre réglé par l'équipe ; tableau vide sans photo). **Une adresse inconnue et une séance non publiée rendent le même 404** : distinguer les deux dirait au public qu'une séance existe sans être encore annoncée.",
    path = "/events/{event_id}/sessions/{slug}",
    tag = "Programmation publique",
    operation_id = "programmation_seance_publique",
    params(
        ("event_id" = Uuid, Path, description = "Édition"),
        ("slug" = String, Path, description = "Adresse d'URL de la séance"),
    ),
    responses(
        (status = 200, description = "Séance publiée, intervenants et organisations", body = Object),
        (status = 404, description = "Adresse inconnue **ou séance non publiée** — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn seance(
    state: web::Data<ProgrammeState>,
    chemin: web::Path<(Uuid, String)>,
) -> Result<HttpResponse> {
    let (event_id, slug) = chemin.into_inner();
    let seance = public_schedule::seance(state.pool(), EventId(event_id), &slug).await?;

    Ok(HttpResponse::Ok().json(seance))
}
