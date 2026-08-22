//! **Le rappel part une fois, et une seule.**
//!
//! Ce que ces tests éprouvent et qu'aucune relecture ne remplace :
//!
//! - **une inscription CRÉÉE à l'état « inscrit » matérialise ses rappels** —
//!   c'est le chemin le plus courant, et celui qu'une lecture du commentaire du
//!   modèle aurait cassé : `programme.registration.confirmed` n'existe pas, et
//!   un consommateur écrit d'après lui ne serait **jamais réveillé** ;
//! - **annuler puis réinscrire rend les rappels à l'attente**, le cas que la clé
//!   d'unicité rend piégeux et qui ne se voit qu'ici ;
//! - **le travail rejoué n'écrit pas un second courriel**, mesuré sur la boîte
//!   et non supposé.
//!
//! Les annonces relayées sont celles que la base a **réellement** émises : elles
//! sont lues dans l'outbox, jamais fabriquées à la main.

mod commun;

use commun::Bac;
use engagement::jobs::send_reminder::SEND_REMINDER;
use engagement::service::rules::{self, ReminderRulePayload};
use engagement::service::schedule;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

const REMINDERS_SCHEDULED: &str = "engagement.reminders.scheduled";
const REGISTRATIONS: &str = "programme.registration.";
const SESSIONS: &str = "programme.session.";

/// Pose une règle d'édition par le **vrai chemin d'écriture** : c'est ce qui
/// fait de l'histoire précédente l'instrument de mesure de celle-ci.
async fn regle(bac: &Bac, terrain: &commun::Terrain, minutes: &[i32]) {
    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &ReminderRulePayload {
            event_id: Some(terrain.edition),
            session_id: None,
            offsets: Some(minutes.to_vec()),
            channels: None,
            type_code: None,
            template_id: None,
            is_active: None,
        },
    )
    .await
    .expect("écriture de la règle");
}

async fn compter_tous_les_rappels(bac: &Bac, session_id: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM engagement.scheduled_reminders WHERE session_id = $1"#,
        session_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des rappels")
}

/// Les instants d'envoi, dans un ordre stable.
async fn instants(bac: &Bac, session_id: Uuid) -> Vec<(Uuid, OffsetDateTime)> {
    sqlx::query!(
        "SELECT id, scheduled_for FROM engagement.scheduled_reminders
          WHERE session_id = $1 ORDER BY id",
        session_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des instants")
    .into_iter()
    .map(|l| (l.id, l.scheduled_for))
    .collect()
}

// -----------------------------------------------------------------------------
// T140 — le chemin courant, et le piège de l'écart n° 126
// -----------------------------------------------------------------------------

/// 🔴 **Une inscription CRÉÉE directement à l'état « inscrit » matérialise ses
/// rappels.**
///
/// C'est le chemin le plus courant, et celui qu'une lecture du commentaire du
/// modèle aurait cassé. `programme.registration.confirmed` n'est émis par
/// personne : un consommateur branché dessus ne serait **jamais réveillé**,
/// aucun rappel ne partirait, sans erreur ni trace, et personne ne s'en
/// apercevrait avant le jour de la séance.
#[tokio::test]
async fn une_inscription_creee_a_letat_inscrit_materialise_ses_rappels() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[1440]).await;

    // L'annonce qui réveille le consommateur est bien une **création**, et son
    // statut voyage dans la charge utile.
    let annonces = commun::annonces(&bac, REGISTRATIONS, false).await;
    assert_eq!(annonces.len(), 3, "les trois inscrits du harnais");
    for annonce in &annonces {
        assert_eq!(
            annonce.event_type, "programme.registration.created",
            "une inscription ordinaire naît par une CRÉATION"
        );
        assert_eq!(annonce.payload["status"], "registered");
    }
    assert_eq!(
        commun::compter_annonces(&bac, "programme.registration.confirmed").await,
        0,
        "cet événement n'existe pas : un consommateur branché dessus dormirait pour toujours"
    );

    commun::relayer(&bac, REGISTRATIONS).await;

    assert_eq!(
        compter_tous_les_rappels(&bac, terrain.seance).await,
        3,
        "un rappel par inscrit, sur l'unique décalage de la règle"
    );
    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "queued").await,
        3
    );
}

// -----------------------------------------------------------------------------
// T141 — la liste d'attente
// -----------------------------------------------------------------------------

/// **Une personne en liste d'attente n'a aucun rappel ; promue, elle les
/// obtient.**
///
/// La comparaison est textuelle, comme dans la fonction du modèle : elle n'a
/// pas de place, elle ne doit pas recevoir « votre séance commence dans une
/// heure ».
#[tokio::test]
async fn la_liste_dattente_ne_recoit_rien_avant_sa_promotion() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[1440]).await;

    let attente = commun::personne(&bac, "attente@example.org", "Bintou", "Diallo").await;
    let inscription = commun::inscrire(&bac, terrain.seance, attente, "waitlisted").await;
    commun::relayer(&bac, REGISTRATIONS).await;

    assert_eq!(
        compter_tous_les_rappels(&bac, terrain.seance).await,
        3,
        "les trois inscrits, et pas la personne en attente"
    );

    sqlx::query!(
        "UPDATE programme.registrations
            SET status = 'registered', waitlist_position = NULL
          WHERE id = $1",
        inscription
    )
    .execute(bac.pool())
    .await
    .expect("promotion");
    commun::relayer(&bac, REGISTRATIONS).await;

    assert_eq!(
        compter_tous_les_rappels(&bac, terrain.seance).await,
        4,
        "promue, elle obtient son rappel"
    );
}

// -----------------------------------------------------------------------------
// T142 — le cas que la clé d'unicité rend piégeux
// -----------------------------------------------------------------------------

/// 🔴 **Annuler puis réinscrire rend les rappels à l'attente**, et non
/// `cancelled`.
///
/// `ux_scheduled_reminders_once` porte sur (séance, personne, canal, décalage)
/// **sans condition d'état** : la ligne annulée existe toujours, et
/// `ON CONFLICT DO NOTHING` ne la ressuscite pas. Sans réactivation, qui se
/// désiste puis revient ne recevrait **plus jamais rien** — en silence, et dès
/// le premier désistement suivi d'un retour (R21).
#[tokio::test]
async fn annuler_puis_reinscrire_reactive_les_rappels() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[1440]).await;
    commun::relayer(&bac, REGISTRATIONS).await;

    let personne = terrain.inscrits[0];
    let lignes_avant = compter_tous_les_rappels(&bac, terrain.seance).await;

    sqlx::query!(
        "UPDATE programme.registrations SET status = 'cancelled'
          WHERE session_id = $1 AND person_id = $2",
        terrain.seance,
        personne
    )
    .execute(bac.pool())
    .await
    .expect("désistement");
    commun::relayer(&bac, REGISTRATIONS).await;

    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "cancelled").await,
        1
    );
    assert!(commun::motifs_des_rappels(&bac, terrain.seance)
        .await
        .contains(&"registration_cancelled".to_owned()));

    sqlx::query!(
        "UPDATE programme.registrations SET status = 'registered'
          WHERE session_id = $1 AND person_id = $2",
        terrain.seance,
        personne
    )
    .execute(bac.pool())
    .await
    .expect("retour");
    commun::relayer(&bac, REGISTRATIONS).await;

    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "cancelled").await,
        0,
        "la ligne est RÉACTIVÉE : la recréer se heurterait à la clé d'unicité, en silence"
    );
    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "pending").await,
        1,
        "et elle repart de l'attente, son travail étant toujours en file"
    );
    assert_eq!(
        compter_tous_les_rappels(&bac, terrain.seance).await,
        lignes_avant,
        "aucune ligne de plus : c'est la même qui revit"
    );
}

// -----------------------------------------------------------------------------
// T143 et T135 — le report déplace, il ne recrée pas
// -----------------------------------------------------------------------------

/// 🔴 **Une séance déplacée de trois heures voit les instants de ses rappels non
/// partis décalés d'autant** — et ses rappels partis inchangés.
///
/// Recréer se heurterait à la clé d'unicité, qui ne porte pas l'instant : les
/// lignes resteraient à l'ancienne heure et **rien ne le dirait**.
#[tokio::test]
async fn une_seance_deplacee_deplace_ses_rappels() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[2880, 1440]).await;
    commun::relayer(&bac, REGISTRATIONS).await;

    let avant = instants(&bac, terrain.seance).await;
    assert_eq!(avant.len(), 6, "trois inscrits, deux décalages");

    // Un rappel déjà parti : il ne doit pas bouger.
    let parti = avant[0].0;
    sqlx::query!(
        "UPDATE engagement.scheduled_reminders SET status = 'sent', sent_at = now() WHERE id = $1",
        parti
    )
    .execute(bac.pool())
    .await
    .expect("rappel marqué parti");

    sqlx::query!(
        "UPDATE programme.sessions
            SET starts_at = starts_at + interval '3 hours',
                ends_at   = ends_at   + interval '3 hours'
          WHERE id = $1",
        terrain.seance
    )
    .execute(bac.pool())
    .await
    .expect("séance déplacée");
    commun::relayer(&bac, SESSIONS).await;

    let apres: std::collections::HashMap<Uuid, OffsetDateTime> =
        instants(&bac, terrain.seance).await.into_iter().collect();

    for (id, instant_avant) in &avant {
        let instant_apres = apres[id];
        if *id == parti {
            assert_eq!(
                instant_apres, *instant_avant,
                "un rappel déjà parti ne se déplace pas : il est parti"
            );
        } else {
            assert_eq!(
                instant_apres - *instant_avant,
                Duration::hours(3),
                "les rappels à traiter suivent le créneau"
            );
        }
    }

    assert_eq!(
        compter_tous_les_rappels(&bac, terrain.seance).await,
        6,
        "déplacés, jamais recréés"
    );

    // **Le travail suit la ligne** : son échéance vit dans `platform.jobs`, et
    // une ligne remise à l'heure dont le travail ne bouge pas enverrait le
    // courriel à l'ancienne — la ligne dirait l'heure nouvelle, le courriel
    // arriverait à l'ancienne, et rien ne le signalerait.
    let a_lheure = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!"
             FROM engagement.scheduled_reminders sr
             JOIN platform.jobs j ON j.id = sr.job_id
            WHERE sr.session_id = $1 AND j.run_at = sr.scheduled_for"#,
        terrain.seance
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des travaux à l'heure");
    assert_eq!(
        a_lheure, 6,
        "chaque travail porte l'instant de sa ligne — les cinq déplacés comme celui qui ne l'était pas"
    );
}

// -----------------------------------------------------------------------------
// T144 — pas de rattrapage
// -----------------------------------------------------------------------------

/// **Un décalage dont l'instant est déjà passé n'est pas créé.**
///
/// « On ne réveille personne à 3 h du matin parce qu'un import a pris du
/// retard » — la règle est celle du modèle, et ce test la rend opposable.
#[tokio::test]
async fn un_decalage_deja_passe_nest_pas_cree() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // La séance est à trente jours. Un décalage de soixante jours tombe donc
    // trente jours derrière nous ; celui d'un jour est encore devant.
    regle(&bac, &terrain, &[60 * 24 * 60, 1440]).await;
    commun::relayer(&bac, REGISTRATIONS).await;

    assert_eq!(
        compter_tous_les_rappels(&bac, terrain.seance).await,
        3,
        "seul le décalage encore devant est matérialisé"
    );
}

// -----------------------------------------------------------------------------
// T145 et T150 — l'idempotence, et ce que le service n'ajoute pas
// -----------------------------------------------------------------------------

/// 🔴 **L'annonce d'inscription rejouée dix fois ne produit ni rappel ni travail
/// supplémentaire.**
///
/// La garde de rejeu de `platform.inbox_events` n'est **pas** posée ici, et
/// c'est délibéré : ce qui reste doit tenir tout seul. Ce sont donc l'unicité du
/// modèle et celle de la file que ce test mesure.
#[tokio::test]
async fn dix_rejeux_ne_produisent_rien_de_plus() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[2880, 1440, 60, 30]).await;

    commun::relayer(&bac, REGISTRATIONS).await;
    let rappels = compter_tous_les_rappels(&bac, terrain.seance).await;
    let travaux = commun::compter_travaux(&bac, SEND_REMINDER).await;
    assert_eq!(rappels, 12, "trois inscrits, quatre décalages");
    assert_eq!(travaux, 12, "un travail par rappel");

    for _ in 0..10 {
        commun::rejouer_les_annonces(&bac, REGISTRATIONS).await;
    }

    assert_eq!(
        compter_tous_les_rappels(&bac, terrain.seance).await,
        rappels
    );
    assert_eq!(commun::compter_travaux(&bac, SEND_REMINDER).await, travaux);
}

/// 🔴 **Le service n'émet ni n'enfile.**
///
/// La fonction du modèle insère les rappels, met **un travail par rappel** en
/// file et émet son annonce, le tout dans le même geste. Un service zélé
/// produirait deux courriels par rappel, et le doublon ne se verrait qu'en
/// production.
#[tokio::test]
async fn une_materialisation_emet_une_annonce_et_un_travail_par_rappel() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[1440, 60]).await;

    let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
    let crees = schedule::materialiser(&mut tx, terrain.seance, None)
        .await
        .expect("matérialisation");
    tx.commit().await.expect("validation");

    assert_eq!(crees, 6, "trois inscrits, deux décalages");
    assert_eq!(
        commun::compter_annonces(&bac, REMINDERS_SCHEDULED).await,
        1,
        "UNE annonce, celle de la fonction du modèle"
    );
    assert_eq!(
        commun::compter_travaux(&bac, SEND_REMINDER).await,
        6,
        "exactement le nombre de rappels créés, jamais le double"
    );
}

// -----------------------------------------------------------------------------
// T146 et T147 — l'envoi
// -----------------------------------------------------------------------------

/// 🔴 **L'heure venue, un courriel part par destinataire ; le travail rejoué
/// n'en produit pas de second.**
///
/// La file est « au moins une fois » : c'est la ligne de rappel, marquée
/// **avant** l'expédition, qui rend le second envoi impossible. La clé d'unicité
/// du modèle ne suffirait pas — elle interdit deux lignes, pas deux envois sur
/// la même ligne.
#[tokio::test]
async fn un_courriel_par_destinataire_et_pas_un_de_plus() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[1440]).await;
    commun::relayer(&bac, REGISTRATIONS).await;

    // Le relais du courrier n'est pas encore passé : rien ne part.
    assert_eq!(bac.boite.compte(), 0);

    commun::avancer_les_rappels(&bac, terrain.seance).await;
    let issues = commun::passer_le_worker(&bac).await;
    assert_eq!(issues.len(), 3);
    assert!(issues.iter().all(Result::is_ok));

    assert_eq!(bac.boite.compte(), 3, "un courriel par inscrit");
    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "sent").await,
        3
    );

    // Le geste du quickstart : `UPDATE platform.jobs SET status='queued'`.
    commun::rejouer_les_travaux(&bac, SEND_REMINDER).await;
    commun::passer_le_worker(&bac).await;

    assert_eq!(
        bac.boite.compte(),
        3,
        "aucun second courriel : la ligne est déjà partie"
    );
}

/// 🔴 **Le worker arrêté, rien ne part et rien n'est perdu ; relancé, le
/// courriel arrive** — le point de contrôle de B1, réemployé.
#[tokio::test]
async fn le_worker_arrete_ne_perd_rien() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[1440]).await;
    commun::relayer(&bac, REGISTRATIONS).await;
    commun::avancer_les_rappels(&bac, terrain.seance).await;

    // Le worker est tué **avant son marquage** : les travaux restent réservés,
    // exactement comme après un Ctrl-C en cours de lot.
    commun::worker_tue_apres_le_travail(&bac).await;
    let partis = bac.boite.compte();

    // Rien n'est perdu : le noyau rend à la file ce que le précédent a laissé.
    commun::worker_relance(&bac).await;
    commun::passer_le_worker(&bac).await;

    assert_eq!(bac.boite.compte(), 3, "les trois courriels, une seule fois");
    assert_eq!(
        partis, 3,
        "le premier passage avait bien tout expédié — c'est son marquage qui manquait"
    );
    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "sent").await,
        3
    );
}

// -----------------------------------------------------------------------------
// T148 et T139 — écarter, avec son motif
// -----------------------------------------------------------------------------

/// 🔴 **Une adresse supprimée ne reçoit rien, et le rappel porte son motif ; un
/// canal coupé de même.**
///
/// Écarter en silence laisserait l'organisation lire « rien n'est parti » sans
/// savoir pourquoi, et l'exploitant chercher une panne là où il n'y en a pas
/// (FR-065).
#[tokio::test]
async fn un_envoi_ecarte_lest_avec_son_motif() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[1440]).await;

    // Une adresse hors circuit.
    sqlx::query!(
        "INSERT INTO engagement.email_suppressions (email, reason)
         VALUES ((SELECT primary_email FROM identity.people WHERE id = $1), 'hard_bounce')",
        terrain.inscrits[0]
    )
    .execute(bac.pool())
    .await
    .expect("suppression de l'adresse");

    // Un canal coupé, par une préférence.
    sqlx::query!(
        "INSERT INTO engagement.notification_preferences (person_id, type_code, channel, is_enabled)
         VALUES ($1, 'programme.session.reminder', 'email', false)",
        terrain.inscrits[1]
    )
    .execute(bac.pool())
    .await
    .expect("préférence posée");

    commun::relayer(&bac, REGISTRATIONS).await;
    commun::avancer_les_rappels(&bac, terrain.seance).await;
    commun::passer_le_worker(&bac).await;

    assert_eq!(
        bac.boite.compte(),
        1,
        "seule la troisième reçoit son rappel"
    );
    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "skipped").await,
        2
    );
    let motifs = commun::motifs_des_rappels(&bac, terrain.seance).await;
    assert_eq!(
        motifs,
        vec!["channel_disabled".to_owned(), "suppressed".to_owned()],
        "deux raisons différentes de ne rien envoyer, et chacune est nommée"
    );
}

// -----------------------------------------------------------------------------
// T149 — la séance annulée
// -----------------------------------------------------------------------------

/// **Une séance annulée annule ses rappels à traiter avec leur motif, et laisse
/// les partis tracés.**
///
/// Les deux moitiés comptent. Annuler ce qui reste est ce qui distingue une
/// annulation d'un oubli : sans cela, les rappels **déjà matérialisés**
/// partiraient quand même, et les inscrits d'une séance annulée recevraient
/// « votre séance commence dans une heure ». Et ne pas toucher aux partis est
/// l'autre moitié : ils sont partis, et le dire autrement serait faux.
#[tokio::test]
async fn une_seance_annulee_annule_ce_qui_reste() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[2880, 1440]).await;
    commun::relayer(&bac, REGISTRATIONS).await;
    assert_eq!(compter_tous_les_rappels(&bac, terrain.seance).await, 6);

    // Un rappel déjà parti — le décalage le plus lointain d'un des inscrits.
    let parti = instants(&bac, terrain.seance).await[0].0;
    sqlx::query!(
        "UPDATE engagement.scheduled_reminders SET status = 'sent', sent_at = now() WHERE id = $1",
        parti
    )
    .execute(bac.pool())
    .await
    .expect("rappel marqué parti");

    // `ck_sessions_cancelled_reason` exige le motif : une séance ne s'annule pas
    // sans dire pourquoi.
    sqlx::query!(
        r#"UPDATE programme.sessions
              SET status = 'cancelled',
                  cancelled_reason = '{"fr":"Intervenante empêchée","en":"Speaker unavailable"}'::jsonb
            WHERE id = $1"#,
        terrain.seance
    )
    .execute(bac.pool())
    .await
    .expect("annulation");
    commun::relayer(&bac, SESSIONS).await;

    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "cancelled").await,
        5,
        "tout ce qui restait à traiter est annulé"
    );
    assert_eq!(
        commun::motifs_des_rappels(&bac, terrain.seance).await,
        vec!["session_cancelled".to_owned()],
        "et le motif dit pourquoi : l'organisation lirait sinon « rien n'est parti »"
    );
    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "sent").await,
        1,
        "le rappel déjà parti reste parti, et sa trace avec"
    );
}

// -----------------------------------------------------------------------------
// T151 — la charge utile d'un travail mort reste lisible
// -----------------------------------------------------------------------------

/// **`engagement.send_reminder` ne déclare pas porter de secret.**
///
/// Sa charge utile ne porte que des identifiants — rappel, séance, personne,
/// canal. Un travail **mort** la garde, et c'est la seule matière de diagnostic
/// d'un rappel qui n'est jamais parti : l'effacer par prudence laisserait une
/// ligne morte sans rien pour comprendre.
#[tokio::test]
async fn le_travail_denvoi_ne_porte_pas_de_secret() {
    let bac = Bac::monter().await;
    let gestionnaires = engagement::job_handlers(bac.db(), &bac.config, bac.state.mailer().clone());

    let envoi = gestionnaires
        .iter()
        .find(|g| g.task() == SEND_REMINDER)
        .expect("le gestionnaire d'envoi est monté");

    assert!(!envoi.carries_secret());
    assert_eq!(
        envoi.queue(),
        "email",
        "la file que la fonction du modèle nomme — une file inécoutée empile en silence"
    );
}

// -----------------------------------------------------------------------------
// T137 — du type au courriel, et le texte de secours
// -----------------------------------------------------------------------------

/// **Le modèle publié sert, et ses variables sont résolues.**
///
/// Le lien porte sa variable dans le gabarit et l'adresse dans le courriel :
/// c'est l'autre bout du piège de R26 — un assainisseur qui aurait détruit
/// `{{lien_participation}}` donnerait ici un lien mort, et cela ne se verrait
/// qu'à la réception.
#[tokio::test]
async fn un_modele_publie_sert_et_ses_variables_sont_resolues() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[1440]).await;

    commun::modele_publie(
        &bac,
        "session-reminder",
        "programme.session.reminder",
        "{{titre_session}} — {{delai}}",
        "<p>Bonjour {{prenom}}, rendez-vous le {{date_session}}. \
         <a href=\"{{lien_participation}}\">Rejoindre</a></p>",
    )
    .await;

    commun::relayer(&bac, REGISTRATIONS).await;
    commun::avancer_les_rappels(&bac, terrain.seance).await;
    commun::passer_le_worker(&bac).await;

    let messages = bac.boite.messages();
    assert_eq!(messages.len(), 3);
    let message = &messages[0];

    assert_eq!(message.subject, "Financer l'adaptation — dans 1 jour");
    let html = message.html.as_deref().expect("le modèle porte du HTML");
    assert!(
        !html.contains("{{"),
        "aucune variable ne doit rester dans le courriel : {html}"
    );
    assert!(
        html.contains("href=\"http"),
        "le lien porte une adresse, pas la variable : {html}"
    );
    assert!(
        html.contains("financer-adaptation"),
        "et il mène à la séance : {html}"
    );
    // Le fuseau voyage avec l'heure : « 14h30 » sans fuseau n'est pas une heure.
    assert!(
        html.contains("America/Belem"),
        "la date porte son fuseau : {html}"
    );
    assert!(
        !message.text.is_empty(),
        "un client qui ne lit pas le HTML doit tout de même lire quelque chose"
    );
}

/// 🔴 **Un type sans révision publiée part quand même, et la trace le dit.**
///
/// Rien ne sème de modèle (écart n° 131). Échouer laisserait **tous** les
/// rappels à terre sur une base neuve ; envoyer sans le dire empêcherait de
/// découvrir qu'un modèle manque. Le texte de secours porte donc le libellé du
/// type, et `email_messages.template_id` reste nul (R27).
#[tokio::test]
async fn un_type_sans_modele_publie_part_avec_le_texte_de_secours() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    regle(&bac, &terrain, &[60]).await;

    commun::relayer(&bac, REGISTRATIONS).await;
    commun::avancer_les_rappels(&bac, terrain.seance).await;
    commun::passer_le_worker(&bac).await;

    let messages = bac.boite.messages();
    assert_eq!(
        messages.len(),
        3,
        "les rappels partent malgré l'absence de modèle"
    );

    let message = &messages[0];
    assert!(
        message.subject.starts_with("Rappel de session"),
        "le sujet vient du libellé du TYPE, résolu en base : {}",
        message.subject
    );
    assert!(message.text.contains("Financer l'adaptation"));
    assert!(message.text.contains("dans 1 heure"));
    assert!(message.text.contains("America/Belem"));
    assert!(
        message.html.is_none(),
        "le secours n'invente pas de mise en forme"
    );

    // La trace **dit** que personne n'a écrit ce courriel.
    let sans_modele = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM engagement.email_messages
            WHERE template_id IS NULL AND status = 'sent'"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des traces");
    assert_eq!(sans_modele, 3);
}
