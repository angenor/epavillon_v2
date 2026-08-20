//! FR-060 et FR-061 : l'effacement ne répond qu'à une demande d'effacement, et
//! il ne détruit **que** l'identité.
//!
//! Trois choses se perdraient sans ce test :
//!
//! - une demande d'export anonymisée détruirait une identité que personne n'a
//!   demandé d'effacer, et rien ne la rendrait ;
//! - `identity.anonymize_person()` **émet elle-même** son événement — c'est le
//!   piège n° 1 du module. En émettre un second passerait sans erreur, et deux
//!   lignes annonceraient la même histoire à qui relit l'outbox ;
//! - les compteurs de participation d'une COP passée ne doivent pas s'effondrer
//!   parce qu'une personne exerce son droit. C'est la raison d'être de
//!   l'anonymisation plutôt que de la suppression.

mod commun;

use commun::{attribuer, connexion, semer, semer_evenement, Bac, Compte};
use identity::domain::admin_users::{PrivacyRequestStatus, PrivacyRequestType};
use identity::domain::ids::PersonId;
use identity::domain::privacy::{PrivacyAction, DEADLINE_DAYS};
use identity::service::privacy;
use uuid::Uuid;

const GLOBALE: &str = "sophie.mensah@francophonie.org";
const DEMANDEUSE: &str = "awa.diallo@example.org";

struct Terrain {
    bac: Bac,
    actrice: Uuid,
    demandeuse: Uuid,
    session_id: Uuid,
}

/// Une administratrice globale, une personne inscrite à une session — donc une
/// participation à compter —, et rien d'autre.
async fn dresser() -> Terrain {
    let bac = Bac::monter().await;
    let cop = semer_evenement(&bac, "cop31", "COP31").await;

    let actrice = semer(&bac, Compte::actif(GLOBALE)).await;
    attribuer(&bac, actrice, "admin", "global", None).await;

    let demandeuse = semer(&bac, Compte::actif(DEMANDEUSE)).await;

    let session_id: Uuid = sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, format, starts_at, ends_at, timezone)
           VALUES ($1, jsonb_build_object('fr', 'Atelier adaptation'),
                   'atelier-adaptation'::text::platform.slug, 'online',
                   now() + interval '31 days', now() + interval '31 days 2 hours',
                   'America/Belem'::platform.timezone_name)
           RETURNING id"#,
        cop
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("insertion de la session");

    sqlx::query!(
        "INSERT INTO programme.registrations (session_id, person_id, joined_at, attendance_minutes)
         VALUES ($1, $2, now(), 95)",
        session_id,
        demandeuse
    )
    .execute(bac.base.pool())
    .await
    .expect("insertion de l'inscription");

    Terrain {
        bac,
        actrice,
        demandeuse,
        session_id,
    }
}

async fn deposer(t: &Terrain, finalite: PrivacyRequestType) -> Uuid {
    privacy::submit(
        &t.bac.state,
        &t.bac.ctx(),
        t.actrice,
        PersonId(t.demandeuse),
        finalite,
    )
    .await
    .expect("dépôt de la demande")
}

async fn participations(t: &Terrain) -> (i64, Option<i32>) {
    let l = sqlx::query!(
        r#"SELECT count(*) AS "n!", max(attendance_minutes) AS "minutes?"
             FROM programme.registrations WHERE session_id = $1"#,
        t.session_id
    )
    .fetch_one(t.bac.base.pool())
    .await
    .expect("comptage des inscriptions");
    (l.n, l.minutes)
}

#[tokio::test]
async fn lanonymisation_est_refusee_sur_une_demande_dexport() {
    let t = dresser().await;
    let demande = deposer(&t, PrivacyRequestType::Export).await;

    let issue = privacy::handle(
        &t.bac.state,
        &t.bac.ctx(),
        t.actrice,
        demande,
        PrivacyAction::Anonymize,
        Some("Erreur de manipulation"),
    )
    .await
    .expect("appel du service");

    assert_eq!(issue.status, "wrong_type");
    assert_eq!(
        issue.request.expect("la demande est rendue").status,
        PrivacyRequestStatus::Received,
        "le refus ne fait pas avancer la demande"
    );

    let statut = sqlx::query_scalar!(
        r#"SELECT status::text AS "s!" FROM identity.people WHERE id = $1"#,
        t.demandeuse
    )
    .fetch_one(t.bac.base.pool())
    .await
    .expect("relecture");
    assert_eq!(statut, "active", "rien n'a été effacé");
}

#[tokio::test]
async fn un_effacement_purge_lidentite_et_laisse_les_compteurs_intacts() {
    let t = dresser().await;

    // Une session ouverte, pour vérifier qu'elle est coupée.
    let ouverte = connexion(&t.bac, DEMANDEUSE).await;
    assert_eq!(commun::sessions_vivantes(&t.bac, t.demandeuse).await, 1);

    let avant = participations(&t).await;
    let demande = deposer(&t, PrivacyRequestType::Erasure).await;

    let issue = privacy::handle(
        &t.bac.state,
        &t.bac.ctx(),
        t.actrice,
        demande,
        PrivacyAction::Anonymize,
        Some("Demande d'effacement du 20/08"),
    )
    .await
    .expect("effacement");

    assert_eq!(issue.status, "anonymized");
    assert_eq!(
        issue.request.expect("la demande est rendue").status,
        PrivacyRequestStatus::Completed
    );

    // L'identité a disparu, l'identifiant technique est resté.
    let personne = sqlx::query!(
        r#"SELECT status::text AS "statut!", primary_email::text AS "email!",
                  last_name, phone, city, is_directory_visible
             FROM identity.people WHERE id = $1"#,
        t.demandeuse
    )
    .fetch_one(t.bac.base.pool())
    .await
    .expect("la ligne survit : c'est elle qui porte les agrégats");

    assert_eq!(personne.statut, "anonymized");
    assert!(personne.email.starts_with("anonymized+"));
    assert!(personne.last_name.starts_with("anonymisé"));
    assert!(personne.phone.is_none() && personne.city.is_none());
    assert!(!personne.is_directory_visible);

    // Comptes supprimés, sessions révoquées : le jeton d'hier n'ouvre plus rien.
    let comptes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.accounts WHERE person_id = $1"#,
        t.demandeuse
    )
    .fetch_one(t.bac.base.pool())
    .await
    .expect("comptage des comptes");
    assert_eq!(comptes, 0);
    assert_eq!(commun::sessions_vivantes(&t.bac, t.demandeuse).await, 0);
    assert_eq!(
        commun::acteur_resolu(&t.bac, &ouverte.access_token).await,
        None,
        "le jeton d'accès ne résout plus personne"
    );

    // **Et les compteurs de participation n'ont pas bougé.**
    assert_eq!(
        participations(&t).await,
        avant,
        "anonymiser n'est pas supprimer : une COP passée garde ses chiffres"
    );
}

/// Le piège n° 1 du module, éprouvé : **un seul** événement d'anonymisation.
#[tokio::test]
async fn un_seul_evenement_est_emis_par_effacement() {
    let t = dresser().await;
    let demande = deposer(&t, PrivacyRequestType::Erasure).await;

    privacy::handle(
        &t.bac.state,
        &t.bac.ctx(),
        t.actrice,
        demande,
        PrivacyAction::Anonymize,
        Some("Demande d'effacement du 20/08"),
    )
    .await
    .expect("effacement");

    let emis = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events
            WHERE event_type = 'identity.person.anonymized'"#
    )
    .fetch_one(t.bac.base.pool())
    .await
    .expect("comptage");

    assert_eq!(
        emis, 1,
        "identity.anonymize_person() émet le sien : le service ne doit pas en ajouter un second"
    );
}

/// Les trois autres actes ne touchent pas à l'identité : ils font avancer le
/// dossier, et rien de plus.
#[tokio::test]
async fn les_actes_administratifs_ne_touchent_pas_a_lidentite() {
    let t = dresser().await;
    let demande = deposer(&t, PrivacyRequestType::Rectification).await;

    for (acte, attendu) in [
        (PrivacyAction::Start, PrivacyRequestStatus::InProgress),
        (PrivacyAction::Complete, PrivacyRequestStatus::Completed),
        (PrivacyAction::Reject, PrivacyRequestStatus::Rejected),
    ] {
        let issue = privacy::handle(
            &t.bac.state,
            &t.bac.ctx(),
            t.actrice,
            demande,
            acte,
            Some("Traitée avec la personne"),
        )
        .await
        .expect("traitement");

        assert_eq!(issue.status, "saved");
        assert_eq!(issue.request.expect("demande rendue").status, attendu);
    }

    assert_eq!(
        sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM platform.outbox_events
                WHERE event_type = 'identity.person.anonymized'"#
        )
        .fetch_one(t.bac.base.pool())
        .await
        .expect("comptage"),
        0
    );
}

/// Une demande inconnue ne fait rien, et le dit — sans erreur HTTP.
#[tokio::test]
async fn une_demande_inconnue_rend_not_found() {
    let t = dresser().await;

    let issue = privacy::handle(
        &t.bac.state,
        &t.bac.ctx(),
        t.actrice,
        Uuid::now_v7(),
        PrivacyAction::Start,
        None,
    )
    .await
    .expect("appel du service");

    assert_eq!(issue.status, "not_found");
    assert!(issue.request.is_none());
}

/// L'échéance annoncée à l'écran est-elle encore celle de la table ?
///
/// `due_at` vient d'un `DEFAULT` : rien ne relie la constante Rust à la valeur
/// SQL, et les deux dériveraient sans que rien ne le dise. Ce test les rapproche
/// — c'est le seul garde-fou possible.
#[tokio::test]
async fn lecheance_annoncee_est_celle_de_la_table() {
    let t = dresser().await;
    deposer(&t, PrivacyRequestType::Export).await;

    let jours = sqlx::query_scalar!(
        r#"SELECT round(extract(epoch FROM (due_at - created_at)) / 86400)::int AS "jours!"
             FROM identity.privacy_requests"#
    )
    .fetch_one(t.bac.base.pool())
    .await
    .expect("lecture de l'échéance");

    assert_eq!(jours, DEADLINE_DAYS);

    let ecran = privacy::queue_screen(t.bac.state.pool())
        .await
        .expect("file");
    assert_eq!(ecran.deadline_days, DEADLINE_DAYS);
}
