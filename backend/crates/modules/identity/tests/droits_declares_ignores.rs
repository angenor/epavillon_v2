//! FR-055 : **un client qui déclare ses propres droits n'est pas un contrôle
//! d'accès.**
//!
//! Le site passe encore deux choses à ses fonctions d'écriture — l'identifiant
//! de l'acteur et la liste de ses permissions. C'était la seule façon de rejouer
//! l'autorisation sur des données simulées, et son propre commentaire annonce
//! leur disparition. Ils ne franchissent déjà pas le réseau ; ce test dit ce qui
//! se passerait s'ils le franchissaient : **rien**.
//!
//! Trois propriétés, et elles se tiennent à des endroits différents :
//!
//! - les charges utiles **ne déclarent aucun champ de droits**, donc rien ne les
//!   lit — et elles ne refusent pas non plus la requête, un champ inconnu
//!   s'ignore ;
//! - la personne visée vient de **l'URL**, jamais du corps ;
//! - l'autorisation se lit **en base, pour l'acteur de la session**.

mod commun;

use commun::{attribuer, semer, semer_evenement, Bac, Compte};
use identity::domain::ids::PersonId;
use identity::routes::admin_users::{GrantRolePayload, RevokeRolePayload, SetPersonStatusPayload};
use identity::service::admin_users::{self, GrantRequest};
use kernel::auth::ScopeType;
use serde_json::json;
use uuid::Uuid;

/// Une charge utile telle qu'un client malveillant l'écrirait : la vraie
/// demande, plus tout ce qu'il aimerait qu'on croie sur lui.
fn charge_bavarde(usurpe: Uuid, cible: Uuid) -> serde_json::Value {
    json!({
        "role_code": "super_admin",
        "scope_type": "global",
        "scope_id": null,
        "note": "je me nomme moi-même",
        // Ce que le site passe encore à sa fonction, et que personne ne lit.
        "person_id": cible,
        "actor_id": usurpe,
        "granted": [
            { "permission_code": "identity.role.assign", "scope_type": "global", "scope_id": null }
        ],
        "is_super_admin": true
    })
}

#[test]
fn les_champs_de_droits_se_lisent_sans_effet_et_sans_refus() {
    let corps = charge_bavarde(Uuid::now_v7(), Uuid::now_v7());

    let demande: GrantRolePayload =
        serde_json::from_value(corps).expect("un champ inconnu s'ignore, il ne fait pas échouer");

    assert_eq!(demande.role_code, "super_admin");
    assert_eq!(demande.scope_type, ScopeType::Global);
    // Rien d'autre n'a été retenu : la structure ne porte ni acteur, ni
    // permissions, ni personne visée. Le compilateur le garantit mieux qu'une
    // assertion — celle-ci ne fait que le rendre lisible.
    assert_eq!(demande.note.as_deref(), Some("je me nomme moi-même"));
}

#[test]
fn les_deux_autres_ecritures_ignorent_les_memes_champs() {
    let retrait: RevokeRolePayload = serde_json::from_value(json!({
        "reason": "fin de mission",
        "assignment_id": Uuid::now_v7(),
        "actor_id": Uuid::now_v7(),
        "granted": []
    }))
    .expect("désérialisation");
    assert_eq!(retrait.reason, "fin de mission");

    let statut: SetPersonStatusPayload = serde_json::from_value(json!({
        "status": "suspended",
        "reason": "propos déplacés",
        "suspended_until": "2027-01-01T00:00:00Z",
        "revoke_sessions": true,
        "person_id": Uuid::now_v7(),
        "actor_id": Uuid::now_v7(),
        "scope": { "is_global": true, "event_ids": [] }
    }))
    .expect("désérialisation");
    assert!(statut.revoke_sessions);
}

/// **Le statut `anonymized` n'est pas posable, et c'est le type qui le dit.**
/// L'effacement passe par la fonction de la base, qui purge l'identité et émet
/// son propre événement ; le poser ici marquerait une fiche comme effacée sans
/// rien effacer.
#[test]
fn le_statut_deffacement_ne_se_pose_pas_par_cette_route() {
    let refus = serde_json::from_value::<SetPersonStatusPayload>(json!({
        "status": "anonymized",
        "reason": "demande RGPD"
    }));
    assert!(refus.is_err());
}

/// L'autorisation se lit en base **pour l'acteur de la session**. Ce que le
/// corps raconte ne change rien.
#[tokio::test]
async fn declarer_ses_droits_ne_les_donne_pas() {
    let bac = Bac::monter().await;
    let cop = semer_evenement(&bac, "cop31-belem", "COP31 Belém").await;

    let patron = semer(&bac, Compte::actif("patronne@example.org")).await;
    attribuer(&bac, patron, "super_admin", "global", None).await;

    let quelconque = semer(&bac, Compte::actif("quelconque@example.org")).await;
    let cible = semer(&bac, Compte::actif("awa.diallo@example.org")).await;

    // La charge utile est celle d'un client qui se déclare super administrateur.
    // Seuls les champs que la structure connaît en sortent.
    let demande: GrantRolePayload =
        serde_json::from_value(charge_bavarde(patron, cible)).expect("désérialisation");

    let issue = admin_users::grant_role(
        &bac.state,
        &bac.ctx(),
        // L'acteur vient de la session, pas du corps.
        quelconque,
        GrantRequest {
            // La personne visée vient de l'URL, pas du corps.
            person_id: PersonId(cible),
            role_code: &demande.role_code,
            scope_type: demande.scope_type,
            scope_id: demande.scope_id,
            valid_from: demande.valid_from,
            valid_until: demande.valid_until,
            note: demande.note.as_deref(),
        },
    )
    .await
    .expect("attribution");

    assert_eq!(
        issue.status, "forbidden_scope",
        "les droits déclarés par le client n'en donnent aucun"
    );

    let ecrites = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.role_assignments WHERE person_id = $1"#,
        cible
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(ecrites, 0);

    // Et pour être sûr que le décor n'était pas la cause du refus : la même
    // écriture, par qui en a réellement le droit, aboutit.
    let issue = admin_users::grant_role(
        &bac.state,
        &bac.ctx(),
        patron,
        GrantRequest {
            person_id: PersonId(cible),
            role_code: "admin",
            scope_type: ScopeType::Event,
            scope_id: Some(cop),
            valid_from: None,
            valid_until: None,
            note: None,
        },
    )
    .await
    .expect("attribution");
    assert_eq!(issue.status, "granted");
}
