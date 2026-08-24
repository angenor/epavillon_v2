//! S'inscrire, annuler, promouvoir, rejoindre.
//!
//! # Ce service n'émet AUCUN événement
//!
//! `tg_registrations_emit_events()` émet `programme.registration.created` puis
//! `programme.registration.<état>` **dans cette transaction**. Émettre à son tour
//! produirait **deux** courriels de confirmation et **deux** jeux de rappels par
//! inscription, et le doublon ne se verrait qu'en production. La promotion depuis
//! la liste d'attente produit d'elle-même l'avis que la personne attend : le
//! service n'a rien à ajouter (R2, R20).
//!
//! # Quatre fenêtres, quatre motifs — et la base n'en vérifie qu'une et demie
//!
//! Elle refuse une séance annulée et une clôture dépassée ; elle ignore
//! `registration_required` et la date d'ouverture (écart n° 115). Les quatre sont
//! donc décidées ici, sur des valeurs **relues sous verrou**.
//!
//! # Le formulaire est résolu, et la validation est faite AVANT toute écriture
//!
//! Le déclencheur ne vérifie rien lorsque la séance ne porte pas de formulaire
//! **attaché** — le cas courant (écart n° 114). Le service revérifie donc la
//! présence des réponses obligatoires : c'est l'unique entorse au principe VIII
//! de ce jalon, justifiée au « Complexity Tracking » du plan. Et quand la base
//! vérifie, elle rend une phrase française listant des codes, d'où le contrat ne
//! peut pas extraire le champ qu'un formulaire doit souligner.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::pg_error;
use serde::Deserialize;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::ids::{RegistrationId, SessionId};
use crate::domain::registration::{
    self, AnnulationRendue, EtatDInscription, Fenetre, IssueDInscription,
};
use crate::domain::{answers, sessions as formes};
use crate::repo::{consents, forms, people, registrations};
use crate::state::ProgrammeState;

/// Ce qu'une tentative d'inscription porte — `SessionRegisterPayload`.
///
/// **Pas `RegisterPayload`** : ce nom est celui de l'ouverture de compte, servi
/// par le module Identité. Deux formes sans rapport sous un même nom faisaient
/// coexister dans le contrat engendré une inscription à une séance et une
/// création de compte, et le garde-fou de contrat validait l'une contre l'autre.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct SessionRegisterPayload {
    /// Clés = `code` des champs **actifs** du formulaire applicable. Une clé
    /// inconnue est refusée : une réponse mal orthographiée qui disparaît sans
    /// un mot est une réponse perdue.
    #[serde(default)]
    pub answers: serde_json::Value,
    /// Langue des envois ultérieurs ; défaut, la langue négociée de la requête.
    #[serde(default)]
    pub locale: Option<String>,
    /// Identité, **uniquement sans session et si le formulaire admet
    /// l'anonyme**. Jamais prise dans les réponses : les codes de champs sont
    /// renommables depuis le back-office, et le jour où quelqu'un renommerait
    /// `email` en `courriel`, les inscriptions sans compte cesseraient de
    /// rattacher qui que ce soit, en silence (R23).
    #[serde(default)]
    pub guest: Option<Invite>,
    /// Exigé dès qu'une réponse est donnée à un champ marqué sensible.
    #[serde(default)]
    pub sensitive_data_consent: bool,
    /// Organisation déclarée par l'inscrit, quand il y en a une.
    #[serde(default)]
    pub organization_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct Invite {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(default)]
    pub civility: Option<String>,
}

/// S'inscrire à une séance.
///
/// `lecteur` est nul quand la requête n'a pas de session : le formulaire décide
/// alors s'il admet l'inscription sans compte.
pub async fn sinscrire(
    state: &ProgrammeState,
    ctx: &RequestContext,
    session_id: SessionId,
    lecteur: Option<Uuid>,
    ip: Option<std::net::IpAddr>,
    payload: SessionRegisterPayload,
) -> Result<IssueDInscription> {
    let (form_id, admet_anonyme, _) = forms::formulaire_applicable(state.pool(), session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let champs = forms::champs_actifs(state.pool(), form_id).await?;
    let champs = forms::resoudre_pour_validation(state.pool(), champs).await?;

    let reponses = payload.answers.as_object().cloned().unwrap_or_default();

    answers::valider(&champs, &reponses)?;

    // Le consentement se décide sur les champs sensibles **auxquels une réponse
    // est donnée** : refuser d'office ferait cocher une case pour une question
    // laissée vide.
    let sensibles = answers::champs_sensibles_repondus(&champs, &reponses);

    let mut tx = state.db().write(ctx).await?;

    let personne = identifier(&mut tx, lecteur, admet_anonyme, payload.guest.as_ref()).await?;

    if !sensibles.is_empty() {
        exiger_le_consentement(
            &mut tx,
            personne,
            &sensibles,
            payload.sensitive_data_consent,
            state.config().programme.privacy_policy_version.as_str(),
            ip,
        )
        .await?;
    }

    // 🔴 LE VERROU : il précède toute écriture d'inscription, et il tient
    // jusqu'à la fin de la transaction (écart n° 124, R19).
    let seance = registrations::verrouiller(&mut tx, session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    match fenetre_de(&seance) {
        Fenetre::Ouverte => {}
        Fenetre::SeanceAnnulee | Fenetre::SansInscription => {
            return Err(ApiError::new(ErrorCode::RegistrationNotAccepted));
        }
        Fenetre::PasEncoreOuverte(opens_at) => {
            return Ok(IssueDInscription::NotOpenYet { opens_at });
        }
        Fenetre::Close(closed_at) => {
            return Ok(IssueDInscription::Closed { closed_at });
        }
    }

    // **Déjà inscrit n'est pas un refus** : la ligne vivante est relue et rendue,
    // ce que l'écran affiche tel quel.
    if let Some(existante) =
        registrations::inscription_vivante(&mut tx, session_id, personne).await?
    {
        return Ok(IssueDInscription::AlreadyRegistered {
            registration: existante,
        });
    }

    let locale = payload.locale.clone().unwrap_or_else(|| ctx.locale.clone());
    let inscription = registrations::inscrire(
        &mut tx,
        session_id,
        personne,
        payload.organization_id,
        &payload.answers,
        &locale,
    )
    .await;

    let issue = match inscription {
        Ok(ligne) => issue_de(ligne),
        Err(erreur) => {
            let issue = traduire_linscription(&erreur, &seance)?;
            tx.rollback().await?;
            return Ok(issue);
        }
    };

    tx.commit().await?;
    Ok(issue)
}

/// La fenêtre, telle que le domaine la décide, sur des valeurs **relues sous
/// verrou**.
fn fenetre_de(seance: &registrations::SeanceVerrouillee) -> Fenetre {
    registration::fenetre(
        EtatDInscription {
            annulee: seance.status == "cancelled",
            inscription_requise: seance.registration_required,
            ouvre_le: seance.registration_opens_at,
            ferme_le: seance.registration_closes_at,
        },
        OffsetDateTime::now_utc(),
    )
}

/// **L'état obtenu est celui que la base a posé.** La bascule automatique en
/// liste d'attente est laissée faire ; le service lit ce qui en est sorti, et la
/// position vient de la ligne, jamais d'un calcul.
fn issue_de(ligne: serde_json::Value) -> IssueDInscription {
    let statut = ligne
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("registered");

    if statut == "waitlisted" {
        let position = ligne
            .get("waitlist_position")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_default() as i32;

        return IssueDInscription::Waitlisted {
            registration: ligne,
            position,
        };
    }

    IssueDInscription::Registered {
        registration: ligne,
    }
}

/// Qui s'inscrit : la personne connectée, ou l'invité que le formulaire admet.
///
/// **L'identité vient de champs dédiés**, jamais des réponses (R23), et la
/// création réutilise la porte bornée de B4 : adresse, prénom, nom, civilité — ni
/// compte, ni mot de passe, ni rôle.
async fn identifier(
    conn: &mut PgConnection,
    lecteur: Option<Uuid>,
    admet_anonyme: bool,
    invite: Option<&Invite>,
) -> Result<Uuid> {
    if let Some(personne) = lecteur {
        return Ok(personne);
    }

    if !admet_anonyme {
        return Err(ApiError::new(ErrorCode::RegistrationAccountRequired));
    }

    let invite = invite.ok_or_else(|| {
        ApiError::validation(
            "Sans compte, votre adresse, votre prénom et votre nom sont nécessaires.",
            "guest",
        )
    })?;

    people::trouver_ou_creer(
        conn,
        people::IdentiteSaisie {
            email: &invite.email,
            first_name: &invite.first_name,
            last_name: &invite.last_name,
            civility: invite.civility.as_deref(),
        },
    )
    .await
}

/// Le consentement aux réponses sensibles — **refus nommant le champ**, puis
/// preuve écrite dans la transaction de la donnée qu'elle couvre (R22).
async fn exiger_le_consentement(
    conn: &mut PgConnection,
    personne: Uuid,
    sensibles: &[&str],
    accorde_maintenant: bool,
    policy_version: &str,
    ip: Option<std::net::IpAddr>,
) -> Result<()> {
    if accorde_maintenant {
        consents::accorder(conn, personne, policy_version, ip).await?;
        return Ok(());
    }

    // Un accord déjà donné vaut : redemander à chaque inscription ferait cocher
    // la même case dix fois pour la même personne.
    if consents::accorde(conn, personne).await? {
        return Ok(());
    }

    Err(ApiError::new(ErrorCode::RegistrationConsentRequired)
        .field(sensibles.first().copied().unwrap_or("answers")))
}

/// Traduire le refus de PostgreSQL **par le geste et par le SQLSTATE, jamais par
/// le texte**.
///
/// La valeur qui accompagne un refus est **relue sur la séance**, jamais extraite
/// du message : le déclencheur écrit « Capacité atteinte (30 places). », et
/// extraire un nombre d'une phrase française est un piège que B3 a déjà nommé.
fn traduire_linscription(
    erreur: &sqlx::Error,
    seance: &registrations::SeanceVerrouillee,
) -> Result<IssueDInscription> {
    match pg_error::sqlstate(erreur).as_deref() {
        // 23001 — trois refus du déclencheur, distingués par ce que la séance
        // porte : elle est annulée, ou close, ou pleine.
        Some("23001") => {
            if seance.status == "cancelled" {
                return Err(ApiError::new(ErrorCode::RegistrationNotAccepted));
            }
            if let Some(closed_at) = seance.registration_closes_at {
                if closed_at < OffsetDateTime::now_utc() {
                    return Ok(IssueDInscription::Closed { closed_at });
                }
            }
            match seance.capacity {
                Some(capacity) => Ok(IssueDInscription::Full { capacity }),
                None => Err(pg_error::translate(erreur)),
            }
        }
        // 23502 — réponses obligatoires manquantes. Un filet : le service a déjà
        // refusé, en nommant le champ.
        Some("23502") => Err(ApiError::new(ErrorCode::RegistrationAnswerInvalid)),
        _ => Err(pg_error::translate(erreur)),
    }
}

// -----------------------------------------------------------------------------
// L'annulation, et la promotion qui l'accompagne
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AnnulationDemandee {
    #[serde(default)]
    pub reason: Option<String>,
}

/// Annuler une inscription, **et promouvoir exactement le nombre de places
/// libérées** — dans la même transaction, sous le même verrou (R20, écart
/// n° 116).
///
/// Annuler une inscription **en attente** ne promeut personne : elle n'occupait
/// aucune place.
pub async fn annuler(
    state: &ProgrammeState,
    ctx: &RequestContext,
    registration_id: RegistrationId,
    session_id: SessionId,
    motif: Option<&str>,
) -> Result<AnnulationRendue> {
    let mut tx = state.db().write(ctx).await?;

    // Le verrou d'abord : la promotion qui suit lit et écrit les mêmes lignes
    // que le contrôle de jauge d'une inscription concurrente.
    registrations::verrouiller(&mut tx, session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    let annulee = registrations::annuler(&mut tx, registration_id, motif)
        .await
        .map_err(|e| traduire_lannulation(&e))?;

    let (registration, precedent) = match annulee {
        Some(couple) => couple,
        // Déjà annulée : rejouer l'annulation ne libère aucune place, et
        // promouvoir sur cette base ferait dépasser la jauge.
        None => {
            let ligne =
                registrations::inscription_vivante(&mut tx, session_id, Uuid::nil()).await?;
            tx.rollback().await?;
            return Err(ligne
                .map(|_| ApiError::new(ErrorCode::RegistrationLocked))
                .unwrap_or_else(ApiError::not_found));
        }
    };

    // Seule une inscription **confirmée** libérait une place.
    let promoted = if precedent == "registered" || precedent == "attended" {
        registrations::promouvoir(&mut tx, session_id.as_uuid(), 1).await?
    } else {
        0
    };

    tx.commit().await?;

    Ok(AnnulationRendue {
        registration,
        promoted,
    })
}

/// Le déclencheur revalide à **chaque** changement d'état, et deux de ses
/// contrôles ne sont pas bornés à l'insertion : on ne peut pas annuler son
/// inscription à une séance annulée, ni une inscription ancienne à laquelle une
/// question devenue obligatoire manque (écart n° 125).
///
/// **Consigné, non contourné** : il n'y a pas de contournement sans modifier le
/// déclencheur. Le service traduit en refus nommé plutôt que de laisser sortir un
/// 500 — le premier symptôme est sans conséquence pratique, une séance annulée ne
/// réunit personne.
fn traduire_lannulation(erreur: &sqlx::Error) -> ApiError {
    match pg_error::sqlstate(erreur).as_deref() {
        Some("23001") | Some("23502") => ApiError::new(ErrorCode::RegistrationLocked),
        _ => pg_error::translate(erreur),
    }
}

/// La **première présence**, écrite une seule fois par la fonction du modèle.
pub async fn rejoindre(
    state: &ProgrammeState,
    ctx: &RequestContext,
    registration_id: RegistrationId,
) -> Result<serde_json::Value> {
    let mut tx = state.db().write(ctx).await?;
    let instant = registrations::rejoindre(&mut tx, registration_id).await?;
    tx.commit().await?;

    let instant = instant.ok_or_else(ApiError::not_found)?;

    Ok(serde_json::json!({
        "joined_at": instant
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(ApiError::internal)?
    }))
}

// -----------------------------------------------------------------------------
// Les lectures
// -----------------------------------------------------------------------------

/// Le formulaire applicable et ses champs actifs, options résolues.
pub async fn formulaire(
    state: &ProgrammeState,
    session_id: SessionId,
) -> Result<forms::FormulaireApplicable> {
    let (form_id, _, form) = forms::formulaire_applicable(state.pool(), session_id)
        .await?
        .ok_or_else(ApiError::not_found)?;

    Ok(forms::FormulaireApplicable {
        form,
        fields: forms::champs_affichables(state.pool(), form_id).await?,
    })
}

/// La liste **nominative** des inscrits d'une séance.
pub async fn liste_nominative(
    state: &ProgrammeState,
    session_id: SessionId,
) -> Result<Vec<serde_json::Value>> {
    registrations::liste_nominative(state.pool(), session_id).await
}

/// « Mes inscriptions » — celles de la personne connectée, et personne d'autre.
pub async fn mes_inscriptions(
    state: &ProgrammeState,
    person_id: Uuid,
) -> Result<Vec<serde_json::Value>> {
    registrations::mes_inscriptions(state.pool(), person_id).await
}

/// Les trois nombres d'une séance, pour l'organisation qui la porte —
/// `TrackedSession`.
pub async fn decomptes(state: &ProgrammeState, session_id: SessionId) -> Result<(i64, i64)> {
    registrations::decomptes(state.pool(), session_id).await
}

/// Rappel de ce que ce module ne rend jamais à une organisation : un nom.
pub use formes::TrackedSession;
