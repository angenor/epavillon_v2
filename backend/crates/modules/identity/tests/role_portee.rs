//! FR-053 : la permission d'attribution se vérifie **sur la portée visée**, et
//! le retrait exige la même permission **sur la portée de l'attribution**.
//!
//! Sans la seconde moitié, une administratrice détachée sur une édition pourrait
//! défaire un rôle global qu'elle n'aurait jamais pu accorder — et le contrat
//! l'écrit noir sur blanc parce que c'est exactement ce qu'on oublie.
//!
//! **Le refus du trigger ressort avec son message français** *(obligation n° 3
//! du principe X)* : le modèle sait déjà dire pourquoi un rôle n'admet pas une
//! portée, et le reformuler ici produirait un second libellé qui se périmerait.

mod commun;

use commun::{attribuer, semer, semer_evenement, Bac, Compte};
use identity::domain::ids::{PersonId, RoleAssignmentId};
use identity::service::admin_users::{self, GrantRequest};
use kernel::auth::ScopeType;
use uuid::Uuid;

struct Decor {
    bac: Bac,
    cop_a: Uuid,
    cop_b: Uuid,
    admin_a: Uuid,
    patron: Uuid,
    cible: Uuid,
}

/// Deux éditions, une administratrice détachée sur la première, une
/// administratrice globale, et quelqu'un à qui confier un rôle.
async fn planter() -> Decor {
    let bac = Bac::monter().await;

    let cop_a = semer_evenement(&bac, "cop31-belem", "COP31 Belém").await;
    let cop_b = semer_evenement(&bac, "cop16-riyad", "COP16 Riyad").await;

    let admin_a = semer(&bac, Compte::actif("admin.a@example.org")).await;
    let patron = semer(&bac, Compte::actif("patronne@example.org")).await;
    let cible = semer(&bac, Compte::actif("awa.diallo@example.org")).await;

    attribuer(&bac, admin_a, "admin", "event", Some(cop_a)).await;
    attribuer(&bac, patron, "super_admin", "global", None).await;

    Decor {
        bac,
        cop_a,
        cop_b,
        admin_a,
        patron,
        cible,
    }
}

async fn attribuer_par(
    decor: &Decor,
    acteur: Uuid,
    role: &str,
    scope_type: ScopeType,
    scope_id: Option<Uuid>,
) -> identity::domain::scope::RoleWriteOutcome {
    admin_users::grant_role(
        &decor.bac.state,
        &decor.bac.ctx(),
        acteur,
        GrantRequest {
            person_id: PersonId(decor.cible),
            role_code: role,
            scope_type,
            scope_id,
            valid_from: None,
            valid_until: None,
            note: Some("comité de sélection"),
        },
    )
    .await
    .expect("attribution")
}

#[tokio::test]
async fn une_administratrice_dedition_attribue_sur_sa_portee() {
    let decor = planter().await;

    let issue = attribuer_par(
        &decor,
        decor.admin_a,
        "reviewer",
        ScopeType::Event,
        Some(decor.cop_a),
    )
    .await;

    assert_eq!(issue.status, "granted");
    let posee = issue.assignment.expect("l'attribution est rendue");
    assert_eq!(posee.role_code, "reviewer");
    assert_eq!(posee.scope_id, Some(decor.cop_a));
    assert_eq!(posee.granted_by, Some(PersonId(decor.admin_a)));
    assert_eq!(posee.note.as_deref(), Some("comité de sélection"));
    assert_eq!(
        issue.assignments.len(),
        1,
        "l'écran se recale sans recharger"
    );

    let evenements = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events
            WHERE event_type = 'identity.role.granted'"#
    )
    .fetch_one(decor.bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(evenements, 1);
}

/// Scénario 2 de l'histoire : ailleurs et globalement, l'écriture est refusée.
#[tokio::test]
async fn elle_ne_peut_attribuer_ni_sur_une_autre_edition_ni_globalement() {
    let decor = planter().await;

    for (portee, cible) in [
        (ScopeType::Event, Some(decor.cop_b)),
        (ScopeType::Global, None),
    ] {
        let issue = attribuer_par(&decor, decor.admin_a, "admin", portee, cible).await;
        assert_eq!(
            issue.status, "forbidden_scope",
            "portée {portee:?} : l'acteur n'a le droit que sur son édition"
        );
        assert!(issue.assignments.is_empty(), "rien n'a été écrit");
    }

    let ecrites = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.role_assignments WHERE person_id = $1"#,
        decor.cible
    )
    .fetch_one(decor.bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(ecrites, 0);
}

/// **Obligation n° 3 du principe X** : l'invariant est porté par la base, et le
/// service en traduit le refus au lieu de le redoubler.
#[tokio::test]
async fn le_refus_du_trigger_ressort_avec_son_message_francais() {
    let decor = planter().await;

    // `super_admin` n'admet que la portée globale. L'actrice est globale : c'est
    // bien le RÔLE que le trigger refuse, pas le droit de l'appelante.
    let issue = attribuer_par(
        &decor,
        decor.patron,
        "super_admin",
        ScopeType::Event,
        Some(decor.cop_a),
    )
    .await;

    assert_eq!(issue.status, "scope_not_allowed");
    let message = issue.message.expect("le message de la base est rendu");
    assert!(
        message.contains("super_admin")
            && message.contains("ne peut pas être attribué sur la portée")
            && message.contains("global"),
        "le texte du trigger est repris tel quel : {message}"
    );
}

/// `ux_role_assignments_active` ne filtre que sur `revoked_at IS NULL` : le
/// doublon nomme la ligne en place pour que l'écran puisse la montrer.
#[tokio::test]
async fn deux_fois_le_meme_role_sur_la_meme_portee_est_un_doublon() {
    let decor = planter().await;

    attribuer_par(
        &decor,
        decor.admin_a,
        "reviewer",
        ScopeType::Event,
        Some(decor.cop_a),
    )
    .await;

    let issue = attribuer_par(
        &decor,
        decor.admin_a,
        "reviewer",
        ScopeType::Event,
        Some(decor.cop_a),
    )
    .await;

    assert_eq!(issue.status, "duplicate");
    let conflit = issue
        .conflict_with
        .expect("l'attribution en place est nommée");
    assert_eq!(conflit.role_code, "reviewer");
    assert_eq!(conflit.scope_id, Some(decor.cop_a));

    let ecrites = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.role_assignments WHERE person_id = $1"#,
        decor.cible
    )
    .fetch_one(decor.bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(ecrites, 1, "le refus n'a rien laissé derrière lui");
}

/// **Le retrait pose trois colonnes ; il ne supprime jamais.** C'est ce qui
/// répond, six mois plus tard, à « pourquoi cette personne n'est-elle plus au
/// comité ? ».
#[tokio::test]
async fn le_retrait_marque_la_ligne_avec_son_auteur_et_son_motif() {
    let decor = planter().await;

    let posee = attribuer_par(
        &decor,
        decor.admin_a,
        "reviewer",
        ScopeType::Event,
        Some(decor.cop_a),
    )
    .await
    .assignment
    .expect("attribution");

    let issue = admin_users::revoke_role(
        &decor.bac.state,
        &decor.bac.ctx(),
        decor.admin_a,
        posee.id,
        "fin de mission",
    )
    .await
    .expect("retrait");

    assert_eq!(issue.status, "revoked");
    assert!(
        issue.assignments.is_empty(),
        "l'attribution n'est plus en cours"
    );

    let ligne = sqlx::query!(
        "SELECT revoked_at, revoked_by, revoked_reason, note
           FROM identity.role_assignments WHERE id = $1",
        posee.id.as_uuid()
    )
    .fetch_one(decor.bac.base.pool())
    .await
    .expect("la ligne existe toujours");

    assert!(ligne.revoked_at.is_some());
    assert_eq!(ligne.revoked_by, Some(decor.admin_a));
    assert_eq!(ligne.revoked_reason.as_deref(), Some("fin de mission"));
    assert_eq!(
        ligne.note.as_deref(),
        Some("comité de sélection"),
        "le motif de l'octroi n'est pas écrasé par celui du retrait"
    );

    let evenements = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events
            WHERE event_type = 'identity.role.revoked'"#
    )
    .fetch_one(decor.bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(evenements, 1);
}

/// La symétrie, et c'est elle qu'on oublie : retirer exige **le droit qu'il
/// aurait fallu pour accorder**, sur la portée de l'attribution visée.
#[tokio::test]
async fn retirer_un_role_global_exige_le_droit_global() {
    let decor = planter().await;

    let globale = attribuer_par(&decor, decor.patron, "editor", ScopeType::Global, None).await;
    let posee = globale.assignment.expect("attribution globale");

    let refus = admin_users::revoke_role(
        &decor.bac.state,
        &decor.bac.ctx(),
        decor.admin_a,
        posee.id,
        "je n'aime pas ce rôle",
    )
    .await
    .expect("retrait");
    assert_eq!(refus.status, "forbidden_scope");

    let toujours_la = sqlx::query_scalar!(
        "SELECT revoked_at FROM identity.role_assignments WHERE id = $1",
        posee.id.as_uuid()
    )
    .fetch_one(decor.bac.base.pool())
    .await
    .expect("relecture");
    assert!(toujours_la.is_none(), "rien n'a été retiré");

    // La même écriture, par qui en a le droit.
    let issue = admin_users::revoke_role(
        &decor.bac.state,
        &decor.bac.ctx(),
        decor.patron,
        posee.id,
        "fin de mission",
    )
    .await
    .expect("retrait");
    assert_eq!(issue.status, "revoked");
}

#[tokio::test]
async fn un_identifiant_inconnu_ne_retire_rien() {
    let decor = planter().await;

    let issue = admin_users::revoke_role(
        &decor.bac.state,
        &decor.bac.ctx(),
        decor.patron,
        RoleAssignmentId(Uuid::now_v7()),
        "peu importe",
    )
    .await
    .expect("retrait");
    assert_eq!(issue.status, "not_found");
}

/// FR-057 : le panneau n'offre que ce que l'appelant peut réellement accorder.
/// Ce qui est hors d'atteinte reste **visible et désactivé** — le taire ferait
/// croire à un bogue à qui cherche une édition qu'il sait présente.
#[tokio::test]
async fn les_options_dattribution_se_bornent_a_ce_que_lacteur_peut_accorder() {
    let decor = planter().await;
    let pool = decor.bac.base.pool();

    let detachee = admin_users::role_options(pool, "fr", decor.admin_a)
        .await
        .expect("options");

    assert!(!detachee.can_assign_global);
    assert_eq!(detachee.grantable_event_ids, vec![decor.cop_a]);
    assert!(
        detachee
            .roles
            .iter()
            .all(|r| r.allowed_scopes.contains(&ScopeType::Event)),
        "un rôle qu'elle ne peut poser nulle part ne lui est pas proposé"
    );
    assert!(
        !detachee.roles.iter().any(|r| r.code == "super_admin"),
        "super_admin n'admet que la portée globale"
    );

    let sur_a = detachee
        .events
        .iter()
        .find(|c| c.scope_id == decor.cop_a)
        .expect("son édition est offerte");
    let sur_b = detachee
        .events
        .iter()
        .find(|c| c.scope_id == decor.cop_b)
        .expect("l'autre édition reste visible");
    assert!(!sur_a.disabled);
    assert!(sur_b.disabled, "visible, jamais sélectionnable");

    let globale = admin_users::role_options(pool, "fr", decor.patron)
        .await
        .expect("options");
    assert!(globale.can_assign_global);
    assert!(
        globale.grantable_event_ids.is_empty(),
        "la portée globale les couvre déjà toutes"
    );
    assert!(globale.events.iter().all(|c| !c.disabled));
    assert!(globale.roles.iter().any(|r| r.code == "super_admin"));
}

/// Une portée ciblée sans cible n'est pas testable : il n'y a rien à interroger.
/// Le refus porte donc **le code que la base rendrait**.
#[tokio::test]
async fn une_portee_ciblee_sans_cible_est_refusee_avant_lautorisation() {
    let decor = planter().await;

    let erreur = admin_users::grant_role(
        &decor.bac.state,
        &decor.bac.ctx(),
        decor.patron,
        GrantRequest {
            person_id: PersonId(decor.cible),
            role_code: "reviewer",
            scope_type: ScopeType::Event,
            scope_id: None,
            valid_from: None,
            valid_until: None,
            note: None,
        },
    )
    .await
    .expect_err("refus attendu");

    assert_eq!(
        erreur.code,
        kernel::error::ErrorCode::IdentityRoleScopeMismatch
    );
    assert_eq!(erreur.field.as_deref(), Some("scope_id"));
}
