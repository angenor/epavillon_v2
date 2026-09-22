//! **Un accès retiré cesse d'ouvrir dès la lecture suivante** — FR-041, SC-005.
//!
//! L'application garde son état en mémoire locale : c'est la lecture de
//! `me/access` qui fait foi, et elle doit basculer sans délai ni cache. « Mon
//! accès » dit alors « accès retiré » — jamais « visiteuse », qui laisserait
//! croire à un compte neuf.

mod commun;

use commun::{
    a_lacces, attribuer, creer_un_code, decor, mon_acces, retirer_lacces, retirer_tous_les_acces,
    saisir, usages_du_code, Bac,
};
use negotiation::domain::access::AccessState;

#[tokio::test]
async fn un_acces_retire_cesse_douvrir_et_se_dit() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", Some(d.space_id), None).await;
    saisir(&bac, d.person_id, &code.code, None).await;
    assert!(a_lacces(&bac, d.person_id, Some(d.space_id)).await);

    let retire = retirer_lacces(
        &bac,
        d.admin_id,
        code.id,
        d.person_id,
        Some("sortie du réseau"),
    )
    .await;
    assert!(retire);

    assert!(!a_lacces(&bac, d.person_id, Some(d.space_id)).await);

    let apres = mon_acces(&bac, d.person_id).await;
    assert_eq!(
        apres.state,
        AccessState::Revoked,
        "« accès retiré », et non « visiteuse » : ce n'est pas un compte neuf"
    );
    assert!(apres.granted.is_none());

    let usages = usages_du_code(&bac, code.id).await;
    assert_eq!(usages.granted_uses, 0);
    assert!(!usages.rows[0].access_active);
    assert!(usages.rows[0].access_revoked_at.is_some());
    assert_eq!(
        usages.rows[0].access_revoked_reason.as_deref(),
        Some("sortie du réseau")
    );
}

#[tokio::test]
async fn retirer_deux_fois_ne_retire_rien_la_seconde() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", Some(d.space_id), None).await;
    saisir(&bac, d.person_id, &code.code, None).await;

    assert!(retirer_lacces(&bac, d.admin_id, code.id, d.person_id, None).await);
    assert!(
        !retirer_lacces(&bac, d.admin_id, code.id, d.person_id, None).await,
        "deux administrateurs peuvent agir à la seconde près, et ce n'est pas une erreur"
    );
}

#[tokio::test]
async fn le_retrait_en_bloc_sort_tout_le_monde_dune_seule_portee() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let autre = commun::personne(&bac, "fatou.sow@example.org").await;

    let code = creer_un_code(&bac, d.admin_id, "Réseau", Some(d.space_id), None).await;
    saisir(&bac, d.person_id, &code.code, None).await;
    saisir(&bac, autre, &code.code, None).await;

    // Un troisième compte, entré par un code de portée globale : il n'est pas
    // concerné par ce retrait, et son accès doit tenir.
    let large = commun::personne(&bac, "kofi.mensah@example.org").await;
    let global = creer_un_code(&bac, d.admin_id, "Guide Négo en entier", None, None).await;
    saisir(&bac, large, &global.code, None).await;

    let retires = retirer_tous_les_acces(&bac, d.admin_id, code.id).await;
    assert_eq!(retires, 2);

    assert!(!a_lacces(&bac, d.person_id, Some(d.space_id)).await);
    assert!(!a_lacces(&bac, autre, Some(d.space_id)).await);
    assert!(
        a_lacces(&bac, large, None).await,
        "l'accès global vient d'un autre code : il n'est pas retiré"
    );

    assert_eq!(
        retirer_tous_les_acces(&bac, d.admin_id, code.id).await,
        0,
        "le second passage ne retire rien : les accès étaient déjà tombés"
    );
}
