//! Rendre publiques les séances qu'une publication de programme désigne.
//!
//! # L'autre moitié d'un geste partagé entre deux modules
//!
//! B3 contrôle, estampille l'édition et **annonce** ; B5 reçoit et rend
//! publiques les séances désignées. La vue de la programmation publique filtre
//! sur `published_at` **de chaque séance**, pas sur la date de l'édition :
//! rendre le programme public exige donc deux écritures, dans deux schémas.
//! Écrire dans `programme` depuis `event` romprait la frontière ; tout confier à
//! `programme` la romprait dans l'autre sens. L'outbox est la troisième voie.
//!
//! # Un seul ordre, deux colonnes — et le changement d'état est VOULU
//!
//! La publication pose la date **et** fait passer « pressenti » à « programmé ».
//! Trois preuves concordantes le demandent (research.md § R12) : le modèle nomme
//! l'état — `'scheduled'  -- programmé et publié` contre `'planned'  -- créneau
//! pressenti, non public` —, la feuille de style du front colore `planned` comme
//! l'état de travail, et les données simulées font de même. Ne poser que la date
//! laisserait `scheduled` **mort**, et le calendrier du back-office colorerait en
//! « état de travail » des séances déjà publiques.
//!
//! Le déclencheur d'émission trie lui-même : son corps sort pour les lignes dont
//! l'état n'a pas changé. Une édition de quarante séances « pressenties » émet
//! donc quarante `programme.session.scheduled` — exactement le signal dont B6 a
//! besoin pour planifier les rappels — et une republication n'en émet aucun.
//!
//! # Ce que ce service n'écrit JAMAIS
//!
//! `event.events.programme_published_at`. Elle est déjà posée par l'émetteur, et
//! écrire hors de son schéma dans un module métier est interdit.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

/// Le prédicat porté par l'annonce, appliqué **tel quel**.
///
/// Il voyage plutôt que d'être redéduit : l'émetteur a compté sous l'instantané
/// de sa transaction, et un consommateur qui recalculerait « les séances de
/// l'édition » publierait autre chose que ce qui a été annoncé.
pub struct Designation<'a> {
    pub event_id: Uuid,
    pub published_at: OffsetDateTime,
    /// États retenus — `planned` et `scheduled` aujourd'hui.
    pub statuses: &'a [String],
    pub only_unpublished: bool,
}

/// Publier les séances désignées, et rendre leur nombre.
pub async fn publier_les_seances(
    conn: &mut PgConnection,
    designation: Designation<'_>,
) -> Result<u64> {
    let publiees = sqlx::query!(
        "UPDATE programme.sessions
            SET published_at = $2,
                status = CASE WHEN status = 'planned'
                              THEN 'scheduled'::programme.session_status
                              ELSE status END
          WHERE event_id = $1
            AND status::text = ANY($3)
            AND ($4 = false OR published_at IS NULL)",
        designation.event_id,
        designation.published_at,
        designation.statuses,
        designation.only_unpublished
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(publiees)
}
