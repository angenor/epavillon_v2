//! **Aucune invitation n'est approuvable par l'organisation, aucune demande
//! n'est acceptable par un jeton** (SC-007).
//!
//! Le statut `pending` recouvre deux attentes inverses, et c'est `invited_at`
//! qui les sépare. Les confondre, c'est faire entrer quelqu'un qui n'a jamais
//! rien accepté — et c'est précisément ce que la colonne existe pour empêcher.
//!
//! Ce test parcourt **toutes les combinaisons** : les deux sortes d'adhésion,
//! les deux gestes.

mod commun;

use commun::{ifdd, personne, Bac};
use kernel::error::ErrorCode;
use org::domain::ids::{MembershipId, OrganizationId, PersonId};
use org::domain::membership::{
    DecideMembership, InviteMember, InviteOutcome, JoinOrganization, MembershipRole,
    MembershipStatus,
};
use org::service::{join, membership};
use uuid::Uuid;

/// Le référent de l'IFDD, semé par `900_seed.sql`.
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

/// Le jeton d'invitation, tel qu'il partirait dans le courriel — la file est le
/// seul endroit où il existe en clair.
async fn jeton_dinvitation(bac: &Bac) -> String {
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

/// **Un référent ne peut pas approuver une invitation.**
#[tokio::test]
async fn une_invitation_nest_pas_approuvable_par_lorganisation() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = referent(&bac, organisation).await;

    let issue = membership::invite(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        OrganizationId(organisation),
        InviteMember {
            organization_id: None,
            email: "invitee@example.org".to_owned(),
            role: MembershipRole::Member,
            job_title: None,
        },
    )
    .await
    .expect("invitation");

    let adhesion = match issue {
        InviteOutcome::Invited { entry } => entry.membership.id,
        autre => panic!("issue inattendue : {autre:?}"),
    };

    let refus = membership::decide(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        MembershipId(adhesion.as_uuid()),
        DecideMembership {
            membership_id: None,
            approved: true,
        },
    )
    .await
    .expect_err("approuver une invitation doit être refusé");

    assert_eq!(
        refus.code,
        ErrorCode::OrgMembershipIsInvitation,
        "c'est le refus qui empêche de faire entrer quelqu'un qui n'a rien accepté"
    );

    // Et l'adhésion n'a pas bougé.
    let apres = org::repo::memberships::by_id(bac.pool(), adhesion)
        .await
        .expect("lecture")
        .expect("l'adhésion");
    assert_eq!(apres.status, MembershipStatus::Pending);
}

/// **Une demande n'est pas acceptable par un jeton.** Elle n'en a pas : aucun
/// jeton d'invitation ne la désigne, et le seul qui existerait pointerait vers
/// une autre adhésion.
#[tokio::test]
async fn une_demande_nest_pas_acceptable_par_un_jeton() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = referent(&bac, organisation).await;

    // Une invitation, pour disposer d'un jeton valide.
    membership::invite(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        OrganizationId(organisation),
        InviteMember {
            organization_id: None,
            email: "invitee@example.org".to_owned(),
            role: MembershipRole::Member,
            job_title: None,
        },
    )
    .await
    .expect("invitation");
    let jeton = jeton_dinvitation(&bac).await;

    // Et une demande spontanée, de quelqu'un d'autre.
    let karim = personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;
    join::join(
        &bac.state,
        &bac.ctx().with_actor(karim),
        PersonId(karim),
        OrganizationId(organisation),
        JoinOrganization {
            organization_id: None,
            job_title: None,
        },
    )
    .await
    .expect("demande");

    // Karim tente d'utiliser le jeton reçu par quelqu'un d'autre.
    let refus = membership::accept_invitation(
        &bac.state,
        &bac.ctx().with_actor(karim),
        Some(PersonId(karim)),
        &jeton,
    )
    .await
    .expect_err("un jeton qui ne vous désigne pas est refusé");

    assert_eq!(refus.code, ErrorCode::OrgInvitationNotYours);

    // Sa demande, elle, attend toujours un référent.
    let sienne =
        org::repo::memberships::by_couple(bac.pool(), organisation.into(), PersonId(karim))
            .await
            .expect("lecture")
            .expect("la demande de Karim");
    assert_eq!(sienne.status, MembershipStatus::Pending);
    assert!(
        !sienne.is_invitation(),
        "c'est une demande, pas une invitation"
    );
}

/// Les **deux files ne se mêlent jamais** : celle du référent ne porte que les
/// demandes reçues, celle de la personne que les invitations émises.
#[tokio::test]
async fn les_deux_files_ne_se_melent_jamais() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = referent(&bac, organisation).await;

    // Une invitation émise par l'organisation.
    membership::invite(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        OrganizationId(organisation),
        InviteMember {
            organization_id: None,
            email: "invitee@example.org".to_owned(),
            role: MembershipRole::Member,
            job_title: None,
        },
    )
    .await
    .expect("invitation");

    let invitee = sqlx::query_scalar!(
        "SELECT id FROM identity.people WHERE primary_email = 'invitee@example.org'::platform.email"
    )
    .fetch_one(bac.pool())
    .await
    .expect("la personne invitée");

    // Une demande reçue de quelqu'un d'autre.
    let karim = personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;
    join::join(
        &bac.state,
        &bac.ctx().with_actor(karim),
        PersonId(karim),
        OrganizationId(organisation),
        JoinOrganization {
            organization_id: None,
            job_title: None,
        },
    )
    .await
    .expect("demande");

    let a_trancher = membership::requests_for_organization(&bac.state, organisation.into())
        .await
        .expect("file du référent");
    let a_accepter = membership::invitations_for_person(&bac.state, PersonId(invitee))
        .await
        .expect("file de la personne");

    assert_eq!(a_trancher.len(), 1, "une seule demande à trancher");
    assert_eq!(a_trancher[0].person.id.as_uuid(), karim);
    assert!(!a_trancher[0].is_invitation);

    assert_eq!(a_accepter.len(), 1, "une seule invitation à accepter");
    assert_eq!(a_accepter[0].person.id.as_uuid(), invitee);
    assert!(a_accepter[0].is_invitation);
}

/// **Une seconde invitation rend `already_invited`**, jamais une erreur de
/// contrainte : l'écran propose de relancer, pas d'en émettre une seconde.
#[tokio::test]
async fn une_seconde_invitation_propose_de_relancer() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = referent(&bac, organisation).await;

    async fn inviter(bac: &Bac, chef: Uuid, organisation: Uuid) -> InviteOutcome {
        membership::invite(
            &bac.state,
            &bac.ctx().with_actor(chef),
            PersonId(chef),
            OrganizationId(organisation),
            InviteMember {
                organization_id: None,
                email: "invitee@example.org".to_owned(),
                role: MembershipRole::Member,
                job_title: None,
            },
        )
        .await
        .expect("invitation")
    }

    assert!(matches!(
        inviter(&bac, chef, organisation).await,
        InviteOutcome::Invited { .. }
    ));

    match inviter(&bac, chef, organisation).await {
        InviteOutcome::AlreadyInvited { entry } => {
            assert!(
                entry.is_invitation,
                "l'adhésion en vol est bien une invitation"
            );
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }

    // Une invitation à quelqu'un qui est **déjà membre** se dit autrement.
    personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;

    let issue = membership::invite(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        OrganizationId(organisation),
        InviteMember {
            organization_id: None,
            email: "karim.ilboudo@example.org".to_owned(),
            role: MembershipRole::Member,
            job_title: None,
        },
    )
    .await
    .expect("invitation de Karim");
    assert!(matches!(issue, InviteOutcome::Invited { .. }));

    // Acceptée, puis réinvitée : « déjà membre ».
    let jeton = jeton_dinvitation(&bac).await;
    membership::accept_invitation(&bac.state, &bac.ctx(), None, &jeton)
        .await
        .expect("acceptation");

    match membership::invite(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        OrganizationId(organisation),
        InviteMember {
            organization_id: None,
            email: "karim.ilboudo@example.org".to_owned(),
            role: MembershipRole::Member,
            job_title: None,
        },
    )
    .await
    .expect("réinvitation")
    {
        InviteOutcome::AlreadyMember { entry } => {
            assert_eq!(entry.membership.status, MembershipStatus::Active);
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }
}

/// L'invitation crée la personne **sans compte et sans nom déduit de
/// l'adresse**. Un nom tiré de `prenom.nom@…` serait faux une fois sur deux et
/// s'afficherait pourtant dans la liste des membres.
#[tokio::test]
async fn linvitation_cree_la_personne_sans_compte_ni_nom_devine() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = referent(&bac, organisation).await;

    membership::invite(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        OrganizationId(organisation),
        InviteMember {
            organization_id: None,
            email: "boureima.ouedraogo@example.org".to_owned(),
            role: MembershipRole::Member,
            job_title: None,
        },
    )
    .await
    .expect("invitation");

    let ligne = sqlx::query!(
        r#"SELECT p.first_name, p.last_name,
                  EXISTS (SELECT 1 FROM identity.accounts a WHERE a.person_id = p.id) AS "a_un_compte!"
             FROM identity.people p
            WHERE p.primary_email = 'boureima.ouedraogo@example.org'::platform.email"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("la personne invitée");

    assert!(!ligne.a_un_compte, "l'invitation ne crée aucun compte");
    assert_ne!(
        ligne.first_name.to_lowercase(),
        "boureima",
        "le nom n'est pas déduit de l'adresse"
    );
}
