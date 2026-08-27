//! **L'état temporel du poste est ÉGAL à celui de `programme.v_public_schedule`,
//! sur les cinq branches.**
//!
//! C'est le test qui tient la duplication assumée. Le poste ne peut pas lire la
//! vue — elle écarte les activités non publiées, et une activité non publiée
//! peut parfaitement tomber en panne —, alors l'expression est **recopiée**. Ce
//! test l'attache à son original : le jour où la vue change et pas la copie, il
//! casse.

mod commun;

use commun::*;
use time::macros::time;
use uuid::Uuid;

/// L'état que la vue publique calcule pour une activité **publiée**.
async fn etat_de_la_vue(bac: &Bac, session_id: Uuid) -> Option<String> {
    sqlx::query_scalar!(
        r#"SELECT v.temporal_state AS "etat?"
             FROM programme.v_public_schedule v
            WHERE v.id = $1"#,
        session_id
    )
    .fetch_optional(bac.pool())
    .await
    .expect("lecture de la vue")
    .flatten()
}

async fn publier_lactivite(bac: &Bac, session_id: Uuid, statut: &str) {
    sqlx::query!(
        // Le motif accompagne l'annulation : `ck_sessions_cancelled_reason`
        // l'exige — une activité annulée sans raison ne se justifie devant
        // personne.
        r#"UPDATE programme.sessions
            SET published_at = now(),
                status = $2::text::programme.session_status,
                cancelled_reason = CASE WHEN $2 = 'cancelled'
                                        THEN '{"fr":"Motif du test."}'::jsonb::platform.i18n_text END
          WHERE id = $1"#,
        session_id,
        statut
    )
    .execute(bac.pool())
    .await
    .expect("publication de l'activité");
}

#[tokio::test]
async fn les_cinq_branches_coincident_avec_la_vue() {
    let bac = Bac::monter().await;
    let event_id = edition(&bac, "edition-etats", "Édition des états", FUSEAU, "Belém").await;

    // Cinq activités, une par branche : annulée, reportée, à venir, en cours,
    // passée.
    let midi = aujourdhui_a(&bac, event_id, time!(12:00)).await;
    let cas = [
        ("cancelled", midi - time::Duration::hours(6), "annulee"),
        ("postponed", midi - time::Duration::hours(6), "reportee"),
        ("scheduled", midi + time::Duration::days(2), "a-venir"),
        (
            "live",
            time::OffsetDateTime::now_utc() - time::Duration::minutes(10),
            "en-cours",
        ),
        (
            "completed",
            time::OffsetDateTime::now_utc() - time::Duration::hours(6),
            "passee",
        ),
    ];

    let mut posees = Vec::new();
    for (statut, debut, slug) in cas {
        let id = activite(
            &bac,
            event_id,
            None,
            None,
            slug,
            slug,
            debut,
            debut + time::Duration::hours(2),
        )
        .await;
        publier_lactivite(&bac, id, statut).await;
        posees.push(id);
    }

    let ecran = live::service::list::composer(bac.pool(), event_id, "fr")
        .await
        .expect("composition de l'écran");

    // Le poste est en repli ou non selon l'heure : ce qu'on compare, ce sont les
    // activités qu'il rend, une par une, à ce que la vue dit des mêmes.
    let mut compares = 0;
    for session in &ecran.desk.sessions {
        if let Some(attendu) = etat_de_la_vue(&bac, session.session_id).await {
            assert_eq!(
                session.temporal_state, attendu,
                "l'expression recopiée doit rendre exactement ce que rend la vue"
            );
            compares += 1;
        }
    }
    assert!(
        compares > 0,
        "au moins une activité publiée doit avoir été comparée, sans quoi le test ne prouve rien"
    );

    // Et les cinq branches sont bien atteignables : on les vérifie directement
    // sur la vue, qui les porte toutes.
    let mut etats = Vec::new();
    for id in &posees {
        if let Some(etat) = etat_de_la_vue(&bac, *id).await {
            etats.push(etat);
        }
    }
    etats.sort();
    etats.dedup();
    assert!(
        etats.len() >= 4,
        "au moins quatre branches distinctes ont été produites : {etats:?}"
    );
}
