//! Journal d'exploitation (`platform.ops_log`) : pannes, refus, pages lentes,
//! travaux en échec.
//!
//! Écrit HORS de toute transaction métier. L'API passe par [`OpsLog`] : un canal
//! mémoire borné qu'une tâche vide toutes les deux secondes ; canal plein ou
//! écriture refusée, la ligne est perdue et comptée, jamais la requête ralentie.
//! Le worker, séquentiel, écrit directement par [`insert`].
//!
//! Ne jamais y mettre : corps ou chaîne de requête, en-têtes d'authentification,
//! mot de passe, adresse IP. `detail` passe par [`sanitize_detail`].

use serde::Serialize;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Borne de `ck_ops_log_detail`.
pub const DETAIL_MAX: usize = 2048;
const CAPACITE: usize = 2000;
const LOT_MAX: usize = 500;
const RYTHME: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpsKind {
    HttpError,
    HttpRefused,
    HttpSlow,
    JobFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpsLevel {
    Error,
    Warn,
    Info,
}

/// Une ligne, dans la forme que `jsonb_to_recordset` relit.
#[derive(Debug, Clone, Serialize)]
pub struct OpsEntry {
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    pub source: &'static str,
    pub kind: OpsKind,
    pub level: OpsLevel,
    pub request_id: Option<String>,
    pub actor_id: Option<Uuid>,
    pub method: Option<String>,
    pub route: Option<String>,
    pub status: Option<i16>,
    pub duration_ms: Option<i32>,
    pub error_code: Option<String>,
    pub task: Option<String>,
    pub job_id: Option<Uuid>,
    pub detail: Option<String>,
}

pub struct HttpOutcome<'a> {
    pub occurred_at: OffsetDateTime,
    pub request_id: &'a str,
    pub actor_id: Option<Uuid>,
    pub method: &'a str,
    pub route: &'a str,
    pub status: u16,
    pub duration: Duration,
    pub error_code: Option<&'a str>,
    pub detail: Option<&'a str>,
}

impl OpsEntry {
    /// Ligne d'une requête HTTP. `detail` n'est gardé que pour une panne
    /// (`ck_ops_log_detail`).
    pub fn http(kind: OpsKind, level: OpsLevel, o: HttpOutcome<'_>) -> Self {
        Self {
            occurred_at: o.occurred_at,
            source: "api",
            kind,
            level,
            request_id: Some(borne(o.request_id, 128)),
            actor_id: o.actor_id,
            method: Some(borne(o.method, 10)),
            route: Some(route_journalisable(o.route)),
            status: i16::try_from(o.status).ok(),
            duration_ms: Some(i32::try_from(o.duration.as_millis()).unwrap_or(i32::MAX)),
            error_code: o.error_code.map(|c| borne(c, 100)),
            task: None,
            job_id: None,
            detail: match kind {
                OpsKind::HttpError => o.detail.map(sanitize_detail),
                _ => None,
            },
        }
    }

    /// Essai d'un travail en échec. `dead` : plus aucun essai à venir.
    pub fn job_failed(task: &str, job_id: Uuid, request_id: &str, dead: bool, error: &str) -> Self {
        Self {
            occurred_at: OffsetDateTime::now_utc(),
            source: "worker",
            kind: OpsKind::JobFailed,
            level: if dead {
                OpsLevel::Error
            } else {
                OpsLevel::Warn
            },
            request_id: Some(borne(request_id, 128)),
            actor_id: None,
            method: None,
            route: None,
            status: None,
            duration_ms: None,
            error_code: None,
            task: Some(borne(task, 200)),
            job_id: Some(job_id),
            detail: Some(sanitize_detail(error)),
        }
    }
}

/// `ck_ops_log_route` : un modèle commençant par `/`, sans `?`, ou `*`.
fn route_journalisable(route: &str) -> String {
    if route.starts_with('/') && !route.contains('?') {
        borne(route, 300)
    } else {
        "*".to_owned()
    }
}

fn borne(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// Détail d'une panne ou d'un travail : courriels et jetons masqués, 2 Ko au
/// plus. Un message d'erreur de la base cite parfois la valeur fautive
/// (`Key (email)=(…)`), et un relais de courriel l'adresse refusée.
pub fn sanitize_detail(brut: &str) -> String {
    let mut sortie = String::with_capacity(brut.len().min(DETAIL_MAX));
    let mut mot = String::new();
    for c in brut.chars() {
        if c.is_alphanumeric() || "._%+-@=~".contains(c) {
            mot.push(c);
        } else {
            vider_mot(&mut mot, &mut sortie);
            sortie.push(c);
        }
    }
    vider_mot(&mut mot, &mut sortie);
    tronquer(sortie, DETAIL_MAX)
}

fn vider_mot(mot: &mut String, sortie: &mut String) {
    if mot.is_empty() {
        return;
    }
    if est_courriel(mot) {
        sortie.push_str("[courriel]");
    } else if est_jeton(mot) {
        sortie.push_str("[jeton]");
    } else {
        sortie.push_str(mot);
    }
    mot.clear();
}

fn est_courriel(mot: &str) -> bool {
    mot.split_once('@')
        .is_some_and(|(avant, apres)| !avant.is_empty() && apres.contains('.'))
}

/// Long, mêlant lettres et chiffres, et pas un uuid : un uuid désigne une
/// ligne, et c'est précisément ce qu'on vient chercher dans le journal.
fn est_jeton(mot: &str) -> bool {
    mot.chars().count() >= 24
        && Uuid::parse_str(mot).is_err()
        && mot.chars().any(|c| c.is_ascii_digit())
        && mot.chars().any(|c| c.is_alphabetic())
}

fn tronquer(mut s: String, max_octets: usize) -> String {
    if s.len() <= max_octets {
        return s;
    }
    let mut fin = max_octets - '…'.len_utf8();
    while !s.is_char_boundary(fin) {
        fin -= 1;
    }
    s.truncate(fin);
    s.push('…');
    s
}

/// Écrit un lot en une requête. Une ligne refusée par une contrainte fait
/// échouer le lot entier : les constructeurs d'[`OpsEntry`] tiennent la forme.
pub async fn insert(pool: &PgPool, lignes: &[OpsEntry]) -> sqlx::Result<u64> {
    if lignes.is_empty() {
        return Ok(0);
    }
    let lot = serde_json::to_value(lignes).map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    let resultat = sqlx::query!(
        r#"INSERT INTO platform.ops_log (
               occurred_at, source, kind, level, request_id, actor_id, method, route,
               status, duration_ms, error_code, task, job_id, detail)
           SELECT r.occurred_at, r.source, r.kind, r.level, r.request_id, r.actor_id, r.method,
                  r.route, r.status, r.duration_ms, r.error_code, r.task, r.job_id, r.detail
             FROM jsonb_to_recordset($1::jsonb) AS r(
                  occurred_at timestamptz, source text, kind text, level text, request_id text,
                  actor_id uuid, method text, route text, status smallint, duration_ms integer,
                  error_code text, task text, job_id uuid, detail text)"#,
        lot
    )
    .execute(pool)
    .await?;
    Ok(resultat.rows_affected())
}

/// Le canal de l'API vers `platform.ops_log`.
#[derive(Clone)]
pub struct OpsLog {
    envoi: mpsc::Sender<OpsEntry>,
    perdues: Arc<AtomicU64>,
}

impl OpsLog {
    /// Lance la tâche d'écriture ; à appeler dans le runtime du serveur.
    pub fn spawn(pool: PgPool) -> Self {
        let (envoi, reception) = mpsc::channel(CAPACITE);
        let perdues = Arc::new(AtomicU64::new(0));
        tokio::spawn(ecrire(pool, reception, perdues.clone()));
        Self { envoi, perdues }
    }

    /// Jamais bloquant : canal plein, la ligne est perdue et comptée.
    pub fn record(&self, ligne: OpsEntry) {
        if self.envoi.try_send(ligne).is_err() {
            self.perdues.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Lignes perdues par ce processus depuis son démarrage.
    pub fn dropped(&self) -> u64 {
        self.perdues.load(Ordering::Relaxed)
    }
}

async fn ecrire(pool: PgPool, mut reception: mpsc::Receiver<OpsEntry>, perdues: Arc<AtomicU64>) {
    let mut tampon = Vec::with_capacity(LOT_MAX);
    let mut rythme = tokio::time::interval(RYTHME);
    rythme.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        rythme.tick().await;
        let mut ferme = false;
        loop {
            match reception.try_recv() {
                Ok(ligne) => tampon.push(ligne),
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    ferme = true;
                    break;
                }
            }
        }
        for lot in tampon.chunks(LOT_MAX) {
            if let Err(e) = insert(&pool, lot).await {
                perdues.fetch_add(lot.len() as u64, Ordering::Relaxed);
                tracing::warn!(erreur = %e, lignes = lot.len(), "journal d'exploitation : lot perdu");
            }
        }
        tampon.clear();
        if ferme {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courriels_et_jetons_masques_uuid_garde() {
        let brut = "Key (email)=(claire.martin@exemple.org) ; jeton eyJhbGciOiJIUzI1NiJ9abc123def456 ; ligne 01a19c2e-7f31-7b20-9d4a-55e0c1b8a6f2";
        let net = sanitize_detail(brut);
        assert!(!net.contains("claire"), "{net}");
        assert!(!net.contains("eyJ"), "{net}");
        assert!(
            net.contains("[courriel]") && net.contains("[jeton]"),
            "{net}"
        );
        assert!(
            net.contains("01a19c2e-7f31-7b20-9d4a-55e0c1b8a6f2"),
            "{net}"
        );
    }

    #[test]
    fn detail_borne_a_deux_ko_sur_une_frontiere_de_caractere() {
        let net = sanitize_detail(&"é ".repeat(3000));
        assert!(net.len() <= DETAIL_MAX);
        assert!(net.ends_with('…'));
    }

    #[test]
    fn route_sans_modele_devient_etoile() {
        assert_eq!(route_journalisable("(aucune route)"), "*");
        assert_eq!(route_journalisable("/api/x?y=1"), "*");
        assert_eq!(route_journalisable("/api/admin/{id}"), "/api/admin/{id}");
    }

    #[test]
    fn detail_garde_pour_une_panne_seulement() {
        let o = |status| HttpOutcome {
            occurred_at: OffsetDateTime::now_utc(),
            request_id: "r",
            actor_id: None,
            method: "GET",
            route: "/api/x",
            status,
            duration: Duration::from_millis(5),
            error_code: Some("INTERNAL"),
            detail: Some("panne"),
        };
        let panne = OpsEntry::http(OpsKind::HttpError, OpsLevel::Error, o(500));
        let refus = OpsEntry::http(OpsKind::HttpRefused, OpsLevel::Info, o(403));
        assert_eq!(panne.detail.as_deref(), Some("panne"));
        assert_eq!(refus.detail, None);
        let json = serde_json::to_value(&panne).unwrap();
        assert_eq!(json["kind"], "http_error");
        assert_eq!(json["level"], "error");
    }
}
