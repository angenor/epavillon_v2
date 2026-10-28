//! **Un dossier qui sort de la course libère son créneau pressenti.**
//!
//! Un dossier en évaluation peut porter une séance `planned` (créneau repris de
//! la v1). Rejeté, retiré ou annulé, il ne doit ni la voir publiée au prochain
//! programme, ni la laisser dans le planificateur.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use programme::domain::transitions::ProposalStatus;
use programme::service::transition;
use uuid::Uuid;

/// Un dossier en évaluation, avec le créneau pressenti que lui donnait la v1.
async fn dossier_avec_creneau(bac: &Bac, terrain: &Terrain, slug: &str) -> (Uuid, Uuid) {
    let dossier = seances::dossier_pret(bac, terrain, slug, slug, Souhaits::default()).await;

    let mut tx = bac.db().write(&bac.ctx()).await.unwrap();
    programme::service::birth::faire_naitre(&mut tx, dossier.id)
        .await
        .expect("créneau pressenti");
    tx.commit().await.unwrap();

    let seance = seances::seances_du_dossier(bac, dossier.id).await.remove(0);
    assert_eq!(seance.status, "planned");
    assert_eq!(seance.published_at, None);
    (dossier.id, seance.id)
}

async fn decider(bac: &Bac, dossier: Uuid, vers: ProposalStatus) {
    let issue = transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.into(),
        vers,
        Some("Hors du champ de l'appel."),
    )
    .await
    .expect("la décision aboutit");
    assert!(
        matches!(issue, transition::Issue::Appliquee(_)),
        "transition appliquée"
    );
}

async fn lignes_de_seance_dans_loutbox(bac: &Bac) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events
            WHERE aggregate_type = 'session'"#
    )
    .fetch_one(bac.pool())
    .await
    .unwrap()
}

async fn themes_de_la_seance(bac: &Bac, session_id: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM reference.entity_terms
            WHERE entity_schema = 'programme' AND entity_table = 'sessions'
              AND entity_id = $1"#,
        session_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap()
}

/// 🔴 Rejeter supprime la séance pressentie, et **rien ne part** vers le direct
/// ni les rappels : aucune ligne d'outbox de séance n'est ajoutée.
#[tokio::test]
async fn rejeter_libere_le_creneau_sans_rien_emettre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let (dossier, seance) = dossier_avec_creneau(&bac, &terrain, "rejete").await;
    assert!(themes_de_la_seance(&bac, seance).await > 0);
    let avant = lignes_de_seance_dans_loutbox(&bac).await;

    decider(&bac, dossier, ProposalStatus::Rejected).await;

    assert!(
        seances::seances_du_dossier(&bac, dossier).await.is_empty(),
        "le créneau pressenti est libéré"
    );
    assert_eq!(
        themes_de_la_seance(&bac, seance).await,
        0,
        "ses thématiques partent avec elle"
    );
    assert_eq!(
        lignes_de_seance_dans_loutbox(&bac).await,
        avant,
        "aucun événement de séance : ni Zoom, ni rappel, ni avis"
    );
    assert_eq!(
        commun::evenements_emis(&bac, dossier)
            .await
            .last()
            .map(String::as_str),
        Some("programme.proposal.rejected"),
        "seul l'événement du dossier, émis par le garde"
    );
}

/// Retirer libère aussi le créneau.
#[tokio::test]
async fn retirer_libere_le_creneau() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let (dossier, _) = dossier_avec_creneau(&bac, &terrain, "retire").await;
    decider(&bac, dossier, ProposalStatus::Withdrawn).await;

    assert!(seances::seances_du_dossier(&bac, dossier).await.is_empty());
}

/// Rejeter un dossier **sans séance** ne touche à rien, et la séance d'un autre
/// dossier reste en place.
#[tokio::test]
async fn rejeter_un_dossier_sans_seance_ne_change_rien() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let (_, voisine) = dossier_avec_creneau(&bac, &terrain, "voisin").await;
    let sans_seance = seances::dossier_pret(
        &bac,
        &terrain,
        "Sans séance",
        "sans-seance",
        Souhaits::default(),
    )
    .await;

    decider(&bac, sans_seance.id, ProposalStatus::Rejected).await;

    let restantes = sqlx::query_scalar!(
        "SELECT id FROM programme.sessions WHERE event_id = $1",
        terrain.edition
    )
    .fetch_all(bac.pool())
    .await
    .unwrap();
    assert_eq!(restantes, vec![voisine], "la séance voisine est intacte");
}

// Le sort d'une séance déjà publiée d'un dossier annulé : voir
// `annulation_dossier_retenu.rs`.
