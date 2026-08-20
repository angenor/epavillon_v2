//! **Siéger au comité n'accorde aucun droit** (FR-072, écart n° 88).
//!
//! Le commentaire du modèle le dit en toutes lettres : `event.call_reviewers`
//! porte la **composition**, l'autorisation restant portée par
//! `identity.role_assignments` sur la portée de l'édition. Le service **n'attribue
//! aucun rôle** en ajoutant un membre ; il se contente de **signaler** que la
//! personne ne détient pas la permission d'évaluer.
//!
//! Laisser croire l'inverse coûterait cher dans les deux sens : un évaluateur
//! sans droits qui ne peut pas ouvrir les dossiers qu'on lui a confiés, ou un
//! droit accordé en silence par un geste que personne ne perçoit comme une
//! attribution.
//!
//! **Et les deux permissions du module s'éprouvent séparément, dans les deux
//! sens.** Détenir `event.event.manage` n'accorde pas `event.call.manage`, et
//! réciproquement. Aujourd'hui aucun rôle du catalogue ne porte l'une sans
//! l'autre — c'est l'écart n° 88, consigné et non corrigé —, et c'est justement
//! pourquoi ce test attribue les permissions **par des rôles construits pour
//! lui** : sans cela, il ne prouverait rien.

mod commun;

use commun::{formulaire_appel, Bac};
use event::domain::ids::{CallId, EventId};
use event::domain::permissions::{CALL_MANAGE, EVENT_MANAGE};
use event::domain::tabs::{CommitteePayload, CommitteeSeat};
use event::service::{call as service_appel, committee as service_comite};
use kernel::auth::Scope;
use uuid::Uuid;

#[tokio::test]
async fn ajouter_quelquun_au_comite_nattribue_aucun_role() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = commun::auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let appel = service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        formulaire_appel(editions.cop31, "cop31"),
    )
    .await
    .expect("création de l'appel")
    .call
    .expect("l'appel créé");

    let membre = commun::personne(&bac, "membre@ifdd.francophonie.org", "Yann", "Corbeil").await;

    let roles_avant = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.role_assignments WHERE person_id = $1"#,
        membre
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(roles_avant, 0);

    let resultat = service_comite::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        CallId::from(appel.id),
        CommitteePayload {
            call_id: None,
            members: vec![CommitteeSeat {
                person_id: membre,
                is_lead: false,
                workload_cap: None,
            }],
        },
    )
    .await
    .expect("ajout");

    let roles_apres = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.role_assignments WHERE person_id = $1"#,
        membre
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(
        roles_apres, 0,
        "aucune attribution de rôle n'est posée : siéger n'accorde rien"
    );

    assert_eq!(resultat.members.len(), 1);
    assert!(
        !resultat.members[0].has_review_permission,
        "l'écran doit pouvoir DIRE que cette personne ne peut pas encore évaluer"
    );
}

/// **Les deux permissions du module se testent séparément, dans les deux sens.**
#[tokio::test]
async fn detenir_lune_des_deux_permissions_naccorde_pas_lautre() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    // Deux rôles construits pour ce test, chacun ne portant qu'une des deux
    // permissions. Le catalogue n'en offre aucun de ce genre — c'est l'écart
    // n° 88 —, et sans eux le test ne prouverait rien.
    for (role, permission) in [
        ("test_evenements", EVENT_MANAGE),
        ("test_appels", CALL_MANAGE),
    ] {
        sqlx::query!(
            r#"INSERT INTO identity.roles (code, label, description, allowed_scopes)
               VALUES ($1::text, jsonb_build_object('fr', $1::text),
                       '{"fr":"Rôle de test"}'::jsonb, '{global,event}')"#,
            role
        )
        .execute(bac.pool())
        .await
        .expect("insertion du rôle");

        sqlx::query!(
            "INSERT INTO identity.role_permissions (role_code, permission_code) VALUES ($1, $2)",
            role,
            permission
        )
        .execute(bac.pool())
        .await
        .expect("attribution de la permission au rôle");
    }

    let decor = commun::personne(&bac, "decor@ifdd.francophonie.org", "Léa", "Marchand").await;
    commun::attribuer(
        &bac,
        decor,
        "test_evenements",
        "event",
        Some(editions.cop31),
    )
    .await;

    let campagne = commun::personne(&bac, "campagne@ifdd.francophonie.org", "Omar", "Diagne").await;
    commun::attribuer(&bac, campagne, "test_appels", "event", Some(editions.cop31)).await;

    let sur_ledition = Scope::Event(editions.cop31);

    // Celle qui tient le décor ne tient pas la campagne.
    assert!(a_la_permission(&bac, decor, EVENT_MANAGE, sur_ledition).await);
    assert!(
        !a_la_permission(&bac, decor, CALL_MANAGE, sur_ledition).await,
        "gérer les événements n'accorde pas de gérer les appels"
    );

    // Et réciproquement.
    assert!(a_la_permission(&bac, campagne, CALL_MANAGE, sur_ledition).await);
    assert!(
        !a_la_permission(&bac, campagne, EVENT_MANAGE, sur_ledition).await,
        "gérer les appels n'accorde pas de gérer les événements"
    );
}

async fn a_la_permission(bac: &Bac, personne: Uuid, permission: &str, portee: Scope) -> bool {
    kernel::auth::has_permission(bac.pool(), personne, permission, portee)
        .await
        .expect("lecture de la permission")
}
