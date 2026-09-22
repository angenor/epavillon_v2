//! **Le vocabulaire est gardé par la base** — un `INSERT` direct d'un terme
//! d'`activity_theme` est refusé par `tg_theme_subscriptions_check_theme`, et
//! l'API traduit le même refus en un message qui **nomme le code**.

mod commun;

use commun::{lignes_de_suivi, personne, suivre, Bac};
use kernel::error::ErrorCode;

#[tokio::test]
async fn la_base_refuse_un_terme_dun_autre_vocabulaire() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    // `biodiversity` n'existe que sous `activity_theme`.
    let intrus: uuid::Uuid = sqlx::query_scalar(
        "SELECT id FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'activity_theme' AND code = 'biodiversity'",
    )
    .fetch_one(bac.pool())
    .await
    .expect("terme du Pavillon");

    let refus = sqlx::query(
        "INSERT INTO negotiation.theme_subscriptions (person_id, theme_term_id) VALUES ($1, $2)",
    )
    .bind(awa)
    .bind(intrus)
    .execute(bac.pool())
    .await
    .expect_err("le trigger doit refuser");

    let message = match &refus {
        sqlx::Error::Database(db) => db.message().to_owned(),
        autre => panic!("refus attendu de la base, reçu {autre:?}"),
    };
    assert!(
        message.contains("negotiation_theme") && message.contains("theme_term_id"),
        "le trigger nomme le vocabulaire attendu et la colonne : {message}"
    );
    assert!(lignes_de_suivi(&bac, awa).await.is_empty());
}

#[tokio::test]
async fn lapi_nomme_le_code_refuse() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    let refus = suivre(&bac, awa, &["adaptation", "biodiversity"], None)
        .await
        .expect_err("un code d'un autre vocabulaire est inconnu ici");
    assert_eq!(refus.code, ErrorCode::NegotiationThemeUnknown);
    assert_eq!(refus.code.status().as_u16(), 400);
    assert!(
        refus.message.contains("« biodiversity »"),
        "le message nomme le code, jamais « une thématique est inconnue » : {}",
        refus.message
    );
    assert_eq!(refus.field.as_deref(), Some("codes"));

    assert!(
        lignes_de_suivi(&bac, awa).await.is_empty(),
        "rien n'est écrit, pas même le code valide de la même liste"
    );
}

#[tokio::test]
async fn une_liste_vide_est_refusee() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    let refus = suivre(&bac, awa, &[], None)
        .await
        .expect_err("au moins une");
    assert_eq!(refus.code, ErrorCode::NegotiationThemesEmpty);
    assert_eq!(refus.code.status().as_u16(), 400);

    let refus = suivre(&bac, awa, &["  ", ""], None)
        .await
        .expect_err("des blancs ne sont pas des codes");
    assert_eq!(refus.code, ErrorCode::NegotiationThemesEmpty);
}
