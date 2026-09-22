//! **Une seule demande en attente par personne et par portée** — FR-024.
//!
//! L'unicité vient des deux index partiels du modèle, pas d'une lecture
//! préalable : deux appareils qui envoient ensemble ne produisent qu'une ligne,
//! et le second reçoit un conflit traduit en français (principe VIII).
//!
//! Ce fichier lance les deux envois **réellement en concurrence**, sur deux
//! connexions distinctes : un test séquentiel passerait avec une vérification
//! préalable, qui est précisément ce que la spécification refuse.

mod commun;

use commun::{annuler_sa_demande, attribuer, decor, demander, personne, Bac};
use kernel::error::ErrorCode;

#[tokio::test]
async fn deux_envois_simultanes_ne_font_quune_demande() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    let (premier, second) = tokio::join!(
        demander(
            &bac,
            d.person_id,
            Some(d.space_id),
            Some("depuis le téléphone")
        ),
        demander(
            &bac,
            d.person_id,
            Some(d.space_id),
            Some("depuis la tablette")
        ),
    );

    let reussites = [&premier, &second].iter().filter(|r| r.is_ok()).count();
    assert_eq!(reussites, 1, "une seule des deux passe");

    let refus = [premier, second]
        .into_iter()
        .find_map(|r| r.err())
        .expect("l'autre est refusée");
    assert_eq!(refus.code, ErrorCode::NegotiationAccessRequestPending);

    let lignes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!"
             FROM negotiation.access_requests
            WHERE person_id = $1 AND status = 'pending'"#,
        d.person_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(lignes, 1);
}

#[tokio::test]
async fn une_demande_annulee_laisse_en_refaire_une() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    let premiere = demander(&bac, d.person_id, Some(d.space_id), None)
        .await
        .expect("première demande");

    annuler_sa_demande(&bac, d.person_id, premiere.id)
        .await
        .expect("annulation");

    demander(&bac, d.person_id, Some(d.space_id), None)
        .await
        .expect("l'index partiel ne compte que les demandes en attente");
}

#[tokio::test]
async fn les_portees_ne_se_genent_pas() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    demander(&bac, d.person_id, Some(d.space_id), None)
        .await
        .expect("demande sur un espace");
    demander(&bac, d.person_id, None, None)
        .await
        .expect("et une sur Guide Négo en entier : deux portées, deux demandes");
}

#[tokio::test]
async fn la_demande_dun_autre_compte_se_refuse_comme_inexistante() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let demande = demander(&bac, d.person_id, Some(d.space_id), None)
        .await
        .expect("demande");

    let curieuse = personne(&bac, "curieuse@example.org").await;
    let refus = annuler_sa_demande(&bac, curieuse, demande.id)
        .await
        .expect_err("on n'annule pas la demande d'un autre");

    assert_eq!(
        refus.code,
        ErrorCode::NotFound,
        "la forme du refus ne dit pas que la demande existe ailleurs"
    );
}
