//! Ce que les tests d'intégration du module partagent : une base jetable, un
//! état de module, et de quoi semer une édition, ses journées, ses activités,
//! des organisations et des messages **dans les cinq portées et les cinq
//! états**.
//!
//! Aucun double en mémoire (principe X) : chaque test travaille sur une vraie
//! base, recopiée d'un modèle chargé depuis `docs/database/`.
//!
//! # LE SEMIS NE FOURNIT AUCUN INCIDENT
//!
//! `900_seed.sql` ne pose ni édition ni message d'incident. **Chaque test pose
//! les siens** — et c'est ce qui rend un décompte lisible : un test qui trouve
//! six lignes en a posé six.
//!
//! # POURQUOI LA VRAIE APPLICATION N'EST PAS MONTÉE ICI
//!
//! La monter demanderait au crate `live` une dépendance de développement vers
//! `api` — qui dépend de tous les autres modules. Le contrôle de frontière du
//! principe II, `cargo tree -p live`, doit ne **rien** rendre vers `modules/` :
//! `cargo tree` liste aussi les dépendances de développement, et cette arête le
//! ferait échouer.
//!
//! Les contrôles de périmètre et d'autorisation vivent donc dans `service/`, et
//! non dans les gestionnaires Actix : les tests les appellent directement, avec
//! un `Perimeter` construit par le noyau.

#![allow(dead_code)]

use kernel::auth::Perimeter;
use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::testing::TestDb;
use kernel::Db;
use live::domain::payload::IncidentPayload;
use live::state::LiveState;
use serde_json::{json, Value};
use std::sync::Arc;
use time::macros::time;
use time::{Date, OffsetDateTime, Time};
use uuid::Uuid;

pub struct Bac {
    pub base: TestDb,
    pub state: LiveState,
    pub config: Arc<Config>,
}

impl Bac {
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = Arc::new(kernel::testing::test_config(base.url()));
        let state = LiveState::new(base.db(), config.clone());
        Self {
            base,
            state,
            config,
        }
    }

    pub fn db(&self) -> Db {
        self.base.db()
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        self.base.pool()
    }

    pub fn ctx(&self, acteur: Uuid) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr").with_actor(acteur)
    }
}

// ---------------------------------------------------------------------------
// Le décor : une édition, ses journées, ses activités, une organisation
// ---------------------------------------------------------------------------

/// L'édition de référence des tests, et tout ce qui s'y rattache.
///
/// **Le fuseau est celui de Belém**, trois heures derrière l'UTC : c'est ce qui
/// fait que le jour du poste de direct ne peut pas être confondu avec la date du
/// serveur. Un test qui trouverait la bonne date sur un fuseau UTC ne prouverait
/// rien.
pub struct Decor {
    pub event_id: Uuid,
    pub timezone: String,
    /// Une seconde édition, **hors du périmètre détaché** : la cible naturelle
    /// d'une URL forgée.
    pub autre_event_id: Uuid,
    pub jour_id: Uuid,
    pub jour_date: Date,
    pub session_id: Uuid,
    pub autre_session_id: Uuid,
    pub organization_id: Uuid,
    /// Une organisation qui n'anime **aucune** activité de l'édition.
    pub organisation_etrangere: Uuid,
}

pub const FUSEAU: &str = "America/Belem";

pub async fn decor(bac: &Bac) -> Decor {
    let event_id = edition(bac, "cop31-belem", "COP31 — Belém", FUSEAU, "Belém").await;
    let autre_event_id = edition(bac, "cop30-bakou", "COP30 — Bakou", "Asia/Baku", "Bakou").await;

    let jour_date = jour_de_ledition(bac, event_id).await;
    let jour_id = journee(bac, event_id, jour_date, Some("Journée finance")).await;

    let organization_id = organisation(
        bac,
        "Organisation porteuse du test",
        "OPT",
        "organisation-porteuse-test",
    )
    .await;
    let organisation_etrangere = organisation(
        bac,
        "Organisation étrangère",
        "OE",
        "organisation-etrangere",
    )
    .await;

    // Deux activités **aujourd'hui dans le fuseau de l'édition** : le poste de
    // direct doit les trouver toutes les deux sans repli.
    let session_id = activite(
        bac,
        event_id,
        Some(jour_id),
        Some(organization_id),
        "Atelier de négociation",
        "atelier-de-negociation",
        aujourdhui_a(bac, event_id, time!(09:00)).await,
        aujourdhui_a(bac, event_id, time!(10:30)).await,
    )
    .await;
    let autre_session_id = activite(
        bac,
        event_id,
        Some(jour_id),
        Some(organization_id),
        "Table ronde sur la finance",
        "table-ronde-finance",
        aujourdhui_a(bac, event_id, time!(14:00)).await,
        aujourdhui_a(bac, event_id, time!(15:30)).await,
    )
    .await;

    Decor {
        event_id,
        timezone: FUSEAU.to_owned(),
        autre_event_id,
        jour_id,
        jour_date,
        session_id,
        autre_session_id,
        organization_id,
        organisation_etrangere,
    }
}

/// Une édition qui couvre aujourd'hui — sans quoi ses activités du jour
/// tomberaient hors de sa propre période.
///
/// **En présentiel, avec une ville** : `ck_events_physical_location` l'exige, et
/// c'est la ville que l'écran affiche en `zone_label` — « heure de Belém », et
/// non « heure de America/Belem ».
pub async fn edition(bac: &Bac, slug: &str, titre: &str, fuseau: &str, ville: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, status, participation_mode,
                timezone, starts_at, ends_at, has_pavilion, country_id, city)
           VALUES (2027, jsonb_build_object('fr', $2::text, 'en', $2::text), $1,
                   '{"fr":"Description."}'::jsonb, 'ongoing', 'hybrid',
                   $3::text::platform.timezone_name,
                   now() - interval '2 days', now() + interval '8 days', true,
                   (SELECT id FROM reference.countries ORDER BY iso2 LIMIT 1), $4)
        RETURNING id"#,
        slug,
        titre,
        fuseau,
        ville
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition")
}

/// Le jour civil de l'édition **dans son fuseau**, tel que la base le calcule.
pub async fn jour_de_ledition(bac: &Bac, event_id: Uuid) -> Date {
    sqlx::query_scalar!(
        r#"SELECT (now() AT TIME ZONE e.timezone)::date AS "jour!"
             FROM event.events e WHERE e.id = $1"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("jour de l'édition")
}

/// Un instant d'aujourd'hui, à l'heure locale de l'édition — **calculé en
/// base**, dans son fuseau, jamais dans celui du serveur de test.
pub async fn aujourdhui_a(bac: &Bac, event_id: Uuid, heure: Time) -> OffsetDateTime {
    sqlx::query_scalar!(
        r#"SELECT (((now() AT TIME ZONE e.timezone)::date + $2::time)
                    AT TIME ZONE e.timezone) AS "instant!"
             FROM event.events e WHERE e.id = $1"#,
        event_id,
        heure
    )
    .fetch_one(bac.pool())
    .await
    .expect("instant local")
}

pub async fn journee(bac: &Bac, event_id: Uuid, date: Date, titre: Option<&str>) -> Uuid {
    let titre = titre.map(|t| json!({ "fr": t, "en": t }));
    sqlx::query_scalar!(
        r#"INSERT INTO event.event_days (event_id, day_date, title, sort_order)
           VALUES ($1, $2, $3, 0)
        RETURNING id"#,
        event_id,
        date,
        titre
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la journée")
}

pub async fn organisation(bac: &Bac, nom: &str, sigle: &str, slug: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, acronym, slug, organization_type_code, status)
           VALUES ($1, $2, $3::text::platform.slug, 'ngo_association', 'active')
        RETURNING id"#,
        nom,
        sigle,
        slug
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'organisation")
}

#[allow(clippy::too_many_arguments)]
pub async fn activite(
    bac: &Bac,
    event_id: Uuid,
    jour_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    titre: &str,
    slug: &str,
    debut: OffsetDateTime,
    fin: OffsetDateTime,
) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, event_day_id, organization_id, title, slug, status,
                starts_at, ends_at, is_streamed, format, timezone)
           VALUES ($1, $2, $3, jsonb_build_object('fr', $4::text, 'en', $4::text),
                   $5::text::platform.slug, 'scheduled', $6, $7, true,
                   'in_person',
                   (SELECT e.timezone FROM event.events e WHERE e.id = $1))
        RETURNING id"#,
        event_id,
        jour_id,
        organization_id,
        titre,
        slug,
        debut,
        fin
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'activité")
}

// ---------------------------------------------------------------------------
// Les comptes et leurs périmètres
// ---------------------------------------------------------------------------

pub struct Comptes {
    /// Administratrice de la plateforme entière.
    pub globale: Uuid,
    /// Administrateur d'une seule édition — celle du décor.
    pub detache: Uuid,
    /// Administratrice de l'édition, **sans** `live.incident.publish` : elle
    /// doit voir la liste, et ne rien pouvoir publier.
    pub lectrice: Uuid,
    /// Aucun droit d'administration.
    pub sans_droit: Uuid,
}

pub async fn comptes(bac: &Bac, decor: &Decor) -> Comptes {
    let globale = personne(bac, "globale@ifdd.francophonie.org", "Claire", "Perret").await;
    attribuer(bac, globale, "super_admin", "global", None).await;

    let detache = personne(bac, "detache@ifdd.francophonie.org", "Détaché", "Test").await;
    attribuer(bac, detache, "admin", "event", Some(decor.event_id)).await;

    // `reviewer` administre-t-il ? Non : il n'entre pas dans
    // `identity.administered_events()`. Le compte « lectrice » est donc un
    // `programmer` privé de la seule permission d'écriture des incidents — ce
    // qui se fait en retirant la ligne du catalogue pour ce rôle sur la portée.
    let lectrice = personne(bac, "lectrice@ifdd.francophonie.org", "Lina", "Traoré").await;
    attribuer(bac, lectrice, "programmer", "event", Some(decor.event_id)).await;
    retirer_la_permission(bac, "programmer", "live.incident.publish").await;

    let sans_droit = personne(bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;
    attribuer(bac, sans_droit, "standard", "global", None).await;

    Comptes {
        globale,
        detache,
        lectrice,
        sans_droit,
    }
}

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

/// Retire une permission d'un rôle, **sur la base jetable du test seulement**.
///
/// C'est ce qui permet d'éprouver « administrer sans pouvoir publier », un
/// couple que le catalogue livré n'offre pas : `admin` et `programmer`
/// détiennent tous deux `live.incident.publish`.
pub async fn retirer_la_permission(bac: &Bac, role_code: &str, permission_code: &str) {
    sqlx::query!(
        "DELETE FROM identity.role_permissions
          WHERE role_code = $1 AND permission_code = $2",
        role_code,
        permission_code
    )
    .execute(bac.pool())
    .await
    .expect("retrait de la permission");
}

/// Le périmètre tel que le garde du noyau le rend.
pub async fn perimetre_de(bac: &Bac, person_id: Uuid) -> Perimeter {
    let scope = kernel::auth::require_perimeter(bac.pool(), person_id)
        .await
        .expect("périmètre non vide");
    Perimeter { person_id, scope }
}

/// Le périmètre d'un compte **sans aucun droit d'administration**.
///
/// `require_perimeter` refuse — c'est justement ce qu'il doit faire —, donc le
/// périmètre est construit à la main : les tests éprouvent alors le refus du
/// service, pas celui de l'extracteur.
pub fn perimetre_vide(person_id: Uuid) -> Perimeter {
    Perimeter {
        person_id,
        scope: kernel::auth::AdminScope {
            is_global: false,
            event_ids: Vec::new(),
        },
    }
}

// ---------------------------------------------------------------------------
// Les messages
// ---------------------------------------------------------------------------

/// Une charge utile **valide et minimale**, de portée `session`.
///
/// Chaque test part de là et ne modifie que ce qu'il éprouve : c'est ce qui fait
/// qu'un refus attendu ne peut pas venir d'ailleurs.
pub fn payload(scope: &str) -> IncidentPayload {
    IncidentPayload {
        scope: scope.to_owned(),
        event_id: None,
        event_day_id: None,
        session_id: None,
        organization_id: None,
        incident_kind_code: "technical_issue".to_owned(),
        severity: "warning".to_owned(),
        title: None,
        message: json!({ "fr": "La diffusion est interrompue.", "en": "The stream is down." }),
        action_url: None,
        is_dismissible: true,
        display_from: OffsetDateTime::now_utc() - time::Duration::minutes(5),
        display_until: None,
        publish: false,
    }
}

/// Pose un message **directement en base**, dans l'état demandé.
///
/// `etat` vaut `draft`, `active`, `scheduled`, `expired` ou `unpublished` — les
/// cinq que `live.event_incidents()` calcule. La publication passe par la
/// **fonction du modèle** : un `UPDATE` direct laisserait `published_by` nul et
/// ferait mentir les tests d'historique.
pub async fn poser(bac: &Bac, acteur: Uuid, scope: &str, cible: Option<Uuid>, etat: &str) -> Uuid {
    // Les bornes sont exprimées en **secondes depuis `now()` côté base**, jamais
    // en instants calculés côté Rust : l'horloge du test et celle de la base ne
    // sont pas la même, et un décalage d'une seconde suffirait à faire basculer
    // un message « programmé » en « actif ».
    let (debut_s, fin_s): (f64, Option<f64>) = match etat {
        "scheduled" => (7_200.0, None),
        "expired" => (-14_400.0, Some(-3_600.0)),
        _ => (-300.0, None),
    };

    let (event_id, event_day_id, session_id, organization_id) = match scope {
        "event" => (cible, None, None, None),
        "event_day" => (None, cible, None, None),
        "session" => (None, None, cible, None),
        "organization" => (None, None, None, cible),
        _ => (None, None, None, None),
    };

    let id = sqlx::query_scalar!(
        r#"INSERT INTO live.incidents
               (scope, event_id, event_day_id, session_id, organization_id,
                incident_kind_code, severity, message, display_from, display_until, created_by)
           VALUES ($1::text::live.incident_scope, $2, $3, $4, $5,
                   'technical_issue', 'warning',
                   '{"fr":"Message.","en":"Message."}'::jsonb,
                   now() + make_interval(secs => $6),
                   CASE WHEN $7::float8 IS NULL THEN NULL
                        ELSE now() + make_interval(secs => $7) END,
                   $8)
        RETURNING id"#,
        scope,
        event_id,
        event_day_id,
        session_id,
        organization_id,
        debut_s,
        fin_s,
        acteur
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du message");

    if matches!(etat, "active" | "scheduled" | "expired" | "unpublished") {
        publier_en_base(bac, acteur, id).await;
    }
    if etat == "unpublished" {
        depublier_en_base(bac, acteur, id, Some("Fin de l'incident.")).await;
    }

    id
}

/// Publie **par la fonction du modèle**, avec l'acteur posé — sans quoi
/// `published_by` serait nul et l'historique afficherait « publié par — ».
pub async fn publier_en_base(bac: &Bac, acteur: Uuid, incident_id: Uuid) {
    let mut tx = bac
        .db()
        .write(&bac.ctx(acteur))
        .await
        .expect("transaction d'écriture");
    sqlx::query!(
        r#"SELECT (live.publish_incident($1)).id AS "id!""#,
        incident_id
    )
    .fetch_one(&mut *tx)
    .await
    .expect("publication");
    tx.commit().await.expect("validation");
}

pub async fn depublier_en_base(bac: &Bac, acteur: Uuid, incident_id: Uuid, motif: Option<&str>) {
    let mut tx = bac
        .db()
        .write(&bac.ctx(acteur))
        .await
        .expect("transaction d'écriture");
    sqlx::query!(
        r#"SELECT (live.unpublish_incident($1, $2)).id AS "id!""#,
        incident_id,
        motif
    )
    .fetch_one(&mut *tx)
    .await
    .expect("dépublication");
    tx.commit().await.expect("validation");
}

// ---------------------------------------------------------------------------
// Ce qu'on inspecte
// ---------------------------------------------------------------------------

/// Les événements d'outbox d'un agrégat. **Les compter** est le seul contrôle
/// qui dise quelque chose d'un doublon : vérifier leur présence n'en dit rien.
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

/// La colonne `published_by`, **telle qu'elle est en base**. C'est elle qui dit
/// si le contexte d'écriture a été posé, pas l'audit.
pub async fn publie_par(bac: &Bac, incident_id: Uuid) -> Option<Uuid> {
    sqlx::query_scalar!(
        "SELECT published_by FROM live.incidents WHERE id = $1",
        incident_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de published_by")
}

/// Le libellé français d'un texte multilingue, pour les assertions.
pub fn fr(valeur: &Value) -> String {
    valeur
        .get("fr")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
