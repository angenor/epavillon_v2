//! **La santé est rendue par le CODE de l'indicateur, seuils NON recalculés.**
//!
//! Le modèle porte déjà la décision de ce qui mérite attention ; la redoubler en
//! Rust ferait deux vérités, et la première divergence passerait inaperçue.

mod commun;

use commun::*;

#[tokio::test]
async fn chaque_indicateur_porte_son_code_et_ses_deux_seuils() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    rafraichir(&bac).await;

    let sante = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .health;

    assert!(!sante.is_empty());
    for ligne in &sante {
        assert!(
            !ligne.code.is_empty(),
            "le CODE, le libellé n'étant qu'un repli"
        );
        assert!(!ligne.libelle.is_empty());
        assert!(!ligne.domaine.is_empty());
        assert!(ligne.seuil_attention <= ligne.seuil_critique);
        assert!(matches!(
            ligne.gravite.as_str(),
            "ok" | "attention" | "critique"
        ));
    }

    // La gravité est celle que la VUE calcule, jamais une recomposition.
    for ligne in &sante {
        let attendue = if ligne.valeur >= ligne.seuil_critique {
            "critique"
        } else if ligne.valeur >= ligne.seuil_attention {
            "attention"
        } else {
            "ok"
        };
        assert_eq!(
            ligne.gravite, attendue,
            "la gravité de `{}` vient de la base",
            ligne.code
        );
    }
}

#[tokio::test]
async fn la_fraicheur_analytique_figure_parmi_les_indicateurs() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    rafraichir(&bac).await;

    let sante = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .health;

    assert!(
        sante.iter().any(|l| l.code == "analytique_perimee"),
        "c'est l'indicateur qui plafonne l'intervalle de rafraîchissement à deux heures"
    );
}
