//! S'inscrire à une journée, changer ses activités, se désinscrire (02/11).
//!
//! # Deux façons d'être reconnu
//!
//! La personne connectée, ou le jeton du lien reçu par courriel quand on s'est
//! inscrit sans compte (en-tête `X-Manage-Token`, jamais dans l'adresse : une
//! adresse se retrouve dans les journaux). La base ne garde que son empreinte.
//!
//! # Ce service n'émet aucun événement
//!
//! `tg_track_registrations_emit_events()` et `tg_registrations_emit_events()`
//! le font dans la transaction : réunion et place Zoom de la journée, rappels
//! des activités gardées. Seul le courriel de confirmation est mis en file ici,
//! parce qu'il porte le jeton en clair.

use kernel::auth::{Perimeter, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::jobs::{self, NewJob};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::answers;
use crate::domain::ids::EventId;
use crate::domain::permissions::REGISTRATION_MANAGE;
use crate::domain::track_registration::{
    sans_doublons, DayRegisterPayload, DayRegistration, DayRegistrationManage,
    DayRegistrationResult, DayRegistrationRow,
};
use crate::repo::{forms, track_registrations as repo};
use crate::service::perimeter::{self, Cible};
use crate::service::registration::{exiger_le_consentement, identifier};
use crate::state::ProgrammeState;

/// L'en-tête qui porte le jeton du lien de gestion.
pub const EN_TETE_JETON: &str = "x-manage-token";

/// Qui demande : la personne connectée, ou le porteur du lien de gestion.
pub enum Demandeur {
    Personne(Uuid),
    Jeton(String),
}

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Un jeton neuf et son empreinte (SHA-256, 32 octets).
pub(crate) fn jeton_neuf() -> (String, [u8; 32]) {
    let jeton = kernel::crypto::random_string(ALPHABET, 43);
    let empreinte = kernel::crypto::token_hash(&jeton);
    (jeton, empreinte)
}

/// Une journée publiée qui prend des inscriptions ; 404 sinon, comme sa page.
async fn journee_ouverte(state: &ProgrammeState, track_id: Uuid) -> Result<repo::Journee> {
    repo::journee(state.pool(), track_id)
        .await?
        .filter(|j| j.is_day && j.published)
        .ok_or_else(ApiError::not_found)
}

/// Le formulaire de la journée : celui de son édition, à défaut de la plateforme.
pub async fn formulaire(
    state: &ProgrammeState,
    track_id: Uuid,
) -> Result<forms::FormulaireApplicable> {
    let journee = journee_ouverte(state, track_id).await?;
    let (form_id, _, form) = forms::formulaire_de_ledition(state.pool(), journee.event_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    Ok(forms::FormulaireApplicable {
        form,
        fields: forms::champs_affichables(state.pool(), form_id).await?,
    })
}

/// Les activités gardées doivent être des activités ouvertes de la journée.
async fn verifier_les_activites(
    state: &ProgrammeState,
    track_id: Uuid,
    demandees: &[Uuid],
) -> Result<Vec<Uuid>> {
    let gardees = sans_doublons(demandees);
    let ouvertes = repo::activites(state.pool(), track_id).await?;
    if gardees.iter().any(|id| !ouvertes.contains(id)) {
        return Err(ApiError::validation(
            "Une des activités choisies ne fait pas partie de cette journée, ou n'a plus lieu.",
            "session_ids",
        ));
    }
    Ok(gardees)
}

async fn mettre_en_file_le_courriel(
    conn: &mut PgConnection,
    track_registration_id: Uuid,
    jeton: Option<String>,
    renvoi: bool,
) -> Result<()> {
    let charge = contracts::engagement::DayRegistrationEmail {
        track_registration_id,
        token: jeton,
        resent: renvoi,
    };
    let mut job = NewJob::new(
        contracts::engagement::SEND_DAY_REGISTRATION,
        serde_json::to_value(&charge).map_err(ApiError::internal)?,
    )
    .idempotent(format!(
        "{track_registration_id}:{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    job.queue = contracts::engagement::EMAIL_QUEUE;
    jobs::enqueue(conn, job).await?;
    Ok(())
}

async fn etat(conn: &mut PgConnection, id: Uuid) -> Result<DayRegistration> {
    repo::etat(&mut *conn, id)
        .await?
        .ok_or_else(|| ApiError::internal("inscription à la journée introuvable après écriture"))
}

/// S'inscrire à une journée. `lecteur` nul : sans compte, si le formulaire de
/// l'édition l'admet.
pub async fn sinscrire(
    state: &ProgrammeState,
    ctx: &RequestContext,
    lecteur: Option<Uuid>,
    ip: Option<std::net::IpAddr>,
    payload: DayRegisterPayload,
) -> Result<DayRegistrationResult> {
    let journee = journee_ouverte(state, payload.track_id).await?;
    if !journee.is_streamed {
        return Err(ApiError::with_message(
            ErrorCode::RegistrationNotAccepted,
            "Cette journée se tient exclusivement en présentiel : elle ne se suit pas en ligne.",
        ));
    }
    if let Some(fin) = journee.ends_at {
        if OffsetDateTime::now_utc() > fin {
            return Ok(DayRegistrationResult::Closed { closed_at: fin });
        }
    }

    let (form_id, admet_anonyme, _) = forms::formulaire_de_ledition(state.pool(), journee.event_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let champs = forms::champs_actifs(state.pool(), form_id).await?;
    let champs = forms::resoudre_pour_validation(state.pool(), champs).await?;
    let reponses = payload.answers.as_object().cloned().unwrap_or_default();
    answers::valider(&champs, &reponses)?;
    let sensibles = answers::champs_sensibles_repondus(&champs, &reponses);

    let gardees = verifier_les_activites(state, journee.id, &payload.session_ids).await?;

    let mut tx = state.db().write(ctx).await?;
    let personne = identifier(&mut tx, lecteur, admet_anonyme, payload.guest.as_ref()).await?;
    if !sensibles.is_empty() {
        exiger_le_consentement(
            &mut tx,
            personne,
            &sensibles,
            payload.sensitive_data_consent,
            kernel::legal::version("privacy"),
            ip,
        )
        .await?;
    }

    let sans_compte = lecteur.is_none();
    let (jeton, empreinte) = if sans_compte {
        let (j, e) = jeton_neuf();
        (Some(j), Some(e))
    } else {
        (None, None)
    };

    let locale = payload.locale.clone().unwrap_or_else(|| ctx.locale.clone());
    let reponses = serde_json::Value::Object(reponses);
    let nouvelle = match repo::vivante(&mut tx, journee.id, personne).await? {
        Some(_) => None,
        None => {
            repo::inserer(
                &mut tx,
                &repo::Nouvelle {
                    track_id: journee.id,
                    person_id: personne,
                    organization_id: payload.organization_id,
                    answers: &reponses,
                    locale: &locale,
                    manage_token_hash: empreinte.as_ref().map(|e| e.as_slice()),
                },
            )
            .await?
        }
    };

    let Some(id) = nouvelle else {
        // Déjà inscrite. Sans compte, rien n'est rendu : le lien repart, neuf.
        let existante = repo::vivante(&mut tx, journee.id, personne)
            .await?
            .ok_or_else(|| ApiError::internal("inscription à la journée disparue"))?;
        if let (Some(jeton), Some(empreinte)) = (jeton, empreinte) {
            repo::poser_empreinte(&mut tx, existante, &empreinte).await?;
            mettre_en_file_le_courriel(&mut tx, existante, Some(jeton), true).await?;
            tx.commit().await?;
            return Ok(DayRegistrationResult::AlreadyRegistered { registration: None });
        }
        let registration = etat(&mut tx, existante).await?;
        tx.commit().await?;
        return Ok(DayRegistrationResult::AlreadyRegistered {
            registration: Some(registration),
        });
    };

    repo::garder(&mut tx, id, &gardees).await?;
    mettre_en_file_le_courriel(&mut tx, id, jeton, false).await?;
    let registration = etat(&mut tx, id).await?;
    tx.commit().await?;
    Ok(DayRegistrationResult::Registered { registration })
}

/// L'inscription vivante de la personne connectée à une journée ; 404 sinon.
pub async fn la_mienne(
    state: &ProgrammeState,
    track_id: Uuid,
    personne: Uuid,
) -> Result<DayRegistration> {
    let mut conn = state.pool().acquire().await?;
    let id = repo::vivante(&mut conn, track_id, personne)
        .await?
        .ok_or_else(ApiError::not_found)?;
    etat(&mut conn, id).await
}

/// L'inscription du porteur du lien de gestion ; 404 pour un jeton inconnu.
pub async fn gestion(state: &ProgrammeState, jeton: &str) -> Result<DayRegistrationManage> {
    let proprietaire = repo::par_empreinte(state.pool(), &kernel::crypto::token_hash(jeton))
        .await?
        .ok_or_else(ApiError::not_found)?;
    let journee = repo::journee(state.pool(), proprietaire.track_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let mut conn = state.pool().acquire().await?;
    Ok(DayRegistrationManage {
        registration: etat(&mut conn, proprietaire.id).await?,
        event_slug: journee.event_slug,
        track_slug: journee.slug,
        track_title: journee.title,
        first_name: repo::prenom(&mut *conn, proprietaire.person_id).await?,
    })
}

/// L'inscription visée, si le demandeur y a droit ; 404 sinon, pour ne rien
/// dire d'une inscription d'autrui.
async fn autoriser(
    state: &ProgrammeState,
    id: Uuid,
    demandeur: &Demandeur,
) -> Result<repo::Proprietaire> {
    let proprietaire = repo::proprietaire(state.pool(), id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let permis = match demandeur {
        Demandeur::Personne(p) => proprietaire.person_id == *p,
        Demandeur::Jeton(j) => repo::par_empreinte(state.pool(), &kernel::crypto::token_hash(j))
            .await?
            .is_some_and(|t| t.id == id),
    };
    if !permis {
        return Err(ApiError::not_found());
    }
    Ok(proprietaire)
}

/// Changer ses activités gardées. Elles ne changent rien au direct.
pub async fn modifier(
    state: &ProgrammeState,
    ctx: &RequestContext,
    id: Uuid,
    demandeur: Demandeur,
    session_ids: &[Uuid],
) -> Result<DayRegistration> {
    let proprietaire = autoriser(state, id, &demandeur).await?;
    if proprietaire.status == "cancelled" {
        return Err(ApiError::new(ErrorCode::RegistrationLocked));
    }
    journee_ouverte(state, proprietaire.track_id).await?;
    let gardees = verifier_les_activites(state, proprietaire.track_id, session_ids).await?;

    let mut tx = state.db().write(ctx).await?;
    repo::garder(&mut tx, id, &gardees).await?;
    let registration = etat(&mut tx, id).await?;
    tx.commit().await?;
    Ok(registration)
}

/// Se désinscrire de la journée : ses activités gardées tombent avec elle.
pub async fn annuler(
    state: &ProgrammeState,
    ctx: &RequestContext,
    id: Uuid,
    demandeur: Demandeur,
) -> Result<DayRegistration> {
    autoriser(state, id, &demandeur).await?;
    let mut tx = state.db().write(ctx).await?;
    repo::annuler(&mut tx, id).await?;
    let registration = etat(&mut tx, id).await?;
    tx.commit().await?;
    Ok(registration)
}

/// La liste nominative d'une journée : `programme.registration.manage` sur son
/// édition, dans le périmètre d'administration.
pub async fn liste_nominative(
    state: &ProgrammeState,
    perimetre: &Perimeter,
    track_id: Uuid,
) -> Result<Vec<DayRegistrationRow>> {
    let journee = repo::journee(state.pool(), track_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    let event_id = perimeter::edition_dans_le_perimetre(
        state.pool(),
        perimetre,
        Cible::Edition(EventId(journee.event_id)),
    )
    .await?;
    kernel::auth::require_permission(
        state.pool(),
        perimetre.person_id,
        REGISTRATION_MANAGE,
        Scope::Event(event_id.as_uuid()),
    )
    .await?;
    repo::liste_nominative(state.pool(), track_id).await
}
