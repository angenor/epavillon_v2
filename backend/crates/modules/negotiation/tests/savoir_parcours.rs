//! **Les coches du parcours**, par le service, sur base réelle. Le `304` et le
//! `401` se prouvent en HTTP, dans `crates/api/tests/routes_negotiation_savoir.rs`.

mod commun;

use commun::savoir::etape;
use commun::{personne, Bac};
use kernel::error::ErrorCode;
use negotiation::service::savoir_coches::{cocher, coches, decocher};
use uuid::Uuid;

async fn ids_de(bac: &Bac, qui: Uuid) -> Vec<Uuid> {
    let mut ids = coches(&bac.state, qui).await.expect("coches").0.step_ids;
    ids.sort();
    ids
}

async fn publier_l_etape(bac: &Bac, id: Uuid, publie: bool) {
    sqlx::query("UPDATE negotiation.pathway_steps SET is_published = $2 WHERE id = $1")
        .bind(id)
        .bind(publie)
        .execute(bac.pool())
        .await
        .expect("publication de l'étape");
}

async fn publier_le_groupe(bac: &Bac, id: Uuid, publie: bool) {
    sqlx::query("UPDATE negotiation.pathway_groups SET is_published = $2 WHERE id = $1")
        .bind(id)
        .bind(publie)
        .execute(bac.pool())
        .await
        .expect("publication du groupe");
}

#[tokio::test]
async fn une_etape_se_coche_et_se_decoche_deux_fois_sans_erreur() {
    let bac = Bac::monter().await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let (_, badge) = etape(&bac, "Le premier jour", "Retirer mon badge").await;
    let ctx = bac.ctx(moussa);

    let (vide, avant) = coches(&bac.state, moussa).await.unwrap();
    assert!(vide.step_ids.is_empty());

    for _ in 0..2 {
        cocher(&bac.state, &ctx, moussa, badge)
            .await
            .expect("coche");
    }
    let (liste, cochee) = coches(&bac.state, moussa).await.unwrap();
    assert_eq!(liste.step_ids, [badge], "une seule coche, pas deux");
    assert_ne!(cochee, avant);
    assert_eq!(coches(&bac.state, moussa).await.unwrap().1, cochee);

    for _ in 0..2 {
        decocher(&bac.state, &ctx, moussa, badge)
            .await
            .expect("décoche");
    }
    let (liste, decochee) = coches(&bac.state, moussa).await.unwrap();
    assert!(liste.step_ids.is_empty());
    assert_eq!(decochee, avant, "l'empreinte revient");
}

#[tokio::test]
async fn les_coches_dune_personne_ne_touchent_pas_celles_dune_autre() {
    let bac = Bac::monter().await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let awa = personne(&bac, "awa@example.org").await;
    let (_, badge) = etape(&bac, "Le premier jour", "Retirer mon badge").await;
    let (_, salles) = etape(&bac, "En salle", "Vérifier la salle").await;

    for id in [badge, salles] {
        cocher(&bac.state, &bac.ctx(moussa), moussa, id)
            .await
            .expect("coche de Moussa");
    }
    cocher(&bac.state, &bac.ctx(awa), awa, badge)
        .await
        .expect("coche d'Awa");
    decocher(&bac.state, &bac.ctx(moussa), moussa, badge)
        .await
        .expect("décoche de Moussa");

    assert_eq!(ids_de(&bac, moussa).await, [salles]);
    assert_eq!(
        ids_de(&bac, awa).await,
        [badge],
        "la décoche de l'un laisse la coche de l'autre"
    );
}

#[tokio::test]
async fn une_etape_inconnue_ou_non_publiee_rend_404_et_se_decoche_quand_meme() {
    let bac = Bac::monter().await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let (_, retiree) = etape(&bac, "Avant de partir", "Étape retirée").await;
    let (groupe_cache, dans_groupe_cache) = etape(&bac, "Brouillon", "Étape cachée").await;
    publier_l_etape(&bac, retiree, false).await;
    publier_le_groupe(&bac, groupe_cache, false).await;
    let ctx = bac.ctx(moussa);

    for id in [retiree, dans_groupe_cache, Uuid::now_v7()] {
        let e = cocher(&bac.state, &ctx, moussa, id)
            .await
            .expect_err("pas de coche hors des étapes publiées");
        assert_eq!(e.code, ErrorCode::NegotiationPathwayStepNotFound);
        assert_eq!(e.code.status().as_u16(), 404);
        decocher(&bac.state, &ctx, moussa, id)
            .await
            .expect("décocher ne se refuse jamais");
    }
}

#[tokio::test]
async fn une_etape_depubliee_sort_de_la_liste_sans_perdre_sa_coche() {
    let bac = Bac::monter().await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let (_, badge) = etape(&bac, "Le premier jour", "Retirer mon badge").await;
    let (groupe, salles) = etape(&bac, "En salle", "Vérifier la salle").await;
    for id in [badge, salles] {
        cocher(&bac.state, &bac.ctx(moussa), moussa, id)
            .await
            .expect("coche");
    }

    publier_l_etape(&bac, badge, false).await;
    publier_le_groupe(&bac, groupe, false).await;
    assert!(ids_de(&bac, moussa).await.is_empty());

    publier_l_etape(&bac, badge, true).await;
    publier_le_groupe(&bac, groupe, true).await;
    let mut deux = vec![badge, salles];
    deux.sort();
    assert_eq!(
        ids_de(&bac, moussa).await,
        deux,
        "la ligne n'est pas effacée"
    );
}
