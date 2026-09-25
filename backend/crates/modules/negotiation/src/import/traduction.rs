//! La traduction automatique des titres de sessions (research R11) : un trait,
//! et son client OpenRouter. Les tests emploient un traducteur fixe.
//!
//! La clé ne sort jamais d'ici : ni dans un journal, ni dans une erreur, ni
//! dans un `Debug` — `Secret` ne rend que sa longueur.

use async_trait::async_trait;
use kernel::config::Secret;
use serde::Deserialize;
use serde_json::json;
use std::fmt;
use std::time::Duration;

/// Titres envoyés en un seul appel.
pub const TAILLE_DU_LOT: usize = 40;

const ADRESSE: &str = "https://openrouter.ai/api/v1/chat/completions";
const DELAI: Duration = Duration::from_secs(30);

const CONSIGNE: &str = "Tu traduis en français des titres de réunions des négociations \
internationales sur le climat (CCNUCC). Français sobre et exact, dans le vocabulaire \
officiel de ces négociations. Garde tels quels, sans les traduire, tous les sigles en \
anglais — organes (SBSTA, SBI, CMA, CMP, COP, IPCC, NDC…) comme groupes de négociation \
(EIG, LMDC, AGN, AOSIS, LDC, G77…) — et les codes de points d'ordre du jour (par ex. \
« 4(a) », « 12 »). \
N'ajoute rien, n'explique rien. On te donne un tableau JSON de titres anglais ; réponds \
par un objet JSON {\"traductions\": [...]} dont le tableau a exactement autant \
d'éléments, dans le même ordre.";

#[derive(Debug)]
pub enum EchecTraduction {
    Injoignable(String),
    DelaiDepasse,
    Illisible(String),
    MalAligne { attendus: usize, rendus: usize },
}

impl fmt::Display for EchecTraduction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Injoignable(cause) => write!(f, "Traduction injoignable : {cause}."),
            Self::DelaiDepasse => write!(f, "La traduction n'a pas répondu dans les 30 secondes."),
            Self::Illisible(cause) => write!(f, "Réponse de traduction illisible : {cause}."),
            Self::MalAligne { attendus, rendus } => write!(
                f,
                "Lot de traduction mal aligné : {attendus} titres, {rendus} traductions."
            ),
        }
    }
}

#[async_trait]
pub trait Traducteur: Send + Sync {
    /// Une traduction par titre, dans l'ordre ; un lot mal aligné est refusé.
    async fn traduire(
        &self,
        modele: &str,
        titres: &[String],
    ) -> Result<Vec<String>, EchecTraduction>;
}

pub struct OpenRouter {
    cle: Secret,
    client: reqwest::Client,
}

impl OpenRouter {
    pub fn new(cle: Secret) -> Result<Self, EchecTraduction> {
        let client = reqwest::Client::builder()
            .timeout(DELAI)
            .user_agent("ePavillon-GuideNego/1.0 (IFDD)")
            .build()
            .map_err(|e| EchecTraduction::Injoignable(format!("client HTTP : {e}")))?;
        Ok(Self { cle, client })
    }
}

#[derive(Deserialize)]
struct Reponse {
    choices: Vec<Choix>,
}

#[derive(Deserialize)]
struct Choix {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

#[derive(Deserialize)]
struct Traductions {
    traductions: Vec<String>,
}

#[async_trait]
impl Traducteur for OpenRouter {
    async fn traduire(
        &self,
        modele: &str,
        titres: &[String],
    ) -> Result<Vec<String>, EchecTraduction> {
        let entree =
            serde_json::to_string(titres).map_err(|e| EchecTraduction::Illisible(e.to_string()))?;
        let corps = json!({
            "model": modele,
            "temperature": 0,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": CONSIGNE },
                { "role": "user", "content": entree },
            ],
        });
        let reponse = self
            .client
            .post(ADRESSE)
            .bearer_auth(self.cle.expose())
            .json(&corps)
            .send()
            .await
            .map_err(echec)?;
        let statut = reponse.status();
        if !statut.is_success() {
            return Err(EchecTraduction::Injoignable(format!(
                "OpenRouter a répondu {}",
                statut.as_u16()
            )));
        }
        let reponse: Reponse = reponse.json().await.map_err(echec)?;
        let contenu = reponse
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| EchecTraduction::Illisible("aucun contenu".into()))?;
        aligner(titres.len(), &contenu)
    }
}

/// Lit la réponse du modèle, clôture Markdown tolérée, et refuse un tableau
/// qui n'a pas un élément non vide par titre.
pub fn aligner(attendus: usize, contenu: &str) -> Result<Vec<String>, EchecTraduction> {
    let contenu = contenu.trim();
    let contenu = contenu
        .strip_prefix("```json")
        .or_else(|| contenu.strip_prefix("```"))
        .and_then(|c| c.strip_suffix("```"))
        .unwrap_or(contenu)
        .trim();
    let lues: Traductions =
        serde_json::from_str(contenu).map_err(|e| EchecTraduction::Illisible(e.to_string()))?;
    let traductions: Vec<String> = lues
        .traductions
        .into_iter()
        .map(|t| t.trim().to_owned())
        .collect();
    if traductions.len() != attendus || traductions.iter().any(String::is_empty) {
        return Err(EchecTraduction::MalAligne {
            attendus,
            rendus: traductions.iter().filter(|t| !t.is_empty()).count(),
        });
    }
    Ok(traductions)
}

fn echec(e: reqwest::Error) -> EchecTraduction {
    if e.is_timeout() {
        EchecTraduction::DelaiDepasse
    } else {
        EchecTraduction::Injoignable(e.without_url().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_tableau_aligne_passe_meme_sous_cloture() {
        let lu = aligner(
            2,
            "```json\n{\"traductions\": [\"Plénière\", \" Clôture \"]}\n```",
        );
        assert_eq!(lu.expect("aligné"), ["Plénière", "Clôture"]);
    }

    #[test]
    fn un_tableau_trop_court_troue_ou_illisible_est_refuse() {
        assert!(matches!(
            aligner(3, r#"{"traductions": ["a", "b"]}"#),
            Err(EchecTraduction::MalAligne {
                attendus: 3,
                rendus: 2
            })
        ));
        assert!(matches!(
            aligner(2, r#"{"traductions": ["a", " "]}"#),
            Err(EchecTraduction::MalAligne { .. })
        ));
        assert!(matches!(
            aligner(1, "pas du JSON"),
            Err(EchecTraduction::Illisible(_))
        ));
    }

    #[test]
    fn la_cle_ne_sort_pas_par_debug() {
        let cle = Secret::from("sk-or-v1-confidentiel".to_owned());
        assert!(!format!("{cle:?}").contains("confidentiel"));
    }
}
