//! Le harnais monte, et le terrain qu'il pose est celui qu'il annonce.
//!
//! Ce fichier n'éprouve **aucune règle métier** : il éprouve la fabrique, dont
//! les huit tests des histoires utilisateur dépendent tous. Un harnais qui pose
//! une séance dans le passé ou une adhésion en attente ferait échouer huit
//! fichiers sur une cause qu'aucun d'eux ne nomme.

mod commun;

use commun::Bac;
use time::OffsetDateTime;

#[tokio::test]
async fn le_terrain_pose_ce_quil_annonce() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    assert_eq!(terrain.inscrits.len(), 3);

    let seance = engagement::repo::cross::seance(bac.pool(), terrain.seance)
        .await
        .expect("lecture de la séance")
        .expect("la séance existe");

    assert_eq!(seance.event_id, terrain.edition);
    assert_eq!(seance.organization_id, Some(terrain.organisation));

    // **La séance est dans le futur** : sans cela, la fonction du modèle
    // n'insérerait aucun rappel, et les tests des histoires échoueraient sur une
    // règle qui fonctionne parfaitement.
    assert!(
        seance.starts_at > OffsetDateTime::now_utc(),
        "la séance de référence doit être à venir"
    );
}

#[tokio::test]
async fn les_inscrits_sont_ceux_qui_ont_droit_a_un_rappel() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Une quatrième personne, en liste d'attente : elle n'a droit à rien.
    let attente = commun::personne(&bac, "attente@example.org", "Sara", "Attente").await;
    commun::inscrire(&bac, terrain.seance, attente, "waitlisted").await;

    let destinataires = engagement::repo::cross::inscrits_a_rappeler(bac.pool(), terrain.seance)
        .await
        .expect("lecture des inscrits");

    assert_eq!(destinataires.len(), 3);
    assert!(destinataires.iter().all(|d| d.status == "registered"));
}

#[tokio::test]
async fn ladhesion_active_ouvre_le_calendrier_et_letrangere_non() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    assert!(engagement::repo::cross::adhesion_active(
        bac.pool(),
        terrain.animatrice,
        terrain.organisation
    )
    .await
    .expect("lecture de l'adhésion"));
    assert!(!engagement::repo::cross::adhesion_active(
        bac.pool(),
        terrain.etrangere,
        terrain.organisation
    )
    .await
    .expect("lecture de l'adhésion"));
}

/// **Le semis ne pose ni règle, ni modèle** : chaque test les pose. Un test qui
/// s'appuierait sur un semis absent passerait au vert pour une mauvaise raison.
#[tokio::test]
async fn le_semis_ne_fournit_ni_regle_ni_modele() {
    let bac = Bac::monter().await;

    let regles = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM engagement.reminder_rules"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage des règles");
    assert_eq!(regles, 0);

    let modeles =
        sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM engagement.message_templates"#)
            .fetch_one(bac.pool())
            .await
            .expect("comptage des modèles");
    assert_eq!(modeles, 0);

    // Le catalogue des types, lui, est bien semé : c'est une donnée de
    // référence, et les règles s'y adossent par leur `type_code`.
    let types =
        sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM engagement.notification_types"#)
            .fetch_one(bac.pool())
            .await
            .expect("comptage des types");
    assert_eq!(types, 18);
}

/// La fabrique de règle traverse les décalages **en minutes**, dans les deux
/// sens — c'est la forme du contrat du front, et elle évite de lire un
/// `interval[]` à l'œil dans un test rouge.
#[tokio::test]
async fn les_decalages_font_laller_et_le_retour_en_minutes() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let regle = commun::regle_dedition(&bac, terrain.edition, &[2880, 1440, 60, 30]).await;

    let relus = sqlx::query_scalar!(
        r#"SELECT array(SELECT (extract(epoch FROM o) / 60)::int
                          FROM unnest(r.offsets) o) AS "minutes!: Vec<i32>"
             FROM engagement.reminder_rules r WHERE r.id = $1"#,
        regle
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture des décalages");

    assert_eq!(relus, vec![2880, 1440, 60, 30]);
}
