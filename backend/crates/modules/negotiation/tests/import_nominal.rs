//! **SC-001** — la première lecture écrit toutes les sessions retenues du jeu
//! archivé, avec leur origine ; une seconde lecture identique n'écrit que leur
//! dernière lecture, sans aucun écart.

mod importation;

use importation::Bac;
use serde_json::{json, Value};
use time::macros::datetime;

async fn instantane(bac: &Bac) -> Value {
    sqlx::query_scalar::<_, Value>(
        "SELECT jsonb_agg(to_jsonb(m) - 'last_read_at' - 'updated_at' ORDER BY m.source_key)
           FROM negotiation.meetings m WHERE m.event_id = $1",
    )
    .bind(bac.edition)
    .fetch_one(bac.pool())
    .await
    .expect("instantané")
}

#[tokio::test]
async fn la_premiere_lecture_ecrit_les_sessions_retenues_avec_leur_origine() {
    let bac = Bac::monter().await;
    bac.lire().await;

    assert_eq!(bac.sessions().await, 44, "47 réunions, dont 3 écartées");
    assert_eq!(
        bac.journal().await,
        [("success".into(), Some(44), Some(44), None)]
    );
    for ecartee in ["654208", "652781", "652457"] {
        assert!(
            bac.session(ecartee).await.is_none(),
            "{ecartee} n'est pas une négociation"
        );
    }

    let s = bac.session("654006").await.expect("session importée");
    assert_eq!(
        s.start_at,
        datetime!(2026-11-10 08:00 UTC),
        "11:00 à Antalya : l'heure que la fiche de la source affiche, jour translaté"
    );
    assert_eq!(s.end_at, Some(datetime!(2026-11-10 09:00 UTC)));
    assert_eq!(
        s.source_url,
        "https://unfccc.int/event/cma-8-a-global-goal-on-adaptation-informal-consultation-1"
    );
    assert_eq!(s.first_read_at, s.last_read_at);
    let titre = "CMA 8 (a) Global goal on adaptation - Informal consultation";
    assert_eq!(s.title_original, titre);
    assert_eq!(s.title, json!({ "fr": titre, "en": titre }));
    assert_eq!(
        (s.kind.as_str(), s.is_ifdd_organized, s.timezone.as_str()),
        ("negotiation_session", false, "Asia/Istanbul")
    );
    assert_eq!(
        (s.status.as_str(), s.is_open_access),
        ("scheduled", Some(true))
    );
    assert_eq!(s.venue_label.as_deref(), Some("Meeting Room 01"));
    assert_eq!(s.type_code.as_deref(), Some("informal_consultations"));
    assert_eq!(
        (s.point_code.as_deref(), s.point_title.as_deref()),
        (Some("CMA 8 (a)"), Some("Global goal on adaptation"))
    );

    let reportee = bac.session("654423").await.expect("session reportée");
    assert_eq!(
        (
            reportee.status.as_str(),
            reportee.cancellation_reason.as_deref()
        ),
        ("cancelled", Some("postponed"))
    );
    assert!(!reportee.title_original.contains("POSTPONED"));
    assert!(reportee.cancelled_at.is_some());
    let annulee = bac.session("654417").await.expect("session annulée");
    assert_eq!(annulee.cancellation_reason.as_deref(), Some("source"));
    assert!(annulee.title_original.starts_with("CMA 11 (c)"));

    for (cle, type_code, groupe) in [
        ("651031", "group_coordination", Some("eig")),
        ("654343", "group_coordination", Some("african_group")),
        ("654555", "group_coordination", Some("african_group")),
        ("654012", "group_coordination", Some("grulac")),
        ("652989", "group_coordination", Some("lmdc")),
        ("651738", "group_coordination", None),
        ("654205", "presidency_consultation", None),
        ("654137", "plenary", None),
        ("652229", "contact_group", None),
        ("650428", "mandated_event", None),
        ("654224", "negotiation_other", None),
    ] {
        let s = bac.session(cle).await.expect("session retenue");
        assert_eq!(
            (s.type_code.as_deref(), s.group_code.as_deref()),
            (Some(type_code), groupe),
            "{cle} {}",
            s.title_original
        );
    }

    let etat = bac.etat().await;
    assert_eq!((etat.missed_reads, etat.last_change_count), (0, Some(44)));
    assert!(etat.last_success_at.is_some() && etat.failing_since.is_none());
    assert!(bac.sert().await);
    assert_eq!(
        bac.suivantes().await.len(),
        1,
        "la lecture suivante est posée"
    );
}

#[tokio::test]
async fn une_seconde_lecture_identique_n_ecrit_que_la_derniere_lecture() {
    let bac = Bac::monter().await;
    bac.lire().await;
    let avant = instantane(&bac).await;
    let premiere = bac.session("654006").await.expect("session");

    bac.lire().await;

    assert_eq!(
        bac.journal().await[0],
        ("success".into(), Some(44), Some(0), None)
    );
    assert_eq!(
        bac.nombre(
            "SELECT count(*) FROM negotiation.meeting_changes c
               JOIN negotiation.meetings m ON m.id = c.meeting_id WHERE m.event_id = $1"
        )
        .await,
        0
    );
    assert_eq!(
        instantane(&bac).await,
        avant,
        "aucune autre colonne ne bouge"
    );
    let champs: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT unnest(changed_fields) FROM platform.audit_log
          WHERE entity_schema = 'negotiation' AND entity_table = 'meetings' AND action = 'update'
          ORDER BY 1",
    )
    .fetch_all(bac.pool())
    .await
    .expect("audit");
    assert_eq!(champs, ["last_read_at", "updated_at"]);
    let seconde = bac.session("654006").await.expect("session");
    assert!(seconde.last_read_at > premiere.last_read_at);
    assert_eq!(seconde.first_read_at, premiere.first_read_at);
    assert_eq!(bac.etat().await.last_change_count, Some(0));
}

#[tokio::test]
async fn le_groupe_se_re_resout_a_chaque_lecture_sans_faire_d_ecart() {
    let bac = Bac::monter().await;
    bac.lire().await;

    bac.executer(
        r#"UPDATE reference.taxonomy_terms SET metadata = '{"denominations":["Environmental Integrity Group"]}'
            WHERE taxonomy_code = 'negotiation_group' AND code = 'eig'"#,
    )
    .await;
    bac.lire().await;
    assert_eq!(
        bac.session("651031").await.expect("session").group_code,
        None
    );

    bac.executer(
        r#"UPDATE reference.taxonomy_terms SET metadata = '{"denominations":["EIG"]}'
            WHERE taxonomy_code = 'negotiation_group' AND code = 'eig'"#,
    )
    .await;
    bac.lire().await;
    assert_eq!(
        bac.session("651031")
            .await
            .expect("session")
            .group_code
            .as_deref(),
        Some("eig")
    );
    assert!(bac.changements("651031").await.is_empty());
    assert_eq!(
        bac.journal().await[0].2,
        Some(0),
        "un rattachement n'est pas un écart"
    );
}
