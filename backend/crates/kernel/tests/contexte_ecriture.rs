//! Principe VII : une écriture sans contexte **n'échoue pas**, elle écrit une
//! trace anonyme et rien ne le signale. C'est le seul défaut du socle qu'aucun
//! mécanisme ne rattrape — d'où ce test, qui le cherche.

use kernel::testing::TestDb;
use kernel::RequestContext;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
async fn une_transaction_du_noyau_pose_lacteur_et_la_requete() {
    let base = TestDb::new().await;
    let db = base.db();

    let acteur = creer_personne(&db, None, "auteur@example.org").await;

    let ctx = RequestContext::new("test-contexte-ecriture", "fr").with_actor(acteur);
    let sujet = creer_personne(&db, Some(&ctx), "sujet@example.org").await;

    let trace = sqlx::query!(
        "SELECT actor_id, request_id
           FROM platform.audit_log
          WHERE entity_schema = 'identity' AND entity_table = 'people' AND entity_id = $1
          ORDER BY occurred_at DESC
          LIMIT 1",
        sujet
    )
    .fetch_one(base.pool())
    .await
    .expect("ligne d'audit");

    assert_eq!(trace.actor_id, Some(acteur));
    assert_eq!(trace.request_id.as_deref(), Some("test-contexte-ecriture"));
}

#[tokio::test]
async fn le_contexte_ne_franchit_pas_la_transaction() {
    let base = TestDb::new().await;

    // UNE seule connexion : sans cela, la relecture pourrait tomber sur une
    // autre session, où `current_setting` rend NULL de toute façon — et
    // l'assertion passerait au vert même si le contexte fuyait.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(base.url())
        .await
        .expect("pool à connexion unique");
    let db = kernel::Db::from_pool(pool.clone());

    let ctx = RequestContext::new("test-set-local", "fr").with_actor(Uuid::now_v7());
    let tx = db.write(&ctx).await.expect("transaction");
    tx.commit().await.expect("validation");

    // `set_config(..., true)` est borné à la transaction : hors d'elle, la
    // valeur ne doit plus rien porter.
    let apres: Option<String> = sqlx::query_scalar("SELECT platform.current_request_id()")
        .fetch_one(&pool)
        .await
        .expect("lecture hors transaction");

    assert_eq!(apres, None);
}

async fn creer_personne(db: &kernel::Db, ctx: Option<&RequestContext>, email: &str) -> Uuid {
    let neutre = RequestContext::new("amorcage", "fr");
    let mut tx = db.write(ctx.unwrap_or(&neutre)).await.expect("transaction");

    let id = sqlx::query_scalar!(
        "INSERT INTO identity.people (primary_email, first_name, last_name)
         VALUES ($1, 'Awa', 'Diallo')
         RETURNING id",
        email as _
    )
    .fetch_one(&mut *tx)
    .await
    .expect("insertion d'une personne");

    tx.commit().await.expect("validation");
    id
}
