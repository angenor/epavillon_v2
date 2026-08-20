//! L'appel à propositions et sa grille — les formes du contrat, et le **diff de
//! grille** qui décide ce qui s'insère, se met à jour ou se supprime.
//!
//! **Aucun nom de champ n'est renégocié** : leur source unique est
//! `frontend/app/types/admin-events.ts`, § 3.5.
//!
//! Le diff se fait **par code** et jamais par identifiant : une ligne ajoutée à
//! l'écran n'a pas encore d'identifiant — le contrat le dit (`id: CriterionId |
//! null`) — et le code est la seule clé stable d'un critère au sein de son appel
//! (`ux_review_criteria`).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use super::detail::EditionCall;

/// Une ligne de grille telle que le formulaire l'envoie — `EditionCriterion`.
///
/// `score_count` n'y figure pas : c'est un décompte joint, rendu par la lecture
/// et ignoré à l'écriture. Le front l'envoie tout de même ; `serde` l'écarte.
#[derive(Debug, Clone, Deserialize)]
pub struct CriterionPayload {
    /// Nul pour une ligne ajoutée et pas encore enregistrée.
    #[serde(default)]
    pub id: Option<Uuid>,
    pub code: String,
    pub label: Value,
    #[serde(default)]
    pub description: Option<Value>,
    pub max_score: f64,
    pub weight: f64,
    pub is_knockout: bool,
    pub sort_order: i16,
}

/// Ce que le formulaire de l'appel envoie — `EditionCallPayload`, **grille
/// comprise**.
///
/// La grille part avec l'appel et non par un appel séparé : un appel sans
/// critère ne peut recevoir aucune évaluation, et deux enregistrements distincts
/// laisseraient exister cet état le temps d'un oubli (research.md § R9).
#[derive(Debug, Clone, Deserialize)]
pub struct EditionCallPayload {
    /// Nul à la création. **Ignoré à la modification** : l'identifiant fait foi
    /// dans l'adresse, jamais dans le corps.
    #[serde(default)]
    pub id: Option<Uuid>,
    /// **Ignoré.** L'édition vient de l'ascendance de l'appel à la modification,
    /// et du corps *vérifié* à la création (research.md § R2).
    pub event_id: Uuid,
    pub code: String,
    pub title: Value,
    #[serde(default)]
    pub description: Option<Value>,
    pub status: String,
    #[serde(with = "time::serde::rfc3339")]
    pub opens_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub closes_at: OffsetDateTime,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub extended_until: Option<OffsetDateTime>,
    #[serde(default)]
    pub results_expected_at: Option<Date>,
    #[serde(default)]
    pub max_proposals_per_organization: Option<i16>,
    pub requires_verified_organization: bool,
    pub min_speakers: i16,
    pub max_speakers: i16,
    pub default_duration_minutes: i16,
    pub min_duration_minutes: i16,
    pub max_duration_minutes: i16,
    /// `HH:MM` ou `HH:MM:SS`, en heure **locale de l'édition** — jamais
    /// convertie : c'est l'heure d'ouverture d'un stand, pas un instant.
    pub daily_start_time: String,
    pub daily_end_time: String,
    pub allowed_formats: Vec<String>,
    pub required_reviews: i16,
    pub blind_review: bool,
    #[serde(default)]
    pub guidelines_url: Option<String>,
    #[serde(default)]
    pub criteria: Vec<CriterionPayload>,
}

/// Contraintes nommées de `event.calls_for_proposals`, telles qu'elles refusent
/// — `CallErrorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CallErrorCode {
    /// `ck_calls_window` — la clôture doit suivre l'ouverture.
    Window,
    /// `ck_calls_extension` — une prolongation dépasse la clôture initiale.
    Extension,
    /// `ck_calls_speakers` — le maximum doit atteindre le minimum.
    Speakers,
    /// `ck_calls_duration_bounds` — **trois conditions sous un seul nom**.
    DurationBounds,
    /// `ck_calls_daily_window` — la fermeture du pavillon suit son ouverture.
    DailyWindow,
    /// `ux_calls_one_per_event` — cette édition porte déjà un appel non annulé.
    AlreadyExists,
    /// `ux_calls_code`.
    CodeTaken,
    /// Règle du service : une grille vide n'évalue rien.
    CriteriaEmpty,
    /// `ux_review_criteria`, **anticipée pour nommer le rang** de la ligne.
    CriterionCodeDuplicate,
    /// Champ obligatoire non renseigné.
    Required,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallFormError {
    pub code: CallErrorCode,
    /// Champ du formulaire à marquer. `null` quand le refus porte sur l'appel.
    pub field: Option<String>,
    /// **Rang** de la ligne de grille fautive, quand le refus porte sur elle.
    pub criterion_index: Option<usize>,
}

impl CallFormError {
    pub fn champ(code: CallErrorCode, field: &str) -> Self {
        Self {
            code,
            field: Some(field.to_owned()),
            criterion_index: None,
        }
    }

    pub fn globale(code: CallErrorCode) -> Self {
        Self {
            code,
            field: None,
            criterion_index: None,
        }
    }

    pub fn ligne(code: CallErrorCode, rang: usize) -> Self {
        Self {
            code,
            field: Some("criteria".to_owned()),
            criterion_index: Some(rang),
        }
    }
}

/// La réponse d'un enregistrement d'appel — `CallSaveResult`.
#[derive(Debug, Clone, Serialize)]
pub struct CallSaveResult {
    pub ok: bool,
    pub call: Option<EditionCall>,
    pub errors: Vec<CallFormError>,
    /// **Un barème modifié déplace des moyennes déjà calculées.** Les notes ne
    /// sont pas perdues, mais `refresh_proposal_score()` les repondère, et un
    /// classement qui bouge sans explication est une conversation difficile avec
    /// le comité (research.md § R9).
    pub scores_affected: bool,
}

impl CallSaveResult {
    pub fn refuse(errors: Vec<CallFormError>) -> Self {
        Self {
            ok: false,
            call: None,
            errors,
            scores_affected: false,
        }
    }
}

// -----------------------------------------------------------------------------
// Le diff de grille
// -----------------------------------------------------------------------------

/// Un critère **tel qu'il existe déjà**, avec ce qui décide de son sort.
#[derive(Debug, Clone)]
pub struct CritereExistant {
    pub id: Uuid,
    pub code: String,
    /// Le libellé multilingue : ce qu'un refus doit **nommer**, plutôt qu'un
    /// identifiant que personne ne reconnaît.
    pub label: Value,
    pub max_score: f64,
    pub weight: f64,
    /// Notes posées : ce qui **interdit** le retrait et ce qui rend une
    /// modification de barème notable.
    pub score_count: i64,
}

impl CritereExistant {
    /// Le libellé français, ou le code à défaut : ce qu'un message de refus
    /// donne à lire. Repli sur l'anglais puis sur le code — un refus sans nom
    /// oblige à ouvrir la base pour savoir de quelle ligne on parle.
    pub fn nom(&self) -> String {
        self.label
            .get("fr")
            .and_then(Value::as_str)
            .or_else(|| self.label.get("en").and_then(Value::as_str))
            .unwrap_or(&self.code)
            .to_owned()
    }
}

/// Ce que l'enregistrement de la grille va faire.
#[derive(Debug, Clone, Default)]
pub struct DiffGrille {
    /// Codes absents en base : à insérer.
    pub a_inserer: Vec<usize>,
    /// Codes présents des deux côtés : `(identifiant, rang dans la charge)`.
    pub a_modifier: Vec<(Uuid, usize)>,
    /// Codes disparus de la charge utile : à supprimer, **ou à refuser**.
    pub a_supprimer: Vec<CritereExistant>,
    /// Un critère **conservé** dont le barème ou le poids change et qui porte
    /// déjà des notes.
    pub scores_affected: bool,
}

/// Le diff, **par code**.
///
/// Les rangs et non les valeurs : la charge utile reste la source, et la copier
/// ici ferait deux endroits où lire un critère.
pub fn diff(charge: &[CriterionPayload], existants: &[CritereExistant]) -> DiffGrille {
    let par_code: HashMap<&str, &CritereExistant> =
        existants.iter().map(|c| (c.code.as_str(), c)).collect();

    let mut resultat = DiffGrille::default();

    for (rang, ligne) in charge.iter().enumerate() {
        match par_code.get(ligne.code.as_str()) {
            None => resultat.a_inserer.push(rang),
            Some(existant) => {
                resultat.a_modifier.push((existant.id, rang));
                let bareme_change = !memes_centiemes(existant.max_score, ligne.max_score)
                    || !memes_centiemes(existant.weight, ligne.weight);
                if bareme_change && existant.score_count > 0 {
                    resultat.scores_affected = true;
                }
            }
        }
    }

    let conserves: Vec<&str> = charge.iter().map(|l| l.code.as_str()).collect();
    resultat.a_supprimer = existants
        .iter()
        .filter(|e| !conserves.contains(&e.code.as_str()))
        .cloned()
        .collect();

    resultat
}

/// Les barèmes sont des `numeric(5,2)` : les comparer au flottant près ferait
/// signaler un changement là où la base n'en verrait aucun.
fn memes_centiemes(a: f64, b: f64) -> bool {
    (a * 100.0).round() as i64 == (b * 100.0).round() as i64
}

/// Le **rang** du premier code apparaissant deux fois, s'il y en a un.
///
/// Le service dédoublonne en amont plutôt que de laisser `ux_review_criteria`
/// refuser : l'index ne dit pas *quelle ligne* de l'écran est en cause, et le
/// contrat du front attend ce rang (`criterion_index`).
pub fn premier_code_en_double(charge: &[CriterionPayload]) -> Option<usize> {
    let mut vus: HashMap<&str, usize> = HashMap::new();

    for (rang, ligne) in charge.iter().enumerate() {
        if vus.insert(ligne.code.as_str(), rang).is_some() {
            return Some(rang);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn critere(code: &str, max_score: f64, weight: f64) -> CriterionPayload {
        CriterionPayload {
            id: None,
            code: code.to_owned(),
            label: json!({ "fr": code }),
            description: None,
            max_score,
            weight,
            is_knockout: false,
            sort_order: 10,
        }
    }

    fn existant(code: &str, max_score: f64, weight: f64, notes: i64) -> CritereExistant {
        CritereExistant {
            id: Uuid::now_v7(),
            code: code.to_owned(),
            label: json!({ "fr": code }),
            max_score,
            weight,
            score_count: notes,
        }
    }

    /// Une ligne ajoutée à l'écran n'a **pas d'identifiant** : c'est son code
    /// qui la range du côté des insertions.
    #[test]
    fn une_ligne_nouvelle_sans_identifiant_sinsere() {
        let d = diff(&[critere("relevance", 5.0, 2.0)], &[]);
        assert_eq!(d.a_inserer, vec![0]);
        assert!(d.a_modifier.is_empty());
        assert!(d.a_supprimer.is_empty());
    }

    #[test]
    fn un_code_disparu_de_la_charge_se_supprime() {
        let base = [
            existant("relevance", 5.0, 2.0, 0),
            existant("impact", 5.0, 1.5, 0),
        ];
        let d = diff(&[critere("relevance", 5.0, 2.0)], &base);
        assert_eq!(d.a_supprimer.len(), 1);
        assert_eq!(d.a_supprimer[0].code, "impact");
    }

    /// **Un barème modifié sur un critère porteur de notes se signale.** Les
    /// notes ne bougent pas ; les moyennes, si.
    #[test]
    fn un_bareme_modifie_sur_un_critere_note_se_signale() {
        let base = [existant("relevance", 5.0, 2.0, 3)];
        let d = diff(&[critere("relevance", 10.0, 2.0)], &base);
        assert!(d.scores_affected);
    }

    #[test]
    fn un_poids_modifie_sur_un_critere_note_se_signale() {
        let base = [existant("relevance", 5.0, 2.0, 3)];
        let d = diff(&[critere("relevance", 5.0, 3.0)], &base);
        assert!(d.scores_affected);
    }

    /// Sans note, un barème modifié ne déplace aucune moyenne : rien à annoncer.
    #[test]
    fn un_bareme_modifie_sans_note_ne_signale_rien() {
        let base = [existant("relevance", 5.0, 2.0, 0)];
        let d = diff(&[critere("relevance", 10.0, 2.0)], &base);
        assert!(!d.scores_affected);
    }

    #[test]
    fn un_bareme_inchange_ne_signale_rien() {
        let base = [existant("relevance", 5.0, 2.0, 7)];
        let d = diff(&[critere("relevance", 5.0, 2.0)], &base);
        assert!(!d.scores_affected);
        assert_eq!(d.a_modifier.len(), 1);
    }

    #[test]
    fn le_rang_du_code_en_double_est_celui_de_la_seconde_ligne() {
        let charge = [
            critere("relevance", 5.0, 2.0),
            critere("impact", 5.0, 1.5),
            critere("relevance", 5.0, 1.0),
        ];
        assert_eq!(premier_code_en_double(&charge), Some(2));
    }

    #[test]
    fn une_grille_sans_doublon_ne_designe_aucun_rang() {
        let charge = [critere("relevance", 5.0, 2.0), critere("impact", 5.0, 1.5)];
        assert_eq!(premier_code_en_double(&charge), None);
    }
}
