//! **EXACTEMENT UNE ligne d'outbox par publication, et une par retrait.**
//!
//! `live.publish_incident()` et `live.unpublish_incident()` émettent déjà, dans
//! la transaction de l'appelant. Le crate n'émet rien — et c'est le piège n° 1
//! des six modules précédents, nommé avant d'être commis.
//!
//! **Le jour où un `emit_event` est ajouté « pour faire comme les autres », le
//! compte double et ce test casse.** C'est tout son objet : vérifier la
//! *présence* d'un événement ne dirait rien d'un doublon.

mod commun;

use commun::*;

#[tokio::test]
async fn une_publication_emet_exactement_un_evenement() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);
    valeurs.publish = true;

    let resultat = live::service::write::creer(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        decor.event_id,
        &valeurs,
    )
    .await
    .expect("écriture");
    let id = resultat.incident.expect("ligne").incident_id;

    let emis = evenements_emis(&bac, id).await;
    assert_eq!(
        emis,
        vec!["live.incident.published"],
        "une seule ligne : la base émet, le service se tait"
    );
}

#[tokio::test]
async fn un_retrait_emet_exactement_un_evenement_de_plus() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let id = poser(
        &bac,
        comptes.detache,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;
    assert_eq!(evenements_emis(&bac, id).await.len(), 1);

    live::service::write::depublier(
        &bac.state,
        &bac.ctx(comptes.detache),
        &perimetre,
        id,
        Some("Rétabli."),
    )
    .await
    .expect("retrait");

    assert_eq!(
        evenements_emis(&bac, id).await,
        vec!["live.incident.published", "live.incident.resolved"],
        "deux lignes en tout, pas quatre"
    );
}

#[tokio::test]
async fn un_brouillon_nemet_rien() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let id = poser(
        &bac,
        comptes.detache,
        "session",
        Some(decor.session_id),
        "draft",
    )
    .await;

    assert!(
        evenements_emis(&bac, id).await.is_empty(),
        "rien n'a été dit à personne : il n'y a rien à annoncer"
    );
}
