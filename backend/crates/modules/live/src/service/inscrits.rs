//! D'une inscription plateforme à un inscrit chez Zoom, sans jamais appeler
//! Zoom : on écrit la ligne, la file fait le reste.
//!
//! Partagé par les consommateurs d'inscriptions et par la reconstruction du
//! back-office, pour qu'un même geste ait un seul chemin.
//!
//! Une JOURNÉE (02/11) a une seule réunion : l'inscription à la journée y
//! prend une place, ses activités gardées n'en prennent aucune, et une activité
//! de journée n'a jamais de réunion propre.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::repo::cross::programme::{self, Inscription, InscriptionJournee, Journee, Seance};
use crate::repo::meetings::{self, NouveauParticipant, NouvelleReunion, NouvelleReunionJournee};

/// Les statuts qui donnent une place chez Zoom. La liste d'attente en est :
/// le jour même, Zoom laisse entrer les trois cents premiers arrivés.
pub fn a_une_place(statut: &str) -> bool {
    matches!(statut, "registered" | "waitlisted" | "attended")
}

/// Aligne la ligne `live.meeting_participants` sur l'état courant de
/// l'inscription, lu en base plutôt que dans l'événement : des événements
/// traités en retard ne remettent pas un état dépassé.
pub async fn synchroniser(conn: &mut PgConnection, registration_id: Uuid) -> Result<()> {
    let Some(inscription) = programme::inscription(conn, registration_id).await? else {
        return Ok(());
    };

    // Activité gardée, ou activité d'une journée : la place est celle de la
    // journée. Une place propre prise avant le rattachement est rendue.
    let de_journee = inscription.track_registration_id.is_some()
        || programme::journee_de_seance(conn, inscription.seance.id)
            .await?
            .is_some();
    if de_journee || inscription.status == "cancelled" {
        if let Some(p) = meetings::participant_de_inscription(conn, registration_id).await? {
            meetings::annuler_participant(conn, p.id).await?;
        }
        return Ok(());
    }

    if !a_une_place(&inscription.status) {
        return Ok(());
    }
    let reunion = match meetings::reunion_active(conn, inscription.seance.id).await? {
        Some(r) => r.id,
        // Une séance non diffusée n'a de réunion que si le back-office l'a créée.
        // La réunion naît avec la première inscription : toutes celles déjà
        // posées sur la séance la rejoignent, pas seulement celle-ci.
        None if inscription.seance.is_streamed => {
            naitre_avec_ses_inscrits(conn, &inscription.seance).await?;
            return Ok(());
        }
        None => return Ok(()),
    };
    assurer_participant(conn, reunion, &inscription).await
}

/// La réunion naît, et toutes les inscriptions déjà posées sur la séance la
/// rejoignent, pas seulement celle qui l'a fait naître.
async fn naitre_avec_ses_inscrits(conn: &mut PgConnection, seance: &Seance) -> Result<Uuid> {
    let reunion = assurer_reunion(conn, seance, None).await?;
    for id in programme::inscriptions_actives(conn, seance.id).await? {
        if let Some(autre) = programme::inscription(conn, id).await? {
            assurer_participant(conn, reunion, &autre).await?;
        }
    }
    Ok(reunion)
}

/// La régie démarre le direct (02/11) : la réunion doit exister, inscription
/// requise ou non, sans quoi ni l'animateur ni le public n'ont de salle. Celle
/// d'une journée naît de même avec les places de ses inscrits.
pub async fn assurer_reunion_du_direct(conn: &mut PgConnection, seance: &Seance) -> Result<()> {
    if let Some(track_id) = programme::journee_de_seance(conn, seance.id).await? {
        if meetings::reunion_active_journee(conn, track_id)
            .await?
            .is_none()
        {
            if let Some(journee) = programme::journee(conn, track_id).await? {
                reconstruire_journee(conn, &journee, None).await?;
            }
        }
        return Ok(());
    }
    if seance.is_streamed && meetings::reunion_active(conn, seance.id).await?.is_none() {
        naitre_avec_ses_inscrits(conn, seance).await?;
    }
    Ok(())
}

pub async fn assurer_reunion(
    conn: &mut PgConnection,
    seance: &Seance,
    external_id: Option<&str>,
) -> Result<Uuid> {
    let topic = intitule(&seance.title);
    meetings::creer_reunion(
        conn,
        &NouvelleReunion {
            session_id: seance.id,
            event_id: seance.event_id,
            topic: &topic,
            scheduled_start_at: seance.starts_at,
            duration_minutes: duree_minutes(seance),
            timezone: &seance.timezone,
            external_id,
        },
    )
    .await
}

async fn assurer_participant(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    i: &Inscription,
) -> Result<()> {
    let nouveau = NouveauParticipant {
        meeting_id,
        registration_id: Some(i.registration_id),
        track_registration_id: None,
        person_id: i.person_id,
        display_name: &i.display_name,
        email: &i.email,
        first_name: &i.first_name,
        last_name: &i.last_name,
    };

    if let Some(p) = meetings::participant_de_inscription(conn, i.registration_id).await? {
        if p.meeting_id == meeting_id && p.cancelled_at.is_none() {
            return Ok(());
        }
        return meetings::reprendre_participant(conn, p.id, &nouveau).await;
    }
    reprendre_ou_inserer(conn, meeting_id, &nouveau, Some(i.registration_id)).await
}

/// L'adresse est unique par réunion : une personne qui s'était désinscrite
/// retrouve sa ligne, et chez Zoom le même inscrit.
async fn reprendre_ou_inserer(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    nouveau: &NouveauParticipant<'_>,
    registration_id: Option<Uuid>,
) -> Result<()> {
    if let Some(p) = meetings::participant_par_email(conn, meeting_id, nouveau.email).await? {
        let meme = registration_id.is_some() && p.registration_id == registration_id;
        if p.cancelled_at.is_none() && meme {
            return Ok(());
        }
        return meetings::reprendre_participant(conn, p.id, nouveau).await;
    }
    meetings::inserer_participant(conn, nouveau).await?;
    Ok(())
}

/// La durée de la séance en minutes entières, bornée comme `live.meetings`.
pub fn duree_minutes(seance: &Seance) -> i32 {
    minutes_entre(seance.starts_at, seance.ends_at)
}

fn minutes_entre(debut: time::OffsetDateTime, fin: time::OffsetDateTime) -> i32 {
    let minutes = ((fin - debut).whole_seconds() + 59) / 60;
    minutes.clamp(1, 1440) as i32
}

/// L'intitulé de la réunion chez Zoom : le titre français, à défaut le premier
/// présent. Zoom le borne à deux cents caractères.
pub fn intitule(titre: &Value) -> String {
    let texte = titre
        .get("fr")
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .or_else(|| {
            titre.as_object().and_then(|o| {
                o.values()
                    .filter_map(Value::as_str)
                    .find(|t| !t.trim().is_empty())
            })
        })
        .unwrap_or("Activité du Pavillon de la Francophonie");
    texte.trim().chars().take(200).collect()
}

// ---------------------------------------------------------------------------
// La journée
// ---------------------------------------------------------------------------

/// La réunion de la journée, de l'accueil à la clôture.
pub async fn assurer_reunion_journee(
    conn: &mut PgConnection,
    journee: &Journee,
    external_id: Option<&str>,
) -> Result<Uuid> {
    let topic = intitule(&journee.title);
    meetings::creer_reunion_journee(
        conn,
        &NouvelleReunionJournee {
            track_id: journee.id,
            event_id: journee.event_id,
            topic: &topic,
            scheduled_start_at: journee.debut,
            duration_minutes: minutes_entre(journee.debut, journee.fin),
            timezone: &journee.timezone,
            external_id,
        },
    )
    .await
}

/// Comme `synchroniser`, pour une inscription à la journée : la réunion de la
/// journée naît avec sa première inscription (le direct est activé d'office).
pub async fn synchroniser_journee(
    conn: &mut PgConnection,
    track_registration_id: Uuid,
) -> Result<()> {
    let Some(inscription) = programme::inscription_journee(conn, track_registration_id).await?
    else {
        return Ok(());
    };
    if !matches!(inscription.status.as_str(), "registered" | "attended") {
        if let Some(p) = meetings::participant_de_journee(conn, track_registration_id).await? {
            meetings::annuler_participant(conn, p.id).await?;
        }
        return Ok(());
    }
    let reunion = match meetings::reunion_active_journee(conn, inscription.track_id).await? {
        Some(r) => r.id,
        None => {
            let Some(journee) = programme::journee(conn, inscription.track_id).await? else {
                return Ok(());
            };
            assurer_reunion_journee(conn, &journee, None).await?
        }
    };
    assurer_participant_journee(conn, reunion, &inscription).await
}

async fn assurer_participant_journee(
    conn: &mut PgConnection,
    meeting_id: Uuid,
    i: &InscriptionJournee,
) -> Result<()> {
    let nouveau = NouveauParticipant {
        meeting_id,
        registration_id: None,
        track_registration_id: Some(i.id),
        person_id: i.person_id,
        display_name: &i.display_name,
        email: &i.email,
        first_name: &i.first_name,
        last_name: &i.last_name,
    };
    if let Some(p) = meetings::participant_de_journee(conn, i.id).await? {
        if p.meeting_id == meeting_id && p.cancelled_at.is_none() {
            return Ok(());
        }
        return meetings::reprendre_participant(conn, p.id, &nouveau).await;
    }
    reprendre_ou_inserer(conn, meeting_id, &nouveau, None).await
}

/// Le déroulé a bougé (activité déplacée, ajoutée, annulée) : la réunion de la
/// journée, si elle existe, prend ses nouvelles bornes.
pub async fn recaler_journee(conn: &mut PgConnection, track_id: Uuid) -> Result<()> {
    if meetings::reunion_active_journee(conn, track_id)
        .await?
        .is_none()
    {
        return Ok(());
    }
    let Some(journee) = programme::journee(conn, track_id).await? else {
        return Ok(());
    };
    meetings::recaler_reunion_journee(
        conn,
        track_id,
        journee.debut,
        minutes_entre(journee.debut, journee.fin),
    )
    .await
}

/// Recrée la réunion de la journée et une place par inscription active.
pub async fn reconstruire_journee(
    conn: &mut PgConnection,
    journee: &Journee,
    external_id: Option<&str>,
) -> Result<(Uuid, bool)> {
    let (meeting_id, creee) = match meetings::reunion_active_journee(conn, journee.id).await? {
        Some(r) => {
            if let Some(n) = external_id {
                if r.external_id.as_deref() != Some(n) {
                    meetings::rattacher(conn, r.id, n).await?;
                }
            }
            (r.id, false)
        }
        None => (
            assurer_reunion_journee(conn, journee, external_id).await?,
            true,
        ),
    };
    for id in programme::inscriptions_journee_actives(conn, journee.id).await? {
        synchroniser_journee(conn, id).await?;
    }
    Ok((meeting_id, creee))
}
