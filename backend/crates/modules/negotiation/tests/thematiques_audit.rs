//! **Toute écriture laisse son auteur** — `platform.audit_log` porte un
//! `actor_id` à chaque ligne des suivis, ouverture comme fermeture. Une
//! transaction ouverte hors de `Db::write` produirait une trace anonyme.

mod commun;

use commun::{personne, suivre, traces_de_suivi, Bac};

#[tokio::test]
async fn ouvrir_et_fermer_nomment_la_personne() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    suivre(&bac, awa, &["adaptation", "gender"], None).await.expect("ouverture");
    suivre(&bac, awa, &["gender"], None).await.expect("fermeture");

    let traces = traces_de_suivi(&bac, awa).await;
    assert_eq!(traces.len(), 3, "deux insertions, une mise à jour : {traces:?}");
    assert_eq!(traces.iter().filter(|(a, _)| a == "insert").count(), 2);
    assert_eq!(traces.iter().filter(|(a, _)| a == "update").count(), 1);
    assert!(
        traces.iter().all(|(_, acteur)| *acteur == Some(awa)),
        "aucune écriture anonyme, et l'auteur est la personne elle-même : {traces:?}"
    );
}
