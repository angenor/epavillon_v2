//! La normalisation du téléphone et celle de la base rendent la même chaîne.
//!
//! `frontend/tests/guide-nego/lexique.test.ts` passe les mêmes chaînes à
//! `normaliserTerme` : si l'un des deux côtés dérivait, un terme résolu sur le
//! téléphone ne le serait plus par `negotiation.glossary_resolve`.

mod commun;

use commun::Bac;
use serde::Deserialize;

#[derive(Deserialize)]
struct Cas {
    texte: String,
    attendu: String,
}

#[tokio::test]
async fn normalize_label_rend_les_formes_attendues_par_le_telephone() {
    let bac = Bac::monter().await;
    let cas: Vec<Cas> =
        serde_json::from_str(include_str!("fixtures/normalisation.json")).expect("fixture lisible");
    assert_eq!(cas.len(), 20);

    for c in cas {
        let forme: Option<String> = sqlx::query_scalar("SELECT platform.normalize_label($1)")
            .bind(&c.texte)
            .fetch_one(bac.pool())
            .await
            .expect("normalisation");
        // La base rend NULL pour une forme vide ; le téléphone, une chaîne vide.
        assert_eq!(forme.unwrap_or_default(), c.attendu, "« {} »", c.texte);
    }
}
