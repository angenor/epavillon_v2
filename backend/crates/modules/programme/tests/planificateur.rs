//! **Tout l'écran d'arbitrage en une réponse** — jours, salles, journées
//! spéciales, canaux, séances placées, séances à placer, et les conflits.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use programme::domain::ids::EventId;
use programme::domain::transitions::ProposalStatus;
use programme::service::{planner, transition};
use uuid::Uuid;

async fn retenir(bac: &Bac, dossier: Uuid) {
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .expect("l'acceptation aboutit");
}

/// Une séance née d'un dossier retenu, prête à être arbitrée.
async fn une_seance(bac: &Bac, terrain: &Terrain, titre: &str, slug: &str) -> Uuid {
    let dossier = seances::dossier_pret(bac, terrain, titre, slug, Souhaits::default()).await;
    retenir(bac, dossier.id).await;
    seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id
}

#[tokio::test]
async fn lecran_porte_les_six_listes_et_les_conflits() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    une_seance(&bac, &terrain, "Atelier", "atelier").await;

    let ecran = planner::ecran(bac.pool(), EventId(terrain.edition))
        .await
        .expect("l'écran se compose")
        .expect("l'édition existe");

    assert_eq!(ecran.event_id, terrain.edition);
    assert_eq!(ecran.timezone, commun::FUSEAU_COP31);
    assert_eq!(
        ecran.zone_label.as_deref(),
        Some("Belém"),
        "le nom de la ville hôte nomme le fuseau à l'écran"
    );
    assert_eq!(ecran.programme_published_at, None);

    assert_eq!(ecran.days.len(), grille.jours.len());
    assert_eq!(ecran.rooms.len(), 2, "une salle physique, une virtuelle");
    assert_eq!(ecran.tracks.len(), 1);
    assert!(
        ecran.channels.iter().any(|c| c.id == grille.canal),
        "le canal de l'édition est offert"
    );
    assert_eq!(ecran.unplaced.len(), 1, "la séance naît sans salle");
    assert!(ecran.placed.is_empty());
    assert!(
        ecran.conflicts.is_empty(),
        "une séance seule ne chevauche rien"
    );
}

/// **Une séance sans salle est au panneau, jamais dans la grille**, et
/// réciproquement : c'est la seule chose qui les distingue.
#[tokio::test]
async fn une_seance_sans_salle_est_a_placer_et_jamais_placee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let seance = une_seance(&bac, &terrain, "Atelier", "atelier").await;

    let avant = planner::ecran(bac.pool(), EventId(terrain.edition))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(avant.unplaced.len(), 1);
    assert!(avant.placed.is_empty());

    placer(
        &bac,
        terrain.edition,
        seance,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await;

    let apres = planner::ecran(bac.pool(), EventId(terrain.edition))
        .await
        .unwrap()
        .unwrap();
    assert!(apres.unplaced.is_empty());
    assert_eq!(apres.placed.len(), 1);
}

/// Ce que la carte du panneau affiche vient du dossier, **sans requête
/// supplémentaire** : le numéro, la note, la durée et le créneau souhaités, les
/// contraintes déclarées au dépôt.
#[tokio::test]
async fn une_seance_nee_dun_dossier_porte_ce_que_la_carte_affiche() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Financer l'adaptation",
        "financer",
        Souhaits::default(),
    )
    .await;
    retenir(&bac, dossier.id).await;

    let ecran = planner::ecran(bac.pool(), EventId(terrain.edition))
        .await
        .unwrap()
        .unwrap();
    let carte = &ecran.unplaced[0];

    assert!(
        carte.reference_code.is_some(),
        "le numéro lisible du dossier"
    );
    assert_eq!(carte.requested_duration_minutes, Some(90));
    assert!(carte.preferred_start_at.is_some());
    assert_eq!(
        carte.scheduling_constraints.as_deref(),
        Some("Pas le matin.")
    );
    assert_eq!(
        carte.organization_name.as_deref(),
        Some("Institut de la Francophonie")
    );
    assert_eq!(carte.organization_acronym.as_deref(), Some("IFDD"));
    assert_eq!(carte.speaker_count, 2);
    assert_eq!(
        carte.themes.as_array().map(Vec::len),
        Some(2),
        "les thématiques, libellé et couleur compris"
    );
}

/// Une édition **sans aucune séance** répond avec ses listes vides, jamais par
/// une erreur.
#[tokio::test]
async fn une_edition_sans_seance_rend_des_listes_vides() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let ecran = planner::ecran(bac.pool(), EventId(terrain.edition))
        .await
        .unwrap()
        .expect("l'édition existe, même sans séance");

    assert!(ecran.placed.is_empty());
    assert!(ecran.unplaced.is_empty());
    assert!(ecran.conflicts.is_empty());
    assert!(!ecran.days.is_empty(), "les jours du calendrier restent");
}

/// Les lectures séparées rendent **exactement** ce que l'écran porte : deux
/// vérités du même fait finiraient par diverger.
#[tokio::test]
async fn les_lectures_separees_rendent_ce_que_lecran_porte() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;

    let a = une_seance(&bac, &terrain, "Première", "premiere").await;
    let b = une_seance(&bac, &terrain, "Seconde", "seconde").await;
    placer(
        &bac,
        terrain.edition,
        a,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await;
    placer(
        &bac,
        terrain.edition,
        b,
        Some(grille.salle),
        "14:30",
        "16:00",
    )
    .await;

    let ecran = planner::ecran(bac.pool(), EventId(terrain.edition))
        .await
        .unwrap()
        .unwrap();
    let seances = planner::seances(bac.pool(), EventId(terrain.edition))
        .await
        .unwrap();
    let conflits = planner::conflits(bac.pool(), EventId(terrain.edition))
        .await
        .unwrap();

    assert_eq!(seances.len(), ecran.placed.len() + ecran.unplaced.len());
    assert_eq!(conflits.len(), ecran.conflicts.len());
    assert!(
        !conflits.is_empty(),
        "deux blocs superposés dans la même salle"
    );
}

/// Placer une séance, **par le service** : c'est l'écriture qu'on éprouve.
async fn placer(
    bac: &Bac,
    edition: Uuid,
    seance: Uuid,
    salle: Option<Uuid>,
    debut: &str,
    fin: &str,
) {
    seances::placer(bac, edition, seance, salle, debut, fin)
        .await
        .expect("le placement aboutit");
}
