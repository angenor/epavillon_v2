//! L'import des sessions officielles d'une édition (research R7).
//!
//! Le travail **réussit toujours** côté file : une lecture manquée est une
//! donnée (`missed_reads`, `import_runs`), pas un échec — sinon l'attente
//! croissante de `fail_job` espacerait les lectures pendant la panne même.
//! Il se replanifie dans la transaction de sa lecture, au créneau suivant ;
//! éteint, il s'arrête ; une lecture manuelle ne replanifie jamais.

use async_trait::async_trait;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde::Deserialize;
use serde_json::json;
use sqlx::postgres::PgConnection;
use std::collections::{HashMap, HashSet};
use time::{OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

use crate::import::archive::LecteurArchive;
use crate::import::comparaison::{comparer, Ecart, Etat, Fiche, Motif};
use crate::import::denominations::{Vocabulaires, COORDINATION};
use crate::import::reel::LecteurReel;
use crate::import::source::{AnnulationSource, EchecLecture, SessionLue, SourceOfficielle};
use crate::jobs::traduction;
use crate::repo::import::{self as depot, Ecriture, Lecture, Origine, Reglage};

pub const IMPORT_OFFICIAL_SESSIONS: &str = "negotiation.import_official_sessions";

#[derive(Deserialize)]
struct Charge {
    event_id: Uuid,
    #[serde(default)]
    manuel: bool,
}

pub struct ImportOfficialSessions {
    db: Db,
    /// Faux sans clé d'OpenRouter : aucun travail de traduction n'est posé.
    traduire: bool,
}

impl ImportOfficialSessions {
    pub fn new(db: Db, traduire: bool) -> Self {
        Self { db, traduire }
    }
}

#[async_trait]
impl JobHandler for ImportOfficialSessions {
    fn task(&self) -> &'static str {
        IMPORT_OFFICIAL_SESSIONS
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let charge: Charge = serde_json::from_value(job.payload.clone()).map_err(|e| {
            ApiError::internal(format!("charge du travail d'import illisible : {e}"))
        })?;

        let reglage = {
            let mut conn = self.db.pool().acquire().await?;
            depot::reglage(&mut conn, charge.event_id).await?
        };
        // « Lire maintenant » lit même éteint : l'affichage, lui, reste coupé.
        let Some(reglage) = reglage.filter(|r| r.is_enabled || charge.manuel) else {
            tracing::info!(event_id = %charge.event_id, "import éteint : ni lecture ni suivante");
            return Ok(());
        };

        let lecture = Lecture {
            import_id: reglage.id,
            debut: OffsetDateTime::now_utc(),
            manuelle: charge.manuel,
        };
        // La lecture se fait hors transaction : quinze secondes d'attente ne
        // doivent pas tenir une connexion ouverte.
        let lues = match lecteur(&reglage) {
            Ok(source) => source.lire().await,
            Err(e) => Err(e),
        };

        let mut tx = self.db.write(&job.context()).await?;
        let issue = match lues {
            Ok(lues) => ecrire(&mut tx, &reglage, &lecture, lues, self.traduire).await?,
            Err(e) => Err(e),
        };
        match &issue {
            Ok(ecarts) => {
                tracing::info!(event_id = %reglage.event_id, ecarts, "source officielle lue");
            }
            Err(echec) => {
                let message = echec.to_string();
                depot::journaliser(&mut tx, &lecture, Some(&message), None, None).await?;
                depot::noter_echec(&mut tx, reglage.id, &message, OffsetDateTime::now_utc())
                    .await?;
                tracing::warn!(event_id = %reglage.event_id, erreur = %message, "lecture manquée");
            }
        }
        depot::purger_journal(&mut tx, reglage.id).await?;
        if !charge.manuel {
            planifier(
                &mut tx,
                reglage.event_id,
                reglage.interval_seconds,
                OffsetDateTime::now_utc(),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

fn lecteur(r: &Reglage) -> std::result::Result<Box<dyn SourceOfficielle>, EchecLecture> {
    let correction_minutes = i64::from(r.time_correction_minutes);
    match r.reader.as_str() {
        "live" => {
            let url = r.live_url.clone().ok_or_else(|| {
                EchecLecture::Injoignable("adresse de la source non renseignée".into())
            })?;
            Ok(Box::new(LecteurReel {
                url,
                correction_minutes,
            }))
        }
        _ => {
            let nom = r
                .archive_name
                .clone()
                .ok_or_else(|| EchecLecture::Injoignable("aucun jeu archivé choisi".into()))?;
            Ok(Box::new(LecteurArchive {
                nom,
                premier_jour: r.archive_first_day,
                correction_minutes,
            }))
        }
    }
}

/// Une session que l'import retient, résolue par les vocabulaires.
struct Retenue {
    lue: SessionLue,
    type_id: Uuid,
    groupe: Option<Uuid>,
}

/// Rend le nombre de sessions touchées, ou la lecture manquée.
async fn ecrire(
    tx: &mut PgConnection,
    r: &Reglage,
    lecture: &Lecture,
    lues: Vec<SessionLue>,
    traduire: bool,
) -> Result<std::result::Result<i32, EchecLecture>> {
    let vocabulaires = depot::vocabulaires(tx).await?;
    let retenues = retenir(&vocabulaires, lues);
    let fiches = fiches(tx, &vocabulaires, &retenues, &r.timezone).await?;
    let (base, groupes_en_base) = depot::etat(tx, r.event_id).await?;

    let cles: Vec<(String, Fiche)> = retenues
        .iter()
        .zip(&fiches)
        .map(|(s, f)| (s.lue.cle.clone(), f.clone()))
        .collect();
    let comparaison = match comparer(&cles, &base) {
        Ok(c) => c,
        Err(e) => return Ok(Err(e)),
    };

    let lu_a = OffsetDateTime::now_utc();
    let touchees = i32::try_from(comparaison.ecarts.len()).unwrap_or(i32::MAX);
    let run_id = depot::journaliser(
        tx,
        lecture,
        None,
        Some(i32::try_from(comparaison.retenues.len()).unwrap_or(i32::MAX)),
        Some(touchees),
    )
    .await?;

    let points = depot::points(tx, r.event_id, &points_cites(&retenues), lu_a).await?;
    let ecriture = |i: usize| Ecriture {
        fiche: &fiches[i],
        type_id: retenues[i].type_id,
        groupe: retenues[i].groupe,
        point: fiches[i]
            .point
            .as_ref()
            .and_then(|c| points.get(c).copied()),
    };

    let space_id = if comparaison
        .ecarts
        .iter()
        .any(|e| matches!(e, Ecart::Apparue { .. }))
    {
        Some(depot::espace_climat(tx).await?)
    } else {
        None
    };

    let mut ecrites = HashSet::new();
    for ecart in &comparaison.ecarts {
        match ecart {
            Ecart::Apparue { lue } => {
                let s = &retenues[*lue].lue;
                let origine = Origine {
                    event_id: r.event_id,
                    space_id: space_id.expect("posé dès qu'une session apparaît"),
                    slug: &r.edition_slug,
                    fuseau: &r.timezone,
                    cle: &s.cle,
                    url: &s.url,
                };
                depot::inserer(tx, &origine, &ecriture(*lue), lu_a).await?;
            }
            Ecart::Changee {
                id,
                lue,
                changements,
                ..
            } => {
                depot::modifier(tx, *id, &ecriture(*lue), lu_a).await?;
                depot::noter_changements(tx, *id, changements, lu_a, run_id).await?;
                ecrites.insert(*id);
            }
            Ecart::Absente {
                id,
                absences,
                changements,
            } => {
                depot::noter_absence(tx, *id, *absences, !changements.is_empty(), lu_a).await?;
                depot::noter_changements(tx, *id, changements, lu_a, run_id).await?;
            }
        }
    }

    let groupes: HashMap<&str, Option<Uuid>> = comparaison
        .retenues
        .iter()
        .map(|&i| (retenues[i].lue.cle.as_str(), retenues[i].groupe))
        .collect();
    // Une session sans écart n'est pas réécrite, pas même sa dernière lecture :
    // l'audit de `meetings` en ferait une ligne par session et par lecture.
    let presentes: HashSet<Uuid> = comparaison.presentes.iter().copied().collect();
    let rattachements: Vec<(Uuid, Option<Uuid>)> = base
        .iter()
        .filter(|b| presentes.contains(&b.id) && !ecrites.contains(&b.id))
        .filter_map(|b| {
            let groupe = groupes.get(b.cle.as_str()).copied().flatten();
            (groupes_en_base.get(&b.id).copied().flatten() != groupe).then_some((b.id, groupe))
        })
        .collect();
    depot::rattacher_groupes(tx, &rattachements, lu_a).await?;
    depot::noter_reussite(tx, r.id, touchees, lu_a).await?;

    if traduire
        && !depot::titres_sans_traduction(tx, r.event_id)
            .await?
            .is_empty()
    {
        traduction::poser(tx, r.event_id, creneau(lu_a, r.interval_seconds)).await?;
    }

    Ok(Ok(touchees))
}

/// Écarte ce qu'aucun type n'admet ; rattache une coordination à son groupe,
/// ou à aucun.
fn retenir(v: &Vocabulaires, lues: Vec<SessionLue>) -> Vec<Retenue> {
    lues.into_iter()
        .filter_map(|lue| {
            let t = v.type_de(&lue.categories, &lue.titre)?;
            let groupe = if t.code == COORDINATION {
                v.groupe_de(&lue.titre).map(|g| g.id)
            } else {
                None
            };
            Some(Retenue {
                type_id: t.id,
                groupe,
                lue,
            })
        })
        .collect()
}

async fn fiches(
    conn: &mut PgConnection,
    v: &Vocabulaires,
    retenues: &[Retenue],
    fuseau: &str,
) -> Result<Vec<Fiche>> {
    let debuts: Vec<PrimitiveDateTime> = retenues.iter().map(|s| s.lue.debut).collect();
    let fins: Vec<PrimitiveDateTime> = retenues.iter().filter_map(|s| s.lue.fin).collect();
    let debuts = depot::situer(conn, &debuts, fuseau).await?;
    let mut fins = depot::situer(conn, &fins, fuseau).await?.into_iter();

    Ok(retenues
        .iter()
        .zip(debuts)
        .map(|(s, debut)| Fiche {
            debut,
            fin: s.lue.fin.and_then(|_| fins.next()),
            salle: s.lue.salle.clone(),
            titre: s.lue.titre.clone(),
            type_code: v.code_du_type(s.type_id).unwrap_or_default().to_owned(),
            acces_ouvert: s.lue.acces_ouvert,
            point: s.lue.point.as_ref().map(|p| p.code.clone()),
            etat: match s.lue.annulation {
                None => Etat::Prevue,
                Some(AnnulationSource::Annulee) => Etat::Annulee(Motif::Source),
                Some(AnnulationSource::Reportee) => Etat::Annulee(Motif::Reportee),
            },
        })
        .collect())
}

/// Chaque code une fois, avec l'intitulé de la première session qui le cite.
fn points_cites(retenues: &[Retenue]) -> Vec<(String, String)> {
    let mut vus = Vec::<(String, String)>::new();
    for p in retenues.iter().filter_map(|s| s.lue.point.as_ref()) {
        if !vus.iter().any(|(c, _)| *c == p.code) {
            vus.push((p.code.clone(), p.intitule.clone()));
        }
    }
    vus
}

/// Pose la lecture du créneau qui suit `maintenant`. Faux : elle était posée.
pub async fn planifier(
    conn: &mut PgConnection,
    event_id: Uuid,
    interval_seconds: i32,
    maintenant: OffsetDateTime,
) -> Result<bool> {
    poser(
        conn,
        event_id,
        interval_seconds,
        creneau(maintenant, interval_seconds) + 1,
    )
    .await
}

/// Pose une lecture tout de suite, sauf si la chaîne en a déjà une en file :
/// allumer l'import, ou réarmer la chaîne au démarrage du worker. Le créneau
/// en cours déjà lu (éteint puis rallumé), elle prend le suivant.
pub async fn poser_maintenant(
    conn: &mut PgConnection,
    event_id: Uuid,
    interval_seconds: i32,
    maintenant: OffsetDateTime,
) -> Result<bool> {
    if chaine_vivante(conn, event_id).await? {
        return Ok(false);
    }
    let courant = creneau(maintenant, interval_seconds);
    let pose = jobs::enqueue(
        conn,
        NewJob::new(IMPORT_OFFICIAL_SESSIONS, json!({ "event_id": event_id }))
            .idempotent(cle(event_id, courant))
            .at(maintenant),
    )
    .await?;
    if pose.is_some() {
        return Ok(true);
    }
    poser(conn, event_id, interval_seconds, courant + 1).await
}

/// « Lire maintenant » : une lecture à sa propre clé, qui ne replanifie pas —
/// deux appels font deux lectures, et la chaîne reste unique (R7).
pub async fn poser_manuel(conn: &mut PgConnection, event_id: Uuid, request_id: &str) -> Result<()> {
    jobs::enqueue(
        conn,
        NewJob::new(
            IMPORT_OFFICIAL_SESSIONS,
            json!({ "event_id": event_id, "manuel": true }),
        )
        .idempotent(format!("import:{event_id}:manuel:{request_id}")),
    )
    .await?;
    Ok(())
}

/// Au démarrage du worker : chaque import allumé retrouve sa chaîne.
pub async fn armer(conn: &mut PgConnection, maintenant: OffsetDateTime) -> Result<usize> {
    let mut armes = 0;
    for (event_id, interval) in depot::allumes(conn).await? {
        if poser_maintenant(conn, event_id, interval, maintenant).await? {
            armes += 1;
        }
    }
    Ok(armes)
}

async fn poser(
    conn: &mut PgConnection,
    event_id: Uuid,
    interval: i32,
    creneau: i64,
) -> Result<bool> {
    let moment = OffsetDateTime::from_unix_timestamp(creneau * i64::from(interval.max(1)))
        .map_err(|e| ApiError::internal(format!("créneau d'import hors bornes : {e}")))?;
    let pose = jobs::enqueue(
        conn,
        NewJob::new(IMPORT_OFFICIAL_SESSIONS, json!({ "event_id": event_id }))
            .idempotent(cle(event_id, creneau))
            .at(moment),
    )
    .await?;
    Ok(pose.is_some())
}

/// Une lecture de la chaîne en file ou en cours ; les lectures manuelles
/// n'en font pas partie.
async fn chaine_vivante(conn: &mut PgConnection, event_id: Uuid) -> Result<bool> {
    let prefixe = format!("import:{event_id}:");
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM platform.jobs
                           WHERE task = $1 AND status IN ('queued', 'running')
                             AND starts_with(idempotency_key, $2)
                             AND NOT starts_with(idempotency_key, $2 || 'manuel:')) AS "existe!""#,
        IMPORT_OFFICIAL_SESSIONS,
        prefixe
    )
    .fetch_one(conn)
    .await?;
    Ok(existe)
}

pub fn creneau(moment: OffsetDateTime, interval_seconds: i32) -> i64 {
    moment
        .unix_timestamp()
        .div_euclid(i64::from(interval_seconds.max(1)))
}

pub fn cle(event_id: Uuid, creneau: i64) -> String {
    format!("import:{event_id}:{creneau}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn le_creneau_est_le_quotient_de_l_epoque_par_l_intervalle() {
        let moment = datetime!(2026-11-09 10:07:30 UTC);
        assert_eq!(creneau(moment, 300), moment.unix_timestamp() / 300);
        assert_eq!(
            creneau(datetime!(2026-11-09 10:05:00 UTC), 300),
            creneau(datetime!(2026-11-09 10:09:59 UTC), 300)
        );
    }
}
