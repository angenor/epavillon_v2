//! La recherche d'un intervenant par son adresse — exacte, ou en cours de frappe.
//!
//! # Ce que ces routes ne font jamais
//!
//! **Elles ne rendent pas l'annuaire.** La clé reste l'adresse :
//! `identity.people.primary_email` est la clé de rapprochement du modèle, et
//! chercher par nom rapprocherait deux homonymes — ce qui est pire qu'un
//! doublon.
//!
//! La suggestion, ajoutée le 16/09 à la demande du commanditaire, complète ce
//! que le déposant **a déjà tapé** : préfixe d'adresse, trois caractères au
//! minimum, huit réponses au plus, et seulement des personnes dont l'adresse est
//! confirmée ou qui ont parlé dans une activité retenue — celles qui acceptent
//! de figurer dans les listes. Sans ces quatre bornes, une autocomplétion est un
//! export de la liste de contacts.
//!
//! Elle vit sous `/people`, préfixe **composé par l'API** : deux `web::scope`
//! du même préfixe ne se complètent pas.

use actix_web::{web, HttpResponse};
use kernel::auth::Actor;
use kernel::error::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::repo::cross;
use crate::state::ProgrammeState;

#[derive(Debug, Deserialize)]
pub struct AdresseCherchee {
    email: String,
}

/// Exactement `PersonLookup`.
#[derive(Debug, Serialize)]
pub struct FicheTrouvee {
    person_id: Uuid,
    civility: Option<String>,
    first_name: String,
    last_name: String,
    email: String,
    /// Fonction et organisation **du profil** : elles amorcent les instantanés
    /// de l'activité, qui restent modifiables.
    job_title: Option<String>,
    organization_name: Option<String>,
    organization_id: Option<Uuid>,
    bio: Option<String>,
    /// **La personne a-t-elle un compte ?** Si oui, son identité lui appartient
    /// et le formulaire verrouille les champs correspondants.
    has_account: bool,
}

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/lookup", web::get().to(chercher))
        .route("/suggest", web::get().to(suggerer));
}

/// En deçà, on ne suggère rien : deux lettres rendraient la moitié des adresses.
const PREFIXE_MINIMAL: usize = 3;
/// Au-delà, la liste cesse d'aider et commence à dénombrer.
const SUGGESTIONS_MAX: i64 = 8;

#[derive(Debug, Deserialize)]
pub struct DebutDadresse {
    q: String,
}

/// La personne qui porte cette adresse, si la plateforme la connaît.
#[utoipa::path(
    get,
    description = "`PersonLookup`, ou `null`. **La clé est l'adresse, et rien d'autre** — aucun appel de ce module ne rend l'annuaire. Même intention que la recherche d'organisations similaires : ne pas créer une seconde fiche pour quelqu'un qui existe déjà, ce qui est le défaut n° 1 de la v1 transposé de l'organisation à l'intervenant, et bien moins visible. `has_account` commande le verrouillage d'identité côté formulaire.",
    path = "/people/lookup",
    tag = "Dépôt",
    operation_id = "depot_chercher_intervenant",
    params(("email" = String, Query, description = "Adresse électronique exacte")),
    responses(
        (status = 200, description = "PersonLookup ou null", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn chercher(
    state: web::Data<ProgrammeState>,
    _acteur: Actor,
    demande: web::Query<AdresseCherchee>,
) -> Result<HttpResponse> {
    let email = demande.email.trim().to_lowercase();
    if email.is_empty() {
        return Ok(HttpResponse::Ok().json(serde_json::Value::Null));
    }

    let fiche = cross::fiche_personne_par_email(state.pool(), &email).await?;
    let organisation = match fiche.as_ref().and_then(|f| f.primary_organization_id) {
        None => None,
        Some(id) => cross::fiche_organisation(state.pool(), id).await?,
    };

    let reponse = fiche.map(|f| FicheTrouvee {
        person_id: f.id,
        civility: f.civility,
        first_name: f.first_name,
        last_name: f.last_name,
        email: f.email,
        job_title: f.job_title,
        organization_name: organisation.as_ref().map(|o| o.legal_name.clone()),
        organization_id: f.primary_organization_id,
        bio: f
            .biography
            .as_ref()
            .map(crate::domain::draft::fr)
            .filter(|b| !b.is_empty()),
        has_account: f.has_account,
    });

    Ok(HttpResponse::Ok().json(reponse))
}

/// Les intervenants dont l'adresse commence par ce qui est tapé.
#[utoipa::path(
    get,
    description = "`PersonLookup[]`. **Une autocomplétion, pas un annuaire** : préfixe d'adresse de trois caractères au minimum, huit réponses au plus, et deux populations seulement — adresse confirmée, ou intervenant d'une activité RETENUE. Le choix de la personne de figurer dans les listes (`is_directory_visible`) est respecté. Rend une liste vide plutôt qu'une erreur : ne rien trouver n'empêche jamais de saisir l'intervenant à la main.",
    path = "/people/suggest",
    tag = "Dépôt",
    operation_id = "depot_suggerer_intervenants",
    params(("q" = String, Query, description = "Début de l'adresse électronique")),
    responses(
        (status = 200, description = "PersonLookup[]", body = Object),
        (status = 401, description = "Aucune session, ou session close", body = crate::routes::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn suggerer(
    state: web::Data<ProgrammeState>,
    _acteur: Actor,
    demande: web::Query<DebutDadresse>,
) -> Result<HttpResponse> {
    let prefixe = demande.q.trim().to_lowercase();
    if prefixe.chars().count() < PREFIXE_MINIMAL {
        return Ok(HttpResponse::Ok().json(Vec::<FicheTrouvee>::new()));
    }

    let fiches = cross::suggestions_dintervenants(state.pool(), &prefixe, SUGGESTIONS_MAX).await?;

    // Les organisations sont résolues en UN appel : une par personne ferait huit
    // requêtes pour une frappe.
    let organisations = cross::fiches_organisations(
        state.pool(),
        &fiches
            .iter()
            .filter_map(|f| f.primary_organization_id)
            .collect::<Vec<_>>(),
    )
    .await?;

    let reponse: Vec<FicheTrouvee> = fiches
        .into_iter()
        .map(|f| {
            let organisation = f
                .primary_organization_id
                .and_then(|id| organisations.iter().find(|o| o.id == id));
            FicheTrouvee {
                person_id: f.id,
                civility: f.civility,
                first_name: f.first_name,
                last_name: f.last_name,
                email: f.email,
                job_title: f.job_title,
                organization_name: organisation.map(|o| o.legal_name.clone()),
                organization_id: f.primary_organization_id,
                bio: f
                    .biography
                    .as_ref()
                    .map(crate::domain::draft::fr)
                    .filter(|b| !b.is_empty()),
                has_account: f.has_account,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(reponse))
}
