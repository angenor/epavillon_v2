//! **`published_by` porte l'identifiant de la personne, jamais NULL.**
//!
//! `live.publish_incident()` le lit de `platform.current_actor_id()`. Une
//! écriture qui contournerait `Db::write()` produirait un `published_by` **nul,
//! sans erreur** — et le back-office afficherait « publié par — ». Le défaut ne
//! se verrait qu'à la relecture d'un message publié, c'est-à-dire trop tard.
//!
//! **Vérifié sur la valeur de la colonne, pas sur l'audit** : l'audit dirait
//! qu'une écriture a eu lieu, pas qu'elle a posé le bon acteur.

mod commun;

use commun::*;

#[tokio::test]
async fn la_colonne_published_by_porte_lacteur() {
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

    assert_eq!(
        publie_par(&bac, id).await,
        Some(comptes.detache),
        "la colonne, pas l'audit"
    );
}

#[tokio::test]
async fn une_publication_depuis_la_ligne_de_liste_pose_aussi_lacteur() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;
    let perimetre = perimetre_de(&bac, comptes.detache).await;

    let id = poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "draft",
    )
    .await;

    live::service::write::publier(&bac.state, &bac.ctx(comptes.detache), &perimetre, id)
        .await
        .expect("publication");

    assert_eq!(publie_par(&bac, id).await, Some(comptes.detache));
}
