//! **Retenir un dossier fait naître ses séances** — l'écart n° 57.
//!
//! C'est l'histoire qui ouvre le module : sans elle, le planificateur, livré
//! depuis le 18/08, reste devant une grille vide alors que l'équipe vient de
//! décider.

mod commun;

use commun::seances::{self, Souhaits};
use commun::Bac;
use programme::domain::transitions::ProposalStatus;
use programme::service::transition;
use uuid::Uuid;

/// Retenir un dossier, **par le service** : poser l'état à la main ne
/// prouverait rien, c'est précisément l'hameçon qu'on éprouve.
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

/// Les lignes d'outbox d'une séance, tous types confondus. **Les compter est le
/// seul contrôle qui dise quelque chose d'un doublon.**
async fn evenements_de_seance(bac: &Bac, session_id: Uuid) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT event_type FROM platform.outbox_events
          WHERE aggregate_schema = 'programme' AND aggregate_type = 'session'
            AND aggregate_id = $1
          ORDER BY occurred_at, id",
        session_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'outbox")
}

#[tokio::test]
async fn un_dossier_a_une_occurrence_produit_une_seance_a_placer() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Atelier mangroves",
        "atelier-mangroves",
        Souhaits::default(),
    )
    .await;

    retenir(&bac, dossier.id).await;

    let seances = seances::seances_du_dossier(&bac, dossier.id).await;
    assert_eq!(seances.len(), 1, "une occurrence, une séance");

    let seance = &seances[0];
    assert_eq!(seance.status, "planned", "créneau pressenti, non public");
    assert_eq!(
        seance.room_id, None,
        "elle naît SANS salle : c'est le panneau"
    );
    assert_eq!(seance.published_at, None, "rien n'est public à ce stade");
    assert_eq!(seance.timezone, commun::FUSEAU_COP31);
    assert_eq!(
        seance.debut_mural, "2027-11-12T14:00",
        "le créneau souhaité par l'organisation, relu en heure murale"
    );
    assert_eq!(
        seance.fin_murale, "2027-11-12T15:30",
        "quatre-vingt-dix minutes"
    );
    assert_eq!(seance.organization_id, Some(terrain.organisation));
}

/// **Trois séances, et exactement trois lignes d'outbox — pas six.** Le
/// déclencheur émet déjà `programme.session.created` à l'insertion ; un service
/// qui émettrait à son tour produirait deux jeux de rappels par séance, et le
/// doublon ne se verrait qu'en production.
#[tokio::test]
async fn trois_occurrences_produisent_trois_seances_et_trois_evenements() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Cycle de webinaires",
        "cycle-webinaires",
        Souhaits {
            occurrences: 3,
            ..Souhaits::default()
        },
    )
    .await;

    retenir(&bac, dossier.id).await;

    let seances = seances::seances_du_dossier(&bac, dossier.id).await;
    assert_eq!(seances.len(), 3);
    assert_eq!(
        seances
            .iter()
            .map(|s| s.sequence_number)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );

    let adresses: std::collections::HashSet<&str> =
        seances.iter().map(|s| s.slug.as_str()).collect();
    assert_eq!(adresses.len(), 3, "trois adresses distinctes");

    for seance in &seances {
        let emis = evenements_de_seance(&bac, seance.id).await;
        assert_eq!(
            emis,
            vec!["programme.session.created"],
            "UNE ligne d'outbox par séance, celle du déclencheur — jamais deux"
        );
    }
}

#[tokio::test]
async fn la_seance_porte_les_memes_intervenants_organisations_et_thematiques() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Financer l'adaptation",
        "financer-adaptation",
        Souhaits::default(),
    )
    .await;

    retenir(&bac, dossier.id).await;
    let seance = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0);

    let intervenants = seances::intervenants_de_la_seance(&bac, seance.id).await;
    assert_eq!(intervenants.len(), dossier.intervenants.len());

    // **Le porteur est posé par déclencheur**, la co-organisation par le
    // service : la séance en porte donc une de plus que ce que le service écrit.
    let organisations = seances::organisations_de_la_seance(&bac, seance.id).await;
    assert_eq!(organisations.len(), dossier.coorganisations.len() + 1);
    assert_eq!(
        organisations
            .iter()
            .filter(|(_, role)| role == "lead")
            .count(),
        1,
        "un seul porteur, posé par le déclencheur"
    );

    let thematiques = seances::thematiques_de_la_seance(&bac, seance.id).await;
    assert_eq!(thematiques.len(), dossier.themes.len());
    for (schema, table, _) in &thematiques {
        assert_eq!((schema.as_str(), table.as_str()), ("programme", "sessions"));
    }
}

/// Le repli : **le premier jour de l'édition à l'heure d'ouverture quotidienne
/// de l'appel**, composé en base et relu dans le fuseau de l'édition.
#[tokio::test]
async fn un_dossier_sans_creneau_prend_le_premier_jour_a_lheure_de_lappel() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Sans créneau",
        "sans-creneau",
        Souhaits {
            creneau: None,
            ..Souhaits::default()
        },
    )
    .await;

    retenir(&bac, dossier.id).await;
    let seance = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0);

    assert_eq!(
        seance.debut_mural, "2027-11-09T09:00",
        "premier jour de la COP31, heure d'ouverture de l'appel, en heure de Belém"
    );
}

/// Sans durée, celle de l'appel ; **sans appel**, le début de l'édition et
/// soixante minutes — la valeur que le modèle lui-même retient.
#[tokio::test]
async fn les_deux_replis_de_duree_et_le_dossier_sans_appel() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let sans_duree = seances::dossier_pret(
        &bac,
        &terrain,
        "Sans durée",
        "sans-duree",
        Souhaits {
            duree_minutes: None,
            ..Souhaits::default()
        },
    )
    .await;
    retenir(&bac, sans_duree.id).await;
    let seance = seances::seances_du_dossier(&bac, sans_duree.id)
        .await
        .remove(0);
    assert_eq!(seance.debut_mural, "2027-11-12T14:00");
    assert_eq!(
        seance.fin_murale, "2027-11-12T15:00",
        "soixante minutes, durée par défaut de l'appel"
    );

    let sans_appel = seances::dossier_pret(
        &bac,
        &terrain,
        "Sans appel",
        "sans-appel",
        Souhaits {
            creneau: None,
            duree_minutes: None,
            avec_appel: false,
            ..Souhaits::default()
        },
    )
    .await;
    retenir(&bac, sans_appel.id).await;
    let seance = seances::seances_du_dossier(&bac, sans_appel.id)
        .await
        .remove(0);
    assert_eq!(
        seance.debut_mural, "2027-11-09T09:00",
        "le début de l'édition, faute d'appel à interroger"
    );
    assert_eq!(seance.fin_murale, "2027-11-09T10:00", "soixante minutes");
}

/// **L'idempotence tombe d'une contrainte**, jamais d'un décompte : une
/// acceptation rejouée ne double aucune séance.
#[tokio::test]
async fn une_acceptation_rejouee_ne_cree_aucune_seance_de_plus() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Rejouée",
        "rejouee",
        Souhaits {
            occurrences: 2,
            ..Souhaits::default()
        },
    )
    .await;

    retenir(&bac, dossier.id).await;
    let premieres = seances::seances_du_dossier(&bac, dossier.id).await;
    assert_eq!(premieres.len(), 2);

    // Retenu → annulé → … la machine à états n'offre pas de retour direct à
    // « en évaluation » depuis « retenu ». Le chemin réel d'une acceptation
    // rejouée est celui d'une action groupée passée deux fois : on retente.
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .expect("la seconde tentative ne lève pas");

    let secondes = seances::seances_du_dossier(&bac, dossier.id).await;
    assert_eq!(secondes.len(), 2, "aucune séance de plus");
    assert_eq!(
        secondes.iter().map(|s| s.id).collect::<Vec<_>>(),
        premieres.iter().map(|s| s.id).collect::<Vec<_>>(),
        "et ce sont les mêmes"
    );
}

/// Une action groupée retenant douze dossiers crée les séances de chacun, et un
/// dossier écarté n'empêche pas les autres.
#[tokio::test]
async fn une_action_groupee_fait_naitre_les_seances_de_chaque_dossier() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let droits = commun::droits(&bac, &terrain).await;

    let mut retenus = Vec::new();
    for rang in 1..=12 {
        let dossier = seances::dossier_pret(
            &bac,
            &terrain,
            &format!("Dossier {rang}"),
            &format!("dossier-{rang}"),
            Souhaits::default(),
        )
        .await;
        retenus.push(dossier.id);
    }

    // Un treizième dossier resté en brouillon : la transition vers « retenu »
    // ne lui est pas offerte, il ressort en écart.
    let brouillon = commun::dossier(&bac, &terrain, "Brouillon", "brouillon").await;
    let mut selection = retenus.clone();
    selection.push(brouillon);

    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;
    let resultat = transition::changer_en_groupe(
        &bac.state,
        &bac.ctx().with_actor(droits.decideur),
        &perimetre,
        transition::ChangeStatusPayload {
            proposal_ids: selection,
            to_status: ProposalStatus::Accepted,
            reason: None,
        },
    )
    .await
    .expect("l'action groupée aboutit");

    assert_eq!(resultat.applied.len(), 12);
    assert_eq!(resultat.skipped.len(), 1, "le brouillon est écarté, nommé");

    for dossier in retenus {
        assert_eq!(
            seances::seances_du_dossier(&bac, dossier).await.len(),
            1,
            "chaque dossier retenu a sa séance"
        );
    }
    assert!(
        seances::seances_du_dossier(&bac, brouillon)
            .await
            .is_empty(),
        "le dossier écarté n'en a aucune"
    );
}

/// **La reprise v1 ne crée aucune séance** : elle n'écrit pas l'état, donc
/// l'hameçon ne se déclenche pas. Les activités de la v1 sont importées par
/// `910_migration_v1.sql`, pas par ce service.
#[tokio::test]
async fn la_reprise_v1_ne_cree_aucune_seance() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Repris de la v1",
        "repris-v1",
        Souhaits::default(),
    )
    .await;

    // Un dossier déjà retenu, dont le journal ne porte pas la transition : le
    // cas exact que la reprise vient combler.
    sqlx::query!(
        "UPDATE programme.proposals SET status = 'accepted', decided_at = now()
          WHERE id = $1",
        dossier.id
    )
    .execute(bac.pool())
    .await
    .expect("acceptation posée à la main");

    sqlx::query!(
        "DELETE FROM programme.sessions WHERE proposal_id = $1",
        dossier.id
    )
    .execute(bac.pool())
    .await
    .expect("remise à zéro des séances");

    programme::service::backfill::deduire(&bac.state, &bac.ctx())
        .await
        .expect("la reprise aboutit");

    assert!(
        seances::seances_du_dossier(&bac, dossier.id)
            .await
            .is_empty(),
        "la reprise journalise, elle ne programme pas"
    );
}

/// **Corriger un dossier retenu ne touche aucune séance** — la garantie de B4,
/// que ce module n'affaiblit pas. Une séance arbitrée par l'équipe ne se
/// redéplace pas parce que l'organisation a corrigé son titre.
#[tokio::test]
async fn corriger_un_dossier_retenu_ne_deplace_pas_sa_seance() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Titre initial",
        "titre-initial",
        Souhaits::default(),
    )
    .await;

    retenir(&bac, dossier.id).await;
    let avant = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0);

    sqlx::query!(
        r#"UPDATE programme.proposals
              SET title = '{"fr":"Titre corrigé"}'::jsonb,
                  preferred_start_at = preferred_start_at + interval '2 days',
                  duration_minutes = 45,
                  format = 'online'
            WHERE id = $1"#,
        dossier.id
    )
    .execute(bac.pool())
    .await
    .expect("correction du dossier");

    let apres = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0);

    assert_eq!(apres.id, avant.id);
    assert_eq!(
        apres.titre_fr, avant.titre_fr,
        "le titre de la séance ne suit pas"
    );
    assert_eq!(apres.debut_mural, avant.debut_mural, "ni son créneau");
    assert_eq!(apres.fin_murale, avant.fin_murale);
    assert_eq!(apres.format, avant.format);
    assert_eq!(apres.slug, avant.slug);
}
