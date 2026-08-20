//! **Retirer un canal qui a servi le désactive — et c'est un succès**
//! (research.md § R7).
//!
//! La clé étrangère est `ON DELETE SET NULL` : aucune séance ne serait perdue
//! par une suppression. Ce qui serait perdu, c'est **la trace du canal sur
//! lequel une activité passée a été diffusée** — précisément ce qu'un bilan
//! d'édition va chercher.
//!
//! `error_code: 'deactivated'` accompagne donc `ok: true`. C'est le seul endroit
//! du module où ce champ ne signale pas une erreur, et l'annotation OpenAPI le
//! dit là où l'on serait tenté de croire l'inverse.

mod commun;

use commun::Bac;
use event::domain::ids::{ChannelId, EventId};
use event::domain::tabs::TabErrorCode;
use event::service::channels as service_canaux;

#[tokio::test]
async fn un_canal_qui_a_diffuse_est_desactive_et_non_supprime() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;
    // La séance est **diffusée** : le modèle lui affecte le canal par défaut de
    // l'édition, qui est justement celui qu'on va tenter de retirer.
    commun::seed::seance(&bac, editions.cop31, &enfants).await;

    let resultat = service_canaux::retirer(
        &bac.state,
        &bac.ctx(),
        EventId::from(editions.cop31),
        ChannelId::from(enfants.canal),
    )
    .await
    .expect("le retrait aboutit");

    assert!(resultat.ok, "la désactivation est un SUCCÈS, pas un refus");
    assert_eq!(resultat.error_code, Some(TabErrorCode::Deactivated));
    assert_eq!(
        resultat.sessions_detached, 1,
        "le chiffre annoncé est celui des séances diffusées sur ce canal"
    );

    let ligne = sqlx::query!(
        "SELECT is_active, is_default FROM event.broadcast_channels WHERE id = $1",
        enfants.canal
    )
    .fetch_optional(bac.pool())
    .await
    .expect("lecture du canal")
    .expect("le canal EXISTE toujours : la trace de la diffusion est conservée");

    assert!(!ligne.is_active, "il est désactivé");
    assert!(
        !ligne.is_default,
        "un canal inactif ne peut pas rester le défaut : l'index ne porte que sur les actifs"
    );

    // Et la séance, elle, n'a pas bougé.
    let seances = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.sessions WHERE broadcast_channel_id = $1"#,
        enfants.canal
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(seances, 1, "aucune séance n'est perdue ni détachée");
}

/// **Un canal qui n'a jamais servi est supprimé.** Laisser s'accumuler des
/// canaux créés par erreur garderait leurs codes pris.
#[tokio::test]
async fn un_canal_qui_na_jamais_servi_est_supprime() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;

    let resultat = service_canaux::retirer(
        &bac.state,
        &bac.ctx(),
        EventId::from(editions.cop31),
        ChannelId::from(enfants.canal),
    )
    .await
    .expect("le retrait aboutit");

    assert!(resultat.ok);
    assert_eq!(
        resultat.error_code, None,
        "une suppression franche ne dit rien de particulier"
    );
    assert_eq!(resultat.sessions_detached, 0);

    let reste = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM event.broadcast_channels WHERE id = $1"#,
        enfants.canal
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(reste, 0, "il est bel et bien supprimé");
}
