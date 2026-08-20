//! **Le plan de génération n'écrit rien** (research.md § R4, SC-012).
//!
//! Rien en base ne dérive les journées d'une édition : `event.event_days` n'a
//! aucun déclencheur de dérivation. La génération est donc un geste
//! d'application — et un geste explicite **s'annonce avant de s'exécuter**.
//!
//! Ce que ce test tient : demander le plan laisse la base **exactement telle
//! qu'elle était**, journée par journée. Et une période d'un an annonce **plus de
//! trois cents journées sans en écrire une** — c'est le cas du cycle de
//! webinaires, celui qui rend l'arbitrage possible plutôt que de le devancer.

mod commun;

use commun::Bac;
use event::domain::ids::EventId;
use event::service::days as service_journees;

#[tokio::test]
async fn demander_le_plan_ne_change_pas_une_ligne() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;
    let _ = enfants;

    let avant = commun::journees_habillees(&bac, editions.cop31).await;

    let plan = service_journees::plan(bac.pool(), EventId::from(editions.cop31))
        .await
        .expect("le plan se lit")
        .expect("l'édition existe");

    assert_eq!(
        plan.to_create.len(),
        11,
        "douze journées du 9 au 20, dont une déjà créée par le semis de test"
    );
    assert_eq!(plan.unchanged, 1);

    let apres = commun::journees_habillees(&bac, editions.cop31).await;
    assert_eq!(
        avant, apres,
        "la base doit être identique avant et après, journée par journée"
    );
}

/// **Une période d'un an annonce plus de trois cents journées sans en écrire
/// une.** C'est le cas du cycle de webinaires, et il est nommé : le plan
/// annonce, il n'impose rien.
#[tokio::test]
async fn une_periode_dun_an_annonce_sans_ecrire() {
    let bac = Bac::monter().await;

    let cycle = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, status, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2027, '{"fr":"Cycle de webinaires"}'::jsonb,
                   'cycle-webinaires-2027'::platform.slug,
                   '{"fr":"Un an de rendez-vous mensuels."}'::jsonb,
                   'announced', 'online', 'Africa/Dakar'::platform.timezone_name,
                   timestamp '2027-01-01 09:00' AT TIME ZONE 'Africa/Dakar',
                   timestamp '2027-12-31 18:00' AT TIME ZONE 'Africa/Dakar')
        RETURNING id"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du cycle");

    let plan = service_journees::plan(bac.pool(), EventId::from(cycle))
        .await
        .expect("le plan se lit")
        .expect("l'édition existe");

    assert_eq!(plan.to_create.len(), 365, "une année pleine");
    assert!(plan.to_create.len() > 300);
    assert_eq!(
        commun::journees(&bac, cycle).await.len(),
        0,
        "et pas une seule journée écrite"
    );
}

/// **Les journées hors période arrivent avec leurs séances.** C'est ce chiffre
/// qui permet à l'équipe d'arbitrer : une soirée d'ouverture la veille est un
/// cas légitime, et rien ne doit la retirer d'office (FR-035).
#[tokio::test]
async fn les_journees_hors_periode_arrivent_avec_leurs_seances() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;
    commun::seed::seance(&bac, editions.cop31, &enfants).await;

    // Une journée la veille de l'ouverture : hors période, et légitime.
    sqlx::query!(
        "INSERT INTO event.event_days (event_id, day_date, sort_order)
         VALUES ($1, DATE '2027-11-08', 0)",
        editions.cop31
    )
    .execute(bac.pool())
    .await
    .expect("insertion de la veille");

    let plan = service_journees::plan(bac.pool(), EventId::from(editions.cop31))
        .await
        .expect("le plan se lit")
        .expect("l'édition existe");

    assert_eq!(plan.to_review.len(), 1, "la veille est signalée");
    assert_eq!(
        plan.to_review[0].day_date,
        time::macros::date!(2027 - 11 - 08)
    );
    assert_eq!(
        plan.to_review[0].session_count, 0,
        "aucune séance ce jour-là : rien à perdre en la retirant"
    );

    assert_eq!(
        commun::journees(&bac, editions.cop31).await.len(),
        2,
        "la veille et la journée du semis : le plan n'a rien retiré"
    );
}
