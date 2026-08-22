//! **Le calendrier des rappels : quatre lignes, et pas un nom.** L'écart n° 34.
//!
//! Ce que ces tests éprouvent et qu'aucune relecture ne remplace : **quarante
//! inscrits et quatre décalages rendent QUATRE lignes portant chacune quarante
//! destinataires**, jamais cent soixante — l'erreur qu'une lecture ligne à ligne
//! produirait, et qui ressemble à un résultat juste ; et le **balayage de la
//! charge utile sérialisée entière**, qui attrape l'identité qu'on ajouterait
//! demain là où un contrôle champ par champ la laisserait passer.

mod commun;

use commun::Bac;
use engagement::domain::reminder::{consolider, ReminderStatus};
use engagement::service::rules::{self, ReminderRulePayload};
use engagement::service::schedule;
use uuid::Uuid;

/// Le défaut du modèle : 2 jours, 1 jour, 1 heure, 30 minutes. **Cumulés.**
const DEFAUT: [i32; 4] = [2880, 1440, 60, 30];

/// Le nombre d'inscrits du test qui compte. Le harnais en pose trois ; les
/// autres arrivent ici.
const INSCRITS: usize = 40;

fn regle_dedition(
    edition: Uuid,
    minutes: &[i32],
    canaux: Option<Vec<String>>,
) -> ReminderRulePayload {
    ReminderRulePayload {
        event_id: Some(edition),
        session_id: None,
        offsets: Some(minutes.to_vec()),
        channels: canaux,
        type_code: None,
        template_id: None,
        is_active: None,
    }
}

/// Complète les inscrits du harnais jusqu'à `INSCRITS`, avec des noms et des
/// adresses **reconnaissables dans un texte** : c'est ce qui rend le balayage
/// concluant.
async fn completer_les_inscrits(bac: &Bac, seance: Uuid, deja: usize) -> Vec<Uuid> {
    let mut ajoutes = Vec::new();
    for i in deja..INSCRITS {
        let personne = commun::personne(
            bac,
            &format!("temoin{i}@example.org"),
            &format!("Prénom{i}"),
            &format!("Patronyme{i}"),
        )
        .await;
        commun::inscrire(bac, seance, personne, "registered").await;
        ajoutes.push(personne);
    }
    ajoutes
}

/// Les identifiants des lignes d'un groupe, dans un ordre stable.
async fn lignes_du_groupe(bac: &Bac, seance: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar!(
        "SELECT id FROM engagement.scheduled_reminders
          WHERE session_id = $1 ORDER BY id",
        seance
    )
    .fetch_all(bac.pool())
    .await
    .expect("lignes du groupe")
}

/// Pose un état sur une ligne précise — et son motif quand elle est morte.
async fn poser_letat(bac: &Bac, rappel: Uuid, etat: ReminderStatus) {
    let motif = matches!(etat, ReminderStatus::Skipped | ReminderStatus::Cancelled)
        .then_some(engagement::domain::reminder::motifs::SUPPRIME);
    let parti = matches!(etat, ReminderStatus::Sent).then(time::OffsetDateTime::now_utc);

    sqlx::query!(
        "UPDATE engagement.scheduled_reminders
            SET status = $2::text::engagement.reminder_status,
                skip_reason = $3,
                sent_at = $4
          WHERE id = $1",
        rappel,
        etat.as_str(),
        motif,
        parti
    )
    .execute(bac.pool())
    .await
    .expect("pose de l'état");
}

// -----------------------------------------------------------------------------
// T124 — le test qui compte
// -----------------------------------------------------------------------------

/// 🔴 **Quarante inscrits et quatre décalages rendent QUATRE lignes portant
/// chacune quarante destinataires.**
///
/// Jamais cent soixante : c'est l'agrégat qui est servi, pas la matière. Cent
/// soixante lignes ressembleraient à un résultat juste — l'écran afficherait
/// simplement une liste très longue — et personne ne s'en apercevrait.
#[tokio::test]
async fn quarante_inscrits_et_quatre_decalages_rendent_quatre_lignes() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    completer_les_inscrits(&bac, terrain.seance, terrain.inscrits.len()).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &regle_dedition(terrain.edition, &DEFAUT, None),
    )
    .await
    .expect("écriture de la règle");

    let crees = commun::materialiser_les_rappels(&bac, terrain.seance).await;
    assert_eq!(
        crees as usize,
        INSCRITS * DEFAUT.len(),
        "cent soixante LIGNES en base : c'est bien la matière qui est posée"
    );

    let calendrier = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("le calendrier de la séance");

    assert!(calendrier.has_rule);
    assert_eq!(
        calendrier.slots.len(),
        4,
        "quatre lignes — une par décalage —, jamais cent soixante"
    );
    for ligne in &calendrier.slots {
        assert_eq!(
            ligne.recipient_count, INSCRITS as i64,
            "chaque ligne porte les quarante destinataires du groupe"
        );
        assert_eq!(ligne.channel, "email");
    }
}

// -----------------------------------------------------------------------------
// T125 — le balayage
// -----------------------------------------------------------------------------

/// 🔴 **Balayage de la charge utile SÉRIALISÉE ENTIÈRE.**
///
/// Champ par champ laisserait passer celui qu'on ajoutera demain : c'est la
/// réponse telle qu'elle part sur le réseau qui est fouillée, et non la liste
/// des champs qu'on a pensé à regarder (FR-048).
#[tokio::test]
async fn aucune_identite_ne_sort_du_calendrier() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let ajoutes = completer_les_inscrits(&bac, terrain.seance, terrain.inscrits.len()).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &regle_dedition(terrain.edition, &DEFAUT, None),
    )
    .await
    .expect("écriture de la règle");
    commun::materialiser_les_rappels(&bac, terrain.seance).await;

    let calendrier = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("le calendrier de la séance");
    let charge = serde_json::to_string(&calendrier).expect("sérialisation du calendrier");

    for personne in terrain.inscrits.iter().chain(ajoutes.iter()) {
        assert!(
            !charge.contains(&personne.to_string()),
            "un identifiant de personne figure dans la charge utile : {charge}"
        );
    }

    let inscriptions = sqlx::query_scalar!(
        "SELECT id FROM programme.registrations WHERE session_id = $1",
        terrain.seance
    )
    .fetch_all(bac.pool())
    .await
    .expect("les inscriptions de la séance");
    for inscription in inscriptions {
        assert!(
            !charge.contains(&inscription.to_string()),
            "un identifiant d'inscription figure dans la charge utile"
        );
    }

    // Les noms et les adresses sont bâtis pour être reconnaissables dans un
    // texte : « Prénom0 », « Patronyme0 », « temoin0@example.org ».
    for motif in [
        "Prénom",
        "Patronyme",
        "temoin",
        "@example.org",
        "Karim",
        "Fatou",
    ] {
        assert!(
            !charge.contains(motif),
            "« {motif} » figure dans la charge utile : {charge}"
        );
    }
}

// -----------------------------------------------------------------------------
// T126 — deux canaux sont deux envois
// -----------------------------------------------------------------------------

/// **Une règle à deux canaux rend huit lignes et non quatre.**
///
/// Le groupe est (décalage, canal) : agréger sur le seul décalage ferait
/// disparaître la moitié des envois de l'écran, et le nombre de destinataires
/// resterait juste — ce qui rendrait la faute invisible.
#[tokio::test]
async fn deux_canaux_sont_deux_envois() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &regle_dedition(
            terrain.edition,
            &DEFAUT,
            Some(vec!["email".to_owned(), "in_app".to_owned()]),
        ),
    )
    .await
    .expect("écriture de la règle à deux canaux");
    commun::materialiser_les_rappels(&bac, terrain.seance).await;

    let calendrier = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("le calendrier de la séance");

    assert_eq!(calendrier.slots.len(), 8, "quatre décalages × deux canaux");
    for canal in ["email", "in_app"] {
        assert_eq!(
            calendrier
                .slots
                .iter()
                .filter(|s| s.channel == canal)
                .count(),
            4,
            "les quatre décalages du canal « {canal} »"
        );
    }
}

// -----------------------------------------------------------------------------
// T127 — la consolidation, et le miroir confronté au SQL
// -----------------------------------------------------------------------------

/// 🔴 **Une ligne encore en attente parmi trente-neuf parties rend un groupe EN
/// ATTENTE**, jamais « parti » (R18).
///
/// « Parti » ne doit pas se dire tant qu'une personne attend encore son
/// courriel : l'organisation lirait « envoyé à quarante personnes » alors que
/// l'une d'elles n'a rien reçu.
#[tokio::test]
async fn une_ligne_en_attente_parmi_trente_neuf_retient_le_groupe() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    completer_les_inscrits(&bac, terrain.seance, terrain.inscrits.len()).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &regle_dedition(terrain.edition, &[1440], None),
    )
    .await
    .expect("écriture de la règle");
    commun::materialiser_les_rappels(&bac, terrain.seance).await;

    let lignes = lignes_du_groupe(&bac, terrain.seance).await;
    assert_eq!(lignes.len(), INSCRITS, "un seul décalage, un seul canal");
    for (rang, rappel) in lignes.iter().enumerate() {
        let etat = if rang == 0 {
            ReminderStatus::Pending
        } else {
            ReminderStatus::Sent
        };
        poser_letat(&bac, *rappel, etat).await;
    }

    let calendrier = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("le calendrier de la séance");

    assert_eq!(calendrier.slots.len(), 1);
    assert_eq!(
        calendrier.slots[0].status, "pending",
        "une seule personne qui attend retient le groupe entier"
    );
    assert_eq!(calendrier.slots[0].recipient_count, INSCRITS as i64);
}

/// 🔴 **Le miroir Rust et la règle SQL, confrontés sur le même jeu de lignes.**
///
/// `consolider()` reproduit la consolidation écrite dans
/// `engagement.session_reminder_schedule()` pour que les cas se prouvent sans
/// écrire trente lignes en base par cas. **L'écriture de référence reste le
/// SQL** : si les deux divergent, c'est le miroir qui a tort — et sans ce test,
/// la divergence ne se verrait nulle part.
#[tokio::test]
async fn le_miroir_rust_dit_la_meme_chose_que_le_sql() {
    use ReminderStatus::*;

    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &regle_dedition(terrain.edition, &[1440], None),
    )
    .await
    .expect("écriture de la règle");
    commun::materialiser_les_rappels(&bac, terrain.seance).await;

    let lignes = lignes_du_groupe(&bac, terrain.seance).await;
    assert_eq!(lignes.len(), 3, "les trois inscrits du harnais");

    for melange in [
        [Sent, Sent, Pending],
        [Sent, Queued, Sent],
        [Sent, Sent, Sent],
        [Skipped, Skipped, Cancelled],
        [Cancelled, Cancelled, Skipped],
        [Skipped, Cancelled, Sent],
    ] {
        for (rappel, etat) in lignes.iter().zip(melange) {
            poser_letat(&bac, *rappel, etat).await;
        }

        let calendrier = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
            .await
            .expect("le calendrier de la séance");
        let du_sql = calendrier.slots[0].status.as_str();
        let du_miroir = consolider(&melange).expect("un groupe non vide a un état");

        assert_eq!(
            du_sql,
            du_miroir.as_str(),
            "le miroir Rust diverge du SQL sur {melange:?} — c'est le miroir qui a tort"
        );
    }
}

/// **Le motif ne sort que sur un groupe mort.** Une ligne écartée dans un groupe
/// encore vivant ferait croire à un incident.
#[tokio::test]
async fn le_motif_ne_sort_que_sur_un_groupe_mort() {
    use ReminderStatus::*;

    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &regle_dedition(terrain.edition, &[1440], None),
    )
    .await
    .expect("écriture de la règle");
    commun::materialiser_les_rappels(&bac, terrain.seance).await;

    let lignes = lignes_du_groupe(&bac, terrain.seance).await;
    for (rappel, etat) in lignes.iter().zip([Skipped, Sent, Sent]) {
        poser_letat(&bac, *rappel, etat).await;
    }
    let vivant = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("le calendrier de la séance");
    assert_eq!(vivant.slots[0].status, "sent");
    assert!(
        vivant.slots[0].skip_reason.is_none(),
        "un groupe vivant ne porte pas de motif"
    );

    for rappel in &lignes {
        poser_letat(&bac, *rappel, Skipped).await;
    }
    let mort = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("le calendrier de la séance");
    assert_eq!(mort.slots[0].status, "skipped");
    assert_eq!(
        mort.slots[0].skip_reason.as_deref(),
        Some(engagement::domain::reminder::motifs::SUPPRIME)
    );
}

// -----------------------------------------------------------------------------
// T128 — l'ordre
// -----------------------------------------------------------------------------

/// **Du décalage le plus lointain au plus proche du début** (FR-050) — et **en
/// minutes**, jamais en texte : `'1 day'` et `'24 hours'` sont le même
/// intervalle pour la base et deux chaînes différentes pour un écran qui les
/// regrouperait par leur libellé.
#[tokio::test]
async fn les_lignes_vont_du_plus_lointain_au_plus_proche() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Écrits dans le désordre : c'est la lecture qui range, pas l'écriture.
    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &regle_dedition(terrain.edition, &[60, 2880, 30, 1440], None),
    )
    .await
    .expect("écriture de la règle");
    commun::materialiser_les_rappels(&bac, terrain.seance).await;

    let calendrier = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("le calendrier de la séance");

    let minutes: Vec<i32> = calendrier.slots.iter().map(|s| s.offset_before).collect();
    assert_eq!(minutes, DEFAUT.to_vec());

    // L'instant d'envoi suit le même ordre : le plus lointain part le premier.
    for paire in calendrier.slots.windows(2) {
        assert!(
            paire[0].scheduled_for < paire[1].scheduled_for,
            "les instants d'envoi ne suivent pas l'ordre des décalages"
        );
    }
}

// -----------------------------------------------------------------------------
// « Aucune règle » n'est pas « tout est parti »
// -----------------------------------------------------------------------------

/// **Une séance sans règle le DIT** (FR-051). Une liste vide muette se confond
/// avec un envoi réussi, et les deux situations demandent des mots différents à
/// l'écran.
#[tokio::test]
async fn une_seance_sans_regle_le_dit() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let calendrier = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("le calendrier de la séance");

    assert!(calendrier.slots.is_empty());
    assert!(
        !calendrier.has_rule,
        "aucune règle ne s'applique, et la réponse doit le dire"
    );

    assert!(
        schedule::regle_applicable(&bac.state, terrain.animatrice, terrain.seance)
            .await
            .expect("la règle applicable")
            .is_none()
    );
}

/// **La règle de séance remplace celle de l'édition**, et la lecture rend son
/// **origine** : sans elle, une règle de séance à deux décalages ne se distingue
/// pas d'une règle d'édition tronquée (FR-074).
#[tokio::test]
async fn la_regle_applicable_rend_son_origine() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &regle_dedition(terrain.edition, &DEFAUT, None),
    )
    .await
    .expect("règle d'édition");

    let dedition = schedule::regle_applicable(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("la règle applicable")
        .expect("une règle s'applique");
    assert_eq!(dedition.origin, "event");
    assert_eq!(dedition.origin_id, terrain.edition);
    assert_eq!(dedition.offsets, DEFAUT.to_vec());

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &ReminderRulePayload {
            event_id: None,
            session_id: Some(terrain.seance),
            offsets: Some(vec![1440, 60]),
            channels: None,
            type_code: None,
            template_id: None,
            is_active: None,
        },
    )
    .await
    .expect("règle de séance");

    let de_seance = schedule::regle_applicable(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("la règle applicable")
        .expect("une règle s'applique");
    assert_eq!(de_seance.origin, "session");
    assert_eq!(de_seance.origin_id, terrain.seance);
    assert_eq!(
        de_seance.offsets,
        vec![1440, 60],
        "deux décalages, jamais six : la règle de séance REMPLACE"
    );
}

// -----------------------------------------------------------------------------
// T121 — la garde
// -----------------------------------------------------------------------------

/// **L'adhésion active, ou le droit de gérer les inscriptions — jamais un
/// périmètre d'administration.**
///
/// Une organisation n'administre rien : lui demander un périmètre lui fermerait
/// le calendrier de sa propre séance.
#[tokio::test]
async fn la_garde_est_ladhesion_active_ou_la_gestion_des_inscriptions() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // L'animatrice : adhésion active à l'organisation qui anime.
    schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect("l'animatrice lit le calendrier de sa séance");

    // L'administratrice de l'édition : `programme.registration.manage`.
    schedule::calendrier(&bac.state, terrain.administratrice, terrain.seance)
        .await
        .expect("l'administratrice de l'édition lit le calendrier");

    // L'étrangère : ni l'un, ni l'autre.
    let refus = schedule::calendrier(&bac.state, terrain.etrangere, terrain.seance)
        .await
        .expect_err("une personne sans lien n'a pas accès au calendrier");
    assert_eq!(refus.code, kernel::ErrorCode::Forbidden);

    // Une séance inconnue reste un 404 : c'est l'objet qui manque, pas le droit.
    let inconnue = schedule::calendrier(&bac.state, terrain.animatrice, Uuid::now_v7())
        .await
        .expect_err("une séance inconnue");
    assert_eq!(inconnue.code, kernel::ErrorCode::NotFound);
}

/// **Une adhésion révoquée ne suffit plus.** C'est le cas que la lecture
/// « membre de l'organisation » manquerait : l'adhésion existe toujours, elle
/// n'est simplement plus active.
#[tokio::test]
async fn une_adhesion_revoquee_ne_donne_plus_acces() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    sqlx::query!(
        "UPDATE org.memberships SET status = 'revoked'
          WHERE organization_id = $1 AND person_id = $2",
        terrain.organisation,
        terrain.animatrice
    )
    .execute(bac.pool())
    .await
    .expect("révocation de l'adhésion");

    let refus = schedule::calendrier(&bac.state, terrain.animatrice, terrain.seance)
        .await
        .expect_err("une adhésion révoquée ne donne pas accès");
    assert_eq!(refus.code, kernel::ErrorCode::Forbidden);
}
