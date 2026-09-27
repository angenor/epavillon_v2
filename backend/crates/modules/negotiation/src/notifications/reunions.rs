//! Prévenir des réunions de la Francophonie (research R8), dans la transaction
//! qui écrit : la réunion est relue après écriture, l'avis dit donc la valeur
//! nouvelle ; un brouillon ne prévient personne. Le courriel est posé du même
//! geste ; l'accord se lit au départ.

use contracts::negotiation::{
    AGGREGATE_MEETING, FRANCOPHONE_MEETING_CHANGED, MEETING_REGISTRATION_PROMOTED,
};
use kernel::error::Result;
use serde_json::json;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::jobs::change_email::{self, Cible as CibleCourriel};
use crate::jobs::promotion_email;
use crate::notifications::avis::{self, Cible, Etat, Reunion, Sujet};
use crate::notifications::emission::emettre;
use crate::repo::courriel::{self as depot, ReunionFrancophone};

pub(crate) fn vue(r: &ReunionFrancophone) -> Reunion<'_> {
    Reunion {
        sujet: Sujet {
            fr: &r.title_fr,
            en: &r.title_en,
        },
        jour: r.jour,
        debut: &r.debut,
        ville: r.ville.as_deref(),
        fuseau: &r.fuseau,
        format: &r.format,
        lieu: r.lieu.as_deref(),
        motif: r.motif.as_deref(),
    }
}

fn cible(id: Uuid) -> Cible<'static> {
    Cible {
        table: "meetings",
        id,
        lien: CibleCourriel::FrancophoneMeeting.chemin(id),
    }
}

/// Annulée, déplacée ou lieu changé : inscrites et liste d'attente.
pub async fn changement(conn: &mut PgConnection, id: Uuid, etat: Etat) -> Result<()> {
    let destinataires = depot::audience_reunion(conn, id).await?;
    if destinataires.is_empty() {
        return Ok(());
    }
    let Some(r) = depot::reunion_francophone(conn, id)
        .await?
        .filter(|r| r.status != "draft")
    else {
        return Ok(());
    };
    let notification = avis::notification(
        FRANCOPHONE_MEETING_CHANGED,
        destinataires.clone(),
        avis::changement_de_reunion(etat, &vue(&r)),
        cible(id),
        Some(avis::cle_du_jour(
            FRANCOPHONE_MEETING_CHANGED,
            id,
            r.aujourdhui,
        )),
        json!({ "change": etat }),
    );
    emettre(
        conn,
        AGGREGATE_MEETING,
        id,
        FRANCOPHONE_MEETING_CHANGED,
        notification,
    )
    .await?;
    change_email::poser_reunion(conn, id, etat, &destinataires).await
}

/// Un avis par personne promue, jamais regroupé : chacune a sa place.
pub async fn promotions(conn: &mut PgConnection, id: Uuid, promues: &[Uuid]) -> Result<()> {
    if promues.is_empty() {
        return Ok(());
    }
    let Some(r) = depot::reunion_francophone(conn, id)
        .await?
        .filter(|r| r.status != "draft")
    else {
        return Ok(());
    };
    for &personne in promues {
        let notification = avis::notification(
            MEETING_REGISTRATION_PROMOTED,
            vec![personne],
            avis::place_obtenue(&vue(&r)),
            cible(id),
            None,
            json!({}),
        );
        emettre(
            conn,
            AGGREGATE_MEETING,
            id,
            MEETING_REGISTRATION_PROMOTED,
            notification,
        )
        .await?;
        promotion_email::poser(conn, id, personne).await?;
    }
    Ok(())
}
