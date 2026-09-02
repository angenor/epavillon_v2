//! Les deux cookies de session, et pourquoi ils n'ont pas la même portée.
//!
//! Le jeton d'accès voyage partout, quinze minutes. Le jeton de rafraîchissement
//! ne va **qu'aux routes de session** : c'est ce qui limite le dégât d'une fuite
//! par une autre route (research.md § R2). Aucun écran ne les lit — ils sont
//! `HttpOnly`, et le site les renvoie seul par `credentials: 'include'`.

use actix_web::cookie::time::Duration as CookieDuration;
use actix_web::cookie::{Cookie, SameSite};
use kernel::config::Config;
use std::time::Duration;
use time::OffsetDateTime;

pub const COOKIE_ACCES: &str = "epavillon_at";
pub const COOKIE_RAFRAICHISSEMENT: &str = "epavillon_rt";

/// Le préfixe `/api` est celui que `.env.example` prescrit pour le raccordement
/// du site : le chemin du cookie est écrit du point de vue du navigateur, pas de
/// celui du routeur.
const CHEMIN_RAFRAICHISSEMENT: &str = "/api/auth";

/// Préfixe les deux chemins par celui sous lequel le site est servi.
///
/// **C'est ce qui rend la cohabitation possible.** Servi sous `/v2`, le
/// navigateur appelle `/v2/api/auth/refresh` ; un cookie posé sur `/api/auth`
/// ne lui serait jamais renvoyé, et la session mourrait au bout du jeton
/// d'accès — quinze minutes, sans message, sans trace, sans erreur en journal.
/// À la racine, `app_base_path` est vide et les chemins ne changent pas.
fn chemin(config: &Config, suffixe: &str) -> String {
    format!("{}{suffixe}", config.app_base_path)
}

fn batir(
    config: &Config,
    nom: &'static str,
    valeur: String,
    chemin: String,
    same_site: SameSite,
    max_age: CookieDuration,
) -> Cookie<'static> {
    let mut cookie = Cookie::build(nom, valeur)
        .http_only(true)
        .secure(config.auth.cookie_secure)
        .same_site(same_site)
        .path(chemin)
        .max_age(max_age)
        .finish();

    if let Some(domaine) = &config.auth.cookie_domain {
        cookie.set_domain(domaine.clone());
    }
    cookie
}

pub fn acces(config: &Config, jeton: String, duree: Duration) -> Cookie<'static> {
    batir(
        config,
        COOKIE_ACCES,
        jeton,
        chemin(config, "/"),
        SameSite::Lax,
        CookieDuration::seconds(duree.as_secs() as i64),
    )
}

pub fn rafraichissement(
    config: &Config,
    jeton: String,
    expires_at: OffsetDateTime,
) -> Cookie<'static> {
    let restant = (expires_at - OffsetDateTime::now_utc())
        .whole_seconds()
        .max(0);
    batir(
        config,
        COOKIE_RAFRAICHISSEMENT,
        jeton,
        chemin(config, CHEMIN_RAFRAICHISSEMENT),
        SameSite::Strict,
        CookieDuration::seconds(restant),
    )
}

/// Un cookie s'efface en le réécrivant vide avec la **même portée** : un chemin
/// différent en laisserait un second, invisible et vivant.
pub fn effacer(config: &Config) -> [Cookie<'static>; 2] {
    [
        batir(
            config,
            COOKIE_ACCES,
            String::new(),
            chemin(config, "/"),
            SameSite::Lax,
            CookieDuration::ZERO,
        ),
        batir(
            config,
            COOKIE_RAFRAICHISSEMENT,
            String::new(),
            chemin(config, CHEMIN_RAFRAICHISSEMENT),
            SameSite::Strict,
            CookieDuration::ZERO,
        ),
    ]
}
