//! Le back-office de l'import (FR-040). Allumer pose la première lecture dans
//! la transaction du réglage ; éteindre coupe l'affichage aussitôt, puisque
//! `import_is_serving` lit l'interrupteur. Aucune session n'est écrite ici.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::admin_import::{
    AgendaItemAdmin, OfficialImportAdmin, UpdateOfficialImportPayload,
};
use crate::import::archive::JEUX;
use crate::jobs::import as travail;
use crate::repo::admin_import as depot;
use crate::state::NegotiationState;

/// Un réglage jamais posé se lit avec les valeurs par défaut du modèle.
const LECTEUR_PAR_DEFAUT: &str = "archive";
const CORRECTION_PAR_DEFAUT: i16 = 60;
const INTERVALLE_PAR_DEFAUT: i32 = 300;
const SEUIL_PAR_DEFAUT: i16 = 3;
/// Une demi-journée suffit à toute erreur de fuseau imaginable.
const CORRECTION_MAX: i16 = 720;
/// `import_is_serving` multiplie seuil et intervalle en `integer` : ces bornes
/// l'en gardent, et une source lue moins d'une fois par jour n'a plus de sens.
const INTERVALLE_MAX: i32 = 86_400;
const SEUIL_MAX: i16 = 100;

async fn edition(conn: &mut PgConnection, slug: &str) -> Result<depot::Edition> {
    depot::edition(conn, slug)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationEditionUnknown).field("edition"))
}

async fn composer(conn: &mut PgConnection, e: depot::Edition) -> Result<OfficialImportAdmin> {
    let (session_count, sans_theme) = depot::comptes(conn, e.id).await?;
    let archives = JEUX.iter().map(|j| (*j).to_owned()).collect();
    let Some(etat) = depot::etat(conn, e.id).await? else {
        return Ok(OfficialImportAdmin {
            edition: e.edition,
            enabled: false,
            reader: LECTEUR_PAR_DEFAUT.to_owned(),
            archive_name: None,
            archive_first_day: None,
            archives,
            live_url: None,
            time_correction_minutes: CORRECTION_PAR_DEFAUT,
            official_programme_url: String::new(),
            interval_seconds: INTERVALLE_PAR_DEFAUT,
            missed_threshold: SEUIL_PAR_DEFAUT,
            missed_reads: 0,
            serving: false,
            last_success_at: None,
            last_attempt_at: None,
            last_error: None,
            failing_since: None,
            last_change_count: None,
            session_count,
            agenda_items_without_theme: sans_theme,
            runs: Vec::new(),
        });
    };
    let runs = depot::journal(conn, etat.import_id).await?;
    Ok(OfficialImportAdmin {
        edition: e.edition,
        enabled: etat.enabled,
        reader: etat.reader,
        archive_name: etat.archive_name,
        archive_first_day: etat.archive_first_day,
        archives,
        live_url: etat.live_url,
        time_correction_minutes: etat.time_correction_minutes,
        official_programme_url: etat.official_programme_url,
        interval_seconds: etat.interval_seconds,
        missed_threshold: etat.missed_threshold,
        missed_reads: etat.missed_reads,
        serving: etat.serving,
        last_success_at: etat.last_success_at,
        last_attempt_at: etat.last_attempt_at,
        last_error: etat.last_error,
        failing_since: etat.failing_since,
        last_change_count: etat.last_change_count,
        session_count,
        agenda_items_without_theme: sans_theme,
        runs,
    })
}

pub async fn lire(state: &NegotiationState, slug: &str) -> Result<OfficialImportAdmin> {
    let mut conn = state.pool().acquire().await?;
    let e = edition(&mut conn, slug).await?;
    composer(&mut conn, e).await
}

fn invalide(champ: &str, message: &str) -> ApiError {
    ApiError::with_message(ErrorCode::NegotiationImportConfigInvalid, message).field(champ)
}

fn est_une_adresse(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= 2048
        && !url.chars().any(char::is_whitespace)
}

/// Les champs vides deviennent nuls ; le premier défaut nomme son champ.
fn valider(mut r: UpdateOfficialImportPayload) -> Result<UpdateOfficialImportPayload> {
    let vide = |v: Option<String>| v.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    r.archive_name = vide(r.archive_name);
    r.live_url = vide(r.live_url);
    r.official_programme_url = r.official_programme_url.trim().to_owned();

    match r.reader.as_str() {
        "archive" => match r.archive_name.as_deref() {
            None => {
                return Err(invalide(
                    "archive_name",
                    "Choisissez le jeu archivé à lire.",
                ))
            }
            Some(nom) if !JEUX.contains(&nom) => {
                return Err(invalide(
                    "archive_name",
                    &format!("Le jeu archivé « {nom} » n'existe pas."),
                ))
            }
            Some(_) => {}
        },
        "live" if r.live_url.is_none() => {
            return Err(invalide(
                "live_url",
                "Indiquez l'adresse de la source officielle.",
            ))
        }
        "live" => {}
        _ => {
            return Err(invalide(
                "reader",
                "Choisissez un lecteur : archive ou source.",
            ))
        }
    }
    if r.live_url.as_deref().is_some_and(|u| !est_une_adresse(u)) {
        return Err(invalide(
            "live_url",
            "L'adresse de la source doit commencer par https://.",
        ));
    }
    if !est_une_adresse(&r.official_programme_url) {
        return Err(invalide(
            "official_programme_url",
            "Indiquez l'adresse du programme officiel, en https://.",
        ));
    }
    if !(60..=INTERVALLE_MAX).contains(&r.interval_seconds) {
        return Err(invalide(
            "interval_seconds",
            "L'intervalle entre deux lectures va de 60 secondes à un jour.",
        ));
    }
    if !(1..=SEUIL_MAX).contains(&r.missed_threshold) {
        return Err(invalide(
            "missed_threshold",
            "Le seuil de lectures manquées va de 1 à 100.",
        ));
    }
    if r.time_correction_minutes.abs() > CORRECTION_MAX {
        return Err(invalide(
            "time_correction_minutes",
            "La correction horaire ne dépasse pas douze heures.",
        ));
    }
    Ok(r)
}

pub async fn regler(
    state: &NegotiationState,
    ctx: &RequestContext,
    acteur: Uuid,
    slug: &str,
    demande: UpdateOfficialImportPayload,
) -> Result<OfficialImportAdmin> {
    let reglage = valider(demande)?;

    let mut tx = state.db().write(ctx).await?;
    let e = edition(&mut tx, slug).await?;
    depot::regler(&mut tx, e.id, &reglage, acteur).await?;
    if reglage.enabled {
        travail::poser_maintenant(
            &mut tx,
            e.id,
            reglage.interval_seconds,
            OffsetDateTime::now_utc(),
        )
        .await?;
    }
    let reponse = composer(&mut tx, e).await?;
    tx.commit().await?;
    Ok(reponse)
}

/// « Lire maintenant ». Éteint, la lecture a lieu et l'affichage reste coupé.
pub async fn lire_maintenant(
    state: &NegotiationState,
    ctx: &RequestContext,
    slug: &str,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    let e = edition(&mut tx, slug).await?;
    if depot::etat(&mut tx, e.id).await?.is_none() {
        return Err(ApiError::new(ErrorCode::NegotiationEditionUnknown).field("edition"));
    }
    travail::poser_manuel(&mut tx, e.id, &ctx.request_id).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn points(state: &NegotiationState, slug: &str) -> Result<Vec<AgendaItemAdmin>> {
    let mut conn = state.pool().acquire().await?;
    let e = edition(&mut conn, slug).await?;
    depot::points(&mut conn, e.id).await
}

/// Rattache un point, ou le détache. Les sessions en héritent à la lecture :
/// rien n'est recopié sur elles.
pub async fn rattacher(
    state: &NegotiationState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: Uuid,
    theme: Option<&str>,
) -> Result<AgendaItemAdmin> {
    let mut tx = state.db().write(ctx).await?;
    let theme_id = match theme.map(str::trim).filter(|c| !c.is_empty()) {
        None => None,
        Some(code) => Some(depot::theme_actif(&mut tx, code).await?.ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::NegotiationThemeUnknown,
                format!("La thématique « {code} » n'existe pas."),
            )
            .field("theme")
        })?),
    };
    if !depot::rattacher(&mut tx, id, theme_id, acteur).await? {
        return Err(ApiError::new(ErrorCode::NegotiationAgendaItemUnknown));
    }
    let point = depot::point(&mut tx, id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationAgendaItemUnknown))?;
    tx.commit().await?;
    Ok(point)
}
