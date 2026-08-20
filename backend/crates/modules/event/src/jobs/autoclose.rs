//! Clôture automatique des appels échus (research.md § R15).
//!
//! **Pourquoi ce travail existe.** `event.is_call_open()` protège la
//! *recevabilité* — elle vérifie le statut *et* la fenêtre. Mais le **statut
//! affiché** resterait « ouvert » après l'échéance, sur la page publique comme
//! dans la liste du back-office, et c'est ce statut que lit une organisation
//! qui se demande si elle peut encore déposer.
//!
//! Le travail se **replanifie lui-même**, sur le patron de la purge des jetons
//! de B1 : rien dans le noyau ne porte de récurrence, et une boucle de plus
//! dans le worker serait un second ordonnanceur à surveiller. La clé d'unicité
//! porte **le créneau visé**, calculé sur une grille ancrée à l'époque Unix —
//! dix redémarrages dans le même créneau n'en produisent pas dix.
//!
//! Ce que cela ne rattrape pas, et c'est écrit : une occurrence qui meurt après
//! ses essais n'a pas replanifié la suivante, et sa clé occupe déjà l'index. La
//! chaîne se réarme au **démarrage du worker**, qui repose le créneau courant.
//!
//! **Ce qui n'est PAS livré ici** : le rappel d'échéance aux organisations. Les
//! règles de rappel et les modèles de message vivent dans le module Engagement
//! (B6) ; les recopier ici produirait un second dispositif, désynchronisé du
//! premier au premier changement.

use async_trait::async_trait;
use contracts::event as contrat;
use kernel::db::Db;
use kernel::error::Result;
use kernel::events::{self, DomainEvent};
use kernel::jobs::{self, ClaimedJob, JobHandler, NewJob};
use serde_json::json;
use sqlx::postgres::PgConnection;
use std::time::Duration;
use time::OffsetDateTime;

pub const CALL_AUTOCLOSE: &str = "event.call.autoclose";

pub struct CloseExpiredCalls {
    db: Db,
    intervalle: Duration,
}

impl CloseExpiredCalls {
    pub fn new(db: Db, intervalle: Duration) -> Self {
        Self { db, intervalle }
    }
}

#[async_trait]
impl JobHandler for CloseExpiredCalls {
    fn task(&self) -> &'static str {
        CALL_AUTOCLOSE
    }

    async fn run(&self, job: &ClaimedJob) -> Result<()> {
        let mut tx = self.db.write(&job.context()).await?;

        let clos = clore_les_echus(&mut tx).await?;

        // La suivante naît dans la MÊME transaction que la clôture : une
        // clôture validée sans sa suivante romprait la chaîne en silence.
        planifier(
            &mut tx,
            prochaine_occurrence(OffsetDateTime::now_utc(), self.intervalle),
        )
        .await?;

        tx.commit().await?;
        if clos > 0 {
            tracing::info!(clos, "appels à propositions échus clos");
        }
        Ok(())
    }
}

/// Passe en `closed` tout appel `open` dont l'échéance effective est passée, et
/// **annonce chacun**. Rend le nombre d'appels clos.
///
/// L'échéance vient de `event.effective_deadline()`, la fonction du modèle :
/// elle est appelée, jamais recalculée. Recopier `COALESCE(extended_until,
/// closes_at)` ici ferait une seconde définition de l'échéance, et la première
/// évolution du SQL les ferait diverger.
///
/// **L'annonce est écrite par le code, pas par un déclencheur.** Aucun
/// déclencheur de `060_events.sql` n'émet d'événement de domaine : ce qui n'est
/// pas annoncé ici n'est annoncé par personne.
async fn clore_les_echus(conn: &mut PgConnection) -> Result<usize> {
    let echus = sqlx::query!(
        r#"UPDATE event.calls_for_proposals c
              SET status = 'closed'
            WHERE c.status = 'open'
              AND event.effective_deadline(c.id) <= now()
        RETURNING c.id, c.event_id, event.effective_deadline(c.id) AS "echeance!""#
    )
    .fetch_all(&mut *conn)
    .await?;

    for appel in &echus {
        let charge = serde_json::to_value(contrat::CallClosed {
            call_id: appel.id,
            event_id: appel.event_id,
            applied_deadline: appel.echeance,
        })
        .map_err(kernel::error::ApiError::internal)?;

        events::emit(
            &mut *conn,
            DomainEvent {
                aggregate_schema: contrat::AGGREGATE_SCHEMA,
                aggregate_type: contrat::AGGREGATE_CALL,
                aggregate_id: appel.id,
                event_type: contrat::CALL_CLOSED,
                payload: charge,
            },
        )
        .await?;
    }

    Ok(echus.len())
}

/// Pose le créneau visé, s'il n'existe pas déjà. Faux : il était posé.
pub async fn planifier(conn: &mut PgConnection, moment: OffsetDateTime) -> Result<bool> {
    let pose = jobs::enqueue(
        conn,
        NewJob::new(CALL_AUTOCLOSE, json!({}))
            .idempotent(format!("{CALL_AUTOCLOSE}:{}", moment.unix_timestamp()))
            .at(moment),
    )
    .await?;

    Ok(pose.is_some())
}

/// Le prochain créneau de la grille, ancrée à l'époque Unix.
///
/// L'ancrage sur une grille plutôt que sur « dans une heure » a la même raison
/// qu'en B1 : sans lui, le créneau dériverait d'un redémarrage à l'autre, et la
/// clé d'unicité ne coïnciderait jamais deux fois — dix redémarrages
/// produiraient dix travaux.
pub fn prochaine_occurrence(depuis: OffsetDateTime, intervalle: Duration) -> OffsetDateTime {
    let pas = intervalle.as_secs().max(1) as i64;
    let suivant = (depuis.unix_timestamp().div_euclid(pas) + 1) * pas;
    OffsetDateTime::from_unix_timestamp(suivant).unwrap_or(depuis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn le_creneau_suivant_tombe_sur_la_grille() {
        let heure = Duration::from_secs(3600);
        assert_eq!(
            prochaine_occurrence(datetime!(2026-08-20 10:17:33 UTC), heure),
            datetime!(2026-08-20 11:00:00 UTC)
        );
        assert_eq!(
            prochaine_occurrence(datetime!(2026-08-20 23:59:59 UTC), heure),
            datetime!(2026-08-21 00:00:00 UTC)
        );
    }

    /// Deux démarrages dans le même créneau visent le même instant : c'est ce
    /// qui fait que la clé d'unicité les confond, et qu'un seul travail naît.
    #[test]
    fn deux_demarrages_du_meme_creneau_visent_le_meme_instant() {
        let heure = Duration::from_secs(3600);
        assert_eq!(
            prochaine_occurrence(datetime!(2026-08-20 10:00:01 UTC), heure),
            prochaine_occurrence(datetime!(2026-08-20 10:59:59 UTC), heure)
        );
    }

    /// Une cadence nulle est refusée au démarrage ; si elle passait quand même,
    /// la grille ne doit pas diviser par zéro.
    #[test]
    fn une_cadence_nulle_ne_fait_pas_tomber_le_calcul() {
        let depuis = datetime!(2026-08-20 10:00:00 UTC);
        assert!(prochaine_occurrence(depuis, Duration::ZERO) > depuis);
    }
}
