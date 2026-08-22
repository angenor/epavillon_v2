//! **La plateforme cesse d'écrire à qui ne veut plus la lire.**
//!
//! Ce que ces tests éprouvent et qu'aucune relecture ne remplace :
//!
//! - **la garde d'envoi s'applique à un message que ce module n'a pas
//!   composé** — c'est ce qui referme l'écart n° 133 sans qu'aucune ligne des
//!   modules livrés ne change ;
//! - **une suppression temporaire échue se lève toute seule**, sans travail
//!   récurrent : la fonction du modèle compare déjà l'échéance à maintenant, et
//!   un second dispositif divergerait du premier en silence ;
//! - **une annonce du fournisseur rejouée est ignorée**, jamais dupliquée.

mod commun;

use commun::Bac;
use engagement::jobs::send_reminder::SEND_REMINDER;
use engagement::service::deliverability::{self, MailEvent, SuppressionPayload};
use engagement::service::rules::{self, ReminderRulePayload};
use kernel::mail::OutgoingMail;
use kernel::ErrorCode;
use uuid::Uuid;

fn suppression(email: &str, motif: &str) -> SuppressionPayload {
    SuppressionPayload {
        email: email.to_owned(),
        reason: motif.to_owned(),
        detail: None,
        expires_at: None,
    }
}

/// Un courriel **tel qu'un module livré le construit** : un identifiant de
/// travail, une adresse, un sujet, un texte. Ni type, ni modèle, ni variables.
fn courriel_dun_module_livre(a: &str) -> OutgoingMail {
    OutgoingMail {
        message_id: Uuid::now_v7().to_string(),
        to: a.to_owned(),
        locale: "fr".to_owned(),
        subject: "Invitation à rejoindre une organisation".to_owned(),
        text: "Bonjour, vous êtes invitée à rejoindre le ROAC.".to_owned(),
        html: None,
    }
}

async fn adresse_de(bac: &Bac, person_id: Uuid) -> String {
    sqlx::query_scalar!(
        r#"SELECT primary_email::text AS "email!" FROM identity.people WHERE id = $1"#,
        person_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("adresse de la personne")
}

// -----------------------------------------------------------------------------
// T175 — la garde s'applique à un module livré
// -----------------------------------------------------------------------------

/// 🔴 **Une adresse supprimée ne reçoit rien, même d'un module livré avant ce
/// jalon** — l'écart n° 133.
///
/// Les six courriels de B1 et B2 appellent le contrat d'envoi du noyau
/// directement, sans garde ni trace. Les réécrire supposerait que `identity` et
/// `org` connaissent `engagement`, ce que le principe II interdit. Le contrat
/// est donc **enveloppé**, et aucun de ces modules ne change d'une ligne.
///
/// **Ce test frappe le décorateur, pas le module Organisations** : monter ce
/// dernier ici demanderait une dépendance de développement que
/// `cargo tree -p engagement` interdit. Ce qu'on éprouve est le mécanisme —
/// un message que ce module n'a **pas** composé, remis à la même porte —, et le
/// point de contrôle du quickstart provoque le cas réel à la main.
#[tokio::test]
async fn la_garde_denvoi_sapplique_a_un_message_dun_autre_module() {
    let bac = Bac::monter().await;
    let redactrice = commun::redactrice(&bac).await;
    let invitee = commun::personne(&bac, "invitee@example.org", "Nadia", "Ben Salah").await;
    let adresse = adresse_de(&bac, invitee).await;

    // Avant la suppression : le courriel part.
    bac.state
        .mailer()
        .send(&courriel_dun_module_livre(&adresse))
        .await
        .expect("remise");
    assert_eq!(bac.boite.compte(), 1);

    deliverability::poser(
        &bac.state,
        &bac.ctx(),
        redactrice,
        &suppression(&adresse, "hard_bounce"),
    )
    .await
    .expect("inscription sur la liste");

    bac.boite.vider();
    bac.state
        .mailer()
        .send(&courriel_dun_module_livre(&adresse))
        .await
        .expect("écarter n'est pas une erreur : le travail ne doit pas mourir cinq fois");

    assert_eq!(
        bac.boite.compte(),
        0,
        "rien ne part, et aucune ligne du module émetteur n'a été modifiée"
    );

    // **La trace existe quand même, et elle dit pourquoi.** Un envoi écarté en
    // silence laisserait chercher une panne là où il n'y en a pas.
    let (etat, motif) = sqlx::query!(
        r#"SELECT status::text AS "etat!", last_error, type_code
             FROM engagement.email_messages
            WHERE to_email::text = $1
            ORDER BY created_at DESC LIMIT 1"#,
        adresse
    )
    .fetch_one(bac.pool())
    .await
    .map(|l| (l.etat, l.last_error.unwrap_or_default()))
    .expect("la trace de l'envoi écarté");

    assert_eq!(etat, "failed", "il n'a pas rebondi : il n'est jamais parti");
    assert!(motif.contains("liste de suppression"), "{motif}");
}

/// **L'annonce porte l'adresse HACHÉE, jamais en clair.**
///
/// L'outbox est durable, indexée par agrégat, relayée et faite pour être relue.
/// Qui détient déjà l'adresse peut vérifier qu'elle est concernée ; personne ne
/// peut la lire.
#[tokio::test]
async fn ladresse_ne_voyage_pas_en_clair_dans_lannonce() {
    let bac = Bac::monter().await;
    let redactrice = commun::redactrice(&bac).await;

    deliverability::poser(
        &bac.state,
        &bac.ctx(),
        redactrice,
        &suppression("zoubeida.temoin@example.org", "complaint"),
    )
    .await
    .expect("inscription sur la liste");

    let charge = sqlx::query_scalar!(
        r#"SELECT payload::text AS "charge!" FROM platform.outbox_events
            WHERE event_type = 'engagement.email.suppressed'"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("l'annonce de suppression");

    assert!(!charge.contains("zoubeida"), "{charge}");
    assert!(!charge.contains("@example.org"), "{charge}");
    assert!(charge.contains("complaint"));
    assert_eq!(
        charge.matches("email_hash").count(),
        1,
        "l'empreinte, et rien d'autre : {charge}"
    );
}

// -----------------------------------------------------------------------------
// T182 — une suppression échue se lève seule
// -----------------------------------------------------------------------------

/// 🔴 **Une suppression temporaire échue se lève sans intervention.**
///
/// Aucun travail récurrent ne la retire : `is_email_suppressed()` compare déjà
/// `expires_at` à maintenant, et une purge programmée serait un second
/// dispositif à tenir d'accord avec le premier — le premier écart entre les deux
/// serait silencieux. La ligne **reste visible** : savoir qu'une boîte était
/// pleine le mois dernier a de la valeur.
#[tokio::test]
async fn une_suppression_echue_se_leve_sans_intervention() {
    let bac = Bac::monter().await;
    let redactrice = commun::redactrice(&bac).await;
    let adresse = "boite-pleine@example.org";

    sqlx::query!(
        "INSERT INTO engagement.email_suppressions (email, reason, expires_at)
         VALUES ($1::text::platform.email, 'invalid_address', now() - interval '1 hour')",
        adresse
    )
    .execute(bac.pool())
    .await
    .expect("suppression échue posée");

    bac.state
        .mailer()
        .send(&courriel_dun_module_livre(adresse))
        .await
        .expect("remise");
    assert_eq!(
        bac.boite.compte(),
        1,
        "l'échéance passée, l'adresse est de nouveau joignable"
    );

    // Elle reste **lisible** : la lever n'est pas l'effacer.
    let lignes = deliverability::lister(&bac.state, redactrice, None)
        .await
        .expect("la liste de suppression");
    assert_eq!(lignes.len(), 1);
    assert!(lignes[0].expires_at.is_some());
}

// -----------------------------------------------------------------------------
// T181 — une annonce rejouée est ignorée
// -----------------------------------------------------------------------------

/// 🔴 **Une annonce de délivrabilité rejouée ne crée pas de seconde trace, et
/// est comptée dans `ignored`.**
///
/// Le fournisseur rejoue volontiers ses annonces ; rendre une erreur le ferait
/// recommencer sans fin. Ignorée n'est pas perdue.
#[tokio::test]
async fn une_annonce_rejouee_est_ignoree() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &ReminderRulePayload {
            event_id: Some(terrain.edition),
            session_id: None,
            offsets: Some(vec![1440]),
            channels: None,
            type_code: None,
            template_id: None,
            is_active: None,
        },
    )
    .await
    .expect("écriture de la règle");
    commun::relayer(&bac, "programme.registration.").await;
    commun::avancer_les_rappels(&bac, terrain.seance).await;
    commun::passer_le_worker(&bac).await;

    // L'identifiant qui relie l'annonce à sa trace est celui que l'API a remis
    // au site — le travail —, jamais celui du fournisseur : le contrat d'envoi
    // du noyau n'en rapporte aucun.
    let travail = sqlx::query_scalar!(
        r#"SELECT id FROM platform.jobs WHERE task = $1 ORDER BY created_at LIMIT 1"#,
        SEND_REMINDER
    )
    .fetch_one(bac.pool())
    .await
    .expect("un travail d'envoi");

    let annonce = vec![MailEvent {
        message_id: travail.to_string(),
        provider_message_id: Some("ses-0001".to_owned()),
        status: "delivered".to_owned(),
        bounce_kind: None,
        detail: None,
    }];

    let premiere = deliverability::ingerer(&bac.state, &bac.ctx(), &annonce)
        .await
        .expect("première ingestion");
    assert_eq!((premiere.applied, premiere.ignored), (1, 0));

    let seconde = deliverability::ingerer(&bac.state, &bac.ctx(), &annonce)
        .await
        .expect("seconde ingestion");
    assert_eq!(
        (seconde.applied, seconde.ignored),
        (0, 1),
        "rejouée, elle est IGNORÉE — jamais appliquée deux fois"
    );

    let traces = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM engagement.email_messages WHERE job_id = $1"#,
        travail
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des traces");
    assert_eq!(traces, 1, "une trace, jamais deux");

    // Une annonce dont la trace est introuvable est ignorée de la même façon :
    // une trace effacée par la purge de partition n'est pas un incident.
    let orpheline = deliverability::ingerer(
        &bac.state,
        &bac.ctx(),
        &[MailEvent {
            message_id: Uuid::now_v7().to_string(),
            provider_message_id: None,
            status: "delivered".to_owned(),
            bounce_kind: None,
            detail: None,
        }],
    )
    .await
    .expect("ingestion d'une orpheline");
    assert_eq!((orpheline.applied, orpheline.ignored), (0, 1));
}

/// **Un rebond dur inscrit l'adresse sur la liste ; un rebond souple, non.**
///
/// C'est le seul geste qui protège la réputation du domaine sans intervention
/// humaine — et une boîte pleine n'est pas une adresse morte.
#[tokio::test]
async fn un_rebond_dur_supprime_ladresse_et_un_souple_ne_la_supprime_pas() {
    let bac = Bac::monter().await;
    let invitee = commun::personne(&bac, "rebond@example.org", "Sara", "Kone").await;
    let adresse = adresse_de(&bac, invitee).await;

    let mail = courriel_dun_module_livre(&adresse);
    bac.state.mailer().send(&mail).await.expect("remise");

    for (rebond, attendu) in [("soft", 0_i64), ("hard", 1)] {
        deliverability::ingerer(
            &bac.state,
            &bac.ctx(),
            &[MailEvent {
                message_id: mail.message_id.clone(),
                provider_message_id: None,
                status: "bounced".to_owned(),
                bounce_kind: Some(rebond.to_owned()),
                detail: Some(format!("rebond {rebond}")),
            }],
        )
        .await
        .expect("ingestion");

        let supprimees = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM engagement.email_suppressions WHERE email::text = $1"#,
            adresse
        )
        .fetch_one(bac.pool())
        .await
        .expect("comptage");
        assert_eq!(supprimees, attendu, "rebond {rebond}");

        // Le second passage doit repartir d'un état différent, sans quoi la
        // garde de rejeu l'ignorerait avant d'arriver à la décision.
        sqlx::query!(
            "UPDATE engagement.email_messages SET status = 'sent' WHERE to_email::text = $1",
            adresse
        )
        .execute(bac.pool())
        .await
        .expect("remise à l'état d'envoi");
    }
}

// -----------------------------------------------------------------------------
// T178 — un type inconnu vaut refus
// -----------------------------------------------------------------------------

/// 🔴 **Un type inconnu vaut refus d'envoi, jamais un envoi par défaut.**
///
/// *« On n'invente pas d'envoi »*, dit le modèle. Et l'écriture d'une préférence,
/// elle, **refuse explicitement** : une ligne orpheline ne serait jamais relue,
/// et la personne croirait avoir coupé quelque chose.
#[tokio::test]
async fn un_type_inconnu_vaut_refus() {
    use engagement::domain::notification::NotificationPreferencePayload;
    use engagement::repo::delivery;
    use engagement::service::notifications;

    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    assert!(
        !delivery::canal_autorise(
            bac.pool(),
            terrain.animatrice,
            "programme.chose.inexistante",
            "email"
        )
        .await
        .expect("consultation du canal"),
        "un type inconnu ne doit jamais valoir autorisation"
    );

    let refus = notifications::ecrire_les_preferences(
        &bac.state,
        &bac.ctx(),
        terrain.animatrice,
        "fr",
        &[NotificationPreferencePayload {
            type_code: "programme.chose.inexistante".to_owned(),
            channel: "email".to_owned(),
            is_enabled: false,
        }],
    )
    .await
    .expect_err("l'écriture doit refuser");
    assert_eq!(refus.code, ErrorCode::EngagementNotificationTypeUnknown);
}

/// La liste de suppression est gardée par la permission **globale** : c'est
/// celui qui écrit les courriels de la plateforme qui répond de leur
/// délivrabilité.
#[tokio::test]
async fn la_liste_de_suppression_est_gardee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let refus = deliverability::lister(&bac.state, terrain.administratrice, None)
        .await
        .expect_err("l'administratrice d'une édition n'y a pas accès");
    assert_eq!(refus.code, ErrorCode::Forbidden);

    let refus = deliverability::poser(
        &bac.state,
        &bac.ctx(),
        terrain.animatrice,
        &suppression("qui-que-ce-soit@example.org", "manual"),
    )
    .await
    .expect_err("ni l'animatrice");
    assert_eq!(refus.code, ErrorCode::Forbidden);
}
