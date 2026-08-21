//! 🔴 **De bout en bout : publier le programme, et le voir devenir public.**
//!
//! C'est l'obligation inscrite aux points bloqués le 20/08 : B3 estampille
//! l'édition et **annonce**, B5 reçoit et rend publiques les séances désignées.
//! Aucun test de module ne peut le prouver — ils vivent chacun d'un côté de la
//! frontière, et c'est précisément la frontière qu'on éprouve ici.
//!
//! Le symptôme redouté est « publié, mais rien de public » : l'édition
//! estampillée, l'annonce partie, et aucune séance visible parce que personne ne
//! l'a consommée. Ce test le reproduit — worker arrêté — puis le fait disparaître.
//!
//! **L'écart entre le nombre annoncé et l'effet est MESURÉ**, jamais supposé
//! nul : l'émetteur compte sous l'instantané de sa transaction, le consommateur
//! applique le prédicat à T + ε.

use kernel::context::RequestContext;
use kernel::events::{self, ConsumerRegistry, OutboxEvent};
use kernel::testing::TestDb;
use uuid::Uuid;

/// Une édition avec trois séances **éligibles** — deux pressenties, une déjà
/// programmée — et une quatrième annulée, que le prédicat ne vise pas.
async fn une_edition_a_publier(base: &TestDb) -> (Uuid, Vec<Uuid>) {
    let edition = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_label, edition_year, title, slug, description, status,
                participation_mode, timezone, starts_at, ends_at, country_id, city)
           VALUES ('COP31', 2027, '{"fr":"COP31"}'::jsonb,
                   'cop31-bout-en-bout'::platform.slug,
                   '{"fr":"Pavillon."}'::jsonb, 'announced', 'hybrid',
                   'America/Belem'::platform.timezone_name,
                   timestamp '2027-11-09 09:00' AT TIME ZONE 'America/Belem',
                   timestamp '2027-11-20 18:00' AT TIME ZONE 'America/Belem',
                   (SELECT id FROM reference.countries WHERE iso3 = 'BRA'), 'Belém')
        RETURNING id"#
    )
    .fetch_one(base.pool())
    .await
    .expect("édition");

    let mut eligibles = Vec::new();
    for (rang, etat) in [(1, "planned"), (2, "planned"), (3, "scheduled")] {
        eligibles.push(seance(base, edition, rang, etat, false).await);
    }
    // Une séance annulée : elle n'est pas dans les états portés par l'annonce.
    seance(base, edition, 4, "cancelled", false).await;

    (edition, eligibles)
}

/// Une séance, avec **une précision de lieu** : sans elle, le contrôle préalable
/// de B3 la réclamerait et retiendrait toute la publication.
async fn seance(base: &TestDb, edition: Uuid, rang: i32, etat: &str, publiee: bool) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, format, timezone, starts_at, ends_at,
                status, location_note, cancelled_reason, published_at)
           VALUES ($1, jsonb_build_object('fr', 'Séance ' || $2::int4::text),
                   ('seance-' || $2::int4::text)::platform.slug, 'hybrid',
                   'America/Belem'::platform.timezone_name,
                   (timestamp '2027-11-12 09:00' AT TIME ZONE 'America/Belem')
                       + make_interval(hours => $2::int4),
                   (timestamp '2027-11-12 10:00' AT TIME ZONE 'America/Belem')
                       + make_interval(hours => $2::int4),
                   $3::text::programme.session_status,
                   '{"fr":"En ligne, lien communiqué aux inscrits."}'::jsonb,
                   CASE WHEN $3 = 'cancelled'
                        THEN '{"fr":"Annulée."}'::jsonb END,
                   CASE WHEN $4 THEN now() END)
        RETURNING id"#,
        edition,
        rang,
        etat,
        publiee
    )
    .fetch_one(base.pool())
    .await
    .expect("séance")
}

/// Un tour de relais — **exactement ce que fait le worker** : réserver, produire
/// l'effet, marquer publié.
async fn relayer_loutbox(base: &TestDb, registre: &ConsumerRegistry) -> usize {
    let db = base.db();

    let candidats: Vec<Uuid> = sqlx::query_scalar!(
        "SELECT id FROM platform.outbox_events
          WHERE published_at IS NULL ORDER BY occurred_at, id"
    )
    .fetch_all(base.pool())
    .await
    .expect("candidats");

    let mut relayes = 0;
    for id in candidats {
        let mut tx = db
            .write(&RequestContext::background("outbox"))
            .await
            .expect("transaction");

        let ligne = sqlx::query!(
            "SELECT aggregate_schema, aggregate_type, aggregate_id, event_type,
                    event_version, payload, metadata, correlation_id, occurred_at
               FROM platform.outbox_events WHERE id = $1",
            id
        )
        .fetch_one(&mut *tx)
        .await
        .expect("relecture");

        let evenement = OutboxEvent {
            id,
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

        sqlx::query!(
            "UPDATE platform.outbox_events SET published_at = now() WHERE id = $1",
            id
        )
        .execute(&mut *tx)
        .await
        .expect("marquage");

        tx.commit().await.expect("validation");
        relayes += 1;
    }

    relayes
}

async fn seances_publiques(base: &TestDb, edition: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.sessions
            WHERE event_id = $1 AND published_at IS NOT NULL"#,
        edition
    )
    .fetch_one(base.pool())
    .await
    .expect("décompte")
}

#[actix_web::test]
async fn publier_le_programme_rend_les_seances_publiques() {
    let base = TestDb::new().await;
    let config = std::sync::Arc::new(kernel::testing::test_config(base.url()));
    let (edition, eligibles) = une_edition_a_publier(&base).await;

    let etat = event::state::EventState::new(base.db(), config.clone());
    let ctx = RequestContext::new("test-publication", "fr");

    // 1. **B3 publie** : contrôle, estampille, annonce.
    let resultat =
        event::service::publication::publier(&etat, &ctx, event::domain::ids::EventId(edition))
            .await
            .expect("la publication aboutit");

    assert!(
        !resultat.blocked,
        "aucun conflit bloquant : {:?}",
        resultat.issues
    );
    assert_eq!(
        resultat.published_count,
        eligibles.len() as i64,
        "trois séances désignées"
    );
    assert!(resultat.published_at.is_some(), "l'édition est estampillée");

    // 2. 🔴 **Worker arrêté : le symptôme se reproduit.** L'édition est publiée
    // et rien n'est public — c'est exactement ce que ce jalon vient corriger.
    assert_eq!(
        seances_publiques(&base, edition).await,
        0,
        "publié, mais rien de public : le symptôme, tant que personne ne consomme"
    );

    // 3. **Le relais travaille**, et le symptôme disparaît.
    let registre = ConsumerRegistry::new().register_all(programme::event_consumers());
    relayer_loutbox(&base, &registre).await;

    let devenues_publiques = seances_publiques(&base, edition).await;

    // 4. **L'égalité est attendue, et l'écart serait mesuré** : entre l'annonce
    // et l'effet, seule la naissance d'une séance peut faire diverger les deux
    // nombres — l'effet peut dépasser l'annonce, jamais l'inverse. Aucun dossier
    // n'est retenu pendant ce test, donc l'égalité tient.
    assert_eq!(
        devenues_publiques, resultat.published_count,
        "le nombre annoncé et le nombre devenu public"
    );

    // 5. Et les états ont bougé : « pressenti » devient « programmé ».
    let programmees = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.sessions
            WHERE event_id = $1 AND status = 'scheduled'"#,
        edition
    )
    .fetch_one(base.pool())
    .await
    .unwrap();
    assert_eq!(
        programmees, 3,
        "les deux pressenties ont rejoint la troisième"
    );

    // 6. **Republier est inoffensif**, et le relais n'a rien de plus à faire.
    let seconde =
        event::service::publication::publier(&etat, &ctx, event::domain::ids::EventId(edition))
            .await
            .expect("republier n'échoue pas");
    assert_eq!(seconde.published_count, 0, "aucune séance de plus annoncée");

    relayer_loutbox(&base, &registre).await;
    assert_eq!(
        seances_publiques(&base, edition).await,
        devenues_publiques,
        "et aucune séance de plus publiée"
    );
}
