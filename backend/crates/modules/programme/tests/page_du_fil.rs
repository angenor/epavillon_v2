//! **La page publique d'un fil** (30/10) : un fil publié se lit en une réponse,
//! déroulé fusionné et trié ; un fil non publié n'existe pas. Et le placement
//! d'une activité dans une partie.

mod commun;

use commun::{Bac, FUSEAU_COP31, SLUG_COP31};
use kernel::error::ErrorCode;
use programme::domain::ids::{EventId, SessionId};
use programme::domain::track_page::{PublicTrackEntryBody, TrackSessionPlacementPayload};
use programme::service::track_page;
use uuid::Uuid;

async fn fil(bac: &Bac, edition: Uuid, slug: &str, publie: bool, lieu: Option<Uuid>) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO event.programme_tracks
               (event_id, code, slug, kind, title, starts_on, color_hex, published_at, venue_id)
           VALUES ($1, replace($2, '-', '_'), $2::text::platform.slug, 'special_day',
                   jsonb_build_object('fr', 'Journée ' || $2), '2027-11-13', '#00A1E4',
                   CASE WHEN $3 THEN now() END, $4)
        RETURNING id"#,
        edition,
        slug,
        publie,
        lieu
    )
    .fetch_one(bac.pool())
    .await
    .expect("fil")
}

/// Une séance posée en base, rattachée au fil. Heures murales de Belém.
async fn seance(
    bac: &Bac,
    edition: Uuid,
    track: Uuid,
    slug: &str,
    debut: &str,
    fin: &str,
    publiee: bool,
    organisation: Uuid,
) -> Uuid {
    let id = sqlx::query_scalar!(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, format, timezone, starts_at, ends_at, status,
                published_at, organization_id)
           VALUES ($1, jsonb_build_object('fr', $2::text), $2::text::platform.slug, 'hybrid',
                   $5::text::platform.timezone_name,
                   ($3::text)::timestamp AT TIME ZONE $5,
                   ($4::text)::timestamp AT TIME ZONE $5,
                   'scheduled', CASE WHEN $6 THEN now() END, $7)
        RETURNING id"#,
        edition,
        slug,
        debut,
        fin,
        FUSEAU_COP31,
        publiee,
        organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("séance");

    sqlx::query!(
        "INSERT INTO programme.session_tracks (session_id, track_id) VALUES ($1, $2)",
        id,
        track
    )
    .execute(bac.pool())
    .await
    .expect("rattachement");
    id
}

async fn partie(bac: &Bac, track: Uuid, nom: &str, rang: i16) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO event.programme_track_parts (track_id, label, sort_order)
         VALUES ($1, jsonb_build_object('fr', $2::text), $3) RETURNING id",
        track,
        nom,
        rang
    )
    .fetch_one(bac.pool())
    .await
    .expect("partie")
}

async fn moment(bac: &Bac, track: Uuid, part: Option<Uuid>, libelle: &str, debut: &str, fin: Option<&str>) {
    sqlx::query!(
        "INSERT INTO event.programme_track_moments (track_id, part_id, label, starts_at, ends_at)
         VALUES ($1, $2, jsonb_build_object('fr', $3::text),
                 ($4::text)::timestamp AT TIME ZONE $6,
                 ($5::text)::timestamp AT TIME ZONE $6)",
        track,
        part,
        libelle,
        debut,
        fin,
        FUSEAU_COP31
    )
    .execute(bac.pool())
    .await
    .expect("moment");
}

fn genre(entree: &programme::domain::track_page::PublicTrackEntry) -> &'static str {
    match entree.body {
        PublicTrackEntryBody::Activity { .. } => "activity",
        PublicTrackEntryBody::Moment { .. } => "moment",
    }
}

#[tokio::test]
async fn un_fil_publie_se_lit_en_une_reponse_deroule_fusionne_et_trie() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let finance = fil(&bac, terrain.edition, "journee-finance", true, None).await;
    let hotel = sqlx::query_scalar!(
        r#"INSERT INTO event.venues (event_id, name, kind)
           VALUES ($1, '{"fr":"Hôtel DoubleTree by Hilton"}'::jsonb, 'other') RETURNING id"#,
        terrain.edition
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    fil(&bac, terrain.edition, "atelier", true, Some(hotel)).await;
    fil(&bac, terrain.edition, "journee-cachee", false, None).await;

    let p1 = partie(&bac, finance, "Partie 1", 1).await;
    let p2 = partie(&bac, finance, "Partie 2", 2).await;
    let tard = seance(&bac, terrain.edition, finance, "cilss", "2027-11-13 10:15", "2027-11-13 11:15", true, terrain.organisation).await;
    let tot = seance(&bac, terrain.edition, finance, "ouverture", "2027-11-13 09:00", "2027-11-13 09:15", true, terrain.organisation).await;
    seance(&bac, terrain.edition, finance, "brouillon", "2027-11-13 11:30", "2027-11-13 12:30", false, terrain.organisation).await;
    sqlx::query!(
        "UPDATE programme.session_tracks SET part_id = $1, is_highlight = true WHERE session_id = $2",
        p1,
        tot
    )
    .execute(bac.pool())
    .await
    .unwrap();
    sqlx::query!("UPDATE programme.session_tracks SET part_id = $1 WHERE session_id = $2", p2, tard)
        .execute(bac.pool())
        .await
        .unwrap();
    moment(&bac, finance, Some(p1), "Accueil et installation", "2027-11-13 08:00", Some("2027-11-13 08:45")).await;
    moment(&bac, finance, Some(p2), "Clôture", "2027-11-13 18:00", None).await;
    let role = sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms WHERE taxonomy_code = 'track_partner_role' AND code = 'partner'"
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO event.programme_track_partners (track_id, organization_id, role_term_id) VALUES ($1, $2, $3)",
        finance,
        terrain.organisation,
        role
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let page = track_page::page(bac.pool(), SLUG_COP31, "journee-finance")
        .await
        .expect("page publiée");

    let deroule: Vec<&str> = page.agenda.iter().map(genre).collect();
    assert_eq!(deroule, ["moment", "activity", "activity", "moment"], "trié par heure, brouillon écarté");
    match &page.agenda[1].body {
        PublicTrackEntryBody::Activity { activity } => {
            assert_eq!(activity.session.id, tot);
            assert!(activity.is_highlight);
            assert_eq!(activity.organizations.len(), 1, "le porteur");
            assert!(activity.organizations[0].get("logo").is_some());
        }
        _ => panic!("une activité attendue"),
    }
    assert_eq!(page.agenda[1].part_id, Some(p1));

    let heure = |h: &str| {
        commun::instant(&format!("2027-11-13T{h}:00-03:00"))
    };
    assert_eq!(page.starts_at, Some(heure("09:00")), "première activité");
    assert_eq!(page.opens_at, Some(heure("08:00")), "accueil avant");
    assert_eq!(page.ends_at, Some(heure("18:00")), "la clôture sans fin compte");
    assert_eq!(page.parts.len(), 2);
    assert_eq!(page.parts[0].number, 1);
    assert_eq!(page.parts[0].starts_at, Some(heure("08:00")));
    assert_eq!(page.parts[0].ends_at, Some(heure("09:15")));

    assert!(!page.is_off_site);
    assert_eq!(page.venue.as_ref().map(|v| v.kind.as_str()), Some("pavilion"));
    assert_eq!(page.edition.timezone, FUSEAU_COP31);
    assert_eq!(page.partners.len(), 1);

    let autres: Vec<(&str, bool, bool)> = page
        .other_tracks
        .iter()
        .map(|t| (t.slug.as_str(), t.is_current, t.is_off_site))
        .collect();
    assert_eq!(autres.len(), 2, "le fil non publié n'est pas relié");
    assert!(autres.contains(&("journee-finance", true, false)));
    assert!(autres.contains(&("atelier", false, true)));

    let atelier = track_page::page(bac.pool(), SLUG_COP31, "atelier").await.expect("atelier");
    assert!(atelier.is_off_site);
    assert_eq!(atelier.venue.map(|v| v.name["fr"].clone()), Some("Hôtel DoubleTree by Hilton".into()));
    assert!(atelier.agenda.is_empty());
}

#[tokio::test]
async fn les_fils_publies_dune_edition_pour_la_programmation() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let finance = fil(&bac, terrain.edition, "journee-finance", true, None).await;
    let atelier = fil(&bac, terrain.edition, "atelier", true, None).await;
    sqlx::query!("UPDATE event.programme_tracks SET kind = 'side_programme', color_hex = NULL WHERE id = $1", atelier)
        .execute(bac.pool())
        .await
        .unwrap();
    fil(&bac, terrain.edition, "journee-cachee", false, None).await;
    seance(&bac, terrain.edition, finance, "ouverture", "2027-11-13 09:00", "2027-11-13 09:15", true, terrain.organisation).await;
    moment(&bac, finance, None, "Clôture", "2027-11-13 18:00", None).await;
    moment(&bac, finance, None, "Accueil", "2027-11-13 08:00", Some("2027-11-13 08:45")).await;
    moment(&bac, atelier, None, "Atelier", "2027-11-08 09:00", Some("2027-11-08 15:00")).await;

    let fils = track_page::fils_de_ledition(bac.pool(), EventId(terrain.edition))
        .await
        .expect("fils");
    let heure = |jour: &str, h: &str| commun::instant(&format!("2027-11-{jour}T{h}:00-03:00"));
    let resume: Vec<(&str, &str, bool)> = fils
        .iter()
        .map(|t| (t.summary.slug.as_str(), t.summary.kind.as_str(), t.summary.is_current))
        .collect();
    assert_eq!(resume.len(), 2, "le fil non publié est tu : {resume:?}");
    assert!(resume.contains(&("journee-finance", "special_day", false)));
    assert!(resume.contains(&("atelier", "side_programme", false)));
    let journee = fils.iter().find(|t| t.summary.slug == "journee-finance").unwrap();
    assert_eq!(journee.summary.starts_at, Some(heure("13", "08:00")), "l'accueil ouvre la journée");
    assert_eq!(journee.summary.ends_at, Some(heure("13", "18:00")));
    let moments: Vec<&str> = journee.moments.iter().map(|m| m.label["fr"].as_str().unwrap()).collect();
    assert_eq!(moments, ["Accueil", "Clôture"], "dans l'ordre de l'heure, ceux du fil seulement");
    let atelier = fils.iter().find(|t| t.summary.slug == "atelier").unwrap();
    assert_eq!(
        (atelier.summary.starts_at, atelier.summary.ends_at),
        (Some(heure("08", "09:00")), Some(heure("08", "15:00")))
    );
    assert_eq!(atelier.moments.len(), 1);

    sqlx::query!("UPDATE event.events SET status = 'draft' WHERE id = $1", terrain.edition)
        .execute(bac.pool())
        .await
        .unwrap();
    let fils = track_page::fils_de_ledition(bac.pool(), EventId(terrain.edition)).await.unwrap();
    assert!(fils.is_empty(), "édition en brouillon : aucun fil");
}

#[tokio::test]
async fn un_fil_non_publie_une_adresse_inconnue_une_edition_en_brouillon_rendent_404() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    fil(&bac, terrain.edition, "journee-cachee", false, None).await;
    fil(&bac, terrain.edition, "journee-finance", true, None).await;

    for (edition, slug) in [
        (SLUG_COP31, "journee-cachee"),
        (SLUG_COP31, "inconnue"),
        ("edition-inconnue", "journee-finance"),
    ] {
        let refus = track_page::page(bac.pool(), edition, slug).await.expect_err(slug);
        assert_eq!(refus.code, ErrorCode::NotFound, "{edition}/{slug}");
    }

    sqlx::query!("UPDATE event.events SET status = 'draft' WHERE id = $1", terrain.edition)
        .execute(bac.pool())
        .await
        .unwrap();
    let refus = track_page::page(bac.pool(), SLUG_COP31, "journee-finance")
        .await
        .expect_err("édition en brouillon");
    assert_eq!(refus.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn placer_une_activite_dans_une_partie_de_son_fil_et_seulement_la() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let finance = fil(&bac, terrain.edition, "journee-finance", true, None).await;
    let jeunesse = fil(&bac, terrain.edition, "journee-jeunesse", true, None).await;
    let partie_finance = partie(&bac, finance, "Partie 1", 1).await;
    let partie_jeunesse = partie(&bac, jeunesse, "S'inspirer", 1).await;
    let s = seance(&bac, terrain.edition, finance, "panel", "2027-11-13 09:15", "2027-11-13 10:00", true, terrain.organisation).await;

    let place = track_page::placer(
        &bac.state,
        &bac.ctx(),
        SessionId::from(s),
        finance,
        TrackSessionPlacementPayload { part_id: Some(partie_finance), is_highlight: true, sort_order: 3 },
    )
    .await
    .expect("placement");
    assert_eq!(place.part_id, Some(partie_finance));
    assert!(place.is_highlight);

    let refus = track_page::placer(
        &bac.state,
        &bac.ctx(),
        SessionId::from(s),
        finance,
        TrackSessionPlacementPayload { part_id: Some(partie_jeunesse), is_highlight: false, sort_order: 0 },
    )
    .await
    .expect_err("partie d'un autre fil");
    assert_eq!(refus.field.as_deref(), Some("part_id"));

    let refus = track_page::placer(
        &bac.state,
        &bac.ctx(),
        SessionId::from(s),
        jeunesse,
        TrackSessionPlacementPayload { part_id: None, is_highlight: false, sort_order: 0 },
    )
    .await
    .expect_err("séance non rattachée à ce fil");
    assert_eq!(refus.code, ErrorCode::NotFound);
}
