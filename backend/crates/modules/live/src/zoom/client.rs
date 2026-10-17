//! Le client de l'API Zoom, application « Server-to-Server OAuth ».
//!
//! Le jeton d'accès vit une heure ; il est gardé en mémoire et renouvelé une
//! minute avant son terme. Une erreur ne porte jamais d'URL (`without_url`) ni
//! de jeton : elle part dans `platform.jobs.last_error`, qu'on relit.

use kernel::config::ZoomConfig;
use kernel::error::{ApiError, ErrorCode, Result};
use reqwest::{Method, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const DELAI: Duration = Duration::from_secs(20);
const DELAI_CONNEXION: Duration = Duration::from_secs(5);
const MARGE_JETON: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub enum ZoomError {
    /// Réseau ou panne de Zoom (5xx) : un nouvel essai a un sens.
    Passagere(String),
    /// Quota (429). `reessayer_dans` : ce que dit `Retry-After`, en secondes.
    Quota { reessayer_dans: Option<u64> },
    /// Refus de Zoom (4xx) : le même appel échouera de la même façon.
    Refusee {
        statut: u16,
        code: Option<i64>,
        message: String,
    },
}

impl ZoomError {
    pub fn est_passagere(&self) -> bool {
        matches!(self, Self::Passagere(_) | Self::Quota { .. })
    }

    pub fn reessayer_dans(&self) -> Option<u64> {
        match self {
            Self::Quota { reessayer_dans } => *reessayer_dans,
            _ => None,
        }
    }
}

impl std::fmt::Display for ZoomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passagere(m) => write!(f, "Zoom momentanément injoignable : {m}"),
            Self::Quota {
                reessayer_dans: Some(s),
            } => write!(f, "quota Zoom atteint, nouvel essai dans {s} s"),
            Self::Quota { .. } => write!(f, "quota Zoom atteint"),
            Self::Refusee {
                statut,
                code,
                message,
            } => match code {
                Some(c) => write!(f, "Zoom refuse ({statut}, code {c}) : {message}"),
                None => write!(f, "Zoom refuse ({statut}) : {message}"),
            },
        }
    }
}

impl From<ZoomError> for ApiError {
    fn from(e: ZoomError) -> Self {
        ApiError::new(ErrorCode::LiveZoomUnavailable).detail(e)
    }
}

pub type ZoomResult<T> = std::result::Result<T, ZoomError>;

/// Un inscrit chez Zoom : son identifiant et son lien personnel (`?tk=`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inscrit {
    pub registrant_id: String,
    pub join_url: Option<String>,
    /// La liste où il a été retrouvé (`approved`, `pending`, `denied`), s'il
    /// a fallu le chercher.
    pub statut: Option<&'static str>,
}

pub struct ZoomClient {
    http: reqwest::Client,
    config: ZoomConfig,
    jeton: Mutex<Option<(String, Instant)>>,
}

#[derive(Deserialize)]
struct ReponseJeton {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
struct CorpsErreur {
    code: Option<i64>,
    message: Option<String>,
}

impl ZoomClient {
    pub fn new(config: ZoomConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(DELAI)
            .connect_timeout(DELAI_CONNEXION)
            .build()
            .map_err(|e| ApiError::internal(format!("client HTTP Zoom : {}", e.without_url())))?;
        Ok(Self {
            http,
            config,
            jeton: Mutex::new(None),
        })
    }

    pub fn hote(&self) -> &str {
        &self.config.host_user
    }

    async fn jeton(&self) -> ZoomResult<String> {
        if let Some((jeton, fin)) = self.jeton.lock().expect("verrou du jeton").clone() {
            if Instant::now() + MARGE_JETON < fin {
                return Ok(jeton);
            }
        }
        let reponse = self
            .http
            .post(&self.config.oauth_url)
            .query(&[
                ("grant_type", "account_credentials"),
                ("account_id", self.config.account_id.as_str()),
            ])
            .basic_auth(
                &self.config.client_id,
                Some(self.config.client_secret.expose()),
            )
            .send()
            .await
            .map_err(|e| ZoomError::Passagere(e.without_url().to_string()))?;
        let reponse = verifier(reponse).await?;
        let corps: ReponseJeton = reponse
            .json()
            .await
            .map_err(|e| ZoomError::Passagere(format!("jeton illisible : {}", e.without_url())))?;
        let fin = Instant::now() + Duration::from_secs(corps.expires_in);
        *self.jeton.lock().expect("verrou du jeton") = Some((corps.access_token.clone(), fin));
        Ok(corps.access_token)
    }

    fn oublier_le_jeton(&self) {
        *self.jeton.lock().expect("verrou du jeton") = None;
    }

    async fn appeler(
        &self,
        methode: Method,
        chemin: &str,
        requete: &[(&str, &str)],
        corps: Option<&Value>,
    ) -> ZoomResult<Option<Value>> {
        let mut deja_renouvele = false;
        loop {
            let jeton = self.jeton().await?;
            let mut envoi = self
                .http
                .request(
                    methode.clone(),
                    format!("{}{chemin}", self.config.api_base_url),
                )
                .bearer_auth(&jeton)
                .query(requete);
            if let Some(corps) = corps {
                envoi = envoi.json(corps);
            }
            let reponse = envoi
                .send()
                .await
                .map_err(|e| ZoomError::Passagere(e.without_url().to_string()))?;
            // Jeton révoqué côté Zoom avant son terme : un seul renouvellement.
            if reponse.status() == StatusCode::UNAUTHORIZED && !deja_renouvele {
                self.oublier_le_jeton();
                deja_renouvele = true;
                continue;
            }
            let reponse = verifier(reponse).await?;
            if reponse.status() == StatusCode::NO_CONTENT {
                return Ok(None);
            }
            let texte = reponse
                .text()
                .await
                .map_err(|e| ZoomError::Passagere(e.without_url().to_string()))?;
            if texte.trim().is_empty() {
                return Ok(None);
            }
            return serde_json::from_str(&texte)
                .map(Some)
                .map_err(|e| ZoomError::Passagere(format!("réponse illisible : {e}")));
        }
    }

    /// `POST /users/{hôte}/meetings`.
    pub async fn creer_reunion(&self, corps: &Value) -> ZoomResult<Value> {
        let chemin = format!("/users/{}/meetings", segment(&self.config.host_user));
        self.appeler(Method::POST, &chemin, &[], Some(corps))
            .await?
            .ok_or_else(|| ZoomError::Passagere("création sans réponse".to_owned()))
    }

    pub async fn lire_reunion(&self, numero: &str) -> ZoomResult<Value> {
        let chemin = format!("/meetings/{}", segment(numero));
        self.appeler(Method::GET, &chemin, &[], None)
            .await?
            .ok_or_else(|| ZoomError::Passagere("lecture sans réponse".to_owned()))
    }

    pub async fn modifier_reunion(&self, numero: &str, corps: &Value) -> ZoomResult<()> {
        let chemin = format!("/meetings/{}", segment(numero));
        self.appeler(Method::PATCH, &chemin, &[], Some(corps))
            .await
            .map(|_| ())
    }

    /// Zoom rend l'inscrit existant quand l'adresse l'est déjà (comportement
    /// constaté sur le forum, non documenté) ; s'il refuse, ou rend un inscrit
    /// sans lien, on le cherche dans la liste.
    pub async fn inscrire(
        &self,
        numero: &str,
        email: &str,
        prenom: &str,
        nom: &str,
    ) -> ZoomResult<Inscrit> {
        let chemin = format!("/meetings/{}/registrants", segment(numero));
        let corps = json!({ "email": email, "first_name": prenom, "last_name": nom });
        match self.appeler(Method::POST, &chemin, &[], Some(&corps)).await {
            Ok(Some(reponse)) => {
                let inscrit = lire_inscrit(&reponse);
                match inscrit {
                    Some(i) if i.join_url.is_some() => Ok(i),
                    autre => match self.trouver_inscrit(numero, email).await? {
                        Some(trouve) => Ok(trouve),
                        None => autre.ok_or_else(|| {
                            ZoomError::Passagere("inscription sans identifiant".to_owned())
                        }),
                    },
                }
            }
            Ok(None) => self
                .trouver_inscrit(numero, email)
                .await?
                .ok_or_else(|| ZoomError::Passagere("inscription sans réponse".to_owned())),
            Err(ZoomError::Refusee {
                statut: statut @ (400 | 409),
                code,
                message,
            }) => match self.trouver_inscrit(numero, email).await? {
                Some(trouve) => Ok(trouve),
                None => Err(ZoomError::Refusee {
                    statut,
                    code,
                    message,
                }),
            },
            Err(e) => Err(e),
        }
    }

    /// Inscrit, puis réapprouve quand Zoom range l'inscrit parmi les refusés ou
    /// en attente (réinscription après annulation, approbation manuelle passée).
    pub async fn inscrire_et_approuver(
        &self,
        numero: &str,
        email: &str,
        prenom: &str,
        nom: &str,
        deja_inscrit: bool,
    ) -> ZoomResult<Inscrit> {
        let inscrit = self.inscrire(numero, email, prenom, nom).await?;
        if deja_inscrit || matches!(inscrit.statut, Some("pending" | "denied")) {
            if let Err(e) = self
                .changer_statut_inscrit(numero, "approve", &inscrit.registrant_id, email)
                .await
            {
                if e.est_passagere() {
                    return Err(e);
                }
                tracing::warn!(erreur = %e, "réapprobation de l'inscrit refusée par Zoom");
            }
        }
        Ok(inscrit)
    }

    /// Parcourt les inscrits approuvés, en attente puis refusés (une
    /// inscription annulée est rangée parmi les refusées).
    pub async fn trouver_inscrit(&self, numero: &str, email: &str) -> ZoomResult<Option<Inscrit>> {
        let chemin = format!("/meetings/{}/registrants", segment(numero));
        for statut in ["approved", "pending", "denied"] {
            let mut suite = String::new();
            loop {
                let mut requete = vec![("status", statut), ("page_size", "300")];
                if !suite.is_empty() {
                    requete.push(("next_page_token", suite.as_str()));
                }
                let Some(page) = self.appeler(Method::GET, &chemin, &requete, None).await? else {
                    break;
                };
                let trouve = page["registrants"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|r| {
                        r["email"]
                            .as_str()
                            .is_some_and(|e| e.eq_ignore_ascii_case(email))
                    })
                    .and_then(|r| {
                        Some(Inscrit {
                            registrant_id: r["id"].as_str()?.to_owned(),
                            join_url: r["join_url"].as_str().map(str::to_owned),
                            statut: Some(statut),
                        })
                    });
                if trouve.is_some() {
                    return Ok(trouve);
                }
                match page["next_page_token"].as_str() {
                    Some(t) if !t.is_empty() => suite = t.to_owned(),
                    _ => break,
                }
            }
        }
        Ok(None)
    }

    /// Les réunions programmées de l'hôte (en cours et à venir) : de quoi
    /// retrouver une réunion créée par un essai dont la réponse s'est perdue.
    pub async fn reunions_a_venir(&self) -> ZoomResult<Vec<Value>> {
        let chemin = format!("/users/{}/meetings", segment(&self.config.host_user));
        let mut reunions = Vec::new();
        let mut suite = String::new();
        loop {
            let mut requete = vec![("type", "scheduled"), ("page_size", "300")];
            if !suite.is_empty() {
                requete.push(("next_page_token", suite.as_str()));
            }
            let Some(page) = self.appeler(Method::GET, &chemin, &requete, None).await? else {
                break;
            };
            reunions.extend(page["meetings"].as_array().into_iter().flatten().cloned());
            match page["next_page_token"].as_str() {
                Some(t) if !t.is_empty() => suite = t.to_owned(),
                _ => break,
            }
        }
        Ok(reunions)
    }

    /// Annule la réunion ; Zoom prévient les inscrits par courriel.
    pub async fn supprimer_reunion(&self, numero: &str) -> ZoomResult<()> {
        let chemin = format!("/meetings/{}", segment(numero));
        self.appeler(
            Method::DELETE,
            &chemin,
            &[("cancel_meeting_reminder", "true")],
            None,
        )
        .await
        .map(|_| ())
    }

    /// `action` : `approve`, `cancel` ou `deny`.
    pub async fn changer_statut_inscrit(
        &self,
        numero: &str,
        action: &str,
        registrant_id: &str,
        email: &str,
    ) -> ZoomResult<()> {
        let chemin = format!("/meetings/{}/registrants/status", segment(numero));
        let corps = json!({
            "action": action,
            "registrants": [{ "id": registrant_id, "email": email }],
        });
        self.appeler(Method::PUT, &chemin, &[], Some(&corps))
            .await
            .map(|_| ())
    }
}

fn lire_inscrit(reponse: &Value) -> Option<Inscrit> {
    Some(Inscrit {
        registrant_id: reponse["registrant_id"].as_str()?.to_owned(),
        join_url: reponse["join_url"].as_str().map(str::to_owned),
        statut: None,
    })
}

/// Un numéro de réunion ou une adresse d'hôte, gardé dans son segment de chemin.
fn segment(valeur: &str) -> String {
    valeur
        .bytes()
        .map(|o| match o {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'@' => {
                (o as char).to_string()
            }
            _ => format!("%{o:02X}"),
        })
        .collect()
}

async fn verifier(reponse: reqwest::Response) -> ZoomResult<reqwest::Response> {
    let statut = reponse.status();
    if statut.is_success() {
        return Ok(reponse);
    }
    if statut == StatusCode::TOO_MANY_REQUESTS {
        let reessayer_dans = reponse
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| delai_retry_after(v, time::OffsetDateTime::now_utc()));
        return Err(ZoomError::Quota { reessayer_dans });
    }
    if statut.is_server_error() {
        return Err(ZoomError::Passagere(format!("statut {}", statut.as_u16())));
    }
    let corps: Option<CorpsErreur> = reponse.json().await.ok();
    Err(ZoomError::Refusee {
        statut: statut.as_u16(),
        code: corps.as_ref().and_then(|c| c.code),
        message: corps
            .and_then(|c| c.message)
            .unwrap_or_else(|| "sans précision".to_owned()),
    })
}

/// `Retry-After` en secondes, ou en date (RFC 3339 chez Zoom pour le quota du
/// jour). Plafonné à un jour.
pub fn delai_retry_after(valeur: &str, maintenant: time::OffsetDateTime) -> Option<u64> {
    let valeur = valeur.trim();
    let secondes = match valeur.parse::<u64>() {
        Ok(s) => s,
        Err(_) => {
            let date =
                time::OffsetDateTime::parse(valeur, &time::format_description::well_known::Rfc3339)
                    .or_else(|_| {
                        time::OffsetDateTime::parse(
                            valeur,
                            &time::format_description::well_known::Rfc2822,
                        )
                    })
                    .ok()?;
            (date - maintenant).whole_seconds().max(1) as u64
        }
    };
    Some(secondes.min(86_400))
}
