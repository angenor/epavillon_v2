//! Ouvrir l'appel à propositions, et sa grille avec lui.
//!
//! **C'est ce que tout le jalon attend** : sans édition pas d'appel, sans appel
//! pas de dossier, sans grille pas d'évaluation.
//!
//! Quatre choses se jouent ici.
//!
//! 1. **L'appel et sa grille en une seule transaction** (research.md § R9). Un
//!    échec sur la grille ne laisse aucun appel derrière lui : laisser exister
//!    un appel sans critère, fût-ce le temps d'un oubli, c'est laisser ouvrir
//!    une campagne que personne ne pourra évaluer.
//! 2. **Le diff de grille se fait par code**, et le retrait d'un critère
//!    **porteur de notes est refusé**. C'est l'unique entorse au principe VIII
//!    de tout le module, et elle est assumée :
//!    `xmod_fk_review_scores_criterion` est `ON DELETE CASCADE`, la base
//!    détruirait les notes sans erreur, sans trace et sans que l'écran puisse le
//!    voir. Or ces notes sont l'argumentaire d'une décision de sélection —
//!    précisément ce que la v1 n'avait pas, et qui rendait un refus inexplicable
//!    à l'organisation qui le contestait.
//! 3. **La traduction des refus de la base** — six contraintes nommées, chacune
//!    sur son champ, en 200 : le contrat du front les exprime.
//! 4. **Les annonces** — ouverture, clôture, prolongation. Aucun déclencheur du
//!    modèle n'émet d'événement de domaine : ce qui n'est pas annoncé ici n'est
//!    annoncé par personne.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::pg_error;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::call::{
    self, CallErrorCode, CallFormError, CallSaveResult, CritereExistant, EditionCallPayload,
};
use crate::domain::ids::{CallId, EventId};
use crate::repo::{calls, criteria, cross};
use crate::state::EventState;

/// Ouvrir un appel sur une édition. L'édition vient du **corps vérifié** : la
/// route a déjà remonté son périmètre avant d'arriver ici.
pub async fn creer(
    state: &EventState,
    ctx: &RequestContext,
    acteur: Uuid,
    event_id: EventId,
    payload: EditionCallPayload,
) -> Result<CallSaveResult> {
    enregistrer(state, ctx, acteur, event_id, None, payload).await
}

/// Modifier un appel. L'édition vient de **l'ascendance de l'appel**, jamais du
/// corps.
pub async fn modifier(
    state: &EventState,
    ctx: &RequestContext,
    acteur: Uuid,
    event_id: EventId,
    call_id: CallId,
    payload: EditionCallPayload,
) -> Result<CallSaveResult> {
    enregistrer(state, ctx, acteur, event_id, Some(call_id), payload).await
}

async fn enregistrer(
    state: &EventState,
    ctx: &RequestContext,
    acteur: Uuid,
    event_id: EventId,
    existant: Option<CallId>,
    payload: EditionCallPayload,
) -> Result<CallSaveResult> {
    if let Some(refus) = refus_de_grille(&payload) {
        return Ok(refus);
    }

    let mut tx = state.db().write(ctx).await?;

    let avant = match existant {
        Some(id) => calls::etat_avant(&mut tx, id).await?,
        None => None,
    };

    // **Le diff AVANT l'écriture de l'appel**, et le refus avec lui : un critère
    // porteur de notes doit interdire l'enregistrement entier, pas seulement sa
    // propre suppression.
    let diff = match existant {
        Some(id) => {
            let existants = grille_existante(&mut tx, id).await?;
            call::diff(&payload.criteria, &existants)
        }
        None => call::diff(&payload.criteria, &[]),
    };

    if let Some(porteur) = diff.a_supprimer.iter().find(|c| c.score_count > 0) {
        return Err(refus_de_critere_note(porteur));
    }

    let call_id = match existant {
        None => match calls::inserer(&mut tx, event_id, &payload, acteur).await {
            Ok(id) => id,
            Err(e) => return refus_de_base(e, &payload),
        },
        Some(id) => match calls::modifier(&mut tx, id, &payload).await {
            // L'appel a disparu entre le contrôle de périmètre et l'écriture.
            Ok(false) => return Err(ApiError::not_found()),
            Ok(true) => id,
            Err(e) => return refus_de_base(e, &payload),
        },
    };

    if let Err(e) = ecrire_la_grille(&mut tx, call_id, &payload, &diff).await {
        return refus_de_base(e, &payload);
    }

    annoncer(&mut tx, call_id, event_id, &payload, avant.as_ref()).await?;

    tx.commit().await?;

    let call = super::detail::appel_par_id(state.pool(), call_id).await?;

    Ok(CallSaveResult {
        ok: true,
        call,
        errors: Vec::new(),
        scores_affected: diff.scores_affected,
    })
}

/// La grille existante, **notes comprises** — les critères viennent du schéma du
/// module, leur décompte de `programme` par le dépôt de la frontière.
async fn grille_existante(
    conn: &mut PgConnection,
    call_id: CallId,
) -> Result<Vec<CritereExistant>> {
    let mut existants = criteria::existants(&mut *conn, call_id).await?;
    let notes = cross::notes_par_critere(&mut *conn, call_id).await?;

    for critere in &mut existants {
        critere.score_count = notes.get(&critere.id).copied().unwrap_or(0);
    }

    Ok(existants)
}

/// L'ordre compte : **supprimer, puis modifier, puis insérer**.
///
/// Un code libéré par une suppression peut être repris par une insertion dans la
/// même charge utile — un critère renommé, par exemple. Insérer d'abord
/// violerait `ux_review_criteria` sur un conflit qui n'existe déjà plus.
async fn ecrire_la_grille(
    conn: &mut PgConnection,
    call_id: CallId,
    payload: &EditionCallPayload,
    diff: &call::DiffGrille,
) -> std::result::Result<(), sqlx::Error> {
    for critere in &diff.a_supprimer {
        criteria::supprimer(&mut *conn, critere.id)
            .await
            .map_err(|_| sqlx::Error::RowNotFound)?;
    }

    for (id, rang) in &diff.a_modifier {
        criteria::modifier(&mut *conn, *id, &payload.criteria[*rang]).await?;
    }

    for rang in &diff.a_inserer {
        criteria::inserer(&mut *conn, call_id, &payload.criteria[*rang]).await?;
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Les refus que le service ajoute au modèle
// -----------------------------------------------------------------------------

/// Les deux règles de grille que la base ne porte pas.
///
/// **Une grille vide** n'est refusée par aucune contrainte : `review_criteria`
/// n'a pas de cardinalité minimale, et elle ne doit pas en avoir — un appel
/// annulé peut légitimement finir sans critère. C'est l'*ouverture* d'une
/// campagne sans grille qui n'a pas de sens.
///
/// **Deux codes identiques** seraient refusés par `ux_review_criteria`, mais
/// l'index ne dit pas *quelle ligne de l'écran* est en cause. Le contrat du
/// front attend ce rang.
fn refus_de_grille(payload: &EditionCallPayload) -> Option<CallSaveResult> {
    if payload.criteria.is_empty() {
        return Some(CallSaveResult::refuse(vec![CallFormError::globale(
            CallErrorCode::CriteriaEmpty,
        )]));
    }

    call::premier_code_en_double(&payload.criteria).map(|rang| {
        CallSaveResult::refuse(vec![CallFormError::ligne(
            CallErrorCode::CriterionCodeDuplicate,
            rang,
        )])
    })
}

/// **Le refus qui sauve les notes** (research.md § R9, écart n° 91).
///
/// Il sort en 422 et non en 200 : `CallErrorCode` n'a aucune variante pour
/// l'exprimer, et en inventer une serait renégocier le contrat du front. La
/// dette est inscrite pour B7.
///
/// Le message **nomme le critère et compte ses notes** : « ce critère porte des
/// notes » sans dire lequel oblige à ouvrir la base pour savoir quoi retirer de
/// la charge utile.
fn refus_de_critere_note(critere: &CritereExistant) -> ApiError {
    ApiError::with_message(
        ErrorCode::EventCriterionHasScores,
        format!(
            "Le critère « {} » porte déjà {} note(s) : le retirer effacerait l'argumentaire des évaluations rendues.",
            critere.nom(),
            critere.score_count
        ),
    )
    .field("criteria")
}

// -----------------------------------------------------------------------------
// La traduction des refus de la base
// -----------------------------------------------------------------------------

fn refus_de_base(erreur: sqlx::Error, payload: &EditionCallPayload) -> Result<CallSaveResult> {
    match traduire(&erreur, payload) {
        Some(errors) => Ok(CallSaveResult::refuse(errors)),
        None => Err(pg_error::translate(&erreur)),
    }
}

/// Les **six** contraintes nommées d'un appel, plus la grille et les champs
/// obligatoires.
///
/// **On branche sur le NOM de la contrainte, jamais sur le texte du message** :
/// le texte est localisé par PostgreSQL et se reformule d'une version à l'autre.
fn traduire(erreur: &sqlx::Error, payload: &EditionCallPayload) -> Option<Vec<CallFormError>> {
    use CallErrorCode::*;

    let erreur_de_ligne = |code, champ: &str| Some(vec![CallFormError::champ(code, champ)]);

    match pg_error::constraint(erreur) {
        Some("ck_calls_window") => erreur_de_ligne(Window, "closes_at"),
        Some("ck_calls_extension") => erreur_de_ligne(Extension, "extended_until"),
        Some("ck_calls_speakers") => erreur_de_ligne(Speakers, "max_speakers"),
        Some("ck_calls_duration_bounds") => {
            erreur_de_ligne(DurationBounds, champ_de_duree(payload))
        }
        Some("ck_calls_daily_window") => erreur_de_ligne(DailyWindow, "daily_end_time"),
        Some("ux_calls_one_per_event") => Some(vec![CallFormError::globale(AlreadyExists)]),
        Some("ux_calls_code") => erreur_de_ligne(CodeTaken, "code"),
        // Le service dédoublonne avant d'écrire : voir cet index remonter veut
        // dire que deux lignes de la charge utile portaient le même code sans
        // que le contrôle les ait vues. On désigne malgré tout un rang.
        Some("ux_review_criteria") => Some(vec![CallFormError::ligne(
            CriterionCodeDuplicate,
            call::premier_code_en_double(&payload.criteria).unwrap_or(0),
        )]),
        Some("calls_for_proposals_code_check") => erreur_de_ligne(Required, "code"),
        Some("review_criteria_code_check") => Some(vec![CallFormError::ligne(Required, 0)]),
        // **Une violation de DOMAINE ne nomme pas sa colonne** : le nom de
        // contrainte y est celui du domaine. Le nom de type, lui, est fiable.
        _ => erreur_de_ligne(Required, champ_du_domaine(erreur, payload)?),
    }
}

/// `ck_calls_duration_bounds` porte **trois conditions sous un seul nom**. On
/// compare les trois valeurs pour désigner la plus probablement fautive — la
/// durée par défaut hors bornes d'abord, c'est le cas courant — **sans
/// réimplémenter la contrainte** : c'est la base qui a refusé, on ne fait que
/// nommer le champ à marquer.
fn champ_de_duree(p: &EditionCallPayload) -> &'static str {
    if p.default_duration_minutes < p.min_duration_minutes
        || p.default_duration_minutes > p.max_duration_minutes
    {
        "default_duration_minutes"
    } else if !(15..=600).contains(&p.min_duration_minutes) {
        "min_duration_minutes"
    } else {
        "max_duration_minutes"
    }
}

/// Le champ que met en cause une violation de **domaine**.
///
/// Un appel porte **deux** champs de domaine `platform.url` — l'adresse des
/// consignes est la seule que le formulaire envoie —, un champ `platform.slug`
/// nul part, et aucun fuseau. La correspondance est donc sans ambiguïté ici, et
/// elle ne le serait pas ailleurs : c'est pour cela qu'elle vit dans le service
/// et non dans le noyau.
fn champ_du_domaine(erreur: &sqlx::Error, p: &EditionCallPayload) -> Option<&'static str> {
    match pg_error::violated_domain(erreur)? {
        "url" if p.guidelines_url.is_some() => Some("guidelines_url"),
        _ => None,
    }
}

// -----------------------------------------------------------------------------
// Les annonces
// -----------------------------------------------------------------------------

/// **Trois annonces, et pas une de plus** — l'ouverture, la clôture, la
/// prolongation. Elles sont émises dans la **même transaction** que le
/// changement d'état.
///
/// Ce qui n'est pas ici n'est annoncé par personne : aucun déclencheur de
/// `060_events.sql` n'émet d'événement de domaine ([`contracts/events.md`]).
async fn annoncer(
    conn: &mut PgConnection,
    call_id: CallId,
    event_id: EventId,
    payload: &EditionCallPayload,
    avant: Option<&calls::EtatAvant>,
) -> Result<()> {
    use contracts::event as contrat;
    use kernel::events::{emit, DomainEvent};

    let echeance = calls::echeance_effective(&mut *conn, call_id).await?;
    let statut_avant = avant.map(|a| a.status.as_str());

    let mut annonces: Vec<(&'static str, serde_json::Value)> = Vec::new();

    if payload.status == "open" && statut_avant != Some("open") {
        annonces.push((
            contrat::CALL_OPENED,
            charge(contrat::CallOpened {
                call_id: call_id.as_uuid(),
                event_id: event_id.as_uuid(),
                effective_deadline: echeance,
            })?,
        ));
    }

    if payload.status == "closed" && statut_avant != Some("closed") {
        annonces.push((
            contrat::CALL_CLOSED,
            charge(contrat::CallClosed {
                call_id: call_id.as_uuid(),
                event_id: event_id.as_uuid(),
                applied_deadline: echeance,
            })?,
        ));
    }

    // **L'échéance initiale voyage avec la nouvelle.** C'est celle qui a été
    // annoncée aux organisations, et un rappel qui l'ignore dit une
    // contre-vérité. À la création, l'échéance initiale est la clôture même :
    // il n'y a pas d'état antérieur à lire.
    if let Some(nouvelle) = payload.extended_until {
        let initiale = avant.map(calls::EtatAvant::echeance);
        let deplacee = initiale.is_none_or(|precedente| precedente != nouvelle);
        if deplacee {
            annonces.push((
                contrat::CALL_DEADLINE_EXTENDED,
                charge(contrat::CallDeadlineExtended {
                    call_id: call_id.as_uuid(),
                    event_id: event_id.as_uuid(),
                    initial_deadline: initiale.unwrap_or(payload.closes_at),
                    new_deadline: nouvelle,
                })?,
            ));
        }
    }

    for (event_type, payload) in annonces {
        emit(
            &mut *conn,
            DomainEvent {
                aggregate_schema: contrat::AGGREGATE_SCHEMA,
                aggregate_type: contrat::AGGREGATE_CALL,
                aggregate_id: call_id.as_uuid(),
                event_type,
                payload,
            },
        )
        .await?;
    }

    Ok(())
}

fn charge<T: serde::Serialize>(valeur: T) -> Result<serde_json::Value> {
    serde_json::to_value(valeur).map_err(ApiError::internal)
}
