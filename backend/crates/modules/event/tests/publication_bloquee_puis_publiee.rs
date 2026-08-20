//! **Le seul contrôle bloquant du module** (research.md § R10, SC-019).
//!
//! Partout ailleurs, le système détecte et signale sans refuser : les
//! chevauchements de créneaux s'écrivent librement, l'équipe arbitre (règle
//! métier n° 2). Ici, et ici seulement, un point de gravité bloquante retient
//! l'écriture — parce qu'une programmation rendue publique avec deux activités
//! dans la même salle au même moment n'est pas un brouillon de travail, c'est
//! une information fausse donnée au public.
//!
//! Le parcours éprouvé est celui de l'histoire : voir la liste **avant**
//! d'essayer, se voir refuser, lever le conflit, publier. Et **exactement un**
//! événement à l'arrivée : c'est le décompte, et non la présence, qui dit
//! quelque chose d'un doublon.

mod commun;

use commun::Bac;
use event::domain::ids::EventId;
use event::service::publication;
use uuid::Uuid;

/// Deux séances qui occupent la **même salle au même créneau** : le conflit de
/// gravité haute que `programme.detect_conflicts()` nomme, et que
/// `publication_readiness()` reprend.
async fn deux_seances_en_conflit(bac: &Bac, event_id: Uuid, room_id: Uuid) -> (Uuid, Uuid) {
    let mut ids = Vec::new();

    for slug in ["premiere-table-ronde", "seconde-table-ronde"] {
        let id = sqlx::query_scalar!(
            r#"INSERT INTO programme.sessions
                   (event_id, title, slug, format, starts_at, ends_at, timezone,
                    room_id, status, location_note)
               VALUES ($1, '{"fr":"Table ronde"}'::jsonb, $2::text::platform.slug, 'in_person',
                       ('2027-11-10 14:00')::timestamp AT TIME ZONE $3,
                       ('2027-11-10 15:30')::timestamp AT TIME ZONE $3,
                       $3::text::platform.timezone_name, $4, 'scheduled',
                       '{"fr":"Stand principal"}'::jsonb)
            RETURNING id"#,
            event_id,
            slug,
            commun::seed::FUSEAU_COP31,
            room_id
        )
        .fetch_one(bac.pool())
        .await
        .expect("insertion de la séance");

        ids.push(id);
    }

    (ids[0], ids[1])
}

#[tokio::test]
async fn le_conflit_bloque_puis_la_publication_aboutit() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;
    let cop31 = EventId::from(editions.cop31);

    let (_, seconde) = deux_seances_en_conflit(&bac, editions.cop31, enfants.salle).await;

    // **La liste se consulte AVANT d'essayer.**
    let points = publication::controle(bac.pool(), cop31)
        .await
        .expect("le contrôle préalable se lit");
    assert!(
        points.iter().any(|p| p.severity == "blocking"),
        "deux séances dans la même salle au même créneau : {points:?}"
    );

    let refus = publication::publier(&bac.state, &bac.ctx(), cop31)
        .await
        .expect("un refus de publication est une réponse, pas une erreur");

    assert!(refus.blocked, "un point bloquant retient la publication");
    assert_eq!(refus.published_count, 0);
    assert!(refus.published_at.is_none());
    assert!(!refus.issues.is_empty(), "la liste dit quoi régler");

    // **Rien n'a été écrit.**
    assert!(
        commun::evenements_emis(&bac, editions.cop31)
            .await
            .is_empty(),
        "un refus n'annonce rien"
    );
    let date = sqlx::query_scalar!(
        "SELECT programme_published_at FROM event.events WHERE id = $1",
        editions.cop31
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert!(date.is_none(), "l'édition n'est pas estampillée");

    // On lève le conflit en décalant la seconde séance.
    sqlx::query!(
        "UPDATE programme.sessions
            SET starts_at = starts_at + interval '2 hours',
                ends_at   = ends_at   + interval '2 hours'
          WHERE id = $1",
        seconde
    )
    .execute(bac.pool())
    .await
    .expect("décalage");

    let publiee = publication::publier(&bac.state, &bac.ctx(), cop31)
        .await
        .expect("la publication aboutit");

    assert!(!publiee.blocked);
    assert_eq!(
        publiee.published_count, 2,
        "les deux séances sont désignées par le prédicat"
    );
    assert!(publiee.published_at.is_some(), "la date est posée");

    let evenements = commun::evenements_emis(&bac, editions.cop31).await;
    assert_eq!(
        evenements,
        vec!["event.programme.published".to_owned()],
        "exactement un événement"
    );
}

/// **Un avertissement ne retient pas.** Une séance sans intervenant déclaré
/// mérite d'être signalée ; interdire tout un programme pour cela ferait de
/// l'avertissement un refus, et l'équipe cesserait de le lire.
#[tokio::test]
async fn un_avertissement_accompagne_sans_retenir() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;
    let cop31 = EventId::from(editions.cop31);

    // Une séance placée, sans intervenant : c'est un avertissement du modèle.
    commun::seed::seance(&bac, editions.cop31, &enfants).await;

    let points = publication::controle(bac.pool(), cop31)
        .await
        .expect("contrôle");
    assert!(
        points.iter().any(|p| p.severity == "warning"),
        "une séance sans intervenant est signalée : {points:?}"
    );
    assert!(
        !points.iter().any(|p| p.severity == "blocking"),
        "et rien ne bloque : {points:?}"
    );

    let publiee = publication::publier(&bac.state, &bac.ctx(), cop31)
        .await
        .expect("publication");

    assert!(!publiee.blocked);
    assert!(
        !publiee.issues.is_empty(),
        "les avertissements accompagnent la réponse"
    );
}

/// **`occurs_at` est un instant**, pas un intervalle mis en forme. Une chaîne
/// figée en base ne pourrait ni se traduire ni se situer dans le fuseau de
/// l'édition, alors que la règle du projet l'exige de toute date affichée.
#[tokio::test]
async fn les_points_portent_un_instant_et_non_un_texte() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;

    deux_seances_en_conflit(&bac, editions.cop31, enfants.salle).await;

    let points = publication::controle(bac.pool(), EventId::from(editions.cop31))
        .await
        .expect("contrôle");

    let rendu = serde_json::to_string(&points).expect("sérialisation");
    assert!(
        !rendu.contains("[\\\"2027"),
        "aucun intervalle brut ne doit franchir la réponse : {rendu}"
    );
    assert!(
        points.iter().any(|p| p.occurs_at.is_some()),
        "un point de conflit situe son moment"
    );
}
