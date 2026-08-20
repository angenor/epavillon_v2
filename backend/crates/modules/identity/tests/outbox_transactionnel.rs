//! Obligation n° 4 du principe X : **l'écriture dans l'outbox**.
//!
//! Un événement de domaine naît dans la transaction du changement d'état. Deux
//! moitiés à éprouver, et la seconde est celle qu'on oublie : un changement
//! d'état écrit **un** événement, et une transaction annulée n'en laisse
//! **aucun**. Un outbox qui survivrait à un `ROLLBACK` annoncerait au reste de
//! la plateforme un changement qui n'a pas eu lieu — et rien, ensuite, ne le
//! rattraperait.

mod commun;

use commun::{semer, Bac, Compte, MOT_DE_PASSE};
use identity::domain::ids::PersonId;
use identity::domain::login::PersonStatus;
use identity::service::admin_users::{self, StatusRequest};
use identity::service::registration::{self, RegisterRequest};
use kernel::events::{self, DomainEvent};
use uuid::Uuid;

const ADRESSE: &str = "awa.diallo@example.org";
const ADMINISTRATRICE: &str = "claire.perret@francophonie.org";

async fn evenements(bac: &Bac, event_type: &str) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events WHERE event_type = $1"#,
        event_type
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage des événements")
}

#[tokio::test]
async fn un_changement_detat_ecrit_exactement_un_evenement() {
    let bac = Bac::monter().await;

    registration::register(
        &bac.state,
        &bac.ctx(),
        RegisterRequest {
            first_name: "Awa",
            last_name: "Diallo",
            email: ADRESSE,
            country_id: None,
            password: MOT_DE_PASSE,
            preferred_locale: "fr",
            timezone: "Africa/Dakar",
        },
    )
    .await
    .expect("inscription");

    assert_eq!(evenements(&bac, "identity.person.registered").await, 1);
}

/// La moitié qu'on oublie. L'événement est écrit **puis** la transaction est
/// abandonnée : l'outbox doit être vide. S'il ne l'était pas, il aurait été
/// écrit hors transaction — un canal parallèle, que le modèle a précisément
/// refusé.
#[tokio::test]
async fn une_transaction_annulee_ne_laisse_aucun_evenement() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;

    let db = bac.db();
    let mut tx = db.write(&bac.ctx()).await.expect("transaction");
    events::emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: "identity",
            aggregate_type: "person",
            aggregate_id: personne,
            event_type: "identity.person.status_changed",
            payload: serde_json::json!({ "person_id": personne }),
        },
    )
    .await
    .expect("émission");
    tx.rollback().await.expect("abandon");

    assert_eq!(
        evenements(&bac, "identity.person.status_changed").await,
        0,
        "l'outbox vit dans la transaction, pas à côté"
    );
}

/// Le même invariant, mais par un vrai chemin de service : la base refuse une
/// suspension sans terme, la transaction est abandonnée — donc ni changement de
/// statut, ni événement.
#[tokio::test]
async fn un_refus_de_la_base_ne_laisse_ni_etat_ni_evenement() {
    let bac = Bac::monter().await;
    let sujet = semer(&bac, Compte::actif(ADRESSE)).await;
    let actrice = semer(&bac, Compte::actif(ADMINISTRATRICE)).await;
    commun::attribuer(&bac, actrice, "admin", "global", None).await;

    let perimetre = commun::perimetre(&bac, actrice)
        .await
        .expect("périmètre global");

    let issue = admin_users::set_status(
        &bac.state,
        &bac.ctx(),
        actrice,
        &perimetre,
        StatusRequest {
            person_id: PersonId(sujet),
            status: PersonStatus::Suspended,
            reason: "Sans terme, donc refusée par la base",
            suspended_until: None,
            revoke_sessions: false,
        },
    )
    .await
    .expect("appel du service");

    assert_eq!(issue.status, "missing_deadline");
    assert_eq!(evenements(&bac, "identity.person.status_changed").await, 0);
    assert_eq!(statut(&bac, sujet).await, "active", "rien n'a changé");
}

/// Une suspension acceptée écrit **un** événement, et la reposer à l'identique
/// n'en écrit pas de second : c'est le changement d'état qui déclenche, pas
/// l'appel.
#[tokio::test]
async fn reposer_le_meme_statut_nemet_rien_de_plus() {
    let bac = Bac::monter().await;
    let sujet = semer(&bac, Compte::actif(ADRESSE)).await;
    let actrice = semer(&bac, Compte::actif(ADMINISTRATRICE)).await;
    commun::attribuer(&bac, actrice, "admin", "global", None).await;
    let perimetre = commun::perimetre(&bac, actrice)
        .await
        .expect("périmètre global");

    let terme = time::OffsetDateTime::now_utc() + time::Duration::days(15);
    for _ in 0..2 {
        let issue = admin_users::set_status(
            &bac.state,
            &bac.ctx(),
            actrice,
            &perimetre,
            StatusRequest {
                person_id: PersonId(sujet),
                status: PersonStatus::Suspended,
                reason: "Propos déplacés en session",
                suspended_until: Some(terme),
                revoke_sessions: false,
            },
        )
        .await
        .expect("suspension");
        assert_eq!(issue.status, "saved");
    }

    assert_eq!(
        evenements(&bac, "identity.person.status_changed").await,
        1,
        "un changement d'état, un événement — pas un par appel"
    );
}

async fn statut(bac: &Bac, person_id: Uuid) -> String {
    sqlx::query_scalar!(
        r#"SELECT status::text AS "status!" FROM identity.people WHERE id = $1"#,
        person_id
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du statut")
}
