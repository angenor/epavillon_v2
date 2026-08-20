//! Ce que **toutes** les écritures d'onglet partagent.
//!
//! Chacune rend `EditionTabResult`, dont `detail` porte la **composition entière
//! recalculée** (FR-024). Écrire cette recomposition une fois évite six copies
//! qui finiraient par diverger — et surtout, elle est la garantie que les
//! décomptes des cinq autres onglets restent justes après une écriture dans le
//! sixième.
//!
//! La traduction des refus est ici aussi, pour la même raison : les six onglets
//! partagent le même vocabulaire d'erreur, et une contrainte de code refuse de
//! la même façon qu'elle vienne d'une salle, d'un fil ou d'un canal.

use kernel::error::{ApiError, Result};
use kernel::pg_error;
use sqlx::PgPool;

use crate::domain::ids::EventId;
use crate::domain::tabs::{EditionTabResult, TabErrorCode};

/// La réussite, avec la composition recalculée.
///
/// L'édition a été vue dans le périmètre juste avant : son absence ici ne peut
/// venir que d'une suppression concurrente, et se rend comme un introuvable.
pub async fn reussite(
    pool: &PgPool,
    event_id: EventId,
    sessions_detached: i64,
) -> Result<EditionTabResult> {
    let detail = super::detail::composer(pool, event_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    Ok(EditionTabResult::reussite(detail, sessions_detached))
}

/// La réussite d'un **canal désactivé** : `ok: true`, avec son mot pour le dire.
pub async fn desactive(
    pool: &PgPool,
    event_id: EventId,
    sessions_detached: i64,
) -> Result<EditionTabResult> {
    let detail = super::detail::composer(pool, event_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    Ok(EditionTabResult::desactive(detail, sessions_detached))
}

/// Le refus de la base, rendu **au formulaire** quand le contrat l'exprime, et
/// remonté en erreur HTTP sinon.
pub fn refus_de_base(erreur: sqlx::Error) -> Result<EditionTabResult> {
    match traduire(&erreur) {
        Some(code) => Ok(EditionTabResult::refuse(code)),
        None => Err(pg_error::translate(&erreur)),
    }
}

/// Les contraintes nommées des six onglets, chacune vers son code du contrat.
///
/// **On branche sur le NOM de la contrainte, jamais sur le texte du message.**
///
/// Ce qui n'est pas ici sort en erreur HTTP par le catalogue du noyau : un
/// responsable de fil inconnu, une langue inconnue, un genre de lieu ou un
/// diffuseur hors vocabulaire — tous `EVENT_UNKNOWN_REFERENCE`, et le contrat
/// des onglets n'a **aucun** code pour les exprimer.
fn traduire(erreur: &sqlx::Error) -> Option<TabErrorCode> {
    use TabErrorCode::*;

    match pg_error::constraint(erreur)? {
        "ck_programme_tracks_period" => Some(Period),
        "ux_programme_tracks_code" | "ux_rooms_code" | "ux_broadcast_channels_code" => {
            Some(CodeTaken)
        }
        "ux_programme_tracks_slug" | "ux_event_days_slug" => Some(SlugTaken),
        "rooms_capacity_check" => Some(Capacity),
        // Forme d'un code, forme d'une couleur : le champ est connu de l'écran,
        // qui n'a qu'un formulaire par onglet.
        "programme_tracks_code_check"
        | "broadcast_channels_code_check"
        | "event_days_color_hex_check"
        | "programme_tracks_color_hex_check" => Some(Required),
        // **Une violation de DOMAINE ne nomme pas sa colonne** : le nom de
        // contrainte y est celui du domaine. Une adresse d'URL ou une adresse de
        // page mal formée est un champ obligatoire mal renseigné, et le contrat
        // des onglets n'a pas de code plus fin.
        "slug_check" => Some(SlugTaken),
        "url_check" => Some(Required),
        _ => None,
    }
}
