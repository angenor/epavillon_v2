//! Le réseau distingue, il n'ouvre rien (ADR-007, FR-013, FR-014).
//!
//! L'appartenance vient **du code utilisé, et de rien d'autre** : aucun champ de
//! personne ne la commande, et le système ne demande, n'affiche ni ne stocke de
//! genre pour décider d'un droit (FR-008).

mod commun;

use commun::{Bac, Graine};
use negotiation::domain::redeem::RedeemIssue;

#[tokio::test]
async fn le_code_du_reseau_accorde_lappartenance_le_code_general_non() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    // Un code général, de même portée, sans réseau.
    commun::semer(
        &bac,
        Graine {
            code: "GENE-001",
            libelle: "Négociateurs de la COP31",
            space_id: Some(decor.space_id),
            ..Graine::default()
        },
    )
    .await;

    let avec_reseau = commun::saisir(&bac, decor.person_id, &decor.code, None).await;
    assert_eq!(avec_reseau.issue, RedeemIssue::Accepted);
    assert_eq!(
        commun::reseaux(&bac, decor.person_id).await,
        vec!["women_negotiators".to_owned()]
    );

    let generale = commun::personne(&bac, "ousmane.diop@example.org").await;
    let sans_reseau = commun::saisir(&bac, generale, "GENE-001", None).await;
    assert_eq!(sans_reseau.issue, RedeemIssue::Accepted);
    assert!(
        commun::reseaux(&bac, generale).await.is_empty(),
        "un code général n'accorde aucune appartenance"
    );
    assert!(
        commun::a_lacces(&bac, generale, Some(decor.space_id)).await,
        "le même accès, sans l'appartenance"
    );
}

/// FR-018 : une personne déjà admise gagne l'appartenance **sans recevoir un
/// second accès ni perdre le sien**.
#[tokio::test]
async fn une_personne_deja_admise_gagne_le_reseau_sans_second_acces() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    // Elle entre d'abord par un code général de portée globale : tout Guide
    // Négo, sans réseau.
    commun::semer(
        &bac,
        Graine {
            code: "GLOB-001",
            libelle: "Guide Négo en entier",
            ..Graine::default()
        },
    )
    .await;
    assert_eq!(
        commun::saisir(&bac, decor.person_id, "GLOB-001", None)
            .await
            .issue,
        RedeemIssue::Accepted
    );
    assert_eq!(
        commun::attributions(&bac, decor.person_id).await,
        vec![("global".to_owned(), None)]
    );

    // Puis elle saisit le code du réseau, de portée `negotiation_space`. Son
    // attribution globale couvre déjà cet espace : rien de nouveau à accorder.
    let ensuite = commun::saisir(&bac, decor.person_id, &decor.code, None).await;

    assert_eq!(ensuite.issue, RedeemIssue::AlreadyGranted);
    assert!(
        ensuite.message.contains("vient de s'y ajouter"),
        "le message doit dire ce que la saisie a réellement produit : {}",
        ensuite.message
    );
    assert_eq!(
        commun::reseaux(&bac, decor.person_id).await,
        vec!["women_negotiators".to_owned()]
    );
    assert_eq!(
        commun::attributions(&bac, decor.person_id).await,
        vec![("global".to_owned(), None)],
        "ni second accès, ni perte de celui qu'elle avait"
    );

    // Et son accès rendu à l'écran reste le plus large : « tout Guide Négo »,
    // jamais « la COP31 » — ce serait dire moins que la vérité.
    let acces = commun::mon_acces(&bac, decor.person_id).await;
    let accorde = acces.granted.expect("un accès accordé");
    assert_eq!(accorde.scope.id, None);
    assert_eq!(acces.networks.len(), 1);
}

/// L'appartenance est idempotente : saisir deux fois le même code du réseau ne
/// la double pas, et le second message ne promet plus rien.
#[tokio::test]
async fn lappartenance_ne_se_double_pas() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    commun::saisir(&bac, decor.person_id, &decor.code, None).await;
    let encore = commun::saisir(&bac, decor.person_id, &decor.code, None).await;

    assert_eq!(encore.issue, RedeemIssue::AlreadyGranted);
    assert!(
        !encore.message.contains("vient de s'y ajouter"),
        "rien ne s'est ajouté la seconde fois : {}",
        encore.message
    );
    assert_eq!(commun::reseaux(&bac, decor.person_id).await.len(), 1);
}
