//! **Les jours civils d'une édition se calculent dans SON fuseau** — le défaut
//! le plus discret du module (research.md § R5).
//!
//! Belém est trois heures derrière l'UTC. Une édition qui se termine le
//! 20 novembre à 22 h locale se termine le **21** en temps universel : un
//! calendrier calculé en UTC lui donnerait treize journées au lieu de douze, et
//! personne ne s'en apercevrait avant que le planificateur n'affiche un jour
//! vide de trop.
//!
//! Le calcul est donc fait **en base**, par `generate_series` sur
//! `(starts_at AT TIME ZONE timezone)::date`. Le faire en Rust demanderait une
//! base de fuseaux qui n'est pas celle de PostgreSQL — or c'est celle de
//! PostgreSQL que `platform.timezone_name` a déjà utilisée pour accepter
//! l'identifiant. Le même dictionnaire vérifie et convertit.

mod commun;

use commun::{auteur, formulaire, instant, journees, serie, Bac};
use event::service::edition_write;
use time::macros::date;

/// L'édition de référence du jalon : Belém, du 9 au 20 novembre 2027.
///
/// Les instants sont choisis pour que **le calcul en UTC donne une autre
/// réponse** : la fin tombe le 21 en temps universel. Un test dont les bornes
/// coïncideraient dans les deux fuseaux ne prouverait rien.
#[tokio::test]
async fn une_edition_a_belem_porte_douze_journees_du_9_au_20() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;
    let climat = serie(&bac, "cop_climate").await;
    let bresil = commun::pays(&bac, "BRA").await;

    let mut p = formulaire("cop31-belem", "COP31 — Conférence des Parties");
    p.series_id = Some(climat);
    p.edition_label = Some("COP31".to_owned());
    p.has_pavilion = true;
    p.acronym = Some("COP31".to_owned());
    p.participation_mode = "hybrid".to_owned();
    p.country_id = Some(bresil);
    p.city = Some("Belém".to_owned());
    p.timezone = "America/Belem".to_owned();
    // 9 novembre, 9 h à Belém.
    p.starts_at = instant("2027-11-09T12:00:00Z");
    // 20 novembre, 22 h à Belém — soit le **21** en temps universel.
    p.ends_at = instant("2027-11-21T01:00:00Z");

    let r = edition_write::creer(&bac.state, &bac.ctx(), acteur, p)
        .await
        .expect("l'écriture aboutit");
    assert!(r.ok, "{:?}", r.errors);

    let edition = r.edition.expect("l'édition");
    assert_eq!(r.days_created, 12, "douze journées annoncées");
    assert_eq!(edition.day_count, 12, "et douze en base");

    let jours = journees(&bac, edition.id).await;
    assert_eq!(
        jours.first().copied(),
        Some(date!(2027 - 11 - 09)),
        "si la première tombe le 8 ou le 10, le fuseau n'a pas été appliqué"
    );
    assert_eq!(
        jours.last().copied(),
        Some(date!(2027 - 11 - 20)),
        "si la dernière tombe le 21, la période a été lue en temps universel"
    );
    assert_eq!(jours.len(), 12);
}

/// L'autre bout du même décalage : une période qui **commence** la veille en
/// heure locale. Calculée en UTC, elle ne porterait qu'une journée.
#[tokio::test]
async fn une_periode_qui_commence_la_veille_en_heure_locale_porte_ses_deux_journees() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let mut p = formulaire("veille-a-belem", "Édition à cheval");
    p.timezone = "America/Belem".to_owned();
    // 8 novembre, 22 h à Belém.
    p.starts_at = instant("2027-11-09T01:00:00Z");
    // 9 novembre, 20 h à Belém.
    p.ends_at = instant("2027-11-09T23:00:00Z");

    let r = edition_write::creer(&bac.state, &bac.ctx(), acteur, p)
        .await
        .expect("l'écriture aboutit");
    assert!(r.ok, "{:?}", r.errors);

    let jours = journees(&bac, r.edition.expect("l'édition").id).await;
    assert_eq!(
        jours,
        vec![date!(2027 - 11 - 08), date!(2027 - 11 - 09)],
        "en temps universel, cette période ne compterait qu'un seul jour"
    );
}

/// **Une journée générée porte sa date, son rang, et rien d'autre** (FR-037).
/// Inventer « Jour 3 » produirait un titre que personne n'a écrit et qui
/// s'afficherait tel quel sur la page publique.
#[tokio::test]
async fn une_journee_generee_ne_porte_aucun_contenu_editorial() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let mut p = formulaire("journees-nues", "Édition à journées nues");
    p.timezone = "America/Belem".to_owned();
    p.starts_at = instant("2027-11-09T12:00:00Z");
    p.ends_at = instant("2027-11-11T21:00:00Z");

    let r = edition_write::creer(&bac.state, &bac.ctx(), acteur, p)
        .await
        .expect("l'écriture aboutit");
    let id = r.edition.expect("l'édition").id;

    let lignes = sqlx::query!(
        "SELECT title, slug::text AS slug, description, is_featured, color_hex, sort_order
           FROM event.event_days WHERE event_id = $1 ORDER BY day_date",
        id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des journées");

    assert_eq!(lignes.len(), 3);
    for (rang, l) in lignes.iter().enumerate() {
        assert!(l.title.is_none(), "aucun titre inventé");
        assert!(l.slug.is_none());
        assert!(l.description.is_none());
        assert!(!l.is_featured);
        assert!(l.color_hex.is_none());
        assert_eq!(l.sort_order, rang as i16, "le rang suit la période");
    }
}

/// **Un enregistrement d'édition crée les journées manquantes et n'en supprime
/// aucune** (FR-033). Une période resserrée laisse donc les journées hors bornes
/// en place : leur retrait est un geste séparé, et explicite.
#[tokio::test]
async fn un_enregistrement_najoute_que_les_journees_manquantes() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let mut p = formulaire("periode-mouvante", "Édition à période mouvante");
    p.timezone = "America/Belem".to_owned();
    p.starts_at = instant("2027-11-09T12:00:00Z");
    p.ends_at = instant("2027-11-11T21:00:00Z");

    let cree = edition_write::creer(&bac.state, &bac.ctx(), acteur, p.clone())
        .await
        .expect("création");
    let id = event::domain::ids::EventId::from(cree.edition.expect("l'édition").id);
    assert_eq!(cree.days_created, 3);

    // Période élargie de deux jours : deux journées de plus, et pas une de
    // recréée.
    p.ends_at = instant("2027-11-13T21:00:00Z");
    let elargie = edition_write::modifier(&bac.state, &bac.ctx(), acteur, id, p.clone())
        .await
        .expect("modification");
    assert_eq!(elargie.days_created, 2);
    assert_eq!(journees(&bac, id.as_uuid()).await.len(), 5);

    // Période resserrée : rien n'est créé, et **rien n'est supprimé**.
    p.ends_at = instant("2027-11-10T21:00:00Z");
    let resserree = edition_write::modifier(&bac.state, &bac.ctx(), acteur, id, p)
        .await
        .expect("modification");
    assert_eq!(resserree.days_created, 0);
    assert_eq!(resserree.days_removed, 0, "toujours zéro ici (FR-033)");
    assert_eq!(resserree.sessions_detached, 0);
    assert_eq!(
        journees(&bac, id.as_uuid()).await.len(),
        5,
        "les journées hors période restent : leur retrait est un geste explicite"
    );
}

/// **Le cycle de webinaires** : une édition d'un an annonce plus de trois cents
/// journées. Le nombre est dit avant d'écrire, et l'arbitrage reste ouvert.
#[tokio::test]
async fn une_edition_dun_an_cree_ses_trois_cent_soixante_cinq_journees() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;
    let webinaires = serie(&bac, "ifdd_webinars").await;

    let mut p = formulaire("cycle-2027", "Cycle de webinaires 2027");
    p.series_id = Some(webinaires);
    p.edition_label = Some("Cycle 2027".to_owned());
    p.timezone = "Africa/Dakar".to_owned();
    p.starts_at = instant("2027-01-01T09:00:00Z");
    p.ends_at = instant("2027-12-31T17:00:00Z");

    let r = edition_write::creer(&bac.state, &bac.ctx(), acteur, p)
        .await
        .expect("l'écriture aboutit");

    assert_eq!(r.days_created, 365);
    assert!(
        r.days_created > 300,
        "c'est ce volume qui rend l'arbitrage nécessaire"
    );
}
