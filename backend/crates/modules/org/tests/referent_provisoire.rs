//! **Le référent d'une fiche en attente de validation est provisoire** (31/10).
//!
//! N'importe qui peut créer une fiche au nom d'une organisation qui n'est pas la
//! sienne. Tant que l'équipe ne l'a pas validée, son créateur n'invite ni
//! n'accepte personne ; ce qui a déjà été fait reste.

mod commun;

use commun::{ifdd, personne, Bac};
use kernel::error::ErrorCode;
use org::domain::ids::{MembershipId, OrganizationId, PersonId};
use org::domain::membership::{DecideMembership, InviteMember, MembershipRole, MembershipStatus};
use org::service::membership;
use uuid::Uuid;

async fn fiche_en_attente(bac: &Bac) -> (Uuid, Uuid) {
    let organisation = ifdd(bac).await;
    sqlx::query!(
        "UPDATE org.organizations SET status = 'candidate', verified_at = NULL, verified_by = NULL WHERE id = $1",
        organisation
    )
    .execute(bac.pool())
    .await
    .expect("fiche repassée en attente");

    let chef = sqlx::query_scalar!(
        "SELECT person_id FROM org.memberships
          WHERE organization_id = $1 AND role = 'manager' AND status = 'active'",
        organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("le référent du semis");
    (organisation, chef)
}

#[tokio::test]
async fn le_referent_provisoire_n_invite_pas() {
    let bac = Bac::monter().await;
    let (organisation, chef) = fiche_en_attente(&bac).await;

    let refus = membership::invite(
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
    .expect_err("fiche en attente de validation");
    assert_eq!(refus.code, ErrorCode::OrgPendingValidation);
}

#[tokio::test]
async fn le_referent_provisoire_n_accepte_pas_de_demande_et_l_existant_reste() {
    let bac = Bac::monter().await;
    let (organisation, chef) = fiche_en_attente(&bac).await;
    let awa = personne(&bac, "awa.diallo@exemple.org", "Awa", "Diallo").await;
    let demande = sqlx::query_scalar!(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, job_title)
         VALUES ($1, $2, 'member', 'pending', 'Chargée de programme') RETURNING id",
        organisation,
        awa
    )
    .fetch_one(bac.pool())
    .await
    .expect("demande");

    let refus = membership::decide(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        MembershipId(demande),
        DecideMembership {
            membership_id: None,
            approved: true,
        },
    )
    .await
    .expect_err("fiche en attente de validation");
    assert_eq!(refus.code, ErrorCode::OrgPendingValidation);

    let encore = org::repo::memberships::by_id(bac.pool(), MembershipId(demande))
        .await
        .expect("lecture")
        .expect("la demande");
    assert_eq!(encore.status, MembershipStatus::Pending);

    let referent_toujours_actif = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.memberships
            WHERE organization_id = $1 AND person_id = $2 AND role = 'manager' AND status = 'active'"#,
        organisation,
        chef
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture");
    assert_eq!(referent_toujours_actif, 1, "rien n'est défait");
}

/// Seule l'équipe nomme un référent : une invitation n'en fait pas un.
#[tokio::test]
async fn une_invitation_ne_fait_pas_de_referent() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let chef = sqlx::query_scalar!(
        "SELECT person_id FROM org.memberships
          WHERE organization_id = $1 AND role = 'manager' AND status = 'active'",
        organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("le référent du semis");

    let refus = membership::invite(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        OrganizationId(organisation),
        InviteMember {
            organization_id: None,
            email: "futur.referent@example.org".to_owned(),
            role: MembershipRole::Manager,
            job_title: None,
        },
    )
    .await
    .expect_err("seule l'équipe nomme un référent");
    assert_eq!(refus.code, ErrorCode::OrgManagerAppointedByTeam);
}
