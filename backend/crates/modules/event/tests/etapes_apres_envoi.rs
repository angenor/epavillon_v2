//! « Ce qui se passe après l'envoi » : la base pose le circuit par défaut à la
//! création, le back-office le réécrit, et un champ vidé fait disparaître le
//! bloc — il ne revient pas au défaut.

mod commun;

use commun::{auteur, formulaire_appel, Bac};
use event::domain::ids::{CallId, EventId};
use event::service::call as service_appel;
use serde_json::json;

#[tokio::test]
async fn defaut_a_la_creation_puis_reecrit_puis_retire() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let cree = service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        formulaire_appel(editions.cop31, "cop31"),
    )
    .await
    .expect("création");
    let appel = cree.call.expect("l'appel créé");
    let defaut = appel.submission_next_steps.expect("le défaut du modèle");
    assert!(defaut["fr"]
        .as_str()
        .is_some_and(|t| t.lines().count() == 3));

    let call_id = CallId::from(appel.id);
    let etapes = json!({ "fr": "Première étape.\nSeconde étape.", "en": "First step." });
    let mut charge = formulaire_appel(editions.cop31, "cop31");
    charge.submission_next_steps = Some(etapes.clone());
    let modifie = service_appel::modifier(&bac.state, &bac.ctx(), acteur, cop31, call_id, charge)
        .await
        .expect("modification");
    assert_eq!(
        modifie.call.and_then(|c| c.submission_next_steps),
        Some(etapes)
    );

    let vide = formulaire_appel(editions.cop31, "cop31");
    let retire = service_appel::modifier(&bac.state, &bac.ctx(), acteur, cop31, call_id, vide)
        .await
        .expect("retrait");
    assert!(retire.ok, "{:?}", retire.errors);
    assert_eq!(retire.call.and_then(|c| c.submission_next_steps), None);
}

#[tokio::test]
async fn creation_avec_etapes_propres() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;

    let etapes = json!({ "fr": "Une seule étape." });
    let mut charge = formulaire_appel(editions.cop31, "cop31");
    charge.submission_next_steps = Some(etapes.clone());
    let cree = service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        EventId::from(editions.cop31),
        charge,
    )
    .await
    .expect("création");

    assert_eq!(
        cree.call.and_then(|c| c.submission_next_steps),
        Some(etapes)
    );
}
