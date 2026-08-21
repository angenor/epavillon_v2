//! Ce que les tests d'intégration du module partagent : une base jetable, un
//! état de module, et **de quoi déposer**.
//!
//! Aucun double en mémoire (principe X) : chaque test travaille sur une vraie
//! base, recopiée d'un modèle chargé depuis `docs/database/`.
//!
//! # Pourquoi une fabrique, et pourquoi elle enchaîne cinq créations
//!
//! `900_seed.sql` **ne pose aucun dossier** dans `programme` — hormis un
//! drapeau de fonctionnalité, qui ne sert à rien ici. Il faut donc, dans
//! l'ordre : une **édition** avec son fuseau (B3), un **appel ouvert** avec sa
//! grille (B3), une **organisation vérifiée** (B2), une **personne** membre
//! active de cette organisation (B1 et B2). C'est le premier parcours de bout
//! en bout du jalon, et sans cette fabrique chaque test d'écriture
//! recommencerait quarante lignes de préparation (research.md § R22).
//!
//! **Le semis ne fournit toujours aucun dossier**, et c'est délibéré : un
//! dossier semé serait un dossier dont l'état, l'appel et l'organisation
//! seraient partagés entre des tests qui les modifient.
//!
//! # Pourquoi la **vraie application** n'est pas montée ici
//!
//! La monter demanderait au crate `programme` une dépendance de développement
//! vers le crate `api` — qui dépend lui-même d'`identity`, d'`org` et
//! d'`event`. Le contrôle bloquant du jalon,
//! `cargo tree -p programme | grep -E 'identity|org|event'`, doit ne **rien**
//! rendre : `cargo tree` liste aussi les dépendances de développement, et cette
//! arête le ferait échouer.
//!
//! Les tests qui frappent les routes sur l'application entière — intergiciels
//! compris — vivent donc dans `crates/api/tests/`, exactement là où B2 a mis
//! les siens après le défaut des trois routes muettes. Ici, les tests appellent
//! les services : ils n'ouvrent pas de session, ils passent l'acteur.

#![allow(dead_code)]

use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::testing::TestDb;
use kernel::Db;
use programme::state::ProgrammeState;
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

pub struct Bac {
    pub base: TestDb,
    pub state: ProgrammeState,
    pub config: Arc<Config>,
}

impl Bac {
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = Arc::new(kernel::testing::test_config(base.url()));
        let state = ProgrammeState::new(base.db(), config.clone());

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

    pub fn ctx(&self) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr")
    }
}

// -----------------------------------------------------------------------------
// La fabrique — ce qu'il faut avoir avant de pouvoir déposer
// -----------------------------------------------------------------------------

/// Tout ce qu'un test de dépôt a besoin de connaître, en une valeur.
pub struct Terrain {
    /// L'édition de référence : COP31, Belém, fuseau `America/Belem`. **Trois
    /// heures derrière l'UTC** — c'est ce décalage qui rend visible une
    /// conversion d'heure murale oubliée.
    pub edition: Uuid,
    pub appel: Uuid,
    /// L'organisation porteuse, **vérifiée** : un appel peut l'exiger.
    pub organisation: Uuid,
    /// La déposante, membre **active** de l'organisation.
    pub deposante: Uuid,
}

pub const FUSEAU_COP31: &str = "America/Belem";
pub const SLUG_COP31: &str = "cop31-belem";

/// L'enchaînement complet : édition, appel ouvert, grille par défaut,
/// organisation vérifiée, adhésion active.
pub async fn terrain(bac: &Bac) -> Terrain {
    let edition = edition_cop31(bac).await;
    let appel = appel_ouvert(bac, edition).await;
    let organisation = organisation_verifiee(bac, "Institut de la Francophonie", "IFDD").await;
    let deposante = personne(bac, "aicha.diallo@example.org", "Aïcha", "Diallo").await;
    adherer(bac, organisation, deposante, "active").await;

    Terrain {
        edition,
        appel,
        organisation,
        deposante,
    }
}

/// L'édition de référence.
///
/// Les instants sont écrits **en heure locale de Belém**, convertis par la
/// base : les écrire en UTC obligerait à faire le décalage à la main dans le
/// test, c'est-à-dire à réimplémenter ce que le test doit vérifier.
pub async fn edition_cop31(bac: &Bac) -> Uuid {
    let climat = serie(bac, "cop_climate").await;
    let bresil = pays(bac, "BRA").await;

    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at,
                country_id, city, has_pavilion)
           VALUES ($1, 'COP31', 2027,
                   '{"fr":"COP31 — Conférence des Parties","en":"COP31"}'::jsonb,
                   'COP31', $2::text::platform.slug,
                   '{"fr":"Pavillon de la Francophonie à la COP31.","en":"Francophonie pavilion at COP31."}'::jsonb,
                   'announced', 'hybrid', $3::text::platform.timezone_name,
                   timestamp '2027-11-09 09:00' AT TIME ZONE $3,
                   timestamp '2027-11-20 18:00' AT TIME ZONE $3,
                   $4, 'Belém', true)
        RETURNING id"#,
        climat,
        SLUG_COP31,
        FUSEAU_COP31,
        bresil
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la COP31")
}

/// Une seconde édition, **sans appel**, pour les tests de périmètre : elle est
/// la cible naturelle d'une URL forgée.
pub async fn edition_secondaire(bac: &Bac) -> Uuid {
    let webinaires = serie(bac, "ifdd_webinars").await;

    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, slug, description,
                status, participation_mode, timezone, starts_at, ends_at, has_pavilion)
           VALUES ($1, 'PACO 2027', 2027,
                   '{"fr":"Rendez-vous du PACO","en":"PACO meeting"}'::jsonb,
                   'rendez-vous-paco-2027'::platform.slug,
                   '{"fr":"Cycle en ligne, sans pavillon.","en":"Online series, no pavilion."}'::jsonb,
                   'announced', 'online', 'Africa/Dakar'::platform.timezone_name,
                   timestamp '2027-03-02 10:00' AT TIME ZONE 'Africa/Dakar',
                   timestamp '2027-03-04 16:00' AT TIME ZONE 'Africa/Dakar',
                   false)
        RETURNING id"#,
        webinaires
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition secondaire")
}

/// Un appel **réellement ouvert** au sens d'`event.is_call_open()` : statut
/// **et** fenêtre. La fenêtre encadre l'instant courant, sans quoi un appel
/// marqué `open` refuserait quand même le dépôt.
///
/// La grille par défaut est semée par **la fonction du modèle**, jamais
/// recopiée : six critères, dont un éliminatoire.
pub async fn appel_ouvert(bac: &Bac, event_id: Uuid) -> Uuid {
    let call_id = sqlx::query_scalar!(
        r#"INSERT INTO event.calls_for_proposals
               (event_id, code, title, status, opens_at, closes_at,
                results_expected_at, required_reviews, blind_review)
           VALUES ($1, 'principal',
                   '{"fr":"Appel à propositions","en":"Call for proposals"}'::jsonb,
                   'open', now() - interval '1 day', now() + interval '30 days',
                   date '2027-09-15', 2, true)
        RETURNING id"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'appel");

    sqlx::query!("SELECT event.seed_default_criteria($1)", call_id)
        .execute(bac.pool())
        .await
        .expect("grille par défaut");

    call_id
}

/// Fermer l'appel — ce que fait le temps qui passe, en une requête.
pub async fn fermer_lappel(bac: &Bac, call_id: Uuid) {
    sqlx::query!(
        "UPDATE event.calls_for_proposals
            SET status = 'closed', closes_at = now() - interval '1 day'
          WHERE id = $1",
        call_id
    )
    .execute(bac.pool())
    .await
    .expect("clôture de l'appel");
}

/// Poser un plafond de dossiers par organisation sur l'appel.
pub async fn plafonner(bac: &Bac, call_id: Uuid, plafond: i16) {
    sqlx::query!(
        "UPDATE event.calls_for_proposals
            SET max_proposals_per_organization = $2 WHERE id = $1",
        call_id,
        plafond
    )
    .execute(bac.pool())
    .await
    .expect("pose du plafond");
}

/// Une organisation **vérifiée** : un appel peut l'exiger, et le semis n'en
/// fournit aucune.
pub async fn organisation_verifiee(bac: &Bac, nom: &str, sigle: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, acronym, slug, organization_type_code, status, verified_at)
           VALUES ($1, $2, platform.slugify($1)::platform.slug,
                   'ngo_association', 'active', now())
        RETURNING id"#,
        nom,
        sigle
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'organisation")
}

/// Une personne, avec son adresse. Le compte n'est pas créé : les tests de ce
/// module ne se connectent pas, ils appellent les services.
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

/// Une adhésion, avec son statut. **`pending` n'est pas `active`** : c'est la
/// distinction que `domain/ownership.rs` fait porter à tout l'espace
/// organisation.
pub async fn adherer(bac: &Bac, organization_id: Uuid, person_id: Uuid, statut: &str) {
    sqlx::query!(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, approved_at)
         VALUES ($1, $2, 'member', $3::text::org.membership_status,
                 CASE WHEN $3 = 'active' THEN now() END)",
        organization_id,
        person_id,
        statut
    )
    .execute(bac.pool())
    .await
    .expect("insertion de l'adhésion");
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

/// Le périmètre tel que le garde du noyau le rend, pour une personne.
pub async fn perimetre_de(bac: &Bac, person_id: Uuid) -> kernel::auth::Perimeter {
    let scope = kernel::auth::require_perimeter(bac.pool(), person_id)
        .await
        .expect("périmètre non vide");
    kernel::auth::Perimeter { person_id, scope }
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

// -----------------------------------------------------------------------------
// Ce qu'un dossier permet d'observer
// -----------------------------------------------------------------------------

/// Un dossier posé **directement en base**, sans passer par le service.
///
/// Il sert aux tests qui éprouvent ce qui vient APRÈS le dépôt — périmètre,
/// voile, transitions. Les tests du dépôt lui-même, eux, appellent le service :
/// poser la ligne à la main y masquerait précisément ce qu'ils vérifient.
pub async fn dossier(bac: &Bac, terrain: &Terrain, titre: &str, slug: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (call_id, event_id, organization_id, submitted_by,
                title, slug, objectives, detailed_presentation, format)
           VALUES ($1, $2, $3, $4,
                   jsonb_build_object('fr', $5::text),
                   $6::text::platform.slug,
                   '{"fr":"Objectifs du dossier."}'::jsonb,
                   '{"fr":"<p>Présentation.</p>"}'::jsonb,
                   'hybrid')
        RETURNING id"#,
        terrain.appel,
        terrain.edition,
        terrain.organisation,
        terrain.deposante,
        titre,
        slug
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier")
}

/// Les événements de l'outbox d'un agrégat, dans l'ordre. Les **compter** est
/// le seul contrôle qui dise quelque chose d'un doublon : vérifier leur
/// présence n'en dit rien — et c'est tout l'enjeu de ce module, dont le
/// déclencheur émet déjà.
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

/// Le journal des transitions d'un dossier, dans l'ordre.
pub async fn journal(bac: &Bac, proposal_id: Uuid) -> Vec<(Option<String>, String)> {
    sqlx::query!(
        r#"SELECT from_status::text AS "depuis?", to_status::text AS "vers!"
             FROM programme.proposal_transitions
            WHERE proposal_id = $1 ORDER BY occurred_at, id"#,
        proposal_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture du journal")
    .into_iter()
    .map(|l| (l.depuis, l.vers))
    .collect()
}

/// Un instant lu tel que le formulaire l'envoie : RFC 3339, fuseau compris.
pub fn instant(rfc3339: &str) -> OffsetDateTime {
    OffsetDateTime::parse(rfc3339, &time::format_description::well_known::Rfc3339)
        .expect("instant RFC 3339")
}

// -----------------------------------------------------------------------------
// De quoi appeler le service d'écriture du brouillon
// -----------------------------------------------------------------------------

use programme::domain::draft::{DraftOrganization, DraftSpeaker, ProposalDraft, SaveDraftPayload};

/// Un brouillon **valide et minimal**. Chaque test part de là et ne modifie que
/// ce qu'il éprouve : c'est ce qui fait qu'un refus attendu ne peut pas venir
/// d'ailleurs.
pub fn brouillon(terrain: &Terrain, titre: &str) -> ProposalDraft {
    ProposalDraft {
        organization_id: Some(terrain.organisation),
        co_organizations: Vec::new(),
        title: titre.to_owned(),
        summary: "Un résumé.".to_owned(),
        objectives: "Les objectifs de l'activité.".to_owned(),
        detailed_presentation: "<p>Une présentation détaillée.</p>".to_owned(),
        expected_outcomes: String::new(),
        target_audiences: vec!["Ministères".to_owned()],
        theme_codes: vec!["adaptation".to_owned()],
        activity_type_code: Some("results_sharing".to_owned()),
        format: Some("hybrid".to_owned()),
        language_codes: vec!["fr".to_owned()],
        country_id: None,
        speakers: Vec::new(),
        preferred_start_at: None,
        duration_minutes: None,
        requested_sessions: 1,
        scheduling_constraints: String::new(),
    }
}

/// La charge utile d'enregistrement, telle que le formulaire l'envoie.
pub fn charge(terrain: &Terrain, brouillon: ProposalDraft) -> SaveDraftPayload {
    SaveDraftPayload {
        proposal_id: None,
        call_id: terrain.appel,
        // Le front l'envoie ; l'API ne le lit pas.
        event_id: Some(terrain.edition),
        draft: brouillon,
    }
}

/// Un intervenant **valide et minimal**.
pub fn intervenant(email: &str, prenom: &str, nom: &str) -> DraftSpeaker {
    DraftSpeaker {
        person_id: None,
        civility: None,
        first_name: prenom.to_owned(),
        last_name: nom.to_owned(),
        email: email.to_owned(),
        job_title: "Directrice".to_owned(),
        organization_name: "IFDD".to_owned(),
        organization_id: None,
        role: "speaker".to_owned(),
        bio: String::new(),
    }
}

/// Une co-organisation **valide et minimale**.
pub fn coorganisation(organization_id: Uuid, role: &str) -> DraftOrganization {
    DraftOrganization {
        organization_id,
        role: role.to_owned(),
    }
}

/// Donner un compte mot de passe à une personne : c'est ce qui **verrouille son
/// identité** vis-à-vis d'un déposant.
pub async fn donner_un_compte(bac: &Bac, person_id: Uuid) {
    sqlx::query!(
        "INSERT INTO identity.accounts (person_id, provider, password_hash)
         VALUES ($1, 'password', '$argon2id$v=19$m=1,t=1,p=1$c2VsMTIzNDU2$empreinte')",
        person_id
    )
    .execute(bac.pool())
    .await
    .expect("insertion du compte");
}

/// La ligne du dossier, telle que la base la porte. Les tests relisent en base
/// plutôt que de croire la réponse : c'est le seul contrôle qui dise quelque
/// chose de ce qui a été écrit.
pub struct LigneDuDossier {
    pub reference_code: String,
    pub slug: String,
    pub status: String,
    pub title_fr: String,
    pub presentation_fr: String,
    pub contact_person_id: Option<Uuid>,
    pub duration_minutes: Option<i16>,
}

pub async fn ligne(bac: &Bac, proposal_id: Uuid) -> LigneDuDossier {
    let l = sqlx::query!(
        r#"SELECT reference_code, slug::text AS "slug!", status::text AS "status!",
                  title ->> 'fr' AS "titre!",
                  detailed_presentation ->> 'fr' AS "presentation!",
                  contact_person_id, duration_minutes
             FROM programme.proposals WHERE id = $1"#,
        proposal_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture du dossier");

    LigneDuDossier {
        reference_code: l.reference_code,
        slug: l.slug,
        status: l.status,
        title_fr: l.titre,
        presentation_fr: l.presentation,
        contact_person_id: l.contact_person_id,
        duration_minutes: l.duration_minutes,
    }
}

/// Le créneau **relu en heure murale**, dans le fuseau de l'édition. C'est
/// l'aller-retour qui compte : un écart de trois heures ici ne serait signalé
/// par rien.
pub async fn creneau_mural(bac: &Bac, proposal_id: Uuid, fuseau: &str) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT to_char(preferred_start_at AT TIME ZONE $2, 'YYYY-MM-DD\"T\"HH24:MI')
           FROM programme.proposals WHERE id = $1",
        proposal_id,
        fuseau
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture du créneau")
}

/// Les thématiques posées, **avec leur triplet d'entité** : c'est le triplet
/// qu'on vérifie, pas seulement les codes.
pub async fn thematiques(bac: &Bac, proposal_id: Uuid) -> Vec<(String, String, String)> {
    sqlx::query!(
        r#"SELECT et.entity_schema, et.entity_table, t.code
             FROM reference.entity_terms et
             JOIN reference.taxonomy_terms t ON t.id = et.term_id
            WHERE et.entity_id = $1
            ORDER BY et.sort_order, t.code"#,
        proposal_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des thématiques")
    .into_iter()
    .map(|l| (l.entity_schema, l.entity_table, l.code))
    .collect()
}

/// Les organisations associées, avec leur rôle.
pub async fn associations(bac: &Bac, proposal_id: Uuid) -> Vec<(Uuid, String)> {
    sqlx::query!(
        r#"SELECT organization_id, role::text AS "role!"
             FROM programme.proposal_organizations
            WHERE proposal_id = $1 ORDER BY role = 'lead' DESC, sort_order"#,
        proposal_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des organisations")
    .into_iter()
    .map(|l| (l.organization_id, l.role))
    .collect()
}

/// La fiche d'une personne, telle que la base la porte.
pub async fn fiche(bac: &Bac, email: &str) -> Option<(Uuid, String, String)> {
    sqlx::query!(
        r#"SELECT id, first_name, last_name FROM identity.people
            WHERE primary_email = $1::text::platform.email"#,
        email
    )
    .fetch_optional(bac.pool())
    .await
    .expect("lecture de la personne")
    .map(|l| (l.id, l.first_name, l.last_name))
}

// -----------------------------------------------------------------------------
// Les trois personnes que la machine à états distingue
// -----------------------------------------------------------------------------

/// Les trois lecteurs d'un même dossier, et **c'est leur différence qui compte**.
///
/// Une transition est offerte à qui est **porteur** et que la règle y autorise,
/// **ou** à qui détient la permission qu'elle nomme **sur l'édition du
/// dossier**. Trois comptes, trois réponses.
pub struct Droits {
    /// Membre actif de l'organisation porteuse. **Aucun périmètre
    /// d'administration** : une organisation n'administre rien.
    pub deposante: Uuid,
    /// Rôle `reviewer` sur l'édition : `programme.review.write`. C'est lui qui
    /// demande des corrections.
    pub noteur: Uuid,
    /// Rôle `admin` sur l'édition : `programme.proposal.decide`. **Il ne détient
    /// PAS `programme.review.write`** — écart n° 50, tranché en A8 : une ligne
    /// de la table des droits, pas une fatalité du code.
    pub decideur: Uuid,
}

pub async fn droits(bac: &Bac, terrain: &Terrain) -> Droits {
    let noteur = personne(bac, "noteur@ifdd.francophonie.org", "Nadia", "Toure").await;
    attribuer(bac, noteur, "reviewer", "event", Some(terrain.edition)).await;

    let decideur = personne(bac, "decideur@ifdd.francophonie.org", "Denis", "Kabore").await;
    attribuer(bac, decideur, "admin", "event", Some(terrain.edition)).await;

    Droits {
        deposante: terrain.deposante,
        noteur,
        decideur,
    }
}

/// Le motif porté par la **colonne** du dossier — celui de la dernière
/// transition, et rien de plus (écart n° 97).
pub async fn motif_en_colonne(bac: &Bac, proposal_id: Uuid) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT decision_reason FROM programme.proposals WHERE id = $1",
        proposal_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture du motif")
}

/// Le journal **avec ses motifs et ses auteurs**, du plus ancien au plus récent.
pub async fn journal_complet(
    bac: &Bac,
    proposal_id: Uuid,
) -> Vec<(Option<String>, String, Option<String>, Option<Uuid>)> {
    sqlx::query!(
        r#"SELECT from_status::text AS "depuis?", to_status::text AS "vers!",
                  reason, actor_id
             FROM programme.proposal_transitions
            WHERE proposal_id = $1 ORDER BY occurred_at, id"#,
        proposal_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture du journal")
    .into_iter()
    .map(|l| (l.depuis, l.vers, l.reason, l.actor_id))
    .collect()
}
