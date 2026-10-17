//! Zoom (17/10) : l'API, le Meeting SDK, les notifications et la clé qui chiffre
//! les secrets de réunion en base.

use super::{invalid, ConfigError, Raw, Secret};

/// Les deux applications Zoom : « Server-to-Server OAuth » pour l'API (créer
/// les réunions, y inscrire les personnes) et « General » pour le Meeting SDK
/// (signer l'entrée d'une personne dans la page de l'activité).
#[derive(Debug, Clone)]
pub struct ZoomConfig {
    pub account_id: String,
    pub client_id: String,
    pub client_secret: Secret,
    /// Identifiant ou adresse du compte Zoom qui héberge les réunions ; `me` par défaut.
    pub host_user: String,
    pub sdk_key: String,
    pub sdk_secret: Secret,
    /// Absent : la route des notifications Zoom n'est pas montée.
    pub webhook_secret_token: Option<Secret>,
    /// 32 octets en base64 : chiffre le code d'accès et le lien hôte en base.
    pub secrets_key: Secret,
    /// Injectables pour les tests, jamais lus de l'environnement.
    pub api_base_url: String,
    pub oauth_url: String,
}

pub const ZOOM_API_BASE_URL: &str = "https://api.zoom.us/v2";
pub const ZOOM_OAUTH_URL: &str = "https://zoom.us/oauth/token";

pub(super) fn depuis(raw: &Raw) -> Result<Option<ZoomConfig>, ConfigError> {
    use base64::Engine as _;

    let obligatoires = [
        ("ZOOM_ACCOUNT_ID", raw.zoom_account_id.trim()),
        ("ZOOM_CLIENT_ID", raw.zoom_client_id.trim()),
        ("ZOOM_CLIENT_SECRET", raw.zoom_client_secret.trim()),
        ("ZOOM_SDK_KEY", raw.zoom_sdk_key.trim()),
        ("ZOOM_SDK_SECRET", raw.zoom_sdk_secret.trim()),
        ("LIVE_SECRETS_KEY", raw.live_secrets_key.trim()),
    ];
    let manquants: Vec<&str> = obligatoires
        .iter()
        .filter(|(_, v)| v.is_empty())
        .map(|(k, _)| *k)
        .collect();
    let webhook = raw.zoom_webhook_secret_token.trim();

    if manquants.len() == obligatoires.len() {
        if !webhook.is_empty() {
            return Err(invalid(
                "ZOOM_WEBHOOK_SECRET_TOKEN est renseigné sans les identifiants Zoom : les notifications ne serviraient à rien.",
            ));
        }
        return Ok(None);
    }
    if !manquants.is_empty() {
        return Err(invalid(format!(
            "Configuration Zoom incomplète, manquent : {}. Tout renseigner, ou tout laisser vide pour éteindre la visioconférence.",
            manquants.join(", ")
        )));
    }

    let cle = base64::engine::general_purpose::STANDARD
        .decode(raw.live_secrets_key.trim())
        .map_err(|_| invalid("LIVE_SECRETS_KEY n'est pas du base64 (openssl rand -base64 32)."))?;
    if cle.len() != 32 {
        return Err(invalid(format!(
            "LIVE_SECRETS_KEY doit coder 32 octets, il en code {} (openssl rand -base64 32).",
            cle.len()
        )));
    }

    let hote = raw.zoom_host_user.trim();
    Ok(Some(ZoomConfig {
        account_id: raw.zoom_account_id.trim().to_owned(),
        client_id: raw.zoom_client_id.trim().to_owned(),
        client_secret: Secret(raw.zoom_client_secret.trim().to_owned()),
        host_user: if hote.is_empty() {
            "me".to_owned()
        } else {
            hote.to_owned()
        },
        sdk_key: raw.zoom_sdk_key.trim().to_owned(),
        sdk_secret: Secret(raw.zoom_sdk_secret.trim().to_owned()),
        webhook_secret_token: Some(webhook)
            .filter(|w| !w.is_empty())
            .map(|w| Secret(w.to_owned())),
        secrets_key: Secret(raw.live_secrets_key.trim().to_owned()),
        api_base_url: ZOOM_API_BASE_URL.to_owned(),
        oauth_url: ZOOM_OAUTH_URL.to_owned(),
    }))
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use figment::providers::Serialized;
    use figment::Figment;
    use serde_json::{json, Value};

    fn config(zoom: Value) -> Result<Config, crate::config::ConfigError> {
        let mut valeurs = json!({
            "database_url": "postgres://x@localhost/x",
            "auth_signing_key": "ZXBhdmlsbG9uLWNsZS1kZS10ZXN0LWVkMjU1MTktYjE",
            "mail_relay_url": "http://localhost:3000/api/internal/mail",
            "mail_relay_token": "jeton",
            "s3_access_key_id": "",
            "media_storage": "filesystem",
            "media_fs_root": "/tmp/epavillon-test",
        });
        for (cle, valeur) in zoom.as_object().unwrap() {
            valeurs[cle] = valeur.clone();
        }
        Config::from_figment(Figment::from(Serialized::defaults(valeurs)))
    }

    fn complete() -> Value {
        json!({
            "zoom_account_id": "a", "zoom_client_id": "b", "zoom_client_secret": "c",
            "zoom_sdk_key": "d", "zoom_sdk_secret": "e",
            "live_secrets_key": "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=",
        })
    }

    #[test]
    fn tout_vide_eteint_zoom() {
        assert!(config(json!({})).unwrap().zoom.is_none());
    }

    #[test]
    fn complete_allume_zoom_avec_hote_par_defaut() {
        let zoom = config(complete()).unwrap().zoom.unwrap();
        assert_eq!(zoom.host_user, "me");
        assert!(zoom.webhook_secret_token.is_none());
    }

    #[test]
    fn a_moitie_rempli_refuse_le_demarrage() {
        let erreur = config(json!({ "zoom_account_id": "a" })).unwrap_err();
        assert!(erreur.to_string().contains("ZOOM_CLIENT_SECRET"));
        let mut courte = complete();
        courte["live_secrets_key"] = json!("Y291cnQ=");
        assert!(config(courte)
            .unwrap_err()
            .to_string()
            .contains("32 octets"));
        assert!(config(json!({ "zoom_webhook_secret_token": "w" })).is_err());
    }
}
