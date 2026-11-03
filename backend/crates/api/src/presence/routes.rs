use actix_web::http::header;
use actix_web::{web, HttpResponse};
use analytics::authz::PresenceRead;
use kernel::auth::{Actor, Requires};
use kernel::error::{ApiError, Result};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::store::{Presence, Zone};

const ONGLET_MIN: usize = 8;
const ONGLET_MAX: usize = 64;

#[derive(Debug, Deserialize)]
pub struct PresenceBeat {
    pub tab: String,
    pub zone: String,
}

#[derive(Debug, Serialize)]
pub struct PresenceAck {
    /// Compté comme connecté. Le site, qui se sait connecté, tourne alors son
    /// jeton d'accès s'il lit faux : passé un quart d'heure sans appel, le
    /// cookie d'accès a expiré et l'onglet serait compté comme visiteur.
    pub signed_in: bool,
}

fn onglet_valide(tab: &str) -> bool {
    (ONGLET_MIN..=ONGLET_MAX).contains(&tab.len())
        && tab
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[utoipa::path(
    post,
    description = "`PresenceBeat` : `{ tab, zone }`, envoyé par chaque onglet visible toutes les 30 s et à chaque changement de zone. `tab` : 8 à 64 caractères parmi lettres, chiffres, `-` et `_`, tiré au hasard par l'onglet. `zone` : `live`, `programme`, `day`, `activity`, `organization`, `admin`, `negotiation` ou `other` (tout autre code compte comme `other`). Connecté ou visiteur se lit de la session, jamais du corps. Rend `PresenceAck`. Ouvert à tous, sans session ; rien n'est gardé au-delà d'une minute de silence.",
    path = "/presence",
    tag = "Présence",
    operation_id = "presence_signaler",
    request_body(content = Object, description = "PresenceBeat"),
    responses(
        (status = 200, description = "PresenceAck", body = Object),
        (status = 422, description = "Identifiant d'onglet invalide", body = crate::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn signaler(
    personne: Option<Actor>,
    corps: web::Json<PresenceBeat>,
    presence: web::Data<Presence>,
) -> Result<HttpResponse> {
    if !onglet_valide(&corps.tab) {
        return Err(ApiError::validation(
            "Identifiant d'onglet invalide.",
            "tab",
        ));
    }
    let connecte = personne.is_some();
    let maintenant = OffsetDateTime::now_utc().unix_timestamp();
    // Magasin plein : l'onglet n'est pas compté, et rien ne le dit au client.
    presence.signaler(&corps.tab, Zone::lire(&corps.zone), connecte, maintenant);
    Ok(HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .json(PresenceAck {
            signed_in: connecte,
        }))
}

#[utoipa::path(
    get,
    description = "`AdminPresence` : onglets visibles qui se sont signalés dans la dernière minute (`total`, dont `signed_in` et `visitors`), par zone (effectif décroissant), une entrée par minute sur la dernière heure (`last_hour`, la dernière en cours) et son pic (`peak`, nul sans personne). Compté en mémoire par cette instance de l'API : la dernière heure repart de zéro à son redémarrage.",
    path = "/admin/presence",
    tag = "Présence",
    operation_id = "admin_presence",
    responses(
        (status = 200, description = "AdminPresence", body = Object),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "`analytics.presence.read` absente en portée globale", body = crate::openapi::ApiErrorBody),
    ),
    security(("session" = []))
)]
pub(crate) async fn lire(
    _droit: Requires<PresenceRead>,
    presence: web::Data<Presence>,
) -> Result<HttpResponse> {
    let releve = presence.releve(OffsetDateTime::now_utc().unix_timestamp());
    Ok(HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .json(releve))
}

#[cfg(test)]
mod tests {
    use super::onglet_valide;

    #[test]
    fn onglet_borne_et_sur() {
        assert!(onglet_valide("a1B2-c3_d4"));
        assert!(onglet_valide(&"x".repeat(64)));
        assert!(!onglet_valide("court"));
        assert!(!onglet_valide(&"x".repeat(65)));
        assert!(!onglet_valide("onglet avec espace"));
        assert!(!onglet_valide("onglet<script>"));
        assert!(!onglet_valide("éàçèùéàç"));
    }
}
