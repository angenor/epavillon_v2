//! Le code circule recopié à la main : casse et séparateurs sont ignorés.
//!
//! Le test compare **la normalisation du code Rust à celle de la base** plutôt
//! que de la supposer : `code_normalized` est une colonne `GENERATED`, et les
//! deux expressions doivent rendre la même chaîne. Si l'une dérivait, la
//! recherche ne trouverait plus les codes existants — sans qu'aucune erreur ne
//! le dise.

mod commun;

use commun::Bac;
use negotiation::domain::code;
use negotiation::domain::redeem::RedeemIssue;

#[tokio::test]
async fn trois_ecritures_du_meme_code_ouvrent_le_meme_acces() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    commun::semer_code(
        &bac,
        "NEGO-024",
        "Réseau des négociatrices",
        Some(decor.space_id),
        None,
    )
    .await;

    // La première passe, les deux suivantes tombent sur la même ligne.
    let formes = ["nego-024", "NEGO 024", "Nego024"];
    let mut issues = Vec::new();
    for forme in formes {
        issues.push(
            commun::saisir(&bac, decor.person_id, forme, None)
                .await
                .issue,
        );
    }

    assert_eq!(
        issues,
        vec![
            RedeemIssue::Accepted,
            RedeemIssue::AlreadyGranted,
            RedeemIssue::AlreadyGranted
        ],
        "les trois écritures désignent le même code"
    );
    assert_eq!(commun::attributions(&bac, decor.person_id).await.len(), 1);
}

/// La normalisation du code et celle de la base rendent **exactement** la même
/// chaîne, éprouvée sur les formes qui circulent réellement.
#[tokio::test]
async fn la_normalisation_du_code_suit_celle_de_la_base() {
    let bac = Bac::monter().await;

    for forme in [
        "nego-024",
        "NEGO 024",
        "Nego024",
        " NEGO-024 ",
        "ne go-0 24",
        "NEGO--024",
    ] {
        let en_base = sqlx::query_scalar!(
            r#"SELECT upper(regexp_replace($1, '[^A-Za-z0-9]', '', 'g')) AS "forme!""#,
            forme
        )
        .fetch_one(bac.pool())
        .await
        .expect("normalisation en base");

        assert_eq!(code::normaliser(forme), en_base, "forme « {forme} »");
    }
}

/// Huit caractères, tirets compris — et un code engendré que la table accepte.
/// `NEGO-24` de la maquette en porte sept et se corrige.
#[tokio::test]
async fn le_code_engendre_est_accepte_par_la_base() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    for _ in 0..5 {
        let engendre = code::engendrer();
        assert_eq!(engendre.chars().count(), 8, "{engendre}");

        let id =
            commun::semer_code(&bac, &engendre, "Code engendré", Some(decor.space_id), None).await;

        // La forme normalisée que la base calcule n'a plus de tiret : sept
        // caractères, et elle correspond à celle du code Rust.
        let normalise = sqlx::query_scalar!(
            r#"SELECT code_normalized AS "forme!" FROM negotiation.invitation_codes WHERE id = $1"#,
            id
        )
        .fetch_one(bac.pool())
        .await
        .expect("lecture de la forme normalisée");

        assert_eq!(normalise, code::normaliser(&engendre));
        assert_eq!(normalise.chars().count(), 7);
    }
}
