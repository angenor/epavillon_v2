//! Le jour J d'une activité retenue : point focal, besoins techniques,
//! publications, présentateur d'un support. Chaque garde d'accès est éprouvée.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::day_plan::SocialPlatform;
use programme::domain::ids::ProposalId;
use programme::domain::transitions::ProposalStatus;
use programme::service::day_plan::{
    self, AddSocialPostPayload, FocalPointInput, SaveFocalPointsPayload, SaveTechnicalNeedsPayload,
};
use programme::service::documents::{self, AttachDocumentPayload};
use programme::service::transition;
use uuid::Uuid;

/// Un dossier retenu, ses séances nées, et la ligne d'un de ses intervenants.
struct Retenu {
    id: ProposalId,
    intervenants: Vec<Uuid>,
}

async fn retenu(bac: &Bac, terrain: &Terrain, slug: &str) -> Retenu {
    let dossier =
        seances::dossier_pret(bac, terrain, "Atelier mangroves", slug, Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .expect("l'acceptation aboutit");
    let intervenants = lignes_dintervenant(bac, dossier.id).await;
    Retenu {
        id: ProposalId(dossier.id),
        intervenants,
    }
}

async fn lignes_dintervenant(bac: &Bac, dossier: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar!(
        "SELECT id FROM programme.proposal_speakers WHERE proposal_id = $1 ORDER BY created_at, id",
        dossier
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des intervenants")
}

fn manuel(nom: &str, tel: &str) -> FocalPointInput {
    FocalPointInput {
        speaker_id: None,
        person_id: None,
        full_name: nom.to_owned(),
        phone: tel.to_owned(),
        whatsapp: String::new(),
    }
}

fn intervenant(speaker_id: Uuid) -> FocalPointInput {
    FocalPointInput {
        speaker_id: Some(speaker_id),
        person_id: None,
        full_name: String::new(),
        phone: String::new(),
        whatsapp: "+55 91 98888 7777".to_owned(),
    }
}

fn liste(points: Vec<FocalPointInput>) -> SaveFocalPointsPayload {
    SaveFocalPointsPayload {
        focal_points: points,
    }
}

fn aucun_besoin() -> SaveTechnicalNeedsPayload {
    SaveTechnicalNeedsPayload {
        video: false,
        remote: false,
        remote_details: String::new(),
        accessibility: false,
        accessibility_details: String::new(),
        other: String::new(),
    }
}

async fn terminer_ledition(bac: &Bac, edition: Uuid) {
    sqlx::query!(
        "UPDATE event.events
            SET starts_at = now() - interval '10 days', ends_at = now() - interval '1 day'
          WHERE id = $1",
        edition
    )
    .execute(bac.pool())
    .await
    .expect("édition passée");
}

#[tokio::test]
async fn le_deposant_est_propose_et_la_date_limite_suit_la_premiere_seance() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    sqlx::query!(
        "UPDATE identity.people SET phone = '+221 77 000 00 00' WHERE id = $1",
        terrain.deposante
    )
    .execute(bac.pool())
    .await
    .expect("téléphone");
    let dossier = retenu(&bac, &terrain, "jour-j-defaut").await;

    let plan = day_plan::lire(&bac.state, terrain.deposante, dossier.id)
        .await
        .expect("lecture");

    assert_eq!(plan.focal_points.len(), 1);
    let defaut = &plan.focal_points[0];
    assert!(defaut.is_default && defaut.id.is_none());
    assert_eq!(defaut.person_id, Some(terrain.deposante));
    assert_eq!(defaut.full_name, "Aïcha Diallo");
    assert_eq!(defaut.phone, "+221 77 000 00 00");
    assert_eq!(defaut.whatsapp, "");
    assert!(plan.technical_needs.updated_at.is_none());
    assert!(plan.social_posts.is_empty());

    let premiere = sqlx::query_scalar!(
        r#"SELECT min(starts_at) AS "debut!" FROM programme.sessions WHERE proposal_id = $1"#,
        dossier.id.as_uuid()
    )
    .fetch_one(bac.pool())
    .await
    .expect("première séance");
    assert_eq!(
        plan.needs_deadline,
        Some(premiere - time::Duration::days(10))
    );
}

#[tokio::test]
async fn sans_seance_la_date_limite_est_nulle() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = commun::dossier(&bac, &terrain, "Brouillon", "brouillon-jour-j").await;

    let plan = day_plan::lire(&bac.state, terrain.deposante, ProposalId(dossier))
        .await
        .expect("lecture");
    assert!(plan.needs_deadline.is_none());
}

#[tokio::test]
async fn la_liste_des_points_focaux_se_remplace_et_ne_se_vide_pas() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let dossier = retenu(&bac, &terrain, "jour-j-points").await;
    let speaker = dossier.intervenants[0];

    let plan = day_plan::enregistrer_points_focaux(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        liste(vec![
            intervenant(speaker),
            manuel("  Paulo Silva ", "+55 91 0000"),
        ]),
    )
    .await
    .expect("enregistrement");

    assert_eq!(plan.focal_points.len(), 2, "le déposant est retiré");
    let premier = &plan.focal_points[0];
    assert!(!premier.is_default && premier.id.is_some());
    assert_eq!(premier.speaker_id, Some(speaker));
    assert!(premier.person_id.is_some(), "la personne vient de la ligne");
    assert_eq!(premier.full_name, "Moussa Ba", "nom repris de la fiche");
    assert_eq!(premier.whatsapp, "+55 91 98888 7777");
    assert_eq!(plan.focal_points[1].full_name, "Paulo Silva");
    assert!(plan.focal_points[1].person_id.is_none());

    let vide = day_plan::enregistrer_points_focaux(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        liste(vec![]),
    )
    .await
    .expect_err("une liste vide est refusée");
    assert_eq!(vide.code, ErrorCode::ValidationFailed);
    assert_eq!(
        vide.message,
        "Gardez au moins une personne à joindre sur place le jour de l'activité."
    );

    let sans_nom = day_plan::enregistrer_points_focaux(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        liste(vec![manuel("  ", "")]),
    )
    .await
    .expect_err("un nom est exigé");
    assert_eq!(sans_nom.field.as_deref(), Some("focal_points[0].full_name"));

    let doublon = day_plan::enregistrer_points_focaux(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        liste(vec![intervenant(speaker), intervenant(speaker)]),
    )
    .await
    .expect_err("une même personne deux fois est refusée");
    assert_eq!(doublon.code, ErrorCode::ValidationFailed);

    // Le refus n'a rien touché : la liste précédente demeure.
    let relu = day_plan::lire(&bac.state, terrain.deposante, dossier.id)
        .await
        .expect("lecture");
    assert_eq!(relu.focal_points.len(), 2);

    // Un intervenant retiré du dossier n'est plus point focal.
    sqlx::query!(
        "DELETE FROM programme.proposal_speakers WHERE id = $1",
        speaker
    )
    .execute(bac.pool())
    .await
    .expect("retrait de l'intervenant");
    let apres = day_plan::lire(&bac.state, terrain.deposante, dossier.id)
        .await
        .expect("lecture");
    assert_eq!(apres.focal_points.len(), 1);
    assert_eq!(apres.focal_points[0].full_name, "Paulo Silva");
}

#[tokio::test]
async fn un_intervenant_ou_une_personne_etrangers_au_dossier_sont_refuses() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let dossier = retenu(&bac, &terrain, "jour-j-a").await;
    let autre = retenu(&bac, &terrain, "jour-j-b").await;

    let refus = day_plan::enregistrer_points_focaux(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        liste(vec![intervenant(autre.intervenants[0])]),
    )
    .await
    .expect_err("intervenant d'un autre dossier");
    assert_eq!(refus.code, ErrorCode::ProposalUnknownReference);
    assert_eq!(refus.field.as_deref(), Some("focal_points[0].speaker_id"));
    assert_eq!(
        refus.message,
        "Cet intervenant ne figure pas dans ce dossier."
    );

    let inconnue = commun::personne(&bac, "inconnue@example.org", "Ina", "Connue").await;
    let refus = day_plan::enregistrer_points_focaux(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        liste(vec![FocalPointInput {
            person_id: Some(inconnue),
            ..manuel("", "")
        }]),
    )
    .await
    .expect_err("personne étrangère au dossier");
    assert_eq!(refus.code, ErrorCode::ProposalUnknownReference);
    assert_eq!(refus.field.as_deref(), Some("focal_points[0].person_id"));

    // Le déposant, désigné par sa personne, passe.
    let plan = day_plan::enregistrer_points_focaux(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        liste(vec![FocalPointInput {
            person_id: Some(terrain.deposante),
            ..manuel("", "+221 1")
        }]),
    )
    .await
    .expect("le déposant passe");
    assert_eq!(plan.focal_points[0].full_name, "Aïcha Diallo");
    assert!(!plan.focal_points[0].is_default);
}

#[tokio::test]
async fn qui_peut_ecrire_et_sur_quel_dossier() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = retenu(&bac, &terrain, "jour-j-droits").await;

    // L'équipe qui décide sur l'édition écrit.
    day_plan::enregistrer_besoins(
        &bac.state,
        &bac.ctx(),
        droits.decideur,
        dossier.id,
        aucun_besoin(),
    )
    .await
    .expect("l'équipe qui décide écrit");

    // Un membre du comité lit la fiche mais n'écrit pas : même refus qu'un
    // dossier inexistant.
    let refus = day_plan::enregistrer_besoins(
        &bac.state,
        &bac.ctx(),
        droits.noteur,
        dossier.id,
        aucun_besoin(),
    )
    .await
    .expect_err("le comité n'écrit pas");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let etrangere = commun::personne(&bac, "etrangere@example.org", "Eva", "Tranger").await;
    let refus = day_plan::lire(&bac.state, etrangere, dossier.id)
        .await
        .expect_err("une étrangère ne lit pas");
    assert_eq!(refus.code, ErrorCode::NotFound);
    let refus = day_plan::ajouter_publication(
        &bac.state,
        &bac.ctx(),
        etrangere,
        dossier.id,
        AddSocialPostPayload {
            url: "https://x.com/a/status/1".to_owned(),
        },
    )
    .await
    .expect_err("une étrangère n'écrit pas");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // Un dossier non retenu ne se prépare pas.
    let depose = seances::dossier_pret(
        &bac,
        &terrain,
        "En évaluation",
        "jour-j-eval",
        Souhaits::default(),
    )
    .await;
    let refus = day_plan::enregistrer_besoins(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(depose.id),
        aucun_besoin(),
    )
    .await
    .expect_err("dossier non retenu");
    assert_eq!(refus.code, ErrorCode::ProposalNotEditable);
    assert_eq!(
        refus.message,
        "Seule une activité retenue se prépare ici : ce dossier n'est pas retenu."
    );
}

#[tokio::test]
async fn la_deposante_en_attente_prepare_son_propre_dossier() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let attente = commun::personne(&bac, "attente@example.org", "Awa", "Attente").await;
    commun::adherer(&bac, terrain.organisation, attente, "pending").await;
    let sien = Terrain {
        edition: terrain.edition,
        appel: terrain.appel,
        organisation: terrain.organisation,
        deposante: attente,
    };
    let dossier = retenu(&bac, &sien, "jour-j-attente").await;
    let collegue = retenu(&bac, &terrain, "jour-j-collegue").await;

    day_plan::ajouter_publication(
        &bac.state,
        &bac.ctx(),
        attente,
        dossier.id,
        AddSocialPostPayload {
            url: "https://www.linkedin.com/posts/a".to_owned(),
        },
    )
    .await
    .expect("son propre dossier");

    let refus = day_plan::ajouter_publication(
        &bac.state,
        &bac.ctx(),
        attente,
        collegue.id,
        AddSocialPostPayload {
            url: "https://www.linkedin.com/posts/a".to_owned(),
        },
    )
    .await
    .expect_err("pas celui d'une collègue");
    assert_eq!(refus.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn apres_ledition_seules_les_publications_et_les_supports_se_modifient() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let dossier = retenu(&bac, &terrain, "jour-j-apres").await;
    terminer_ledition(&bac, terrain.edition).await;

    let refus = day_plan::enregistrer_points_focaux(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        liste(vec![manuel("Paulo Silva", "")]),
    )
    .await
    .expect_err("point focal fermé");
    assert_eq!(refus.code, ErrorCode::ProposalNotEditable);
    assert_eq!(
        refus.message,
        "Cette édition est terminée : le point focal et les besoins techniques ne se modifient plus."
    );
    let refus = day_plan::enregistrer_besoins(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        aucun_besoin(),
    )
    .await
    .expect_err("besoins fermés");
    assert_eq!(refus.code, ErrorCode::ProposalNotEditable);

    let plan = day_plan::ajouter_publication(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        AddSocialPostPayload {
            url: "https://youtu.be/bilan".to_owned(),
        },
    )
    .await
    .expect("le bilan se fait après");
    assert_eq!(plan.social_posts[0].platform, SocialPlatform::Youtube);
}

#[tokio::test]
async fn les_besoins_se_declarent_et_le_detail_suit_sa_case() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let dossier = retenu(&bac, &terrain, "jour-j-besoins").await;

    let plan = day_plan::enregistrer_besoins(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        SaveTechnicalNeedsPayload {
            video: true,
            remote: true,
            remote_details: " Une intervenante depuis Dakar ".to_owned(),
            accessibility: false,
            accessibility_details: "Texte resté derrière".to_owned(),
            other: "Micro sans fil".to_owned(),
        },
    )
    .await
    .expect("déclaration");

    let besoins = &plan.technical_needs;
    assert!(besoins.updated_at.is_some(), "déclaré");
    assert!(besoins.video && besoins.remote && !besoins.accessibility);
    assert_eq!(besoins.remote_details, "Une intervenante depuis Dakar");
    assert_eq!(
        besoins.accessibility_details, "",
        "case décochée, détail effacé"
    );
    assert_eq!(besoins.other, "Micro sans fil");

    // « Aucun besoin » est une réponse : la ligne reste, datée.
    let plan = day_plan::enregistrer_besoins(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        aucun_besoin(),
    )
    .await
    .expect("aucun besoin");
    assert!(plan.technical_needs.updated_at.is_some());
    assert!(!plan.technical_needs.video);
}

#[tokio::test]
async fn les_publications_sajoutent_se_reconnaissent_et_se_retirent() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let dossier = retenu(&bac, &terrain, "jour-j-reseaux").await;
    let ajouter = |url: &str| AddSocialPostPayload {
        url: url.to_owned(),
    };

    let plan = day_plan::ajouter_publication(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        ajouter(" https://www.linkedin.com/posts/ifdd_cop31-1 "),
    )
    .await
    .expect("ajout");
    assert_eq!(plan.social_posts.len(), 1);
    assert_eq!(plan.social_posts[0].platform, SocialPlatform::Linkedin);
    assert_eq!(
        plan.social_posts[0].url,
        "https://www.linkedin.com/posts/ifdd_cop31-1"
    );

    let doublon = day_plan::ajouter_publication(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        ajouter("https://www.linkedin.com/posts/ifdd_cop31-1"),
    )
    .await
    .expect_err("doublon");
    assert_eq!(doublon.code, ErrorCode::Conflict);
    assert_eq!(
        doublon.message,
        "Ce lien figure déjà parmi les publications de l'activité."
    );

    for invalide in [
        "linkedin.com/posts/1",
        "ftp://exemple.org/a",
        "https://exemple",
    ] {
        let refus = day_plan::ajouter_publication(
            &bac.state,
            &bac.ctx(),
            terrain.deposante,
            dossier.id,
            ajouter(invalide),
        )
        .await
        .expect_err("adresse invalide");
        assert_eq!(refus.code, ErrorCode::ValidationFailed, "{invalide}");
        assert_eq!(refus.field.as_deref(), Some("url"));
        assert_eq!(
            refus.message,
            "Collez l'adresse complète de la publication, commençant par https://."
        );
    }

    let post = plan.social_posts[0].id;
    let plan =
        day_plan::retirer_publication(&bac.state, &bac.ctx(), terrain.deposante, dossier.id, post)
            .await
            .expect("retrait");
    assert!(plan.social_posts.is_empty());

    let refus =
        day_plan::retirer_publication(&bac.state, &bac.ctx(), terrain.deposante, dossier.id, post)
            .await
            .expect_err("déjà retirée");
    assert_eq!(refus.code, ErrorCode::NotFound);
}

/// Un support présenté par un intervenant du dossier ; celui d'un autre dossier
/// est refusé, et retirer l'intervenant laisse la pièce sans présentateur.
#[tokio::test]
async fn une_presentation_porte_son_presentateur() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let dossier = retenu(&bac, &terrain, "jour-j-support").await;
    let autre = retenu(&bac, &terrain, "jour-j-support-b").await;
    let asset = sqlx::query_scalar!(
        r#"INSERT INTO media.assets
               (bucket, object_key, checksum_sha256, mime_type, byte_size,
                original_filename, owner_organization_id, status, scan_verdict)
           VALUES ('epavillon', '2027/03/' || gen_random_uuid()::text || '/support.pptx',
                   md5(gen_random_uuid()::text) || md5(gen_random_uuid()::text),
                   'application/vnd.openxmlformats-officedocument.presentationml.presentation',
                   4096, 'support.pptx', $1, 'ready', 'clean')
        RETURNING id"#,
        terrain.organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("objet stocké");
    let payload = |presentateur: Uuid| AttachDocumentPayload {
        asset_id: asset,
        title: String::new(),
        document_type_code: Some("presentation".to_owned()),
        is_public: false,
        sort_order: 0,
        presented_by_speaker_id: Some(presentateur),
    };

    let refus = documents::rattacher(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        payload(autre.intervenants[0]),
    )
    .await
    .expect_err("présentateur d'un autre dossier");
    assert_eq!(refus.code, ErrorCode::ProposalUnknownReference);
    assert_eq!(refus.field.as_deref(), Some("presented_by_speaker_id"));

    let speaker = dossier.intervenants[0];
    let piece = documents::rattacher(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier.id,
        payload(speaker),
    )
    .await
    .expect("rattachement");
    assert_eq!(piece.presented_by_speaker_id, Some(speaker));

    sqlx::query!(
        "DELETE FROM programme.proposal_speakers WHERE id = $1",
        speaker
    )
    .execute(bac.pool())
    .await
    .expect("retrait de l'intervenant");
    let pieces = documents::lister(&bac.state, terrain.deposante, dossier.id)
        .await
        .expect("lecture");
    assert_eq!(pieces.len(), 1, "la pièce demeure");
    assert!(pieces[0].document.presented_by_speaker_id.is_none());
}
