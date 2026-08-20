//! FR-059 : la file RGPD ne se borne pas par édition. **Portée globale, ou
//! 403.**
//!
//! Une demande d'effacement porte sur la plateforme entière : il n'existe aucune
//! édition à laquelle la rapporter. Rendre à un administrateur d'édition la
//! part de la file qui « le concerne » lui ferait croire qu'il a tout traité,
//! alors qu'il n'aurait vu qu'un morceau — et le délai réglementaire court
//! quand même sur le reste.
//!
//! Le refus éprouvé ici est **celui de la route** : l'extracteur
//! `Requires<PersonManage>` appelle `require_permission(…, Scope::Global)`, et
//! son erreur est un 403. Le test appelle la même fonction, avec les mêmes
//! arguments.

mod commun;

use commun::{attribuer, semer, semer_evenement, Bac, Compte};
use identity::domain::admin_users::PrivacyRequestType;
use identity::domain::ids::PersonId;
use identity::service::privacy;
use kernel::auth::{self, Scope};
use kernel::error::ErrorCode;
use uuid::Uuid;

const GLOBALE: &str = "sophie.mensah@francophonie.org";
const DEDITION: &str = "claire.perret@francophonie.org";
const DEMANDEUSE: &str = "awa.diallo@example.org";

/// Ce que fait l'extracteur de la route avant que le gestionnaire existe.
async fn garde_de_route(bac: &Bac, acteur: Uuid) -> kernel::error::Result<()> {
    auth::require_permission(
        bac.base.pool(),
        acteur,
        "identity.person.manage",
        Scope::Global,
    )
    .await
}

#[tokio::test]
async fn un_administrateur_dedition_recoit_403_jamais_une_file_filtree() {
    let bac = Bac::monter().await;
    let cop = semer_evenement(&bac, "cop31", "COP31").await;

    let dedition = semer(&bac, Compte::actif(DEDITION)).await;
    attribuer(&bac, dedition, "admin", "event", Some(cop)).await;

    // Son périmètre n'est pas vide : il administre bien une édition. C'est
    // exactement le cas piège — un périmètre non vide ne vaut pas la portée
    // globale, et confondre les deux ouvrirait la file.
    let perimetre = commun::perimetre(&bac, dedition)
        .await
        .expect("périmètre non vide");
    assert!(!perimetre.is_global);
    assert_eq!(perimetre.event_ids, vec![cop]);

    let refus = garde_de_route(&bac, dedition)
        .await
        .expect_err("la portée globale est exigée");
    assert_eq!(refus.code, ErrorCode::Forbidden);
    assert_eq!(refus.code.status().as_u16(), 403);
}

#[tokio::test]
async fn un_administrateur_global_voit_la_file_entiere() {
    let bac = Bac::monter().await;
    let cop = semer_evenement(&bac, "cop31", "COP31").await;

    let globale = semer(&bac, Compte::actif(GLOBALE)).await;
    attribuer(&bac, globale, "admin", "global", None).await;
    garde_de_route(&bac, globale).await.expect("portée globale");

    // Deux demandeurs, dont un rattaché à une édition et l'autre à rien : la
    // file les porte tous les deux, sans distinction.
    let attachee = semer(&bac, Compte::actif(DEMANDEUSE)).await;
    attribuer(&bac, attachee, "reviewer", "event", Some(cop)).await;
    let sans_attache = semer(&bac, Compte::actif("karim.ilboudo@example.org")).await;

    for (personne, finalite) in [
        (attachee, PrivacyRequestType::Erasure),
        (sans_attache, PrivacyRequestType::Export),
    ] {
        privacy::submit(
            &bac.state,
            &bac.ctx(),
            globale,
            PersonId(personne),
            finalite,
        )
        .await
        .expect("dépôt de la demande");
    }

    let ecran = privacy::queue_screen(bac.state.pool())
        .await
        .expect("file RGPD");

    assert_eq!(ecran.requests.len(), 2);
    assert_eq!(ecran.open_count, 2);
    assert_eq!(ecran.overdue_count, 0);
    assert!(
        ecran
            .requests
            .iter()
            .any(|r| r.person_id.as_uuid() == sans_attache),
        "la demande d'une personne rattachée à aucune édition est dans la file"
    );
}

/// Le dépôt émet son événement — une seule fois, et sans secret.
#[tokio::test]
async fn un_depot_emet_levenement_de_reception() {
    let bac = Bac::monter().await;
    let globale = semer(&bac, Compte::actif(GLOBALE)).await;
    attribuer(&bac, globale, "admin", "global", None).await;
    let demandeuse = semer(&bac, Compte::actif(DEMANDEUSE)).await;

    let request_id = privacy::submit(
        &bac.state,
        &bac.ctx(),
        globale,
        PersonId(demandeuse),
        PrivacyRequestType::Erasure,
    )
    .await
    .expect("dépôt");

    let ligne = sqlx::query!(
        r#"SELECT aggregate_id, payload FROM platform.outbox_events
            WHERE event_type = 'identity.privacy_request.received'"#
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("un événement, et un seul");

    assert_eq!(ligne.aggregate_id, request_id);
    assert_eq!(ligne.payload["request_type"], "erasure");
    assert!(
        ligne.payload["due_at"].is_string(),
        "l'échéance voyage avec l'événement : c'est elle qui déclenche les rappels"
    );
    assert!(
        ligne.payload.get("person_email").is_none(),
        "aucune adresse dans l'outbox : elle n'est pas le sujet de cet événement"
    );
}
