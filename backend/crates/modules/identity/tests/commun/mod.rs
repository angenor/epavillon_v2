//! Ce que les tests d'intégration du module partagent : une base jetable, un
//! état de module, et de quoi semer des comptes couvrant chaque cas.
//!
//! Aucun double en mémoire (principe X) : chaque test travaille sur une vraie
//! base, recopiée d'un modèle chargé depuis `docs/database/`.

#![allow(dead_code)]

use identity::repo::sessions::ClientKind;
use identity::state::IdentityState;
use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::crypto::Passwords;
use kernel::testing::TestDb;
use kernel::Db;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

/// Conforme aux trois exigences : huit signes, une majuscule, une minuscule.
pub const MOT_DE_PASSE: &str = "Belem2027!";

pub struct Bac {
    pub base: TestDb,
    pub state: IdentityState,
    pub config: Arc<Config>,
}

impl Bac {
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = Arc::new(kernel::testing::test_config(base.url()));
        let passwords = Arc::new(Passwords::new().expect("paramètres Argon2id"));
        let state = IdentityState::new(base.db(), config.clone(), passwords)
            .expect("état du module identity");

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
}

/// Ce qui distingue un compte d'un autre dans les tests : tout le reste est
/// identique, et c'est voulu — une seule variable par cas.
pub struct Compte<'a> {
    pub email: &'a str,
    pub verifie: bool,
    pub statut: &'a str,
    pub suspendu_jusqu_a: Option<OffsetDateTime>,
    pub avec_mot_de_passe: bool,
    pub second_facteur: bool,
}

impl<'a> Compte<'a> {
    pub fn actif(email: &'a str) -> Self {
        Self {
            email,
            verifie: true,
            statut: "active",
            suspendu_jusqu_a: None,
            avec_mot_de_passe: true,
            second_facteur: false,
        }
    }

    pub fn non_verifie(email: &'a str) -> Self {
        Self {
            verifie: false,
            ..Self::actif(email)
        }
    }

    pub fn suspendu(email: &'a str, jusqu_a: OffsetDateTime) -> Self {
        Self {
            statut: "suspended",
            suspendu_jusqu_a: Some(jusqu_a),
            ..Self::actif(email)
        }
    }

    pub fn bloque(email: &'a str) -> Self {
        Self {
            statut: "blocked",
            ..Self::actif(email)
        }
    }

    pub fn sans_mot_de_passe(email: &'a str) -> Self {
        Self {
            avec_mot_de_passe: false,
            ..Self::actif(email)
        }
    }

    pub fn avec_second_facteur(email: &'a str) -> Self {
        Self {
            second_facteur: true,
            ..Self::actif(email)
        }
    }
}

pub async fn semer(bac: &Bac, compte: Compte<'_>) -> Uuid {
    let db = bac.db();
    let ctx = bac.ctx();
    let mut tx = db.write(&ctx).await.expect("transaction");

    let verifie_le = compte.verifie.then(OffsetDateTime::now_utc);

    let person_id = sqlx::query_scalar!(
        "INSERT INTO identity.people
             (primary_email, first_name, last_name, email_verified_at, status, suspended_until)
         VALUES ($1::text::platform.email, 'Awa', 'Diallo', $2,
                 $3::text::identity.person_status, $4)
         RETURNING id",
        compte.email,
        verifie_le,
        compte.statut,
        compte.suspendu_jusqu_a
    )
    .fetch_one(&mut *tx)
    .await
    .expect("insertion de la personne");

    if compte.avec_mot_de_passe {
        let empreinte = bac
            .state
            .passwords()
            .hash(MOT_DE_PASSE)
            .expect("hachage du mot de passe");
        let mfa = compte.second_facteur.then(OffsetDateTime::now_utc);

        sqlx::query!(
            "INSERT INTO identity.accounts
                 (person_id, provider, password_hash, password_changed_at, mfa_enabled_at)
             VALUES ($1, 'password', $2, now(), $3)",
            person_id,
            empreinte,
            mfa
        )
        .execute(&mut *tx)
        .await
        .expect("insertion du compte");
    }

    tx.commit().await.expect("validation");
    person_id
}

/// Les sessions d'une personne, motif compris : ce que la rotation et la
/// détection de rejeu écrivent se relit ici.
pub async fn sessions(bac: &Bac, person_id: Uuid) -> Vec<(Uuid, Option<String>)> {
    sqlx::query!(
        "SELECT id, revoked_reason FROM identity.sessions
          WHERE person_id = $1 ORDER BY issued_at",
        person_id
    )
    .fetch_all(bac.base.pool())
    .await
    .expect("lecture des sessions")
    .into_iter()
    .map(|l| (l.id, l.revoked_reason))
    .collect()
}

/// Fait remonter la dernière rotation de `secondes` dans le passé : c'est la
/// seule façon d'éprouver la fenêtre de tolérance sans attendre une minute.
pub async fn vieillir_la_rotation(bac: &Bac, person_id: Uuid, secondes: i64) {
    sqlx::query!(
        "UPDATE identity.sessions
            SET revoked_at = revoked_at - make_interval(secs => $2)
          WHERE person_id = $1 AND revoked_at IS NOT NULL",
        person_id,
        secondes as f64
    )
    .execute(bac.base.pool())
    .await
    .expect("vieillissement de la rotation");
}

pub async fn sessions_vivantes(bac: &Bac, person_id: Uuid) -> i64 {
    identity::repo::sessions::count_active(bac.base.pool(), person_id)
        .await
        .expect("comptage des sessions")
}

/// Une connexion réussie, avec ses deux jetons. Panique sur toute autre issue :
/// un test qui croit ouvrir une session ne doit pas continuer sans.
pub async fn connexion(bac: &Bac, email: &str) -> identity::service::session::IssuedSession {
    connexion_avec(bac, email, appareil_du_site(), false).await
}

/// L'appareil que le site déclare : rien. L'objet `client` est absent de ses
/// appels, et c'est ce qui prouve la non-régression.
pub fn appareil_du_site() -> identity::service::session::Device<'static> {
    identity::service::session::Device {
        user_agent: Some("test"),
        ip: "127.0.0.1".parse().ok(),
        ..Default::default()
    }
}

/// L'appareil que Guide Négo déclare. **Aucun de ces champs n'accorde de
/// droit** : un client peut les forger.
pub fn appareil_de_lapplication() -> identity::service::session::Device<'static> {
    identity::service::session::Device {
        user_agent: Some("test"),
        ip: "127.0.0.1".parse().ok(),
        client_kind: ClientKind::App,
        device_id: Some("9f2c-appareil-de-test"),
        device_label: Some("Android · Chrome"),
        device_platform: Some("android"),
    }
}

pub async fn connexion_avec(
    bac: &Bac,
    email: &str,
    device: identity::service::session::Device<'_>,
    remember_me: bool,
) -> identity::service::session::IssuedSession {
    let reponse = identity::service::auth::login(
        &bac.state,
        &bac.ctx(),
        identity::service::auth::LoginRequest {
            email,
            password: MOT_DE_PASSE,
            remember_me,
            device,
        },
    )
    .await
    .expect("connexion");

    assert!(
        reponse.outcome.est_authentifie(),
        "issue inattendue : {:?}",
        reponse.outcome
    );
    reponse.session.expect("session ouverte")
}

/// Ce que la ligne de session porte réellement : d'où elle vient, quel appareil,
/// et quand elle expire. C'est la seule façon de prouver qu'une rotation a
/// recopié le client — la réponse HTTP, elle, n'en dit rien.
pub struct LigneDeSession {
    pub client_kind: String,
    pub device_id: Option<String>,
    pub device_label: Option<String>,
    pub device_platform: Option<String>,
    pub expires_at: OffsetDateTime,
}

pub async fn ligne_de_session(bac: &Bac, session_id: Uuid) -> LigneDeSession {
    let l = sqlx::query!(
        r#"SELECT client_kind::text AS "client_kind!", device_id, device_label,
                  device_platform, expires_at
             FROM identity.sessions WHERE id = $1"#,
        session_id
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture de la session");

    LigneDeSession {
        client_kind: l.client_kind,
        device_id: l.device_id,
        device_label: l.device_label,
        device_platform: l.device_platform,
        expires_at: l.expires_at,
    }
}

/// Ce que l'intergiciel de session de l'API ferait de ce jeton d'accès. Une
/// erreur ici est une panne de base, pas une session invalide : le test doit
/// s'arrêter dessus, jamais la confondre avec « déconnecté ».
pub async fn acteur_resolu(bac: &Bac, jeton_dacces: &str) -> Option<Uuid> {
    identity::resolve_actor(bac.base.pool(), bac.state.tokens(), jeton_dacces)
        .await
        .expect("résolution de session")
        .map(|resolue| resolue.person_id)
}

/// Une édition, pour donner une cible aux portées d'événement. Le module
/// `event` n'a pas de crate dans ce jalon : la ligne se sème en SQL, comme le
/// ferait la migration. Elle est déclarée en ligne — `ck_events_physical_location`
/// exigerait sinon un pays et une ville qui ne changeraient rien au périmètre.
pub async fn semer_evenement(bac: &Bac, slug: &str, titre: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2027, jsonb_build_object('fr', $2::text), $1::text::platform.slug,
                   jsonb_build_object('fr', $2::text), 'online',
                   'America/Belem'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#,
        slug,
        titre
    )
    .fetch_one(bac.base.pool())
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
) -> Uuid {
    let db = bac.db();
    let ctx = bac.ctx();
    let mut tx = db.write(&ctx).await.expect("transaction");

    let id = sqlx::query_scalar!(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
         VALUES ($1, $2, $3::text::identity.scope_type, $4)
         RETURNING id",
        person_id,
        role_code,
        scope_type,
        scope_id
    )
    .fetch_one(&mut *tx)
    .await
    .expect("attribution du rôle");

    tx.commit().await.expect("validation");
    id
}

/// Le périmètre d'administration tel que le garde du noyau le lit — refus
/// compris. C'est ce que l'extracteur `Perimeter` fait avant toute liste.
pub async fn perimetre(
    bac: &Bac,
    person_id: Uuid,
) -> kernel::error::Result<kernel::auth::AdminScope> {
    kernel::auth::require_perimeter(bac.base.pool(), person_id).await
}
