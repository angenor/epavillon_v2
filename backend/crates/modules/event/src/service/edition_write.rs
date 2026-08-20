//! Créer et modifier une édition.
//!
//! Trois choses se jouent ici, et une seule est une règle que le service ajoute
//! au modèle.
//!
//! 1. **La règle du sigle** (research.md § R1) — vérifiée sur l'état
//!    **résultant** de l'écriture, avec une valeur proposée. Le modèle ne la
//!    porte pas et ne doit pas la porter.
//! 2. **La traduction des refus de la base** — six contraintes nommées, chacune
//!    rendue sur **son** champ, en 200 : le contrat du front les exprime, donc
//!    ce ne sont pas des erreurs HTTP.
//! 3. **Le calendrier** — les journées manquantes sont créées, **aucune n'est
//!    supprimée** (FR-033).
//!
//! Et une chose ne s'y joue pas : les images. Le formulaire en envoie les
//! identifiants, ce service les **accepte sans les poser** — le rattachement
//! média appartient à B6 (§ R17).

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::pg_error;
use serde_json::json;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::edition::{
    EditionErrorCode, EditionFormError, EditionFormPayload, EditionSaveResult,
};
use crate::domain::ids::EventId;
use crate::domain::{acronym, calendar};
use crate::repo::{days, editions};
use crate::state::EventState;

/// Créer une édition. La portée **globale** est exigée par la route (FR-011) :
/// une édition qui n'existe pas encore n'offre aucune portée où vérifier un
/// droit.
pub async fn creer(
    state: &EventState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: EditionFormPayload,
) -> Result<EditionSaveResult> {
    enregistrer(state, ctx, acteur, payload, None).await
}

/// Modifier une édition. L'identifiant vient de **l'adresse**, jamais du corps :
/// celui de la charge utile est ignoré, comme l'`event_id` des routes d'onglet.
pub async fn modifier(
    state: &EventState,
    ctx: &RequestContext,
    acteur: Uuid,
    id: EventId,
    payload: EditionFormPayload,
) -> Result<EditionSaveResult> {
    enregistrer(state, ctx, acteur, payload, Some(id)).await
}

async fn enregistrer(
    state: &EventState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: EditionFormPayload,
    existante: Option<EventId>,
) -> Result<EditionSaveResult> {
    if let Some(refus) = refus_de_sigle(&payload) {
        return Ok(refus);
    }

    let mut tx = state.db().write(ctx).await?;

    // Lu AVANT l'écriture : c'est la seule façon de dire ensuite *ce qui* a
    // changé, et l'annonce le porte plutôt que d'obliger le consommateur à tout
    // relire.
    let avant = match existante {
        Some(id) => editions::etat_avant(&mut tx, id).await?,
        None => None,
    };

    let id = match existante {
        None => match editions::inserer(&mut tx, &payload, acteur).await {
            Ok(id) => id,
            Err(e) => return refus_de_base(e, &payload),
        },
        Some(id) => match editions::modifier(&mut tx, id, &payload).await {
            // L'édition a disparu entre le contrôle de périmètre et l'écriture.
            Ok(false) => return Err(ApiError::not_found()),
            Ok(true) => id,
            Err(e) => return refus_de_base(e, &payload),
        },
    };

    let days_created = creer_les_journees_manquantes(&mut tx, id).await?;
    annoncer(&mut tx, id, &payload, avant.as_ref()).await?;

    tx.commit().await?;

    let edition = super::edition_read::ligne(state.pool(), id).await?;

    Ok(EditionSaveResult {
        ok: true,
        edition,
        errors: Vec::new(),
        days_created,
        // **Toujours zéro ici** (FR-033) : un enregistrement d'édition ne
        // supprime aucune journée. Le retrait est un geste séparé, et explicite.
        days_removed: 0,
        sessions_detached: 0,
        suggested_acronym: None,
    })
}

// -----------------------------------------------------------------------------
// La règle du sigle
// -----------------------------------------------------------------------------

/// Le refus, s'il y a lieu — **sur l'état résultant** de l'écriture.
///
/// Quatre chemins mènent ici, et on n'en oublie aucun : créer une édition à
/// pavillon sans sigle, basculer une édition en pavillon sans en fournir un,
/// **retirer** le sigle d'une édition à pavillon, et fournir un sigle mal formé.
/// Le troisième est celui qu'on oublie le plus souvent.
fn refus_de_sigle(payload: &EditionFormPayload) -> Option<EditionSaveResult> {
    let sigle = payload
        .acronym
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if acronym::exiger(payload.has_pavilion, sigle) {
        return Some(
            EditionSaveResult::refuse(vec![EditionFormError::new(
                EditionErrorCode::Required,
                "acronym",
            )])
            .avec_sigle_propose(acronym::proposer(payload.titre_pour_proposition())),
        );
    }

    match sigle {
        Some(s) if !acronym::format_valide(s) => Some(
            EditionSaveResult::refuse(vec![EditionFormError::new(
                EditionErrorCode::Required,
                "acronym",
            )])
            .avec_sigle_propose(acronym::proposer(payload.titre_pour_proposition())),
        ),
        _ => None,
    }
}

// -----------------------------------------------------------------------------
// La traduction des refus de la base
// -----------------------------------------------------------------------------

/// Le refus de la base, rendu **au formulaire** quand le contrat l'exprime, et
/// remonté en erreur HTTP sinon.
///
/// La transaction n'est pas reprise : PostgreSQL l'a mise en échec, et elle
/// s'annule en sortant de portée.
fn refus_de_base(erreur: sqlx::Error, payload: &EditionFormPayload) -> Result<EditionSaveResult> {
    match traduire(&erreur, payload) {
        Some(errors) => Ok(EditionSaveResult::refuse(errors)),
        None => Err(pg_error::translate(&erreur)),
    }
}

/// Les **six** contraintes nommées d'une édition, plus les champs obligatoires.
///
/// Ce qui n'est pas ici sort en erreur HTTP par le catalogue du noyau : une
/// série ou un pays inconnus (`EVENT_UNKNOWN_REFERENCE`), un fuseau que la base
/// de fuseaux de PostgreSQL ne connaît pas. Ces refus-là n'ont **aucune place**
/// dans `EditionFormError`, et les y forcer aurait obligé à inventer un code.
fn traduire(erreur: &sqlx::Error, payload: &EditionFormPayload) -> Option<Vec<EditionFormError>> {
    use EditionErrorCode::*;

    // **On branche sur le NOM de la contrainte, jamais sur le texte du
    // message** : le texte est localisé par PostgreSQL et se reformule d'une
    // version à l'autre ; le nom est écrit dans `060_events.sql` et ne bouge
    // qu'avec lui.
    let (code, champ) = match pg_error::constraint(erreur) {
        Some("ck_events_period") => (Period, "ends_at"),
        // « Hors ligne, pays ET ville » : on nomme celui des deux qui manque,
        // le pays d'abord — c'est l'ordre du formulaire.
        Some("ck_events_physical_location") => (
            PhysicalLocation,
            if payload.country_id.is_none() {
                "country_id"
            } else {
                "city"
            },
        ),
        Some("ux_events_slug") => (SlugTaken, "slug"),
        Some("ux_events_series_edition") => (EditionTaken, "edition_label"),
        Some("events_edition_year_check") => (YearRange, "edition_year"),
        // On nomme celle qui a été **donnée seule** : c'est elle que l'écran
        // doit marquer, l'autre étant simplement absente.
        Some("ck_events_coordinates") => (
            Coordinates,
            if payload.latitude.is_some() {
                "latitude"
            } else {
                "longitude"
            },
        ),
        Some("events_latitude_check") => (Coordinates, "latitude"),
        Some("events_longitude_check") => (Coordinates, "longitude"),
        // **Une violation de DOMAINE ne nomme pas sa colonne** : le nom de
        // contrainte y est celui du domaine. Le nom de type, lui, est fiable —
        // c'est ce que `champ_du_domaine` exploite.
        _ => (Required, champ_du_domaine(erreur)?),
    };

    Some(vec![EditionFormError::new(code, champ)])
}

// -----------------------------------------------------------------------------
// Le calendrier
// -----------------------------------------------------------------------------

/// Crée les journées manquantes de la période, **et n'en supprime aucune**
/// (FR-033).
///
/// La période est calculée **en base**, dans le fuseau de l'édition : une
/// édition à Belém commence le 9 novembre, pas le 8 (§ R5). Le rang d'une
/// journée est sa position dans la période entière, et non dans le lot créé :
/// deux générations successives laissent ainsi un ordre cohérent.
async fn creer_les_journees_manquantes(conn: &mut PgConnection, id: EventId) -> Result<i64> {
    let periode = editions::periode_civile(&mut *conn, id).await?;
    let (Some(premier), Some(dernier)) = (periode.first(), periode.last()) else {
        return Ok(0);
    };

    let existantes = days::dates_existantes(&mut *conn, id).await?;
    let plan = calendar::plan(*premier, *dernier, &existantes);

    let a_creer: Vec<(time::Date, i16)> = plan
        .to_create
        .iter()
        .map(|jour| {
            let rang = periode.iter().position(|d| d == jour).unwrap_or(0);
            (*jour, i16::try_from(rang).unwrap_or(i16::MAX))
        })
        .collect();

    let creees = days::creer(conn, id, &a_creer).await?;
    Ok(creees as i64)
}

// -----------------------------------------------------------------------------
// L'annonce
// -----------------------------------------------------------------------------

/// **Aucun déclencheur de `060_events.sql` n'émet d'événement de domaine** : ce
/// qui n'est pas annoncé ici n'est annoncé par personne (`contracts/events.md`).
/// L'émission est dans la **même transaction** que le changement d'état.
async fn annoncer(
    conn: &mut PgConnection,
    id: EventId,
    payload: &EditionFormPayload,
    avant: Option<&editions::EtatAvant>,
) -> Result<()> {
    use contracts::event as contrat;
    use kernel::events::{emit, DomainEvent};

    let (event_type, charge) = match avant {
        None => (
            contrat::EDITION_CREATED,
            json!(contrat::EditionCreated {
                event_id: id.as_uuid(),
                slug: payload.slug.clone(),
                series_id: payload.series_id,
                edition_year: payload.edition_year,
                has_pavilion: payload.has_pavilion,
            }),
        ),
        Some(a) => (
            contrat::EDITION_UPDATED,
            json!(contrat::EditionUpdated {
                event_id: id.as_uuid(),
                period_changed: a.starts_at != payload.starts_at || a.ends_at != payload.ends_at,
                status_changed: a.status != payload.status,
                pavilion_changed: a.has_pavilion != payload.has_pavilion,
                timezone_changed: a.timezone != payload.timezone,
            }),
        ),
    };

    emit(
        conn,
        DomainEvent {
            aggregate_schema: contrat::AGGREGATE_SCHEMA,
            aggregate_type: contrat::AGGREGATE_EDITION,
            aggregate_id: id.as_uuid(),
            event_type,
            payload: charge,
        },
    )
    .await?;

    Ok(())
}

/// Le champ que met en cause une violation de **domaine**.
///
/// Le catalogue du noyau ne peut pas le nommer : le nom de contrainte d'un
/// domaine est celui du domaine — `slug_check` pour `platform.slug` — et ne dit
/// ni la table ni la colonne. Le **nom de type** l'est (`PG_DIAG_DATATYPE_NAME`),
/// et une édition ne porte qu'un seul champ par domaine : la correspondance est
/// donc sans ambiguïté ici, et elle ne le serait pas ailleurs.
///
/// Le fuseau n'y figure pas : un identifiant que la base de fuseaux de
/// PostgreSQL ne connaît pas est une **référence inconnue**, et
/// `EditionFormError` n'a pas de code pour ça — il sort en 422 par le catalogue
/// du noyau.
fn champ_du_domaine(erreur: &sqlx::Error) -> Option<&'static str> {
    match pg_error::violated_domain(erreur)? {
        "slug" => Some("slug"),
        _ => None,
    }
}
