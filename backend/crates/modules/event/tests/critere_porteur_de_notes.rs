//! **Le refus qui sauve les notes** — l'unique entorse au principe VIII de tout
//! le module (research.md § R9, écart n° 91).
//!
//! `xmod_fk_review_scores_criterion` est `ON DELETE CASCADE`. Retirer une ligne
//! de la grille effacerait toutes les notes posées sur ce critère : **sans
//! erreur, sans trace, et sans que l'écran puisse le voir**. Or ces notes sont
//! l'argumentaire d'une décision de sélection — précisément ce que la v1 n'avait
//! pas, et qui rendait un refus inexplicable à l'organisation qui le contestait.
//!
//! **Ce test a deux moitiés, et la seconde est celle qui compte.** Vérifier le
//! refus ne prouve rien : il faut vérifier que **les notes sont toujours là
//! après**. Un service qui supprimerait d'abord et refuserait ensuite passerait
//! la première moitié sans sourciller.

mod commun;

use commun::{auteur, formulaire_appel, notes_du_critere, Bac};
use event::domain::ids::{CallId, EventId};
use event::service::call as service_appel;
use kernel::error::ErrorCode;

#[tokio::test]
async fn retirer_un_critere_porteur_de_notes_est_refuse_et_les_notes_restent() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    // Un appel à deux critères, dont l'un recevra une note.
    let mut ouverture = formulaire_appel(editions.cop31, "cop31");
    ouverture.criteria = vec![
        commun::critere("relevance", 2.0),
        commun::critere("impact", 1.5),
    ];
    let cree = service_appel::creer(&bac.state, &bac.ctx(), acteur, cop31, ouverture)
        .await
        .expect("création");
    assert!(cree.ok, "{:?}", cree.errors);
    let appel = cree.call.expect("l'appel créé");
    let call_id = CallId::from(appel.id);

    let critere_note =
        commun::seed::note_sur_le_critere(&bac, editions.cop31, appel.id, "relevance").await;
    assert_eq!(
        notes_du_critere(&bac, critere_note).await,
        1,
        "la note est bien posée avant l'essai de retrait"
    );

    // La charge utile ne porte plus que le second critère : le premier
    // disparaîtrait, avec sa note.
    let mut retrait = formulaire_appel(editions.cop31, "cop31");
    retrait.criteria = vec![commun::critere("impact", 1.5)];

    let refus = service_appel::modifier(&bac.state, &bac.ctx(), acteur, cop31, call_id, retrait)
        .await
        .expect_err("le retrait doit être refusé");

    assert_eq!(refus.code, ErrorCode::EventCriterionHasScores);
    assert_eq!(refus.field.as_deref(), Some("criteria"));
    assert!(
        refus.message.contains("relevance") || refus.message.contains("Critère relevance"),
        "le refus doit NOMMER le critère : {}",
        refus.message
    );
    assert!(
        refus.message.contains('1'),
        "le refus doit compter les notes : {}",
        refus.message
    );

    // **La seconde moitié, celle qui prouve quelque chose.**
    assert_eq!(
        notes_du_critere(&bac, critere_note).await,
        1,
        "la note doit être intacte après le refus"
    );
    let grille = commun::grille_en_base(&bac, appel.id).await;
    assert_eq!(
        grille.len(),
        2,
        "la grille entière est intacte : le refus annule TOUT l'enregistrement"
    );
}

/// **Un critère sans note se retire sans discussion.** Le refus est borné à ce
/// qu'il protège ; l'étendre interdirait de corriger une grille avant la
/// première évaluation, ce qui est le cas courant.
#[tokio::test]
async fn retirer_un_critere_sans_note_se_fait_sans_refus() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let mut ouverture = formulaire_appel(editions.cop31, "cop31");
    ouverture.criteria = vec![
        commun::critere("relevance", 2.0),
        commun::critere("impact", 1.5),
    ];
    let cree = service_appel::creer(&bac.state, &bac.ctx(), acteur, cop31, ouverture)
        .await
        .expect("création");
    let appel = cree.call.expect("l'appel créé");

    let mut retrait = formulaire_appel(editions.cop31, "cop31");
    retrait.criteria = vec![commun::critere("impact", 1.5)];

    let resultat = service_appel::modifier(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        CallId::from(appel.id),
        retrait,
    )
    .await
    .expect("le retrait d'un critère sans note aboutit");

    assert!(resultat.ok, "{:?}", resultat.errors);
    assert_eq!(commun::grille_en_base(&bac, appel.id).await.len(), 1);
}

/// **Un barème modifié sur un critère noté ne bloque rien — il se signale.**
/// Les notes ne bougent pas ; les moyennes, si, et l'écran doit le dire plutôt
/// que de le taire.
#[tokio::test]
async fn un_bareme_modifie_sur_un_critere_note_signale_sans_bloquer() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let cree = service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        formulaire_appel(editions.cop31, "cop31"),
    )
    .await
    .expect("création");
    let appel = cree.call.expect("l'appel créé");
    assert!(!cree.scores_affected, "aucune note à la création");

    commun::seed::note_sur_le_critere(&bac, editions.cop31, appel.id, "relevance").await;

    let mut nouveau_poids = formulaire_appel(editions.cop31, "cop31");
    nouveau_poids.criteria = vec![commun::critere("relevance", 3.0)];

    let resultat = service_appel::modifier(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        CallId::from(appel.id),
        nouveau_poids,
    )
    .await
    .expect("la modification de barème aboutit");

    assert!(resultat.ok, "{:?}", resultat.errors);
    assert!(
        resultat.scores_affected,
        "un poids modifié sur un critère noté déplace des moyennes : il faut le dire"
    );
    assert_eq!(commun::grille_en_base(&bac, appel.id).await[0].2, 3.0);
}
