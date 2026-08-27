//! **La cible est rendue par son NOM, et une journée sans titre par sa date.**
//!
//! C'est `live.event_incidents()` qui résout, et nulle part ailleurs : le
//! back-office affiche « Atelier de négociation », jamais un identifiant.

mod commun;

use commun::*;

#[tokio::test]
async fn la_cible_porte_le_nom_de_lactivite() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    assert_eq!(
        ecran.rows[0].target_label.as_deref(),
        Some("Atelier de négociation")
    );
}

#[tokio::test]
async fn une_journee_sans_titre_est_designee_par_sa_date() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let demain = decor.jour_date.next_day().expect("jour suivant");
    let jour_muet = journee(&bac, decor.event_id, demain, None).await;
    poser(
        &bac,
        comptes.globale,
        "event_day",
        Some(jour_muet),
        "active",
    )
    .await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    let attendu = format!(
        "{:02}/{:02}/{}",
        demain.day(),
        demain.month() as u8,
        demain.year()
    );
    assert_eq!(
        ecran.rows[0].target_label.as_deref(),
        Some(attendu.as_str()),
        "JJ/MM/AAAA, comme le modèle le fait"
    );
}
