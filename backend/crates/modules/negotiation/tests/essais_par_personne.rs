//! La limite d'essais se compte **par personne**, tous appareils confondus.
//!
//! C'est le test qui vaut le plus de ce fichier : `device_id` vient du corps de
//! la requête, donc il se forge. Compter « par personne et par appareil »
//! suffirait à changer d'identifiant à chaque essai pour que la limite ne limite
//! rien — et rien, dans le code, ne l'aurait dit.

mod commun;

use commun::Bac;
use negotiation::domain::redeem::RedeemIssue;

#[tokio::test]
async fn cinq_echecs_puis_un_essai_dun_autre_appareil_reste_bloque() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;
    commun::regler_les_essais(&bac, 5, 15, 15).await;

    for essai in 0..5 {
        let refus = commun::saisir(
            &bac,
            decor.person_id,
            "ZZZZ-999",
            Some("appareil-de-la-negociatrice"),
        )
        .await;
        assert_eq!(refus.issue, RedeemIssue::Unknown, "essai {essai}");
    }

    // Le sixième essai vient d'un AUTRE appareil, avec le BON code.
    let bloquee = commun::saisir(
        &bac,
        decor.person_id,
        &decor.code,
        Some("un-autre-appareil"),
    )
    .await;

    assert_eq!(
        bloquee.issue,
        RedeemIssue::Throttled,
        "changer d'identifiant d'appareil ne doit pas remettre le compteur à zéro"
    );
    assert!(
        !commun::a_lacces(&bac, decor.person_id, Some(decor.space_id)).await,
        "l'accès ne doit pas s'ouvrir par un essai que la limite refuse"
    );

    // L'appareil est gardé comme information : les deux se retrouvent dans la
    // table, et c'est ce qui permet à un administrateur de lire la série.
    let essais = commun::essais(&bac, decor.person_id).await;
    assert_eq!(essais.len(), 6);
    assert_eq!(
        essais[5],
        ("throttled".to_owned(), Some("un-autre-appareil".to_owned()))
    );
}

/// Une autre personne, au même moment, n'est pas gênée : le compteur est
/// personnel. Le confondre avec un compteur global ferait d'un essai maladroit
/// une panne pour tout un réseau.
#[tokio::test]
async fn la_limite_dune_personne_nengage_quelle_meme() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;
    commun::regler_les_essais(&bac, 2, 15, 15).await;

    for _ in 0..2 {
        commun::saisir(&bac, decor.person_id, "ZZZZ-999", None).await;
    }
    assert_eq!(
        commun::saisir(&bac, decor.person_id, &decor.code, None)
            .await
            .issue,
        RedeemIssue::Throttled
    );

    let autre = commun::personne(&bac, "nafi.cisse@example.org").await;
    assert_eq!(
        commun::saisir(&bac, autre, &decor.code, None).await.issue,
        RedeemIssue::Accepted
    );
}

/// **Les essais refusés par la limite ne prolongent pas la limite.**
///
/// Un essai `throttled` est lui-même un essai non accepté : le compter ferait
/// repartir la fenêtre à chaque appui, et le quart d'heure annoncé à l'écran ne
/// finirait jamais. Le défaut a été trouvé en écrivant le service, et corrigé
/// dans la fonction de comptage du modèle.
#[tokio::test]
async fn un_essai_refuse_par_la_limite_ne_prolonge_pas_la_limite() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;
    commun::regler_les_essais(&bac, 2, 15, 15).await;

    for _ in 0..2 {
        commun::saisir(&bac, decor.person_id, "ZZZZ-999", None).await;
    }
    // Trois appuis de plus pendant le verrou.
    for _ in 0..3 {
        assert_eq!(
            commun::saisir(&bac, decor.person_id, &decor.code, None)
                .await
                .issue,
            RedeemIssue::Throttled
        );
    }

    // Le compteur n'a pas bougé : deux essais consommés, cinq lignes écrites.
    let consommes = sqlx::query_scalar!(
        r#"SELECT negotiation.invitation_attempts_recent(
                      $1, make_interval(mins => 15)) AS "compte!""#,
        decor.person_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des essais");

    assert_eq!(
        consommes, 2,
        "les essais refusés par la limite ne comptent pas"
    );
    assert_eq!(commun::essais(&bac, decor.person_id).await.len(), 5);
}
