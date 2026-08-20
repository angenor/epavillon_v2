//! **Les six onglets en une réponse, et des décomptes qui tombent juste.**
//!
//! L'écran de détail est une composition, pas douze lectures : l'équipe passe
//! d'un onglet à l'autre sans arrêt en préparant une COP, et attendre un
//! aller-retour à chaque fois est ce que le contrat du front refuse (FR-023).
//!
//! Le second point compte autant que le premier. Les décomptes viennent de
//! **trois schémas** et doivent rester cohérents entre eux : l'onglet des
//! journées ne peut pas annoncer une séance quand celui des salles n'en compte
//! aucune, pour la même édition et au même instant. C'est ce que
//! l'instantané commun de la transaction en lecture seule garantit — et c'est ce
//! que ce fichier vérifie, une séance réelle à l'appui. Sans elle, tous les
//! décomptes vaudraient zéro et le test ne prouverait rien.

mod commun;

use commun::{seed, Bac};
use event::domain::ids::EventId;
use event::service::detail;

#[tokio::test]
async fn les_six_onglets_sont_presents() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;

    let vue = detail::composer(bac.pool(), EventId::from(editions.cop31))
        .await
        .expect("la composition")
        .expect("la COP31 existe");

    assert_eq!(vue.edition.id, editions.cop31);
    assert_eq!(vue.days.len(), 1);
    assert_eq!(vue.tracks.len(), 1);
    assert_eq!(vue.venues.len(), 1);
    assert_eq!(vue.venues[0].rooms.len(), 1);
    // Celui de l'édition, **et** le canal général du semis : l'onglet montre les
    // deux, comme le front les compose déjà.
    assert_eq!(vue.channels.len(), 2);
    assert!(vue.call.is_some());
    assert!(vue.committee.is_empty(), "aucun membre n'a été désigné");

    let appel = vue.call.expect("l'appel");
    assert_eq!(appel.id, enfants.appel);
    assert_eq!(
        appel.criteria.len(),
        6,
        "la grille par défaut du modèle en pose six"
    );
    assert!(appel.is_open, "la fenêtre encadre l'instant courant");
}

/// **Le détail porte ce que la liste ne porte pas** : les deux textes longs, la
/// période en dates civiles, et les trois déclinaisons d'image.
#[tokio::test]
async fn le_detail_porte_les_textes_la_periode_et_les_trois_images() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;

    let vue = detail::composer(bac.pool(), EventId::from(editions.cop31))
        .await
        .expect("la composition")
        .expect("la COP31 existe");

    assert!(vue.description.get("fr").is_some());

    // Belém est trois heures derrière l'UTC : la période va du 9 au 20, et non
    // du 8 au 20. Un décalage ici voudrait dire que le fuseau de l'édition n'a
    // pas été appliqué.
    assert_eq!(vue.period.first_day, time::macros::date!(2027 - 11 - 09));
    assert_eq!(vue.period.last_day, time::macros::date!(2027 - 11 - 20));

    // **Les trois clés sont toujours là**, à `null` tant que rien n'a été
    // téléversé : la boucle d'affichage n'a alors aucune garde à écrire.
    for role in ["banner", "cover", "thumbnail"] {
        assert!(
            vue.images.get(role).is_some(),
            "la déclinaison « {role} » doit être présente, même vide"
        );
        assert!(vue.images[role].is_null());
    }
}

/// **Les décomptes sont cohérents entre eux.** Une séance placée en salle,
/// diffusée et rattachée à un fil doit se retrouver dans les cinq compteurs qui
/// la concernent, et nulle part ailleurs.
#[tokio::test]
async fn une_seance_se_compte_pareil_dans_tous_les_onglets() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    seed::seance(&bac, editions.cop31, &enfants).await;

    let vue = detail::composer(bac.pool(), EventId::from(editions.cop31))
        .await
        .expect("la composition")
        .expect("la COP31 existe");

    assert_eq!(vue.edition.session_count, 1);
    assert_eq!(vue.edition.scheduled_session_count, 1);
    assert_eq!(vue.edition.day_count, vue.days.len() as i64);

    let journee = vue.days.iter().find(|j| j.id == enfants.journee).unwrap();
    assert_eq!(
        journee.session_count, 1,
        "le modèle rattache la séance à la journée de sa date, dans le fuseau de l'édition"
    );

    assert_eq!(vue.tracks[0].session_count, 1);
    assert_eq!(vue.venues[0].rooms[0].session_count, 1);

    let canal = vue.channels.iter().find(|c| c.id == enfants.canal).unwrap();
    assert_eq!(
        canal.session_count, 1,
        "une séance diffusée occupe le canal par défaut de son édition"
    );

    let general = vue.channels.iter().find(|c| c.event_id.is_none()).unwrap();
    assert_eq!(
        general.session_count, 0,
        "le canal général n'est pas celui qui a été occupé"
    );
}

/// **Les dossiers déposés se comptent, les brouillons non** (FR-020). La ligne
/// répond à « combien de dossiers cette édition a-t-elle *reçus* ? », et un
/// brouillon n'a rien été reçu.
#[tokio::test]
async fn les_brouillons_ne_comptent_pas_comme_des_dossiers_recus() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    seed::dossiers(&bac, editions.cop31, enfants.appel).await;

    let vue = detail::composer(bac.pool(), EventId::from(editions.cop31))
        .await
        .expect("la composition")
        .expect("la COP31 existe");

    assert_eq!(
        vue.edition.proposal_count, 1,
        "deux dossiers, un seul déposé"
    );
    assert_eq!(vue.call.expect("l'appel").proposal_count, 1);
}

/// **Le journée hors période est signalée, pas supprimée** (FR-035). Une soirée
/// d'ouverture la veille est un cas légitime, et le choix appartient à l'équipe.
#[tokio::test]
async fn une_journee_hors_periode_est_signalee() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;

    sqlx::query!(
        "INSERT INTO event.event_days (event_id, day_date) VALUES ($1, $2)",
        editions.cop31,
        time::macros::date!(2027 - 11 - 08)
    )
    .execute(bac.pool())
    .await
    .expect("la veille de l'ouverture");

    let vue = detail::composer(bac.pool(), EventId::from(editions.cop31))
        .await
        .expect("la composition")
        .expect("la COP31 existe");

    let veille = &vue.days[0];
    assert_eq!(veille.day_date, time::macros::date!(2027 - 11 - 08));
    assert!(veille.is_outside_period);
}

/// Une édition **inexistante** ne compose rien : c'est ce qui permet à la route
/// de rendre le même refus qu'un hors-périmètre.
#[tokio::test]
async fn une_edition_inexistante_ne_compose_rien() {
    let bac = Bac::monter().await;
    seed::editions(&bac).await;

    let vue = detail::composer(bac.pool(), EventId::from(uuid::Uuid::now_v7()))
        .await
        .expect("la composition");

    assert!(vue.is_none());
}
