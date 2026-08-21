//! **Placer, déplacer, redimensionner, retirer** — et la règle qui gouverne tout
//! l'écran : *les chevauchements ne sont jamais bloqués*.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::{EventId, SessionId};
use programme::domain::sessions::PlannerMutationResult;
use programme::domain::transitions::ProposalStatus;
use programme::service::{planner, transition};
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
    .expect("l'acceptation aboutit");

    seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id
}

fn conflits_de(resultat: &PlannerMutationResult, nature: &str) -> usize {
    resultat
        .conflicts
        .iter()
        .filter(|c| c.conflict_kind == nature)
        .count()
}

/// 🔴 **Deux séances superposées en salle physique : l'écriture ABOUTIT**, et le
/// conflit remonte. C'est le contrat le plus important du module.
#[tokio::test]
async fn deux_seances_superposees_saccrivent_et_le_conflit_remonte() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;

    let premiere = seance(&bac, &terrain, "Première", "premiere").await;
    let seconde = seance(&bac, &terrain, "Seconde", "seconde").await;

    seances::placer(
        &bac,
        terrain.edition,
        premiere,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await
    .expect("la première se place");

    // La seconde salle physique : deux salles distinctes, donc le conflit de
    // STAND — l'IFDD n'en tient qu'un.
    let seconde_salle = seances::salle(
        &bac,
        seances::lieu(&bac, terrain.edition).await,
        "annexe",
        "Annexe",
        false,
    )
    .await;

    let resultat = seances::placer(
        &bac,
        terrain.edition,
        seconde,
        Some(seconde_salle),
        "15:00",
        "16:00",
    )
    .await
    .expect("🔴 aucun chevauchement ne refuse une écriture");

    assert_eq!(
        conflits_de(&resultat, "venue_capacity"),
        1,
        "le stand unique de l'édition"
    );
    assert!(resultat.conflicts.iter().any(|c| c.severity == "blocking"));
}

/// Une salle **virtuelle** n'occupe aucun mètre carré du pavillon : elle ne
/// produit aucun conflit de stand. C'est la correction du 18/08, sans laquelle
/// une alerte qu'on apprend à ignorer cesse d'être une alerte.
#[tokio::test]
async fn une_salle_virtuelle_noccupe_pas_le_stand() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;

    let physique = seance(&bac, &terrain, "En salle", "en-salle").await;
    let virtuelle = seance(&bac, &terrain, "En ligne", "en-ligne").await;

    seances::placer(
        &bac,
        terrain.edition,
        physique,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();

    let resultat = seances::placer(
        &bac,
        terrain.edition,
        virtuelle,
        Some(grille.salle_virtuelle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();

    assert_eq!(conflits_de(&resultat, "venue_capacity"), 0);
    assert_eq!(conflits_de(&resultat, "room"), 0);
}

/// **Une séance sans salle n'occupe rien** : c'est l'état normal d'une activité
/// retenue mais pas encore installée, et le panneau ne doit pas saturer le
/// bandeau avant que l'arbitrage ait commencé.
#[tokio::test]
async fn une_seance_sans_salle_ne_produit_aucun_conflit() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;

    let placee = seance(&bac, &terrain, "Placée", "placee").await;
    let a_placer = seance(&bac, &terrain, "À placer", "a-placer").await;

    seances::placer(
        &bac,
        terrain.edition,
        placee,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();

    let resultat = seances::placer(&bac, terrain.edition, a_placer, None, "14:00", "15:30")
        .await
        .unwrap();

    // **Aucun conflit de LIEU** : la séance n'occupe ni le stand ni une salle.
    // L'avertissement « organisation programmée deux fois » subsiste, et c'est
    // exact — les deux séances sont portées par la même organisation, qui ne
    // peut être à deux endroits à la fois. Ce que la règle écarte, c'est le
    // conflit MATÉRIEL d'une séance qui n'occupe rien.
    assert_eq!(conflits_de(&resultat, "venue_capacity"), 0);
    assert_eq!(conflits_de(&resultat, "room"), 0);
}

/// Deux séances dans la **même** salle physique remontent **une seule fois**,
/// par le conflit qui nomme la salle : sans l'exclusion posée dans la fonction,
/// chaque double réservation était comptée deux fois.
#[tokio::test]
async fn la_meme_salle_remonte_une_seule_fois_en_nommant_la_salle() {
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
    let resultat = seances::placer(
        &bac,
        terrain.edition,
        b,
        Some(grille.salle),
        "15:00",
        "16:00",
    )
    .await
    .unwrap();

    assert_eq!(conflits_de(&resultat, "room"), 1, "la salle est nommée");
    assert_eq!(
        conflits_de(&resultat, "venue_capacity"),
        0,
        "et le stand ne le redit pas"
    );
}

/// Retirer la salle **renvoie la séance au panneau** : elle existe toujours, son
/// créneau est intact, et rien n'est supprimé.
#[tokio::test]
async fn retirer_la_salle_renvoie_au_panneau_sans_rien_supprimer() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    seances::placer(
        &bac,
        terrain.edition,
        id,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();

    let resultat = seances::placer(&bac, terrain.edition, id, None, "14:00", "15:30")
        .await
        .unwrap();

    assert_eq!(resultat.session.room_id, None);

    let relue = seances::seance(&bac, id).await;
    assert_eq!(relue.room_id, None);
    assert_eq!(
        relue.debut_mural, "2027-11-12T14:00",
        "le créneau est intact"
    );
    assert_eq!(relue.status, "planned", "et rien n'est supprimé");
}

/// 🔴 **L'écart n° 113** : une séance déplacée du 12 au 14 novembre est
/// rattachée au 14 — vérifié en relisant le jour EN BASE, jamais en croyant la
/// réponse.
#[tokio::test]
async fn une_seance_deplacee_change_de_jour_de_rattachement() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    seances::placer(
        &bac,
        terrain.edition,
        id,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();

    let le_12 = seances::jour_de_rattachement(&bac, id)
        .await
        .expect("la base a déduit le jour");
    assert_eq!(le_12, time::macros::date!(2027 - 11 - 12));

    seances::placer_le(
        &bac,
        terrain.edition,
        id,
        Some(grille.salle),
        "2027-11-14",
        "14:00",
        "15:30",
    )
    .await
    .unwrap();

    let le_14 = seances::jour_de_rattachement(&bac, id)
        .await
        .expect("le jour est redéduit");
    assert_eq!(
        le_14,
        time::macros::date!(2027 - 11 - 14),
        "sans la mise à nul, la séance resterait rangée au 12, en silence"
    );
}

/// Une journée **explicitement fournie** est retenue telle quelle : le régime
/// de cette colonne est « déduite quand elle n'est pas fournie, saisissable
/// sinon ».
#[tokio::test]
async fn une_journee_explicitement_fournie_est_retenue() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    // Le premier jour de l'édition, alors que le créneau est le 12 : c'est
    // volontaire, et c'est ce que « saisissable » veut dire.
    let premier_jour = grille.jours[0];
    let debut = seances::instant_local(&bac, terrain.edition, "2027-11-12 14:00").await;
    let fin = seances::instant_local(&bac, terrain.edition, "2027-11-12 15:30").await;

    planner::placer(
        &bac.state,
        &bac.ctx(),
        EventId(terrain.edition),
        SessionId(id),
        planner::ScheduleSessionPayload {
            session_id: Some(id),
            room_id: Some(grille.salle),
            starts_at: debut,
            ends_at: fin,
            event_day_id: Some(premier_jour),
            time_range: None,
            enforce_room_exclusivity: None,
        },
    )
    .await
    .expect("la journée fournie est acceptée");

    let relue = seances::seance(&bac, id).await;
    assert_eq!(relue.event_day_id, Some(premier_jour));
}

/// **Deux refus nommant leur champ**, et la séance inchangée après chacun.
#[tokio::test]
async fn les_deux_champs_derives_sont_refuses_en_se_nommant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    let debut = seances::instant_local(&bac, terrain.edition, "2027-11-12 14:00").await;
    let fin = seances::instant_local(&bac, terrain.edition, "2027-11-12 15:30").await;

    let charge = |time_range: Option<serde_json::Value>, exclusivite: Option<bool>| {
        planner::ScheduleSessionPayload {
            session_id: Some(id),
            room_id: Some(grille.salle),
            starts_at: debut,
            ends_at: fin,
            event_day_id: None,
            time_range,
            enforce_room_exclusivity: exclusivite,
        }
    };

    for (charge, champ) in [
        (
            charge(Some(serde_json::json!("[2027-11-12,2027-11-13)")), None),
            "time_range",
        ),
        (charge(None, Some(true)), "enforce_room_exclusivity"),
    ] {
        let erreur = planner::placer(
            &bac.state,
            &bac.ctx(),
            EventId(terrain.edition),
            SessionId(id),
            charge,
        )
        .await
        .expect_err("une valeur déduite ne se saisit pas");

        assert_eq!(erreur.code, ErrorCode::SessionDerivedField);
        assert_eq!(erreur.field.as_deref(), Some(champ));

        let relue = seances::seance(&bac, id).await;
        assert_eq!(relue.room_id, None, "et la séance n'a pas bougé");
    }
}

/// Une fin antérieure ou égale au début est refusée **sur son champ**, en
/// français ; une salle d'une autre édition est refusée en le disant.
#[tokio::test]
async fn un_creneau_invalide_et_une_salle_etrangere_sont_refuses() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    let erreur = seances::placer_le(
        &bac,
        terrain.edition,
        id,
        Some(grille.salle),
        "2027-11-12",
        "15:30",
        "14:00",
    )
    .await
    .expect_err("une fin antérieure au début est refusée");
    assert_eq!(erreur.field.as_deref(), Some("ends_at"));

    // Une salle d'une AUTRE édition : ni la base ni aucun déclencheur ne le
    // vérifient — c'est le service, et lui seul.
    let autre = commun::edition_secondaire(&bac).await;
    let salle_etrangere = seances::salle(
        &bac,
        seances::lieu(&bac, autre).await,
        "ailleurs",
        "Ailleurs",
        false,
    )
    .await;

    let erreur = seances::placer(
        &bac,
        terrain.edition,
        id,
        Some(salle_etrangere),
        "14:00",
        "15:30",
    )
    .await
    .expect_err("une salle d'une autre édition est refusée");
    assert_eq!(erreur.code, ErrorCode::SessionUnknownReference);
    assert_eq!(erreur.field.as_deref(), Some("room_id"));
}

/// **La réponse porte les conflits de TOUTE l'édition** : un déplacement résout
/// le conflit d'un bloc situé un autre jour, et le bandeau doit le montrer.
#[tokio::test]
async fn la_reponse_porte_les_conflits_de_toute_ledition() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;

    let a = seance(&bac, &terrain, "Séance A", "seance-a").await;
    let b = seance(&bac, &terrain, "Séance B", "seance-b").await;
    let temoin = seance(&bac, &terrain, "Témoin", "temoin").await;

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

    // Une troisième séance, placée un autre jour : son écriture ne la concerne
    // pas, et pourtant elle voit le conflit du 12.
    let resultat = seances::placer_le(
        &bac,
        terrain.edition,
        temoin,
        Some(grille.salle),
        "2027-11-15",
        "10:00",
        "11:00",
    )
    .await
    .unwrap();
    assert_eq!(conflits_de(&resultat, "room"), 1);

    // Déplacer B résout le conflit, et l'écriture de B le montre.
    let resultat = seances::placer_le(
        &bac,
        terrain.edition,
        b,
        Some(grille.salle),
        "2027-11-13",
        "15:00",
        "16:00",
    )
    .await
    .unwrap();
    assert!(
        resultat.conflicts.is_empty(),
        "le bandeau doit cesser d'afficher un conflit qui n'existe plus"
    );
}

/// Un intervenant attendu à deux endroits et une organisation programmée deux
/// fois remontent en **avertissement** : gênant mais possible, l'équipe juge.
#[tokio::test]
async fn lintervenant_et_lorganisation_remontent_en_avertissement() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;

    // Deux dossiers de la même organisation : le second reprend le premier
    // intervenant, ce qui produit les deux avertissements d'un coup.
    let premier =
        seances::dossier_pret(&bac, &terrain, "Premier", "premier", Souhaits::default()).await;
    let second =
        seances::dossier_pret(&bac, &terrain, "Second", "second", Souhaits::default()).await;

    sqlx::query!(
        "INSERT INTO programme.proposal_speakers (proposal_id, person_id, role)
         VALUES ($1, $2, 'speaker')
         ON CONFLICT DO NOTHING",
        second.id,
        premier.intervenants[0]
    )
    .execute(bac.pool())
    .await
    .expect("le même intervenant sur les deux dossiers");

    for dossier in [premier.id, second.id] {
        transition::tenter(
            &bac.state,
            &bac.ctx(),
            dossier.into(),
            ProposalStatus::Accepted,
            None,
        )
        .await
        .unwrap();
    }

    let a = seances::seances_du_dossier(&bac, premier.id)
        .await
        .remove(0)
        .id;
    let b = seances::seances_du_dossier(&bac, second.id)
        .await
        .remove(0)
        .id;

    // Deux salles VIRTUELLES : aucun conflit matériel ne vient brouiller la
    // mesure — ce sont les deux avertissements qu'on cherche.
    seances::placer(
        &bac,
        terrain.edition,
        a,
        Some(grille.salle_virtuelle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();
    let resultat = seances::placer(
        &bac,
        terrain.edition,
        b,
        Some(grille.salle_virtuelle),
        "14:30",
        "16:00",
    )
    .await
    .unwrap();

    assert_eq!(conflits_de(&resultat, "speaker"), 1);
    assert_eq!(conflits_de(&resultat, "organization"), 1);
    assert!(
        resultat.conflicts.iter().all(|c| c.severity == "warning"),
        "gênant mais possible : jamais bloquant"
    );
}
