//! La régie du jour (02/11), éprouvée de bout en bout : les gestes du module
//! `programme`, le relais des annonces et le travail d'envoi d'`engagement`,
//! le report de l'annulation Zoom de `live`. Les courriels se lisent dans la
//! file : un courriel posé est un travail `engagement.notification_email` de
//! clé `regie:<geste>:<personne>`.

use engagement::consumers::regie::RegieConsumer;
use engagement::jobs::regie_notice::RegieNotice;
use kernel::auth::Perimeter;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode};
use kernel::events::{EventConsumer, OutboxEvent};
use kernel::jobs::{ClaimedJob, JobHandler};
use kernel::testing::TestDb;
use programme::domain::regie::{RegieCancelPayload, RegieUndoPayload};
use programme::service::{regie, regie_ecran};
use programme::state::ProgrammeState;
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

struct Bac {
    base: TestDb,
    state: ProgrammeState,
}

impl Bac {
    async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = Arc::new(kernel::testing::test_config(base.url()));
        let state = ProgrammeState::new(base.db(), config);
        Self { base, state }
    }

    fn pool(&self) -> &sqlx::PgPool {
        self.base.pool()
    }

    fn ctx(&self, acteur: Uuid) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr").with_actor(acteur)
    }
}

async fn edition(bac: &Bac, slug: &str) -> Uuid {
    sqlx::query(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at, country_id, city, has_pavilion)
           SELECT (SELECT id FROM event.event_series WHERE code = 'cop_climate'), $1, 2026,
                  '{"fr":"CdP31","en":"COP31"}'::jsonb, 'CdP31', $1::platform.slug,
                  '{"fr":"Pavillon.","en":"Pavilion."}'::jsonb, 'announced', 'hybrid',
                  'Europe/Istanbul'::platform.timezone_name, now() - interval '2 days',
                  now() + interval '5 days', (SELECT id FROM reference.countries WHERE iso3 = 'TUR'),
                  'Antalya', true
        RETURNING id"#,
    )
    .bind(slug)
    .fetch_one(bac.pool())
    .await
    .expect("édition")
    .get(0)
}

async fn personne(bac: &Bac, email: &str) -> Uuid {
    sqlx::query(
        "INSERT INTO identity.people (primary_email, first_name, last_name, email_verified_at, status)
         VALUES ($1::text::platform.email, 'Aïcha', 'Kone', now(), 'active') RETURNING id",
    )
    .bind(email)
    .fetch_one(bac.pool())
    .await
    .expect("personne")
    .get(0)
}

async fn admin_de(bac: &Bac, email: &str, event_id: Uuid) -> Perimeter {
    let id = personne(bac, email).await;
    sqlx::query(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
         VALUES ($1, 'admin', 'event', $2)",
    )
    .bind(id)
    .bind(event_id)
    .execute(bac.pool())
    .await
    .expect("rôle");
    let scope = kernel::auth::require_perimeter(bac.pool(), id)
        .await
        .expect("périmètre");
    Perimeter {
        person_id: id,
        scope,
    }
}

/// Une séance publiée, programmée, qui commence dans `debut` minutes.
async fn seance(bac: &Bac, event_id: Uuid, slug: &str, debut: i32) -> Uuid {
    sqlx::query(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, status, starts_at, ends_at, is_streamed, format, timezone,
                published_at, registration_required)
           VALUES ($1, '{"fr":"Atelier mangroves","en":"Mangrove workshop"}', $2::platform.slug,
                   'scheduled', now() + make_interval(mins => $3),
                   now() + make_interval(mins => $3 + 60), true, 'hybrid', 'Europe/Istanbul', now(), true)
        RETURNING id"#,
    )
    .bind(event_id)
    .bind(slug)
    .bind(debut)
    .fetch_one(bac.pool())
    .await
    .expect("séance")
    .get(0)
}

async fn inscrire(bac: &Bac, session_id: Uuid, n: usize) {
    for i in 0..n {
        let p = personne(bac, &format!("inscrit{i}-{session_id}@example.org")).await;
        sqlx::query("INSERT INTO programme.registrations (session_id, person_id) VALUES ($1, $2)")
            .bind(session_id)
            .bind(p)
            .execute(bac.pool())
            .await
            .expect("inscription");
    }
}

async fn statut(bac: &Bac, session_id: Uuid) -> String {
    sqlx::query("SELECT status::text FROM programme.sessions WHERE id = $1")
        .bind(session_id)
        .fetch_one(bac.pool())
        .await
        .expect("statut")
        .get(0)
}

/// Relaie les annonces de la régie vers `engagement`, comme le worker.
async fn relayer(bac: &Bac) {
    let lignes = sqlx::query(
        "SELECT id, aggregate_schema, aggregate_type, aggregate_id, event_type, event_version,
                payload, metadata, correlation_id, occurred_at
           FROM platform.outbox_events WHERE event_type LIKE 'programme.regie.%'
          ORDER BY occurred_at, id",
    )
    .fetch_all(bac.pool())
    .await
    .expect("outbox");
    let consommateur = RegieConsumer;
    for l in lignes {
        let evenement = OutboxEvent {
            id: l.get("id"),
            aggregate_schema: l.get("aggregate_schema"),
            aggregate_type: l.get("aggregate_type"),
            aggregate_id: l.get("aggregate_id"),
            event_type: l.get("event_type"),
            event_version: l.get("event_version"),
            payload: l.get("payload"),
            metadata: l.get("metadata"),
            correlation_id: l.get("correlation_id"),
            occurred_at: l.get("occurred_at"),
        };
        let mut conn = bac.pool().acquire().await.expect("connexion");
        if !kernel::events::claim(&mut conn, consommateur.name(), evenement.id)
            .await
            .expect("garde")
        {
            continue;
        }
        consommateur
            .handle(&mut conn, &evenement)
            .await
            .expect("annonce relayée");
    }
}

/// Fait tourner, comme si l'heure était venue, les travaux d'envoi encore en file.
async fn envoyer(bac: &Bac) {
    let lignes = sqlx::query(
        "UPDATE platform.jobs SET status = 'succeeded', completed_at = now()
          WHERE task = 'engagement.regie_notice' AND status = 'queued'
        RETURNING id, queue, task, payload, attempts, max_attempts",
    )
    .fetch_all(bac.pool())
    .await
    .expect("travaux");
    let travail = RegieNotice::new(bac.base.db());
    for l in lignes {
        let job = ClaimedJob {
            id: l.get("id"),
            queue: l.get("queue"),
            task: l.get("task"),
            payload: l.get("payload"),
            attempts: l.get("attempts"),
            max_attempts: l.get("max_attempts"),
        };
        travail.run(&job).await.expect("envoi");
    }
}

async fn courriels(bac: &Bac, prefixe: &str) -> i64 {
    sqlx::query(
        "SELECT count(*) FROM platform.jobs
          WHERE task = 'engagement.notification_email' AND idempotency_key LIKE $1 || '%'",
    )
    .bind(prefixe)
    .fetch_one(bac.pool())
    .await
    .expect("courriels")
    .get(0)
}

fn code(e: ApiError) -> ErrorCode {
    e.code
}

fn annulation(prevenir: bool) -> RegieCancelPayload {
    RegieCancelPayload {
        reason_code: "no_show".to_owned(),
        note: None,
        notify_registrants: prevenir,
    }
}

#[actix_web::test]
async fn annuler_puis_defaire_avant_lenvoi_ne_laisse_partir_aucun_courriel() {
    let bac = Bac::monter().await;
    let cop = edition(&bac, "cdp31-a").await;
    let admin = admin_de(&bac, "regie@example.org", cop).await;
    let s = seance(&bac, cop, "atelier-a", 30).await;
    inscrire(&bac, s, 3).await;

    let g = regie::annuler(
        &bac.state,
        &bac.ctx(admin.person_id),
        &admin,
        s,
        annulation(true),
    )
    .await
    .expect("annulation");
    assert_eq!(g.registrants, 3);
    assert!(g.notify_at.is_some(), "courriel dû dans cinq minutes");
    assert_eq!(statut(&bac, s).await, "cancelled");
    let absent: bool =
        sqlx::query("SELECT is_present FROM programme.session_presence WHERE session_id = $1")
            .bind(s)
            .fetch_one(bac.pool())
            .await
            .expect("présence")
            .get(0);
    assert!(!absent, "« pas venus » note les organisateurs absents");

    relayer(&bac).await;
    let du: bool = sqlx::query(
        "SELECT run_at > now() + interval '4 minutes' FROM platform.jobs WHERE task = 'engagement.regie_notice'",
    )
    .fetch_one(bac.pool())
    .await
    .expect("travail en file")
    .get(0);
    assert!(du, "l'envoi attend cinq minutes");

    regie::defaire(
        &bac.state,
        &bac.ctx(admin.person_id),
        &admin,
        g.action_id,
        RegieUndoPayload {
            notify_registrants: true,
        },
    )
    .await
    .expect("défaire");
    assert_eq!(statut(&bac, s).await, "scheduled");
    let presence: Option<bool> = sqlx::query_scalar(
        "SELECT is_present FROM programme.session_presence WHERE session_id = $1",
    )
    .bind(s)
    .fetch_optional(bac.pool())
    .await
    .expect("présence");
    assert_eq!(presence, None, "la présence redevient inconnue");

    relayer(&bac).await;
    envoyer(&bac).await;
    assert_eq!(courriels(&bac, &format!("regie:{}:", g.action_id)).await, 0);
    assert_eq!(
        courriels(&bac, "regie-maintenue:").await,
        0,
        "aucun correctif sans courriel parti"
    );

    let ecran = regie_ecran::ecran(&bac.state, &admin, cop, None)
        .await
        .expect("écran");
    let entree = ecran
        .journal
        .iter()
        .find(|e| e.id == g.action_id)
        .expect("au journal");
    assert!(entree.undone_at.is_some() && !entree.can_undo);
    let avis = entree.notice.as_ref().expect("état des courriels");
    assert_eq!(serde_json::to_value(avis.state).unwrap(), "withdrawn");
}

#[actix_web::test]
async fn annuler_puis_retablir_apres_lenvoi_ecrit_le_correctif() {
    let bac = Bac::monter().await;
    let cop = edition(&bac, "cdp31-b").await;
    let admin = admin_de(&bac, "regie@example.org", cop).await;
    let s = seance(&bac, cop, "atelier-b", 30).await;
    inscrire(&bac, s, 2).await;

    let g = regie::annuler(
        &bac.state,
        &bac.ctx(admin.person_id),
        &admin,
        s,
        annulation(true),
    )
    .await
    .expect("annulation");
    relayer(&bac).await;
    envoyer(&bac).await;
    assert_eq!(
        courriels(&bac, &format!("regie:{}:", g.action_id)).await,
        2,
        "les inscrits sont prévenus"
    );

    // L'échéance est passée : le correctif est demandé.
    sqlx::query(
        "UPDATE programme.control_room_actions
            SET data = jsonb_set(data, '{notify_at}', to_jsonb(to_char((now() - interval '1 minute') AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')))
          WHERE id = $1",
    )
    .bind(g.action_id)
    .execute(bac.pool())
    .await
    .expect("échéance passée");

    regie::defaire(
        &bac.state,
        &bac.ctx(admin.person_id),
        &admin,
        g.action_id,
        RegieUndoPayload {
            notify_registrants: true,
        },
    )
    .await
    .expect("rétablir");
    assert_eq!(statut(&bac, s).await, "scheduled");
    relayer(&bac).await;
    envoyer(&bac).await;
    assert_eq!(
        courriels(&bac, &format!("regie-maintenue:{}:", g.action_id)).await,
        2,
        "« finalement maintenue » à ceux qui ont reçu l'annulation"
    );
}

#[actix_web::test]
async fn un_seul_direct_a_la_fois_toutes_editions_confondues() {
    let bac = Bac::monter().await;
    let cop = edition(&bac, "cdp31-c").await;
    let autre = edition(&bac, "cdp16-c").await;
    let admin = admin_de(&bac, "regie@example.org", cop).await;
    let voisin = admin_de(&bac, "voisin@example.org", autre).await;
    let a = seance(&bac, cop, "atelier-c1", 0).await;
    let b = seance(&bac, cop, "atelier-c2", 60).await;
    let c = seance(&bac, autre, "atelier-c3", 0).await;

    regie::demarrer(&bac.state, &bac.ctx(admin.person_id), &admin, a)
        .await
        .expect("premier direct");
    let refus = regie::demarrer(&bac.state, &bac.ctx(admin.person_id), &admin, b)
        .await
        .expect_err("second direct refusé");
    assert_eq!(code(refus.clone()), ErrorCode::RegieLiveAlreadyRunning);
    assert!(
        refus.message.contains("Atelier mangroves"),
        "{}",
        refus.message
    );
    let ailleurs = regie::demarrer(&bac.state, &bac.ctx(voisin.person_id), &voisin, c)
        .await
        .expect_err("direct d'une autre édition refusé");
    assert_eq!(code(ailleurs.clone()), ErrorCode::RegieLiveAlreadyRunning);
    assert!(
        !ailleurs.message.contains("Atelier"),
        "le titre d'une autre édition ne fuit pas"
    );
    assert_eq!(statut(&bac, b).await, "scheduled");
}

#[actix_web::test]
async fn terminer_puis_reprendre_dans_les_quinze_minutes_et_refus_apres() {
    let bac = Bac::monter().await;
    let cop = edition(&bac, "cdp31-d").await;
    let admin = admin_de(&bac, "regie@example.org", cop).await;
    let s = seance(&bac, cop, "atelier-d", 0).await;
    let ctx = bac.ctx(admin.person_id);

    regie::demarrer(&bac.state, &ctx, &admin, s)
        .await
        .expect("direct");
    let fin = regie::terminer(&bac.state, &ctx, &admin, s)
        .await
        .expect("fin");
    assert!(fin.resumable_until.is_some());
    assert_eq!(statut(&bac, s).await, "completed");
    regie::defaire(
        &bac.state,
        &ctx,
        &admin,
        fin.action_id,
        RegieUndoPayload::default(),
    )
    .await
    .expect("reprise");
    assert_eq!(statut(&bac, s).await, "live");
    let debut: Option<time::OffsetDateTime> =
        sqlx::query_scalar("SELECT live_ended_at FROM programme.sessions WHERE id = $1")
            .bind(s)
            .fetch_one(bac.pool())
            .await
            .expect("bornes");
    assert_eq!(debut, None);

    let fin = regie::terminer(&bac.state, &ctx, &admin, s)
        .await
        .expect("seconde fin");
    sqlx::query(
        "UPDATE programme.sessions SET live_ended_at = now() - interval '16 minutes' WHERE id = $1",
    )
    .bind(s)
    .execute(bac.pool())
    .await
    .expect("seize minutes plus tard");
    let refus = regie::defaire(
        &bac.state,
        &ctx,
        &admin,
        fin.action_id,
        RegieUndoPayload::default(),
    )
    .await
    .expect_err("reprise refusée");
    assert_eq!(code(refus), ErrorCode::RegieResumeExpired);
    assert_eq!(statut(&bac, s).await, "completed");
}

#[actix_web::test]
async fn un_administrateur_dune_autre_edition_est_refuse() {
    let bac = Bac::monter().await;
    let cop = edition(&bac, "cdp31-e").await;
    let autre = edition(&bac, "cdp16-e").await;
    let admin = admin_de(&bac, "regie@example.org", cop).await;
    let voisin = admin_de(&bac, "voisin@example.org", autre).await;
    let s = seance(&bac, cop, "atelier-e", 30).await;

    let g = regie::confirmer_presence(&bac.state, &bac.ctx(admin.person_id), &admin, s)
        .await
        .expect("présence");
    let ctx = bac.ctx(voisin.person_id);
    for refus in [
        regie_ecran::ecran(&bac.state, &voisin, cop, None)
            .await
            .map(|_| ()),
        regie::confirmer_presence(&bac.state, &ctx, &voisin, s)
            .await
            .map(|_| ()),
        regie::annuler(&bac.state, &ctx, &voisin, s, annulation(false))
            .await
            .map(|_| ()),
        regie::demarrer(&bac.state, &ctx, &voisin, s)
            .await
            .map(|_| ()),
        regie::defaire(
            &bac.state,
            &ctx,
            &voisin,
            g.action_id,
            RegieUndoPayload::default(),
        )
        .await
        .map(|_| ()),
    ] {
        assert_eq!(
            code(refus.expect_err("hors périmètre")),
            ErrorCode::NotFound
        );
    }
    assert_eq!(statut(&bac, s).await, "scheduled");
}

#[actix_web::test]
async fn le_journal_garde_chaque_geste_et_son_retour_en_arriere() {
    let bac = Bac::monter().await;
    let cop = edition(&bac, "cdp31-f").await;
    let admin = admin_de(&bac, "regie@example.org", cop).await;
    let collegue = admin_de(&bac, "collegue@example.org", cop).await;
    let s = seance(&bac, cop, "atelier-f", 30).await;

    let presence = regie::confirmer_presence(&bac.state, &bac.ctx(admin.person_id), &admin, s)
        .await
        .expect("présence");
    regie::defaire(
        &bac.state,
        &bac.ctx(collegue.person_id),
        &collegue,
        presence.action_id,
        RegieUndoPayload::default(),
    )
    .await
    .expect("un collègue défait");
    let encore = regie::defaire(
        &bac.state,
        &bac.ctx(admin.person_id),
        &admin,
        presence.action_id,
        RegieUndoPayload::default(),
    )
    .await
    .expect_err("déjà défait");
    assert_eq!(code(encore), ErrorCode::RegieActionAlreadyUndone);
    let raison = regie::annuler(
        &bac.state,
        &bac.ctx(admin.person_id),
        &admin,
        s,
        RegieCancelPayload {
            reason_code: "inventee".to_owned(),
            note: None,
            notify_registrants: false,
        },
    )
    .await
    .expect_err("raison inconnue");
    assert_eq!(code(raison), ErrorCode::RegieCancellationReasonUnknown);
    let annule = regie::annuler(
        &bac.state,
        &bac.ctx(admin.person_id),
        &admin,
        s,
        RegieCancelPayload {
            reason_code: "other".to_owned(),
            note: Some("Salle inondée".to_owned()),
            notify_registrants: false,
        },
    )
    .await
    .expect("annulation");

    let ecran = regie_ecran::ecran(&bac.state, &admin, cop, None)
        .await
        .expect("écran");
    assert_eq!(ecran.journal.len(), 2);
    let premier = &ecran.journal[0];
    assert_eq!(premier.id, annule.action_id);
    assert!(premier.actor_is_me && premier.can_undo);
    let second = &ecran.journal[1];
    assert_eq!(second.undone_by.as_deref(), Some("Aïcha Kone"));
    let ligne = ecran
        .sessions
        .iter()
        .find(|x| x.session_id == s)
        .expect("séance du jour");
    let annulation = ligne.cancellation.as_ref().expect("annulée");
    assert_eq!(annulation.action_id, Some(annule.action_id));
    assert_eq!(annulation.note.as_deref(), Some("Salle inondée"));
    assert_eq!(annulation.reason_code.as_deref(), Some("other"));
    assert_eq!(ligne.last_action_id, Some(annule.action_id));
}
