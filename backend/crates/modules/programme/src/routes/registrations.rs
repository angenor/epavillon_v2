//! Le scope `/registrations`, et les deux chemins d'inscription servis sous
//! `/sessions`.
//!
//! # Trois régimes d'autorisation, et ils ne se confondent pas
//!
//! - **S'inscrire** : la session, **ou aucune** si le formulaire admet l'anonyme
//!   — le formulaire décide, jamais la route.
//! - **La liste nominative** : `programme.registration.manage` **sur l'édition de
//!   la séance**. Le rôle de programmation ne la détient pas (écart n° 119).
//! - **Annuler** : l'inscrit lui-même, **ou** la permission de gérer les
//!   inscriptions. Rejoindre : l'inscrit seul.
//!
//! # Chemins littéraux avant chemins paramétrés
//!
//! `/registrations/mine` est déclaré avant `/registrations/{id}/…`.

use actix_web::{web, HttpRequest, HttpResponse};
use kernel::auth::{Actor, Perimeter, Scope};
use kernel::error::{ApiError, Result};
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::ids::{RegistrationId, SessionId};
use crate::domain::permissions::REGISTRATION_MANAGE;
use crate::routes::{contexte_de, locale_de};
use crate::service::perimeter::{self, Cible};
use crate::service::registration::{self, AnnulationDemandee, SessionRegisterPayload};
use crate::state::ProgrammeState;

#[derive(Debug, Deserialize)]
pub struct SeanceDemandee {
    session_id: Uuid,
}

/// `/registrations` — la liste nominative et « mes inscriptions ».
pub fn chemins_litteraux(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(liste_nominative))
        .route("/mine", web::get().to(les_miennes));
}

/// `/registrations/{id}/…` — les deux gestes de l'inscrit.
pub fn chemins_dinscription(cfg: &mut web::ServiceConfig) {
    cfg.route("/{id}/cancel", web::post().to(annuler))
        .route("/{id}/join", web::post().to(rejoindre));
}

// -----------------------------------------------------------------------------
// Sous `/sessions` — le formulaire et l'inscription
// -----------------------------------------------------------------------------

/// Le formulaire applicable à une séance.
#[utoipa::path(
    get,
    description = "`{ form, fields }` — le formulaire **applicable** : celui de la séance, à défaut celui de son édition, à défaut celui de la plateforme. **Lecture publique** : l'écran d'inscription s'ouvre avant qu'on se connecte. Seuls les champs **actifs** sont rendus, dans leur ordre d'affichage, et les options d'un champ adossé à une taxonomie sont **résolues avec leur libellé traduit** — n'exposer que les codes forcerait l'écran à recharger la taxonomie, et c'est ainsi que les libellés se sont retrouvés figés dans le frontend de la v1.",
    path = "/sessions/{id}/registration-form",
    tag = "Inscriptions",
    operation_id = "inscriptions_formulaire",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    responses(
        (status = 200, description = "Formulaire applicable et champs actifs", body = Object),
        (status = 404, description = "Séance inexistante", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn formulaire(
    state: web::Data<ProgrammeState>,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let formulaire = registration::formulaire(&state, SessionId(id.into_inner())).await?;
    Ok(HttpResponse::Ok().json(formulaire))
}

/// S'inscrire à une séance.
#[utoipa::path(
    post,
    description = "`SessionRegisterPayload` → `RegistrationResult`. **Six issues, toutes en 200** : inscrit, placé en liste d'attente avec sa position, déjà inscrit, complet avec le nombre de places, clos avec son échéance, pas encore ouvert avec sa date. Ce sont des issues normales d'une tentative bien formée — une personne peut arriver une minute après la clôture. Les réponses sont validées contre le formulaire **résolu**, avant toute écriture, et une clé inconnue est **refusée** plutôt qu'ignorée. Une réponse à un champ marqué sensible exige un consentement, dont la preuve est écrite dans la même transaction. **Sans session**, l'inscription n'aboutit que si le formulaire admet l'anonyme, et l'identité vient de champs dédiés — jamais des réponses.",
    path = "/sessions/{id}/registrations",
    tag = "Inscriptions",
    operation_id = "inscriptions_sinscrire",
    params(("id" = Uuid, Path, description = "Identifiant de la séance")),
    request_body = SessionRegisterPayload,
    responses(
        (status = 200, description = "RegistrationResult", body = Object),
        (status = 401, description = "Le formulaire n'admet pas l'inscription sans compte", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Réponse invalide, consentement manquant, ou séance ne prenant pas d'inscription", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn sinscrire(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    id: web::Path<Uuid>,
    corps: web::Json<SessionRegisterPayload>,
) -> Result<HttpResponse> {
    let seance = SessionId(id.into_inner());
    // La session est **facultative** : c'est le formulaire qui décide, et
    // exiger un extracteur de session ici refuserait avant lui.
    let lecteur = session_facultative(&requete);
    let ctx = match lecteur {
        Some(personne) => contexte_de(&requete, personne),
        None => kernel::context::RequestContext::new(
            kernel::context::RequestContext::generated_request_id(),
            locale_de(&requete),
        ),
    };

    let issue = registration::sinscrire(
        &state,
        &ctx,
        seance,
        lecteur,
        adresse_dappel(&requete),
        corps.into_inner(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(issue))
}

/// La personne connectée, **quand il y en a une**.
fn session_facultative(requete: &HttpRequest) -> Option<Uuid> {
    use actix_web::HttpMessage;

    requete
        .extensions()
        .get::<kernel::context::RequestContext>()
        .and_then(|ctx| ctx.actor_id)
}

/// L'adresse d'appel, telle que l'intergiciel l'a résolue : elle accompagne la
/// preuve de consentement, et rien d'autre.
fn adresse_dappel(requete: &HttpRequest) -> Option<std::net::IpAddr> {
    requete.peer_addr().map(|adresse| adresse.ip())
}

// -----------------------------------------------------------------------------
// Sous `/registrations`
// -----------------------------------------------------------------------------

/// La liste **nominative** des inscrits d'une séance.
#[utoipa::path(
    get,
    description = "`RegistrationRow[]` — la liste **nominative**, avec la personne et son organisation. Elle exige `programme.registration.manage` **sur l'édition de la séance** : le rôle de programmation ne la détient pas, et une chargée de programmation compose donc la grille sans pouvoir ouvrir cette liste. Ce n'est pas une fatalité du code — c'est une ligne de la table des droits, modifiable au back-office.",
    path = "/registrations",
    tag = "Inscriptions",
    operation_id = "inscriptions_liste_nominative",
    params(("session_id" = Uuid, Query, description = "Séance dont on liste les inscrits")),
    responses(
        (status = 200, description = "RegistrationRow[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 403, description = "Permission de gérer les inscriptions absente", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Séance inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn liste_nominative(
    state: web::Data<ProgrammeState>,
    perimetre: Perimeter,
    demande: web::Query<SeanceDemandee>,
) -> Result<HttpResponse> {
    let seance = SessionId(demande.session_id);
    exiger_la_gestion(&state, &perimetre, Cible::Seance(seance)).await?;

    Ok(HttpResponse::Ok().json(registration::liste_nominative(&state, seance).await?))
}

/// « Mes inscriptions », annulations comprises.
#[utoipa::path(
    get,
    description = "`Registration[]` — ce à quoi la personne **connectée** est inscrite, annulations comprises. L'identifiant de personne que le front envoie encore est **ignoré** : l'API lit sa propre session.",
    path = "/registrations/mine",
    tag = "Inscriptions",
    operation_id = "inscriptions_les_miennes",
    responses(
        (status = 200, description = "Registration[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn les_miennes(
    state: web::Data<ProgrammeState>,
    acteur: Actor,
) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(registration::mes_inscriptions(&state, acteur.0).await?))
}

/// Annuler son inscription.
#[utoipa::path(
    post,
    description = "`{ reason? }` → `CancelRegistrationResult`. L'annulation **promeut exactement le nombre de places libérées** — zéro ou une —, dans la même transaction et sous le même verrou : le contrôle de capacité de la base ne porte que sur l'insertion, et promouvoir davantage ferait dépasser la jauge sans un mot. Annuler une inscription **en attente** ne promeut personne : elle n'occupait aucune place. Elle est ouverte à **l'inscrit lui-même** ou à qui gère les inscriptions de l'édition.",
    path = "/registrations/{id}/cancel",
    tag = "Inscriptions",
    operation_id = "inscriptions_annuler",
    params(("id" = Uuid, Path, description = "Identifiant de l'inscription")),
    request_body = AnnulationDemandee,
    responses(
        (status = 200, description = "CancelRegistrationResult", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Inscription inexistante **ou hors périmètre**", body = crate::routes::openapi::ApiErrorBody),
        (status = 422, description = "Le déclencheur refuse la modification — séance annulée, ou question devenue obligatoire", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn annuler(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    id: web::Path<Uuid>,
    corps: Option<web::Json<AnnulationDemandee>>,
) -> Result<HttpResponse> {
    let inscription = RegistrationId(id.into_inner());
    let etat = crate::repo::registrations::etat(state.pool(), inscription)
        .await?
        .ok_or_else(ApiError::not_found)?;

    // L'inscrit lui-même, **ou** qui gère les inscriptions de l'édition. Le
    // refus est le même dans les deux échecs : une URL forgée ne dit pas si
    // l'inscription existe.
    if etat.person_id != acteur.0 {
        exiger_la_gestion(
            &state,
            &Perimeter {
                person_id: acteur.0,
                scope: kernel::auth::administered_events(state.pool(), acteur.0).await?,
            },
            Cible::Inscription(inscription),
        )
        .await?;
    }

    let ctx = contexte_de(&requete, acteur.0);
    let motif = corps.and_then(|c| c.into_inner().reason);
    let resultat = registration::annuler(
        &state,
        &ctx,
        inscription,
        SessionId::from(etat.session_id),
        motif.as_deref(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(resultat))
}

/// Rejoindre — la **première** présence.
#[utoipa::path(
    post,
    description = "`{ joined_at }` — la **première** présence, écrite une seule fois par la fonction du modèle : un second clic ne l'écrase pas, et c'est ce qui donne un taux de participation réel. Réservée à **l'inscrit lui-même**.",
    path = "/registrations/{id}/join",
    tag = "Inscriptions",
    operation_id = "inscriptions_rejoindre",
    params(("id" = Uuid, Path, description = "Identifiant de l'inscription")),
    responses(
        (status = 200, description = "{ joined_at }", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
        (status = 404, description = "Inscription inexistante, ou celle de quelqu'un d'autre — indiscernables", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn rejoindre(
    requete: HttpRequest,
    state: web::Data<ProgrammeState>,
    acteur: Actor,
    id: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let inscription = RegistrationId(id.into_inner());
    let etat = crate::repo::registrations::etat(state.pool(), inscription)
        .await?
        .ok_or_else(ApiError::not_found)?;

    // **Le même refus qu'une inscription inexistante** : rejoindre à la place
    // d'un autre ne doit pas dire que son inscription existe.
    if etat.person_id != acteur.0 {
        return Err(ApiError::not_found());
    }

    let ctx = contexte_de(&requete, acteur.0);
    Ok(HttpResponse::Ok().json(registration::rejoindre(&state, &ctx, inscription).await?))
}

/// **Ascendance, périmètre, puis permission de gérer les inscriptions.**
async fn exiger_la_gestion(
    state: &ProgrammeState,
    perimetre: &Perimeter,
    cible: Cible,
) -> Result<()> {
    let event_id = perimeter::edition_dans_le_perimetre(state.pool(), perimetre, cible).await?;

    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        REGISTRATION_MANAGE,
        Scope::Event(event_id.as_uuid()),
    )
    .await
}
