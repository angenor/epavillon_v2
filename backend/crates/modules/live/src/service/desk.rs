//! Le poste de direct — **ce qui se joue en ce moment**.
//!
//! # POURQUOI CE BLOC EST EN TÊTE D'ÉCRAN
//!
//! Un message d'incident se rédige presque toujours pendant qu'une activité se
//! tient : la salle attend, l'intervenante ne s'est pas connectée, la diffusion
//! vient de tomber. Demander alors de choisir une portée parmi cinq, une nature
//! parmi neuf et une cible dans une liste de trente activités, c'est demander
//! trois décisions à quelqu'un qui n'a pas trois secondes. Le poste pose la
//! question à l'envers : **voici ce qui se joue, que se passe-t-il ?**
//!
//! # LE JOUR EST CELUI DE L'ÉDITION
//!
//! `(now() AT TIME ZONE events.timezone)::date`, calculé **en base**. À Belém,
//! un serveur en UTC bascule de jour trois heures trop tôt : le poste montrerait
//! alors les activités du lendemain pendant que la salle est encore pleine.
//!
//! # LE REPLI, ET CE QU'IL NE DOIT PAS FAIRE CROIRE
//!
//! Sans aucune activité aujourd'hui, le poste montre les **quatre** prochaines
//! et `is_fallback` vaut vrai — mais `day` **reste aujourd'hui**. « Rien
//! aujourd'hui » et « voici la suite » ne sont pas la même information, et les
//! confondre ferait croire à un direct en cours hors période.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use std::collections::HashMap;
use time::Date;
use uuid::Uuid;

use crate::domain::desk::{LiveDesk, LiveDeskSession};
use crate::repo::cross::programme;
use crate::repo::incidents;

/// Le nombre d'activités montrées en repli. Règle d'écran : aucune notion de
/// repli n'existe en base.
const REPLI: i64 = 4;

pub async fn composer(
    conn: &mut PgConnection,
    event_id: Uuid,
    aujourdhui: Date,
) -> Result<LiveDesk> {
    let du_jour = programme::du_jour(&mut *conn, event_id, aujourdhui).await?;
    let is_fallback = du_jour.is_empty();

    let activites = if is_fallback {
        programme::les_prochaines(&mut *conn, event_id, REPLI).await?
    } else {
        du_jour
    };

    // Les compteurs sont lus **une fois**, pas une requête par activité : le
    // poste en montre jusqu'à trente un jour de COP.
    let compteurs: HashMap<Uuid, i64> = incidents::actifs_par_activite(&mut *conn, event_id)
        .await?
        .into_iter()
        .collect();

    let sessions = activites
        .into_iter()
        .map(|a| LiveDeskSession {
            active_incident_count: compteurs.get(&a.session_id).copied().unwrap_or(0),
            session_id: a.session_id,
            title: a.title,
            starts_at: a.starts_at,
            ends_at: a.ends_at,
            room_name: a.room_name,
            is_streamed: a.is_streamed,
            status: a.status,
            temporal_state: a.temporal_state,
        })
        .collect();

    Ok(LiveDesk {
        day: aujourdhui,
        sessions,
        is_fallback,
    })
}
