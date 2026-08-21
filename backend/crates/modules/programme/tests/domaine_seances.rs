//! Les règles pures de la partie « séances » — **sans base**.
//!
//! Elles se prouvent seules, et c'est pour cela qu'elles vivent dans `domain/` :
//! la validation d'un formulaire configurable est la règle la plus dense du
//! module, et l'éprouver à travers une inscription réelle demanderait de monter
//! une édition, un dossier, une séance et un formulaire pour vérifier qu'un
//! nombre n'est pas un texte.

use kernel::error::ErrorCode;
use programme::domain::answers::{self, ChampResolu};
use serde_json::{json, Map, Value};

fn champ(code: &str, nature: &str, obligatoire: bool) -> ChampResolu {
    ChampResolu {
        code: code.to_owned(),
        field_type: nature.to_owned(),
        is_required: obligatoire,
        is_sensitive: false,
        options: None,
        validation: json!({}),
    }
}

fn reponses(valeurs: Value) -> Map<String, Value> {
    valeurs.as_object().expect("un document").clone()
}

/// Le refus nomme **le champ**, jamais la nature de la faute : un formulaire
/// branche sur le champ pour souligner la bonne question.
fn refus(champs: &[ChampResolu], document: Value) -> (ErrorCode, String) {
    let erreur = answers::valider(champs, &reponses(document)).expect_err("un refus était attendu");
    (
        erreur.code,
        erreur.field.expect("le champ fautif est nommé"),
    )
}

// -----------------------------------------------------------------------------
// Les six familles de refus
// -----------------------------------------------------------------------------

#[test]
fn une_reponse_obligatoire_absente_est_refusee_en_nommant_son_champ() {
    let champs = [champ("country", "country", true)];
    let (code, champ_fautif) = refus(&champs, json!({}));

    assert_eq!(code, ErrorCode::RegistrationAnswerInvalid);
    assert_eq!(champ_fautif, "country");
}

/// **Le vide vaut absence**, exactement comme la base le fait — une chaîne
/// d'espaces n'est pas une réponse.
#[test]
fn une_reponse_vide_vaut_une_absence() {
    let champs = [champ("job_title", "text", true)];
    let (_, champ_fautif) = refus(&champs, json!({ "job_title": "   " }));

    assert_eq!(champ_fautif, "job_title");
}

#[test]
fn un_type_incompatible_est_refuse() {
    let champs = [champ("participants", "number", false)];
    let (code, champ_fautif) = refus(&champs, json!({ "participants": "beaucoup" }));

    assert_eq!(code, ErrorCode::RegistrationAnswerInvalid);
    assert_eq!(champ_fautif, "participants");
}

#[test]
fn une_valeur_hors_options_est_refusee() {
    let mut source = champ("referral_source", "single_choice", false);
    source.options = Some(vec!["newsletter".to_owned(), "reseaux".to_owned()]);

    let (_, champ_fautif) = refus(&[source], json!({ "referral_source": "un ami" }));
    assert_eq!(champ_fautif, "referral_source");
}

#[test]
fn une_valeur_hors_bornes_est_refusee() {
    let mut age = champ("age", "number", false);
    age.validation = json!({ "min": 18, "max": 99 });

    let (_, champ_fautif) = refus(&[age], json!({ "age": 12 }));
    assert_eq!(champ_fautif, "age");
}

/// **Une clé inconnue est un refus, pas un silence** (FR-075) : une réponse mal
/// orthographiée qui disparaît sans un mot est une réponse perdue, et personne
/// ne s'en apercevra avant l'export.
#[test]
fn une_cle_inconnue_est_refusee_en_se_nommant() {
    let champs = [champ("job_title", "text", false)];
    let (_, champ_fautif) = refus(&champs, json!({ "fonction": "Directrice" }));

    assert_eq!(champ_fautif, "fonction");
}

/// La divergence assumée avec la base : `->>` d'un tableau vide rend `'[]'`,
/// qui n'est pas vide, et le déclencheur laisse donc passer.
#[test]
fn un_choix_multiple_obligatoire_vide_est_refuse() {
    let mut interets = champ("interets", "multiple_choice", true);
    interets.options = Some(vec!["adaptation".to_owned()]);

    let (_, champ_fautif) = refus(&[interets], json!({ "interets": [] }));
    assert_eq!(champ_fautif, "interets");
}

// -----------------------------------------------------------------------------
// Ce qui passe, et ce qui est ignoré
// -----------------------------------------------------------------------------

#[test]
fn un_document_complet_et_bien_forme_passe() {
    let mut pays = champ("country", "country", true);
    pays.options = Some(vec!["SN".to_owned(), "BR".to_owned()]);
    let mut source = champ("referral_source", "single_choice", false);
    source.options = Some(vec!["newsletter".to_owned()]);

    let document = reponses(json!({
        "country": "SN",
        "referral_source": "newsletter",
        "job_title": "Directrice",
    }));

    answers::valider(
        &[pays, source, champ("job_title", "text", false)],
        &document,
    )
    .expect("le document est valide");
}

/// Une règle qu'une version future de l'API poserait ne doit pas fermer les
/// inscriptions d'une édition entière : ignorer est le comportement sûr (R16).
#[test]
fn une_regle_de_saisie_inconnue_est_ignoree() {
    let mut fonction = champ("job_title", "text", false);
    fonction.validation = json!({ "unknownRule": 3, "maxLength": 50 });

    let document = reponses(json!({ "job_title": "Directrice" }));
    answers::valider(&[fonction], &document).expect("la règle inconnue est ignorée");
}

/// Une expression fautive est une donnée d'administrateur, pas une faute de
/// l'inscrit (R27).
#[test]
fn un_motif_invalide_est_ignore_plutot_que_de_refuser_linscrit() {
    let mut code_postal = champ("code_postal", "text", false);
    code_postal.validation = json!({ "pattern": "[" });

    let document = reponses(json!({ "code_postal": "75015" }));
    answers::valider(&[code_postal], &document).expect("le motif invalide est ignoré");
}

#[test]
fn un_motif_valide_refuse_ce_quil_ne_reconnait_pas() {
    let mut code_postal = champ("code_postal", "text", false);
    code_postal.validation = json!({ "pattern": "^[0-9]{5}$" });

    let (_, champ_fautif) = refus(&[code_postal], json!({ "code_postal": "abc" }));
    assert_eq!(champ_fautif, "code_postal");
}

/// L'écart n° 11, tranché : le code ISO à deux lettres, et lui seul.
#[test]
fn le_pays_est_un_code_iso_a_deux_lettres() {
    let mut pays = champ("country", "country", true);
    pays.options = Some(vec!["SN".to_owned()]);

    let (_, champ_fautif) = refus(&[pays.clone()], json!({ "country": "Sénégal" }));
    assert_eq!(champ_fautif, "country");

    let (_, champ_fautif) = refus(&[pays.clone()], json!({ "country": "ZZ" }));
    assert_eq!(champ_fautif, "country");

    answers::valider(&[pays], &reponses(json!({ "country": "sn" })))
        .expect("la casse d'un code ISO ne se retourne pas contre l'inscrit");
}

/// Le service a besoin de savoir **quel** champ nommer dans son refus de
/// consentement : « une donnée sensible » ne dit pas quelle case cocher.
#[test]
fn les_champs_sensibles_repondus_sont_nommes_un_par_un() {
    let mut handicap = champ("besoin_accessibilite", "text", false);
    handicap.is_sensitive = true;
    let mut regime = champ("regime_alimentaire", "text", false);
    regime.is_sensitive = true;

    let document = reponses(json!({ "besoin_accessibilite": "Boucle magnétique" }));
    let champs = [handicap, regime];
    let sensibles = answers::champs_sensibles_repondus(&champs, &document);

    assert_eq!(sensibles, vec!["besoin_accessibilite"]);
}
