//! **Dix passages du balayage : aucune paire en double, aucune paire écartée
//! ramenée** (SC-013).
//!
//! Une seule ligne SQL tient les deux moitiés de FR-059 :
//! `ON CONFLICT … DO UPDATE … WHERE reviewed_at IS NULL`. Une paire déjà
//! arbitrée n'est pas ressuscitée, une paire en attente est mise à jour.

mod commun;

use commun::{perimetres, Bac};
use kernel::jobs::ClaimedJob;
use kernel::jobs::JobHandler;
use org::domain::duplicates::DuplicateDecision;
use org::domain::ids::PersonId;
use org::jobs::duplicates::ScanDuplicates;
use org::service::duplicates;
use serde_json::json;
use uuid::Uuid;

/// Exécute le balayage jusqu'à épuisement, comme le worker le ferait en
/// enchaînant les tranches.
async fn balayer(bac: &Bac) {
    let handler = ScanDuplicates::new(bac.db(), bac.config.as_ref());
    let mut curseur: Option<Uuid> = None;
    let jour = "2026-08-20";

    // Le référentiel de test tient en deux ou trois tranches ; la borne évite
    // qu'un défaut de curseur fasse boucler le test sans fin.
    for _ in 0..50 {
        let avant = fiches_apres(bac, curseur).await;
        handler
            .run(&travail(json!({ "jour": jour, "apres": curseur })))
            .await
            .expect("tranche de balayage");

        if avant.is_empty() {
            return;
        }
        curseur = avant.last().copied();
    }
    panic!("le balayage n'a pas convergé");
}

async fn fiches_apres(bac: &Bac, apres: Option<Uuid>) -> Vec<Uuid> {
    sqlx::query_scalar!(
        "SELECT id FROM org.organizations
          WHERE status IN ('candidate', 'active') AND ($1::uuid IS NULL OR id > $1)
          ORDER BY id LIMIT 200",
        apres
    )
    .fetch_all(bac.pool())
    .await
    .expect("tranche")
}

fn travail(payload: serde_json::Value) -> ClaimedJob {
    ClaimedJob {
        id: Uuid::now_v7(),
        queue: "default".to_owned(),
        task: "org.duplicates.scan".to_owned(),
        payload,
        attempts: 0,
        max_attempts: 5,
    }
}

async fn paires(bac: &Bac) -> Vec<(Uuid, Uuid, Option<String>)> {
    sqlx::query!(
        "SELECT left_id, right_id, decision FROM org.duplicate_candidates
          ORDER BY left_id, right_id"
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de la file")
    .into_iter()
    .map(|l| (l.left_id, l.right_id, l.decision))
    .collect()
}

#[tokio::test]
async fn dix_passages_ne_produisent_ni_doublon_ni_resurrection() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    balayer(&bac).await;

    let apres_un_passage = paires(&bac).await;
    assert!(
        apres_un_passage
            .iter()
            .any(|(g, d, _)| (*g == osed.complete && *d == osed.jumelle)
                || (*g == osed.jumelle && *d == osed.complete)),
        "la paire OSED doit être consignée : elle partage nom et domaine"
    );

    // La paire est écartée : ce ne sont pas des doublons.
    let pair_id = sqlx::query_scalar!(
        "SELECT id FROM org.duplicate_candidates
          WHERE left_id IN ($1, $2) AND right_id IN ($1, $2)",
        osed.complete,
        osed.jumelle
    )
    .fetch_one(bac.pool())
    .await
    .expect("la paire");

    duplicates::decide(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        DuplicateDecision {
            pair_id: Some(pair_id),
            decision: "distinct".to_owned(),
            note: Some("deux antennes distinctes".to_owned()),
        },
    )
    .await
    .expect("décision");

    // Neuf passages de plus.
    for _ in 0..9 {
        balayer(&bac).await;
    }

    let finales = paires(&bac).await;

    assert_eq!(
        finales.len(),
        apres_un_passage.len(),
        "aucune paire en double après dix passages"
    );

    let ecartee = finales
        .iter()
        .find(|(g, d, _)| {
            (*g == osed.complete && *d == osed.jumelle)
                || (*g == osed.jumelle && *d == osed.complete)
        })
        .expect("la paire écartée est toujours là");
    assert_eq!(
        ecartee.2.as_deref(),
        Some("distinct"),
        "une paire écartée n'est JAMAIS ramenée dans la file"
    );

    let en_attente = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.duplicate_candidates WHERE reviewed_at IS NULL"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(en_attente, 0, "la file est vide : tout a été arbitré");
}

/// **Une paire en attente est mise à jour**, elle, et c'est l'autre moitié de la
/// même ligne SQL.
#[tokio::test]
async fn une_paire_en_attente_est_mise_a_jour() {
    let bac = Bac::monter().await;
    let osed = commun::seed::paire_osed(&bac).await;

    balayer(&bac).await;

    let premiere = sqlx::query!(
        r#"SELECT detected_at, score::float8 AS "score!" FROM org.duplicate_candidates
            WHERE left_id IN ($1, $2) AND right_id IN ($1, $2)"#,
        osed.complete,
        osed.jumelle
    )
    .fetch_one(bac.pool())
    .await
    .expect("la paire");

    balayer(&bac).await;

    let seconde = sqlx::query!(
        r#"SELECT detected_at, score::float8 AS "score!" FROM org.duplicate_candidates
            WHERE left_id IN ($1, $2) AND right_id IN ($1, $2)"#,
        osed.complete,
        osed.jumelle
    )
    .fetch_one(bac.pool())
    .await
    .expect("la paire");

    assert!(
        seconde.detected_at >= premiere.detected_at,
        "la détection est rafraîchie"
    );
    assert_eq!(seconde.score, premiere.score);
}
