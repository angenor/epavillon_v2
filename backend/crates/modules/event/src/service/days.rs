//! Le calendrier d'une édition — **le plan, puis son exécution**.
//!
//! Rien en base ne dérive ces journées : `event.event_days` n'a aucun
//! déclencheur de dérivation. Créer le calendrier est donc un comportement
//! d'application, et un **geste explicite** — qui s'annonce avant de s'exécuter.
//!
//! Trois règles gouvernent ce fichier.
//!
//! 1. **Le plan n'écrit rien.** Il se demande, se lit, et laisse la base
//!    exactement telle qu'il l'a trouvée. Une période d'un an annonce plus de
//!    trois cents journées sans en écrire une.
//! 2. **L'exécution recalcule le plan dans sa propre transaction** (research.md
//!    § R4). Entre l'affichage et le clic, quelqu'un peut avoir modifié la
//!    période ou créé une journée. Faire confiance au plan renvoyé, c'est
//!    **supprimer une journée qui vient d'entrer dans la période**, avec les
//!    séances qu'elle porte.
//! 3. **La régénération n'écrase aucun contenu éditorial.** Titre, adresse,
//!    couleur et mise en avant appartiennent à l'équipe ; la génération ne
//!    connaît que des dates et des rangs.

use kernel::context::RequestContext;
use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgPool;
use time::Date;
use uuid::Uuid;

use crate::domain::calendar;
use crate::domain::ids::{EventDayId, EventId};
use crate::domain::tabs::{
    DayGenerationPlan, DayToReview, EditionDayPayload, EditionTabResult, TabErrorCode,
};
use crate::repo::{cross, days, editions};
use crate::state::EventState;

use super::tabs;

/// **Ce que la génération ferait, sans rien faire.**
///
/// Les journées hors période arrivent avec **le nombre de séances qu'elles
/// portent** : c'est ce chiffre qui permet à l'équipe d'arbitrer plutôt que de
/// se voir imposer un retrait.
pub async fn plan(pool: &PgPool, event_id: EventId) -> Result<Option<DayGenerationPlan>> {
    let mut conn = pool.acquire().await?;
    calculer(&mut conn, event_id).await
}

/// Le plan, sur une connexion donnée — la même fonction sert la lecture et
/// l'exécution, qui la rejoue dans sa transaction.
async fn calculer(conn: &mut PgConnection, event_id: EventId) -> Result<Option<DayGenerationPlan>> {
    let periode = editions::periode_civile(&mut *conn, event_id).await?;
    let (Some(premier), Some(dernier)) = (periode.first().copied(), periode.last().copied()) else {
        return Ok(None);
    };

    let existantes = days::journees_du_plan(&mut *conn, event_id).await?;
    let dates: Vec<Date> = existantes.iter().map(|(_, date)| *date).collect();
    let brut = calendar::plan(premier, dernier, &dates);

    let seances = cross::seances_par_journee(&mut *conn, event_id).await?;

    let to_review = existantes
        .iter()
        .filter(|(_, date)| brut.to_review.contains(date))
        .map(|(id, date)| DayToReview {
            id: *id,
            day_date: *date,
            session_count: seances.get(id).copied().unwrap_or(0),
        })
        .collect();

    Ok(Some(DayGenerationPlan {
        to_create: brut.to_create,
        to_review,
        unchanged: brut.unchanged,
    }))
}

/// **Générer le calendrier.** Le plan est recalculé ici, et le client n'apporte
/// que son drapeau.
///
/// Sans `remove_outside_period`, **aucune journée n'est retirée** : une soirée
/// d'ouverture la veille est un cas légitime, et le choix appartient à l'équipe
/// (FR-035).
pub async fn generer(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    retirer_hors_periode: bool,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    let Some(plan) = calculer(&mut tx, event_id).await? else {
        return Ok(EditionTabResult::refuse(TabErrorCode::NotFound));
    };

    // Le rang est la position dans la **période entière**, et non dans le lot
    // créé : deux générations successives laissent ainsi un ordre cohérent.
    let periode = editions::periode_civile(&mut *tx, event_id).await?;
    let a_creer: Vec<(Date, i16)> = plan
        .to_create
        .iter()
        .map(|jour| {
            let rang = periode.iter().position(|d| d == jour).unwrap_or(0);
            (*jour, i16::try_from(rang).unwrap_or(i16::MAX))
        })
        .collect();

    days::creer(&mut tx, event_id, &a_creer).await?;

    // **Le décompte AVANT le retrait** : après l'ordre, le lien n'existe plus et
    // le chiffre rendrait zéro (research.md § R8).
    let mut detachees = 0;
    if retirer_hors_periode {
        detachees = plan.to_review.iter().map(|j| j.session_count).sum();
        let ids: Vec<Uuid> = plan.to_review.iter().map(|j| j.id).collect();
        days::supprimer(&mut tx, &ids).await?;
    }

    tx.commit().await?;

    tabs::reussite(state.pool(), event_id, detachees).await
}

/// L'habillage **éditorial** d'une journée. La date ne s'y modifie pas : elle
/// vient de la période, et la déplacer ferait un doublon ou un trou.
pub async fn habiller(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
    id: EventDayId,
    payload: EditionDayPayload,
) -> Result<EditionTabResult> {
    let mut tx = state.db().write(ctx).await?;

    match days::habiller(&mut tx, id, &payload).await {
        Err(e) => return tabs::refus_de_base(e),
        Ok(false) => return Ok(EditionTabResult::refuse(TabErrorCode::NotFound)),
        Ok(true) => {}
    }

    tx.commit().await?;

    tabs::reussite(state.pool(), event_id, 0).await
}
