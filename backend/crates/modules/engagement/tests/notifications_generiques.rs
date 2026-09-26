//! La branche générique du consommateur (Guide Négo 3b, research R8) :
//! l'émetteur a résolu destinataires et texte, `engagement` écrit sans rien
//! savoir de ses règles. Le remplacement d'un avis non lu, le filtre par
//! module, et les avis de `programme` inchangés.

mod commun;

use commun::Bac;
use contracts::negotiation::{
    Notification, NotificationSubject, NotificationText, WithNotification, MEETING_CHANGED,
};
use engagement::domain::notification::NotificationPreferencePayload;
use engagement::service::notifications::{self, FilQuery};
use kernel::events::{emit, DomainEvent};
use serde_json::json;
use uuid::Uuid;

fn fil(module: Option<&str>) -> FilQuery {
    FilQuery {
        unread_only: false,
        limit: None,
        before: None,
        module: module.map(str::to_owned),
    }
}

fn avis(recipients: Vec<Uuid>, titre: &str, group_key: Option<&str>) -> Notification {
    Notification {
        type_code: MEETING_CHANGED.to_owned(),
        recipients,
        title: NotificationText {
            fr: titre.to_owned(),
            en: format!("{titre} (en)"),
        },
        body: NotificationText {
            fr: format!("{titre}. Sessions de négociation"),
            en: format!("{titre}. Negotiation sessions"),
        },
        link_path: "/guide-nego/negociations/0192".to_owned(),
        subject: NotificationSubject {
            schema: "negotiation".to_owned(),
            table: "meetings".to_owned(),
            id: Uuid::nil(),
        },
        replace: group_key.is_some(),
        group_key: group_key.map(str::to_owned),
        variables: json!({ "change": "deplacee" }),
    }
}

async fn emettre(bac: &Bac, n: Notification) {
    let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
    emit(
        &mut tx,
        DomainEvent {
            aggregate_schema: "negotiation",
            aggregate_type: "meeting",
            aggregate_id: Uuid::now_v7(),
            event_type: MEETING_CHANGED,
            payload: serde_json::to_value(WithNotification { notification: n }).expect("charge"),
        },
    )
    .await
    .expect("émission");
    tx.commit().await.expect("validation");
}

#[tokio::test]
async fn un_avis_par_destinataire_apres_le_canal() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    notifications::ecrire_les_preferences(
        &bac.state,
        &bac.ctx(),
        t.inscrits[1],
        "fr",
        &[NotificationPreferencePayload {
            type_code: MEETING_CHANGED.to_owned(),
            channel: "in_app".to_owned(),
            is_enabled: false,
        }],
    )
    .await
    .expect("préférence");

    emettre(
        &bac,
        avis(vec![t.inscrits[0], t.inscrits[1]], "Déplacée — CMA 8", None),
    )
    .await;
    commun::relayer(&bac, "negotiation.").await;

    let lu = notifications::fil(&bac.state, t.inscrits[0], &fil(None))
        .await
        .expect("fil");
    assert_eq!(lu.items.len(), 1);
    let n = &lu.items[0];
    assert_eq!(n.type_code, MEETING_CHANGED);
    assert_eq!(n.title.as_ref().expect("titre")["fr"], "Déplacée — CMA 8");
    assert_eq!(
        n.link_path.as_deref(),
        Some("/guide-nego/negociations/0192")
    );
    assert_eq!(n.subject_table.as_deref(), Some("meetings"));
    assert_eq!(
        notifications::fil(&bac.state, t.inscrits[1], &fil(None))
            .await
            .expect("fil")
            .items
            .len(),
        0,
        "elle a coupé l'écran pour ce type"
    );
    assert_eq!(
        notifications::fil(&bac.state, t.inscrits[2], &fil(None))
            .await
            .expect("fil")
            .items
            .len(),
        0,
        "hors de la liste reçue, rien"
    );
}

#[tokio::test]
async fn un_type_inactif_nest_pas_ecrit() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    sqlx::query!(
        "UPDATE engagement.notification_types SET is_active = false WHERE code = $1",
        MEETING_CHANGED
    )
    .execute(bac.pool())
    .await
    .expect("type éteint");

    emettre(&bac, avis(vec![t.inscrits[0]], "Annulée — CMA 8", None)).await;
    commun::relayer(&bac, "negotiation.").await;

    assert!(notifications::fil(&bac.state, t.inscrits[0], &fil(None))
        .await
        .expect("fil")
        .items
        .is_empty());
}

#[tokio::test]
async fn meme_cle_non_lue_remplace_le_texte_et_compte() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let cle = "negotiation.meeting.changed:x:2026-11-10";

    emettre(
        &bac,
        avis(vec![t.inscrits[0]], "Déplacée — CMA 8", Some(cle)),
    )
    .await;
    emettre(
        &bac,
        avis(vec![t.inscrits[0]], "Annulée — CMA 8", Some(cle)),
    )
    .await;
    commun::relayer(&bac, "negotiation.").await;

    let lu = notifications::fil(&bac.state, t.inscrits[0], &fil(None))
        .await
        .expect("fil");
    assert_eq!(lu.items.len(), 1, "une ligne");
    assert_eq!(lu.items[0].group_count, 2);
    assert_eq!(
        lu.items[0].title.as_ref().expect("titre")["fr"],
        "Annulée — CMA 8",
        "l'état final"
    );
    assert_eq!(
        lu.items[0].body.as_ref().expect("corps")["en"],
        "Annulée — CMA 8. Negotiation sessions"
    );
}

#[tokio::test]
async fn le_filtre_module_trie_la_liste_et_le_compte_programme_inchange() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let personne = t.inscrits[0];

    sqlx::query!(
        "UPDATE programme.sessions
            SET starts_at = starts_at + interval '1 hour', ends_at = ends_at + interval '1 hour'
          WHERE id = $1",
        t.seance
    )
    .execute(bac.pool())
    .await
    .expect("séance déplacée");
    commun::relayer(&bac, "programme.session.rescheduled").await;
    emettre(&bac, avis(vec![personne], "Déplacée — CMA 8", None)).await;
    commun::relayer(&bac, "negotiation.").await;

    let tout = notifications::fil(&bac.state, personne, &fil(None))
        .await
        .expect("fil");
    assert_eq!((tout.items.len(), tout.unread_count), (2, 2));

    let nego = notifications::fil(&bac.state, personne, &fil(Some("negotiation")))
        .await
        .expect("fil");
    assert_eq!((nego.items.len(), nego.unread_count), (1, 1));
    assert_eq!(nego.items[0].type_code, MEETING_CHANGED);

    let prog = notifications::fil(&bac.state, personne, &fil(Some("programme")))
        .await
        .expect("fil");
    assert_eq!((prog.items.len(), prog.unread_count), (1, 1));
    let p = &prog.items[0];
    assert_eq!(p.type_code, "programme.session.rescheduled");
    assert_eq!(p.subject_schema.as_deref(), Some("programme"));
    assert_eq!(
        p.title.as_ref().expect("libellé du type")["fr"],
        "Session reprogrammée",
        "la branche de programme écrit toujours le libellé du type"
    );
}

#[tokio::test]
async fn tout_marquer_dun_module_laisse_les_avis_du_site_non_lus() {
    let bac = Bac::monter().await;
    let t = commun::terrain(&bac).await;
    let personne = t.inscrits[0];

    sqlx::query!(
        "UPDATE programme.sessions
            SET starts_at = starts_at + interval '1 hour', ends_at = ends_at + interval '1 hour'
          WHERE id = $1",
        t.seance
    )
    .execute(bac.pool())
    .await
    .expect("séance déplacée");
    commun::relayer(&bac, "programme.session.rescheduled").await;
    emettre(&bac, avis(vec![personne], "Déplacée — CMA 8", None)).await;
    commun::relayer(&bac, "negotiation.").await;

    let marquees = notifications::marquer_lues(
        &bac.state,
        &bac.ctx(),
        personne,
        &notifications::MarquagePayload { ids: None },
        Some("negotiation"),
    )
    .await
    .expect("marquage");
    assert_eq!(marquees, 1);

    let nego = notifications::fil(&bac.state, personne, &fil(Some("negotiation")))
        .await
        .expect("fil");
    assert_eq!(nego.unread_count, 0);
    let prog = notifications::fil(&bac.state, personne, &fil(Some("programme")))
        .await
        .expect("fil");
    assert_eq!(
        (prog.unread_count, prog.items[0].read_at.is_none()),
        (1, true),
        "l'avis du site reste non lu"
    );
}
