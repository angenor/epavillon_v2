//! **Deux familles ne sont pas filtrées par édition, et ne révèlent l'existence
//! d'aucune autre.**
//!
//! Les doublons d'organisation ne se rattachent à aucune édition ; un message de
//! portée globale s'affiche partout. Les deux remontent pour un compte détaché —
//! et ne nomment que des organisations et des textes, jamais une édition.

mod commun;

use analytics::domain::action::AdminActionKind;
use commun::*;

#[tokio::test]
async fn les_doublons_et_les_messages_globaux_remontent_pour_un_compte_detache() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    sqlx::query!(
        "INSERT INTO org.duplicate_candidates (left_id, right_id, score, reasons)
         VALUES ($1, $2, 150, ARRAY['nom', 'domaine']::text[])",
        decor.organization_id,
        decor.organisation_etrangere
    )
    .execute(bac.pool())
    .await
    .expect("doublon");
    poser(&bac, comptes.globale, "global", None, "active").await;
    rafraichir(&bac).await;

    let perimetre = perimetre_de(&bac, comptes.detache).await;
    let actions = analytics::service::dashboard::ecran(&bac.state, &perimetre, decor.event_id)
        .await
        .expect("tableau de bord")
        .actions;

    let familles: Vec<AdminActionKind> = actions.iter().map(|a| a.kind).collect();
    assert!(familles.contains(&AdminActionKind::OrganizationDuplicates));
    assert!(familles.contains(&AdminActionKind::ActiveIncidents));

    let doublons = actions
        .iter()
        .find(|a| a.kind == AdminActionKind::OrganizationDuplicates)
        .expect("les doublons");
    assert_eq!(doublons.count, 1);
    assert!(
        doublons.examples[0].label.contains('/'),
        "les DEUX dénominations, comme « IFDD / Institut de la Francophonie » — c'est le défaut n° 1 de la v1"
    );
}

#[tokio::test]
async fn une_paire_deja_arbitree_ne_remonte_plus() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    sqlx::query!(
        "INSERT INTO org.duplicate_candidates
             (left_id, right_id, score, reasons, reviewed_at, reviewed_by, decision)
         VALUES ($1, $2, 150, ARRAY['nom']::text[], now(), $3, 'distinct')",
        decor.organization_id,
        decor.organisation_etrangere,
        comptes.globale
    )
    .execute(bac.pool())
    .await
    .expect("doublon arbitré");
    rafraichir(&bac).await;

    let actions = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .actions;

    assert!(
        !actions
            .iter()
            .any(|a| a.kind == AdminActionKind::OrganizationDuplicates),
        "une paire tranchée n'appelle plus rien : la laisser remonter ferait de la liste un journal"
    );
}
