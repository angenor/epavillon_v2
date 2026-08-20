//! **Un seul appel par édition — zéro s'il n'y a pas de pavillon** (règle métier
//! n° 5).
//!
//! La cardinalité est tenue par `ux_calls_one_per_event`, jamais par
//! l'application : le service ne va pas compter les appels avant d'écrire, il
//! laisse l'index refuser et **traduit** son refus en `already_exists`.
//!
//! **Et l'index exclut les annulés.** C'est écrit dans le SQL, et c'est ce qui
//! permet de repartir après une annulation sans effacer l'historique. Un test
//! qui ne vérifierait que le refus laisserait passer une traduction trop large,
//! qui interdirait de rouvrir une campagne annulée.

mod commun;

use commun::{auteur, formulaire_appel, Bac};
use event::domain::call::CallErrorCode;
use event::domain::ids::{CallId, EventId};
use event::service::call as service_appel;

#[tokio::test]
async fn un_second_appel_est_refuse_et_lannule_ne_compte_pas() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let premier = service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        formulaire_appel(editions.cop31, "cop31"),
    )
    .await
    .expect("la création d'un premier appel aboutit");
    assert!(premier.ok, "premier appel : {:?}", premier.errors);

    // Un second appel, avec un code différent : c'est bien la CARDINALITÉ qui
    // refuse, pas l'unicité du code.
    let second = service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        formulaire_appel(editions.cop31, "cop31_bis"),
    )
    .await
    .expect("un refus de cardinalité est une réponse, pas une erreur HTTP");

    assert!(!second.ok, "une édition ne porte qu'un appel");
    assert!(second.call.is_none(), "rien n'a été écrit");
    assert_eq!(second.errors.len(), 1, "{:?}", second.errors);
    assert_eq!(second.errors[0].code, CallErrorCode::AlreadyExists);
    assert_eq!(
        second.errors[0].field, None,
        "le refus porte sur l'appel entier, pas sur un champ"
    );

    // On annule le premier, puis on recrée : l'index partiel doit laisser faire.
    let call_id = CallId::from(premier.call.expect("l'appel créé").id);
    let mut annulation = formulaire_appel(editions.cop31, "cop31");
    annulation.status = "cancelled".to_owned();
    let annule =
        service_appel::modifier(&bac.state, &bac.ctx(), acteur, cop31, call_id, annulation)
            .await
            .expect("l'annulation aboutit");
    assert!(annule.ok, "annulation : {:?}", annule.errors);
    assert_eq!(
        annule.call.as_ref().map(|c| c.status.as_str()),
        Some("cancelled"),
        "l'appel annulé est rendu, et non perdu"
    );

    let recree = service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        formulaire_appel(editions.cop31, "cop31_2"),
    )
    .await
    .expect("la recréation après annulation aboutit");

    assert!(
        recree.ok,
        "un appel ANNULÉ n'occupe pas la place : {:?}",
        recree.errors
    );
}

/// L'édition sans pavillon n'est pas empêchée d'avoir un appel — le modèle ne
/// l'interdit pas —, mais elle en porte tout autant un seul.
#[tokio::test]
async fn chaque_edition_a_son_propre_appel() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;

    for edition in [editions.cop31, editions.sans_pavillon] {
        let resultat = service_appel::creer(
            &bac.state,
            &bac.ctx(),
            acteur,
            EventId::from(edition),
            formulaire_appel(edition, "campagne"),
        )
        .await
        .expect("création");

        assert!(
            resultat.ok,
            "l'unicité porte sur l'édition, pas sur la plateforme : {:?}",
            resultat.errors
        );
    }
}
