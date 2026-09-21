//! Une seule transaction : l'usage, l'attribution, l'annuaire, l'appartenance
//! et l'événement d'outbox.
//!
//! Principe IV et principe VII. Si l'un des cinq partait seul, la plateforme
//! aurait deux vérités : un usage sans accès, ou un événement pour un accès qui
//! n'existe pas. Le test ne vérifie pas que le code « ouvre une transaction » —
//! il vérifie que les cinq écritures sont **toutes là**, et toutes attribuées au
//! même auteur.

mod commun;

use commun::Bac;
use negotiation::domain::redeem::RedeemIssue;

#[tokio::test]
async fn une_entree_ecrit_les_cinq_choses_ensemble() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    let accepte = commun::saisir(&bac, decor.person_id, &decor.code, Some("appareil-1")).await;
    assert_eq!(accepte.issue, RedeemIssue::Accepted);

    // 1 — l'usage, rattaché au code.
    assert_eq!(commun::usages(&bac, decor.code_id).await, (1, 1));

    // 2 — l'attribution, avec la portée du code.
    assert_eq!(
        commun::attributions(&bac, decor.person_id).await,
        vec![("negotiation_space".to_owned(), Some(decor.space_id))]
    );

    // 3 — l'annuaire de l'espace, qui n'accorde rien mais dit qui en fait partie.
    let dans_lannuaire = sqlx::query_scalar!(
        r#"SELECT EXISTS (
               SELECT 1 FROM negotiation.space_members
                WHERE space_id = $1 AND person_id = $2 AND left_at IS NULL
           ) AS "present!""#,
        decor.space_id,
        decor.person_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'annuaire");
    assert!(dans_lannuaire);

    // 4 — l'appartenance au réseau, avec le code par lequel elle est venue.
    let source = sqlx::query_scalar!(
        "SELECT source_code_id FROM negotiation.network_memberships
          WHERE person_id = $1 AND left_at IS NULL",
        decor.person_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'appartenance");
    assert_eq!(source, Some(decor.code_id));

    // 5 — l'événement, posé sur l'attribution elle-même.
    let attribution = sqlx::query_scalar!(
        "SELECT ra.id FROM identity.role_assignments ra
          WHERE ra.person_id = $1 AND ra.revoked_at IS NULL",
        decor.person_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'attribution");

    assert_eq!(
        commun::evenements(&bac, attribution).await,
        vec!["negotiation.space_access.granted".to_owned()]
    );

    // La charge utile porte l'identifiant du code, **jamais le code lui-même** :
    // l'outbox est durable et faite pour être relue.
    let charge = sqlx::query_scalar!(
        "SELECT payload FROM platform.outbox_events WHERE aggregate_id = $1",
        attribution
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la charge utile");

    assert_eq!(
        charge["invitation_code_id"].as_str(),
        Some(decor.code_id.to_string().as_str())
    );
    assert_eq!(charge["origin"].as_str(), Some("invitation_code"));
    assert!(
        !charge.to_string().contains(&decor.code),
        "le code en clair n'a rien à faire dans l'outbox : {charge}"
    );
}

/// **Toute écriture laisse son auteur** (principe VII, FR-046). Sans le contexte
/// posé par `Db::write`, l'audit serait anonyme et rien ne le signalerait.
#[tokio::test]
async fn lentree_laisse_son_auteur_dans_laudit() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    commun::saisir(&bac, decor.person_id, &decor.code, None).await;

    let auteurs = sqlx::query_scalar!(
        "SELECT DISTINCT actor_id FROM platform.audit_log
          WHERE entity_schema = 'negotiation'
            AND entity_table IN ('network_memberships', 'space_members')",
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'audit");

    assert_eq!(auteurs, vec![Some(decor.person_id)]);
}

/// Un refus n'écrit **que** son essai : ni usage, ni attribution, ni événement.
#[tokio::test]
async fn un_refus_ne_laisse_que_son_essai() {
    let bac = Bac::monter().await;
    let decor = commun::decor(&bac).await;

    let refus = commun::saisir(&bac, decor.person_id, "ZZZZ-999", None).await;
    assert_eq!(refus.issue, RedeemIssue::Unknown);

    assert_eq!(commun::usages(&bac, decor.code_id).await, (0, 0));
    assert!(commun::attributions(&bac, decor.person_id).await.is_empty());
    assert!(commun::reseaux(&bac, decor.person_id).await.is_empty());
    assert_eq!(commun::essais(&bac, decor.person_id).await.len(), 1);
}
