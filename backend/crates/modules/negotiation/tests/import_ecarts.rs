//! **SC-002, FR-015** — `lecture-2` porte exactement quatre écarts, et chacun
//! garde la valeur précédente ; la disparue ne change pas à sa première absence,
//! passe annulée à la seconde, et redevient prévue si elle reparaît.

mod importation;

use importation::Bac;
use serde_json::json;
use time::macros::datetime;

#[tokio::test]
async fn lecture_2_porte_quatre_ecarts_et_garde_les_valeurs_precedentes() {
    let bac = Bac::monter().await;
    bac.lire().await;
    bac.jeu("cop30/lecture-2").await;
    bac.lire().await;

    assert_eq!(
        bac.journal().await[0],
        ("success".into(), Some(44), Some(4), None)
    );
    assert_eq!(bac.etat().await.last_change_count, Some(4));

    let avancee = bac.session("654214").await.expect("session avancée");
    assert_eq!(avancee.start_at, datetime!(2026-11-10 11:00 UTC));
    assert_eq!(
        bac.changements("654214").await,
        [
            (
                "start".into(),
                json!("2026-11-10T12:00:00Z"),
                json!("2026-11-10T11:00:00Z")
            ),
            (
                "end".into(),
                json!("2026-11-10T13:00:00Z"),
                json!("2026-11-10T12:00:00Z")
            ),
        ]
    );

    let salle = bac
        .session("654010")
        .await
        .expect("session changée de salle");
    assert_eq!(salle.venue_label.as_deref(), Some("Meeting Room 03"));
    assert_eq!(
        bac.changements("654010").await,
        [(
            "venue".into(),
            json!("Meeting Room 01"),
            json!("Meeting Room 03")
        )]
    );

    let nouvelle = bac.session("699001").await.expect("session nouvelle");
    assert_eq!(nouvelle.status, "scheduled");
    assert!(bac.changements("699001").await.is_empty());

    let disparue = bac.session("654364").await.expect("jamais effacée");
    assert_eq!(
        (disparue.status.as_str(), disparue.absent_reads),
        ("scheduled", 1)
    );
    assert!(bac.changements("654364").await.is_empty());
}

#[tokio::test]
async fn a_sa_premiere_absence_la_disparue_garde_la_derniere_lecture_ou_elle_figurait() {
    let bac = Bac::monter().await;
    bac.lire().await;
    bac.lire().await;
    let derniere = bac.etat().await.last_success_at.expect("réussie");
    bac.jeu("cop30/lecture-2").await;
    bac.lire().await;
    bac.lire().await;

    let disparue = bac.session("654364").await.expect("jamais effacée");
    assert!(disparue.first_read_at < derniere);
    assert_eq!(disparue.last_read_at, derniere);
}

#[tokio::test]
async fn la_disparue_est_annulee_a_la_seconde_absence_puis_revient_si_elle_reparait() {
    let bac = Bac::monter().await;
    bac.lire().await;
    bac.jeu("cop30/lecture-2").await;
    bac.lire().await;
    bac.lire().await;

    let annulee = bac.session("654364").await.expect("jamais effacée");
    assert_eq!(
        (
            annulee.status.as_str(),
            annulee.cancellation_reason.as_deref(),
            annulee.absent_reads
        ),
        ("cancelled", Some("removed"), 2)
    );
    assert!(annulee.cancelled_at.is_some());
    assert_eq!(
        bac.changements("654364").await,
        [(
            "status".into(),
            json!({ "status": "scheduled" }),
            json!({ "status": "cancelled", "reason": "removed" })
        )]
    );
    assert_eq!(
        bac.journal().await[0].2,
        Some(1),
        "seule la disparue est touchée"
    );

    bac.lire().await;
    assert_eq!(
        bac.journal().await[0].2,
        Some(0),
        "une annulée absente n'est plus touchée"
    );

    bac.jeu("cop30/lecture-1").await;
    bac.lire().await;
    let reparue = bac.session("654364").await.expect("session");
    assert_eq!(
        (
            reparue.status.as_str(),
            reparue.cancellation_reason,
            reparue.absent_reads
        ),
        ("scheduled", None, 0)
    );
    assert!(reparue.cancelled_at.is_none());
    assert_eq!(bac.changements("654364").await.len(), 2);
    assert_eq!(
        bac.session("699001").await.expect("session").absent_reads,
        1,
        "la nouvelle de lecture-2 manque à son tour"
    );
    assert_eq!(
        bac.session("654214").await.expect("session").start_at,
        datetime!(2026-11-10 12:00 UTC)
    );
}
