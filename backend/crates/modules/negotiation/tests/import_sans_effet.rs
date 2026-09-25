//! **FR-021** — l'import n'émet aucun événement d'outbox, pas même à
//! l'annulation, et écrit sous le contexte de son travail.

mod importation;

use importation::Bac;

async fn evenements(bac: &Bac) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM platform.outbox_events")
        .fetch_one(bac.pool())
        .await
        .expect("outbox")
}

#[tokio::test]
async fn aucun_evenement_meme_a_l_annulation() {
    let bac = Bac::monter().await;
    let avant = evenements(&bac).await;

    bac.lire().await;
    bac.jeu("cop30/lecture-2").await;
    bac.lire().await;
    bac.lire().await;
    assert_eq!(
        bac.session("654364").await.expect("session").status,
        "cancelled"
    );
    bac.jeu("cop30/lecture-1").await;
    bac.lire().await;

    assert_eq!(evenements(&bac).await, avant);
}

#[tokio::test]
async fn l_ecriture_porte_le_contexte_du_travail() {
    let bac = Bac::monter().await;
    let travail = bac.lire().await;

    let traces: Vec<(Option<String>, Option<uuid::Uuid>)> = sqlx::query_as(
        "SELECT DISTINCT request_id, actor_id FROM platform.audit_log
          WHERE entity_schema = 'negotiation' AND entity_table IN ('meetings', 'agenda_items')",
    )
    .fetch_all(bac.pool())
    .await
    .expect("audit");
    assert_eq!(traces, [(Some(format!("job:{}", travail.id)), None)]);
}
