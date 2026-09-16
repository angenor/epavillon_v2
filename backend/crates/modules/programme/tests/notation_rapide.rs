//! **La notation rapide (16/09)** : une note sur 20 et un commentaire, qui
//! entrent dans la même moyenne que la grille et n'éliminent jamais.

mod commun;

use commun::comite::{comite, notation, notation_rapide};
use commun::Bac;
use kernel::error::ErrorCode;
use programme::domain::ids::ProposalId;
use programme::service::review;
use programme::service::review::SaveReviewPayload;
use uuid::Uuid;

async fn notes_par_critere(bac: &Bac, dossier: Uuid, membre: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!"
             FROM programme.review_scores rs
             JOIN programme.reviews r ON r.id = rs.review_id
            WHERE r.proposal_id = $1 AND r.reviewer_id = $2"#,
        dossier,
        membre
    )
    .fetch_one(bac.pool())
    .await
    .expect("décompte des notes par critère")
}

#[tokio::test]
async fn un_depot_rapide_sans_note_est_refuse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    let refus = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation_rapide(None, true),
    )
    .await
    .expect_err("une notation rapide se dépose avec sa note");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("score_out_of_20"));

    review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation_rapide(None, false),
    )
    .await
    .expect("un brouillon rapide peut attendre sa note");

    let hors_bornes = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation_rapide(Some(21.0), false),
    )
    .await
    .expect_err("une note au-delà de 20 est refusée");
    assert_eq!(hors_bornes.field.as_deref(), Some("score_out_of_20"));

    let mode_inconnu = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        SaveReviewPayload {
            mode: "express".to_owned(),
            ..notation_rapide(Some(10.0), false)
        },
    )
    .await
    .expect_err("un mode inconnu est refusé");
    assert_eq!(mode_inconnu.field.as_deref(), Some("mode"));
}

/// **La note rapide entre telle quelle dans la moyenne**, et se mélange à une
/// revue détaillée.
#[tokio::test]
async fn une_note_rapide_entre_dans_la_moyenne_du_dossier() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    let rendu = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation_rapide(Some(14.0), true),
    )
    .await
    .expect("dépôt rapide");
    assert_eq!(rendu.proposal_average_score, Some(14.0));
    assert_eq!(rendu.review.mode, "quick");
    assert_eq!(rendu.review.score_out_of_20, Some(14.0));
    assert!(rendu.review.weighted_score.is_some());
    assert_eq!(
        rendu.review.comment.as_deref(),
        Some("Sujet solide, intervenants à confirmer.")
    );

    let detaillee = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.seconde,
        ProposalId(comite.dossier),
        notation(&comite.criteres, 0.5, true),
    )
    .await
    .expect("dépôt détaillé");
    let sa_note = detaillee
        .review
        .score_out_of_20
        .expect("la base calcule la note détaillée");
    let moyenne = detaillee
        .proposal_average_score
        .expect("moyenne du dossier");

    assert_eq!(detaillee.review_count, 2);
    assert!(
        (moyenne - (14.0 + sa_note) / 2.0).abs() <= 0.01,
        "moyenne {moyenne}, note détaillée {sa_note}"
    );
}

#[tokio::test]
async fn une_note_rapide_nelimine_jamais() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    let rendu = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation_rapide(Some(0.0), true),
    )
    .await
    .expect("dépôt rapide à zéro");
    assert!(!rendu.is_knocked_out);
    assert_eq!(rendu.proposal_average_score, Some(0.0));
}

/// **Passer en rapide efface la grille** : un zéro éliminatoire laissé en place
/// resterait en base sans rien dire.
#[tokio::test]
async fn passer_en_rapide_efface_les_notes_par_critere() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;
    let membre = comite.premiere.person_id;

    let mut charge = notation(&comite.criteres, 0.8, false);
    let eliminatoire = comite
        .criteres
        .iter()
        .find(|(_, _, ko)| *ko)
        .expect("la grille par défaut porte un critère éliminatoire");
    charge.scores.insert(eliminatoire.0, 0.0);
    review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        charge,
    )
    .await
    .expect("brouillon détaillé");
    assert_eq!(
        notes_par_critere(&bac, comite.dossier, membre).await,
        comite.criteres.len() as i64
    );

    let rendu = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation_rapide(Some(12.0), true),
    )
    .await
    .expect("dépôt rapide");
    assert_eq!(notes_par_critere(&bac, comite.dossier, membre).await, 0);
    assert!(!rendu.is_knocked_out);
    assert_eq!(rendu.proposal_average_score, Some(12.0));

    // Revenir au détaillé sans critère ne garde pas la note rapide.
    let retour = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        SaveReviewPayload {
            scores: Default::default(),
            ..notation(&comite.criteres, 0.8, false)
        },
    )
    .await
    .expect("retour au détaillé");
    assert_eq!(retour.review.mode, "detailed");
    assert_eq!(retour.review.score_out_of_20, None);
    assert_eq!(retour.review.weighted_score, None);

    // Déposée, une revue rapide ne perd plus sa note.
    let refus = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation_rapide(None, false),
    )
    .await
    .expect_err("une revue déposée garde sa note");
    assert_eq!(refus.field.as_deref(), Some("score_out_of_20"));
}
