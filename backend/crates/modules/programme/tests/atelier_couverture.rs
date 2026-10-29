//! L'atelier de l'image de couverture : la lecture en une fois, les choix qui
//! se gardent, les gardes d'accès et d'état.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::cover_design::CoverDesignOptions;
use programme::domain::ids::ProposalId;
use programme::domain::transitions::ProposalStatus;
use programme::service::cover_design::{self, SaveCoverDesignPayload};
use programme::service::transition;
use serde_json::json;
use uuid::Uuid;

fn choix(template_code: &str, accent: &str, options: CoverDesignOptions) -> SaveCoverDesignPayload {
    SaveCoverDesignPayload {
        template_code: template_code.to_owned(),
        accent: accent.to_owned(),
        options,
    }
}

fn par_defaut() -> SaveCoverDesignPayload {
    choix("bandeau", "cyan", CoverDesignOptions::default())
}

/// Une image prête, rattachée à une entité sous un rôle, posée en base.
async fn image(bac: &Bac, terrain: &Terrain, schema: &str, table: &str, id: Uuid, role: &str) {
    let asset = sqlx::query_scalar!(
        r#"INSERT INTO media.assets
               (object_key, original_filename, mime_type, byte_size, checksum_sha256,
                status, alt_text, owner_organization_id, scan_verdict, scanned_at)
           VALUES ('atelier/' || gen_random_uuid()::text || '.jpg', 'image.jpg', 'image/jpeg', 120000,
                   md5(gen_random_uuid()::text) || md5(gen_random_uuid()::text), 'ready',
                   '{"fr":"Une image"}'::jsonb, $1, 'clean', now())
        RETURNING id"#,
        terrain.organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("dépôt de l'objet");

    sqlx::query!(
        "INSERT INTO media.attachments (asset_id, owner_schema, owner_table, owner_id, role)
         VALUES ($1, $2, $3, $4, $5::text::media.attachment_role)",
        asset,
        schema,
        table,
        id,
        role
    )
    .execute(bac.pool())
    .await
    .expect("rattachement");
}

#[tokio::test]
async fn la_lecture_est_vide_puis_rend_les_choix_et_les_images() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = ProposalId(commun::dossier(&bac, &terrain, "Mangroves", "atelier-vide").await);

    let vide = cover_design::lire(&bac.state, terrain.deposante, dossier)
        .await
        .expect("lecture");
    assert!(vide.design.is_none());
    assert!(vide.source_photo.is_none());
    assert!(vide.edition_banner.is_none());
    assert!(vide.edition_thumbnail.is_none());
    assert!(vide.organization_logo.is_none());

    let id = dossier.as_uuid();
    image(&bac, &terrain, "programme", "proposals", id, "cover_source").await;
    image(&bac, &terrain, "event", "events", terrain.edition, "banner").await;
    image(
        &bac,
        &terrain,
        "event",
        "events",
        terrain.edition,
        "thumbnail",
    )
    .await;
    image(
        &bac,
        &terrain,
        "org",
        "organizations",
        terrain.organisation,
        "logo",
    )
    .await;

    let options = CoverDesignOptions {
        show_date: true,
        show_speakers: true,
        ..CoverDesignOptions::default()
    };
    let lu = cover_design::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier,
        choix("  affiche-2 ", "vert", options),
    )
    .await
    .expect("enregistrement");

    let design = lu.design.expect("les choix sont gardés");
    assert_eq!(design.template_code, "affiche-2", "code nettoyé");
    assert_eq!(design.accent, "vert");
    assert_eq!(design.options, options);
    for image in [
        &lu.source_photo,
        &lu.edition_banner,
        &lu.edition_thumbnail,
        &lu.organization_logo,
    ] {
        let image = image.as_ref().expect("image servie");
        assert!(image.get("url").is_some() && image.get("asset_id").is_some());
    }

    // Un second enregistrement remplace, il n'ajoute pas.
    let relu = cover_design::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier,
        choix("bandeau", "jaune", CoverDesignOptions::default()),
    )
    .await
    .expect("modification");
    let design = relu.design.expect("choix");
    assert_eq!(
        (design.template_code.as_str(), design.accent.as_str()),
        ("bandeau", "jaune")
    );
    assert_eq!(design.options, CoverDesignOptions::default());
    let lignes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.proposal_cover_designs WHERE proposal_id = $1"#,
        id
    )
    .fetch_one(bac.pool())
    .await
    .expect("compte");
    assert_eq!(lignes, 1);
}

#[tokio::test]
async fn des_options_partielles_sont_tolerees_et_ecrites_completes() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = ProposalId(commun::dossier(&bac, &terrain, "Mangroves", "atelier-partiel").await);

    // Une ligne écrite avant l'ajout d'une option, ou à la main.
    sqlx::query!(
        r#"INSERT INTO programme.proposal_cover_designs (proposal_id, template_code, accent, options)
           VALUES ($1, 'bandeau', 'cyan', '{"show_logos": true, "inconnue": 3}'::jsonb)"#,
        dossier.as_uuid()
    )
    .execute(bac.pool())
    .await
    .expect("ligne partielle");
    let lu = cover_design::lire(&bac.state, terrain.deposante, dossier)
        .await
        .expect("lecture");
    assert_eq!(
        lu.design.expect("choix").options,
        CoverDesignOptions {
            show_logos: true,
            ..CoverDesignOptions::default()
        }
    );

    // Un corps aux options incomplètes : les absentes valent faux.
    let corps: SaveCoverDesignPayload = serde_json::from_value(json!({
        "template_code": "bandeau",
        "accent": "cyan",
        "options": { "show_event_icon": true }
    }))
    .expect("corps partiel accepté");
    cover_design::enregistrer(&bac.state, &bac.ctx(), terrain.deposante, dossier, corps)
        .await
        .expect("enregistrement");

    let ecrit = sqlx::query_scalar!(
        "SELECT options FROM programme.proposal_cover_designs WHERE proposal_id = $1",
        dossier.as_uuid()
    )
    .fetch_one(bac.pool())
    .await
    .expect("options");
    assert_eq!(
        ecrit,
        json!({
            "show_date": false,
            "show_speakers": false,
            "show_logos": false,
            "show_event_banner": false,
            "show_event_icon": true
        })
    );
}

#[tokio::test]
async fn un_code_mal_forme_est_refuse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = ProposalId(commun::dossier(&bac, &terrain, "Mangroves", "atelier-codes").await);

    for (modele, couleur, champ, message) in [
        (
            "Bandeau",
            "cyan",
            "template_code",
            "Choisissez un modèle d'image dans la liste.",
        ),
        (
            "",
            "cyan",
            "template_code",
            "Choisissez un modèle d'image dans la liste.",
        ),
        (
            "bandeau",
            "bleu nuit",
            "accent",
            "Choisissez une couleur dans la liste.",
        ),
    ] {
        let refus = cover_design::enregistrer(
            &bac.state,
            &bac.ctx(),
            terrain.deposante,
            dossier,
            choix(modele, couleur, CoverDesignOptions::default()),
        )
        .await
        .expect_err("code refusé");
        assert_eq!(refus.code, ErrorCode::ValidationFailed);
        assert_eq!(refus.field.as_deref(), Some(champ));
        assert_eq!(refus.message, message);
    }
    let lu = cover_design::lire(&bac.state, terrain.deposante, dossier)
        .await
        .expect("lecture");
    assert!(lu.design.is_none(), "rien d'écrit");
}

#[tokio::test]
async fn hors_de_lorganisation_et_du_perimetre_rien_ne_se_lit_ni_ne_secrit() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = ProposalId(commun::dossier(&bac, &terrain, "Mangroves", "atelier-droits").await);

    let etrangere = commun::personne(&bac, "etrangere@example.org", "Eva", "Tranger").await;
    let refus = cover_design::lire(&bac.state, etrangere, dossier)
        .await
        .expect_err("une étrangère ne lit pas");
    assert_eq!(refus.code, ErrorCode::NotFound);
    let refus = cover_design::enregistrer(&bac.state, &bac.ctx(), etrangere, dossier, par_defaut())
        .await
        .expect_err("une étrangère n'écrit pas");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // Le comité lit la fiche mais n'en change pas l'image.
    let refus =
        cover_design::enregistrer(&bac.state, &bac.ctx(), droits.noteur, dossier, par_defaut())
            .await
            .expect_err("le comité n'écrit pas");
    assert_eq!(refus.code, ErrorCode::NotFound);

    let inexistant = ProposalId(Uuid::now_v7());
    let refus = cover_design::lire(&bac.state, terrain.deposante, inexistant)
        .await
        .expect_err("dossier inexistant");
    assert_eq!(refus.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn lequipe_qui_decide_et_la_deposante_en_attente_ecrivent() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = ProposalId(commun::dossier(&bac, &terrain, "Mangroves", "atelier-equipe").await);

    let lu = cover_design::enregistrer(
        &bac.state,
        &bac.ctx(),
        droits.decideur,
        dossier,
        par_defaut(),
    )
    .await
    .expect("l'équipe qui décide écrit");
    assert!(lu.design.is_some());
    let auteur = sqlx::query_scalar!(
        "SELECT updated_by FROM programme.proposal_cover_designs WHERE proposal_id = $1",
        dossier.as_uuid()
    )
    .fetch_one(bac.pool())
    .await
    .expect("auteur");
    assert_eq!(auteur, Some(droits.decideur));

    let attente = commun::personne(&bac, "attente@example.org", "Awa", "Attente").await;
    commun::adherer(&bac, terrain.organisation, attente, "pending").await;
    let sien = Terrain {
        edition: terrain.edition,
        appel: terrain.appel,
        organisation: terrain.organisation,
        deposante: attente,
    };
    let le_sien = ProposalId(commun::dossier(&bac, &sien, "Le sien", "atelier-attente").await);
    cover_design::enregistrer(&bac.state, &bac.ctx(), attente, le_sien, par_defaut())
        .await
        .expect("son propre dossier");
    let refus = cover_design::enregistrer(&bac.state, &bac.ctx(), attente, dossier, par_defaut())
        .await
        .expect_err("pas celui d'une collègue");
    assert_eq!(refus.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn un_dossier_rejete_ne_change_plus_dimage_mais_une_edition_terminee_si() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let depose = seances::dossier_pret(
        &bac,
        &terrain,
        "Écarté",
        "atelier-rejete",
        Souhaits::default(),
    )
    .await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        depose.id.into(),
        ProposalStatus::Rejected,
        None,
    )
    .await
    .expect("le rejet aboutit");

    let refus = cover_design::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(depose.id),
        par_defaut(),
    )
    .await
    .expect_err("dossier rejeté");
    assert_eq!(refus.code, ErrorCode::ProposalNotEditable);
    assert_eq!(
        refus.message,
        "Ce dossier n'a pas été retenu : son image de couverture ne se modifie plus."
    );
    cover_design::lire(&bac.state, terrain.deposante, ProposalId(depose.id))
        .await
        .expect("la lecture reste ouverte");

    // Les images restent modifiables après la fin de l'édition.
    let dossier = ProposalId(commun::dossier(&bac, &terrain, "Mangroves", "atelier-apres").await);
    sqlx::query!(
        "UPDATE event.events
            SET starts_at = now() - interval '10 days', ends_at = now() - interval '1 day'
          WHERE id = $1",
        terrain.edition
    )
    .execute(bac.pool())
    .await
    .expect("édition passée");
    cover_design::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier,
        par_defaut(),
    )
    .await
    .expect("permis après l'édition");
}
