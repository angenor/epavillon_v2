//! **La reprise des dossiers de la v1 : une frise qui ne ment pas, et qui
//! n'envoie aucun courriel.**
//!
//! Deux choses s'y prouvent, et la seconde est celle qui coûterait le plus cher
//! à ne pas avoir : la déduction écrit dans le journal **sans passer par la
//! mise à jour de l'état**, donc sans réveiller le déclencheur. Émettre huit
//! mille événements de dossiers décidés il y a deux ans déclencherait autant de
//! courriels.

mod commun;

use commun::{Bac, Terrain};
use programme::service::backfill;
use uuid::Uuid;

/// Un dossier **repris de la v1** : posé dans son état final, sans journal.
///
/// L'insertion échappe au garde d'état — il n'est posé que sur la mise à jour
/// de `status` —, et c'est exactement ce que la migration produit.
async fn dossier_repris(bac: &Bac, terrain: &Terrain, titre: &str, slug: &str) -> Uuid {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (call_id, event_id, organization_id, submitted_by,
                title, slug, objectives, detailed_presentation, format,
                status, submitted_at, decided_at)
           VALUES ($1, $2, $3, $4,
                   jsonb_build_object('fr', $5::text),
                   $6::text::platform.slug,
                   '{"fr":"Objectifs."}'::jsonb,
                   '{"fr":"<p>Présentation.</p>"}'::jsonb,
                   'in_person', 'accepted',
                   now() - interval '400 days', now() - interval '380 days')
        RETURNING id"#,
        terrain.appel,
        terrain.edition,
        terrain.organisation,
        terrain.deposante,
        titre,
        slug
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du dossier repris");

    // La migration ne laisse aucune ligne de journal ; le déclencheur
    // d'insertion en pose une, on la retire pour reproduire l'état réel.
    sqlx::query!(
        "DELETE FROM programme.proposal_transitions WHERE proposal_id = $1",
        id
    )
    .execute(bac.pool())
    .await
    .expect("vidage du journal");

    sqlx::query!(
        "DELETE FROM platform.outbox_events WHERE aggregate_id = $1",
        id
    )
    .execute(bac.pool())
    .await
    .expect("vidage de l'outbox");

    id
}

#[tokio::test]
async fn la_deduction_seme_trois_lignes_puis_zero_et_nemet_rien() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = dossier_repris(&bac, &terrain, "Dossier de la COP30", "dossier-cop30").await;

    assert!(
        commun::journal(&bac, dossier).await.is_empty(),
        "un dossier repris arrive sans journal"
    );

    let premiere = backfill::deduire(&bac.state, &bac.ctx())
        .await
        .expect("la déduction");
    assert_eq!(premiere.transitions, 3, "création, dépôt, décision");
    assert_eq!(premiere.proposals, 1);

    // **Les trois lignes sont dans l'ordre**, et l'ordre vient des dates du
    // dossier, non de l'ordre d'insertion.
    let journal = commun::journal(&bac, dossier).await;
    assert_eq!(
        journal,
        vec![
            (None, "draft".to_owned()),
            (Some("draft".to_owned()), "submitted".to_owned()),
            (Some("submitted".to_owned()), "accepted".to_owned()),
        ]
    );

    // **Aucun événement.** C'est ce qui rend la reprise sûre : elle écrit dans
    // le journal sans passer par la mise à jour de l'état, donc sans réveiller
    // le déclencheur.
    assert!(
        commun::evenements_emis(&bac, dossier).await.is_empty(),
        "une reprise ne déclenche pas huit mille courriels"
    );

    // **Rejouable** : la condition « journal vide » est dans la requête
    // d'insertion, et une seconde exécution ne sème rien.
    let seconde = backfill::deduire(&bac.state, &bac.ctx())
        .await
        .expect("la seconde exécution");
    assert_eq!(seconde.transitions, 0);
    assert_eq!(seconde.proposals, 0);
    assert_eq!(commun::journal(&bac, dossier).await.len(), 3);
}

/// **Ce qui n'est pas dans les dates n'est pas déduit.**
///
/// Un dossier encore en évaluation n'a pas de date de décision : la déduction
/// s'arrête au dépôt, et n'invente ni le passage par l'évaluation ni une
/// demande de correction.
#[tokio::test]
async fn la_deduction_ninvente_ni_evaluation_ni_correction() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let id = sqlx::query_scalar!(
        r#"INSERT INTO programme.proposals
               (call_id, event_id, organization_id, submitted_by,
                title, slug, objectives, detailed_presentation, format,
                status, submitted_at)
           VALUES ($1, $2, $3, $4,
                   '{"fr":"Dossier en instruction"}'::jsonb,
                   'dossier-en-instruction'::platform.slug,
                   '{"fr":"Objectifs."}'::jsonb,
                   '{"fr":"<p>Présentation.</p>"}'::jsonb,
                   'online', 'under_review', now() - interval '30 days')
        RETURNING id"#,
        terrain.appel,
        terrain.edition,
        terrain.organisation,
        terrain.deposante
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion");

    sqlx::query!(
        "DELETE FROM programme.proposal_transitions WHERE proposal_id = $1",
        id
    )
    .execute(bac.pool())
    .await
    .expect("vidage du journal");

    let resultat = backfill::deduire(&bac.state, &bac.ctx())
        .await
        .expect("la déduction");
    assert_eq!(resultat.transitions, 2, "création et dépôt, rien de plus");

    let journal = commun::journal(&bac, id).await;
    assert_eq!(
        journal,
        vec![
            (None, "draft".to_owned()),
            (Some("draft".to_owned()), "submitted".to_owned()),
        ],
        "le passage par l'évaluation n'est pas déductible : l'inventer serait pire qu'un trou"
    );
}

/// **Un dossier au journal déjà peuplé n'est pas touché.**
#[tokio::test]
async fn un_dossier_au_journal_peuple_est_laisse_tel_quel() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Ce dossier-ci garde la ligne posée par le déclencheur d'insertion.
    let vivant = commun::dossier(&bac, &terrain, "Dossier vivant", "dossier-vivant").await;
    let repris = dossier_repris(&bac, &terrain, "Dossier repris", "dossier-repris").await;

    let avant = commun::journal(&bac, vivant).await;
    assert_eq!(avant.len(), 1);

    backfill::deduire(&bac.state, &bac.ctx())
        .await
        .expect("la déduction");

    assert_eq!(
        commun::journal(&bac, vivant).await,
        avant,
        "un journal déjà peuplé n'est pas complété"
    );
    assert_eq!(commun::journal(&bac, repris).await.len(), 3);
}

/// **Un dossier effacé ne laisse aucun lien de thématique derrière lui**
/// (écart n° 94).
///
/// `reference.entity_terms` est polymorphe : aucune clé étrangère vers les
/// propositions, aucune cascade. Sans purge explicite, les liens restent —
/// invisibles, mais comptés par tout ce qui agrège par thématique.
#[tokio::test]
async fn un_dossier_efface_ne_laisse_aucun_lien_de_thematique() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;

    let mut brouillon = commun::brouillon(&terrain, "Atelier adaptation");
    brouillon.theme_codes = vec!["adaptation".to_owned(), "mitigation".to_owned()];
    let ligne = programme::service::draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, brouillon),
    )
    .await
    .expect("enregistrement");

    assert_eq!(commun::thematiques(&bac, ligne.proposal_id).await.len(), 2);

    programme::service::detail::effacer(
        &bac.state,
        &bac.ctx(),
        &perimetre,
        programme::domain::ids::ProposalId(ligne.proposal_id),
        Some("Doublon d'un dossier déjà déposé."),
    )
    .await
    .expect("l'effacement logique");

    assert!(
        commun::thematiques(&bac, ligne.proposal_id)
            .await
            .is_empty(),
        "les liens de thématique sont purgés à la main : aucune cascade ne les atteint"
    );

    let efface = sqlx::query!(
        "SELECT deleted_at, deleted_by, deleted_reason
           FROM programme.proposals WHERE id = $1",
        ligne.proposal_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture");
    assert!(efface.deleted_at.is_some());
    assert_eq!(efface.deleted_by, Some(droits.decideur));
    assert_eq!(
        efface.deleted_reason.as_deref(),
        Some("Doublon d'un dossier déjà déposé.")
    );

    // **L'effacement est logique** : la ligne demeure, mais rien n'y donne plus
    // accès — la vue de pilotage l'exclut, et la déduction aussi.
    let refus = programme::service::detail::dossier(
        &bac.state,
        terrain.deposante,
        programme::domain::ids::ProposalId(ligne.proposal_id),
    )
    .await
    .expect_err("un dossier effacé se refuse comme un inexistant");
    assert_eq!(refus.code, kernel::error::ErrorCode::NotFound);
}
