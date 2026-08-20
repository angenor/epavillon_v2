//! **Trois cas, trois réponses différentes.**
//!
//! C'est la règle métier n° 8 sur l'écran qui la rend visible : un compte
//! détaché sur une édition n'en voit qu'une, un compte global les voit toutes,
//! et un compte sans aucun droit reçoit un **refus** — jamais une liste vide.
//!
//! Le troisième cas est celui qui se perd le plus facilement : un garde écrit
//! « pas global → filtrer » confond « aucun droit » et « une seule édition », et
//! affiche une page vide à qui n'a rien le droit de voir. Personne ne peut alors
//! savoir s'il n'y a rien, ou s'il n'a pas accès.

mod commun;

use commun::{perimetres, seed, Bac};
use event::service::edition_read;
use kernel::error::ErrorCode;

#[tokio::test]
async fn un_compte_global_voit_toutes_les_editions() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let perimetre = commun::perimetre_de(&bac, p.globale).await;
    let ecran = edition_read::ecran(bac.pool(), &perimetre.scope)
        .await
        .expect("la liste");

    assert_eq!(ecran.rows.len(), 2);
    assert!(
        ecran.is_global_scope,
        "l'écran doit pouvoir distinguer un filtrage d'une absence"
    );
}

#[tokio::test]
async fn un_compte_detache_ne_voit_que_son_edition() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let perimetre = commun::perimetre_de(&bac, p.detache).await;
    let ecran = edition_read::ecran(bac.pool(), &perimetre.scope)
        .await
        .expect("la liste");

    assert_eq!(ecran.rows.len(), 1);
    assert_eq!(ecran.rows[0].id, editions.sans_pavillon);
    assert!(!ecran.is_global_scope);
}

/// **Un refus, pas une liste vide.** Le service le tient lui-même, en plus de
/// l'extracteur : appelé d'ailleurs, il doit refuser tout autant.
#[tokio::test]
async fn un_perimetre_vide_recoit_un_refus_et_non_une_liste_vide() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let vide = kernel::auth::administered_events(bac.pool(), p.sans_droit)
        .await
        .expect("lecture du périmètre");
    assert!(vide.is_empty());

    let refus = edition_read::ecran(bac.pool(), &vide)
        .await
        .expect_err("un périmètre vide se refuse");
    assert_eq!(refus.code, ErrorCode::Forbidden);
}

/// **Les facettes se comptent sur le même jeu de lignes que la liste**
/// (FR-018). Un compte détaché ne doit pas voir au filtre la série d'une édition
/// qu'il n'administre pas : ce serait divulguer par la facette ce que la liste
/// vient de masquer.
#[tokio::test]
async fn les_facettes_se_comptent_sur_les_lignes_visibles() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let globale = commun::perimetre_de(&bac, p.globale).await;
    let tout = edition_read::ecran(bac.pool(), &globale.scope)
        .await
        .expect("la liste");
    assert_eq!(tout.series.len(), 2, "climat et webinaires, pas les quatre");
    assert!(tout.series.iter().all(|s| s.edition_count == 1));
    assert_eq!(tout.years, vec![2027]);

    let detache = commun::perimetre_de(&bac, p.detache).await;
    let borne = edition_read::ecran(bac.pool(), &detache.scope)
        .await
        .expect("la liste");
    assert_eq!(
        borne.series.len(),
        1,
        "la série de la COP31 n'a pas à apparaître au filtre d'un compte qui ne l'administre pas"
    );
}

/// Les décomptes joints suivent la ligne, et une édition qui n'a **rien** à
/// compter reste visible avec des zéros — le piège classique du décompte en
/// jointure interne, qui fait disparaître exactement les lignes cherchées.
#[tokio::test]
async fn une_edition_sans_rien_reste_visible_a_zero() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let perimetre = commun::perimetre_de(&bac, p.detache).await;
    let ecran = edition_read::ecran(bac.pool(), &perimetre.scope)
        .await
        .expect("la liste");

    let ligne = &ecran.rows[0];
    assert_eq!(ligne.proposal_count, 0);
    assert_eq!(ligne.session_count, 0);
    assert_eq!(ligne.scheduled_session_count, 0);
    assert_eq!(ligne.day_count, 0);
    assert_eq!(ligne.call_status, None);
}

/// **La seule exception du module, et elle est écrite.** Le sélecteur d'édition
/// du back-office est *filtré* et non refusé : un périmètre vide y rend une
/// liste vide, parce que le contrat du front le veut ainsi et que c'est le store
/// qui décide alors de l'écran.
///
/// Ce test est ce qui empêche de « corriger » cette exception par symétrie avec
/// les autres lectures.
#[tokio::test]
async fn le_selecteur_est_filtre_et_non_refuse() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let p = perimetres(&bac, &editions).await;

    let globale = kernel::auth::administered_events(bac.pool(), p.globale)
        .await
        .unwrap();
    let detache = kernel::auth::administered_events(bac.pool(), p.detache)
        .await
        .unwrap();
    let vide = kernel::auth::administered_events(bac.pool(), p.sans_droit)
        .await
        .unwrap();

    let toutes = event::repo::editions::selecteur(bac.pool(), &globale)
        .await
        .expect("le sélecteur");
    assert_eq!(toutes.len(), 2);

    let sienne = event::repo::editions::selecteur(bac.pool(), &detache)
        .await
        .expect("le sélecteur");
    assert_eq!(sienne.len(), 1);
    assert_eq!(sienne[0].id, editions.sans_pavillon);

    let rien = event::repo::editions::selecteur(bac.pool(), &vide)
        .await
        .expect("une liste vide, et non un refus");
    assert!(rien.is_empty());
}
