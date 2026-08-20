//! Ce que les tests d'intégration du module partagent : une base jetable, un
//! état de module, et de quoi semer des organisations, des comptes et des
//! périmètres d'administration.
//!
//! Aucun double en mémoire (principe X) : chaque test travaille sur une vraie
//! base, recopiée d'un modèle chargé depuis `docs/database/`.

#![allow(dead_code)]

use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::testing::TestDb;
use kernel::Db;
use org::state::OrgState;
use std::sync::Arc;
use uuid::Uuid;

pub mod seed;

pub struct Bac {
    pub base: TestDb,
    pub state: OrgState,
    pub config: Arc<Config>,
}

impl Bac {
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = Arc::new(kernel::testing::test_config(base.url()));
        let state = OrgState::new(base.db(), config.clone());

        Self {
            base,
            state,
            config,
        }
    }

    pub fn db(&self) -> Db {
        self.base.db()
    }

    pub fn ctx(&self) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr")
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        self.base.pool()
    }
}

/// L'IFDD, que `900_seed.sql` sème **avec ses cinq dénominations**. C'est la
/// fiche sur laquelle les cinq façons de désigner une organisation se
/// vérifient sans rien avoir à semer.
pub async fn ifdd(bac: &Bac) -> Uuid {
    sqlx::query_scalar!("SELECT id FROM org.organizations WHERE slug = 'ifdd'")
        .fetch_one(bac.pool())
        .await
        .expect("l'IFDD est semée par 900_seed.sql")
}

/// Un pays du référentiel, par son code ISO à trois lettres.
pub async fn pays(bac: &Bac, iso3: &str) -> Uuid {
    sqlx::query_scalar!("SELECT id FROM reference.countries WHERE iso3 = $1", iso3)
        .fetch_one(bac.pool())
        .await
        .unwrap_or_else(|_| panic!("le pays {iso3} est semé par 020_reference.sql"))
}

/// Une personne, avec son adresse. Le compte mot de passe n'est pas créé : les
/// tests de ce module ne se connectent pas, ils appellent les services.
pub async fn personne(bac: &Bac, email: &str, prenom: &str, nom: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO identity.people
               (primary_email, first_name, last_name, email_verified_at, status)
           VALUES ($1::text::platform.email, $2, $3, now(), 'active')
        RETURNING id"#,
        email,
        prenom,
        nom
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la personne")
}

/// Une personne dont l'adresse **n'est pas** vérifiée : le rattachement
/// automatique ne doit pas jouer pour elle.
pub async fn personne_non_verifiee(bac: &Bac, email: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO identity.people
               (primary_email, first_name, last_name, status)
           VALUES ($1::text::platform.email, 'Sans', 'Preuve', 'active')
        RETURNING id"#,
        email
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la personne")
}

/// Une édition, pour donner une cible aux portées d'événement. Le module
/// `event` n'a pas de crate dans ce jalon : la ligne se sème en SQL.
pub async fn evenement(bac: &Bac, slug: &str, titre: &str, annee: i16) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES ($3, jsonb_build_object('fr', $2::text), $1::text::platform.slug,
                   jsonb_build_object('fr', $2::text), 'online',
                   'America/Belem'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#,
        slug,
        titre,
        annee
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition")
}

/// Une attribution de rôle, avec sa portée. C'est **la portée** qui distingue
/// « administrateur de la plateforme » de « administrateur de la COP31 » : le
/// nom du rôle est le même.
pub async fn attribuer(
    bac: &Bac,
    person_id: Uuid,
    role_code: &str,
    scope_type: &str,
    scope_id: Option<Uuid>,
) {
    sqlx::query!(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
         VALUES ($1, $2, $3::text::identity.scope_type, $4)",
        person_id,
        role_code,
        scope_type,
        scope_id
    )
    .execute(bac.pool())
    .await
    .expect("attribution du rôle");
}

/// Les trois périmètres dont les tests du back-office ont besoin.
pub struct Perimetres {
    /// Administratrice de la plateforme entière.
    pub globale: Uuid,
    /// Administratrice d'une seule édition — celle que rend le second champ.
    pub detachee: Uuid,
    pub edition_detachee: Uuid,
    /// Aucun droit d'administration.
    pub sans_droit: Uuid,
}

pub async fn perimetres(bac: &Bac) -> Perimetres {
    let globale = personne(bac, "globale@ifdd.francophonie.org", "Claire", "Perret").await;
    attribuer(bac, globale, "super_admin", "global", None).await;

    let edition = evenement(bac, "cop31-perimetre", "COP31", 2027).await;
    let detachee = personne(bac, "detachee@ifdd.francophonie.org", "Détachée", "Test").await;
    attribuer(bac, detachee, "admin", "event", Some(edition)).await;

    // Le rôle d'utilisateur ordinaire détient la permission de consultation des
    // organisations, et **aucun périmètre** : c'est ce couple-là qui doit être
    // refusé (écart n° 73).
    let sans_droit = personne(bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;
    attribuer(bac, sans_droit, "standard", "global", None).await;

    Perimetres {
        globale,
        detachee,
        edition_detachee: edition,
        sans_droit,
    }
}

/// Les événements de l'outbox d'un agrégat, dans l'ordre. Le test de fusion les
/// **compte** : vérifier leur présence ne dirait rien d'un doublon.
pub async fn evenements_emis(bac: &Bac, aggregate_id: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT event_type FROM platform.outbox_events
          WHERE aggregate_id = $1 ORDER BY occurred_at, id",
        aggregate_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'outbox")
}

/// Les travaux mis en file, tâche et charge utile.
pub async fn travaux(bac: &Bac) -> Vec<(String, serde_json::Value)> {
    sqlx::query!("SELECT task, payload FROM platform.jobs ORDER BY created_at, id")
        .fetch_all(bac.pool())
        .await
        .expect("lecture de la file")
        .into_iter()
        .map(|l| (l.task, l.payload))
        .collect()
}
