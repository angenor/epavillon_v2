//! **Les trois refus de validation sortent en 200, chacun sur son champ.**
//!
//! Le contrat du site range dix issues sous un seul discriminant, et l'écran les
//! traduit une par une sous le champ concerné. Répondre 422 ferait lever le
//! client là où il attend un message posé dans son formulaire.

mod commun;

use commun::*;
use live::domain::incident::IncidentWriteStatus;
use serde_json::json;

async fn ecrire(
    bac: &Bac,
    decor: &Decor,
    acteur: uuid::Uuid,
    valeurs: &live::domain::payload::IncidentPayload,
) -> IncidentWriteStatus {
    let perimetre = perimetre_de(bac, acteur).await;
    live::service::write::creer(
        &bac.state,
        &bac.ctx(acteur),
        &perimetre,
        decor.event_id,
        valeurs,
    )
    .await
    .expect("l'écriture répond toujours")
    .status
}

#[tokio::test]
async fn une_cible_manquante_rend_missing_target() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // Portée `session`, aucune activité désignée.
    let valeurs = payload("session");
    assert_eq!(
        ecrire(&bac, &decor, comptes.detache, &valeurs).await,
        IncidentWriteStatus::MissingTarget
    );
}

#[tokio::test]
async fn un_message_dans_une_seule_langue_rend_missing_message() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);
    // La base n'exige qu'un document non nul — et c'est voulu, les données
    // reprises de la v1 n'ont qu'une langue. L'API, elle, exige les deux : un
    // bandeau publié maintenant s'adresse à une COP bilingue.
    valeurs.message = json!({ "fr": "Seulement en français." });

    assert_eq!(
        ecrire(&bac, &decor, comptes.detache, &valeurs).await,
        IncidentWriteStatus::MissingMessage
    );
}

#[tokio::test]
async fn une_fenetre_inversee_rend_invalid_window() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);
    valeurs.display_until = Some(valeurs.display_from - time::Duration::hours(1));

    assert_eq!(
        ecrire(&bac, &decor, comptes.detache, &valeurs).await,
        IncidentWriteStatus::InvalidWindow
    );
}

#[tokio::test]
async fn une_fin_daffichage_nulle_est_legitime() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let mut valeurs = payload("session");
    valeurs.session_id = Some(decor.session_id);
    valeurs.display_until = None;

    assert_eq!(
        ecrire(&bac, &decor, comptes.detache, &valeurs).await,
        IncidentWriteStatus::Created,
        "« jusqu'à dépublication explicite » est le vrai danger de la table, que l'interface signale — pas un refus"
    );
}
