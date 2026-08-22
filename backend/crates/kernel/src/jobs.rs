//! File de travaux différés : mise en file, réservation, échec.
//!
//! La reprise d'essai n'est pas réécrite : `platform.fail_job()` porte déjà le
//! délai croissant plafonné à une heure et le passage en file morte au-delà de
//! `max_attempts` (principe VIII).

use async_trait::async_trait;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::context::RequestContext;
use crate::error::Result;

pub const DEFAULT_QUEUE: &str = "default";

pub struct NewJob<'a> {
    pub queue: &'a str,
    /// Nom en deux segments, `module.action` — la forme de l'exemple porté par
    /// `COMMENT ON COLUMN platform.jobs.task`.
    pub task: &'a str,
    pub payload: Value,
    /// Clé d'unicité métier : empêche structurellement le double envoi.
    pub idempotency_key: Option<String>,
    pub run_at: Option<OffsetDateTime>,
}

impl<'a> NewJob<'a> {
    pub fn new(task: &'a str, payload: Value) -> Self {
        Self {
            queue: DEFAULT_QUEUE,
            task,
            payload,
            idempotency_key: None,
            run_at: None,
        }
    }

    pub fn idempotent(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    pub fn at(mut self, moment: OffsetDateTime) -> Self {
        self.run_at = Some(moment);
        self
    }
}

/// Mise en file **dans la transaction** du changement d'état. `None` : un
/// travail de même tâche et de même clé existe déjà, quel que soit son état
/// sauf `cancelled` (`ux_jobs_idempotency`) — y compris mort.
pub async fn enqueue(conn: &mut PgConnection, job: NewJob<'_>) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at)
         VALUES ($1, $2, $3, $4, COALESCE($5, now()))
         ON CONFLICT DO NOTHING
         RETURNING id",
        job.queue,
        job.task,
        job.payload,
        job.idempotency_key,
        job.run_at
    )
    .fetch_optional(conn)
    .await?;

    Ok(id)
}

#[derive(Debug, Clone)]
pub struct ClaimedJob {
    pub id: Uuid,
    pub queue: String,
    pub task: String,
    pub payload: Value,
    pub attempts: i16,
    pub max_attempts: i16,
}

impl ClaimedJob {
    /// Contexte d'écriture du travail : son identifiant sert d'identifiant de
    /// requête, ce qui relie la ligne d'audit au travail qui l'a produite.
    pub fn context(&self) -> RequestContext {
        RequestContext::new(format!("job:{}", self.id), "fr")
    }
}

pub async fn claim(
    conn: &mut PgConnection,
    queue: &str,
    worker_id: &str,
    limit: i32,
) -> Result<Vec<ClaimedJob>> {
    let rows = sqlx::query!(
        "SELECT id, queue, task, payload, attempts, max_attempts
           FROM platform.claim_jobs($1, $2, $3)",
        queue,
        worker_id,
        limit
    )
    .fetch_all(conn)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| ClaimedJob {
            id: r.id.expect("platform.jobs.id est NOT NULL"),
            queue: r.queue.expect("platform.jobs.queue est NOT NULL"),
            task: r.task.expect("platform.jobs.task est NOT NULL"),
            payload: r.payload.expect("platform.jobs.payload est NOT NULL"),
            attempts: r.attempts.expect("platform.jobs.attempts est NOT NULL"),
            max_attempts: r
                .max_attempts
                .expect("platform.jobs.max_attempts est NOT NULL"),
        })
        .collect())
}

/// Réussite : la charge utile est **vidée**. Le travail garde sa trace — un
/// courriel est parti, quand, après combien d'essais — sans garder son contenu,
/// qui porte le jeton en clair.
pub async fn succeed(conn: &mut PgConnection, job_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE platform.jobs
            SET status = 'succeeded',
                completed_at = now(),
                payload = '{}'::jsonb,
                locked_at = NULL,
                locked_by = NULL
          WHERE id = $1",
        job_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn fail(conn: &mut PgConnection, job_id: Uuid, erreur: &str) -> Result<()> {
    sqlx::query!("SELECT platform.fail_job($1, $2)", job_id, erreur)
        .execute(conn)
        .await?;
    Ok(())
}

/// Rend à la file les travaux qu'un worker tué a laissés réservés.
///
/// La base sait les VOIR — `ix_jobs_stuck` et l'alerte `travaux_bloques` de
/// `analytics.v_operational_health` — mais ne les répare pas : `claim_jobs()`
/// ne prend que les `queued`. Sans cette reprise, un `Ctrl-C` en cours de lot
/// perd définitivement les travaux réservés.
///
/// Le bail est franchement plus long que le seuil d'alerte de quinze minutes :
/// une variante d'image ou un rafraîchissement analytique peuvent légitimement
/// s'attarder, et les reprendre les ferait tourner DEUX FOIS en parallèle du
/// worker vivant.
pub async fn reclaim_stalled(
    conn: &mut PgConnection,
    queue: &str,
    lease_seconds: f64,
) -> Result<u64> {
    let reprises = sqlx::query!(
        "UPDATE platform.jobs
            SET status = 'queued',
                locked_at = NULL,
                locked_by = NULL,
                last_error = 'verrou expiré : travail repris, l''essai déjà compté n''est pas rendu'
          WHERE queue = $1
            AND status = 'running'
            AND locked_at < now() - make_interval(secs => $2)",
        queue,
        lease_seconds
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(reprises)
}

#[async_trait]
pub trait JobHandler: Send + Sync {
    fn task(&self) -> &'static str;

    /// La file où ce travail arrive.
    ///
    /// **Le worker n'écoute que les files que ses gestionnaires déclarent** :
    /// `platform.claim_jobs()` filtre strictement sur la file, et un travail
    /// déposé dans une file inécoutée s'empile sans erreur, sans trace, et sans
    /// que rien ne l'exécute jamais.
    ///
    /// Les déclencheurs du modèle nomment **quatre** files hors de la file par
    /// défaut — `media.tg_enqueue_processing()` dans « media », la
    /// matérialisation des rappels dans « email », le direct dans « live »,
    /// l'analytique dans « analytics ». Ce n'est donc pas un réglage de
    /// confort : c'est ce qui relie un déclencheur à son gestionnaire.
    fn queue(&self) -> &'static str {
        DEFAULT_QUEUE
    }

    /// La charge utile de cette tâche porte-t-elle un secret ?
    ///
    /// `succeed()` vide la charge utile de tout travail réussi ; un travail
    /// **mort**, lui, la garde — c'est la seule matière de diagnostic d'un
    /// envoi de variante d'image ou d'un rafraîchissement analytique en échec.
    /// Pour les tâches qui transportent un jeton en clair, ce diagnostic est un
    /// secret durable dans une table qu'on relit : elles le déclarent ici, et
    /// le worker efface leur charge utile en même temps qu'il constate leur
    /// mort.
    fn carries_secret(&self) -> bool {
        false
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()>;
}

/// **Déplace l'échéance de travaux encore en file**, du même décalage.
///
/// Un travail porte son propre `run_at`, indépendant de l'état métier qui l'a
/// produit : déplacer une séance sans déplacer ses travaux ferait partir les
/// rappels à l'ancienne heure, en silence — la ligne dirait l'heure nouvelle et
/// le courriel arriverait à l'ancienne.
///
/// Le décalage est **le même pour tous** : quand un créneau bouge, tous les
/// rappels qui en dépendent bougent d'autant. Un travail déjà pris, réussi ou
/// mort n'est pas touché — son échéance n'a plus de sens.
///
/// Vit ici et non dans un module : `platform.jobs` appartient au noyau, et un
/// module qui l'écrirait lui-même sortirait de son schéma.
pub async fn reschedule_by(conn: &mut PgConnection, job_ids: &[Uuid], seconds: f64) -> Result<u64> {
    if job_ids.is_empty() || seconds == 0.0 {
        return Ok(0);
    }

    let deplaces = sqlx::query!(
        "UPDATE platform.jobs
            SET run_at = run_at + make_interval(secs => $2)
          WHERE id = ANY($1) AND status = 'queued'",
        job_ids,
        seconds
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(deplaces)
}

/// Efface la charge utile d'un travail **mort**, et lui seul : un travail
/// simplement replanifié en a encore besoin pour son prochain essai.
pub async fn redact_dead(conn: &mut PgConnection, job_id: Uuid) -> Result<bool> {
    let efface = sqlx::query!(
        "UPDATE platform.jobs
            SET payload = '{}'::jsonb
          WHERE id = $1 AND status = 'dead' AND payload <> '{}'::jsonb",
        job_id
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(efface == 1)
}
