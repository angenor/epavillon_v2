//! **Ce qui ne parle pas ne sort pas — et une édition inconnue rend une liste
//! vide, jamais 404.**
//!
//! C'est délibéré : cette route ne doit pas devenir un moyen de savoir si une
//! édition existe, et un bandeau absent se lit exactement comme une édition sans
//! incident — ce qui est le cas normal.

mod commun;

use commun::*;

#[tokio::test]
async fn brouillon_expire_et_retire_ne_sortent_pas() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "draft",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "expired",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "unpublished",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "scheduled",
    )
    .await;

    let lignes = live::repo::active::pour_ledition(bac.pool(), decor.event_id)
        .await
        .expect("lecture publique");

    assert!(
        lignes.is_empty(),
        "quatre conditions cumulées décident qu'un bandeau parle — la v1 les oubliait une par une"
    );
}

#[tokio::test]
async fn une_edition_inconnue_rend_une_liste_vide() {
    let bac = Bac::monter().await;

    let lignes = live::repo::active::pour_ledition(bac.pool(), uuid::Uuid::now_v7())
        .await
        .expect("aucune erreur : cette route ne dit pas si une édition existe");

    assert!(lignes.is_empty());
}
