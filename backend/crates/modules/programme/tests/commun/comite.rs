//! La fabrique de l'évaluation : un dossier confié à deux membres du comité.

#![allow(dead_code)]

use kernel::auth::Perimeter;
use programme::service::review::SaveReviewPayload;
use std::collections::BTreeMap;
use uuid::Uuid;

use super::{Bac, Terrain};

pub struct Comite {
    pub dossier: Uuid,
    /// Affectée, n'a pas encore noté : **le voile est baissé pour elle**.
    pub premiere: Perimeter,
    /// Affectée, a déposé sa revue : elle n'ancre plus personne.
    pub seconde: Perimeter,
    /// Décide sans noter, **donc n'est pas affectée, donc pas voilée**.
    pub decideur: Perimeter,
    pub criteres: Vec<(Uuid, f64, bool)>,
}

pub async fn perimetre(bac: &Bac, personne: Uuid) -> Perimeter {
    super::perimetre_de(bac, personne).await
}

pub async fn noteur(bac: &Bac, terrain: &Terrain, courriel: &str, prenom: &str) -> Uuid {
    let personne = super::personne(bac, courriel, prenom, "Comite").await;
    super::attribuer(bac, personne, "reviewer", "event", Some(terrain.edition)).await;
    sqlx::query!(
        "INSERT INTO event.call_reviewers (call_id, person_id) VALUES ($1, $2)",
        terrain.appel,
        personne
    )
    .execute(bac.pool())
    .await
    .expect("inscription au comité");
    personne
}

pub async fn confier(bac: &Bac, dossier: Uuid, membre: Uuid) {
    sqlx::query!(
        "INSERT INTO programme.review_assignments (proposal_id, reviewer_id) VALUES ($1, $2)",
        dossier,
        membre
    )
    .execute(bac.pool())
    .await
    .expect("affectation");
}

/// La grille par défaut de l'appel : six critères, dont un éliminatoire.
pub async fn criteres(bac: &Bac, appel: Uuid) -> Vec<(Uuid, f64, bool)> {
    sqlx::query!(
        r#"SELECT id, max_score::float8 AS "max!", is_knockout
             FROM event.review_criteria WHERE call_id = $1 ORDER BY sort_order, code"#,
        appel
    )
    .fetch_all(bac.pool())
    .await
    .expect("grille de l'appel")
    .into_iter()
    .map(|l| (l.id, l.max, l.is_knockout))
    .collect()
}

pub async fn comite(bac: &Bac, terrain: &Terrain) -> Comite {
    let dossier = super::dossier(bac, terrain, "Atelier adaptation", "atelier-adaptation").await;
    let premiere = noteur(bac, terrain, "premiere@ifdd.francophonie.org", "Prisca").await;
    let seconde = noteur(bac, terrain, "seconde@ifdd.francophonie.org", "Sophie").await;
    let decideur = super::personne(bac, "decideur@ifdd.francophonie.org", "Denis", "Kabore").await;
    super::attribuer(bac, decideur, "admin", "event", Some(terrain.edition)).await;

    confier(bac, dossier, premiere).await;
    confier(bac, dossier, seconde).await;

    Comite {
        dossier,
        premiere: perimetre(bac, premiere).await,
        seconde: perimetre(bac, seconde).await,
        decideur: perimetre(bac, decideur).await,
        criteres: criteres(bac, terrain.appel).await,
    }
}

/// Une charge utile de notation qui pose la même note sur chaque critère.
pub fn notation(criteres: &[(Uuid, f64, bool)], part: f64, deposer: bool) -> SaveReviewPayload {
    let mut scores = BTreeMap::new();
    for (id, max, _) in criteres {
        scores.insert(*id, (max * part * 100.0).round() / 100.0);
    }

    SaveReviewPayload {
        recommendation: "accept".to_owned(),
        mode: "detailed".to_owned(),
        score_out_of_20: None,
        comment: None,
        scores,
        comments: BTreeMap::new(),
        strengths: Some("Un sujet bien cadré.".to_owned()),
        weaknesses: None,
        private_note: Some("À suivre en séance.".to_owned()),
        submit: deposer,
    }
}

/// Une notation rapide : une note sur 20, aucun critère.
pub fn notation_rapide(note: Option<f64>, deposer: bool) -> SaveReviewPayload {
    SaveReviewPayload {
        recommendation: "neutral".to_owned(),
        mode: "quick".to_owned(),
        score_out_of_20: note,
        comment: Some("Sujet solide, intervenants à confirmer.".to_owned()),
        scores: BTreeMap::new(),
        comments: BTreeMap::new(),
        strengths: None,
        weaknesses: None,
        private_note: None,
        submit: deposer,
    }
}
