//! Ce que les tests d'intégration du module partagent : une base jetable, un
//! état de module, et **de quoi programmer un rappel**.
//!
//! Aucun double en mémoire (principe X).
//!
//! # Le semis ne fournit ni règle, ni modèle de message — et c'est délibéré
//!
//! `900_seed.sql` sème le **catalogue** des types de notification, et rien
//! d'autre : aucune règle de rappel, aucun modèle, aucune révision publiée.
//! Chaque test pose les siens. Une règle semée serait une règle dont la portée
//! et les décalages seraient partagés entre des tests qui les modifient — et la
//! clé d'unicité du modèle, une par édition et une par séance, rendrait le
//! partage particulièrement piégeux.
//!
//! # La séance est datée DANS LE FUTUR, et ce n'est pas un détail
//!
//! `engagement.schedule_session_reminders()` n'insère que les rappels dont
//! l'instant d'envoi est **encore devant** — « on ne réveille personne à 3 h du
//! matin parce qu'un import a pris du retard ». Une séance datée dans le passé
//! ne produirait donc aucune ligne, et le test échouerait sur une règle qui
//! fonctionne parfaitement.
//!
//! # Pourquoi la **vraie application** n'est pas montée ici
//!
//! Même raison que dans les cinq modules précédents : la monter demanderait une
//! dépendance de développement vers `api`, et
//! `cargo tree -p engagement | grep -E 'media|identity|org|event|programme'`
//! doit ne **rien** rendre. Les tests de routes vivent dans `crates/api/tests/`.

#![allow(dead_code)]

use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::events::OutboxEvent;
use kernel::mail::{MailError, Mailer, OutgoingMail};
use kernel::testing::TestDb;
use kernel::Db;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use engagement::state::EngagementState;

pub struct Bac {
    pub base: TestDb,
    pub state: EngagementState,
    pub config: Arc<Config>,
    /// L'expéditeur de fin de chaîne : il retient ce qui lui est remis au lieu
    /// de l'envoyer. **Ce n'est pas un mock de base** — le principe X interdit
    /// de simuler PostgreSQL, pas de remplacer un serveur SMTP par un carnet.
    pub boite: Arc<BoiteAuxLettres>,
}

impl Bac {
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = Arc::new(kernel::testing::test_config(base.url()));
        let boite = Arc::new(BoiteAuxLettres::default());
        // **Le décorateur enveloppe la boîte**, exactement comme il enveloppe
        // le relais en production : c'est lui qu'on éprouve, pas l'expéditeur.
        let mailer: Arc<dyn Mailer> = Arc::new(engagement::GardedMailer::new(
            boite.clone(),
            base.db(),
            "test",
        ));
        let state = EngagementState::new(base.db(), config.clone(), mailer);

        Self {
            base,
            state,
            config,
            boite,
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

/// Ce qui a été remis à l'expéditeur réel — donc ce qui serait **parti**.
#[derive(Default)]
pub struct BoiteAuxLettres {
    remis: Mutex<Vec<OutgoingMail>>,
}

impl BoiteAuxLettres {
    pub fn messages(&self) -> Vec<OutgoingMail> {
        self.remis.lock().expect("boîte lisible").clone()
    }

    pub fn compte(&self) -> usize {
        self.remis.lock().expect("boîte lisible").len()
    }

    pub fn vider(&self) {
        self.remis.lock().expect("boîte lisible").clear();
    }
}

#[async_trait::async_trait]
impl Mailer for BoiteAuxLettres {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError> {
        self.remis
            .lock()
            .expect("boîte inscriptible")
            .push(mail.clone());
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// La fabrique — une édition, une séance à venir, des inscrits
// -----------------------------------------------------------------------------

/// Tout ce qu'un test de rappel a besoin de connaître.
pub struct Terrain {
    pub edition: Uuid,
    /// La séance de référence, **datée dans le futur**.
    pub seance: Uuid,
    /// L'organisation qui l'anime, par le dossier dont elle est issue.
    pub organisation: Uuid,
    /// Membre **active** de cette organisation : c'est elle qui a le droit de
    /// lire le calendrier des rappels.
    pub animatrice: Uuid,
    /// Une personne sans lien : la cible naturelle d'une URL forgée.
    pub etrangere: Uuid,
    /// Une personne qui **administre cette édition-là**, et aucune autre : c'est
    /// elle qui paramètre les rappels. Ni une adhésion, ni une permission
    /// globale — la permission **sur la portée de l'édition**, et le périmètre
    /// d'administration qui va avec (règle métier n° 8).
    pub administratrice: Uuid,
    /// Les inscrits de la séance, à l'état « inscrit ».
    pub inscrits: Vec<Uuid>,
}

/// Le nombre de jours d'avance de la séance de référence. Assez pour que les
/// quatre décalages du défaut du modèle — deux jours au plus lointain — soient
/// tous encore devant.
pub const JOURS_DAVANCE: i64 = 30;

pub async fn terrain(bac: &Bac) -> Terrain {
    let organisation = organisation(bac, "Réseau ouest-africain climat", "ROAC").await;
    let animatrice = personne(bac, "animatrice@example.org", "Awa", "Sow Fall").await;
    let etrangere = personne(bac, "etrangere@example.org", "Léa", "Perret").await;
    adherer(bac, organisation, animatrice, "active").await;

    let edition = edition_cop31(bac).await;
    let dossier = dossier_retenu(bac, edition, organisation, animatrice).await;
    let seance = seance_a_venir(bac, edition, dossier).await;

    let mut inscrits = Vec::new();
    for (i, prenom) in ["Karim", "Fatou", "Jean"].into_iter().enumerate() {
        let personne = personne(
            bac,
            Box::leak(format!("inscrit{i}@example.org").into_boxed_str()),
            prenom,
            "Participant",
        )
        .await;
        inscrire(bac, seance, personne, "registered").await;
        inscrits.push(personne);
    }

    let administratrice = personne(bac, "admin@ifdd.org", "Sylvie", "Nomo").await;
    attribuer(bac, administratrice, "admin", "event", Some(edition)).await;

    Terrain {
        edition,
        seance,
        organisation,
        animatrice,
        etrangere,
        administratrice,
        inscrits,
    }
}

/// **Qui répond des courriels de la plateforme.**
///
/// La permission de gérer les modèles et la liste de suppression se tient sur la
/// portée **globale** : ces objets servent toutes les éditions à la fois, et
/// l'administratrice d'**une** COP ne les détient donc pas. C'est la seule garde
/// de ce module qui ne soit ni un périmètre d'édition ni une adhésion.
pub async fn redactrice(bac: &Bac) -> Uuid {
    let personne = personne(bac, "redaction@ifdd.org", "Miriam", "Kaboré").await;
    attribuer(bac, personne, "admin", "global", None).await;
    personne
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

/// Matérialise les rappels d'une séance — **la fonction du modèle**, appelée
/// telle quelle. C'est elle qui met un travail par rappel en file et qui émet
/// son annonce ; aucun code de ce module ne redouble l'un ou l'autre.
pub async fn materialiser_les_rappels(bac: &Bac, session_id: Uuid) -> i32 {
    sqlx::query_scalar!(
        r#"SELECT engagement.schedule_session_reminders($1) AS "crees!""#,
        session_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("matérialisation des rappels")
}

/// Le nombre de rappels d'une séance dans un état donné.
pub async fn compter_rappels(bac: &Bac, session_id: Uuid, statut: &str) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!"
             FROM engagement.scheduled_reminders
            WHERE session_id = $1 AND status::text = $2"#,
        session_id,
        statut
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des rappels")
}

pub async fn organisation(bac: &Bac, nom: &str, sigle: &str) -> Uuid {
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

pub async fn adherer(bac: &Bac, organisation: Uuid, personne: Uuid, statut: &str) {
    sqlx::query!(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, approved_at)
         VALUES ($1, $2, 'member', $3::text::org.membership_status, now())",
        organisation,
        personne,
        statut
    )
    .execute(bac.pool())
    .await
    .expect("insertion de l'adhésion");
}

pub async fn edition_cop31(bac: &Bac) -> Uuid {
    let serie = sqlx::query_scalar!("SELECT id FROM event.event_series WHERE code = 'cop_climate'")
        .fetch_one(bac.pool())
        .await
        .expect("série climat du semis");

    // `ck_events_physical_location` exige un pays et une ville dès que l'édition
    // n'est pas entièrement en ligne : une COP se tient quelque part.
    let bresil = sqlx::query_scalar!("SELECT id FROM reference.countries WHERE iso3 = 'BRA'")
        .fetch_one(bac.pool())
        .await
        .expect("Brésil du référentiel");

    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at,
                country_id, city, has_pavilion)
           VALUES ($1, 'COP31', 2027,
                   '{"fr":"COP31 — Conférence des Parties","en":"COP31"}'::jsonb,
                   'COP31', 'cop31-belem'::platform.slug,
                   '{"fr":"Pavillon de la Francophonie à la COP31.","en":"Francophonie pavilion."}'::jsonb,
                   'announced', 'hybrid', 'America/Belem'::platform.timezone_name,
                   now() + interval '20 days', now() + interval '32 days',
                   $2, 'Belém', true)
        RETURNING id"#,
        serie,
        bresil
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la COP31")
}

/// Un dossier retenu, porté par l'organisation : c'est lui qui rattache la
/// séance à son animatrice.
pub async fn dossier_retenu(
    bac: &Bac,
    event_id: Uuid,
    organisation: Uuid,
    deposante: Uuid,
) -> Uuid {
    let call_id = sqlx::query_scalar!(
        r#"INSERT INTO event.calls_for_proposals
               (event_id, code, title, status, opens_at, closes_at, required_reviews)
           VALUES ($1, 'principal',
                   '{"fr":"Appel à propositions","en":"Call for proposals"}'::jsonb,
                   'open', now() - interval '1 day', now() + interval '30 days', 2)
        RETURNING id"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'appel");

    // La ligne de porteur de `proposal_organizations` est posée par un
    // DÉCLENCHEUR depuis `proposals.organization_id` : l'écrire à la main la
    // dupliquerait, ou la ferait diverger de la colonne qui fait foi.
    sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (event_id, call_id, organization_id, slug, title, summary,
                objectives, detailed_presentation, format, submitted_by, status, submitted_at)
           VALUES ($1, $2, $3, 'financer-adaptation'::platform.slug,
                   '{"fr":"Financer l''adaptation","en":"Financing adaptation"}'::jsonb,
                   '{"fr":"Un résumé.","en":"A summary."}'::jsonb,
                   '{"fr":"Des objectifs.","en":"Objectives."}'::jsonb,
                   '{"fr":"<p>Une présentation.</p>","en":"<p>A presentation.</p>"}'::jsonb,
                   'hybrid', $4, 'accepted', now())
        RETURNING id"#,
        event_id,
        call_id,
        organisation,
        deposante
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier")
}

/// La séance de référence, **datée dans le futur** : sans cela, la fonction du
/// modèle n'insère aucun rappel et le test échoue sur une règle qui marche.
pub async fn seance_a_venir(bac: &Bac, event_id: Uuid, proposal_id: Uuid) -> Uuid {
    seance_datee(bac, event_id, proposal_id, JOURS_DAVANCE).await
}

/// Une séance à `jours` d'ici — négatif pour la dater dans le passé.
pub async fn seance_datee(bac: &Bac, event_id: Uuid, proposal_id: Uuid, jours: i64) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, proposal_id, title, slug, status, format,
                starts_at, ends_at, timezone)
           VALUES ($1, $2,
                   '{"fr":"Financer l''adaptation","en":"Financing adaptation"}'::jsonb,
                   'financer-adaptation', 'scheduled', 'hybrid',
                   now() + make_interval(days => $3::int),
                   now() + make_interval(days => $3::int, hours => 2),
                   'America/Belem'::platform.timezone_name)
        RETURNING id"#,
        event_id,
        proposal_id,
        jours as i32
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la séance")
}

/// Une inscription, **créée directement à son état d'arrivée**.
///
/// C'est le chemin le plus courant, et c'est celui qu'une lecture du commentaire
/// du modèle aurait cassé : `programme.registration.confirmed` n'existe pas, et
/// le déclencheur émet `programme.registration.created` avec le statut en charge
/// utile (écart n° 126).
pub async fn inscrire(bac: &Bac, session_id: Uuid, person_id: Uuid, statut: &str) -> Uuid {
    // `ck_registrations_waitlist` lie le rang d'attente au statut : l'un ne va
    // pas sans l'autre, dans les deux sens.
    let rang = (statut == "waitlisted").then_some(1_i32);

    sqlx::query_scalar!(
        r#"INSERT INTO programme.registrations
               (session_id, person_id, status, locale, waitlist_position)
           VALUES ($1, $2, $3::text::programme.registration_status, 'fr', $4)
        RETURNING id"#,
        session_id,
        person_id,
        statut,
        rang
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'inscription")
}

/// Une règle de rappel d'édition. Le semis n'en fournit aucune.
pub async fn regle_dedition(bac: &Bac, event_id: Uuid, minutes: &[i32]) -> Uuid {
    regle(bac, Some(event_id), None, minutes).await
}

/// Une règle de rappel de séance — elle **remplace** celle de son édition, sans
/// cumul.
pub async fn regle_de_seance(bac: &Bac, session_id: Uuid, minutes: &[i32]) -> Uuid {
    regle(bac, None, Some(session_id), minutes).await
}

async fn regle(
    bac: &Bac,
    event_id: Option<Uuid>,
    session_id: Option<Uuid>,
    minutes: &[i32],
) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO engagement.reminder_rules (event_id, session_id, offsets, channels)
           VALUES ($1, $2,
                   (SELECT array_agg(make_interval(mins => m) ORDER BY m DESC)
                      FROM unnest($3::int[]) m),
                   '{email}')
        RETURNING id"#,
        event_id,
        session_id,
        minutes
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la règle de rappel")
}

/// Un modèle de message avec une révision **publiée**. Le semis n'en fournit
/// aucun : un type sans révision publiée part avec un texte de secours, et c'est
/// une situation qu'un test doit pouvoir provoquer **ou** éviter.
pub async fn modele_publie(
    bac: &Bac,
    cle: &str,
    type_code: &str,
    sujet: &str,
    corps: &str,
) -> Uuid {
    let template = sqlx::query_scalar!(
        r#"INSERT INTO engagement.message_templates (key, label, type_code)
           VALUES ($1::text::platform.slug,
                   jsonb_build_object('fr', $1::text, 'en', $1::text)::platform.i18n_text,
                   $2)
        RETURNING id"#,
        cle,
        type_code
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du modèle");

    sqlx::query!(
        r#"INSERT INTO engagement.template_versions
               (template_id, version, subject, body_html, published_at)
           VALUES ($1, 1,
                   jsonb_build_object('fr', $2::text, 'en', $2::text)::platform.i18n_text,
                   jsonb_build_object('fr', $3::text, 'en', $3::text)::platform.i18n_text,
                   now())"#,
        template,
        sujet,
        corps
    )
    .execute(bac.pool())
    .await
    .expect("insertion de la révision");

    sqlx::query!(
        "UPDATE engagement.message_templates SET current_version = 1 WHERE id = $1",
        template
    )
    .execute(bac.pool())
    .await
    .expect("publication du modèle");

    template
}

/// **L'heure venue** — les rappels d'une séance sont ramenés à maintenant.
///
/// Plus rapide qu'attendre, et tout aussi probant.
///
/// # L'instant vient de la BASE, jamais de l'horloge du test
///
/// La base tourne dans un conteneur, dont l'horloge dérive de celle de la
/// machine. Poser `run_at` depuis `OffsetDateTime::now_utc()` peut donc le
/// placer dans le **futur** de la base — `claim_jobs()` ne réserve alors rien,
/// le test attend un courriel qui ne peut pas partir, et il échoue **une fois
/// sur deux** sur une chaîne qui fonctionne. La dérive est de quelques
/// secondes, ce qui suffit.
///
/// # L'échéance du TRAVAIL est avancée avec celle du rappel
///
/// Un rappel porte deux horloges : la sienne, dans `scheduled_for`, et celle de
/// son travail, dans `platform.jobs.run_at`. `claim_jobs()` ne réserve que ce
/// dont l'échéance est passée : n'avancer que la première donnerait un test qui
/// attend un courriel qui ne peut pas partir.
pub async fn avancer_les_rappels(bac: &Bac, session_id: Uuid) {
    avancer(bac, session_id, None).await;
}

/// **Un seul décalage avancé**, les autres laissés en place.
///
/// Sans lui, une séance à trois décalages part d'un coup et l'on ne peut pas
/// éprouver ce qui change **entre deux vagues** — la révision servie, par
/// exemple.
pub async fn avancer_un_decalage(bac: &Bac, session_id: Uuid, minutes: i32) {
    avancer(bac, session_id, Some(minutes)).await;
}

/// `None` : tous les décalages.
async fn avancer(bac: &Bac, session_id: Uuid, minutes: Option<i32>) {
    let travaux = sqlx::query_scalar!(
        r#"UPDATE engagement.scheduled_reminders
              SET scheduled_for = now()
            WHERE session_id = $1
              AND status IN ('pending', 'queued')
              AND ($2::int IS NULL OR offset_before = make_interval(mins => $2))
        RETURNING job_id"#,
        session_id,
        minutes
    )
    .fetch_all(bac.pool())
    .await
    .expect("avance des rappels");

    let travaux: Vec<Uuid> = travaux.into_iter().flatten().collect();
    sqlx::query!(
        "UPDATE platform.jobs SET run_at = now() WHERE id = ANY($1)",
        &travaux
    )
    .execute(bac.pool())
    .await
    .expect("avance des travaux");
}

// -----------------------------------------------------------------------------
// Le relais d'outbox et la file, depuis un test
// -----------------------------------------------------------------------------

/// Les annonces que la base a **réellement** émises, dans l'ordre.
///
/// Elles sont lues telles quelles, jamais fabriquées à la main : c'est ce qui
/// fait que le test éprouve la charge utile du déclencheur — celle sur laquelle
/// le consommateur branche — et non celle qu'on aurait imaginée. Le piège de
/// l'écart n° 126 ne se voit qu'ainsi.
///
/// `non_relayees` : celles que le relais n'a pas encore prises.
pub async fn annonces(bac: &Bac, prefixe: &str, non_relayees: bool) -> Vec<OutboxEvent> {
    let lignes = sqlx::query!(
        "SELECT id, aggregate_schema, aggregate_type, aggregate_id, event_type,
                event_version, payload, metadata, correlation_id, occurred_at
           FROM platform.outbox_events
          WHERE event_type LIKE $1 || '%'
            AND ($2 = false OR published_at IS NULL)
          ORDER BY occurred_at, id",
        prefixe,
        non_relayees
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'outbox");

    lignes
        .into_iter()
        .map(|l| OutboxEvent {
            id: l.id,
            aggregate_schema: l.aggregate_schema,
            aggregate_type: l.aggregate_type,
            aggregate_id: l.aggregate_id,
            event_type: l.event_type,
            event_version: l.event_version,
            payload: l.payload,
            metadata: l.metadata,
            correlation_id: l.correlation_id,
            occurred_at: l.occurred_at,
        })
        .collect()
}

/// **Relaie les annonces d'un préfixe qui ne l'ont pas encore été**, une
/// transaction par annonce, comme le relais du worker — et les marque relayées.
///
/// La garde de rejeu de `platform.inbox_events` n'est **pas** posée ici, et
/// c'est délibéré : ce qui reste doit tenir tout seul. Un test qui rejoue
/// mesure alors l'idempotence de la fonction du modèle et celle de la file, et
/// non celle d'une garde qui les masquerait — voir [`rejouer_les_annonces`].
pub async fn relayer(bac: &Bac, prefixe: &str) -> usize {
    let annonces = annonces(bac, prefixe, true).await;
    let consommateurs = engagement::event_consumers(bac.db(), bac.state.mailer().clone());

    let mut traitees = 0;
    for annonce in &annonces {
        for consommateur in &consommateurs {
            if !consommateur.handles(&annonce.event_type) {
                continue;
            }
            let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
            consommateur
                .handle(&mut tx, annonce)
                .await
                .expect("relais de l'annonce");
            tx.commit().await.expect("validation");
            traitees += 1;
        }
        sqlx::query!(
            "UPDATE platform.outbox_events SET published_at = now() WHERE id = $1",
            annonce.id
        )
        .execute(bac.pool())
        .await
        .expect("marquage de l'annonce");
    }
    traitees
}

/// **Rejoue des annonces déjà relayées** — le cas que la garde d'inbox absorbe
/// en production, et qu'on veut ici mesurer sans elle.
pub async fn rejouer_les_annonces(bac: &Bac, prefixe: &str) -> usize {
    sqlx::query!(
        "UPDATE platform.outbox_events SET published_at = NULL
          WHERE event_type LIKE $1 || '%'",
        prefixe
    )
    .execute(bac.pool())
    .await
    .expect("annonces rendues au relais");

    relayer(bac, prefixe).await
}

/// **Un passage du worker, par le vrai chemin.**
///
/// Les travaux sont réservés par `platform.claim_jobs()` sur la file que le
/// gestionnaire déclare, exécutés, puis marqués comme le worker les marque.
/// Appeler le gestionnaire directement laisserait passer une file mal nommée —
/// un travail déposé dans une file qu'aucun worker n'écoute s'empile sans
/// erreur et sans trace, et c'est le défaut trouvé en phase 4.
///
/// **La réservation se fait par FILE, jamais par gestionnaire** : deux
/// gestionnaires peuvent nommer la même, et réserver au nom du premier lui
/// ferait tenir les travaux du second — réservés, non exécutés, invisibles au
/// tour suivant. C'est ce que fait le worker, qui parcourt les files et
/// distribue par tâche.
pub async fn passer_le_worker(bac: &Bac) -> Vec<kernel::error::Result<()>> {
    let gestionnaires = engagement::job_handlers(bac.db(), &bac.config, bac.state.mailer().clone());
    let mut issues = Vec::new();

    for file in files_de(&gestionnaires) {
        loop {
            let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
            let travaux = kernel::jobs::claim(&mut tx, &file, "test-worker", 50)
                .await
                .expect("réservation");
            tx.commit().await.expect("validation");

            if travaux.is_empty() {
                break;
            }

            for travail in travaux {
                let Some(gestionnaire) = gestionnaires.iter().find(|g| g.task() == travail.task)
                else {
                    continue;
                };
                let issue = gestionnaire.run(&travail).await;

                let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
                match &issue {
                    Ok(()) => kernel::jobs::succeed(&mut tx, travail.id)
                        .await
                        .expect("succès"),
                    Err(e) => kernel::jobs::fail(&mut tx, travail.id, &e.to_string())
                        .await
                        .expect("échec"),
                }
                tx.commit().await.expect("validation");
                issues.push(issue);
            }
        }
    }

    issues
}

/// Les files déclarées, dédoublonnées — le `queues()` du registre du worker.
fn files_de(gestionnaires: &[std::sync::Arc<dyn kernel::jobs::JobHandler>]) -> Vec<String> {
    let mut files: Vec<String> = gestionnaires.iter().map(|g| g.queue().to_owned()).collect();
    files.sort();
    files.dedup();
    files
}

/// **Le worker tué entre l'exécution et son marquage.** Les travaux restent
/// `running`, exactement comme après un `Ctrl-C` en cours de lot.
pub async fn worker_tue_apres_le_travail(bac: &Bac) -> Vec<kernel::error::Result<()>> {
    let gestionnaires = engagement::job_handlers(bac.db(), &bac.config, bac.state.mailer().clone());
    let mut issues = Vec::new();

    for file in files_de(&gestionnaires) {
        let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
        let travaux = kernel::jobs::claim(&mut tx, &file, "test-worker", 50)
            .await
            .expect("réservation");
        tx.commit().await.expect("validation");

        for travail in travaux {
            if let Some(gestionnaire) = gestionnaires.iter().find(|g| g.task() == travail.task) {
                issues.push(gestionnaire.run(&travail).await);
            }
        }
    }

    issues
}

/// **Le worker relancé rend à la file ce que le précédent a laissé réservé.**
pub async fn worker_relance(bac: &Bac) {
    let gestionnaires = engagement::job_handlers(bac.db(), &bac.config, bac.state.mailer().clone());
    let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
    for file in files_de(&gestionnaires) {
        kernel::jobs::reclaim_stalled(&mut tx, &file, 0.0)
            .await
            .expect("reprise des travaux bloqués");
    }
    tx.commit().await.expect("validation");
}

/// Rejoue les travaux d'une tâche : c'est le geste du quickstart, écrit une
/// fois — `UPDATE platform.jobs SET status='queued'`.
pub async fn rejouer_les_travaux(bac: &Bac, tache: &str) -> u64 {
    sqlx::query!(
        "UPDATE platform.jobs
            SET status = 'queued', locked_at = NULL, locked_by = NULL, completed_at = NULL
          WHERE task = $1 AND status <> 'queued'",
        tache
    )
    .execute(bac.pool())
    .await
    .expect("rejeu des travaux")
    .rows_affected()
}

/// Le nombre de travaux d'une tâche, tous états confondus.
pub async fn compter_travaux(bac: &Bac, tache: &str) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs WHERE task = $1"#,
        tache
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des travaux")
}

/// Le nombre d'annonces d'un type dans l'outbox.
pub async fn compter_annonces(bac: &Bac, event_type: &str) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events WHERE event_type = $1"#,
        event_type
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des annonces")
}

/// Le motif écrit sur les rappels d'une séance, s'il n'y en a qu'un.
pub async fn motifs_des_rappels(bac: &Bac, session_id: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT DISTINCT skip_reason FROM engagement.scheduled_reminders
          WHERE session_id = $1 AND skip_reason IS NOT NULL
          ORDER BY skip_reason",
        session_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des motifs")
    .into_iter()
    .flatten()
    .collect()
}
