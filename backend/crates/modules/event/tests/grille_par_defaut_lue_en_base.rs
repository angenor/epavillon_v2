//! **La grille par défaut est lue en base, jamais recopiée** (FR-062, SC-010).
//!
//! `event.seed_default_criteria()` porte les six critères, leurs libellés
//! bilingues, leurs poids et l'unique éliminatoire. Les recopier dans un tableau
//! Rust en ferait une seconde vérité, désynchronisée du modèle au premier
//! ajustement — le défaut n° 1 de la v1 appliqué à une grille d'évaluation.
//!
//! Ce test compare la route au **semis lui-même**, ligne par ligne : il ne
//! réécrit aucune des six valeurs attendues, sans quoi il aurait exactement le
//! défaut qu'il vérifie.

mod commun;

use commun::Bac;
use event::repo::criteria;

/// Le semis, exécuté à part : c'est la référence, et elle vient de la base.
async fn semis_de_reference(bac: &Bac) -> Vec<(String, serde_json::Value, f64, f64, bool)> {
    let editions = commun::seed::editions(bac).await;

    let appel = sqlx::query_scalar!(
        r#"INSERT INTO event.calls_for_proposals
               (event_id, code, title, opens_at, closes_at)
           VALUES ($1, 'reference', '{"fr":"Référence"}'::jsonb,
                   now(), now() + interval '10 days')
        RETURNING id"#,
        editions.cop31
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'appel de référence");

    sqlx::query!("SELECT event.seed_default_criteria($1)", appel)
        .execute(bac.pool())
        .await
        .expect("semis de la grille");

    sqlx::query!(
        r#"SELECT code, label, max_score::float8 AS "max_score!",
                  weight::float8 AS "weight!", is_knockout
             FROM event.review_criteria WHERE call_id = $1
            ORDER BY sort_order, code"#,
        appel
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de la grille de référence")
    .into_iter()
    .map(|l| (l.code, l.label, l.max_score, l.weight, l.is_knockout))
    .collect()
}

#[tokio::test]
async fn la_grille_servie_est_exactement_celle_que_le_modele_seme() {
    let bac = Bac::monter().await;

    let reference = semis_de_reference(&bac).await;
    let servie = criteria::grille_par_defaut(bac.pool())
        .await
        .expect("lecture de la grille par défaut");

    assert_eq!(reference.len(), 6, "le modèle sème six critères");
    assert_eq!(servie.len(), reference.len());

    for (attendu, rendu) in reference.iter().zip(&servie) {
        assert_eq!(rendu.code, attendu.0);
        assert_eq!(rendu.label, attendu.1, "libellé de « {} »", attendu.0);
        assert_eq!(rendu.max_score, attendu.2);
        assert_eq!(rendu.weight, attendu.3);
        assert_eq!(rendu.is_knockout, attendu.4);
    }

    // Les libellés sont **bilingues** : un écran anglophone ne doit pas recevoir
    // du français par défaut.
    for critere in &servie {
        assert!(
            critere.label.get("fr").is_some() && critere.label.get("en").is_some(),
            "le critère « {} » doit porter ses deux langues",
            critere.code
        );
    }

    assert_eq!(
        servie.iter().filter(|c| c.is_knockout).count(),
        1,
        "un seul critère éliminatoire : une note nulle y disqualifie"
    );
}

/// **Les lignes proposées sont NOUVELLES.** Elles viennent d'une transaction
/// annulée : rendre leurs identifiants ferait croire à l'écran qu'elles
/// existent, et l'enregistrement suivant tenterait de les modifier.
#[tokio::test]
async fn les_lignes_proposees_nont_pas_didentifiant() {
    let bac = Bac::monter().await;

    let servie = criteria::grille_par_defaut(bac.pool())
        .await
        .expect("lecture de la grille par défaut");

    assert!(servie.iter().all(|c| c.id.is_none()));
}

/// **Rien ne subsiste de l'appel jetable.** La lecture écrit pour lire ; si sa
/// transaction n'était pas annulée, chaque affichage du formulaire laisserait
/// une édition fantôme dans la liste du back-office.
#[tokio::test]
async fn la_lecture_ne_laisse_ni_edition_ni_appel_derriere_elle() {
    let bac = Bac::monter().await;

    let editions_avant = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM event.events"#)
        .fetch_one(bac.pool())
        .await
        .unwrap();

    criteria::grille_par_defaut(bac.pool())
        .await
        .expect("lecture de la grille par défaut");

    let editions_apres = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM event.events"#)
        .fetch_one(bac.pool())
        .await
        .unwrap();
    let appels = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM event.calls_for_proposals"#)
        .fetch_one(bac.pool())
        .await
        .unwrap();
    let criteres = sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM event.review_criteria"#)
        .fetch_one(bac.pool())
        .await
        .unwrap();

    assert_eq!(editions_apres, editions_avant, "aucune édition fantôme");
    assert_eq!(appels, 0, "aucun appel fantôme");
    assert_eq!(criteres, 0, "aucune ligne de grille fantôme");
}
