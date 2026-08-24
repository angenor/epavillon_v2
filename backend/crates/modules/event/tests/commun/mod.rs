//! Ce que les tests d'intégration du module partagent : une base jetable, un
//! état de module, et de quoi semer des éditions, des comptes et des périmètres
//! d'administration.
//!
//! Aucun double en mémoire (principe X) : chaque test travaille sur une vraie
//! base, recopiée d'un modèle chargé depuis `docs/database/`.
//!
//! ## Pourquoi la **vraie application** n'est pas montée ici
//!
//! La monter demanderait au crate `event` une dépendance de développement vers
//! le crate `api` — qui dépend lui-même de `identity` et de `org`. Le contrôle
//! bloquant du jalon, `cargo tree -p event | grep -E 'identity|org'`, doit ne
//! **rien** rendre : `cargo tree` liste aussi les dépendances de
//! développement, et cette arête le ferait échouer.
//!
//! Les tests qui frappent les routes sur l'application entière — intergiciels
//! compris — vivent donc dans `crates/api/tests/`, exactement là où B2 a mis
//! les siens après le défaut des trois routes muettes. Ici, les tests appellent
//! les services : ils n'ouvrent pas de session, ils passent l'acteur.

#![allow(dead_code)]

use event::domain::edition::EditionFormPayload;
use event::state::EventState;
use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::testing::TestDb;
use kernel::Db;
use serde_json::json;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

pub mod seed;

pub struct Bac {
    pub base: TestDb,
    pub state: EventState,
    pub config: Arc<Config>,
}

impl Bac {
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = Arc::new(kernel::testing::test_config(base.url()));
        let state = EventState::new(base.db(), config.clone());

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

/// Une série semée par `900_seed.sql`, par son code. Les tests **ne créent
/// aucune série** : il y en a déjà quatre.
pub async fn serie(bac: &Bac, code: &str) -> Uuid {
    sqlx::query_scalar!("SELECT id FROM event.event_series WHERE code = $1", code)
        .fetch_one(bac.pool())
        .await
        .unwrap_or_else(|_| panic!("la série {code} est semée par 900_seed.sql"))
}

/// Un pays du référentiel, par son code ISO à trois lettres.
pub async fn pays(bac: &Bac, iso3: &str) -> Uuid {
    sqlx::query_scalar!("SELECT id FROM reference.countries WHERE iso3 = $1", iso3)
        .fetch_one(bac.pool())
        .await
        .unwrap_or_else(|_| panic!("le pays {iso3} est semé par 900_seed.sql"))
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
    /// Administrateur d'une seule édition — celle que rend le second champ.
    pub detache: Uuid,
    pub edition_detachee: Uuid,
    /// Aucun droit d'administration : le rôle d'utilisateur ordinaire, qui
    /// détient des permissions de consultation et **aucun périmètre**. C'est ce
    /// couple-là qui doit être refusé, pas seulement l'absence de rôle.
    pub sans_droit: Uuid,
}

/// Les trois comptes, adossés aux **deux éditions du semis de test** : la
/// COP31 reste hors du périmètre détaché, ce qui en fait la cible naturelle
/// d'une URL forgée.
pub async fn perimetres(bac: &Bac, editions: &seed::Editions) -> Perimetres {
    let globale = personne(bac, "globale@ifdd.francophonie.org", "Claire", "Perret").await;
    attribuer(bac, globale, "super_admin", "global", None).await;

    let detache = personne(bac, "detache@ifdd.francophonie.org", "Détaché", "Test").await;
    attribuer(bac, detache, "admin", "event", Some(editions.sans_pavillon)).await;

    let sans_droit = personne(bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;
    attribuer(bac, sans_droit, "standard", "global", None).await;

    Perimetres {
        globale,
        detache,
        edition_detachee: editions.sans_pavillon,
        sans_droit,
    }
}

/// Le périmètre tel que le garde du noyau le rend, pour une personne.
pub async fn perimetre_de(bac: &Bac, person_id: Uuid) -> kernel::auth::Perimeter {
    let scope = kernel::auth::require_perimeter(bac.pool(), person_id)
        .await
        .expect("périmètre non vide");
    kernel::auth::Perimeter { person_id, scope }
}

/// Les événements de l'outbox d'un agrégat, dans l'ordre. Les **compter** est
/// le seul contrôle qui dise quelque chose d'un doublon : vérifier leur
/// présence n'en dit rien.
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

// -----------------------------------------------------------------------------
// De quoi appeler le service d'écriture
// -----------------------------------------------------------------------------

/// Une charge utile **valide et minimale** : en ligne, sans pavillon, sans
/// sigle. Chaque test part de là et ne modifie que ce qu'il éprouve — c'est ce
/// qui fait qu'un refus attendu ne peut pas venir d'ailleurs.
///
/// La période est donnée en instants complets, comme le formulaire les compose :
/// le service ne décide d'aucun fuseau à la place de l'appelant (FR-032).
pub fn formulaire(slug: &str, titre_fr: &str) -> EditionFormPayload {
    EditionFormPayload {
        id: None,
        series_id: None,
        edition_label: None,
        edition_year: 2027,
        title: json!({ "fr": titre_fr }),
        acronym: None,
        slug: slug.to_owned(),
        description: json!({ "fr": "Description de l'édition." }),
        status: "announced".to_owned(),
        participation_mode: "online".to_owned(),
        timezone: "Africa/Dakar".to_owned(),
        starts_at: instant("2027-03-02T10:00:00Z"),
        ends_at: instant("2027-03-04T16:00:00Z"),
        country_id: None,
        city: None,
        address: None,
        latitude: None,
        longitude: None,
        has_pavilion: false,
        highlights: None,
    }
}

/// Un instant lu tel que le formulaire l'envoie : RFC 3339, fuseau compris.
pub fn instant(rfc3339: &str) -> OffsetDateTime {
    OffsetDateTime::parse(rfc3339, &time::format_description::well_known::Rfc3339)
        .expect("instant RFC 3339")
}

/// L'acteur des tests : une personne réelle, parce que `xmod_fk_events_creator`
/// exige qu'elle existe.
///
/// **Idempotent** : un test qui écrit dix éditions l'appelle dix fois, et
/// `ux_people_primary_email` refuserait la seconde insertion. L'index étant
/// partiel — il ne porte que sur les personnes non anonymisées —, on relit
/// plutôt que de s'appuyer sur un `ON CONFLICT` qui devrait redire sa clause.
pub async fn auteur(bac: &Bac) -> Uuid {
    const ADRESSE: &str = "auteur@ifdd.francophonie.org";

    let existante = sqlx::query_scalar!(
        "SELECT id FROM identity.people WHERE primary_email = $1::text::platform.email",
        ADRESSE
    )
    .fetch_optional(bac.pool())
    .await
    .expect("lecture de l'auteur");

    match existante {
        Some(id) => id,
        None => personne(bac, ADRESSE, "Aïcha", "Diallo").await,
    }
}

/// Les journées du calendrier d'une édition, croissantes.
pub async fn journees(bac: &Bac, event_id: Uuid) -> Vec<time::Date> {
    sqlx::query_scalar!(
        "SELECT day_date FROM event.event_days WHERE event_id = $1 ORDER BY day_date",
        event_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des journées")
}

// -----------------------------------------------------------------------------
// De quoi appeler le service de l'appel à propositions
// -----------------------------------------------------------------------------

/// Une ligne de grille valide et minimale.
pub fn critere(code: &str, poids: f64) -> event::domain::call::CriterionPayload {
    event::domain::call::CriterionPayload {
        id: None,
        code: code.to_owned(),
        label: json!({ "fr": format!("Critère {code}"), "en": format!("Criterion {code}") }),
        description: None,
        max_score: 5.0,
        weight: poids,
        is_knockout: false,
        sort_order: 10,
    }
}

/// Une charge utile d'appel **valide et minimale**, avec une grille d'un seul
/// critère. Chaque test part de là et ne modifie que ce qu'il éprouve : c'est ce
/// qui fait qu'un refus attendu ne peut pas venir d'ailleurs.
///
/// La fenêtre encadre l'instant courant, de sorte qu'un appel `open` le soit
/// vraiment au sens de `event.is_call_open()` — statut **et** fenêtre.
pub fn formulaire_appel(event_id: Uuid, code: &str) -> event::domain::call::EditionCallPayload {
    event::domain::call::EditionCallPayload {
        id: None,
        event_id,
        code: code.to_owned(),
        title: json!({ "fr": "Appel à propositions", "en": "Call for proposals" }),
        description: None,
        status: "draft".to_owned(),
        opens_at: OffsetDateTime::now_utc() - time::Duration::days(1),
        closes_at: OffsetDateTime::now_utc() + time::Duration::days(30),
        extended_until: None,
        results_expected_at: None,
        max_proposals_per_organization: None,
        requires_verified_organization: false,
        min_speakers: 1,
        max_speakers: 10,
        default_duration_minutes: 60,
        min_duration_minutes: 45,
        max_duration_minutes: 150,
        daily_start_time: "09:00:00".to_owned(),
        daily_end_time: "17:00:00".to_owned(),
        allowed_formats: vec![
            "online".to_owned(),
            "in_person".to_owned(),
            "hybrid".to_owned(),
        ],
        required_reviews: 2,
        blind_review: true,
        guidelines_url: None,
        criteria: vec![critere("relevance", 2.0)],
    }
}

/// Les critères d'un appel, tels que la base les porte : code, barème, poids.
pub async fn grille_en_base(bac: &Bac, call_id: Uuid) -> Vec<(String, f64, f64)> {
    sqlx::query!(
        r#"SELECT code, max_score::float8 AS "max_score!", weight::float8 AS "weight!"
             FROM event.review_criteria WHERE call_id = $1 ORDER BY sort_order, code"#,
        call_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de la grille")
    .into_iter()
    .map(|l| (l.code, l.max_score, l.weight))
    .collect()
}

/// Le nombre de notes posées sur un critère. **C'est le chiffre qui prouve
/// qu'un refus a bien préservé l'argumentaire** : sans lui, le test ne dit rien.
pub async fn notes_du_critere(bac: &Bac, criterion_id: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.review_scores WHERE criterion_id = $1"#,
        criterion_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("décompte des notes")
}

// -----------------------------------------------------------------------------
// De quoi appeler les services d'onglet
// -----------------------------------------------------------------------------

/// Une charge utile de lieu, valide et minimale.
pub fn formulaire_lieu(event_id: Uuid, nom: &str) -> event::domain::tabs::EditionVenuePayload {
    event::domain::tabs::EditionVenuePayload {
        id: None,
        event_id,
        name: json!({ "fr": nom }),
        kind: "pavilion".to_owned(),
        address: None,
        map_url: None,
    }
}

/// Une charge utile de salle, valide et minimale. **`is_virtual` est faux par
/// défaut** : une salle du stand occupe un lieu, et c'est ce qui rend un
/// chevauchement signalable.
pub fn formulaire_salle(venue_id: Uuid, code: &str) -> event::domain::tabs::EditionRoomPayload {
    event::domain::tabs::EditionRoomPayload {
        id: None,
        venue_id,
        name: json!({ "fr": format!("Salle {code}") }),
        code: code.to_owned(),
        capacity: Some(80),
        is_virtual: false,
        has_streaming: false,
        equipment: Vec::new(),
        sort_order: 10,
    }
}

/// Une charge utile de canal, valide et minimale.
pub fn formulaire_canal(
    event_id: Uuid,
    code: &str,
    par_defaut: bool,
) -> event::domain::tabs::EditionChannelPayload {
    event::domain::tabs::EditionChannelPayload {
        id: None,
        event_id,
        code: code.to_owned(),
        name: json!({ "fr": format!("Canal {code}") }),
        provider: "youtube".to_owned(),
        channel_ref: None,
        locale: None,
        is_default: par_defaut,
        is_active: true,
    }
}

/// Les canaux **par défaut et actifs** du groupe d'une édition. C'est ce que
/// l'index `ux_broadcast_channels_default` autorise à un seul exemplaire.
pub async fn canaux_par_defaut(bac: &Bac, event_id: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT code FROM event.broadcast_channels
          WHERE event_id = $1 AND is_default AND is_active
          ORDER BY code",
        event_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des canaux par défaut")
}

/// Le canal général de la plateforme semé par `900_seed.sql`, avec son état.
pub async fn canal_general(bac: &Bac) -> (Uuid, bool, bool) {
    let ligne = sqlx::query!(
        "SELECT id, is_default, is_active FROM event.broadcast_channels
          WHERE event_id IS NULL AND code = 'ifdd_principal'"
    )
    .fetch_one(bac.pool())
    .await
    .expect("le canal général est semé par 900_seed.sql");

    (ligne.id, ligne.is_default, ligne.is_active)
}

/// Une charge utile de fil, valide et minimale.
pub fn formulaire_fil(event_id: Uuid, code: &str) -> event::domain::tabs::EditionTrackPayload {
    event::domain::tabs::EditionTrackPayload {
        id: None,
        event_id,
        code: code.to_owned(),
        slug: code.replace('_', "-"),
        kind: "special_day".to_owned(),
        title: json!({ "fr": format!("Journée {code}") }),
        subtitle: None,
        description: None,
        starts_on: None,
        ends_on: None,
        color_hex: None,
        curated_by: None,
        is_published: false,
        sort_order: 10,
        theme_codes: Vec::new(),
    }
}

/// Les journées d'une édition, **avec tout leur habillage éditorial** : c'est ce
/// qu'une régénération ne doit pas toucher.
pub async fn journees_habillees(
    bac: &Bac,
    event_id: Uuid,
) -> Vec<(
    time::Date,
    Option<serde_json::Value>,
    Option<String>,
    bool,
    Option<String>,
)> {
    sqlx::query!(
        r#"SELECT day_date, title, slug::text AS "slug?", is_featured, color_hex
             FROM event.event_days WHERE event_id = $1 ORDER BY day_date"#,
        event_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des journées")
    .into_iter()
    .map(|l| (l.day_date, l.title, l.slug, l.is_featured, l.color_hex))
    .collect()
}
