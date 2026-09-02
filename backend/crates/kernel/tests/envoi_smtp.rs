//! L'envoi SMTP direct part vraiment, et la configuration refuse ce qui
//! traverserait le réseau en clair.
//!
//! Le premier test frappe **Mailpit**, du même `docker compose` que la base :
//! il n'y a pas d'autre façon de prouver qu'un message est parti que d'aller le
//! relire chez celui qui l'a reçu. Un faux transport ne prouverait que le code
//! qu'on vient d'écrire.

use kernel::config::{Config, Secret, SmtpConfig, SmtpEncryption};
use kernel::mail::{Mailer, OutgoingMail, SmtpMailer};
use uuid::Uuid;

fn env_ou(cle: &str, defaut: &str) -> String {
    std::env::var(cle)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| defaut.to_owned())
}

fn vers_mailpit() -> SmtpConfig {
    SmtpConfig {
        host: env_ou("SMTP_HOST", "localhost"),
        port: env_ou("SMTP_PORT", "1025").parse().expect("SMTP_PORT"),
        // Mailpit n'écoute ni en TLS ni avec authentification. C'est le seul
        // endroit du dépôt où ce mode est légitime.
        encryption: SmtpEncryption::None,
        username: String::new(),
        password: Secret::from(String::new()),
        from: "ne-pas-repondre@epavillon.local".parse().expect("adresse"),
    }
}

/// Cherche chez Mailpit ce qui vient d'être envoyé. L'adresse du destinataire
/// est unique par exécution : deux tests en parallèle ne se relisent pas.
async fn recu_par(adresse: &str) -> serde_json::Value {
    let ui = env_ou("MAILPIT_UI_PORT", "8025");
    let reponse = reqwest::Client::new()
        .get(format!("http://127.0.0.1:{ui}/api/v1/search"))
        .query(&[("query", format!("to:{adresse}"))])
        .send()
        .await
        .expect("Mailpit injoignable : `make up` a-t-il été lancé ?")
        .json::<serde_json::Value>()
        .await
        .expect("réponse de Mailpit");

    reponse["messages"]
        .as_array()
        .and_then(|m| m.first())
        .cloned()
        .unwrap_or_else(|| panic!("aucun message reçu pour {adresse}"))
}

#[tokio::test]
async fn un_message_arrive_chez_le_destinataire() {
    let adresse = format!("{}@epavillon.local", Uuid::now_v7());
    let expediteur = SmtpMailer::new(&vers_mailpit()).expect("expéditeur");

    expediteur
        .send(&OutgoingMail {
            message_id: Uuid::now_v7().to_string(),
            to: adresse.clone(),
            locale: "fr".to_owned(),
            subject: "Vérification de votre adresse".to_owned(),
            text: "Bonjour, confirmez votre adresse.".to_owned(),
            html: Some("<p>Bonjour, confirmez votre adresse.</p>".to_owned()),
        })
        .await
        .expect("remise");

    let message = recu_par(&adresse).await;
    assert_eq!(message["Subject"], "Vérification de votre adresse");
    assert_eq!(message["To"][0]["Address"], adresse);
    assert_eq!(
        message["From"]["Address"],
        "ne-pas-repondre@epavillon.local"
    );
}

/// Une adresse que le module n'a pas su composer ne doit pas faire tomber le
/// worker, ni être rejouée toutes les heures : elle est refusée définitivement.
#[tokio::test]
async fn une_adresse_inexploitable_est_un_refus_definitif() {
    let expediteur = SmtpMailer::new(&vers_mailpit()).expect("expéditeur");

    let echec = expediteur
        .send(&OutgoingMail {
            message_id: Uuid::now_v7().to_string(),
            to: "pas une adresse".to_owned(),
            locale: "fr".to_owned(),
            subject: "Sujet".to_owned(),
            text: "Corps".to_owned(),
            html: None,
        })
        .await
        .expect_err("une adresse mal formée ne peut pas partir");

    assert!(
        matches!(echec, kernel::mail::MailError::Rejected { statut: 550, .. }),
        "attendu un refus définitif, obtenu {echec:?}"
    );
}

/// **Le refus qui compte le plus.** Sans lui, une variable oubliée au
/// déploiement ferait traverser l'Internet au mot de passe de la boîte, en
/// clair, à chaque courriel — et rien ne le signalerait.
#[test]
fn un_identifiant_sans_chiffrement_est_refuse_au_demarrage() {
    let echec = Config::from_figment(base_valide().merge(
        figment::providers::Serialized::defaults(serde_json::json!({
            "mail_transport": "smtp",
            "smtp_host": "mail.exemple.org",
            "smtp_from": "ne-pas-repondre@exemple.org",
            "smtp_encryption": "none",
            "smtp_username": "ne-pas-repondre@exemple.org",
            "smtp_password": "secret",
        })),
    ))
    .expect_err("la configuration doit être refusée");

    assert!(
        echec.to_string().contains("en clair"),
        "le refus doit dire pourquoi : {echec}"
    );
}

#[test]
fn le_transport_smtp_exige_un_serveur() {
    let echec = Config::from_figment(base_valide().merge(
        figment::providers::Serialized::defaults(serde_json::json!({ "mail_transport": "smtp" })),
    ))
    .expect_err("la configuration doit être refusée");

    assert!(
        echec.to_string().contains("SMTP_HOST"),
        "le refus doit nommer la clé manquante : {echec}"
    );
}

/// Le minimum qu'une configuration doit porter pour que le refus attendu soit
/// bien celui du courriel, et non celui d'une autre clé absente.
///
/// Les clés sont **en minuscules** : `Raw` reflète les noms de variables
/// abaissés par `Env::raw()`, ce qu'un provider `Serialized` ne fait pas.
fn base_valide() -> figment::Figment {
    figment::Figment::from(figment::providers::Serialized::defaults(
        serde_json::json!({
            "database_url": "postgres://postgres:dev@localhost:5432/epavillon",
            "auth_signing_key": "clef-de-test-assez-longue-pour-passer",
            "app_public_url": "http://localhost:3000",
        }),
    ))
}
