//! **Refuser, redemander, refuser encore — jamais plus d'une ligne** (SC-008).
//!
//! `ux_memberships (organization_id, person_id)` **ne connaît pas le statut** :
//! une adhésion révoquée occupe la place. Lire puis écrire laisserait une fenêtre
//! où deux demandes simultanées produiraient une violation de contrainte au lieu
//! d'une réponse propre. La demande est donc un unique ordre avec reprise
//! conditionnelle, et c'est la base qui tranche (écart n° 72).

mod commun;

use commun::{ifdd, personne, Bac};
use org::domain::ids::{MembershipId, OrganizationId, PersonId};
use org::domain::membership::{DecideMembership, JoinOrganization, JoinOutcome, MembershipStatus};
use org::service::{join, membership};
use uuid::Uuid;

async fn rejoindre(bac: &Bac, qui: Uuid, organisation: Uuid) -> JoinOutcome {
    join::join(
        &bac.state,
        &bac.ctx().with_actor(qui),
        PersonId(qui),
        OrganizationId(organisation),
        JoinOrganization {
            organization_id: None,
            job_title: Some("Chargée de projet".to_owned()),
        },
    )
    .await
    .expect("demande de rattachement")
}

async fn lignes(bac: &Bac, organisation: Uuid, qui: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.memberships
            WHERE organization_id = $1 AND person_id = $2"#,
        organisation,
        qui
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage")
}

#[tokio::test]
async fn refuser_redemander_refuser_ne_laisse_quune_seule_ligne() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;

    // Le référent : le compte d'administration du semis l'est déjà.
    let referent = sqlx::query_scalar!(
        "SELECT person_id FROM org.memberships
          WHERE organization_id = $1 AND role = 'manager' AND status = 'active'",
        organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("le référent du semis");

    // Une adresse **hors** des deux domaines de l'IFDD : sans quoi le
    // rattachement serait automatique et il n'y aurait rien à décider.
    let karim = personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;

    for tour in 1..=3 {
        let issue = rejoindre(&bac, karim, organisation).await;
        let adhesion_id = match issue {
            JoinOutcome::Pending { membership_id, .. } => membership_id,
            autre => panic!("tour {tour} — issue inattendue : {autre:?}"),
        };

        assert_eq!(
            lignes(&bac, organisation, karim).await,
            1,
            "tour {tour} — jamais plus d'une ligne par (organisation, personne)"
        );

        // Le référent refuse : la ligne est **révoquée**, jamais supprimée.
        membership::decide(
            &bac.state,
            &bac.ctx().with_actor(referent),
            PersonId(referent),
            MembershipId(adhesion_id.as_uuid()),
            DecideMembership {
                membership_id: None,
                approved: false,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("tour {tour} — refus : {e}"));

        let apres = org::repo::memberships::by_id(bac.pool(), adhesion_id)
            .await
            .expect("lecture")
            .expect("l'adhésion survit au refus");
        assert_eq!(apres.status, MembershipStatus::Revoked);
        assert!(apres.revoked_at.is_some());
    }

    assert_eq!(
        lignes(&bac, organisation, karim).await,
        1,
        "après trois allers-retours, toujours une seule ligne"
    );
}

/// Ce que la reprise remet à zéro, et ce qu'elle ne touche pas. La date de
/// création reste celle de la **première** demande : l'histoire de l'adhésion se
/// lit dans le journal d'audit, pas dans une ligne réécrite.
#[tokio::test]
async fn la_reprise_efface_la_revocation_et_garde_la_date_de_creation() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let referent = sqlx::query_scalar!(
        "SELECT person_id FROM org.memberships
          WHERE organization_id = $1 AND role = 'manager' AND status = 'active'",
        organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("le référent");

    let karim = personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;

    let premiere = match rejoindre(&bac, karim, organisation).await {
        JoinOutcome::Pending { membership_id, .. } => membership_id,
        autre => panic!("issue inattendue : {autre:?}"),
    };
    let creee_le = org::repo::memberships::by_id(bac.pool(), premiere)
        .await
        .expect("lecture")
        .expect("l'adhésion")
        .created_at;

    membership::decide(
        &bac.state,
        &bac.ctx().with_actor(referent),
        PersonId(referent),
        MembershipId(premiere.as_uuid()),
        DecideMembership {
            membership_id: None,
            approved: false,
        },
    )
    .await
    .expect("refus");

    rejoindre(&bac, karim, organisation).await;

    let reprise = org::repo::memberships::by_id(bac.pool(), premiere)
        .await
        .expect("lecture")
        .expect("la même ligne");

    assert_eq!(reprise.status, MembershipStatus::Pending);
    assert!(reprise.revoked_at.is_none(), "la révocation est effacée");
    assert_eq!(
        reprise.created_at, creee_le,
        "la date de création est celle de la première demande"
    );
}

/// Une adhésion **vivante** ne bouge pas : la réponse est « déjà membre », et
/// elle porte l'état de l'adhésion existante.
#[tokio::test]
async fn une_adhesion_vivante_rend_deja_membre() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let karim = personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;

    rejoindre(&bac, karim, organisation).await;
    let seconde = rejoindre(&bac, karim, organisation).await;

    match seconde {
        JoinOutcome::AlreadyMember {
            membership_status, ..
        } => assert_eq!(membership_status, MembershipStatus::Pending),
        autre => panic!("issue inattendue : {autre:?}"),
    }

    assert_eq!(lignes(&bac, organisation, karim).await, 1);
}

/// Cent demandes simultanées de la même personne : **une seule ligne**, et
/// aucune violation de contrainte. C'est la base qui tranche, pas une lecture
/// suivie d'une écriture.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cent_demandes_simultanees_ne_produisent_quune_ligne() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let karim = personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;

    let mut taches = Vec::with_capacity(100);
    for _ in 0..100 {
        let state = bac.state.clone();
        let ctx = bac.ctx().with_actor(karim);
        taches.push(tokio::spawn(async move {
            join::join(
                &state,
                &ctx,
                PersonId(karim),
                OrganizationId(organisation),
                JoinOrganization {
                    organization_id: None,
                    job_title: Some("Chargée de projet".to_owned()),
                },
            )
            .await
        }));
    }

    let mut ouvertes = 0;
    for tache in taches {
        match tache.await.expect("la tâche ne panique pas") {
            Ok(JoinOutcome::Pending { .. }) | Ok(JoinOutcome::Joined { .. }) => ouvertes += 1,
            Ok(JoinOutcome::AlreadyMember { .. }) => {}
            Err(e) => panic!("aucune demande ne doit échouer sur une contrainte : {e}"),
        }
    }

    assert_eq!(ouvertes, 1, "une seule demande ouvre l'adhésion");
    assert_eq!(lignes(&bac, organisation, karim).await, 1);
}
