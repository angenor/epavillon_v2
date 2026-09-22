//! **Une demande tranchée ne se retranche pas** — et c'est le trigger qui le
//! dit, pas un `if` du service.
//!
//! `tg_access_request_transition()` refuse toute sortie d'un état final.
//! Le service lit l'état pour répondre proprement avant d'écrire, mais le
//! dernier mot reste à la base : deux administrateurs qui décident à la même
//! seconde sont sérialisés par le verrou de ligne, et le second reçoit
//! `NEGOTIATION_ACCESS_REQUEST_DECIDED`.
//!
//! Le second test écrit **directement en SQL**, sans passer par le service :
//! c'est le seul moyen de prouver que l'invariant vit en base et non dans le
//! code qui l'appelle.

mod commun;

use commun::{admettre, attribuer, decor, demander, etat_de_la_demande, refuser, Bac};
use kernel::error::ErrorCode;

#[tokio::test]
async fn une_demande_admise_ne_se_refuse_plus() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let demande = demander(&bac, d.person_id, Some(d.space_id), None)
        .await
        .expect("demande");

    admettre(&bac, d.admin_id, demande.id, None)
        .await
        .expect("admission");

    let refus = refuser(&bac, d.admin_id, demande.id, Some("trop tard"))
        .await
        .expect_err("une demande admise est close");
    assert_eq!(refus.code, ErrorCode::NegotiationAccessRequestDecided);

    assert_eq!(
        etat_de_la_demande(&bac, demande.id).await,
        "approved",
        "la seconde décision n'a rien réécrit"
    );
}

#[tokio::test]
async fn la_base_refuse_la_transition_meme_sans_passer_par_le_service() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let demande = demander(&bac, d.person_id, None, None)
        .await
        .expect("demande");
    refuser(&bac, d.admin_id, demande.id, Some("hors périmètre"))
        .await
        .expect("refus");

    let brut = sqlx::query!(
        "UPDATE negotiation.access_requests
            SET status = 'approved', decided_at = now()
          WHERE id = $1",
        demande.id
    )
    .execute(bac.pool())
    .await;

    let erreur = brut.expect_err("le trigger refuse la transition");
    assert_eq!(
        kernel::pg_error::sqlstate(&erreur).as_deref(),
        Some("23000"),
        "integrity_constraint_violation, levé par tg_access_request_transition"
    );
}

#[tokio::test]
async fn un_refus_nempeche_pas_une_nouvelle_demande() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let premiere = demander(&bac, d.person_id, Some(d.space_id), None)
        .await
        .expect("demande");
    refuser(&bac, d.admin_id, premiere.id, Some("dossier incomplet"))
        .await
        .expect("refus");

    // **Une nouvelle demande est une NOUVELLE LIGNE** : l'historique des
    // décisions se conserve, et c'est ce que le modèle exige.
    let seconde = demander(&bac, d.person_id, Some(d.space_id), Some("je complète"))
        .await
        .expect("seconde demande");
    assert_ne!(premiere.id, seconde.id);

    assert_eq!(etat_de_la_demande(&bac, premiere.id).await, "rejected");
    assert_eq!(etat_de_la_demande(&bac, seconde.id).await, "pending");
}
