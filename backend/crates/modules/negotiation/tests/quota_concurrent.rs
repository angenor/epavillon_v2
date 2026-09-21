//! Deux entrées simultanées sur le dernier usage d'un code.
//!
//! FR-038 bis exige que le dépassement soit **impossible**, pas seulement
//! improbable. Un `SELECT used_count` avant l'insertion lirait la même valeur
//! dans les deux requêtes, et le code de 120 usages en accorderait 121 : c'est
//! le verrou de ligne pris par le trigger d'incrément qui les sérialise, et
//! `ck_invitation_codes_quota` qui refuse la seconde.

mod commun;

use commun::{Bac, Graine};
use negotiation::domain::redeem::RedeemIssue;

#[tokio::test]
async fn deux_entrees_simultanees_sur_le_dernier_usage() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    let code_id = commun::semer(
        &bac,
        Graine {
            code: "UNIQ-001",
            libelle: "Un seul usage",
            space_id: Some(decor.space_id),
            max_uses: Some(1),
            ..Graine::default()
        },
    )
    .await;

    let premiere = commun::personne(&bac, "awa.premiere@example.org").await;
    let seconde = commun::personne(&bac, "awa.seconde@example.org").await;

    // Les deux saisies partent ensemble : `join!` les fait avancer en
    // alternance, et chacune tient sa propre connexion du pool.
    let (a, b) = tokio::join!(
        commun::saisir(&bac, premiere, "UNIQ-001", None),
        commun::saisir(&bac, seconde, "UNIQ-001", None),
    );

    let issues = [a.issue, b.issue];
    assert_eq!(
        issues
            .iter()
            .filter(|i| **i == RedeemIssue::Accepted)
            .count(),
        1,
        "une seule des deux entrées doit passer : {issues:?}"
    );
    assert_eq!(
        issues
            .iter()
            .filter(|i| **i == RedeemIssue::Exhausted)
            .count(),
        1,
        "l'autre doit recevoir « épuisé », et non une panne : {issues:?}"
    );

    let (lignes, compteur) = commun::usages(&bac, code_id).await;
    assert_eq!(lignes, 1, "un seul usage écrit");
    assert_eq!(compteur, 1, "used_count n'excède jamais max_uses");

    // Et l'accès n'est allé qu'à une seule des deux.
    let admise_a = commun::a_lacces(&bac, premiere, Some(decor.space_id)).await;
    let admise_b = commun::a_lacces(&bac, seconde, Some(decor.space_id)).await;
    assert!(
        admise_a ^ admise_b,
        "exactement une des deux doit être admise"
    );
}

/// Le quota tient au-delà de deux : quatre entrées simultanées sur un code de
/// deux usages en laissent exactement deux passer.
#[tokio::test]
async fn quatre_entrees_sur_un_code_de_deux_usages() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    let code_id = commun::semer(
        &bac,
        Graine {
            code: "DEUX-001",
            libelle: "Deux usages",
            space_id: Some(decor.space_id),
            max_uses: Some(2),
            ..Graine::default()
        },
    )
    .await;

    let p1 = commun::personne(&bac, "candidate1@example.org").await;
    let p2 = commun::personne(&bac, "candidate2@example.org").await;
    let p3 = commun::personne(&bac, "candidate3@example.org").await;
    let p4 = commun::personne(&bac, "candidate4@example.org").await;

    let (a, b, c, d) = tokio::join!(
        commun::saisir(&bac, p1, "DEUX-001", None),
        commun::saisir(&bac, p2, "DEUX-001", None),
        commun::saisir(&bac, p3, "DEUX-001", None),
        commun::saisir(&bac, p4, "DEUX-001", None),
    );

    let issues = [a.issue, b.issue, c.issue, d.issue];
    assert_eq!(
        issues
            .iter()
            .filter(|i| **i == RedeemIssue::Accepted)
            .count(),
        2,
        "{issues:?}"
    );
    assert_eq!(commun::usages(&bac, code_id).await, (2, 2));
}
