//! **Sans activité aujourd'hui, le poste montre les quatre prochaines — et le
//! dit.**
//!
//! « Rien aujourd'hui » et « voici la suite » ne sont pas la même information :
//! les confondre ferait croire à un direct en cours hors période. `day` reste
//! donc **aujourd'hui**, et `is_fallback` porte la différence.

mod commun;

use commun::*;
use time::macros::time;

#[tokio::test]
async fn sans_activite_aujourdhui_le_repli_porte_les_quatre_prochaines() {
    let bac = Bac::monter().await;
    // Décor nu : une édition, aucune activité aujourd'hui.
    let event_id = edition(&bac, "edition-repli", "Édition du repli", FUSEAU, "Belém").await;

    // Cinq activités à venir : le poste n'en montre que quatre.
    for (i, heure) in [
        time!(09:00),
        time!(11:00),
        time!(13:00),
        time!(15:00),
        time!(17:00),
    ]
    .into_iter()
    .enumerate()
    {
        let debut = aujourdhui_a(&bac, event_id, heure).await + time::Duration::days(3);
        activite(
            &bac,
            event_id,
            None,
            None,
            &format!("Activité {i}"),
            &format!("activite-repli-{i}"),
            debut,
            debut + time::Duration::hours(1),
        )
        .await;
    }

    let ecran = live::service::list::composer(bac.pool(), event_id, "fr")
        .await
        .expect("composition de l'écran");

    assert!(ecran.desk.is_fallback, "le repli est annoncé, pas deviné");
    assert_eq!(
        ecran.desk.day,
        jour_de_ledition(&bac, event_id).await,
        "le jour reste AUJOURD'HUI : l'écran dit « rien aujourd'hui, voici la suite »"
    );
    assert_eq!(ecran.desk.sessions.len(), 4, "les quatre prochaines");

    let debuts: Vec<_> = ecran.desk.sessions.iter().map(|s| s.starts_at).collect();
    let mut tries = debuts.clone();
    tries.sort_unstable();
    assert_eq!(debuts, tries, "par début croissant");
}

#[tokio::test]
async fn avec_une_activite_aujourdhui_il_ny_a_pas_de_repli() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;

    let ecran = live::service::list::composer(bac.pool(), decor.event_id, "fr")
        .await
        .expect("composition de l'écran");

    assert!(!ecran.desk.is_fallback);
    assert_eq!(ecran.desk.sessions.len(), 2, "les deux activités du jour");
}
