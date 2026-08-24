//! **Le public lit le programme** — sans session, et sans rien voir de ce qui
//! n'est pas publié.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::EventId;
use programme::domain::transitions::ProposalStatus;
use programme::service::{public_schedule, transition};
use uuid::Uuid;

async fn seance(bac: &Bac, terrain: &Terrain, titre: &str, slug: &str) -> Uuid {
    let dossier = seances::dossier_pret(bac, terrain, titre, slug, Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id
}

/// Rendre une séance publique **à la main** : c'est ce qui fait de cette lecture
/// l'instrument de mesure de la publication (US6), et non l'inverse.
async fn publier(bac: &Bac, session_id: Uuid) {
    sqlx::query!(
        "UPDATE programme.sessions
            SET published_at = now(), status = 'scheduled'
          WHERE id = $1",
        session_id
    )
    .execute(bac.pool())
    .await
    .expect("publication posée à la main");
}

/// La lecture répond **sans session**, et ne porte que des séances publiées.
#[tokio::test]
async fn la_programmation_ne_porte_que_le_publie() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;

    let publique = seance(&bac, &terrain, "Publique", "publique").await;
    let interne = seance(&bac, &terrain, "Interne", "interne").await;
    seances::placer(
        &bac,
        terrain.edition,
        publique,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();
    publier(&bac, publique).await;

    // Aucun acteur, aucun périmètre : la lecture ne garde rien.
    let lignes = public_schedule::programmation(bac.pool(), Some(EventId(terrain.edition)), None)
        .await
        .expect("la lecture aboutit sans session");

    assert_eq!(lignes.len(), 1);
    assert_eq!(lignes[0].id, publique);
    assert!(
        lignes.iter().all(|l| l.id != interne),
        "une séance non publiée n'y figure pas"
    );
}

/// Chaque ligne porte **tout ce que la carte affiche**, déjà joint et déjà
/// résolu : chaque colonne manquante coûterait une requête par écran.
#[tokio::test]
async fn chaque_ligne_porte_ce_que_la_carte_affiche() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;

    // Le pays de l'organisation porteuse : il situe l'organisation aussi sûrement
    // que son nom, et la vue le joint pour que l'écran n'ait pas à le chercher.
    sqlx::query!(
        "UPDATE org.organizations
            SET country_id = (SELECT id FROM reference.countries WHERE iso3 = 'BRA')
          WHERE id = $1",
        terrain.organisation
    )
    .execute(bac.pool())
    .await
    .expect("pays de l'organisation");

    let id = seance(&bac, &terrain, "Atelier mangroves", "atelier-mangroves").await;
    seances::placer(
        &bac,
        terrain.edition,
        id,
        Some(grille.salle),
        "14:00",
        "15:30",
    )
    .await
    .unwrap();
    seances::rattacher(&bac, terrain.edition, id, vec![grille.fil], None)
        .await
        .unwrap();
    publier(&bac, id).await;

    let ligne = public_schedule::programmation(bac.pool(), Some(EventId(terrain.edition)), None)
        .await
        .unwrap()
        .remove(0);

    assert_eq!(ligne.room_id, Some(grille.salle));
    assert!(ligne.room_name.is_some(), "la salle, déjà jointe");
    assert_eq!(
        ligne.organization_name.as_deref(),
        Some("Institut de la Francophonie")
    );
    assert_eq!(ligne.organization_acronym.as_deref(), Some("IFDD"));
    assert_eq!(
        ligne.organization_country_code.as_deref(),
        Some("BR"),
        "le code ISO situe l'organisation aussi sûrement que son nom"
    );
    assert_eq!(
        ligne.tracks.as_array().map(Vec::len),
        Some(1),
        "les journées spéciales, agrégées"
    );
    assert_eq!(ligne.theme_codes.len(), 2, "les codes, pour filtrer");
    assert_eq!(
        ligne.themes.as_array().map(Vec::len),
        Some(2),
        "et les pastilles, pour afficher"
    );
    assert_eq!(
        ligne.temporal_state, "upcoming",
        "l'état temporel est calculé en base, une fois"
    );
    assert_eq!(ligne.registered_count, 0);
}

/// **Le repli de couverture est la règle, pas une commodité** : une organisation
/// joint son image au dépôt, et personne ne revient en téléverser une seconde
/// après l'acceptation.
#[tokio::test]
async fn la_couverture_se_replie_sur_celle_du_dossier() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Avec image",
        "avec-image",
        Souhaits::default(),
    )
    .await;

    // Une image rattachée **au dossier**, et à lui seul.
    let asset = sqlx::query_scalar!(
        r#"INSERT INTO media.assets
               (object_key, original_filename, mime_type, byte_size, checksum_sha256,
                status, alt_text, owner_organization_id, scan_verdict, scanned_at)
           VALUES ('couvertures/mangroves.jpg', 'mangroves.jpg', 'image/jpeg', 120000,
                   repeat('a', 64), 'ready',
                   '{"fr":"Mangrove à marée basse"}'::jsonb, $1, 'clean', now())
        RETURNING id"#,
        terrain.organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("dépôt de l'objet");

    sqlx::query!(
        "INSERT INTO media.attachments
             (asset_id, owner_schema, owner_table, owner_id, role)
         VALUES ($1, 'programme', 'proposals', $2, 'cover')",
        asset,
        dossier.id
    )
    .execute(bac.pool())
    .await
    .expect("rattachement au dossier");

    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    let id = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0)
        .id;
    publier(&bac, id).await;

    let ligne = public_schedule::programmation(bac.pool(), Some(EventId(terrain.edition)), None)
        .await
        .unwrap()
        .remove(0);

    assert!(
        ligne.cover.is_some(),
        "la séance n'a pas d'image, celle du dossier prend le relais"
    );
}

/// **Une adresse inconnue et une séance non publiée rendent le même refus** :
/// distinguer les deux dirait au public qu'une séance existe sans être encore
/// annoncée.
#[tokio::test]
async fn ladresse_inconnue_et_la_seance_non_publiee_rendent_le_meme_refus() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let interne = seance(&bac, &terrain, "Pas encore", "pas-encore").await;
    let adresse = seances::seance(&bac, interne).await.slug;

    let refus_non_publiee = public_schedule::seance(bac.pool(), EventId(terrain.edition), &adresse)
        .await
        .expect_err("une séance non publiée reste invisible");
    let refus_inconnue =
        public_schedule::seance(bac.pool(), EventId(terrain.edition), "nexiste-pas")
            .await
            .expect_err("une adresse inconnue aussi");

    assert_eq!(refus_non_publiee.code, ErrorCode::NotFound);
    assert_eq!(refus_inconnue.code, ErrorCode::NotFound);
    assert_eq!(refus_non_publiee.message, refus_inconnue.message);
}

/// Une édition dont le programme n'est pas publié rend une réponse **vide**,
/// jamais une erreur : c'est un état normal, que l'écran annonce.
#[tokio::test]
async fn une_edition_non_publiee_rend_une_liste_vide() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    seance(&bac, &terrain, "Interne", "interne").await;

    let lignes = public_schedule::programmation(bac.pool(), Some(EventId(terrain.edition)), None)
        .await
        .expect("aucune erreur");

    assert!(lignes.is_empty());
}

/// Le détail d'une séance publiée porte ses intervenants et ses organisations.
#[tokio::test]
async fn le_detail_dune_seance_publiee_porte_ses_participants() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let id = seance(&bac, &terrain, "Atelier", "atelier").await;
    publier(&bac, id).await;
    let adresse = seances::seance(&bac, id).await.slug;

    let detail = public_schedule::seance(bac.pool(), EventId(terrain.edition), &adresse)
        .await
        .expect("la séance publiée est lisible");

    assert_eq!(detail.session.id, id);
    assert_eq!(detail.speakers.len(), 2);
    assert_eq!(detail.organizations.len(), 2, "le porteur et sa partenaire");
}

/// **Sans édition, la lecture traverse toutes les éditions** — c'est ce que
/// compose l'accueil du site, qui n'a pas d'édition à nommer.
///
/// Et elle est BORNÉE dans les deux sens : les séances passées n'y figurent
/// pas, et le plafond est respecté. Sans ces deux bornes, la page d'accueil
/// rendrait la programmation entière de toutes les COP de l'histoire de la
/// plateforme à chaque affichage.
#[tokio::test]
async fn sans_edition_la_lecture_rend_les_seances_a_venir_de_toutes_les_editions() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let premiere = seance(&bac, &terrain, "Ouverture", "ouverture").await;
    let seconde = seance(&bac, &terrain, "Clôture", "cloture").await;
    publier(&bac, premiere).await;
    publier(&bac, seconde).await;

    let toutes = public_schedule::programmation(bac.pool(), None, Some(50))
        .await
        .expect("la lecture aboutit sans édition et sans session");

    assert!(
        toutes.iter().any(|l| l.id == premiere),
        "une séance publiée d'une édition à venir figure dans la liste sans édition"
    );
    assert!(
        toutes.iter().all(|l| l.temporal_state != "past"),
        "aucune séance passée : l'accueil annonce ce qui vient, pas ce qui a eu lieu"
    );

    let plafonnee = public_schedule::programmation(bac.pool(), None, Some(1))
        .await
        .expect("la lecture aboutit");
    assert_eq!(plafonnee.len(), 1, "le plafond est respecté");
}
