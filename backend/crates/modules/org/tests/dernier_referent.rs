//! **Le retrait du dernier référent actif est refusé** (FR-041).
//!
//! Une organisation sans référent n'a plus personne pour accepter une demande :
//! elle se retrouve close sans que quiconque l'ait voulu. Le refus est
//! contournable par la permission de gestion — il y a des cas où il le faut, et
//! l'administrateur qui la détient saura ce qu'il fait.
//!
//! C'est le **seul point d'application** de la règle : sans la route de
//! révocation, elle n'aurait aucun endroit où s'exercer.

mod commun;

use commun::{attribuer, ifdd, personne, Bac};
use org::domain::ids::{MembershipId, PersonId};
use org::domain::membership::{MembershipStatus, RevokeOutcome};
use org::service::membership;
use uuid::Uuid;

/// Un référent **ordinaire**, sans droit d'administration.
///
/// Le référent que sème `900_seed.sql` est le compte d'administration : il
/// détient la permission de gestion des organisations, et le refus n'est pas
/// fait pour lui. L'éprouver sur lui ne prouverait rien.
async fn referent(bac: &Bac, organisation: Uuid) -> (Uuid, MembershipId) {
    let boureima = personne(bac, "b.ouedraogo@osed-sahel.org", "Boureima", "Ouédraogo").await;
    let id = ajouter_referent(bac, organisation, boureima).await;
    (boureima, id)
}

async fn ajouter_referent(bac: &Bac, organisation: Uuid, qui: Uuid) -> MembershipId {
    let id = sqlx::query_scalar!(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, approved_at)
         VALUES ($1, $2, 'manager', 'active', now())
         RETURNING id",
        organisation,
        qui
    )
    .fetch_one(bac.pool())
    .await
    .expect("second référent");

    MembershipId(id)
}

#[tokio::test]
async fn le_retrait_du_dernier_referent_est_refuse_puis_accepte_apres_remplacement() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    // Le référent du semis est retiré de la course : ce test porte sur un
    // référent ordinaire, et il en faut **un seul** pour que la règle joue.
    retirer_le_referent_du_semis(&bac, organisation).await;
    let (chef, adhesion) = referent(&bac, organisation).await;

    // Un seul référent : le retrait est refusé.
    let issue = membership::revoke(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        adhesion,
    )
    .await
    .expect("la révocation ne produit pas d'erreur");

    assert!(
        matches!(issue, RevokeOutcome::LastManager),
        "l'organisation n'aurait plus aucun référent : {issue:?}"
    );

    let inchangee = org::repo::memberships::by_id(bac.pool(), adhesion)
        .await
        .expect("lecture")
        .expect("l'adhésion");
    assert_eq!(inchangee.status, MembershipStatus::Active);

    // Un remplaçant est désigné : le retrait passe.
    let awa = personne(&bac, "awa.diallo@ifdd.francophonie.org", "Awa", "Diallo").await;
    ajouter_referent(&bac, organisation, awa).await;

    let issue = membership::revoke(
        &bac.state,
        &bac.ctx().with_actor(chef),
        PersonId(chef),
        adhesion,
    )
    .await
    .expect("révocation");

    assert!(matches!(issue, RevokeOutcome::Revoked));

    let apres = org::repo::memberships::by_id(bac.pool(), adhesion)
        .await
        .expect("lecture")
        .expect("l'adhésion survit à sa révocation");
    assert_eq!(apres.status, MembershipStatus::Revoked);
    assert!(apres.revoked_at.is_some());
    assert!(!apres.is_primary, "la primauté retombe avec l'adhésion");
}

/// Le compte d'administration du semis est référent de l'IFDD : les tests qui
/// éprouvent la règle du dernier référent le retirent d'abord, sinon il en reste
/// toujours un et la règle ne joue jamais.
async fn retirer_le_referent_du_semis(bac: &Bac, organisation: Uuid) {
    sqlx::query!(
        "UPDATE org.memberships SET status = 'revoked', revoked_at = now(), is_primary = false
          WHERE organization_id = $1 AND role = 'manager' AND status = 'active'",
        organisation
    )
    .execute(bac.pool())
    .await
    .expect("retrait du référent du semis");
}

/// **Un administrateur passe outre.** La permission de gestion des
/// organisations le porte, et le refus n'est pas fait pour lui.
#[tokio::test]
async fn un_administrateur_force_le_retrait_du_dernier_referent() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    retirer_le_referent_du_semis(&bac, organisation).await;
    let (_, adhesion) = referent(&bac, organisation).await;

    let admin = personne(
        &bac,
        "admin.plateforme@ifdd.francophonie.org",
        "Ada",
        "Admin",
    )
    .await;
    attribuer(&bac, admin, "super_admin", "global", None).await;

    let issue = membership::revoke(
        &bac.state,
        &bac.ctx().with_actor(admin),
        PersonId(admin),
        adhesion,
    )
    .await
    .expect("révocation forcée");

    assert!(
        matches!(issue, RevokeOutcome::Revoked),
        "l'administrateur passe outre : {issue:?}"
    );

    let restants = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.memberships
            WHERE organization_id = $1 AND role = 'manager' AND status = 'active'"#,
        organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(restants, 0);
}

/// Qui n'est ni la personne concernée, ni référent, ni administrateur, ne
/// retire personne.
#[tokio::test]
async fn un_tiers_ne_retire_personne() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let (_, adhesion) = referent(&bac, organisation).await;

    let karim = personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;

    let refus = membership::revoke(
        &bac.state,
        &bac.ctx().with_actor(karim),
        PersonId(karim),
        adhesion,
    )
    .await
    .expect_err("un tiers ne retire personne");

    assert_eq!(refus.code, kernel::error::ErrorCode::OrgNotManager);
}

/// **Une personne quitte son organisation elle-même**, sans être référente : la
/// règle du dernier référent ne la concerne pas.
#[tokio::test]
async fn une_personne_quitte_son_organisation() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;

    let awa = personne(&bac, "awa.diallo@ifdd.francophonie.org", "Awa", "Diallo").await;
    let sienne = sqlx::query_scalar!(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, approved_at)
         VALUES ($1, $2, 'member', 'active', now())
         RETURNING id",
        organisation,
        awa
    )
    .fetch_one(bac.pool())
    .await
    .expect("adhésion");

    let issue = membership::revoke(
        &bac.state,
        &bac.ctx().with_actor(awa),
        PersonId(awa),
        MembershipId(sienne),
    )
    .await
    .expect("départ");

    assert!(matches!(issue, RevokeOutcome::Revoked));

    // Le motif du départ voyage dans l'événement : un départ ne se lit pas comme
    // un retrait.
    let charge = sqlx::query_scalar!(
        r#"SELECT payload AS "payload!" FROM platform.outbox_events
            WHERE event_type = 'org.membership.revoked' ORDER BY occurred_at DESC LIMIT 1"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'outbox");
    assert_eq!(charge["cause"], "left");
}
