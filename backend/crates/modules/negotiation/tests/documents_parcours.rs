//! **Le parcours d'un document, de bout en bout, par les services** : un
//! fichier publié se lit, un lien se liste, et la bibliothèque le dit.

mod commun;

use commun::documents::{administratrice, fichier_publie, lien_publie};
use commun::Bac;
use negotiation::service::documents as public;

#[tokio::test]
async fn un_fichier_publie_se_liste_et_se_lit() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let enb = lien_publie(
        &bac,
        ifdd,
        "Bulletin des négociations",
        "https://enb.iisd.org/cop30",
        false,
    )
    .await;

    let (bibliotheque, empreinte) = public::bibliotheque(&bac.state, None, "fr")
        .await
        .expect("bibliothèque");
    assert!(empreinte.starts_with('"'));
    let ids: Vec<_> = bibliotheque.documents.iter().map(|d| d.id).collect();
    assert!(ids.contains(&guide) && ids.contains(&enb));
    let g = bibliotheque
        .documents
        .iter()
        .find(|d| d.id == guide)
        .unwrap();
    assert_eq!(g.source, "file");
    assert_eq!(g.page_count, Some(4));
    assert_eq!(g.mode, Some("reflow"));
    assert!(g.reading_etag.is_some());
    let l = bibliotheque.documents.iter().find(|d| d.id == enb).unwrap();
    assert_eq!(l.link_host.as_deref(), Some("enb.iisd.org"));

    let (lecture, _, _) = public::lecture(&bac.state, None, guide)
        .await
        .expect("lecture");
    assert_eq!(lecture.page_count, 4);
    assert_eq!(
        lecture.pages.iter().filter(|p| p.image.is_some()).count(),
        1,
        "seule la page du tableau garde son image"
    );

    let image = public::image(&bac.state, None, guide, 3)
        .await
        .expect("image");
    assert!(image.octets.starts_with(&[0xFF, 0xD8]));

    let trouve = public::rechercher(&bac.state, None, "negociations reprennent")
        .await
        .expect("recherche");
    assert_eq!(
        trouve.hits.first().map(|h| h.document_id),
        Some(guide),
        "sans accent, le texte est trouvé"
    );
    assert_eq!(trouve.hits[0].pages[0].index, 1);
}
