//! **Un seul canal par défaut par édition — et le canal général n'est pas
//! délogé** (research.md § R6, SC-015).
//!
//! `ux_broadcast_channels_default` est un index unique **partiel** sur
//! `COALESCE(event_id, …)`, restreint aux canaux `is_default AND is_active`. Il
//! n'est **pas différable** : poser le nouveau défaut avant d'avoir retiré
//! l'ancien viole l'unicité au milieu de la transaction. Retirer d'abord est la
//! seule séquence qui passe, et c'est ce que le service fait.
//!
//! **Deux pièges de cet index, tous deux éprouvés ici :**
//!
//! 1. les canaux **généraux de la plateforme** forment leur propre groupe, sous
//!    un identifiant de substitution — poser un défaut d'édition **ne déloge
//!    pas** celui que le semis pose ;
//! 2. l'index ne porte que sur les canaux **actifs** : un défaut désactivé
//!    libère la place, ce qui est cohérent — un canal inactif n'occupe rien.

mod commun;

use commun::{formulaire_canal, Bac};
use event::domain::ids::{ChannelId, EventId};
use event::service::channels as service_canaux;

#[tokio::test]
async fn un_second_defaut_retire_le_premier() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    for code in ["premier", "second"] {
        let resultat = service_canaux::enregistrer(
            &bac.state,
            &bac.ctx(),
            cop31,
            None,
            formulaire_canal(editions.cop31, code, true),
        )
        .await
        .expect("l'enregistrement d'un canal par défaut aboutit");

        assert!(resultat.ok, "{:?}", resultat.error_code);
    }

    assert_eq!(
        commun::canaux_par_defaut(&bac, editions.cop31).await,
        vec!["second".to_owned()],
        "poser un défaut retire le précédent, dans la même transaction"
    );
}

/// **Le canal général semé n'est pas délogé.** Il est le défaut de son propre
/// groupe — celui des canaux sans édition — et sert les diffusions dont
/// l'événement n'a pas le sien.
#[tokio::test]
async fn le_canal_general_de_la_plateforme_reste_par_defaut() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let (_, defaut_avant, actif_avant) = commun::canal_general(&bac).await;
    assert!(
        defaut_avant && actif_avant,
        "le semis pose un canal général actif et par défaut"
    );

    service_canaux::enregistrer(
        &bac.state,
        &bac.ctx(),
        EventId::from(editions.cop31),
        None,
        formulaire_canal(editions.cop31, "cop31_direct", true),
    )
    .await
    .expect("enregistrement");

    let (_, defaut_apres, actif_apres) = commun::canal_general(&bac).await;
    assert!(
        defaut_apres && actif_apres,
        "un défaut d'édition ne touche pas au groupe des canaux généraux"
    );
}

/// **Deux écritures concurrentes.** L'index n'autorise qu'un défaut actif : si
/// les deux transactions posaient avant de retirer, l'une échouerait sur
/// l'unicité. Ici, l'une attend l'autre et le résultat reste cohérent — **un
/// seul défaut**, quel qu'il soit.
#[tokio::test]
async fn deux_ecritures_concurrentes_laissent_un_seul_defaut() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let (ctx_a, ctx_b) = (bac.ctx(), bac.ctx());
    let (a, b) = tokio::join!(
        service_canaux::enregistrer(
            &bac.state,
            &ctx_a,
            cop31,
            None,
            formulaire_canal(editions.cop31, "concurrent_a", true),
        ),
        service_canaux::enregistrer(
            &bac.state,
            &ctx_b,
            cop31,
            None,
            formulaire_canal(editions.cop31, "concurrent_b", true),
        )
    );

    assert!(
        a.is_ok() && b.is_ok(),
        "aucune des deux n'échoue : {a:?} {b:?}"
    );
    assert_eq!(
        commun::canaux_par_defaut(&bac, editions.cop31).await.len(),
        1,
        "un seul défaut subsiste, quel qu'il soit"
    );
}

/// **Un canal par défaut désactivé libère la place.** L'index ne porte que sur
/// les canaux actifs, et un canal inactif n'occupe rien.
#[tokio::test]
async fn un_defaut_desactive_libere_la_place() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let cree = service_canaux::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        formulaire_canal(editions.cop31, "ancien", true),
    )
    .await
    .expect("enregistrement");
    let ancien = cree
        .detail
        .expect("la composition")
        .channels
        .into_iter()
        .find(|c| c.code == "ancien")
        .expect("le canal créé");

    let mut desactivation = formulaire_canal(editions.cop31, "ancien", false);
    desactivation.is_active = false;
    service_canaux::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        Some(ChannelId::from(ancien.id)),
        desactivation,
    )
    .await
    .expect("désactivation");

    let nouveau = service_canaux::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        formulaire_canal(editions.cop31, "nouveau", true),
    )
    .await
    .expect("le nouveau défaut passe");

    assert!(nouveau.ok, "{:?}", nouveau.error_code);
    assert_eq!(
        commun::canaux_par_defaut(&bac, editions.cop31).await,
        vec!["nouveau".to_owned()]
    );
}
