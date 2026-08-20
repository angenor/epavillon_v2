//! **Une paire écartée par erreur se remet dans la file.**
//!
//! Le défaut relevé le 20/08 : l'écran range les paires selon `reviewed_at`,
//! que *toute* décision renseigne. Un report posé sur une paire écartée la
//! laissait donc parmi les paires tranchées, et le bouton « remettre dans la
//! file » ne remettait rien. Seul un report sur une paire *déjà reportée* la
//! ramenait — c'est-à-dire jamais le cas qui compte, puisqu'on se trompe en
//! écartant, pas en reportant.
//!
//! Et la fusion, elle, ne se reprend pas : rejuger une paire fusionnée
//! effacerait la trace de la fusion sans défaire la fusion.

mod commun;

use commun::{perimetres, Bac};
use org::domain::duplicates::DuplicateDecision;
use org::domain::ids::PersonId;
use org::service::duplicates;
use uuid::Uuid;

/// Consigne la paire OSED comme le balayage le ferait, sans le faire tourner.
async fn consigner(bac: &Bac, gauche: Uuid, droite: Uuid) -> Uuid {
    let (g, d) = if gauche < droite {
        (gauche, droite)
    } else {
        (droite, gauche)
    };

    sqlx::query_scalar!(
        "INSERT INTO org.duplicate_candidates (left_id, right_id, score, reasons)
         VALUES ($1, $2, 88.0, ARRAY['name_similarity', 'shared_domain'])
         RETURNING id",
        g,
        d
    )
    .fetch_one(bac.pool())
    .await
    .expect("consignation de la paire")
}

async fn decider(
    bac: &Bac,
    acteur: Uuid,
    paire: Uuid,
    decision: &str,
) -> kernel::error::Result<()> {
    duplicates::decide(
        &bac.state,
        &bac.ctx().with_actor(acteur),
        PersonId(acteur),
        DuplicateDecision {
            pair_id: Some(paire),
            decision: decision.to_owned(),
            note: None,
        },
    )
    .await
    .map(|_| ())
}

#[tokio::test]
async fn une_paire_ecartee_puis_remise_en_file_y_revient() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;
    let paire = consigner(&bac, osed.complete, osed.jumelle).await;

    decider(&bac, p.globale, paire, "distinct")
        .await
        .expect("écarter la paire");

    let file = duplicates::queue(&bac.state).await.expect("la file");
    assert!(
        file.pending.iter().all(|e| e.id.as_uuid() != paire),
        "une paire écartée quitte la file"
    );
    assert!(file.settled.iter().any(|e| e.id.as_uuid() == paire));

    // Le geste de l'écran : « remettre dans la file ».
    decider(&bac, p.globale, paire, "deferred")
        .await
        .expect("remettre la paire dans la file");

    let file = duplicates::queue(&bac.state).await.expect("la file");
    let revenue = file
        .pending
        .iter()
        .find(|e| e.id.as_uuid() == paire)
        .expect("la paire écartée est revenue dans la file");
    assert!(
        revenue.reviewed_at.is_none(),
        "elle redevient un dossier ouvert"
    );
    assert!(revenue.decision.is_none());
    assert!(file.settled.iter().all(|e| e.id.as_uuid() != paire));
}

/// Un report sur une paire **en attente** la met de côté : c'est l'autre moitié
/// du même geste, et elle ne doit pas être emportée par la correction.
#[tokio::test]
async fn un_report_sur_une_paire_en_attente_la_sort_de_la_file() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;
    let paire = consigner(&bac, osed.complete, osed.jumelle).await;

    decider(&bac, p.globale, paire, "deferred")
        .await
        .expect("reporter la paire");

    let file = duplicates::queue(&bac.state).await.expect("la file");
    assert!(file.pending.iter().all(|e| e.id.as_uuid() != paire));
    let rangee = file
        .settled
        .iter()
        .find(|e| e.id.as_uuid() == paire)
        .expect("la paire reportée est rangée");
    assert_eq!(rangee.decision.as_deref(), Some("deferred"));
}

#[tokio::test]
async fn une_paire_fusionnee_ne_se_rejuge_pas() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;
    let paire = consigner(&bac, osed.complete, osed.jumelle).await;

    sqlx::query!(
        "UPDATE org.duplicate_candidates SET decision = 'merged', reviewed_at = now()
          WHERE id = $1",
        paire
    )
    .execute(bac.pool())
    .await
    .expect("marquer la paire fusionnée");

    let refus = decider(&bac, p.globale, paire, "deferred")
        .await
        .expect_err("une paire fusionnée ne se rejuge pas");
    assert_eq!(refus.code, kernel::error::ErrorCode::ValidationFailed);

    let toujours = sqlx::query_scalar!(
        "SELECT decision FROM org.duplicate_candidates WHERE id = $1",
        paire
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture");
    assert_eq!(toujours.as_deref(), Some("merged"));
}
