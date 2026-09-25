//! **Les termes favoris**, par le service, sur base réelle. Le `304` et le `401`
//! se prouvent en HTTP, dans `crates/api/tests/routes_negotiation_savoir.rs`.

mod commun;

use commun::savoir::terme;
use commun::{personne, Bac};
use kernel::error::ErrorCode;
use negotiation::service::savoir_favoris::{favoris, poser_un_favori, retirer_un_favori};
use uuid::Uuid;

async fn ids_de(bac: &Bac, qui: Uuid) -> Vec<Uuid> {
    let mut ids = favoris(&bac.state, qui).await.expect("favoris").0.entry_ids;
    ids.sort();
    ids
}

async fn statut(bac: &Bac, id: Uuid, statut: &str) {
    sqlx::query(
        "UPDATE negotiation.glossary_entries SET status = $2::text::negotiation.knowledge_status WHERE id = $1",
    )
    .bind(id)
    .bind(statut)
    .execute(bac.pool())
    .await
    .expect("changement de statut");
}

#[tokio::test]
async fn un_terme_se_pose_et_se_retire_deux_fois_sans_erreur() {
    let bac = Bac::monter().await;
    let lectrice = personne(&bac, "lectrice@example.org").await;
    let groupe = terme(&bac, "Contact group", "published", None, &[]).await;
    let ctx = bac.ctx(lectrice);

    let (vide, avant) = favoris(&bac.state, lectrice).await.unwrap();
    assert!(vide.entry_ids.is_empty());

    for _ in 0..2 {
        poser_un_favori(&bac.state, &ctx, lectrice, groupe)
            .await
            .expect("pose");
    }
    let (liste, pose) = favoris(&bac.state, lectrice).await.unwrap();
    assert_eq!(liste.entry_ids, [groupe], "un seul favori, pas deux");
    assert_ne!(pose, avant);
    assert_eq!(favoris(&bac.state, lectrice).await.unwrap().1, pose);

    for _ in 0..2 {
        retirer_un_favori(&bac.state, &ctx, lectrice, groupe)
            .await
            .expect("retrait");
    }
    let (liste, retire) = favoris(&bac.state, lectrice).await.unwrap();
    assert!(liste.entry_ids.is_empty());
    assert_eq!(retire, avant, "l'empreinte revient");
}

#[tokio::test]
async fn les_favoris_dune_personne_ne_touchent_pas_ceux_dune_autre() {
    let bac = Bac::monter().await;
    let lectrice = personne(&bac, "lectrice@example.org").await;
    let voisine = personne(&bac, "voisine@example.org").await;
    let groupe = terme(&bac, "Contact group", "published", None, &[]).await;
    let pleniere = terme(&bac, "Plénière", "published", None, &[]).await;

    for id in [groupe, pleniere] {
        poser_un_favori(&bac.state, &bac.ctx(lectrice), lectrice, id)
            .await
            .expect("favori de la lectrice");
    }
    poser_un_favori(&bac.state, &bac.ctx(voisine), voisine, groupe)
        .await
        .expect("favori de la voisine");

    let mut deux = vec![groupe, pleniere];
    deux.sort();
    assert_eq!(ids_de(&bac, lectrice).await, deux);
    assert_eq!(ids_de(&bac, voisine).await, [groupe]);

    retirer_un_favori(&bac.state, &bac.ctx(lectrice), lectrice, groupe)
        .await
        .expect("retrait par la lectrice");
    assert_eq!(ids_de(&bac, lectrice).await, [pleniere]);
    assert_eq!(
        ids_de(&bac, voisine).await,
        [groupe],
        "le retrait de l'une laisse le favori de l'autre"
    );
}

#[tokio::test]
async fn un_brouillon_ou_un_inconnu_rend_404_et_a_verifier_saccepte() {
    let bac = Bac::monter().await;
    let lectrice = personne(&bac, "lectrice@example.org").await;
    let brouillon = terme(&bac, "Bracket", "draft", None, &[]).await;
    let a_revoir = terme(&bac, "Chair", "to_review", None, &[]).await;
    let ctx = bac.ctx(lectrice);

    for id in [brouillon, Uuid::now_v7()] {
        let e = poser_un_favori(&bac.state, &ctx, lectrice, id)
            .await
            .expect_err("pas de favori hors des entrées servies");
        assert_eq!(e.code, ErrorCode::NegotiationGlossaryNotFound);
        assert_eq!(e.code.status().as_u16(), 404);
    }

    poser_un_favori(&bac.state, &ctx, lectrice, a_revoir)
        .await
        .expect("une entrée à vérifier reste servie");
    assert_eq!(ids_de(&bac, lectrice).await, [a_revoir]);
}

#[tokio::test]
async fn une_entree_repassee_en_brouillon_sort_de_la_liste() {
    let bac = Bac::monter().await;
    let lectrice = personne(&bac, "lectrice@example.org").await;
    let groupe = terme(&bac, "Contact group", "published", None, &[]).await;
    let pleniere = terme(&bac, "Plénière", "published", None, &[]).await;
    for id in [groupe, pleniere] {
        poser_un_favori(&bac.state, &bac.ctx(lectrice), lectrice, id)
            .await
            .expect("pose");
    }

    statut(&bac, groupe, "draft").await;
    assert_eq!(ids_de(&bac, lectrice).await, [pleniere]);

    statut(&bac, groupe, "published").await;
    let mut deux = vec![groupe, pleniere];
    deux.sort();
    assert_eq!(
        ids_de(&bac, lectrice).await,
        deux,
        "la ligne n'est pas effacée"
    );
}
