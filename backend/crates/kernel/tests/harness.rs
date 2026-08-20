//! Le harnais de test se teste lui-même : sans lui, aucun autre test du projet
//! ne prouve rien.

use kernel::testing::TestDb;
use sqlx::postgres::PgConnection;
use sqlx::Connection;

const SCHEMAS_ATTENDUS: [&str; 16] = [
    "platform",
    "reference",
    "identity",
    "org",
    "event",
    "programme",
    "live",
    "publication",
    "negotiation",
    "engagement",
    "media",
    "tool",
    "content",
    "training",
    "analytics",
    "legacy",
];

#[tokio::test]
async fn une_base_jetable_porte_les_seize_schemas_et_disparait_en_sortant() {
    let nom;
    let url_admin;

    {
        let base = TestDb::new().await;
        nom = base.name().to_owned();

        let presents: Vec<String> = sqlx::query_scalar(
            "SELECT schema_name FROM information_schema.schemata WHERE schema_name = ANY($1)",
        )
        .bind(&SCHEMAS_ATTENDUS[..])
        .fetch_all(base.pool())
        .await
        .expect("lecture des schémas");

        assert_eq!(
            presents.len(),
            SCHEMAS_ATTENDUS.len(),
            "chargement du modèle incomplet : {presents:?}"
        );

        // Les quatre étiquettes que le noyau écrit à la main doivent rester
        // celles de l'ENUM : un renommage côté SQL passerait la compilation et
        // n'échouerait qu'à l'exécution, en production.
        let portees: Vec<String> =
            sqlx::query_scalar("SELECT unnest(enum_range(NULL::identity.scope_type))::text")
                .fetch_all(base.pool())
                .await
                .expect("valeurs de identity.scope_type");
        assert_eq!(
            portees,
            vec!["global", "organization", "event", "negotiation_space"],
            "identity.scope_type a changé : kernel::auth::ScopeType est à reprendre"
        );

        // Le modèle n'est pas seulement présent, il est vivant : la fonction
        // d'autorisation répond.
        let inconnu: bool = sqlx::query_scalar(
            "SELECT identity.has_permission(gen_random_uuid(), 'programme.proposal.read_all')",
        )
        .fetch_one(base.pool())
        .await
        .expect("identity.has_permission");
        assert!(!inconnu);

        url_admin = base.admin_url().to_owned();
    }

    let mut admin = PgConnection::connect(&url_admin)
        .await
        .expect("connexion d'administration");
    let reste: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(&nom)
            .fetch_one(&mut admin)
            .await
            .expect("recherche de la base jetable");

    assert!(!reste, "la base jetable {nom} a survécu au test");
}
