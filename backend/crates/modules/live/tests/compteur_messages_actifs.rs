//! **Le compteur d'une activité compte les messages ACTIFS DE PORTÉE `session`
//! qui la visent, et rien d'autre.**
//!
//! Il sert à ne pas publier deux fois la même panne. Y compter un message
//! d'édition ou un brouillon ferait croire à un bandeau qui n'existe pas, et
//! l'équipe s'abstiendrait de signaler ce qui n'a pas encore été dit.

mod commun;

use commun::*;

#[tokio::test]
async fn le_compteur_ne_prend_que_les_actifs_de_portee_session() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // Ce qui compte.
    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;
    // Ce qui ne compte pas : un brouillon, un expiré, un retiré, un message
    // d'édition, et un message visant l'AUTRE activité.
    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "draft",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "expired",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "unpublished",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "active",
    )
    .await;
    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.autre_session_id),
        "active",
    )
    .await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    let compte = |id| {
        ecran
            .desk
            .sessions
            .iter()
            .find(|s| s.session_id == id)
            .map(|s| s.active_incident_count)
    };

    assert_eq!(compte(decor.session_id), Some(1));
    assert_eq!(compte(decor.autre_session_id), Some(1));
}
