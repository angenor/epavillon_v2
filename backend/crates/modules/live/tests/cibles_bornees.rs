//! **On ne peut viser que ce qui appartient à l'édition** — règle métier n° 8.
//!
//! Un administrateur détaché sur la COP31 ne doit pas pouvoir viser une journée
//! d'une autre édition, y compris en forgeant une requête. Et les organisations
//! offertes sont celles qui **animent** une activité de l'édition : le même
//! critère que la portée `organization` du modèle, sans quoi le formulaire
//! proposerait une cible que la lecture écarterait ensuite.

mod commun;

use commun::*;
use time::macros::time;

#[tokio::test]
async fn les_cibles_ne_portent_que_ce_qui_appartient_a_ledition() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;

    // De quoi tenter la fuite : une journée et une activité de l'AUTRE édition.
    let jour_ailleurs = jour_de_ledition(&bac, decor.autre_event_id).await;
    journee(
        &bac,
        decor.autre_event_id,
        jour_ailleurs,
        Some("Journée d'ailleurs"),
    )
    .await;
    activite(
        &bac,
        decor.autre_event_id,
        None,
        Some(decor.organisation_etrangere),
        "Séance d'ailleurs",
        "seance-ailleurs",
        aujourdhui_a(&bac, decor.autre_event_id, time!(09:00)).await,
        aujourdhui_a(&bac, decor.autre_event_id, time!(10:00)).await,
    )
    .await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    assert_eq!(ecran.targets.event.id, decor.event_id);
    assert_eq!(ecran.targets.days.len(), 1, "la seule journée de l'édition");
    assert_eq!(ecran.targets.days[0].id, decor.jour_id);
    assert_eq!(ecran.targets.sessions.len(), 2, "les deux activités");
    assert!(ecran
        .targets
        .sessions
        .iter()
        .all(|s| s.id == decor.session_id || s.id == decor.autre_session_id));

    // **Seulement celles qui animent** : l'organisation étrangère anime ailleurs.
    assert_eq!(ecran.targets.organizations.len(), 1);
    assert_eq!(ecran.targets.organizations[0].id, decor.organization_id);
}

#[tokio::test]
async fn une_activite_porte_son_debut_comme_instant_et_aucune_precision() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    let activite = &ecran.targets.sessions[0];
    assert!(
        activite.starts_at.is_some(),
        "un INSTANT, que l'interface affichera dans le fuseau de l'édition"
    );
    assert!(
        activite.hint.is_none(),
        "et aucune précision textuelle : les mélanger avait fait apparaître un horodatage brut dans une liste déroulante"
    );
    assert!(!activite.label.is_empty(), "le libellé est résolu");
}
