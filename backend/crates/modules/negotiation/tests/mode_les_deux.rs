//! **En mode « les deux », un code juste n'ouvre pas** — FR-023.
//!
//! Il ouvre une demande **qui porte ce code**, et c'est précisément ce que
//! l'administrateur lira pour trancher : « cette personne détient le code du
//! réseau des négociatrices » n'est pas la même information que « cette
//! personne dit faire partie d'une délégation ».
//!
//! Deux choses ne bougent pas dans ce mode : le quota du code n'est pas entamé
//! — on n'est pas entré —, et une personne qui détient **déjà** l'accès n'ouvre
//! aucune demande (FR-018).

mod commun;

use commun::{
    a_lacces, admettre, attribuer, creer_un_code, decor, mon_acces, personne, regler_le_mode,
    saisir, usages, Bac,
};
use negotiation::domain::access::AccessState;
use negotiation::domain::redeem::RedeemIssue;

#[tokio::test]
async fn un_code_juste_ouvre_une_demande_qui_le_porte() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;
    regler_le_mode(&bac, "code_and_approval").await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", Some(d.space_id), None).await;
    let issue = saisir(&bac, d.person_id, &code.code, None).await;

    assert_eq!(issue.issue, RedeemIssue::PendingApproval);
    assert!(!a_lacces(&bac, d.person_id, Some(d.space_id)).await);

    let demande = issue.request.expect("une demande s'est ouverte");
    let porte = sqlx::query_scalar!(
        "SELECT invitation_code_id FROM negotiation.access_requests WHERE id = $1",
        demande.id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la demande");

    assert_eq!(porte, Some(code.id), "la demande porte le code reconnu");

    // **Le quota n'est pas entamé** : une demande peut être refusée, et le code
    // ne doit pas avoir perdu une entrée pour rien.
    assert_eq!(usages(&bac, code.id).await, (0, 0));

    assert_eq!(
        mon_acces(&bac, d.person_id).await.state,
        AccessState::Pending
    );
}

#[tokio::test]
async fn ressaisir_le_code_retrouve_la_meme_demande() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;
    regler_le_mode(&bac, "code_and_approval").await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", Some(d.space_id), None).await;

    let premiere = saisir(&bac, d.person_id, &code.code, None).await;
    let seconde = saisir(&bac, d.person_id, &code.code, None).await;

    assert_eq!(
        premiere.request.expect("première").id,
        seconde.request.expect("seconde").id,
        "la saisie est idempotente : on retrouve sa demande, pas un refus"
    );
}

#[tokio::test]
async fn une_personne_deja_admise_nouvre_aucune_demande() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let acces = creer_un_code(&bac, d.admin_id, "Premier code", Some(d.space_id), None).await;
    saisir(&bac, d.person_id, &acces.code, None).await;

    // La bascule arrive APRÈS l'entrée : la personne a son accès, et un second
    // code — celui du réseau — ne doit pas la faire attendre pour un droit
    // qu'elle détient (FR-018).
    regler_le_mode(&bac, "code_and_approval").await;
    let reseau = creer_un_code(
        &bac,
        d.admin_id,
        "Réseau des négociatrices",
        Some(d.space_id),
        Some("women_negotiators"),
    )
    .await;

    let issue = saisir(&bac, d.person_id, &reseau.code, None).await;
    assert_eq!(issue.issue, RedeemIssue::AlreadyGranted);
    assert!(issue.request.is_none());
    assert_eq!(
        commun::reseaux(&bac, d.person_id).await,
        vec!["women_negotiators".to_owned()],
        "seule l'appartenance s'est ajoutée"
    );
}

#[tokio::test]
async fn en_approbation_seule_le_code_ouvre_aussi_une_demande() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;
    regler_le_mode(&bac, "approval").await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", Some(d.space_id), None).await;
    let quelquun = personne(&bac, "binta.ba@example.org").await;

    // L'application ne propose pas la saisie dans ce mode (FR-022), mais la
    // route reste montée : une personne qui l'atteint quand même — écran gardé
    // en mémoire, bascule entre deux gestes — doit repartir avec une suite, pas
    // avec une impasse (FR-015).
    let issue = saisir(&bac, quelquun, &code.code, None).await;
    assert_eq!(issue.issue, RedeemIssue::PendingApproval);

    let demande = issue.request.expect("une demande s'est ouverte");
    admettre(&bac, d.admin_id, demande.id, None)
        .await
        .expect("admission");

    assert!(a_lacces(&bac, quelquun, Some(d.space_id)).await);
}
