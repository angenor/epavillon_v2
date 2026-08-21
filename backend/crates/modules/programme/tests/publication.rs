//! **La programmation devient réellement publique** — l'obligation inscrite aux
//! points bloqués le 20/08.
//!
//! B3 contrôle, estampille l'édition et **annonce** ; ce module reçoit et rend
//! publiques les séances désignées. C'est le premier consommateur d'outbox du
//! dépôt, et la garde de rejeu est celle du noyau : le relais réserve
//! `(consommateur, événement)` **avant** d'appeler le consommateur.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::context::RequestContext;
use kernel::events::{self, ConsumerRegistry, DomainEvent, OutboxEvent};
use programme::domain::transitions::ProposalStatus;
use programme::service::transition;
use uuid::Uuid;

async fn seance(bac: &Bac, terrain: &Terrain, titre: &str, slug: &str) -> Uuid {
    let dossier = seances::dossier_pret(bac, terrain, titre, slug, Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id
}

/// L'annonce **telle que B3 l'émet** : même type d'événement, même charge utile,
/// même prédicat. Recomposer autre chose ici ne prouverait rien.
async fn annoncer(bac: &Bac, event_id: Uuid) -> (Uuid, time::OffsetDateTime) {
    let published_at = time::OffsetDateTime::now_utc();

    let charge = serde_json::to_value(contracts::event::ProgrammePublished {
        event_id,
        published_at,
        selection: contracts::event::SessionSelection {
            event_id,
            statuses: vec!["planned".to_owned(), "scheduled".to_owned()],
            only_unpublished: true,
        },
        published_count: 0,
    })
    .expect("charge utile");

    let mut tx = bac
        .db()
        .write(&RequestContext::background("test"))
        .await
        .expect("transaction");

    let id = events::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: contracts::event::AGGREGATE_SCHEMA,
            aggregate_type: contracts::event::AGGREGATE_PROGRAMME,
            aggregate_id: event_id,
            event_type: contracts::event::PROGRAMME_PUBLISHED,
            payload: charge,
        },
    )
    .await
    .expect("annonce émise");

    tx.commit().await.expect("validation");
    (id, published_at)
}

/// Un tour de relais, réduit à ce qui compte : **réserver**, produire l'effet,
/// marquer publié. C'est la garde du noyau qui rend une seconde livraison sans
/// effet, et non un code de ce module.
async fn relayer(bac: &Bac, registre: &ConsumerRegistry, event_id: Uuid) {
    let mut tx = bac
        .db()
        .write(&RequestContext::background("outbox"))
        .await
        .expect("transaction");

    let ligne = sqlx::query!(
        "SELECT aggregate_schema, aggregate_type, aggregate_id, event_type, event_version,
                payload, metadata, correlation_id, occurred_at
           FROM platform.outbox_events WHERE id = $1",
        event_id
    )
    .fetch_one(&mut *tx)
    .await
    .expect("relecture");

    let evenement = OutboxEvent {
        id: event_id,
        aggregate_schema: ligne.aggregate_schema,
        aggregate_type: ligne.aggregate_type,
        aggregate_id: ligne.aggregate_id,
        event_type: ligne.event_type,
        event_version: ligne.event_version,
        payload: ligne.payload,
        metadata: ligne.metadata,
        correlation_id: ligne.correlation_id,
        occurred_at: ligne.occurred_at,
    };

    for consommateur in registre.interested(&evenement.event_type) {
        if events::claim(&mut tx, consommateur.name(), evenement.id)
            .await
            .expect("réservation")
        {
            consommateur
                .handle(&mut tx, &evenement)
                .await
                .expect("effet du consommateur");
        }
    }

    tx.commit().await.expect("validation");
}

fn registre() -> ConsumerRegistry {
    ConsumerRegistry::new().register_all(programme::event_consumers())
}

/// Les séances aux **états portés par l'annonce**, **non encore publiques**,
/// reçoivent la date **de l'annonce** — et une séance d'une autre édition n'est
/// pas touchée.
#[tokio::test]
async fn les_seances_designees_recoivent_la_date_de_lannonce() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let ici = seance(&bac, &terrain, "Ici", "ici").await;

    // Une édition voisine, avec sa séance : elle ne doit pas bouger.
    let autre = commun::edition_secondaire(&bac).await;
    let terrain_voisin = commun::Terrain {
        edition: autre,
        appel: commun::appel_ouvert(&bac, autre).await,
        organisation: commun::organisation_verifiee(&bac, "Voisine", "VSN").await,
        deposante: commun::personne(&bac, "voisine@example.org", "Vera", "Sow").await,
    };
    commun::adherer(
        &bac,
        terrain_voisin.organisation,
        terrain_voisin.deposante,
        "active",
    )
    .await;
    let ailleurs = {
        let dossier = seances::dossier_pret(
            &bac,
            &terrain_voisin,
            "Ailleurs",
            "ailleurs",
            Souhaits {
                creneau: Some("2027-03-02 10:00"),
                ..Souhaits::default()
            },
        )
        .await;
        transition::tenter(
            &bac.state,
            &bac.ctx(),
            dossier.id.into(),
            ProposalStatus::Accepted,
            None,
        )
        .await
        .unwrap();
        seances::seances_du_dossier(&bac, dossier.id)
            .await
            .remove(0)
            .id
    };

    let (annonce, published_at) = annoncer(&bac, terrain.edition).await;
    relayer(&bac, &registre(), annonce).await;

    let publiee = seances::seance(&bac, ici).await;
    assert_eq!(
        publiee.published_at,
        Some(published_at),
        "la date posée est celle de l'annonce, jamais l'instant du traitement"
    );

    let intacte = seances::seance(&bac, ailleurs).await;
    assert_eq!(
        intacte.published_at, None,
        "l'autre édition n'est pas touchée"
    );
}

/// 🔴 **Les séances « pressenties » passent à « programmées »**, les déjà
/// programmées ne bougent pas, et **le nombre d'événements émis est celui des
/// seules séances dont l'état a changé** : le déclencheur trie lui-même.
#[tokio::test]
async fn la_publication_fait_passer_pressenti_a_programme() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let pressentie = seance(&bac, &terrain, "Pressentie", "pressentie").await;
    let deja = seance(&bac, &terrain, "Déjà programmée", "deja-programmee").await;

    sqlx::query!(
        "UPDATE programme.sessions SET status = 'scheduled' WHERE id = $1",
        deja
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let evenements_avant = evenements_de(&bac, deja).await.len();

    let (annonce, _) = annoncer(&bac, terrain.edition).await;
    relayer(&bac, &registre(), annonce).await;

    assert_eq!(seances::seance(&bac, pressentie).await.status, "scheduled");
    assert_eq!(seances::seance(&bac, deja).await.status, "scheduled");

    let emis_pressentie = evenements_de(&bac, pressentie).await;
    assert!(
        emis_pressentie.contains(&"programme.session.scheduled".to_owned()),
        "l'état a changé : le signal dont B6 a besoin pour les rappels"
    );

    assert_eq!(
        evenements_de(&bac, deja).await.len(),
        evenements_avant,
        "une séance simplement rendue publique n'émet rien"
    );
}

/// **La même annonce livrée deux fois ne publie aucune séance de plus**, et le
/// registre d'entrée porte **une** ligne. La garde vient du noyau.
#[tokio::test]
async fn une_annonce_rejouee_est_sans_effet() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    let (annonce, published_at) = annoncer(&bac, terrain.edition).await;
    let registre = registre();

    relayer(&bac, &registre, annonce).await;
    relayer(&bac, &registre, annonce).await;

    assert_eq!(
        seances::seance(&bac, id).await.published_at,
        Some(published_at),
        "la date n'a pas été réécrite"
    );

    let reservations = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.inbox_events
            WHERE event_id = $1 AND consumer = 'programme.publication'"#,
        annonce
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(reservations, 1, "une seule réservation");
}

/// **Le module n'écrit pas `event.events.programme_published_at`** : elle est
/// posée par l'émetteur, et écrire hors de son schéma dans un module métier est
/// interdit.
#[tokio::test]
async fn le_module_necrit_pas_la_date_de_ledition() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    seance(&bac, &terrain, "Atelier", "atelier").await;

    let avant = date_de_publication(&bac, terrain.edition).await;
    let (annonce, _) = annoncer(&bac, terrain.edition).await;
    relayer(&bac, &registre(), annonce).await;
    let apres = date_de_publication(&bac, terrain.edition).await;

    assert_eq!(avant, apres, "la date de l'édition appartient à l'émetteur");
    assert_eq!(apres, None, "l'annonce a été forgée sans estampiller");
}

/// Une édition **sans aucune séance** se publie sans erreur et sans rien
/// publier ; une séance **annulée** reste visible avec son état — elle n'est pas
/// dans les états portés par l'annonce.
#[tokio::test]
async fn une_edition_vide_et_une_seance_annulee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let (vide, _) = annoncer(&bac, terrain.edition).await;
    relayer(&bac, &registre(), vide).await;

    let annulee = seance(&bac, &terrain, "Annulée", "annulee").await;
    sqlx::query!(
        r#"UPDATE programme.sessions
              SET status = 'cancelled',
                  cancelled_reason = '{"fr":"Reportée à l''an prochain."}'::jsonb
            WHERE id = $1"#,
        annulee
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let (annonce, _) = annoncer(&bac, terrain.edition).await;
    relayer(&bac, &registre(), annonce).await;

    let relue = seances::seance(&bac, annulee).await;
    assert_eq!(relue.status, "cancelled", "l'état est conservé");
    assert_eq!(
        relue.published_at, None,
        "une séance annulée n'entre pas dans les états portés par l'annonce"
    );
}

async fn evenements_de(bac: &Bac, session_id: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT event_type FROM platform.outbox_events
          WHERE aggregate_type = 'session' AND aggregate_id = $1
          ORDER BY occurred_at, id",
        session_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'outbox")
}

async fn date_de_publication(bac: &Bac, event_id: Uuid) -> Option<time::OffsetDateTime> {
    sqlx::query_scalar!(
        "SELECT programme_published_at FROM event.events WHERE id = $1",
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'édition")
}
