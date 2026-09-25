//! **`GET /negotiation/sessions`** — coupé, la liste est vide quelles que soient
//! les lignes en base (SC-004) ; servi, chaque session porte la forme du
//! contrat, « Déplacée » avec sa valeur d'avant, la thématique héritée de son
//! point, et l'heure de la dernière lecture où elle figurait.

mod importation;

use importation::Bac;
use kernel::error::ErrorCode;
use negotiation::domain::sessions::{EtatServi, MotifDeCoupure, OfficialSession, OfficialSessions};
use negotiation::state::NegotiationState;
use serde_json::json;
use std::sync::Arc;
use time::macros::datetime;
use uuid::Uuid;

fn etat(bac: &Bac) -> NegotiationState {
    NegotiationState::new(
        bac.base.db(),
        Arc::new(kernel::testing::test_config(bac.base.url())),
    )
}

async fn servir(bac: &Bac) -> OfficialSessions {
    negotiation::service::sessions::de_ledition(&etat(bac), "cop31")
        .await
        .expect("lecture des sessions")
}

async fn session<'a>(bac: &Bac, rendu: &'a OfficialSessions, cle: &str) -> &'a OfficialSession {
    let id: Uuid = bac.session(cle).await.expect("session en base").id;
    rendu
        .sessions
        .iter()
        .find(|s| s.id == id)
        .expect("session servie")
}

#[tokio::test]
async fn import_eteint_coupe_disabled_et_liste_vide() {
    let bac = Bac::monter().await;
    bac.regler("is_enabled = false").await;

    let rendu = servir(&bac).await;
    assert_eq!(rendu.state, EtatServi::Cut);
    assert_eq!(rendu.cut_reason, Some(MotifDeCoupure::Disabled));
    assert!(rendu.sessions.is_empty());
    assert!(rendu.read_at.is_none());
    assert_eq!(
        rendu.official_programme_url,
        "https://unfccc.int/cop31/schedule"
    );
    assert_eq!(
        (rendu.edition.slug.as_str(), rendu.edition.timezone.as_str()),
        ("cop31", "Asia/Istanbul")
    );
}

#[tokio::test]
async fn jamais_lu_est_coupe_disabled() {
    let bac = Bac::monter().await;
    let rendu = servir(&bac).await;
    assert_eq!(
        (rendu.state, rendu.cut_reason),
        (EtatServi::Cut, Some(MotifDeCoupure::Disabled))
    );
}

#[tokio::test]
async fn coupe_la_liste_est_vide_meme_avec_des_lignes_en_base() {
    let bac = Bac::monter().await;
    bac.lire().await;
    assert_eq!(servir(&bac).await.sessions.len(), 44);

    bac.regler("missed_reads = missed_threshold, failing_since = now()")
        .await;
    let rendu = servir(&bac).await;
    assert_eq!(bac.sessions().await, 44, "les lignes restent en base");
    assert_eq!(
        (rendu.state, rendu.cut_reason),
        (EtatServi::Cut, Some(MotifDeCoupure::Unreachable))
    );
    assert!(rendu.sessions.is_empty());
    assert!(rendu.failing_since.is_some() && rendu.read_at.is_some());
}

#[tokio::test]
async fn edition_inconnue_404() {
    let bac = Bac::monter().await;
    let refus = negotiation::service::sessions::de_ledition(&etat(&bac), "cop99")
        .await
        .expect_err("édition inconnue");
    assert_eq!(refus.code, ErrorCode::NegotiationEditionUnknown);
    assert_eq!(refus.code.status().as_u16(), 404);
}

#[tokio::test]
async fn servi_les_formes_du_contrat() {
    let bac = Bac::monter().await;
    bac.lire().await;
    let titre = "CMA 8 (a) Global goal on adaptation - Informal consultation";
    sqlx::query(
        "INSERT INTO negotiation.title_translations (source_text, text_fr, model)
         VALUES ($1, 'CMA 8 (a) Objectif mondial d''adaptation — consultation informelle', 'essai')",
    )
    .bind(titre)
    .execute(bac.pool())
    .await
    .expect("traduction");
    bac.executer(
        "UPDATE negotiation.agenda_items SET theme_term_id = (
             SELECT id FROM reference.taxonomy_terms
              WHERE taxonomy_code = 'negotiation_theme' AND code = 'adaptation')
          WHERE code = 'CMA 8 (a)'",
    )
    .await;

    let rendu = servir(&bac).await;
    assert_eq!((rendu.state, rendu.cut_reason), (EtatServi::Serving, None));
    assert_eq!(rendu.sessions.len(), 44);
    assert!(rendu
        .sessions
        .windows(2)
        .all(|p| p[0].start_at <= p[1].start_at));

    let s = serde_json::to_value(session(&bac, &rendu, "654006").await).expect("json");
    assert_eq!(s["title_en"], titre);
    assert_eq!(
        s["title_fr"],
        "CMA 8 (a) Objectif mondial d'adaptation — consultation informelle"
    );
    assert_eq!(s["start_at"], "2026-11-10T08:00:00Z");
    assert_eq!(s["end_at"], "2026-11-10T09:00:00Z");
    assert_eq!(s["venue"], "Meeting Room 01");
    assert_eq!(s["previous"], json!(null));
    assert_eq!(s["type"]["code"], "informal_consultations");
    assert!(s["type"]["label"]["fr"].is_string());
    assert_eq!(s["type"]["term_en"], s["type"]["label"]["en"]);
    assert_eq!(s["group"], json!(null));
    assert_eq!(s["theme"], "adaptation", "héritée du point");
    assert_eq!(
        s["agenda_item"],
        json!({ "code": "CMA 8 (a)", "title": "Global goal on adaptation" })
    );
    assert_eq!(s["open_access"], true);
    assert_eq!(s["status"], "scheduled");
    assert_eq!(s["cancelled"], json!(null));
    assert!(s["source_url"].is_string());
    assert!(s.get("documents").is_none());

    let coordination = serde_json::to_value(session(&bac, &rendu, "654012").await).expect("json");
    assert_eq!(coordination["group"]["code"], "grulac");
    assert!(coordination["group"]["label"]["fr"].is_string());
    assert_eq!(coordination["theme"], json!(null));
    assert_eq!(coordination["title_fr"], json!(null));

    let annulee = serde_json::to_value(session(&bac, &rendu, "654417").await).expect("json");
    assert_eq!(annulee["status"], "cancelled");
    assert_eq!(annulee["cancelled"]["reason"], "source");
    assert!(annulee["cancelled"]["at"].is_string());
}

#[tokio::test]
async fn la_deplacee_porte_sa_valeur_d_avant() {
    let bac = Bac::monter().await;
    bac.lire().await;
    bac.jeu("cop30/lecture-2").await;
    bac.lire().await;
    let rendu = servir(&bac).await;

    let avancee = session(&bac, &rendu, "654214").await;
    assert_eq!(avancee.start_at, datetime!(2026-11-10 11:00 UTC));
    let avant = avancee.previous.as_ref().expect("déplacée");
    assert_eq!(avant.start_at, datetime!(2026-11-10 12:00 UTC));
    assert_eq!(avant.end_at, Some(datetime!(2026-11-10 13:00 UTC)));
    assert_eq!(avant.venue, avancee.venue, "la salle n'a pas changé");

    let salle = session(&bac, &rendu, "654010").await;
    let avant = salle.previous.as_ref().expect("changée de salle");
    assert_eq!(avant.venue.as_deref(), Some("Meeting Room 01"));
    assert_eq!(salle.venue.as_deref(), Some("Meeting Room 03"));
    assert_eq!(
        (avant.start_at, avant.end_at),
        (salle.start_at, salle.end_at)
    );
    assert!(session(&bac, &rendu, "699001").await.previous.is_none());
}

/// Principe XII : une session relue sans changement à T1 ne dit jamais « lue à
/// T0 » ; seule une session absente garde sa dernière lecture.
#[tokio::test]
async fn l_heure_de_lecture_suit_la_derniere_lecture_ou_elle_figurait() {
    let bac = Bac::monter().await;
    bac.lire().await;
    let t0 = bac.etat().await.last_success_at.expect("T0");
    bac.lire().await;
    let t1 = bac.etat().await.last_success_at.expect("T1");
    assert!(t1 > t0);

    let rendu = servir(&bac).await;
    assert_eq!(rendu.read_at, Some(t1));
    let s = session(&bac, &rendu, "654006").await;
    assert_eq!(
        bac.session("654006").await.expect("en base").last_read_at,
        t0
    );
    assert_eq!(s.read_at, t1);

    bac.jeu("cop30/lecture-2").await;
    bac.lire().await;
    let t2 = bac.etat().await.last_success_at.expect("T2");
    let rendu = servir(&bac).await;
    assert_eq!(session(&bac, &rendu, "654006").await.read_at, t2);
    assert_eq!(
        session(&bac, &rendu, "654364").await.read_at,
        t1,
        "absente à T2 : sa dernière lecture est T1"
    );
}

#[tokio::test]
async fn l_empreinte_ignore_l_heure_du_serveur_et_suit_les_ecarts() {
    let bac = Bac::monter().await;
    bac.lire().await;
    let a = servir(&bac).await;
    let b = servir(&bac).await;
    assert_eq!(a.empreinte(), b.empreinte());
    assert!(kernel::empreinte::correspond(
        &a.empreinte(),
        &b.empreinte()
    ));

    bac.jeu("cop30/lecture-2").await;
    bac.lire().await;
    assert_ne!(servir(&bac).await.empreinte(), a.empreinte());
}
