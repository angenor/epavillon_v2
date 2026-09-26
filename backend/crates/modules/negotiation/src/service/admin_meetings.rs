//! Saisir, publier, annuler une réunion de la Francophonie, la lier au
//! Pavillon. Les invariants sont en base ; le code les traduit et nomme le
//! champ (`repo::meetings::refus`). Les chevauchements ne sont jamais bloqués
//! (FR-019).

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::admin_meetings::{
    AdminFrancophoneMeeting, AdminFrancophoneMeetings, AdminMeetingRegistrations,
    FrancophoneMeetingInput, PavilionActivityOption,
};
use crate::domain::sessions::EditionServie;
use crate::repo::admin_meetings::{self as repo, Avant, Origine, Saisie};
use crate::repo::meetings::{self, invalide};
use crate::repo::{cross, import, meeting_registrations};
use crate::service::meeting_registrations::prevenir_des_promotions;
use crate::state::NegotiationState;

fn inconnue() -> ApiError {
    ApiError::new(ErrorCode::NegotiationMeetingUnknown)
}

async fn edition_de(conn: &mut sqlx::PgConnection, slug: &str) -> Result<meetings::Edition> {
    meetings::edition(conn, slug.trim())
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::NegotiationEditionUnknown).field("edition"))
}

pub async fn lister(state: &NegotiationState, edition: &str) -> Result<AdminFrancophoneMeetings> {
    let mut conn = state.pool().acquire().await?;
    let edition = edition_de(&mut conn, edition).await?;
    let reunions = repo::lire(&mut conn, Some(edition.event_id), None).await?;
    Ok(AdminFrancophoneMeetings {
        edition: EditionServie {
            slug: edition.slug,
            timezone: edition.timezone,
            city: edition.city,
        },
        meetings: reunions,
    })
}

async fn une(conn: &mut sqlx::PgConnection, id: Uuid) -> Result<AdminFrancophoneMeeting> {
    repo::lire(conn, None, Some(id))
        .await?
        .into_iter()
        .next()
        .ok_or_else(inconnue)
}

pub async fn fiche(state: &NegotiationState, id: Uuid) -> Result<AdminFrancophoneMeeting> {
    let mut conn = state.pool().acquire().await?;
    une(&mut conn, id).await
}

fn texte(v: &Value, champ: &str) -> Result<()> {
    let fr = v.get("fr").and_then(Value::as_str).unwrap_or("").trim();
    let objet_de_chaines = v
        .as_object()
        .is_some_and(|o| o.values().all(Value::is_string));
    if fr.is_empty() || !objet_de_chaines {
        return Err(invalide(champ, "Le texte en français est obligatoire."));
    }
    Ok(())
}

fn nettoye(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

/// Le corps → ce qui s'écrit. La nature dit le `kind` (R9 bis) ; sans nature,
/// un brouillon est une concertation.
async fn saisie(conn: &mut sqlx::PgConnection, e: FrancophoneMeetingInput) -> Result<Saisie> {
    texte(&e.title, "title")?;
    if let Some(d) = &e.description {
        texte(d, "description")?;
    }
    let access_audience = if e.open_access {
        None
    } else {
        e.access_audience
    };
    if let Some(a) = &access_audience {
        texte(a, "access_audience")?;
    }
    if !e.is_ifdd_organized && e.organizer_org_id.is_none() {
        return Err(invalide(
            "organizer_org_id",
            "Choisissez l'organisation qui tient la réunion.",
        ));
    }
    let code = nettoye(e.type_code);
    let nature = match &code {
        Some(c) => Some(
            repo::nature(conn, c)
                .await?
                .ok_or_else(|| invalide("type", "Cette nature de réunion n'existe pas."))?,
        ),
        None => None,
    };
    Ok(Saisie {
        nature,
        kind: match code.as_deref() {
            Some("preparatory_workshop") => "preparatory_workshop",
            _ => "francophone_consultation",
        },
        title: e.title,
        description: e.description,
        start_at: e.start_at,
        end_at: e.end_at,
        format: e.format.as_str(),
        venue: nettoye(e.venue),
        external_url: nettoye(e.external_url),
        capacity: e.capacity,
        waitlist_enabled: e.waitlist_enabled,
        requires_registration: e.requires_registration,
        registration_opens_at: e.registration_opens_at,
        registration_closes_at: e.registration_closes_at,
        open_access: e.open_access,
        access_audience,
        is_ifdd_organized: e.is_ifdd_organized,
        organizer_org_id: if e.is_ifdd_organized {
            None
        } else {
            e.organizer_org_id
        },
    })
}

/// `<édition>-<nature>-<suffixe>` : le suffixe vient de la partie aléatoire
/// d'un UUIDv7.
fn slug(edition: &str, nature: Option<Uuid>, code: Option<&str>) -> String {
    let nature = match (nature, code) {
        (Some(_), Some(c)) => c.replace('_', "-"),
        _ => "reunion".to_owned(),
    };
    let aleatoire = Uuid::now_v7().simple().to_string();
    format!("{edition}-{nature}-{}", &aleatoire[aleatoire.len() - 6..])
}

pub async fn creer(
    state: &NegotiationState,
    ctx: &RequestContext,
    entree: FrancophoneMeetingInput,
) -> Result<AdminFrancophoneMeeting> {
    let auteur = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let slug_edition = entree
        .edition
        .clone()
        .ok_or_else(|| invalide("edition", "Choisissez l'édition de la réunion."))?;
    let code = nettoye(entree.type_code.clone());
    let mut tx = state.db().write(ctx).await?;
    let edition = edition_de(&mut tx, &slug_edition).await?;
    let s = saisie(&mut tx, entree).await?;
    let space_id = import::espace_climat(&mut tx).await?;
    let slug = slug(&edition.slug, s.nature, code.as_deref());
    let id = repo::inserer(
        &mut tx,
        &Origine {
            space_id,
            event_id: edition.event_id,
            slug: &slug,
            timezone: &edition.timezone,
            created_by: auteur,
        },
        &s,
    )
    .await?;
    let reunion = une(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reunion)
}

/// Pas de garde sur `updated_at` : chaque inscription réécrit la réunion (R4).
pub async fn modifier(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    entree: FrancophoneMeetingInput,
) -> Result<AdminFrancophoneMeeting> {
    let mut tx = state.db().write(ctx).await?;
    let avant = repo::verrouiller(&mut tx, id).await?.ok_or_else(inconnue)?;
    let s = saisie(&mut tx, entree).await?;
    repo::modifier(&mut tx, id, &s).await?;

    let relevee = match (avant.capacity, s.capacity) {
        (Some(_), None) => true,
        (Some(a), Some(b)) => b > a,
        (None, _) => false,
    };
    if relevee {
        let promues = meeting_registrations::promouvoir(&mut tx, id).await?;
        prevenir_des_promotions(id, &promues);
    }
    if avant.status == "scheduled" && change_dheure_ou_de_lieu(&avant, &s) {
        prevenir_dun_changement(id, &avant, &s);
    }
    let reunion = une(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reunion)
}

fn change_dheure_ou_de_lieu(avant: &Avant, s: &Saisie) -> bool {
    avant.start_at != s.start_at
        || avant.end_at != Some(s.end_at)
        || avant.format != s.format
        || avant.venue != s.venue
}

/// T016 : l'avis et le courriel du changement d'heure ou de lieu d'une réunion
/// publiée partiront d'ici (`meeting_audience()`), l'ancienne et la nouvelle
/// valeur en main. Rien n'est émis à la phase 3.
fn prevenir_dun_changement(_meeting_id: Uuid, _avant: &Avant, _apres: &Saisie) {}

/// T016 : l'avis et le courriel de l'annulation, motif compris, partiront d'ici
/// vers les inscrites et la liste d'attente. Rien n'est émis à la phase 3.
fn prevenir_de_lannulation(_meeting_id: Uuid, _motif: &str) {}

/// Idempotent sur une réunion déjà publiée ; une annulée ne se republie pas.
pub async fn publier(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
) -> Result<AdminFrancophoneMeeting> {
    let mut tx = state.db().write(ctx).await?;
    let avant = repo::verrouiller(&mut tx, id).await?.ok_or_else(inconnue)?;
    match avant.status.as_str() {
        "draft" => repo::publier(&mut tx, id).await?,
        "cancelled" => return Err(ApiError::new(ErrorCode::NegotiationMeetingUnavailable)),
        _ => {}
    }
    let reunion = une(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reunion)
}

pub async fn annuler(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    motif: &str,
) -> Result<AdminFrancophoneMeeting> {
    let motif = motif.trim();
    if motif.is_empty() {
        return Err(invalide("reason", "L'annulation demande un motif."));
    }
    let mut tx = state.db().write(ctx).await?;
    let avant = repo::verrouiller(&mut tx, id).await?.ok_or_else(inconnue)?;
    if avant.status != "cancelled" {
        repo::annuler(&mut tx, id, motif).await?;
        if avant.status != "draft" {
            prevenir_de_lannulation(id, motif);
        }
    }
    let reunion = une(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reunion)
}

/// La base garde la même édition (`ck_meetings_pavilion_edition`).
pub async fn lier_au_pavillon(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    activite: Option<Uuid>,
) -> Result<AdminFrancophoneMeeting> {
    let mut tx = state.db().write(ctx).await?;
    repo::verrouiller(&mut tx, id).await?.ok_or_else(inconnue)?;
    repo::lier_au_pavillon(&mut tx, id, activite).await?;
    let reunion = une(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reunion)
}

pub async fn inscrites(state: &NegotiationState, id: Uuid) -> Result<AdminMeetingRegistrations> {
    let mut conn = state.pool().acquire().await?;
    une(&mut conn, id).await?;
    let mut reponse = AdminMeetingRegistrations {
        registered: Vec::new(),
        waitlisted: Vec::new(),
    };
    for (statut, ligne) in repo::inscrites(&mut conn, id).await? {
        if statut == "waitlisted" {
            reponse.waitlisted.push(ligne);
        } else {
            reponse.registered.push(ligne);
        }
    }
    Ok(reponse)
}

pub async fn activites_du_pavillon(
    state: &NegotiationState,
    edition: &str,
) -> Result<Vec<PavilionActivityOption>> {
    let mut conn = state.pool().acquire().await?;
    let edition = edition_de(&mut conn, edition).await?;
    cross::activites_du_pavillon(&mut conn, edition.event_id).await
}
