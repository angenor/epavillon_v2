//! Les drapeaux de fonctionnalité, **résolus pour qui demande**.
//!
//! LE SITE NE REFAIT PAS LE CALCUL. Un drapeau ne dit pas seulement « ouvert » ou
//! « fermé » : il porte un pourcentage de déploiement et une liste de personnes
//! explicitement ouvertes, et c'est `platform.is_feature_enabled()` qui tranche.
//! Rendre la table brute obligerait le site à réimplémenter cette fonction — donc
//! à en donner une seconde version, qui divergerait au premier ajustement — et à
//! publier au passage les identifiants des personnes du déploiement progressif.
//!
//! **Aucune session n'est exigée**, et c'est indispensable : le routage du site
//! lit ces drapeaux à la première navigation, avant même de savoir qui regarde.
//! Sans session, seul un déploiement à 100 % ouvre — la valeur sûre.
//!
//! LA DESCRIPTION N'EST PAS RENDUE. Elle est écrite pour l'exploitant, en clair,
//! et nomme parfois ce qui n'est pas encore annoncé.

use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::error::Result;
use serde::Serialize;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.route("/platform/feature-flags", web::get().to(drapeaux));
}

#[derive(Debug, Serialize)]
pub struct ResolvedFeatureFlag {
    pub key: String,
    /// Le verdict pour l'appelant, déjà calculé par `platform.is_feature_enabled()`.
    pub is_enabled: bool,
}

#[utoipa::path(
    get,
    description = "`ResolvedFeatureFlag[]` — chaque drapeau et son verdict POUR L'APPELANT. Le déploiement progressif est tranché par `platform.is_feature_enabled()`, jamais par le site : une seconde implémentation du calcul divergerait, et rendre `enabled_for` publierait les identifiants des personnes visées. Sans session, seul un déploiement à 100 % ouvre.",
    path = "/platform/feature-flags",
    tag = "Plateforme",
    operation_id = "platform_drapeaux",
    responses((status = 200, description = "ResolvedFeatureFlag[]", body = Object))
)]
pub(crate) async fn drapeaux(db: web::Data<Db>, requete: HttpRequest) -> Result<HttpResponse> {
    // Hors session, `actor_id` est nul : `is_feature_enabled()` n'ouvre alors
    // qu'un drapeau déployé à 100 %, ce qui est la valeur sûre.
    let personne = requete
        .extensions()
        .get::<RequestContext>()
        .and_then(|ctx| ctx.actor_id);

    let lignes = sqlx::query_as!(
        ResolvedFeatureFlag,
        r#"SELECT key AS "key!",
                  platform.is_feature_enabled(key, $1) AS "is_enabled!"
             FROM platform.feature_flags
            ORDER BY key"#,
        personne
    )
    .fetch_all(db.pool())
    .await?;

    Ok(HttpResponse::Ok().json(lignes))
}
