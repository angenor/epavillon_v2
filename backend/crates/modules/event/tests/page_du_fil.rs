//! **La page publique d'un fil, côté back-office** (30/10) : lieu, accroche,
//! publics, partenaires et mention, encart, parties, moments.
//!
//! Les services sont appelés après la garde, comme le font les routes ; la
//! garde elle-même (ascendance, périmètre) est éprouvée sur la cible du fil.

mod commun;

use commun::{perimetres, seed, Bac};
use event::domain::ids::TrackId;
use event::domain::track_page::{
    TrackMomentPayload, TrackPagePayload, TrackPartPayload, TrackPartnerPayload,
};
use event::service::{edition_dans_le_perimetre, track_page, Cible};
use kernel::error::ErrorCode;
use serde_json::json;
use uuid::Uuid;

async fn organisation(bac: &Bac, nom: &str, adresse: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO org.organizations (legal_name, slug, organization_type_code, status)
           VALUES ($1, $2::text::platform.slug, 'igo', 'active')
        RETURNING id"#,
        nom,
        adresse
    )
    .fetch_one(bac.pool())
    .await
    .expect("organisation")
}

async fn hotel(bac: &Bac, event_id: Uuid) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.venues (event_id, name, kind, address)
           VALUES ($1, '{"fr":"Hôtel DoubleTree by Hilton"}'::jsonb, 'other',
                   'Etiler, Adnan Menderes Blv. No:61, 07010 Muratpaşa / Antalya')
        RETURNING id"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("l'hôtel")
}

fn page_vide() -> TrackPagePayload {
    TrackPagePayload {
        venue_id: None,
        tagline: None,
        target_audiences: Vec::new(),
        support_note: None,
        feature_kicker: None,
        feature_title: None,
        feature_body: None,
        feature_link_label: None,
        feature_link_url: None,
        partners: Vec::new(),
    }
}

fn partie(nom: &str, rang: i16) -> TrackPartPayload {
    TrackPartPayload {
        label: Some(json!({ "fr": nom })),
        title: None,
        summary: None,
        sort_order: rang,
    }
}

fn moment(part_id: Option<Uuid>, heure: &str, libelle: &str) -> TrackMomentPayload {
    TrackMomentPayload {
        part_id,
        starts_at: commun::instant(heure),
        ends_at: None,
        label: json!({ "fr": libelle }),
        description: None,
    }
}

#[tokio::test]
async fn la_page_secrit_et_se_relit_entiere_partenaires_remplaces() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    let fil = TrackId::from(enfants.fil);
    let lieu = hotel(&bac, editions.cop31).await;
    let oif = organisation(&bac, "Organisation internationale de la Francophonie", "oif").await;
    let quebec = organisation(&bac, "Gouvernement du Québec", "quebec").await;

    let mut page = page_vide();
    page.venue_id = Some(lieu);
    page.tagline = Some(json!({ "fr": "Hackathon, pitchs, jeu, scène" }));
    page.target_audiences = vec![json!({ "fr": "Négociateurs" }), json!({ "fr": "ONG" })];
    page.support_note = Some(json!({ "fr": "Avec le soutien financier du gouvernement du Québec." }));
    page.feature_title = Some(json!({ "fr": "Guide des négociations de la CdP31" }));
    page.feature_link_label = Some(json!({ "fr": "Ouvrir dans Guide Négo" }));
    page.feature_link_url = Some("/guide-nego".to_owned());
    page.partners = vec![
        TrackPartnerPayload { organization_id: oif, role_code: "organizer".to_owned() },
        TrackPartnerPayload { organization_id: quebec, role_code: "financial_support".to_owned() },
        // Citée deux fois : sa première place compte.
        TrackPartnerPayload { organization_id: oif, role_code: "partner".to_owned() },
    ];

    let reglages = track_page::enregistrer(&bac.state, &bac.ctx(), fil, page)
        .await
        .expect("enregistrement");

    assert_eq!(reglages.venue_id, Some(lieu));
    assert_eq!(reglages.target_audiences.len(), 2);
    assert_eq!(reglages.feature_link_url.as_deref(), Some("/guide-nego"));
    let roles: Vec<(Uuid, &str)> = reglages
        .partners
        .iter()
        .map(|p| (p.organization_id, p.role_code.as_str()))
        .collect();
    assert_eq!(roles, vec![(oif, "organizer"), (quebec, "financial_support")]);
    assert_eq!(reglages.partners[1].role_label["fr"], "Soutien financier");

    let mut seconde = page_vide();
    seconde.partners = vec![TrackPartnerPayload {
        organization_id: quebec,
        role_code: "partner".to_owned(),
    }];
    let reglages = track_page::enregistrer(&bac.state, &bac.ctx(), fil, seconde)
        .await
        .expect("seconde écriture");

    assert_eq!(reglages.venue_id, None, "écriture totale : le lieu revient au pavillon");
    assert_eq!(reglages.partners.len(), 1, "la liste est remplacée");
    assert_eq!(reglages.partners[0].role_code, "partner");
}

#[tokio::test]
async fn un_lieu_dune_autre_edition_un_role_inconnu_un_lien_sans_libelle_sont_refuses() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    let fil = TrackId::from(enfants.fil);
    let ailleurs = hotel(&bac, editions.sans_pavillon).await;

    let mut page = page_vide();
    page.venue_id = Some(ailleurs);
    let refus = track_page::enregistrer(&bac.state, &bac.ctx(), fil, page)
        .await
        .expect_err("lieu d'une autre édition");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);

    let oif = organisation(&bac, "OIF", "oif-bis").await;
    let mut page = page_vide();
    page.partners = vec![TrackPartnerPayload {
        organization_id: oif,
        role_code: "mecene_imaginaire".to_owned(),
    }];
    let refus = track_page::enregistrer(&bac.state, &bac.ctx(), fil, page)
        .await
        .expect_err("rôle inconnu");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("partners"));

    let mut page = page_vide();
    page.feature_title = Some(json!({ "fr": "Guide" }));
    page.feature_link_url = Some("/guide-nego".to_owned());
    let refus = track_page::enregistrer(&bac.state, &bac.ctx(), fil, page)
        .await
        .expect_err("lien sans libellé");
    assert_eq!(refus.field.as_deref(), Some("feature_link_url"));
}

#[tokio::test]
async fn retirer_une_partie_garde_ses_moments_et_ses_activites_dans_le_fil() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    let seance = seed::seance(&bac, editions.cop31, &enfants).await;
    let fil = TrackId::from(enfants.fil);

    let reglages = track_page::creer_partie(&bac.state, &bac.ctx(), fil, partie("Matin", 1))
        .await
        .expect("partie");
    let matin = reglages.parts[0].id;

    track_page::creer_moment(
        &bac.state,
        &bac.ctx(),
        fil,
        moment(Some(matin), "2027-11-10T12:00:00-03:00", "Accueil"),
    )
    .await
    .expect("moment");

    sqlx::query!(
        "UPDATE programme.session_tracks SET part_id = $1 WHERE session_id = $2",
        matin,
        seance
    )
    .execute(bac.pool())
    .await
    .expect("activité dans la partie");

    let reglages = track_page::supprimer_partie(&bac.state, &bac.ctx(), fil, matin)
        .await
        .expect("retrait");

    assert!(reglages.parts.is_empty());
    assert_eq!(reglages.moments.len(), 1, "le moment reste");
    assert_eq!(reglages.moments[0].part_id, None);
    assert_eq!(reglages.sessions.len(), 1, "l'activité reste rattachée");
    assert_eq!(reglages.sessions[0].part_id, None);
}

#[tokio::test]
async fn une_partie_dun_autre_fil_est_introuvable_et_ne_se_designe_pas() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    let finance = TrackId::from(enfants.fil);
    let atelier = TrackId::from(
        sqlx::query_scalar!(
            r#"INSERT INTO event.programme_tracks (event_id, code, slug, kind, title)
               VALUES ($1, 'atelier', 'atelier'::platform.slug, 'side_programme',
                       '{"fr":"Atelier préparatoire"}'::jsonb)
            RETURNING id"#,
            editions.cop31
        )
        .fetch_one(bac.pool())
        .await
        .expect("second fil"),
    );

    let partie_finance =
        track_page::creer_partie(&bac.state, &bac.ctx(), finance, partie("Partie 1", 1))
            .await
            .expect("partie")
            .parts[0]
            .id;

    let refus =
        track_page::modifier_partie(&bac.state, &bac.ctx(), atelier, partie_finance, partie("x", 1))
            .await
            .expect_err("partie d'un autre fil");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let refus = track_page::creer_moment(
        &bac.state,
        &bac.ctx(),
        atelier,
        moment(Some(partie_finance), "2027-11-08T12:00:00-03:00", "Déjeuner"),
    )
    .await
    .expect_err("un moment ne désigne pas la partie d'un autre fil");
    assert_eq!(refus.field.as_deref(), Some("part_id"));

    let mut sans_fin = moment(None, "2027-11-08T12:00:00-03:00", "Déjeuner");
    sans_fin.ends_at = Some(commun::instant("2027-11-08T11:00:00-03:00"));
    let refus = track_page::creer_moment(&bac.state, &bac.ctx(), atelier, sans_fin)
        .await
        .expect_err("fin avant le début");
    assert_eq!(refus.field.as_deref(), Some("ends_at"));
}

/// **Règle n° 8** : le fil d'une édition hors périmètre est refusé comme un fil
/// inexistant, avant que la page ne soit lue ou écrite.
#[tokio::test]
async fn hors_perimetre_le_fil_est_refuse_comme_inexistant() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    let p = perimetres(&bac, &editions).await;
    let detache = commun::perimetre_de(&bac, p.detache).await;

    let hors = edition_dans_le_perimetre(bac.pool(), &detache, Cible::Fil(TrackId::from(enfants.fil)))
        .await
        .expect_err("la COP31 n'est pas dans son périmètre");
    let inconnu =
        edition_dans_le_perimetre(bac.pool(), &detache, Cible::Fil(TrackId::from(Uuid::now_v7())))
            .await
            .expect_err("fil inconnu");

    assert_eq!(hors.code, ErrorCode::NotFound);
    assert_eq!(hors.code, inconnu.code);

    let globale = commun::perimetre_de(&bac, p.globale).await;
    edition_dans_le_perimetre(bac.pool(), &globale, Cible::Fil(TrackId::from(enfants.fil)))
        .await
        .expect("l'administratrice de la plateforme y accède");
}
