//! **Le poste parle du jour de l'ÉDITION, pas de celui du serveur.**
//!
//! Éprouvé sur un fuseau qui diffère de l'UTC de plusieurs heures : à Belém, un
//! serveur en UTC bascule de jour trois heures trop tôt, et le poste montrerait
//! les activités du lendemain pendant que la salle est encore pleine.

mod commun;

use commun::*;

#[tokio::test]
async fn le_jour_est_celui_du_fuseau_de_ledition() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    let attendu = jour_de_ledition(&bac, decor.event_id).await;
    assert_eq!(
        ecran.desk.day, attendu,
        "(now() AT TIME ZONE events.timezone)::date, calculé en base"
    );
    assert_eq!(ecran.timezone, FUSEAU);
    assert_eq!(
        ecran.zone_label.as_deref(),
        Some("Belém"),
        "la VILLE, pas l'identifiant IANA — « heure de Belém »"
    );
}

#[tokio::test]
async fn deux_editions_de_fuseaux_differents_peuvent_ne_pas_etre_le_meme_jour() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;

    let ici = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("écran de Belém");
    let ailleurs = live::service::list::composer(bac.pool(), decor.autre_event_id, "fr")
        .await
        .expect("écran de Bakou");

    // Onze heures séparent Belém de Bakou : les deux jours coïncident une partie
    // de la journée seulement. Ce qu'on vérifie, c'est que chacun a lu SON
    // fuseau — donc que les deux valeurs viennent bien de la base.
    assert_eq!(ici.desk.day, jour_de_ledition(&bac, decor.event_id).await);
    assert_eq!(
        ailleurs.desk.day,
        jour_de_ledition(&bac, decor.autre_event_id).await
    );
}
