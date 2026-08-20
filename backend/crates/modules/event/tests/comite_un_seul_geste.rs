//! **Ajouts, retraits et plafonds d'un seul geste** (FR-069 à FR-073).
//!
//! L'écran envoie la composition complète : ce qui n'y figure plus est retiré.
//! Un ajout et un retrait séparés laisseraient exister, entre les deux, un
//! comité que personne n'a voulu — et si le second échouait, il resterait.
//!
//! Trois choses se vérifient ici :
//!
//! - la **charge utile dédoublonnée par le service**, jamais remontée comme
//!   erreur de base : la clé primaire `(call_id, person_id)` ne doit jamais se
//!   plaindre ;
//! - une **personne inconnue refusée en la nommant** — la clé étrangère
//!   refuserait aussi, mais sans dire laquelle des lignes est en cause ;
//! - les membres retirés **portant encore des dossiers**, nommés dans la
//!   réponse : un retrait silencieux laisse des dossiers sans lecteur à trois
//!   jours de la décision.

mod commun;

use commun::{formulaire_appel, Bac};
use event::domain::ids::{CallId, EventId};
use event::domain::tabs::{CommitteePayload, CommitteeSeat};
use event::service::{call as service_appel, committee as service_comite};
use kernel::error::ErrorCode;
use uuid::Uuid;

/// L'appel de la COP31, prêt à recevoir un comité.
async fn appel(bac: &Bac, event_id: Uuid) -> Uuid {
    let acteur = commun::auteur(bac).await;
    service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        EventId::from(event_id),
        formulaire_appel(event_id, "cop31"),
    )
    .await
    .expect("création de l'appel")
    .call
    .expect("l'appel créé")
    .id
}

fn composition(sieges: Vec<CommitteeSeat>) -> CommitteePayload {
    CommitteePayload {
        call_id: None,
        members: sieges,
    }
}

fn siege(person_id: Uuid, responsable: bool, plafond: Option<i16>) -> CommitteeSeat {
    CommitteeSeat {
        person_id,
        is_lead: responsable,
        workload_cap: plafond,
    }
}

#[tokio::test]
async fn ajouts_retraits_et_plafonds_partent_ensemble() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let call_id = CallId::from(appel(&bac, editions.cop31).await);
    let cop31 = EventId::from(editions.cop31);

    let anne = commun::personne(&bac, "anne@ifdd.francophonie.org", "Anne", "Rivard").await;
    let bakary = commun::personne(&bac, "bakary@ifdd.francophonie.org", "Bakary", "Sow").await;
    let claire = commun::personne(&bac, "claire@ifdd.francophonie.org", "Claire", "Dubois").await;

    let premier = service_comite::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        call_id,
        composition(vec![
            siege(anne, true, Some(10)),
            siege(bakary, false, None),
        ]),
    )
    .await
    .expect("l'enregistrement aboutit");

    assert!(premier.ok);
    assert_eq!(premier.members.len(), 2);
    let anne_siege = premier
        .members
        .iter()
        .find(|m| m.person_id == anne)
        .unwrap();
    assert!(anne_siege.is_lead);
    assert_eq!(anne_siege.workload_cap, Some(10));

    // Un seul geste : Bakary sort, Claire entre, le plafond d'Anne change.
    let second = service_comite::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        call_id,
        composition(vec![
            siege(anne, false, Some(20)),
            siege(claire, true, None),
        ]),
    )
    .await
    .expect("l'enregistrement aboutit");

    let presents: Vec<Uuid> = second.members.iter().map(|m| m.person_id).collect();
    assert_eq!(presents.len(), 2);
    assert!(presents.contains(&anne) && presents.contains(&claire));
    assert!(
        !presents.contains(&bakary),
        "ce qui ne figure plus dans la liste est retiré"
    );

    let anne_siege = second.members.iter().find(|m| m.person_id == anne).unwrap();
    assert!(!anne_siege.is_lead, "le rôle de responsable a changé");
    assert_eq!(anne_siege.workload_cap, Some(20));
}

/// **Un doublon de charge utile est dédoublonné par le service.** La clé
/// primaire ne doit jamais se plaindre : ce serait un message technique pour une
/// situation que le service sait résoudre seul.
#[tokio::test]
async fn un_doublon_de_charge_utile_ne_fait_quun_siege() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let call_id = CallId::from(appel(&bac, editions.cop31).await);

    let anne = commun::personne(&bac, "anne@ifdd.francophonie.org", "Anne", "Rivard").await;

    let resultat = service_comite::enregistrer(
        &bac.state,
        &bac.ctx(),
        EventId::from(editions.cop31),
        call_id,
        composition(vec![
            siege(anne, false, Some(5)),
            siege(anne, true, Some(12)),
        ]),
    )
    .await
    .expect("le doublon ne fait pas échouer l'enregistrement");

    assert_eq!(resultat.members.len(), 1);
    assert!(resultat.members[0].is_lead, "la dernière ligne l'emporte");
    assert_eq!(resultat.members[0].workload_cap, Some(12));
}

/// **Une personne inconnue est refusée en la nommant.**
#[tokio::test]
async fn une_personne_inconnue_est_refusee_en_la_nommant() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let call_id = CallId::from(appel(&bac, editions.cop31).await);

    let fantome = Uuid::now_v7();

    let refus = service_comite::enregistrer(
        &bac.state,
        &bac.ctx(),
        EventId::from(editions.cop31),
        call_id,
        composition(vec![siege(fantome, false, None)]),
    )
    .await
    .expect_err("une personne inconnue est refusée");

    assert_eq!(refus.code, ErrorCode::EventUnknownReference);
    assert_eq!(refus.field.as_deref(), Some("person_id"));
    assert!(
        refus.message.contains(&fantome.to_string()),
        "le refus doit NOMMER la personne en cause : {}",
        refus.message
    );

    let sieges = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM event.call_reviewers WHERE call_id = $1"#,
        call_id.as_uuid()
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(sieges, 0, "et rien n'a été écrit");
}

/// **Un membre retiré qui portait des dossiers est nommé** — et ses revues déjà
/// rendues restent au dossier.
#[tokio::test]
async fn un_membre_retire_portant_des_dossiers_est_nomme() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;
    let call_id = CallId::from(enfants.appel);
    let cop31 = EventId::from(editions.cop31);

    let organisation = commun::seed::dossiers(&bac, editions.cop31, enfants.appel).await;
    let _ = organisation;

    let evaluateur =
        commun::personne(&bac, "evaluatrice@ifdd.francophonie.org", "Rita", "Mendes").await;

    service_comite::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        call_id,
        composition(vec![siege(evaluateur, false, None)]),
    )
    .await
    .expect("ajout");

    // Un dossier lui est confié.
    let dossier = sqlx::query_scalar!(
        "SELECT id FROM programme.proposals WHERE call_id = $1 AND status <> 'draft'",
        enfants.appel
    )
    .fetch_one(bac.pool())
    .await
    .expect("le dossier déposé");

    sqlx::query!(
        "INSERT INTO programme.review_assignments (proposal_id, reviewer_id) VALUES ($1, $2)",
        dossier,
        evaluateur
    )
    .execute(bac.pool())
    .await
    .expect("affectation");

    let retrait = service_comite::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        call_id,
        composition(Vec::new()),
    )
    .await
    .expect("le retrait aboutit");

    assert!(retrait.members.is_empty());
    assert_eq!(
        retrait.removed_with_assignments.len(),
        1,
        "le membre retiré portait un dossier : il faut le dire"
    );
    assert_eq!(retrait.removed_with_assignments[0].full_name, "Rita Mendes");
    assert_eq!(retrait.removed_with_assignments[0].assigned_count, 1);

    // **L'affectation survit** : le retrait du comité n'annule rien.
    let affectations = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.review_assignments WHERE reviewer_id = $1"#,
        evaluateur
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(affectations, 1);
}
