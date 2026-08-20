//! Principe VII : **une écriture sans contexte n'échoue pas.**
//!
//! Elle écrit une trace anonyme, et rien ne le signale — ni erreur, ni
//! avertissement, ni compteur. C'est le seul défaut du socle qu'aucun mécanisme
//! ne rattrape : la seule porte d'écriture pose le contexte elle-même, mais rien
//! n'empêche d'ouvrir une transaction par le pool. D'où ce test, qui joue le
//! cycle d'administration complet et fouille l'audit après coup.
//!
//! **Une exception existe, et elle est légitime** : une personne qui s'inscrit
//! n'a pas encore d'identifiant au moment où sa ligne est écrite. Le second test
//! la nomme, précisément pour qu'elle reste la seule.

mod commun;

use commun::{attribuer, semer, semer_evenement, Bac, Compte, MOT_DE_PASSE};
use identity::domain::admin_users::PrivacyRequestType;
use identity::domain::ids::{PersonId, RoleAssignmentId};
use identity::domain::login::PersonStatus;
use identity::domain::privacy::PrivacyAction;
use identity::service::admin_users::{self, GrantRequest, StatusRequest};
use identity::service::registration::{self, RegisterRequest};
use identity::service::{password_reset, privacy};
use kernel::auth::ScopeType;
use time::OffsetDateTime;
use uuid::Uuid;

const ACTRICE: &str = "sophie.mensah@francophonie.org";
const SUJET: &str = "awa.diallo@example.org";

/// Les traces anonymes écrites depuis un instant donné, nommées par leur table.
async fn traces_sans_auteur(bac: &Bac, depuis: OffsetDateTime) -> Vec<String> {
    sqlx::query!(
        r#"SELECT entity_schema AS "schema!", entity_table AS "table!", action AS "action!"
             FROM platform.audit_log
            WHERE occurred_at >= $1 AND actor_id IS NULL
            ORDER BY occurred_at"#,
        depuis
    )
    .fetch_all(bac.base.pool())
    .await
    .expect("lecture de l'audit")
    .into_iter()
    .map(|l| format!("{}.{} ({})", l.schema, l.table, l.action))
    .collect()
}

#[tokio::test]
async fn aucune_ecriture_du_cycle_dadministration_ne_perd_son_auteur() {
    let bac = Bac::monter().await;
    let cop = semer_evenement(&bac, "cop31", "COP31").await;

    let actrice = semer(&bac, Compte::actif(ACTRICE)).await;
    attribuer(&bac, actrice, "admin", "global", None).await;
    let sujet = semer(&bac, Compte::actif(SUJET)).await;

    let perimetre = commun::perimetre(&bac, actrice)
        .await
        .expect("périmètre global");

    // Le semis est fait. Tout ce qui suit passe par un service, avec un acteur.
    let depart = OffsetDateTime::now_utc();

    let attribution = admin_users::grant_role(
        &bac.state,
        &bac.ctx(),
        actrice,
        GrantRequest {
            person_id: PersonId(sujet),
            role_code: "reviewer",
            scope_type: ScopeType::Event,
            scope_id: Some(cop),
            valid_from: None,
            valid_until: None,
            note: Some("Comité de lecture COP31"),
        },
    )
    .await
    .expect("attribution");
    assert_eq!(attribution.status, "granted");
    let attribution_id = attribution.assignment.expect("attribution rendue").id;

    admin_users::revoke_role(
        &bac.state,
        &bac.ctx(),
        actrice,
        RoleAssignmentId(attribution_id.as_uuid()),
        "Fin de mandat",
    )
    .await
    .expect("retrait");

    admin_users::set_status(
        &bac.state,
        &bac.ctx(),
        actrice,
        &perimetre,
        StatusRequest {
            person_id: PersonId(sujet),
            status: PersonStatus::Suspended,
            reason: "Propos déplacés en session",
            suspended_until: Some(OffsetDateTime::now_utc() + time::Duration::days(15)),
            revoke_sessions: true,
        },
    )
    .await
    .expect("suspension");

    // Réinitialisation : la personne n'a **pas de session**, et son identifiant
    // sort du jeton consommé — donc de l'intérieur de la transaction. C'est le
    // cas que `kernel::db::set_actor()` existe pour couvrir.
    password_reset::request(&bac.state, &bac.ctx(), SUJET)
        .await
        .expect("demande de lien");
    let jeton = sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'identity.send_password_reset_email' ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail")
    .expect("jeton en clair");
    password_reset::confirm(&bac.state, &bac.ctx(), &jeton, "Kinshasa2028!")
        .await
        .expect("réinitialisation");

    // Effacement : `identity.anonymize_person()` écrit sur `identity.people` en
    // SECURITY DEFINER, et lit l'acteur par `platform.current_actor_id()`. Si le
    // contexte n'était pas posé, la trace la plus lourde du module serait
    // anonyme.
    let demande = privacy::submit(
        &bac.state,
        &bac.ctx(),
        actrice,
        PersonId(sujet),
        PrivacyRequestType::Erasure,
    )
    .await
    .expect("dépôt");
    privacy::handle(
        &bac.state,
        &bac.ctx(),
        actrice,
        demande,
        PrivacyAction::Anonymize,
        Some("Demande d'effacement du 20/08"),
    )
    .await
    .expect("effacement");

    let anonymes = traces_sans_auteur(&bac, depart).await;
    assert!(
        anonymes.is_empty(),
        "des écritures ont perdu leur auteur : {anonymes:?}"
    );

    // Et l'auteur est **nommé**, pas seulement identifié : la fiche d'une
    // personne anonymisée doit rester lisible six mois plus tard.
    let sans_nom = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.audit_log
            WHERE occurred_at >= $1 AND actor_label IS NULL"#,
        depart
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(sans_nom, 0);

    // Le motif voyage avec la trace : sans lui, l'audit dit qu'un statut a
    // changé sans dire pourquoi — c'est le défaut que le modèle a corrigé.
    let motifs = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.audit_log
            WHERE occurred_at >= $1
              AND entity_table = 'people'
              AND new_data ->> 'status_reason' IS NOT NULL"#,
        depart
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage des motifs");
    assert!(motifs >= 2, "suspension et effacement portent leur motif");
}

/// L'unique trace légitimement anonyme : la personne qui s'inscrit n'a pas
/// encore d'identifiant quand sa ligne est écrite. Écrire ici « acteur = la
/// personne créée » serait un mensonge — elle n'était pas authentifiée.
///
/// Le test la borne : **une seule** ligne, sur **sa propre** création.
#[tokio::test]
async fn la_seule_trace_anonyme_est_linscription_de_soi_meme() {
    let bac = Bac::monter().await;
    let depart = OffsetDateTime::now_utc();

    registration::register(
        &bac.state,
        &bac.ctx(),
        RegisterRequest {
            first_name: "Awa",
            last_name: "Diallo",
            email: SUJET,
            country_id: None,
            password: MOT_DE_PASSE,
            preferred_locale: "fr",
            timezone: "Africa/Dakar",
        },
    )
    .await
    .expect("inscription");

    let inscrite: Uuid = sqlx::query_scalar!(
        "SELECT id FROM identity.people WHERE primary_email = $1::text::platform.email",
        SUJET
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("relecture de la personne");

    let anonymes = sqlx::query!(
        r#"SELECT entity_table AS "table!", action AS "action!", entity_id
             FROM platform.audit_log
            WHERE occurred_at >= $1 AND actor_id IS NULL"#,
        depart
    )
    .fetch_all(bac.base.pool())
    .await
    .expect("lecture de l'audit");

    assert_eq!(anonymes.len(), 1, "une seule trace anonyme");
    assert_eq!(anonymes[0].table, "people");
    assert_eq!(anonymes[0].action, "insert");
    assert_eq!(anonymes[0].entity_id, Some(inscrite));
}
