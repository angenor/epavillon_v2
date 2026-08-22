//! **Les inscriptions et les séances font vivre les rappels.**
//!
//! # `programme.registration.confirmed` N'EXISTE PAS
//!
//! Le commentaire de `engagement.schedule_session_reminders()` annonce que la
//! fonction est appelée « par l'abonné outbox sur
//! `programme.registration.confirmed` ». **Cet événement n'est émis par
//! personne.** `programme.registration_status` vaut `registered`, `waitlisted`,
//! `cancelled`, `attended`, `no_show` — jamais `confirmed` —, et le déclencheur
//! émet `programme.registration.created` à la **création**, avec le statut en
//! charge utile : une inscription ordinaire naît **à l'état inscrit, par une
//! création**.
//!
//! Un consommateur écrit d'après ce commentaire **ne serait jamais réveillé** :
//! aucun rappel ne partirait, sans erreur, sans trace, et personne ne s'en
//! apercevrait avant le jour de la séance (écart n° 126).
//!
//! **On branche donc sur `payload->>'status'`, jamais sur le type
//! d'événement.** Un état ajouté demain à l'énuméré arrive alors ici comme un
//! état, pas comme un nom d'événement inconnu qu'on ignorerait en silence.
//!
//! # Les huit annonces de séance, et ce que chacune fait
//!
//! | Annonce | Effet | Pourquoi |
//! |---|---|---|
//! | `created`, `planned`, `scheduled` | matérialiser | le créneau est connu ; la fonction du modèle ne crée que ce qui manque |
//! | `rescheduled` | **déplacer** | le créneau bouge sans que l'état bouge ; recréer se heurterait à la clé d'unicité |
//! | `postponed`, `cancelled` | annuler | plus de créneau tenable ; les rappels déjà partis restent partis |
//! | `live`, `completed` | rien | la séance a commencé : un rappel n'a plus d'objet |
//!
//! # Rien n'est émis, rien n'est enfilé
//!
//! `engagement.schedule_session_reminders()` insère les rappels, **met un
//! travail par rappel en file** et **émet** son annonce. Ce fichier ne redouble
//! ni l'un ni l'autre : il produirait deux courriels par rappel, et le doublon
//! ne se verrait qu'en production.

use async_trait::async_trait;
use kernel::error::Result;
use kernel::events::{EventConsumer, OutboxEvent};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::reminder::motifs;
use crate::service::schedule;

pub struct RemindersConsumer;

/// Les états d'inscription qui **donnent droit** à un rappel. Écrits en toutes
/// lettres et comparés au texte, comme la fonction du modèle : le module
/// `programme` peut faire évoluer son énuméré sans casser ceci.
const ETATS_A_RAPPELER: [&str; 2] = ["registered", "attended"];

#[async_trait]
impl EventConsumer for RemindersConsumer {
    /// **Inscrit dans `platform.inbox_events`.** Le renommer ferait rejouer
    /// toutes les inscriptions jamais enregistrées.
    fn name(&self) -> &'static str {
        "engagement.reminders"
    }

    fn handles(&self, event_type: &str) -> bool {
        event_type.starts_with("programme.registration.")
            || event_type.starts_with("programme.session.")
    }

    async fn handle(&self, conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
        if event.event_type.starts_with("programme.registration.") {
            return inscription(conn, event).await;
        }
        seance(conn, event).await
    }
}

/// **Le statut porté par la charge utile décide, pas le nom de l'événement.**
async fn inscription(conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
    let Some(session_id) = uuid_de(event, "session_id") else {
        return Ok(());
    };
    let Some(person_id) = uuid_de(event, "person_id") else {
        return Ok(());
    };
    let statut = texte_de(event, "status").unwrap_or_default();

    if ETATS_A_RAPPELER.contains(&statut.as_str()) {
        let crees = schedule::materialiser(conn, session_id, Some(person_id)).await?;
        tracing::debug!(%session_id, crees, statut = %statut, "rappels matérialisés");
    } else {
        // `waitlisted`, `cancelled`, `no_show` : rien ne doit partir, et le
        // motif est ce qui distingue une annulation d'un oubli. Une personne
        // promue depuis la liste d'attente repasse par la branche du dessus, et
        // ses lignes sont **réactivées** plutôt que recréées.
        let annules = schedule::annuler_une_inscription(conn, session_id, person_id).await?;
        if annules > 0 {
            tracing::debug!(%session_id, annules, statut = %statut, "rappels annulés");
        }
    }
    Ok(())
}

async fn seance(conn: &mut PgConnection, event: &OutboxEvent) -> Result<()> {
    let session_id = event.aggregate_id;

    match event.event_type.as_str() {
        contracts::programme::SESSION_CREATED
        | contracts::programme::SESSION_PLANNED
        | contracts::programme::SESSION_SCHEDULED => {
            let crees = schedule::materialiser(conn, session_id, None).await?;
            tracing::debug!(%session_id, crees, "rappels matérialisés à la programmation");
        }
        contracts::programme::SESSION_RESCHEDULED => {
            // **Le seul événement qui ne soit pas un état** : le créneau a
            // bougé sans que le statut bouge, et la charge utile porte alors
            // l'ancien début. Sans lui, on ne saurait pas de combien déplacer —
            // et recalculer depuis la règle recréerait des lignes que la clé
            // d'unicité refuserait, en silence.
            let (Some(avant), Some(apres)) = (
                instant_de(event, "previous_starts_at"),
                instant_de(event, "starts_at"),
            ) else {
                tracing::warn!(%session_id, "report sans les deux instants : rappels inchangés");
                return Ok(());
            };
            let secondes = (apres - avant).as_seconds_f64();
            let deplaces = schedule::deplacer(conn, session_id, secondes).await?;
            tracing::info!(%session_id, deplaces, secondes, "rappels déplacés");
        }
        contracts::programme::SESSION_POSTPONED | contracts::programme::SESSION_CANCELLED => {
            let annules =
                schedule::annuler_la_seance(conn, session_id, motifs::SEANCE_ANNULEE).await?;
            tracing::info!(%session_id, annules, etat = %event.event_type, "rappels annulés");
        }
        // `live` et `completed` : la séance a commencé, un rappel n'a plus
        // d'objet. Ne rien faire est la bonne réponse, et l'écrire évite qu'on
        // se demande un jour si c'est un oubli.
        _ => {}
    }
    Ok(())
}

fn uuid_de(event: &OutboxEvent, cle: &str) -> Option<Uuid> {
    event.payload.get(cle)?.as_str()?.parse().ok()
}

fn texte_de(event: &OutboxEvent, cle: &str) -> Option<String> {
    Some(event.payload.get(cle)?.as_str()?.to_owned())
}

/// Les instants voyagent dans l'outbox au format que `to_jsonb(timestamptz)`
/// produit : ISO 8601 avec décalage, soit du RFC 3339. Le repli sur l'ISO
/// complet couvre les variantes que le réglage `DateStyle` du serveur pourrait
/// produire — un instant illisible ferait un report muet, et un report muet
/// laisse les rappels à l'ancienne heure.
fn instant_de(event: &OutboxEvent, cle: &str) -> Option<OffsetDateTime> {
    use time::format_description::well_known::{Iso8601, Rfc3339};
    let brut = event.payload.get(cle)?.as_str()?;
    OffsetDateTime::parse(brut, &Rfc3339)
        .or_else(|_| OffsetDateTime::parse(brut, &Iso8601::DEFAULT))
        .ok()
}
