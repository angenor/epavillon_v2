//! L'entrée libre : une séance diffusée sans inscription
//! (`registration_required` faux) accueille toute personne connectée.
//!
//! Sa ligne `live.meeting_participants` naît à la première entrée, sans
//! inscription plateforme, et l'inscription chez Zoom se fait DANS la requête :
//! la réponse doit porter le jeton `tk` tout de suite. Un échec de Zoom laisse
//! la ligne en `failed` pour le filet de sécurité et répond « pas prête ».

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use std::time::Duration;
use uuid::Uuid;

use crate::repo::cross::identity;
use crate::repo::cross::programme::Seance;
use crate::repo::meetings::{self, Place};
use crate::service::inscrits;
use crate::state::LiveState;
use crate::zoom::Zoom;

/// Au-delà, la personne réessaie ; la ligne reste à reprendre.
const DELAI_ZOOM: Duration = Duration::from_secs(10);

pub async fn place(
    state: &LiveState,
    ctx: &RequestContext,
    zoom: &Zoom,
    seance: &Seance,
    statut: Option<&str>,
    person_id: Uuid,
) -> Result<Option<Place>> {
    let pas_prete = || ApiError::new(ErrorCode::LiveMeetingNotReady);
    let mut tx = state.db().write(ctx).await?;

    let Some(reunion) = meetings::reunion_active(&mut tx, seance.id).await? else {
        if seance.is_streamed {
            inscrits::assurer_reunion(&mut tx, seance, None).await?;
            tx.commit().await?;
        }
        return Err(pas_prete());
    };
    let numero = match (&reunion.external_id, statut) {
        (Some(n), Some("scheduled" | "started")) => n.clone(),
        _ => return Err(pas_prete()),
    };

    let personne = identity::personne(&mut tx, person_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let charge = serde_json::json!({
        "email": personne.email,
        "first_name": personne.first_name,
        "last_name": personne.last_name,
    });

    let (participant_id, deja_inscrit) =
        match meetings::participant_libre(&mut tx, reunion.id, person_id, &personne.email).await? {
            Some(p) if p.pret => {
                let place = meetings::place_du_participant(&mut tx, p.id).await?;
                tx.commit().await?;
                return Ok(place);
            }
            Some(p) => {
                if !meetings::reserver_participant_libre(
                    &mut tx,
                    p.id,
                    person_id,
                    &personne.display_name,
                    &charge,
                )
                .await?
                {
                    return Err(pas_prete());
                }
                (p.id, p.external_registrant_id.is_some())
            }
            None => {
                let id = meetings::inserer_participant_libre(
                    &mut tx,
                    reunion.id,
                    person_id,
                    &personne.display_name,
                    &personne.email,
                    &charge,
                )
                .await?
                .ok_or_else(pas_prete)?;
                (id, false)
            }
        };
    tx.commit().await?;

    let prenom: String = if personne.first_name.trim().is_empty() {
        personne.display_name.chars().take(64).collect()
    } else {
        personne.first_name.chars().take(64).collect()
    };
    let nom: String = personne.last_name.chars().take(64).collect();
    let resultat = tokio::time::timeout(
        DELAI_ZOOM,
        zoom.client
            .inscrire_et_approuver(&numero, &personne.email, &prenom, &nom, deja_inscrit),
    )
    .await;

    let mut tx = state.db().write(ctx).await?;
    let issue = match resultat {
        Ok(Ok(inscrit)) => {
            meetings::inscrit_synchronise(
                &mut tx,
                participant_id,
                &inscrit.registrant_id,
                inscrit.join_url.as_deref(),
            )
            .await?;
            meetings::place_du_participant(&mut tx, participant_id).await?
        }
        Ok(Err(e)) => {
            meetings::inscrit_en_echec(&mut tx, participant_id, &e.to_string()).await?;
            tracing::warn!(participant = %participant_id, erreur = %e, "entrée libre : Zoom n'a pas inscrit");
            None
        }
        Err(_) => {
            meetings::inscrit_en_echec(&mut tx, participant_id, "Zoom n'a pas répondu à temps")
                .await?;
            None
        }
    };
    tx.commit().await?;
    issue.map(Some).ok_or_else(pas_prete)
}
