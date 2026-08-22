//! **L'organisation sait combien de personnes viendront, jamais qui** — les
//! écarts n° 36 et n° 108.

mod commun;

use commun::seances::{self, Souhaits};
use commun::Bac;
use programme::domain::transitions::ProposalStatus;
use programme::service::{transition, workspace};

/// Les trois nombres sont exacts, **relus en base** ; un dossier **non retenu**
/// porte une liste vide, jamais absente.
#[tokio::test]
async fn les_trois_nombres_sont_exacts_et_un_dossier_non_retenu_est_vide() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let retenu = seances::dossier_pret(
        &bac,
        &terrain,
        "Atelier retenu",
        "atelier-retenu",
        Souhaits::default(),
    )
    .await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        retenu.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();

    // Un second dossier, resté en évaluation : sa liste doit être vide.
    let en_cours = seances::dossier_pret(
        &bac,
        &terrain,
        "En évaluation",
        "en-evaluation",
        Souhaits::default(),
    )
    .await;

    let seance = seances::seances_du_dossier(&bac, retenu.id)
        .await
        .remove(0)
        .id;
    seances::ouvrir_les_inscriptions(&bac, seance, Some(2), true).await;

    // Deux confirmées, une en attente : trois nombres distincts, pour qu'aucun
    // ne puisse se faire passer pour un autre.
    for rang in 0..3 {
        let personne = commun::personne(
            &bac,
            &format!("inscrit{rang}@example.org"),
            "Inscrit",
            &format!("Numéro{rang}"),
        )
        .await;
        seances::sinscrire(&bac, seance, Some(personne), seances::reponses_valides())
            .await
            .unwrap();
    }

    let espace = workspace::espace(&bac.state, terrain.deposante, terrain.organisation)
        .await
        .expect("l'espace de l'organisation");

    let suivi_retenu = espace
        .proposals
        .iter()
        .find(|p| p.proposal.id == retenu.id)
        .expect("le dossier retenu");
    assert_eq!(suivi_retenu.sessions.len(), 1);

    let suivie = &suivi_retenu.sessions[0];
    assert_eq!(
        suivie.registered_count, 2,
        "confirmées : registered ET attended"
    );
    assert_eq!(suivie.waitlisted_count, 1);
    assert_eq!(suivie.capacity, Some(2));
    assert!(
        suivie.reminders.is_empty(),
        "aucune règle de rappel ne s'applique : liste vide, jamais absente"
    );
    assert!(suivie.room.is_none(), "la séance n'est pas encore placée");

    let suivi_en_cours = espace
        .proposals
        .iter()
        .find(|p| p.proposal.id == en_cours.id)
        .expect("le dossier en évaluation");
    assert!(
        suivi_en_cours.sessions.is_empty(),
        "vide tant que le dossier n'est pas retenu — jamais absente"
    );
}

/// 🔴 **Balayage de la charge utile ENTIÈRE** : ni nom d'inscrit, ni adresse, ni
/// valeur de réponse au formulaire n'y figurent. Cherchés **dans la réponse
/// sérialisée**, et non champ par champ — c'est ainsi qu'on attrape ce qu'on
/// n'avait pas soupçonné.
#[tokio::test]
async fn aucun_nom_dinscrit_ne_sort_vers_lorganisation() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier =
        seances::dossier_pret(&bac, &terrain, "Atelier", "atelier", Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();

    let seance = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0)
        .id;
    seances::ouvrir_les_inscriptions(&bac, seance, None, false).await;

    // Une inscrite dont le nom, l'adresse et une réponse sont **reconnaissables**
    // dans un texte : c'est ce qui rend le balayage concluant.
    let inscrite = commun::personne(
        &bac,
        "zoubeida-temoin@example.org",
        "Zoubeïda",
        "Ranaivoson",
    )
    .await;
    seances::sinscrire(
        &bac,
        seance,
        Some(inscrite),
        programme::service::registration::RegisterPayload {
            answers: serde_json::json!({
                "country": "SN",
                "job_title": "Directrice-du-temoin-unique"
            }),
            ..seances::reponses_valides()
        },
    )
    .await
    .unwrap();

    let espace = workspace::espace(&bac.state, terrain.deposante, terrain.organisation)
        .await
        .unwrap();
    let charge = serde_json::to_string(&espace).expect("sérialisation");

    for temoin in [
        "Zoubeïda",
        "Ranaivoson",
        "zoubeida-temoin@example.org",
        "Directrice-du-temoin-unique",
        &inscrite.to_string(),
    ] {
        assert!(
            !charge.contains(temoin),
            "« {temoin} » ne doit pas franchir la frontière : le filtrage est à la source"
        );
    }

    // Et le décompte, lui, est bien là.
    let suivi = &espace.proposals[0];
    assert_eq!(suivi.sessions[0].registered_count, 1);
}

/// Une séance **terminée sans compte rendu** produit l'action correspondante ;
/// une séance à venir n'en produit pas.
#[tokio::test]
async fn une_seance_terminee_sans_compte_rendu_produit_son_action() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Atelier terminé",
        "atelier-termine",
        Souhaits::default(),
    )
    .await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    let seance = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0)
        .id;

    // À venir : aucune action.
    let espace = workspace::espace(&bac.state, terrain.deposante, terrain.organisation)
        .await
        .unwrap();
    assert!(
        !espace
            .actions
            .iter()
            .any(|a| a.kind == "session_report_missing"),
        "une séance à venir ne réclame aucun compte rendu"
    );

    // Terminée, sans compte rendu.
    sqlx::query!(
        "UPDATE programme.sessions
            SET starts_at = now() - interval '2 days',
                ends_at = now() - interval '2 days' + interval '90 minutes'
          WHERE id = $1",
        seance
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let espace = workspace::espace(&bac.state, terrain.deposante, terrain.organisation)
        .await
        .unwrap();
    let action = espace
        .actions
        .iter()
        .find(|a| a.kind == "session_report_missing")
        .expect("l'action est produite");

    assert_eq!(action.proposal_id, Some(dossier.id));
    assert_eq!(
        action.subject, "Atelier terminé",
        "elle nomme la séance : une organisation à trois occurrences doit savoir laquelle"
    );
    assert!(
        action.due_at.is_some(),
        "le compte rendu est dû depuis la fin"
    );
}

// -----------------------------------------------------------------------------
// L'écart n° 108 — le calendrier des rappels, servi par l'espace organisation
// -----------------------------------------------------------------------------

/// Pose une règle de rappel d'édition et **matérialise** les rappels de la
/// séance par la fonction du modèle.
///
/// Les deux gestes appartiennent au module Engagement : ce fichier ne peut pas
/// appeler son service — un module ne dépend jamais d'un autre module —, et il
/// pose donc les lignes en SQL, comme le ferait n'importe quel semis de test.
async fn rappels_materialises(bac: &Bac, edition: uuid::Uuid, seance: uuid::Uuid, minutes: &[i32]) {
    sqlx::query!(
        r#"INSERT INTO engagement.reminder_rules (event_id, offsets, channels)
           VALUES ($1,
                   (SELECT array_agg(make_interval(mins => m) ORDER BY m DESC)
                      FROM unnest($2::int[]) m),
                   '{email}')"#,
        edition,
        minutes
    )
    .execute(bac.pool())
    .await
    .expect("insertion de la règle de rappel");

    sqlx::query_scalar!(
        r#"SELECT engagement.schedule_session_reminders($1) AS "crees!""#,
        seance
    )
    .fetch_one(bac.pool())
    .await
    .expect("matérialisation des rappels");
}

/// 🔴 **L'espace organisation porte le calendrier des rappels** — l'écart n° 108,
/// laissé ouvert par B4, est refermé.
///
/// Ce que ce test éprouve et qu'aucune relecture ne remplace : **la forme
/// exacte du contrat du front**. Les sept clés sont écrites ici et dans
/// `engagement`, où la même agrégation est sérialisée par `ReminderSlot` ; une
/// divergence de nom ferait un écran vide sans erreur.
///
/// Et **trois inscrits sur deux décalages font deux lignes de trois**, jamais
/// six lignes de un : l'agrégation est celle de la fonction du modèle, la même
/// que sert la lecture par séance.
#[tokio::test]
async fn lespace_organisation_porte_le_calendrier_des_rappels() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier =
        seances::dossier_pret(&bac, &terrain, "Atelier", "atelier", Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();

    let seance = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0)
        .id;
    seances::ouvrir_les_inscriptions(&bac, seance, None, false).await;

    for rang in 0..3 {
        let personne = commun::personne(
            &bac,
            &format!("rappele{rang}@example.org"),
            "Inscrit",
            &format!("Numéro{rang}"),
        )
        .await;
        seances::sinscrire(&bac, seance, Some(personne), seances::reponses_valides())
            .await
            .unwrap();
    }

    rappels_materialises(&bac, terrain.edition, seance, &[1440, 60]).await;

    let espace = workspace::espace(&bac.state, terrain.deposante, terrain.organisation)
        .await
        .expect("l'espace de l'organisation");
    let suivie = &espace
        .proposals
        .iter()
        .find(|p| p.proposal.id == dossier.id)
        .expect("le dossier retenu")
        .sessions[0];

    assert_eq!(
        suivie.reminders.len(),
        2,
        "deux décalages : deux lignes, jamais six"
    );

    // **La forme exacte du contrat du front.** Les clés, et rien d'autre.
    let mut clefs: Vec<&str> = suivie.reminders[0]
        .as_object()
        .expect("une ligne est un objet")
        .keys()
        .map(String::as_str)
        .collect();
    clefs.sort_unstable();
    assert_eq!(
        clefs,
        vec![
            "channel",
            "offset_before",
            "recipient_count",
            "scheduled_for",
            "sent_at",
            "skip_reason",
            "status",
        ]
    );

    let minutes: Vec<i64> = suivie
        .reminders
        .iter()
        .map(|r| r["offset_before"].as_i64().expect("un décalage en minutes"))
        .collect();
    assert_eq!(minutes, vec![1440, 60], "du plus lointain au plus proche");

    for ligne in &suivie.reminders {
        assert_eq!(
            ligne["recipient_count"].as_i64(),
            Some(3),
            "chaque ligne porte les trois inscrits"
        );
        assert_eq!(ligne["channel"], "email");
    }

    // **Un nombre, jamais un nom** : la même garantie qu'à la lecture par séance,
    // et elle vaut ici parce que c'est la même fonction du modèle qui la porte.
    let charge = serde_json::to_string(&suivie.reminders).expect("sérialisation");
    for motif in ["rappele", "@example.org", "Numéro"] {
        assert!(
            !charge.contains(motif),
            "« {motif} » figure dans le calendrier servi à l'organisation"
        );
    }
}
