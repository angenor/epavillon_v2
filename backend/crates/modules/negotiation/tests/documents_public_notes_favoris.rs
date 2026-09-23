//! **Les lectures publiques des documents, suite** : les notes de correction,
//! le compteur de téléchargements et les favoris — par les services, sur base
//! réelle. Le `304` se prouve en HTTP, dans `crates/api/tests/`.

mod commun;

use commun::documents::{administratrice, creer, expert, fichier_publie, lien_publie};
use commun::{personne, Bac};
use kernel::error::ErrorCode;
use negotiation::domain::admin_documents::CorrectionNoteInput;
use negotiation::service::{admin_documents, corrections, documents as public};
use serde_json::json;
use uuid::Uuid;

async fn telechargements(bac: &Bac, id: Uuid) -> i32 {
    sqlx::query_scalar::<_, i32>("SELECT download_count FROM negotiation.documents WHERE id = $1")
        .bind(id)
        .fetch_one(bac.pool())
        .await
        .expect("compteur")
}

/// Les favoris de la personne, triés : l'ordre n'est pas au contrat.
async fn favoris_de(bac: &Bac, personne: Uuid) -> Vec<Uuid> {
    let mut ids: Vec<Uuid> = public::favoris(&bac.state, personne)
        .await
        .expect("favoris")
        .0
        .bookmarks
        .iter()
        .map(|b| b.document_id)
        .collect();
    ids.sort();
    ids
}

// -----------------------------------------------------------------------------
// Les notes de correction
// -----------------------------------------------------------------------------

#[tokio::test]
async fn les_notes_vivantes_des_documents_publies_vont_a_tous() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let relectrice = expert(&bac, "expert@example.org").await;
    // Un autre nom que celui de l'administratrice : une jointure sur la
    // mauvaise personne se verrait.
    sqlx::query(
        "UPDATE identity.people SET first_name = 'Mariam', last_name = 'Koné' WHERE id = $1",
    )
    .bind(relectrice)
    .execute(bac.pool())
    .await
    .expect("renommage");
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let retire = fichier_publie(&bac, ifdd, "Guide retiré", false).await;

    let (vide, avant) = public::notes(&bac.state, None, "fr").await.unwrap();
    assert!(vide.notes.is_empty());

    let note = corrections::poser(
        &bac.state,
        &bac.ctx(relectrice),
        guide,
        &CorrectionNoteInput {
            page_index: 2,
            passage: Some("le chiffre cité".to_owned()),
            body: json!({ "fr": "Lire 1,5 °C.", "en": "Read 1.5 °C." }),
        },
    )
    .await
    .expect("note posée");
    assert_eq!(note.author.id, relectrice);
    let _sur_le_retire = corrections::poser(
        &bac.state,
        &bac.ctx(relectrice),
        retire,
        &CorrectionNoteInput {
            page_index: 1,
            passage: None,
            body: json!({ "fr": "Note d'un document bientôt dépublié." }),
        },
    )
    .await
    .expect("note posée");
    admin_documents::depublier(&bac.state, &bac.ctx(ifdd), retire)
        .await
        .expect("dépublication");

    let (en, _) = public::notes(&bac.state, None, "en").await.unwrap();
    assert_eq!(en.notes.len(), 1, "seules les notes des documents publiés");
    let n = &en.notes[0];
    assert_eq!(n.id, note.id);
    assert_eq!(n.document_id, guide);
    assert_eq!(n.page_index, 2);
    assert_eq!(n.passage.as_deref(), Some("le chiffre cité"));
    assert_eq!(n.body, "Read 1.5 °C.", "résolue dans la langue demandée");
    assert_eq!(
        n.author_name, "Mariam Koné",
        "l'expert, pas l'administratrice"
    );
    assert_eq!(n.posted_at, note.posted_at);

    let (fr, apres) = public::notes(&bac.state, None, "fr").await.unwrap();
    assert_eq!(fr.notes[0].body, "Lire 1,5 °C.");
    assert_ne!(apres, avant, "une note posée change l'empreinte");
    let (_, meme) = public::notes(&bac.state, None, "fr").await.unwrap();
    assert_eq!(meme, apres, "même empreinte à contenu égal");

    corrections::retirer(&bac.state, &bac.ctx(relectrice), note.id)
        .await
        .expect("retrait");
    let (sans, retiree) = public::notes(&bac.state, None, "fr").await.unwrap();
    assert!(sans.notes.is_empty(), "une note retirée ne se sert plus");
    assert_eq!(
        retiree, avant,
        "retirée, la liste revient à son empreinte vide"
    );
}

// -----------------------------------------------------------------------------
// Le compteur
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_compteur_sans_compte_incremente_les_telechargements() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let autre = lien_publie(&bac, ifdd, "Bulletin", "https://enb.iisd.org/", false).await;
    assert_eq!(telechargements(&bac, guide).await, 0);

    public::compter_un_telechargement(&bac.state, None, guide)
        .await
        .expect("compté sans compte");
    public::compter_un_telechargement(&bac.state, None, guide)
        .await
        .expect("compté une seconde fois");
    assert_eq!(telechargements(&bac, guide).await, 2);
    assert_eq!(
        telechargements(&bac, autre).await,
        0,
        "le seul document compté"
    );
}

// -----------------------------------------------------------------------------
// Les favoris
// -----------------------------------------------------------------------------

#[tokio::test]
async fn un_favori_se_pose_et_se_retire_deux_fois_sans_erreur() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let lectrice = personne(&bac, "lectrice@example.org").await;
    let guide = fichier_publie(&bac, ifdd, "Guide des négociations", false).await;
    let ctx = bac.ctx(lectrice);

    let (vide, avant) = public::favoris(&bac.state, lectrice).await.unwrap();
    assert!(vide.bookmarks.is_empty());

    public::poser_un_favori(&bac.state, &ctx, lectrice, guide)
        .await
        .expect("premier favori");
    public::poser_un_favori(&bac.state, &ctx, lectrice, guide)
        .await
        .expect("le même, une seconde fois");
    let (liste, pose) = public::favoris(&bac.state, lectrice).await.unwrap();
    let ids: Vec<Uuid> = liste.bookmarks.iter().map(|b| b.document_id).collect();
    assert_eq!(ids, [guide], "un seul favori, pas deux");
    assert_ne!(pose, avant);
    let (_, meme) = public::favoris(&bac.state, lectrice).await.unwrap();
    assert_eq!(meme, pose, "même empreinte à contenu égal");

    public::retirer_un_favori(&bac.state, &ctx, lectrice, guide)
        .await
        .expect("retrait");
    public::retirer_un_favori(&bac.state, &ctx, lectrice, guide)
        .await
        .expect("retrait d'un favori déjà retiré");
    let (liste, retire) = public::favoris(&bac.state, lectrice).await.unwrap();
    assert!(liste.bookmarks.is_empty());
    assert_eq!(retire, avant);
}

#[tokio::test]
async fn les_favoris_dune_personne_ne_touchent_pas_ceux_dune_autre() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let lectrice = personne(&bac, "lectrice@example.org").await;
    let voisine = personne(&bac, "voisine@example.org").await;
    let guide = lien_publie(&bac, ifdd, "Guide", "https://unfccc.int/guide", false).await;
    let bulletin = lien_publie(&bac, ifdd, "Bulletin", "https://enb.iisd.org/", false).await;

    for id in [guide, bulletin] {
        public::poser_un_favori(&bac.state, &bac.ctx(lectrice), lectrice, id)
            .await
            .expect("favori de la lectrice");
    }
    public::poser_un_favori(&bac.state, &bac.ctx(voisine), voisine, guide)
        .await
        .expect("favori de la voisine");

    let mut deux = vec![guide, bulletin];
    deux.sort();
    assert_eq!(favoris_de(&bac, lectrice).await, deux);
    assert_eq!(
        favoris_de(&bac, voisine).await,
        [guide],
        "rien de la lectrice"
    );

    public::retirer_un_favori(&bac.state, &bac.ctx(lectrice), lectrice, guide)
        .await
        .expect("retrait par la lectrice");
    assert_eq!(favoris_de(&bac, lectrice).await, [bulletin]);
    assert_eq!(
        favoris_de(&bac, voisine).await,
        [guide],
        "le retrait de l'une laisse le favori de l'autre"
    );
}

#[tokio::test]
async fn un_favori_sur_un_document_non_publie_est_refuse() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let lectrice = personne(&bac, "lectrice@example.org").await;
    let brouillon = creer(&bac, ifdd, "Brouillon", false).await;
    let depublie = fichier_publie(&bac, ifdd, "Guide retiré", false).await;
    admin_documents::depublier(&bac.state, &bac.ctx(ifdd), depublie)
        .await
        .expect("dépublication");
    let ctx = bac.ctx(lectrice);

    for id in [brouillon, depublie, Uuid::now_v7()] {
        let e = public::poser_un_favori(&bac.state, &ctx, lectrice, id)
            .await
            .expect_err("pas de favori sur un document non publié");
        assert_eq!(e.code, ErrorCode::NegotiationDocumentNotFound);
        assert_eq!(e.code.status().as_u16(), 404);
    }
    let (liste, _) = public::favoris(&bac.state, lectrice).await.unwrap();
    assert!(liste.bookmarks.is_empty());
}
