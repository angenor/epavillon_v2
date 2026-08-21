//! **La diffusion, et la règle « un seul direct »** — règle métier n° 4.
//!
//! Le canal est une ressource réservable au même titre qu'une salle : deux
//! directs simultanés remontent en gravité bloquante. **Ils s'écrivent quand
//! même** — l'équipe doit pouvoir poser deux directs le temps d'en décaler un.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::transitions::ProposalStatus;
use programme::service::transition;
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

/// Sans canal choisi, **le déclencheur pose celui de l'édition** : c'est ce qui
/// évite qu'une séance « diffusée » échappe à la règle du direct unique par
/// simple distraction.
#[tokio::test]
async fn sans_canal_choisi_le_canal_par_defaut_est_pose() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    seances::diffuser(&bac, terrain.edition, id, true, None)
        .await
        .expect("la diffusion s'active");

    let relue = seances::seance(&bac, id).await;
    assert!(relue.is_streamed);
    assert_eq!(
        relue.broadcast_channel_id,
        Some(grille.canal),
        "le canal par défaut de l'édition"
    );
}

/// 🔴 **Un canal choisi est retenu tel quel**, et non remplacé par le canal
/// d'office : le déclencheur ne pose le canal par défaut que lorsque la colonne
/// est nulle — il complète, il n'écrase jamais. Refuser ce champ, comme l'écart
/// n° 7 le demandait à la lettre, casserait une fonctionnalité livrée.
#[tokio::test]
async fn un_canal_choisi_est_retenu_et_non_remplace() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    let second = seances::canal_secondaire(&bac, terrain.edition, "langue-anglaise").await;

    seances::diffuser(&bac, terrain.edition, id, true, Some(second))
        .await
        .expect("le canal choisi est accepté");

    let relue = seances::seance(&bac, id).await;
    assert_eq!(
        relue.broadcast_channel_id,
        Some(second),
        "l'écran laisse le choix quand l'édition a plusieurs canaux"
    );
}

/// Retirer la diffusion **efface** le canal — c'est la base qui le fait. Le
/// retirer **en désignant un canal** est donc refusé : c'est le seul endroit où
/// un choix disparaîtrait en silence.
#[tokio::test]
async fn retirer_la_diffusion_efface_le_canal_et_refuse_quon_en_designe_un() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let grille = seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    seances::diffuser(&bac, terrain.edition, id, true, None)
        .await
        .unwrap();
    seances::diffuser(&bac, terrain.edition, id, false, None)
        .await
        .unwrap();

    let relue = seances::seance(&bac, id).await;
    assert!(!relue.is_streamed);
    assert_eq!(relue.broadcast_channel_id, None, "la base efface le canal");

    let erreur = seances::diffuser(&bac, terrain.edition, id, false, Some(grille.canal))
        .await
        .expect_err("désigner un canal sans diffusion est refusé");
    assert_eq!(erreur.code, ErrorCode::SessionDerivedField);
    assert_eq!(erreur.field.as_deref(), Some("broadcast_channel_id"));
}

/// Un canal d'une autre édition, ou désactivé, est refusé **en le disant** :
/// aucune clé étrangère ne le vérifie, seul le service.
#[tokio::test]
async fn un_canal_etranger_ou_desactive_est_refuse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;

    let autre = commun::edition_secondaire(&bac).await;
    let canal_etranger = seances::canal_par_defaut(&bac, autre).await;

    let erreur = seances::diffuser(&bac, terrain.edition, id, true, Some(canal_etranger))
        .await
        .expect_err("un canal d'une autre édition est refusé");
    assert_eq!(erreur.code, ErrorCode::SessionUnknownReference);
    assert_eq!(erreur.field.as_deref(), Some("broadcast_channel_id"));

    let retire = seances::canal_secondaire(&bac, terrain.edition, "retire").await;
    sqlx::query!(
        "UPDATE event.broadcast_channels SET is_active = false WHERE id = $1",
        retire
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let erreur = seances::diffuser(&bac, terrain.edition, id, true, Some(retire))
        .await
        .expect_err("un canal désactivé ne s'offre plus");
    assert_eq!(erreur.code, ErrorCode::SessionUnknownReference);
}

/// 🔴 **Deux directs simultanés, sur DEUX ÉDITIONS différentes** : les deux
/// écritures aboutissent, et le conflit remonte **depuis l'une comme depuis
/// l'autre**. Le direct est une ressource de la plateforme, pas de l'événement.
#[tokio::test]
async fn deux_directs_simultanes_saccrivent_et_remontent_des_deux_cotes() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    // Une seconde édition, complète, avec sa propre séance.
    let autre = commun::edition_secondaire(&bac).await;
    let terrain_voisin = commun::Terrain {
        edition: autre,
        appel: commun::appel_ouvert(&bac, autre).await,
        organisation: commun::organisation_verifiee(&bac, "Voisine", "VSN").await,
        deposante: commun::personne(&bac, "voisine@example.org", "Vera", "Sow").await,
    };
    commun::adherer(
        &bac,
        terrain_voisin.organisation,
        terrain_voisin.deposante,
        "active",
    )
    .await;
    seances::grille(&bac, autre).await;

    let ici = seance(&bac, &terrain, "Ici", "ici").await;
    let ailleurs = {
        let dossier = seances::dossier_pret(
            &bac,
            &terrain_voisin,
            "Ailleurs",
            "ailleurs",
            Souhaits {
                creneau: Some("2027-03-02 14:00"),
                ..Souhaits::default()
            },
        )
        .await;
        transition::tenter(
            &bac.state,
            &bac.ctx(),
            dossier.id.into(),
            ProposalStatus::Accepted,
            None,
        )
        .await
        .unwrap();
        seances::seances_du_dossier(&bac, dossier.id)
            .await
            .remove(0)
            .id
    };

    // Un canal **de la plateforme** — `event_id IS NULL` — applicable aux deux.
    let canal_commun = sqlx::query_scalar!(
        r#"INSERT INTO event.broadcast_channels (event_id, code, name, is_default)
           VALUES (NULL, 'plateforme', '{"fr":"Chaîne de la plateforme"}'::jsonb, false)
        RETURNING id"#
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();

    // Le même créneau, en instants absolus : les deux éditions n'ont pas le
    // même fuseau, c'est donc l'instant qui compte.
    let debut = seances::instant_local(&bac, terrain.edition, "2027-11-12 14:00").await;
    for (edition, id) in [(terrain.edition, ici), (autre, ailleurs)] {
        sqlx::query!(
            "UPDATE programme.sessions
                SET starts_at = $2::timestamptz,
                    ends_at = $2::timestamptz + interval '90 minutes'
              WHERE id = $1",
            id,
            debut
        )
        .execute(bac.pool())
        .await
        .unwrap();

        seances::diffuser(&bac, edition, id, true, Some(canal_commun))
            .await
            .expect("🔴 deux directs simultanés s'écrivent");
    }

    for (edition, _) in [(terrain.edition, ici), (autre, ailleurs)] {
        let conflits = programme::service::planner::conflits(
            bac.pool(),
            programme::domain::ids::EventId(edition),
        )
        .await
        .unwrap();

        assert!(
            conflits
                .iter()
                .any(|c| c.conflict_kind == "broadcast" && c.severity == "blocking"),
            "le conflit de diffusion remonte depuis l'édition {edition}"
        );
    }
}
