//! **`ck_incidents_scope_target` vérifie la cohérence portée/cible, JAMAIS
//! l'appartenance à une édition.**
//!
//! La base est muette là-dessus : la règle « la cible appartient à l'édition
//! depuis laquelle on agit » est une règle de service, et l'écrire n'est donc
//! pas une réimplémentation d'invariant.

mod commun;

use commun::*;
use live::domain::incident::IncidentWriteStatus;

#[tokio::test]
async fn une_journee_dune_autre_edition_rend_missing_target() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.globale).await;

    let jour_ailleurs = jour_de_ledition(&bac, decor.autre_event_id).await;
    let journee_etrangere =
        journee(&bac, decor.autre_event_id, jour_ailleurs, Some("Ailleurs")).await;

    let mut valeurs = payload("event_day");
    valeurs.event_day_id = Some(journee_etrangere);

    let resultat = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.globale),
        &perimetre,
        decor.event_id,
        &valeurs,
    )
    .await
    .expect("réponse");

    assert_eq!(resultat.status, IncidentWriteStatus::MissingTarget);
}

#[tokio::test]
async fn une_organisation_qui_nanime_pas_ledition_rend_missing_target() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.globale).await;

    let mut valeurs = payload("organization");
    valeurs.organization_id = Some(decor.organisation_etrangere);

    let resultat = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.globale),
        &perimetre,
        decor.event_id,
        &valeurs,
    )
    .await
    .expect("réponse");

    assert_eq!(
        resultat.status,
        IncidentWriteStatus::MissingTarget,
        "même critère que la portée `organization` du modèle : elle doit y ANIMER"
    );
}

#[tokio::test]
async fn deux_cibles_a_la_fois_rendent_missing_target() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.globale).await;

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);
    valeurs.event_day_id = Some(decor.jour_id);

    let resultat = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.globale),
        &perimetre,
        decor.event_id,
        &valeurs,
    )
    .await
    .expect("réponse");

    assert_eq!(resultat.status, IncidentWriteStatus::MissingTarget);
}
