//! **Le bandeau public : sans session, le plus grave en tête, chaque ligne
//! nommant son sujet.**
//!
//! `target_label` est déjà résolu par le modèle — « Atelier de négociation », le
//! nom légal d'une organisation : un message de portée `session` reste lisible
//! sur une page qui parle de trente activités.

mod commun;

use commun::*;

#[tokio::test]
async fn les_messages_actifs_sortent_le_plus_grave_en_tete() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let leger = poser(
        &bac,
        comptes.globale,
        "event",
        Some(decor.event_id),
        "active",
    )
    .await;
    let grave = poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;
    sqlx::query!(
        "UPDATE live.incidents SET severity = 'critical' WHERE id = $1",
        grave
    )
    .execute(bac.pool())
    .await
    .expect("gravité");

    let lignes = live::repo::active::pour_ledition(bac.pool(), decor.event_id)
        .await
        .expect("lecture publique");

    assert_eq!(lignes.len(), 2);
    assert_eq!(lignes[0].incident_id, grave, "le plus grave en tête");
    assert_eq!(lignes[1].incident_id, leger);
    assert_eq!(
        lignes[0].target_label.as_deref(),
        Some("Atelier de négociation"),
        "le bandeau NOMME son sujet"
    );
}
