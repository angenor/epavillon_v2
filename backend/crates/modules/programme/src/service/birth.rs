//! **Une activité retenue devient une séance à placer** — l'écart n° 57,
//! ouvert depuis le 18/08.
//!
//! # Ce que la base ne fait pas, et que personne d'autre ne fera
//!
//! Aucun déclencheur, aucune fonction : `programme.sessions` n'est peuplée que
//! par une insertion. Sans ce fichier, retenir un dossier ne produit rien, et le
//! planificateur reste devant une grille vide alors que l'équipe vient de
//! décider.
//!
//! # Pourquoi c'est SYNCHRONE, et pas un consommateur d'outbox
//!
//! Plus orthodoxe, et moins bon : le planificateur doit avoir quelque chose à
//! placer **au moment où l'équipe regarde son écran**. Un décalage de quelques
//! secondes entre « retenu » et l'apparition de la carte se lit comme une panne,
//! et personne ne rechargera pour vérifier. La transaction unique donne aussi
//! l'atomicité : un dossier retenu **a** ses séances, ou n'est pas retenu (R3).
//!
//! # Ce service n'émet AUCUN événement
//!
//! `tg_sessions_emit_events()` émet `programme.session.created` à l'insertion,
//! **dans cette transaction**. Émettre à son tour produirait deux annonces par
//! séance née — donc deux jeux de rappels planifiés par B6 —, et **le doublon ne
//! se verrait qu'en production**.

use kernel::error::{ApiError, Result};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::birth::{self, Debut};
use crate::domain::ids::EventId;
use crate::domain::slug;
use crate::repo::sessions::CreneauNaissant;
use crate::repo::{session_parts, sessions, themes};

/// Créer les séances d'un dossier retenu, **dans la transaction de la
/// décision**.
///
/// Rend le nombre de séances effectivement créées : zéro sur une acceptation
/// rejouée, ce qui est le comportement voulu et non un échec.
pub async fn faire_naitre(conn: &mut PgConnection, proposal_id: Uuid) -> Result<usize> {
    let Some(dossier) = sessions::dossier_a_retenir(conn, proposal_id).await? else {
        // Le dossier vient d'être mis à jour dans cette même transaction : son
        // absence est une incohérence, pas un cas d'usage.
        return Err(ApiError::internal(
            "le dossier retenu est introuvable dans sa propre transaction",
        ));
    };

    let regles = match dossier.call_id {
        Some(call_id) => sessions::regles_de_naissance(conn, call_id).await?,
        // Sans appel, il n'y a ni heure d'ouverture ni durée par défaut à lire :
        // le repli est le début de l'édition et soixante minutes (R5).
        None => None,
    };

    let duree_minutes = birth::duree_minutes(
        dossier.duration_minutes,
        regles.as_ref().map(|(_, duree)| *duree),
    );

    // Le domaine décide **quel** créneau ; la composition, elle, se fait en
    // base : une heure murale posée sur un jour civil dans le fuseau d'une
    // édition demande la base de fuseaux de PostgreSQL (R4).
    let creneau = match birth::debut(
        dossier.preferred_start_at,
        regles.as_ref().map(|(heure, _)| *heure),
    ) {
        Debut::Souhaite(instant) => CreneauNaissant {
            souhaite: Some(instant),
            heure_de_lappel: None,
            duree_minutes,
        },
        Debut::PremierJourALHeureDeLAppel(heure) => CreneauNaissant {
            souhaite: None,
            heure_de_lappel: Some(heure_murale(heure)),
            duree_minutes,
        },
        Debut::DebutDeLEdition => CreneauNaissant {
            souhaite: None,
            heure_de_lappel: None,
            duree_minutes,
        },
    };

    let base = slug::base(Some(&dossier.slug));
    let mut nees = 0;

    for rang in birth::rangs(dossier.requested_sessions) {
        let voulue = birth::adresse(&base, rang, dossier.requested_sessions);

        if let Some(session_id) = creer_une_seance(conn, &dossier, rang, &voulue, &creneau).await? {
            composer(conn, dossier.id, session_id).await?;
            nees += 1;
        }
    }

    Ok(nees)
}

/// L'heure d'ouverture, telle que PostgreSQL la relira pour composer un
/// instant. Le format est celui d'un littéral horaire, sans fuseau : c'est
/// l'édition qui en donne un.
fn heure_murale(heure: time::Time) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        heure.hour(),
        heure.minute(),
        heure.second()
    )
}

/// Insérer la séance, **en suffixant l'adresse sur collision**.
///
/// Le suffixe se pose sur collision et jamais par comptage préalable : compter
/// les homonymes avant d'insérer laisserait la course entre deux acceptations
/// simultanées faire échouer la seconde de toute façon (R7).
async fn creer_une_seance(
    conn: &mut PgConnection,
    dossier: &sessions::DossierARetenir,
    rang: i16,
    voulue: &str,
    creneau: &CreneauNaissant,
) -> Result<Option<Uuid>> {
    let edition = EventId::from(dossier.event_id);

    for tentative in 0..slug::TENTATIVES_MAX {
        let adresse = slug::tentative(voulue, tentative);

        if sessions::adresse_prise(conn, edition, &adresse).await? {
            continue;
        }

        return sessions::creer(conn, dossier.id, rang, &adresse, creneau).await;
    }

    Err(ApiError::internal(
        "dix adresses d'URL déjà prises pour la même séance",
    ))
}

/// Ce qui accompagne une séance naissante : intervenants, co-organisations,
/// thématiques.
///
/// **La ligne du porteur n'y est pas** : `tg_sessions_sync_lead_organization()`
/// la pose, et le service ne l'écrit jamais.
async fn composer(conn: &mut PgConnection, proposal_id: Uuid, session_id: Uuid) -> Result<()> {
    session_parts::recopier_les_intervenants(conn, proposal_id, session_id).await?;
    session_parts::recopier_les_coorganisations(conn, proposal_id, session_id).await?;
    themes::recopier_sur_la_seance(conn, proposal_id, session_id).await?;
    Ok(())
}
