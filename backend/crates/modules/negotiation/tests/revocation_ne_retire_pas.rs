//! **Révoquer un code ne retire aucun accès déjà accordé** — ADR-006, FR-039.
//!
//! C'est la distinction qui coûte le plus cher si on la perd : une fuite de
//! code se referme en le révoquant, et le réseau déjà entré doit garder son
//! accès. Confondre les deux gestes ferait de chaque fuite une exclusion
//! collective.

mod commun;

use commun::{
    a_lacces, attribuer, creer_un_code, decor, etat_du_code, mon_acces, revoquer_le_code, saisir,
    usages_du_code, Bac,
};
use negotiation::domain::access::AccessState;
use negotiation::domain::redeem::RedeemIssue;

#[tokio::test]
async fn revoquer_le_code_laisse_les_acces_deja_accordes() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let code = creer_un_code(
        &bac,
        d.admin_id,
        "Réseau des négociatrices",
        Some(d.space_id),
        Some("women_negotiators"),
    )
    .await;

    let entree = saisir(&bac, d.person_id, &code.code, None).await;
    assert_eq!(entree.issue, RedeemIssue::Accepted);
    assert!(a_lacces(&bac, d.person_id, Some(d.space_id)).await);

    let apres = revoquer_le_code(
        &bac,
        d.admin_id,
        code.id,
        Some("code diffusé hors du groupe"),
    )
    .await
    .expect("le code existe");

    assert_eq!(apres.state, "revoked");
    assert!(
        apres.revoked_at.is_some(),
        "la date de révocation est gardée"
    );
    assert_eq!(
        apres.revoked_reason.as_deref(),
        Some("code diffusé hors du groupe")
    );

    assert!(
        a_lacces(&bac, d.person_id, Some(d.space_id)).await,
        "l'accès accordé avant la révocation tient : révoquer n'est pas retirer"
    );
    assert_eq!(
        mon_acces(&bac, d.person_id).await.state,
        AccessState::Granted
    );

    let usages = usages_du_code(&bac, code.id).await;
    assert_eq!(usages.granted_uses, 1);
    assert!(usages.rows[0].access_active);
}

#[tokio::test]
async fn le_code_revoque_nouvre_plus_des_la_tentative_suivante() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", Some(d.space_id), None).await;
    revoquer_le_code(&bac, d.admin_id, code.id, None).await;

    assert_eq!(etat_du_code(&bac, code.id).await, "revoked");

    // **Le refus dit la date**, et c'est ce qui le distingue d'un code inconnu :
    // sans elle, la personne chercherait une faute de frappe dans un code juste.
    let refus = saisir(&bac, d.person_id, &code.code, None).await;
    assert_eq!(refus.issue, RedeemIssue::Revoked);
    assert!(refus.revoked_at.is_some());
    assert!(!a_lacces(&bac, d.person_id, Some(d.space_id)).await);
}

#[tokio::test]
async fn revoquer_deux_fois_ne_reecrit_ni_la_date_ni_le_motif() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", None, None).await;
    let premier = revoquer_le_code(&bac, d.admin_id, code.id, Some("première raison"))
        .await
        .expect("le code existe");
    let second = revoquer_le_code(&bac, d.admin_id, code.id, Some("seconde raison"))
        .await
        .expect("le code existe toujours");

    assert_eq!(premier.revoked_at, second.revoked_at);
    assert_eq!(second.revoked_reason.as_deref(), Some("première raison"));
}
