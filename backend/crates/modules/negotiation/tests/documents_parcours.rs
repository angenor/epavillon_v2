//! **Le parcours d'un document, de bout en bout, par les services** : un
//! fichier publié se lit, un lien se liste, et la bibliothèque le dit.

mod commun;

use commun::documents::{administratrice, fichier_publie, lien_publie, PETIT};
use commun::Bac;
use negotiation::domain::documents::serialiser_la_lecture;
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
    assert!(g.has_text && g.large_text);
    assert!(g.reading_etag.is_some());
    let l = bibliotheque.documents.iter().find(|d| d.id == enb).unwrap();
    assert_eq!(l.link_host.as_deref(), Some("enb.iisd.org"));

    let (lecture, _, _) = public::lecture(&bac.state, None, guide)
        .await
        .expect("lecture");
    assert_eq!(lecture.page_count, 4);
    assert_eq!(
        g.reading_bytes,
        Some((PETIT.len() + serialiser_la_lecture(&lecture).len()) as i64),
        "la taille annoncée est celle de la copie : le PDF et la lecture servie"
    );

    let entier = public::lire_le_fichier(&bac.state, None, guide, None, true)
        .await
        .expect("le fichier entier");
    assert_eq!(entier.octets, PETIT, "le PDF déposé, octet pour octet");

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
