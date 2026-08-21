//! **Les journées spéciales sont composées à la main** — règle métier n° 7.
//!
//! Le rattachement est un choix éditorial de l'IFDD, jamais une déduction faite
//! sur les dates : toutes les activités du 12 novembre ne relèvent pas de la
//! « Journée finance durable ».

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::transitions::ProposalStatus;
use programme::service::transition;
use uuid::Uuid;

async fn seance(bac: &Bac, terrain: &Terrain, titre: &str, slug: &str) -> Uuid {
    let dossier = seances::dossier_pret(bac, terrain, titre, slug, Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id
}

/// **La liste envoyée remplace la précédente**, et la base retient qui a
/// rattaché quoi : il arrive d'avoir à l'expliquer à une organisation qui
/// s'étonne de ne pas y figurer.
#[tokio::test]
async fn la_liste_remplace_la_precedente_et_lauteur_est_retenu() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    let second_fil =
        seances::fil_publie(&bac, terrain.edition, "jeunesse", "Journée jeunesse").await;
    let curatrice = commun::personne(&bac, "curatrice@ifdd.org", "Clara", "Mensah").await;

    seances::rattacher(
        &bac,
        terrain.edition,
        id,
        vec![grille.fil, second_fil],
        Some(curatrice),
    )
    .await
    .expect("le rattachement aboutit");

    let poses = seances::fils_de_la_seance(&bac, id).await;
    assert_eq!(poses.len(), 2);
    assert!(
        poses.iter().all(|(_, auteur)| *auteur == Some(curatrice)),
        "l'acteur vient de la session, jamais de la charge utile"
    );

    // La seconde liste **remplace** : ce qui n'y figure plus est détaché.
    seances::rattacher(&bac, terrain.edition, id, vec![second_fil], Some(curatrice))
        .await
        .unwrap();

    let restants = seances::fils_de_la_seance(&bac, id).await;
    assert_eq!(restants.len(), 1);
    assert_eq!(restants[0].0, second_fil);
}

/// Un fil d'une **autre édition** est refusé avec un code stable et un message
/// français — jamais l'exception brute du déclencheur.
#[tokio::test]
async fn un_fil_dune_autre_edition_est_refuse_par_un_code_stable() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    let autre = commun::edition_secondaire(&bac).await;
    let fil_etranger = seances::fil_publie(&bac, autre, "ailleurs", "Journée d'ailleurs").await;

    let erreur = seances::rattacher(&bac, terrain.edition, id, vec![fil_etranger], None)
        .await
        .expect_err("un fil d'une autre édition ne se rattache pas");

    assert_eq!(erreur.code, ErrorCode::SessionTrackEventMismatch);
    assert!(
        seances::fils_de_la_seance(&bac, id).await.is_empty(),
        "et rien n'est posé"
    );
}

/// La même liste envoyée deux fois laisse le même état, sans doublon.
#[tokio::test]
async fn la_meme_liste_deux_fois_laisse_le_meme_etat() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    for _ in 0..2 {
        seances::rattacher(&bac, terrain.edition, id, vec![grille.fil], None)
            .await
            .unwrap();
    }

    assert_eq!(seances::fils_de_la_seance(&bac, id).await.len(), 1);
}

/// La réponse porte la séance **et** les conflits de l'édition, comme les autres
/// écritures : une seule forme pour les trois gestes, donc une seule occasion de
/// diverger.
#[tokio::test]
async fn la_reponse_porte_la_seance_et_les_conflits() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;

    let a = seance(&bac, &terrain, "Séance A", "seance-a").await;
    let b = seance(&bac, &terrain, "Séance B", "seance-b").await;
    seances::placer(
        &bac,
        terrain.edition,
        a,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();
    seances::placer(
        &bac,
        terrain.edition,
        b,
        Some(grille.salle),
        "15:00",
        "16:00",
    )
    .await
    .unwrap();

    let resultat = seances::rattacher(&bac, terrain.edition, a, vec![grille.fil], None)
        .await
        .unwrap();

    assert_eq!(resultat.session.id, a);
    assert_eq!(resultat.session.track_ids, vec![grille.fil]);
    assert!(
        !resultat.conflicts.is_empty(),
        "les conflits de l'édition accompagnent chaque écriture"
    );
}
