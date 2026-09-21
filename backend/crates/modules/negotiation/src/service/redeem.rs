//! Saisir un code d'invitation — les neuf issues, et une seule transaction.
//!
//! # L'ORDRE DES CONTRÔLES N'EST PAS ARBITRAIRE
//!
//! **La limite d'essais passe avant tout le reste.** Une fois le seuil franchi,
//! la réponse ne doit plus rien apprendre du code essayé : ni qu'il existe, ni
//! qu'il est révoqué, ni qu'il est épuisé (FR-017). Chercher le code d'abord
//! pour « mieux » répondre ferait du formulaire un oracle qui distingue les
//! codes justes des faux, à raison d'un bit par essai.
//!
//! **L'accès déjà détenu passe avant le mode d'admission.** Une personne admise
//! qui saisit le code du réseau gagne son appartenance, sans qu'aucun
//! administrateur n'ait à trancher quoi que ce soit (FR-018) : lui ouvrir une
//! demande en mode « les deux » serait lui demander d'attendre pour un droit
//! qu'elle a déjà.
//!
//! # CE QUE LA BASE TIENT, ET QUE CE FICHIER NE REFAIT PAS
//!
//! Le quota, l'unicité de l'usage, l'unicité de la demande en attente, la
//! cohérence de la portée, l'appartenance du terme à sa taxonomie. Aucun
//! `SELECT` préalable : deux requêtes simultanées le contourneraient, et c'est
//! exactement le cas que FR-038 bis exige de rendre **impossible** plutôt
//! qu'improbable.

use contracts::negotiation as evenements;
use kernel::auth::{self, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use kernel::events::{self, DomainEvent};
use uuid::Uuid;

use crate::domain::admission::AdmissionMode;
use crate::domain::code;
use crate::domain::permissions::SPACE_ACCESS;
use crate::domain::redeem::{RedeemIssue, RedeemResult};
use crate::repo::codes::{CodeTrouve, Usage};
use crate::repo::{access, attempts, codes, requests, settings};
use crate::state::NegotiationState;

pub struct Saisie<'a> {
    pub person_id: Uuid,
    /// La session d'où la saisie vient, gardée avec l'usage : c'est ce qui
    /// permet plus tard de dire d'où une entrée est partie.
    pub session_id: Option<Uuid>,
    pub code: &'a str,
    /// Déclaré par le client. **Information, jamais borne du compteur.**
    pub device_id: Option<&'a str>,
    pub locale: &'a str,
}

pub async fn redeem(
    state: &NegotiationState,
    ctx: &RequestContext,
    saisie: Saisie<'_>,
) -> Result<RedeemResult> {
    let mut conn = state.pool().acquire().await?;

    // Relus à chaque tentative, sans cache : c'est ce qui fait qu'une bascule
    // au back-office se voit à l'entrée suivante (SC-002).
    let mode = settings::mode_dadmission(&mut conn).await?;
    let limite = settings::limite_dessais(&mut conn).await?;

    let essais = attempts::recents(&mut conn, saisie.person_id, limite.fenetre_minutes).await?;
    if essais >= limite.max {
        let attente = i64::from(limite.verrou_minutes) * 60;
        return refus(state, ctx, &saisie, RedeemResult::trop_dessais(attente)).await;
    }

    let normalise = code::normaliser(saisie.code);
    if normalise.is_empty() {
        return Err(ApiError::validation(
            "Saisissez le code d'invitation reçu : huit caractères, tirets compris.",
            "code",
        ));
    }

    let trouve = codes::par_forme_normalisee(&mut conn, &normalise, saisie.locale).await?;

    // La connexion de lecture se rend AVANT d'ouvrir la transaction d'écriture :
    // la garder ferait tenir deux connexions du pool par saisie, pour rien.
    drop(conn);

    let Some(trouve) = trouve else {
        return refus(state, ctx, &saisie, RedeemResult::inconnu()).await;
    };

    // L'état vient de `v_invitation_codes` — la même expression que celle du
    // back-office, et non un calcul refait ici.
    match trouve.state.as_str() {
        "revoked" => {
            let revoque_le = trouve.revoked_at.ok_or_else(|| {
                ApiError::internal("code révoqué sans date de révocation : la vue a divergé")
            })?;
            refus(state, ctx, &saisie, RedeemResult::revoque(revoque_le)).await
        }
        "expired" => refus(state, ctx, &saisie, RedeemResult::termine()).await,
        "not_yet_valid" => {
            refus(
                state,
                ctx,
                &saisie,
                RedeemResult::pas_encore_ouvert(trouve.valid_from),
            )
            .await
        }
        "exhausted" => refus(state, ctx, &saisie, RedeemResult::epuise()).await,
        "active" => entrer(state, ctx, &saisie, &trouve, mode).await,
        autre => Err(ApiError::internal(format!(
            "état de code « {autre} » inconnu du code : la vue et l'énuméré ont divergé"
        ))),
    }
}

/// Le code est juste. Reste à savoir ce qu'il ouvre, et si le mode le laisse
/// ouvrir.
async fn entrer(
    state: &NegotiationState,
    ctx: &RequestContext,
    saisie: &Saisie<'_>,
    trouve: &CodeTrouve,
    mode: AdmissionMode,
) -> Result<RedeemResult> {
    let portee = portee_du_code(trouve)?;
    let deja_admise =
        auth::has_permission(state.pool(), saisie.person_id, SPACE_ACCESS, portee).await?;

    // Une seule transaction, ouverte par la porte unique : elle pose
    // `app.actor_id`, sans quoi l'audit de l'attribution serait anonyme.
    let mut tx = state.db().write(ctx).await?;

    if !deja_admise && !mode.ouvre_aussitot() {
        // Mode « approbation » ou « les deux » : le code reconnu n'ouvre pas, il
        // ouvre une demande **qui le porte** (FR-023). Aucun usage n'est
        // enregistré — on n'est pas entré —, et le quota du code n'est pas
        // entamé par une demande qui peut être refusée.
        let demande = match requests::en_attente(
            &mut tx,
            saisie.person_id,
            trouve.scope_type.as_str(),
            trouve.space_id,
        )
        .await?
        {
            Some(existante) => existante,
            None => {
                requests::ouvrir(
                    &mut tx,
                    saisie.person_id,
                    trouve.scope_type.as_str(),
                    trouve.space_id,
                    Some(trouve.id),
                    None,
                )
                .await?
            }
        };

        attempts::enregistrer(
            &mut tx,
            saisie.person_id,
            saisie.device_id,
            RedeemIssue::Accepted.outcome_db(),
        )
        .await?;
        tx.commit().await?;
        return Ok(RedeemResult::demande_ouverte(demande));
    }

    // L'usage d'abord : son insertion prend le verrou de la ligne du code, et
    // c'est ce verrou qui sérialise deux entrées simultanées sur le dernier
    // usage disponible.
    let usage =
        codes::enregistrer_usage(&mut tx, trouve.id, saisie.person_id, saisie.session_id).await?;
    if usage == Usage::Epuise {
        // La transaction est perdue dès que la contrainte a parlé : le refus
        // s'enregistre dans la suivante.
        tx.rollback().await?;
        return refus(state, ctx, saisie, RedeemResult::epuise()).await;
    }

    let reseau_rejoint = match trouve.grants_network_term_id {
        Some(terme) => {
            let ajoute =
                access::rejoindre_le_reseau(&mut tx, saisie.person_id, terme, trouve.id).await?;
            ajoute.then(|| trouve.network_label.clone()).flatten()
        }
        None => None,
    };

    if !deja_admise {
        let attribution = access::accorder(
            &mut tx,
            saisie.person_id,
            trouve.scope_type.as_str(),
            trouve.space_id,
            &format!("Code d'invitation {}", trouve.code),
        )
        .await?;

        // L'annuaire de l'espace, qui n'accorde rien mais dit qui en fait
        // partie. Une portée globale n'a pas d'espace à rejoindre.
        if let Some(space_id) = trouve.space_id {
            access::inscrire_a_lespace(&mut tx, space_id, saisie.person_id).await?;
        }

        // Le code accepté remplace la demande en attente (US4, scénario 6).
        access::annuler_la_demande_en_attente(
            &mut tx,
            saisie.person_id,
            trouve.scope_type.as_str(),
            trouve.space_id,
        )
        .await?;

        if let Some(attribution) = attribution {
            emettre_laccess_accorde(&mut tx, saisie.person_id, trouve, attribution).await?;
        }
    }

    attempts::enregistrer(
        &mut tx,
        saisie.person_id,
        saisie.device_id,
        RedeemIssue::Accepted.outcome_db(),
    )
    .await?;

    // Lues **dans la transaction**, donc après l'écriture : l'état rendu à
    // l'écran est celui qui vient d'être posé, jamais celui d'avant.
    let accorde = access::accorde(&mut tx, saisie.person_id, saisie.locale)
        .await?
        .ok_or_else(|| ApiError::internal("accès introuvable juste après avoir été accordé"))?;
    let reseaux = access::reseaux(&mut tx, saisie.person_id, saisie.locale).await?;

    tx.commit().await?;

    Ok(if deja_admise {
        RedeemResult::deja_admise(accorde, reseaux, reseau_rejoint.as_deref())
    } else {
        RedeemResult::accepte(accorde, reseaux, reseau_rejoint.as_deref())
    })
}

/// La portée du code, telle que le rôle `negotiator` l'autorise — et rien
/// d'autre. `ck_invitation_codes_scope` garantit déjà l'accord entre
/// `scope_type` et `space_id` ; un désaccord ici ne peut venir que d'une
/// divergence entre le modèle et l'énuméré du noyau.
fn portee_du_code(trouve: &CodeTrouve) -> Result<Scope> {
    match (trouve.scope_type.as_str(), trouve.space_id) {
        ("global", None) => Ok(Scope::Global),
        ("negotiation_space", Some(space_id)) => Ok(Scope::NegotiationSpace(space_id)),
        (autre, _) => Err(ApiError::internal(format!(
            "portée de code « {autre} » incohérente : ck_invitation_codes_scope aurait dû la refuser"
        ))),
    }
}

async fn emettre_laccess_accorde(
    conn: &mut sqlx::postgres::PgConnection,
    person_id: Uuid,
    trouve: &CodeTrouve,
    attribution: Uuid,
) -> Result<()> {
    let charge = serde_json::to_value(evenements::SpaceAccessGranted {
        person_id,
        scope_type: trouve.scope_type.clone(),
        space_id: trouve.space_id,
        origin: evenements::AccessOrigin::InvitationCode,
        // L'identifiant du code, jamais le code lui-même : l'outbox est
        // durable et faite pour être relue.
        invitation_code_id: Some(trouve.id),
        network_term_id: trouve.grants_network_term_id,
    })
    .map_err(ApiError::internal)?;

    events::emit(
        conn,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_SPACE_ACCESS,
            aggregate_id: attribution,
            event_type: evenements::SPACE_ACCESS_GRANTED,
            payload: charge,
        },
    )
    .await?;

    Ok(())
}

/// Un refus : l'essai s'écrit, et rien d'autre.
///
/// Sa propre transaction, parce que les refus n'ont pas tous la même histoire —
/// certains n'ont jamais ouvert de transaction, celui du quota en a perdu une.
/// Un refus qui ne s'enregistrerait pas ferait de la limite d'essais une
/// décoration.
async fn refus(
    state: &NegotiationState,
    ctx: &RequestContext,
    saisie: &Saisie<'_>,
    resultat: RedeemResult,
) -> Result<RedeemResult> {
    let mut tx = state.db().write(ctx).await?;
    attempts::enregistrer(
        &mut tx,
        saisie.person_id,
        saisie.device_id,
        resultat.issue.outcome_db(),
    )
    .await?;
    tx.commit().await?;

    Ok(resultat)
}
