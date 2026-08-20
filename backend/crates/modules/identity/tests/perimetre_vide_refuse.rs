//! FR-048 : les trois cas du périmètre restent distincts.
//!
//! Aucun droit se dit par un **refus**, jamais par une liste vide. Le défaut
//! est ancien et silencieux : un garde qui teste « pas global » confond
//! « aucun droit » et « administrateur d'une seule édition », et affiche une
//! page vide là où il faut fermer la porte.

mod commun;

use commun::{attribuer, semer, semer_evenement, Bac, Compte};
use identity::domain::ids::PersonId;
use identity::service::rbac;
use kernel::error::ErrorCode;

#[tokio::test]
async fn aucun_droit_se_refuse_au_lieu_de_rendre_une_liste_vide() {
    let bac = Bac::monter().await;
    let quidam = semer(&bac, Compte::actif("quidam@example.org")).await;

    let refus = commun::perimetre(&bac, quidam)
        .await
        .expect_err("un compte sans attribution n'ouvre aucune liste du back-office");
    assert_eq!(refus.code, ErrorCode::Forbidden);

    // Et la fonction du modèle, elle, répond toujours : c'est bien une valeur
    // pleine qui dit « aucun droit », pas une absence de réponse.
    let vide = rbac::administered_events(bac.base.pool(), PersonId(quidam))
        .await
        .expect("le périmètre se lit toujours");
    assert!(!vide.is_global);
    assert!(vide.event_ids.is_empty());
}

#[tokio::test]
async fn les_trois_cas_ne_se_confondent_pas() {
    let bac = Bac::monter().await;
    let cop = semer_evenement(&bac, "cop31-belem", "COP31 Belém").await;

    let global = semer(&bac, Compte::actif("global@example.org")).await;
    let detache = semer(&bac, Compte::actif("detache@example.org")).await;
    let sans_droit = semer(&bac, Compte::actif("sans.droit@example.org")).await;

    attribuer(&bac, global, "admin", "global", None).await;
    attribuer(&bac, detache, "admin", "event", Some(cop)).await;

    let vu_global = commun::perimetre(&bac, global)
        .await
        .expect("périmètre global");
    assert!(vu_global.is_global);
    assert!(
        vu_global.allows(cop),
        "un périmètre global couvre les éditions présentes et à venir"
    );

    let vu_detache = commun::perimetre(&bac, detache)
        .await
        .expect("périmètre d'édition");
    assert!(!vu_detache.is_global);
    assert_eq!(vu_detache.event_ids, vec![cop]);
    assert!(!vu_detache.allows(uuid::Uuid::now_v7()), "et rien d'autre");

    assert!(commun::perimetre(&bac, sans_droit).await.is_err());
}

/// Un rôle qui n'apporte pas la permission de lecture des propositions ne donne
/// aucun périmètre d'administration : le filtre de la fonction porte sur elle,
/// et pas sur le nom du rôle.
#[tokio::test]
async fn un_role_sans_le_droit_qui_compte_ne_donne_pas_de_perimetre() {
    let bac = Bac::monter().await;
    let editeur = semer(&bac, Compte::actif("editeur@example.org")).await;

    attribuer(&bac, editeur, "editor", "global", None).await;

    let refus = commun::perimetre(&bac, editeur)
        .await
        .expect_err("un éditeur n'administre aucune édition");
    assert_eq!(refus.code, ErrorCode::Forbidden);

    // Il détient pourtant bien une permission, sur une autre préoccupation :
    // périmètre vide ne veut pas dire compte sans droits.
    let effectives = rbac::effective_permissions(bac.base.pool(), PersonId(editeur))
        .await
        .expect("permissions effectives");
    assert!(effectives
        .iter()
        .any(|p| p.permission_code == "publication.article.moderate"));
    assert!(!vec_contient_lecture_des_propositions(&effectives));
}

fn vec_contient_lecture_des_propositions(
    permissions: &[identity::domain::rbac::EffectivePermission],
) -> bool {
    permissions
        .iter()
        .any(|p| p.permission_code == "programme.proposal.read_all")
}
