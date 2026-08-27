//! **Le seuil vit en base, et le changer change l'écran — sans redéploiement.**
//!
//! C'est l'écart n° 43, ouvert le 17/08 : le seuil de vingt et un jours était
//! écrit dans le code du site, ce que le principe I qualifie de « dette
//! immédiate ». C'est une règle d'exploitation que l'IFDD ajuste d'une COP à
//! l'autre.

mod commun;

use analytics::domain::action::AdminActionKind;
use commun::*;

/// Le décompte de la famille « dossiers sans évaluation », ou zéro.
async fn dossiers_signales(bac: &Bac, event_id: uuid::Uuid) -> i64 {
    analytics::service::dashboard::composer(bac.pool(), event_id)
        .await
        .expect("composition")
        .actions
        .iter()
        .find(|a| a.kind == AdminActionKind::ProposalsUnreviewed)
        .map(|a| a.count)
        .unwrap_or(0)
}

#[tokio::test]
async fn changer_le_seuil_change_le_contenu_de_la_famille() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;

    // Un dossier confié, dont l'échéance de revue tombe dans dix jours : à
    // vingt et un jours il s'allume, à un jour il ne s'allume plus.
    let comptes = comptes(&bac, &decor).await;
    let appel_id = appel(&bac, decor.event_id, "cop31_appel", 60).await;
    let dossier = dossier_depose(
        &bac,
        decor.event_id,
        Some(appel_id),
        decor.organization_id,
        comptes.globale,
        "Dossier confié",
    )
    .await;
    sqlx::query!(
        "INSERT INTO programme.review_assignments (proposal_id, reviewer_id, due_at)
         VALUES ($1, $2, now() + interval '10 days')",
        dossier,
        comptes.globale
    )
    .execute(bac.pool())
    .await
    .expect("affectation");
    rafraichir(&bac).await;

    assert_eq!(
        dossiers_signales(&bac, decor.event_id).await,
        1,
        "à 21 jours, une échéance à 10 jours est une alerte"
    );

    poser_le_seuil(&bac, 1).await;
    assert_eq!(
        dossiers_signales(&bac, decor.event_id).await,
        0,
        "à 1 jour, elle ne l'est plus — et rien n'a été redéployé"
    );
}

#[tokio::test]
async fn le_reglage_supprime_fait_retomber_sur_vingt_et_un() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;

    let comptes = comptes(&bac, &decor).await;
    let appel_id = appel(&bac, decor.event_id, "cop31_appel", 60).await;
    let dossier = dossier_depose(
        &bac,
        decor.event_id,
        Some(appel_id),
        decor.organization_id,
        comptes.globale,
        "Dossier confié",
    )
    .await;
    sqlx::query!(
        "INSERT INTO programme.review_assignments (proposal_id, reviewer_id, due_at)
         VALUES ($1, $2, now() + interval '15 days')",
        dossier,
        comptes.globale
    )
    .execute(bac.pool())
    .await
    .expect("affectation");
    rafraichir(&bac).await;

    supprimer_le_seuil(&bac).await;
    assert_eq!(
        dossiers_signales(&bac, decor.event_id).await,
        1,
        "le repli vaut 21 — la valeur même que 130_analytics.sql déclare"
    );
}
