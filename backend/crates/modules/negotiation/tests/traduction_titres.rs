//! **FR-023, FR-024, SC-010** — une traduction par titre anglais, refaite
//! seulement s'il change ; un échec n'écrit rien ; sans clé, rien n'est posé.
//! Un traducteur fixe tient lieu d'OpenRouter : aucun test ne l'appelle.

mod importation;

use async_trait::async_trait;
use importation::Bac;
use kernel::jobs::{ClaimedJob, JobHandler};
use negotiation::import::traduction::{EchecTraduction, Traducteur};
use negotiation::jobs::traduction::{TranslateSessionTitles, TRANSLATE_SESSION_TITLES};
use serde_json::json;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Default)]
struct Fixe {
    demandes: Mutex<Vec<String>>,
    appels: Mutex<usize>,
}

#[async_trait]
impl Traducteur for Fixe {
    async fn traduire(
        &self,
        _modele: &str,
        titres: &[String],
    ) -> Result<Vec<String>, EchecTraduction> {
        *self.appels.lock().unwrap() += 1;
        self.demandes.lock().unwrap().extend_from_slice(titres);
        Ok(titres.iter().map(|t| format!("FR {t}")).collect())
    }
}

struct EnPanne;

#[async_trait]
impl Traducteur for EnPanne {
    async fn traduire(&self, _: &str, _: &[String]) -> Result<Vec<String>, EchecTraduction> {
        Err(EchecTraduction::DelaiDepasse)
    }
}

/// Rend un élément de moins que demandé.
struct Bancal;

#[async_trait]
impl Traducteur for Bancal {
    async fn traduire(&self, _: &str, titres: &[String]) -> Result<Vec<String>, EchecTraduction> {
        Ok(titres.iter().skip(1).cloned().collect())
    }
}

async fn traduire(bac: &Bac, traducteur: Option<Arc<dyn Traducteur>>) {
    let travail = ClaimedJob {
        id: Uuid::now_v7(),
        queue: "default".into(),
        task: TRANSLATE_SESSION_TITLES.into(),
        payload: json!({ "event_id": bac.edition }),
        attempts: 1,
        max_attempts: 5,
    };
    TranslateSessionTitles::new(bac.base.db(), traducteur)
        .run(&travail)
        .await
        .expect("le travail de traduction réussit toujours côté file");
}

async fn traductions(bac: &Bac) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM negotiation.title_translations")
        .fetch_one(bac.pool())
        .await
        .expect("traductions")
}

async fn posees(bac: &Bac) -> i64 {
    bac.nombre(
        "SELECT count(*) FROM platform.jobs
          WHERE task = 'negotiation.translate_session_titles'
            AND payload ->> 'event_id' = $1::text",
    )
    .await
}

/// La traduction servie, par la jointure que lira la route : `source_text`
/// = `title_original`.
async fn servie(bac: &Bac, cle: &str) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT t.text_fr FROM negotiation.meetings m
           JOIN negotiation.title_translations t ON t.source_text = m.title_original
          WHERE m.event_id = $1 AND m.source_key = $2",
    )
    .bind(bac.edition)
    .bind(cle)
    .fetch_optional(bac.pool())
    .await
    .expect("traduction servie")
}

#[tokio::test]
async fn un_titre_partage_par_deux_sessions_n_est_traduit_qu_une_fois() {
    let bac = Bac::monter().await;
    bac.lire().await;
    let titre = bac.session("654006").await.expect("session").title_original;
    sqlx::query(
        "UPDATE negotiation.meetings SET title_original = $2 WHERE event_id = $1 AND source_key = '654010'",
    )
    .bind(bac.edition)
    .bind(&titre)
    .execute(bac.pool())
    .await
    .expect("titre partagé");
    let distincts = bac
        .nombre(
            "SELECT count(DISTINCT title_original) FROM negotiation.meetings WHERE event_id = $1",
        )
        .await;

    let fixe = Arc::new(Fixe::default());
    traduire(&bac, Some(fixe.clone())).await;
    traduire(&bac, Some(fixe.clone())).await;

    let demandes = fixe.demandes.lock().unwrap().clone();
    assert_eq!(
        demandes.len() as i64,
        distincts,
        "chaque titre demandé une fois"
    );
    assert_eq!(demandes.iter().filter(|t| **t == titre).count(), 1);
    assert_eq!(
        *fixe.appels.lock().unwrap(),
        (distincts as usize).div_ceil(40),
        "un appel par lot de quarante, aucun au second passage"
    );
    assert_eq!(traductions(&bac).await, distincts);
    let attendue = format!("FR {titre}");
    assert_eq!(
        servie(&bac, "654006").await.as_deref(),
        Some(attendue.as_str())
    );
    assert_eq!(
        servie(&bac, "654010").await.as_deref(),
        Some(attendue.as_str())
    );

    let modele: String = sqlx::query_scalar(
        "SELECT DISTINCT t.model FROM negotiation.title_translations t
          WHERE t.model = (SELECT value #>> '{}' FROM platform.settings WHERE key = 'ai.drafting_model')",
    )
    .fetch_one(bac.pool())
    .await
    .expect("le modèle du réglage est noté");
    assert!(!modele.is_empty());
}

#[tokio::test]
async fn un_titre_anglais_change_recoit_une_nouvelle_traduction() {
    let bac = Bac::monter().await;
    bac.lire().await;
    let fixe = Arc::new(Fixe::default());
    traduire(&bac, Some(fixe.clone())).await;
    let ancienne = servie(&bac, "654006").await.expect("traduite");

    sqlx::query(
        "UPDATE negotiation.meetings SET title_original = 'Closing plenary of the SBSTA'
          WHERE event_id = $1 AND source_key = '654006'",
    )
    .bind(bac.edition)
    .execute(bac.pool())
    .await
    .expect("titre changé à la source");
    fixe.demandes.lock().unwrap().clear();
    traduire(&bac, Some(fixe.clone())).await;

    assert_eq!(
        *fixe.demandes.lock().unwrap(),
        ["Closing plenary of the SBSTA"],
        "seul le nouveau titre est demandé"
    );
    let nouvelle = servie(&bac, "654006").await.expect("traduite");
    assert_eq!(nouvelle, "FR Closing plenary of the SBSTA");
    assert_ne!(nouvelle, ancienne, "l'ancienne n'est plus servie");
}

#[tokio::test]
async fn un_echec_ou_un_lot_mal_aligne_n_ecrit_rien() {
    let bac = Bac::monter().await;
    bac.lire().await;

    traduire(&bac, Some(Arc::new(EnPanne))).await;
    assert_eq!(traductions(&bac).await, 0);

    traduire(&bac, Some(Arc::new(Bancal))).await;
    assert_eq!(traductions(&bac).await, 0);
    assert_eq!(servie(&bac, "654006").await, None, "l'anglais seul");
}

#[tokio::test]
async fn sans_cle_rien_n_est_pose_ni_traduit() {
    let bac = Bac::monter().await;
    bac.lire().await;
    bac.lire_a_la_main().await;
    assert_eq!(posees(&bac).await, 0);

    traduire(&bac, None).await;
    assert_eq!(traductions(&bac).await, 0);
}

#[tokio::test]
async fn avec_cle_l_import_pose_un_seul_travail_tant_qu_il_attend() {
    let bac = Bac::monter().await;
    bac.lire_en_traduisant().await;
    assert_eq!(posees(&bac).await, 1);
    bac.lire_en_traduisant().await;
    assert_eq!(
        posees(&bac).await,
        1,
        "aucun second travail tant que le premier attend"
    );

    traduire(&bac, Some(Arc::new(Fixe::default()))).await;
    sqlx::query("UPDATE platform.jobs SET status = 'succeeded' WHERE task = $1")
        .bind(TRANSLATE_SESSION_TITLES)
        .execute(bac.pool())
        .await
        .expect("travail fini");
    bac.lire_en_traduisant().await;
    assert_eq!(posees(&bac).await, 1, "tout est traduit : rien à poser");
}
