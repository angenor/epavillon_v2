//! **Accepter une invitation n'exige pas de session** (R10).
//!
//! Le jeton **est** la preuve d'adresse, comme pour la vérification d'adresse de
//! B1 — qui n'exige pas non plus de session. L'exiger rendrait l'invitation
//! inutilisable par la personne qu'elle vise le plus souvent : celle qui n'a pas
//! encore de compte.

mod commun;

use commun::{ifdd, personne, Bac};
use kernel::error::ErrorCode;
use kernel::tokens::TokenRejection;
use org::domain::ids::{OrganizationId, PersonId};
use org::domain::membership::{
    AcceptInvitationOutcome, InviteMember, MembershipRole, MembershipStatus,
};
use org::service::membership;
use uuid::Uuid;

const INVITEE: &str = "invitee@example.org";

async fn referent(bac: &Bac, organisation: Uuid) -> Uuid {
    sqlx::query_scalar!(
        "SELECT person_id FROM org.memberships
          WHERE organization_id = $1 AND role = 'manager' AND status = 'active'",
        organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("le référent du semis")
}

/// Invite, et rend le jeton en clair tel qu'il partirait dans le courriel.
async fn inviter(bac: &Bac, organisation: Uuid, chef: Uuid) -> String {
    membership::invite(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        OrganizationId(organisation),
        InviteMember {
            organization_id: None,
            email: INVITEE.to_owned(),
            role: MembershipRole::Member,
            job_title: Some("Chargé de mission".to_owned()),
        },
    )
    .await
    .expect("invitation");

    sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'org.membership.invitation_email'
          ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture du travail d'envoi")
    .expect("le travail porte le jeton en clair")
}

#[tokio::test]
async fn une_invitation_saccepte_sans_session() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = referent(&bac, organisation).await;
    let jeton = inviter(&bac, organisation, chef).await;

    // **Aucune session** : c'est le cas de la personne qui n'a pas de compte.
    let issue = membership::accept_invitation(&bac.state, &bac.ctx(), None, &jeton)
        .await
        .expect("acceptation");

    let adhesion = match issue {
        AcceptInvitationOutcome::Accepted { membership, .. } => membership,
        autre => panic!("issue inattendue : {autre:?}"),
    };

    assert_eq!(adhesion.status, MembershipStatus::Active);
    assert!(
        adhesion.is_invitation(),
        "l'histoire de l'adhésion garde sa direction"
    );

    // **L'adresse est marquée vérifiée** : le lien vient de la prouver, et
    // redemander un second lien pour la même adresse serait une formalité vide.
    let verifiee = sqlx::query_scalar!(
        "SELECT email_verified_at FROM identity.people
          WHERE primary_email = $1::text::platform.email",
        INVITEE
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la personne");
    assert!(verifiee.is_some(), "le lien a prouvé l'adresse");
}

/// Le rejeu du lien dit « déjà utilisé » — la consommation est atomique.
#[tokio::test]
async fn le_rejeu_du_lien_dit_deja_utilise() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = referent(&bac, organisation).await;
    let jeton = inviter(&bac, organisation, chef).await;

    membership::accept_invitation(&bac.state, &bac.ctx(), None, &jeton)
        .await
        .expect("première acceptation");

    let seconde = membership::accept_invitation(&bac.state, &bac.ctx(), None, &jeton)
        .await
        .expect("le rejeu ne produit pas d'erreur");

    match seconde {
        AcceptInvitationOutcome::Rejected { reason } => {
            assert_eq!(reason, TokenRejection::AlreadyUsed)
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }
}

/// **La session d'une autre personne est refusée.** C'est le seul cas gênant :
/// quelqu'un de connecté qui suit le lien reçu par un collègue.
#[tokio::test]
async fn la_session_dune_autre_personne_est_refusee() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = referent(&bac, organisation).await;
    let jeton = inviter(&bac, organisation, chef).await;

    let karim = personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;

    let refus = membership::accept_invitation(
        &bac.state,
        &bac.ctx().with_actor(karim),
        Some(PersonId(karim)),
        &jeton,
    )
    .await
    .expect_err("une session étrangère est refusée");

    assert_eq!(refus.code, ErrorCode::OrgInvitationNotYours);

    // Le jeton a bien été consommé au passage — la consommation est atomique et
    // précède le contrôle : c'est le prix de l'atomicité, et un lien qu'un tiers
    // a essayé de détourner ne doit de toute façon plus servir.
    let invitee = sqlx::query_scalar!(
        "SELECT id FROM identity.people WHERE primary_email = $1::text::platform.email",
        INVITEE
    )
    .fetch_one(bac.pool())
    .await
    .expect("la personne invitée");

    let adhesion =
        org::repo::memberships::by_couple(bac.pool(), organisation.into(), PersonId(invitee))
            .await
            .expect("lecture")
            .expect("l'adhésion");
    assert_eq!(
        adhesion.status,
        MembershipStatus::Pending,
        "l'adhésion de l'invitée n'a pas bougé"
    );
}

/// Un jeton inconnu est **invalide**, pas « périmé » : les deux refus mènent à
/// des suites différentes à l'écran.
#[tokio::test]
async fn un_jeton_inconnu_est_invalide() {
    let bac = Bac::monter().await;

    let issue = membership::accept_invitation(&bac.state, &bac.ctx(), None, "jeton-inexistant")
        .await
        .expect("un jeton inconnu ne produit pas d'erreur");

    match issue {
        AcceptInvitationOutcome::Rejected { reason } => {
            assert_eq!(reason, TokenRejection::Invalid)
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }
}

/// La session **de la bonne personne** passe : le contrôle ne gêne que
/// l'usurpation.
#[tokio::test]
async fn la_session_de_la_personne_invitee_passe() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = referent(&bac, organisation).await;
    let jeton = inviter(&bac, organisation, chef).await;

    let invitee = sqlx::query_scalar!(
        "SELECT id FROM identity.people WHERE primary_email = $1::text::platform.email",
        INVITEE
    )
    .fetch_one(bac.pool())
    .await
    .expect("la personne invitée");

    let issue = membership::accept_invitation(
        &bac.state,
        &bac.ctx().with_actor(invitee),
        Some(PersonId(invitee)),
        &jeton,
    )
    .await
    .expect("acceptation par la personne visée");

    assert!(matches!(issue, AcceptInvitationOutcome::Accepted { .. }));
}
