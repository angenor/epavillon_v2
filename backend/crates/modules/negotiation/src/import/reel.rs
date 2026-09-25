//! Le lecteur de la source réelle : le JSON du calendrier de conférence, par
//! HTTP. Écrit, mais la protection anti-robot de la CCNUCC le refuse tant que
//! l'accès ouvert par l'accord du secrétariat manque (research R2).

use async_trait::async_trait;
use std::time::Duration;

use super::ccnucc;
use super::source::{EchecLecture, SessionLue, SourceOfficielle};

const DELAI: Duration = Duration::from_secs(15);

pub struct LecteurReel {
    pub url: String,
    pub correction_minutes: i64,
}

#[async_trait]
impl SourceOfficielle for LecteurReel {
    async fn lire(&self) -> Result<Vec<SessionLue>, EchecLecture> {
        let client = reqwest::Client::builder()
            .timeout(DELAI)
            .user_agent("ePavillon-GuideNego/1.0 (IFDD)")
            .build()
            .map_err(|e| EchecLecture::Injoignable(format!("client HTTP : {e}")))?;

        let reponse = client
            .get(&self.url)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(echec)?;
        let statut = reponse.status();
        if !statut.is_success() {
            return Err(EchecLecture::Injoignable(format!(
                "la source a répondu {}",
                statut.as_u16()
            )));
        }
        let texte = reponse.text().await.map_err(echec)?;
        ccnucc::analyser(&texte, self.correction_minutes)
    }
}

fn echec(e: reqwest::Error) -> EchecLecture {
    if e.is_timeout() {
        EchecLecture::DelaiDepasse
    } else {
        EchecLecture::Injoignable(e.without_url().to_string())
    }
}
