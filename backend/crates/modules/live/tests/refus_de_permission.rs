//! **Sans `live.incident.publish` sur la portée visée : 200 `forbidden`, PAS
//! 403.**
//!
//! Le contrat du site nomme cette issue et l'écran la traduit dans son
//! formulaire. Un 403 ferait lever le client, qui afficherait une panne là où il
//! doit poser un message sous un champ.

mod commun;

use commun::*;
use live::domain::incident::IncidentWriteStatus;

#[tokio::test]
async fn les_quatre_ecritures_rendent_forbidden_en_200() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    // `lectrice` administre l'édition mais ne détient pas la permission.
    let perimetre = perimetre_de(&bac, comptes.lectrice).await;
    let ctx = bac.ctx(comptes.lectrice);

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);

    let creation =
        live::service::write::creer(&bac.state, &ctx, &perimetre, decor.event_id, &valeurs)
            .await
            .expect("réponse");
    assert_eq!(creation.status, IncidentWriteStatus::Forbidden);
    assert!(creation.incident.is_none());

    let existant = poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;

    let correction = live::service::write::corriger(
        &bac.state,
        &ctx,
        &perimetre,
        decor.event_id,
        existant,
        &valeurs,
    )
    .await
    .expect("réponse");
    assert_eq!(correction.status, IncidentWriteStatus::Forbidden);

    let publication = live::service::write::publier(&bac.state, &ctx, &perimetre, existant)
        .await
        .expect("réponse");
    assert_eq!(publication.status, IncidentWriteStatus::Forbidden);

    let retrait = live::service::write::depublier(&bac.state, &ctx, &perimetre, existant, None)
        .await
        .expect("réponse");
    assert_eq!(retrait.status, IncidentWriteStatus::Forbidden);
}
