//! **L'équipe accepte une demande et change un rôle** depuis le back-office
//! (31/10) : l'IFDD n'avait plus de référent, et personne ne pouvait accepter.
//!
//! Les deux règles du référent tiennent : une invitation ne s'accepte pas à la
//! place de la personne, et le dernier référent ne perd son rôle que sur
//! confirmation.

mod commun;

use commun::{evenements_emis, ifdd, perimetres, personne, Bac};
use kernel::auth::AdminScope;
use kernel::error::ErrorCode;
use org::domain::admin::{AdminMembershipDecision, MembershipRoleChange, OrganizationWriteOutcome};
use org::domain::ids::{MembershipId, OrganizationId, PersonId};
use org::domain::membership::{MembershipRole, MembershipStatus};
use org::service::admin_members;
use uuid::Uuid;

async fn equipe(bac: &Bac) -> (Uuid, AdminScope) {
    let p = perimetres(bac).await;
    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");
    (p.globale, perimetre)
}

async fn sans_referent(bac: &Bac, organisation: Uuid) {
    sqlx::query!(
        "UPDATE org.memberships SET status = 'revoked', revoked_at = now(), is_primary = false
          WHERE organization_id = $1 AND role = 'manager' AND status = 'active'",
        organisation
    )
    .execute(bac.pool())
    .await
    .expect("retrait du référent du semis");
}

async fn adhesion(
    bac: &Bac,
    organisation: Uuid,
    qui: Uuid,
    sql_statut: &str,
    invitee_par: Option<Uuid>,
) -> MembershipId {
    let id = sqlx::query_scalar!(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, job_title, invited_by, invited_at, approved_at)
         VALUES ($1, $2, 'member', $3::text::org.membership_status, 'Chargée de programme', $4,
                 CASE WHEN $4::uuid IS NULL THEN NULL ELSE now() END,
                 CASE WHEN $3 = 'active' THEN now() END)
         RETURNING id",
        organisation,
        qui,
        sql_statut,
        invitee_par
    )
    .fetch_one(bac.pool())
    .await
    .expect("adhésion");
    MembershipId(id)
}

async fn relue(bac: &Bac, id: MembershipId) -> org::domain::membership::Membership {
    org::repo::memberships::by_id(bac.pool(), id)
        .await
        .expect("lecture")
        .expect("l'adhésion")
}

#[tokio::test]
async fn l_equipe_accepte_une_demande_et_en_fait_le_referent() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    sans_referent(&bac, organisation).await;
    let (admin, perimetre) = equipe(&bac).await;
    let awa = personne(&bac, "awa.diallo@ifdd.francophonie.org", "Awa", "Diallo").await;
    let demande = adhesion(&bac, organisation, awa, "pending", None).await;

    let issue = admin_members::decide(
        &bac.state,
        &bac.ctx().with_actor(admin),
        &perimetre,
        PersonId(admin),
        OrganizationId(organisation),
        demande,
        AdminMembershipDecision {
            approved: true,
            role: Some(MembershipRole::Manager),
        },
    )
    .await
    .expect("acceptation");
    assert!(
        matches!(issue, OrganizationWriteOutcome::Saved { .. }),
        "{issue:?}"
    );

    let apres = relue(&bac, demande).await;
    assert_eq!(apres.status, MembershipStatus::Active);
    assert_eq!(apres.role, MembershipRole::Manager);
    assert_eq!(apres.approved_by, Some(PersonId(admin)));

    assert!(evenements_emis(&bac, demande.as_uuid())
        .await
        .contains(&"org.membership.approved".to_owned()));

    let auteur = sqlx::query_scalar!(
        "SELECT actor_id FROM platform.audit_log
          WHERE entity_table = 'memberships' AND entity_id = $1 AND action = 'update'
          ORDER BY occurred_at DESC LIMIT 1",
        demande.as_uuid()
    )
    .fetch_one(bac.pool())
    .await
    .expect("ligne d'audit");
    assert_eq!(auteur, Some(admin), "l'audit nomme l'équipe");
}

/// Écart n° 33 : l'équipe ne fait pas entrer quelqu'un qui n'a rien accepté.
#[tokio::test]
async fn une_invitation_ne_s_accepte_pas_a_la_place_de_la_personne() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let (admin, perimetre) = equipe(&bac).await;
    let koffi = personne(&bac, "k.mensah@ifdd.francophonie.org", "Koffi", "Mensah").await;
    let invitation = adhesion(&bac, organisation, koffi, "pending", Some(admin)).await;

    let refus = admin_members::decide(
        &bac.state,
        &bac.ctx().with_actor(admin),
        &perimetre,
        PersonId(admin),
        OrganizationId(organisation),
        invitation,
        AdminMembershipDecision {
            approved: true,
            role: None,
        },
    )
    .await
    .expect_err("une invitation attend la personne");
    assert_eq!(refus.code, ErrorCode::OrgMembershipIsInvitation);
    assert_eq!(
        relue(&bac, invitation).await.status,
        MembershipStatus::Pending
    );
}

#[tokio::test]
async fn le_dernier_referent_ne_perd_son_role_que_sur_confirmation() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    sans_referent(&bac, organisation).await;
    let (admin, perimetre) = equipe(&bac).await;
    let claire = personne(&bac, "c.dubois@ifdd.francophonie.org", "Claire", "Dubois").await;
    let membre = adhesion(&bac, organisation, claire, "active", None).await;

    let ctx = bac.ctx().with_actor(admin);
    let changer = |role, confirm_last_manager| {
        admin_members::set_role(
            &bac.state,
            &ctx,
            &perimetre,
            OrganizationId(organisation),
            membre,
            MembershipRoleChange {
                role,
                confirm_last_manager,
            },
        )
    };

    let issue = changer(MembershipRole::Manager, false)
        .await
        .expect("désignation");
    assert!(matches!(issue, OrganizationWriteOutcome::Saved { .. }));
    assert_eq!(relue(&bac, membre).await.role, MembershipRole::Manager);
    assert!(evenements_emis(&bac, membre.as_uuid())
        .await
        .contains(&"org.membership.role_changed".to_owned()));

    let issue = changer(MembershipRole::Member, false)
        .await
        .expect("refus rendu");
    assert!(
        matches!(issue, OrganizationWriteOutcome::LastManager),
        "{issue:?}"
    );
    assert_eq!(relue(&bac, membre).await.role, MembershipRole::Manager);

    let issue = changer(MembershipRole::Member, true)
        .await
        .expect("retrait confirmé");
    assert!(matches!(issue, OrganizationWriteOutcome::Saved { .. }));
    assert_eq!(relue(&bac, membre).await.role, MembershipRole::Member);
}

#[tokio::test]
async fn une_demande_ne_change_pas_de_role_et_une_url_forgee_ne_touche_rien() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let (admin, perimetre) = equipe(&bac).await;
    let awa = personne(&bac, "awa.diallo@ifdd.francophonie.org", "Awa", "Diallo").await;
    let demande = adhesion(&bac, organisation, awa, "pending", None).await;

    let refus = admin_members::set_role(
        &bac.state,
        &bac.ctx().with_actor(admin),
        &perimetre,
        OrganizationId(organisation),
        demande,
        MembershipRoleChange {
            role: MembershipRole::Manager,
            confirm_last_manager: false,
        },
    )
    .await
    .expect_err("une demande s'accepte d'abord");
    assert_eq!(refus.code, ErrorCode::OrgMembershipNotActive);

    let ailleurs = admin_members::decide(
        &bac.state,
        &bac.ctx().with_actor(admin),
        &perimetre,
        PersonId(admin),
        OrganizationId(Uuid::now_v7()),
        demande,
        AdminMembershipDecision {
            approved: true,
            role: None,
        },
    )
    .await
    .expect("lecture");
    assert!(matches!(ailleurs, OrganizationWriteOutcome::NotFound));
    assert_eq!(relue(&bac, demande).await.status, MembershipStatus::Pending);
}
