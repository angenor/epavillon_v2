//! **Le chiffre annoncé égale le chiffre réel** — journée et fil (research.md
//! § R8, SC-017).
//!
//! Les deux cas ne détachent pas la même chose, et c'est ce que ce test
//! distingue :
//!
//! - retirer une **journée** détache des séances de leur jour
//!   (`ON DELETE SET NULL`) : elles survivent, sans date de rattachement ;
//! - supprimer un **fil** détruit des **rattachements** séance–fil
//!   (`ON DELETE CASCADE`) : aucune séance n'est supprimée, c'est du travail
//!   éditorial qui est perdu.
//!
//! Dans les deux cas, le décompte se prend **avant** l'ordre : après, le lien
//! n'existe plus et le chiffre rendrait zéro.

mod commun;

use commun::{formulaire_fil, Bac};
use event::domain::ids::{EventId, TrackId};
use event::service::{days as service_journees, tracks as service_fils};

#[tokio::test]
async fn retirer_une_journee_hors_periode_annonce_les_seances_quelle_detache() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;
    let seance = commun::seed::seance(&bac, editions.cop31, &enfants).await;

    // La séance du 10 novembre est rattachée à la journée du semis. On resserre
    // la période pour que cette journée devienne hors bornes.
    sqlx::query!(
        "UPDATE event.events
            SET starts_at = ('2027-11-12 09:00')::timestamp AT TIME ZONE timezone::text
          WHERE id = $1",
        editions.cop31
    )
    .execute(bac.pool())
    .await
    .expect("resserrement de la période");

    let rattachee = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.sessions
            WHERE id = $1 AND event_day_id IS NOT NULL"#,
        seance
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(rattachee, 1, "la séance est bien rattachée à sa journée");

    let resultat =
        service_journees::generer(&bac.state, &bac.ctx(), EventId::from(editions.cop31), true)
            .await
            .expect("la génération avec retrait aboutit");

    assert!(resultat.ok);
    assert_eq!(
        resultat.sessions_detached, 1,
        "une séance portée par la journée retirée"
    );

    let orpheline = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.sessions
            WHERE id = $1 AND event_day_id IS NULL"#,
        seance
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(
        orpheline, 1,
        "le chiffre annoncé égale le chiffre réel : la séance survit, détachée"
    );
}

/// **Sans le drapeau, aucune journée n'est retirée.** Une soirée d'ouverture la
/// veille est un cas légitime, et le choix appartient à l'équipe.
#[tokio::test]
async fn sans_le_drapeau_rien_nest_retire() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    commun::seed::enfants(&bac, editions.cop31).await;

    sqlx::query!(
        "UPDATE event.events
            SET starts_at = ('2027-11-12 09:00')::timestamp AT TIME ZONE timezone::text
          WHERE id = $1",
        editions.cop31
    )
    .execute(bac.pool())
    .await
    .expect("resserrement");

    let resultat =
        service_journees::generer(&bac.state, &bac.ctx(), EventId::from(editions.cop31), false)
            .await
            .expect("génération");

    assert_eq!(resultat.sessions_detached, 0);
    let restantes = commun::journees(&bac, editions.cop31).await;
    assert!(
        restantes.contains(&commun::seed::JOUR_SEANCE),
        "la journée hors période est toujours là"
    );
}

/// **Supprimer un fil ne supprime aucune séance.** Ce qui disparaît, ce sont les
/// rattachements — et c'est ce que le chiffre annonce.
#[tokio::test]
async fn supprimer_un_fil_chiffre_les_rattachements_et_garde_les_seances() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;
    let seance = commun::seed::seance(&bac, editions.cop31, &enfants).await;

    let resultat = service_fils::supprimer(
        &bac.state,
        &bac.ctx(),
        EventId::from(editions.cop31),
        TrackId::from(enfants.fil),
    )
    .await
    .expect("la suppression aboutit");

    assert!(resultat.ok);
    assert_eq!(
        resultat.sessions_detached, 1,
        "un rattachement séance–fil perdu"
    );

    let seances = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.sessions WHERE id = $1"#,
        seance
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(seances, 1, "la séance, elle, n'a pas bougé");

    let rattachements = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.session_tracks WHERE session_id = $1"#,
        seance
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(rattachements, 0, "le chiffre annoncé égale le chiffre réel");
}

/// **Un fil sans rattachement n'annonce rien** — et non une valeur posée au
/// hasard.
#[tokio::test]
async fn supprimer_un_fil_vide_nannonce_aucun_rattachement() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let fil = service_fils::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        formulaire_fil(editions.cop31, "journee_vide"),
    )
    .await
    .expect("création")
    .detail
    .expect("la composition")
    .tracks
    .remove(0);

    let resultat = service_fils::supprimer(&bac.state, &bac.ctx(), cop31, TrackId::from(fil.id))
        .await
        .expect("suppression");

    assert_eq!(resultat.sessions_detached, 0);
}
