//! **Toute écriture du module nomme son auteur** — FR-046, principe VII.
//!
//! La trace ne demande aucun code : `platform.tg_audit()` est déjà posé sur les
//! tables, et `Db::write(&ctx)` pose `app.actor_id` avant la première écriture.
//! Ce que ce fichier garde, c'est qu'aucun service n'ouvre sa transaction par
//! une connexion nue — auquel cas la trace existerait, mais anonyme, et la
//! question « qui a retiré cet accès ? » resterait sans réponse six mois plus
//! tard.

mod commun;

use commun::{
    attribuer, attribution_de, creer_un_code, decor, retirer_lacces, revoquer_le_code, saisir,
    traces, Bac,
};

#[tokio::test]
async fn creer_revoquer_et_retirer_nomment_leur_auteur() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", Some(d.space_id), None).await;
    saisir(&bac, d.person_id, &code.code, None).await;
    let attribution = attribution_de(&bac, d.person_id, Some(d.space_id)).await;

    revoquer_le_code(&bac, d.admin_id, code.id, Some("fuite")).await;
    retirer_lacces(&bac, d.admin_id, code.id, d.person_id, Some("abus")).await;

    let sur_le_code = traces(&bac, "invitation_codes", code.id).await;
    assert!(
        sur_le_code
            .iter()
            .any(|(action, acteur)| action == "insert" && *acteur == Some(d.admin_id)),
        "la création porte son auteur : {sur_le_code:?}"
    );
    assert!(
        sur_le_code
            .iter()
            .any(|(action, acteur)| action == "update" && *acteur == Some(d.admin_id)),
        "la révocation aussi : {sur_le_code:?}"
    );

    let sur_lattribution = traces(&bac, "role_assignments", attribution).await;
    assert!(
        sur_lattribution
            .iter()
            .any(|(action, acteur)| action == "insert" && *acteur == Some(d.person_id)),
        "l'attribution venue d'un code porte la personne entrée : {sur_lattribution:?}"
    );
    assert!(
        sur_lattribution
            .iter()
            .any(|(action, acteur)| action == "update" && *acteur == Some(d.admin_id)),
        "et son retrait porte l'administrateur qui l'a décidé : {sur_lattribution:?}"
    );

    assert!(
        sur_le_code
            .iter()
            .chain(sur_lattribution.iter())
            .all(|(_, acteur)| acteur.is_some()),
        "aucune écriture anonyme : une transaction ouverte hors de `Db::write` en produirait"
    );
}
