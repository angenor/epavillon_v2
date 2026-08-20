//! Les adhésions : inviter, décider, accepter, révoquer.
//!
//! **Deux files qui ne se confondent jamais.** Un référent tranche ce qu'il a
//! reçu — `invited_at` nul —, une personne accepte ce qu'on lui a envoyé. Les
//! confondre ferait entrer quelqu'un qui n'a rien accepté, et c'est le refus
//! `ORG_MEMBERSHIP_IS_INVITATION` qui l'empêche.

use contracts::org as evenements;
use kernel::auth::Scope;
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::events::{self, DomainEvent};
use kernel::tokens::{self, TokenPurpose, TokenRejection};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::ids::{MembershipId, OrganizationId, PersonId};
use crate::domain::membership::{
    AcceptInvitationOutcome, DecideMembership, InvitationPayload, InviteMember, InviteOutcome,
    MemberEntry, Membership, RevokeOutcome,
};
use crate::domain::permissions::ORGANIZATION_MANAGE;
use crate::jobs;
use crate::repo::{memberships, organizations};
use crate::state::OrgState;

/// Inviter quelqu'un par son adresse.
///
/// **La personne est créée sans compte et sans nom déduit de l'adresse.** Un nom
/// tiré de `prenom.nom@…` serait faux une fois sur deux et s'afficherait
/// pourtant dans la liste des membres ; la personne le renseignera en
/// s'inscrivant — ce qu'elle peut désormais faire (research.md § R9).
pub async fn invite(
    state: &OrgState,
    ctx: &RequestContext,
    acteur: PersonId,
    organisation: OrganizationId,
    demande: InviteMember,
) -> Result<InviteOutcome> {
    let mut tx = state.db().write(ctx).await?;

    if !memberships::is_manager(&mut *tx, organisation, acteur).await? {
        return Err(ApiError::new(ErrorCode::OrgNotManager));
    }

    let Some(fiche) = organizations::by_id(&mut *tx, organisation).await? else {
        return Err(ApiError::not_found());
    };

    let personne = trouver_ou_creer_personne(&mut tx, &demande.email).await?;

    // **`already_invited` se dit avant que la contrainte ne remonte** (FR-039) :
    // le refus doit porter l'adhésion en vol, pour que l'écran propose de
    // relancer plutôt que d'émettre une seconde invitation.
    if let Some(existante) = memberships::by_couple(&mut *tx, organisation, personne).await? {
        if existante.status != crate::domain::membership::MembershipStatus::Revoked {
            let entree = memberships::entry_of(&mut *tx, existante.id)
                .await?
                .ok_or_else(|| ApiError::internal("adhésion lue puis disparue"))?;
            tx.commit().await?;

            // **L'état d'abord, la direction ensuite.** `invited_at` reste
            // renseignée après l'acceptation — elle porte l'histoire, pas
            // l'attente : une adhésion active née d'une invitation dirait
            // « déjà invitée » et l'écran proposerait de relancer un lien dont
            // personne n'a plus besoin.
            return Ok(if existante.is_pending_invitation() {
                InviteOutcome::AlreadyInvited {
                    entry: Box::new(entree),
                }
            } else {
                InviteOutcome::AlreadyMember {
                    entry: Box::new(entree),
                }
            });
        }
    }

    let adhesion = memberships::invite(
        &mut tx,
        organisation,
        personne,
        demande.role,
        demande.job_title.as_deref(),
        acteur,
    )
    .await?
    .ok_or_else(|| ApiError::internal("l'invitation n'a rien rendu sur une place libre"))?;

    // Le jeton naît dans la transaction du changement d'état : ni lui ni son
    // travail d'envoi ne survivent à un `ROLLBACK`.
    tokens::invalidate_pending(&mut tx, personne.as_uuid(), TokenPurpose::Invitation).await?;
    let jeton = tokens::create(
        &mut tx,
        &state.config().auth.token_ttl,
        personne.as_uuid(),
        TokenPurpose::Invitation,
        InvitationPayload {
            organization_id: organisation.as_uuid(),
            membership_id: adhesion.id.as_uuid(),
            email: demande.email.clone(),
        }
        .to_value(),
    )
    .await?;

    let destinataire = lire_destinataire(&mut tx, personne).await?;
    jobs::emails::mettre_en_file_invitation(
        &mut tx,
        adhesion.id,
        &destinataire.email,
        &destinataire.locale,
        &destinataire.first_name,
        &fiche.legal_name,
        &jeton.clear,
    )
    .await?;

    emettre_demande(
        &mut tx,
        &adhesion,
        evenements::MembershipDirection::Invited,
        false,
    )
    .await?;

    let entree = memberships::entry_of(&mut *tx, adhesion.id)
        .await?
        .ok_or_else(|| ApiError::internal("adhésion créée puis disparue"))?;

    tx.commit().await?;
    Ok(InviteOutcome::Invited {
        entry: Box::new(entree),
    })
}

/// La décision d'un référent sur une **demande**.
pub async fn decide(
    state: &OrgState,
    ctx: &RequestContext,
    acteur: PersonId,
    id: MembershipId,
    demande: DecideMembership,
) -> Result<Option<Membership>> {
    let mut tx = state.db().write(ctx).await?;

    let Some(adhesion) = memberships::by_id(&mut *tx, id).await? else {
        return Ok(None);
    };

    if !memberships::is_manager(&mut *tx, adhesion.organization_id, acteur).await? {
        return Err(ApiError::new(ErrorCode::OrgNotManager));
    }

    // **Le refus qui empêche de faire entrer quelqu'un qui n'a rien accepté**
    // (écart n° 33). Une invitation attend la personne, pas l'organisation.
    if adhesion.is_pending_invitation() {
        return Err(ApiError::new(ErrorCode::OrgMembershipIsInvitation));
    }

    if adhesion.status != crate::domain::membership::MembershipStatus::Pending {
        return Err(ApiError::new(ErrorCode::OrgMembershipNotPending));
    }

    let issue = if demande.approved {
        let approuvee = memberships::approve(&mut tx, id, acteur)
            .await?
            .ok_or_else(|| ApiError::new(ErrorCode::OrgMembershipNotPending))?;

        emettre_approbation(&mut tx, &approuvee).await?;
        jobs::emails::mettre_en_file_approbation(
            &mut tx,
            approuvee.id,
            approuvee.person_id.as_uuid(),
            approuvee.organization_id,
        )
        .await?;
        jobs::planifier_apres_ecriture(&mut tx, state.config(), approuvee.organization_id).await?;
        approuvee
    } else {
        // **Un refus révoque, il ne supprime pas.** Supprimer effacerait
        // l'histoire, et la personne pourrait redemander comme si rien ne
        // s'était passé.
        let refusee = memberships::revoke(&mut tx, id)
            .await?
            .ok_or_else(|| ApiError::new(ErrorCode::OrgMembershipNotPending))?;

        emettre_revocation(&mut tx, &refusee, evenements::RevocationCause::Declined).await?;
        refusee
    };

    tx.commit().await?;
    Ok(Some(issue))
}

/// Accepter une invitation par son jeton — **sans session exigée** (R10).
///
/// Le jeton **est** la preuve d'adresse, comme pour la vérification d'adresse de
/// B1. Exiger une session rendrait l'invitation inutilisable par la personne
/// qu'elle vise le plus souvent : celle qui n'a pas encore de compte.
pub async fn accept_invitation(
    state: &OrgState,
    ctx: &RequestContext,
    session: Option<PersonId>,
    jeton: &str,
) -> Result<AcceptInvitationOutcome> {
    let mut tx = state.db().write(ctx).await?;

    let consomme = match tokens::consume(&mut tx, jeton, TokenPurpose::Invitation).await? {
        Ok(consomme) => consomme,
        Err(refus) => return Ok(AcceptInvitationOutcome::Rejected { reason: refus }),
    };

    let Some(person_id) = consomme.person_id.map(PersonId) else {
        return Ok(AcceptInvitationOutcome::Rejected {
            reason: TokenRejection::Invalid,
        });
    };

    // Le seul cas gênant : quelqu'un de connecté qui suit le lien reçu par un
    // collègue et entre à sa place.
    if let Some(connectee) = session {
        if connectee != person_id {
            return Err(ApiError::new(ErrorCode::OrgInvitationNotYours));
        }
    }

    // L'acteur ne se connaissait pas à l'ouverture — la personne n'a pas
    // forcément de session, et son identifiant sort du jeton qu'on vient de
    // consommer. Sans cela, l'audit porterait un acteur nul pour un geste
    // qu'elle a bel et bien fait.
    kernel::db::set_actor(&mut tx, person_id.as_uuid()).await?;

    let charge: InvitationPayload = serde_json::from_value(consomme.payload)
        .map_err(|e| ApiError::internal(format!("charge utile du jeton illisible : {e}")))?;

    let adhesion_id = MembershipId(charge.membership_id);
    let Some(adhesion) = memberships::by_id(&mut *tx, adhesion_id).await? else {
        return Ok(AcceptInvitationOutcome::Rejected {
            reason: TokenRejection::Invalid,
        });
    };

    let approuvee = match memberships::approve(&mut tx, adhesion_id, person_id).await? {
        Some(a) => a,
        // L'adhésion n'attend plus : elle a été révoquée, ou déjà acceptée par
        // un autre onglet. Le jeton, lui, vient d'être consommé.
        None if adhesion.status == crate::domain::membership::MembershipStatus::Active => adhesion,
        None => {
            return Ok(AcceptInvitationOutcome::Rejected {
                reason: TokenRejection::Invalid,
            })
        }
    };

    // **Le lien vient de prouver l'adresse** : laisser la personne redemander un
    // second lien pour la même adresse serait une formalité vide.
    sqlx::query!(
        "UPDATE identity.people SET email_verified_at = now()
          WHERE id = $1 AND email_verified_at IS NULL",
        person_id.as_uuid()
    )
    .execute(&mut *tx)
    .await?;

    let Some(organisation) = organizations::by_id(&mut *tx, approuvee.organization_id).await?
    else {
        return Err(ApiError::internal("organisation de l'invitation disparue"));
    };

    emettre_approbation(&mut tx, &approuvee).await?;
    jobs::planifier_apres_ecriture(&mut tx, state.config(), approuvee.organization_id).await?;

    tx.commit().await?;

    Ok(AcceptInvitationOutcome::Accepted {
        membership: Box::new(approuvee),
        organization: Box::new(organisation),
    })
}

/// Retirer un membre, ou quitter une organisation.
///
/// **Le retrait du dernier référent actif est refusé** (FR-041) : une
/// organisation sans référent n'a plus personne pour accepter une demande, et se
/// retrouve close sans que quiconque l'ait voulu. Un administrateur détenant la
/// permission de gestion peut passer outre — il y a des cas où il le faut, et
/// lui saura ce qu'il fait.
pub async fn revoke(
    state: &OrgState,
    ctx: &RequestContext,
    acteur: PersonId,
    id: MembershipId,
) -> Result<RevokeOutcome> {
    let mut tx = state.db().write(ctx).await?;

    let Some(adhesion) = memberships::by_id(&mut *tx, id).await? else {
        return Err(ApiError::not_found());
    };

    let soi_meme = adhesion.person_id == acteur;
    let referent = memberships::is_manager(&mut *tx, adhesion.organization_id, acteur).await?;
    let administrateur =
        kernel::auth::has_permission(state.pool(), acteur.0, ORGANIZATION_MANAGE, Scope::Global)
            .await?;

    if !soi_meme && !referent && !administrateur {
        return Err(ApiError::new(ErrorCode::OrgNotManager));
    }

    let dernier_referent = adhesion.role == crate::domain::membership::MembershipRole::Manager
        && adhesion.status == crate::domain::membership::MembershipStatus::Active
        && memberships::other_active_managers(&mut tx, adhesion.organization_id, id).await? == 0;

    if dernier_referent && !administrateur {
        return Ok(RevokeOutcome::LastManager);
    }

    let Some(revoquee) = memberships::revoke(&mut tx, id).await? else {
        // Déjà révoquée : le geste est sans effet, et le dire ainsi évite de
        // faire croire à une erreur.
        tx.commit().await?;
        return Ok(RevokeOutcome::Revoked);
    };

    let cause = if soi_meme {
        evenements::RevocationCause::Left
    } else {
        evenements::RevocationCause::Removed
    };

    emettre_revocation(&mut tx, &revoquee, cause).await?;
    jobs::planifier_apres_ecriture(&mut tx, state.config(), revoquee.organization_id).await?;

    tx.commit().await?;
    Ok(RevokeOutcome::Revoked)
}

/// La file d'un référent, et celle d'une personne.
pub async fn requests_for_organization(
    state: &OrgState,
    organisation: OrganizationId,
) -> Result<Vec<MemberEntry>> {
    memberships::requests_for_organization(state.pool(), organisation).await
}

pub async fn invitations_for_person(
    state: &OrgState,
    person_id: PersonId,
) -> Result<Vec<MemberEntry>> {
    memberships::invitations_for_person(state.pool(), person_id).await
}

// -----------------------------------------------------------------------------

struct Destinataire {
    email: String,
    locale: String,
    first_name: String,
}

async fn lire_destinataire(conn: &mut PgConnection, person_id: PersonId) -> Result<Destinataire> {
    let ligne = sqlx::query!(
        r#"SELECT primary_email::text AS "email!", preferred_locale, first_name
             FROM identity.people WHERE id = $1"#,
        person_id.as_uuid()
    )
    .fetch_one(conn)
    .await?;

    Ok(Destinataire {
        email: ligne.email,
        locale: ligne.preferred_locale,
        first_name: ligne.first_name,
    })
}

/// La personne visée par une invitation, créée **sans compte** si l'adresse est
/// inconnue.
///
/// Les deux colonnes de nom sont `NOT NULL` : on y met un libellé neutre plutôt
/// qu'un nom déduit de l'adresse, qui serait faux une fois sur deux et
/// s'afficherait pourtant dans la liste des membres.
async fn trouver_ou_creer_personne(conn: &mut PgConnection, email: &str) -> Result<PersonId> {
    let connue = sqlx::query_scalar!(
        "SELECT id FROM identity.people WHERE primary_email = $1::text::platform.email",
        email
    )
    .fetch_optional(&mut *conn)
    .await?;

    if let Some(id) = connue {
        return Ok(PersonId(id));
    }

    let id: Uuid = sqlx::query_scalar!(
        r#"INSERT INTO identity.people (primary_email, first_name, last_name, status)
           VALUES ($1::text::platform.email, 'Invité·e', '—', 'active')
        RETURNING id"#,
        email
    )
    .fetch_one(conn)
    .await?;

    Ok(PersonId(id))
}

async fn emettre_demande(
    conn: &mut PgConnection,
    adhesion: &Membership,
    direction: evenements::MembershipDirection,
    auto_joined: bool,
) -> Result<()> {
    events::emit(
        conn,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_MEMBERSHIP,
            aggregate_id: adhesion.id.as_uuid(),
            event_type: evenements::MEMBERSHIP_REQUESTED,
            payload: serde_json::to_value(evenements::MembershipRequested {
                membership_id: adhesion.id.as_uuid(),
                organization_id: adhesion.organization_id.as_uuid(),
                person_id: adhesion.person_id.as_uuid(),
                direction,
                auto_joined,
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;
    Ok(())
}

async fn emettre_approbation(conn: &mut PgConnection, adhesion: &Membership) -> Result<()> {
    events::emit(
        conn,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_MEMBERSHIP,
            aggregate_id: adhesion.id.as_uuid(),
            event_type: evenements::MEMBERSHIP_APPROVED,
            payload: serde_json::to_value(evenements::MembershipApproved {
                membership_id: adhesion.id.as_uuid(),
                organization_id: adhesion.organization_id.as_uuid(),
                person_id: adhesion.person_id.as_uuid(),
                role: adhesion.role.as_str().to_owned(),
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;
    Ok(())
}

async fn emettre_revocation(
    conn: &mut PgConnection,
    adhesion: &Membership,
    cause: evenements::RevocationCause,
) -> Result<()> {
    events::emit(
        conn,
        DomainEvent {
            aggregate_schema: evenements::AGGREGATE_SCHEMA,
            aggregate_type: evenements::AGGREGATE_MEMBERSHIP,
            aggregate_id: adhesion.id.as_uuid(),
            event_type: evenements::MEMBERSHIP_REVOKED,
            payload: serde_json::to_value(evenements::MembershipRevoked {
                membership_id: adhesion.id.as_uuid(),
                organization_id: adhesion.organization_id.as_uuid(),
                person_id: adhesion.person_id.as_uuid(),
                cause,
            })
            .map_err(ApiError::internal)?,
        },
    )
    .await?;
    Ok(())
}
