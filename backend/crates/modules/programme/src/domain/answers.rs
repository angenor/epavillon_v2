//! La validation des réponses au formulaire d'inscription — **dynamique et
//! pure** (R15, R16, R17, R18).
//!
//! # Ce que la base ne vérifie pas, et qui est donc ici
//!
//! `tg_validate_registration()` ne connaît que la **présence** d'une réponse
//! obligatoire, et seulement quand la séance porte un formulaire **attaché**
//! (écart n° 114) — alors que le formulaire applicable vient le plus souvent de
//! l'édition ou de la plateforme. Le type, les options, les bornes et les clés
//! inconnues ne sont vérifiés nulle part (écart n° 6). Ce fichier les porte, et
//! il est le seul.
//!
//! # Pur : aucune requête, aucun accès à l'état
//!
//! Il reçoit les champs **actifs** du formulaire résolu, leurs jeux d'options
//! **déjà résolus**, et le document de réponses. Le service fait deux lectures
//! avant de l'appeler ; une lecture par champ à choix serait un N+1 sur un
//! formulaire de six questions.
//!
//! # Trois décisions qui divergent de la base, toutes dans le sens STRICT
//!
//! - **Le vide vaut absence**, exactement comme `COALESCE(NEW.answers ->> f.code,
//!   '') = ''` le fait en base.
//! - **Un choix multiple obligatoire vide est refusé**, ce que la base ne voit
//!   pas : `->>` d'un tableau vide rend `'[]'`, qui n'est pas vide.
//! - **Une clé inconnue est refusée**, et non ignorée (FR-075) : une réponse mal
//!   orthographiée qui disparaît sans un mot est une réponse perdue, et personne
//!   ne s'en apercevra avant l'export.
//!
//! Diverger dans le sens strict est sans risque — l'API refuse avant la base.
//! Diverger dans l'autre produirait un refus du déclencheur que le service
//! n'aurait pas su expliquer.

use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::{Map, Value};

/// Un champ du formulaire, **ses options déjà résolues**.
///
/// `options` porte les valeurs admises quelle que soit leur origine : liste
/// explicite du formulaire, codes d'une taxonomie, ou codes ISO des pays. Un
/// seul mécanisme de comparaison, donc une seule occasion de diverger.
#[derive(Debug, Clone)]
pub struct ChampResolu {
    pub code: String,
    /// Valeur de `programme.form_field_type`, telle que la base la nomme.
    pub field_type: String,
    pub is_required: bool,
    pub is_sensitive: bool,
    /// `None` quand le champ n'est pas à choix. `Some(vide)` est un champ à
    /// choix dont la taxonomie ne rend rien : **toute valeur y sera refusée**,
    /// et c'est le comportement voulu — la question n'a pas de réponse valide.
    pub options: Option<Vec<String>>,
    /// `minLength`, `maxLength`, `pattern`, `min`, `max`. Toute autre clé est
    /// **ignorée avec une trace** (R16).
    pub validation: Value,
}

/// Les cinq clés de `validation` que l'API honore.
///
/// Ces règles sont des **données écrites par un administrateur** depuis le
/// back-office : refuser un formulaire porteur d'une clé qu'une version future
/// ne connaît pas encore fermerait les inscriptions d'une édition entière au
/// premier déploiement décalé. Ignorer est le comportement sûr.
const REGLES_HONOREES: [&str; 5] = ["minLength", "maxLength", "pattern", "min", "max"];

fn refuser(code: &str, message: impl Into<String>) -> ApiError {
    ApiError::with_message(ErrorCode::RegistrationAnswerInvalid, message).field(code)
}

/// Valider un document de réponses contre les champs actifs d'un formulaire.
///
/// L'ordre est celui du formulaire : la première faute rencontrée est rendue,
/// et c'est le champ qu'elle nomme que l'écran souligne.
pub fn valider(champs: &[ChampResolu], reponses: &Map<String, Value>) -> Result<()> {
    refuser_les_cles_inconnues(champs, reponses)?;

    for champ in champs {
        let valeur = reponses.get(&champ.code);

        if est_absente(champ, valeur) {
            if champ.is_required {
                return Err(refuser(&champ.code, "Cette réponse est obligatoire."));
            }
            continue;
        }

        let valeur = valeur.expect("une réponse jugée présente existe");
        verifier_le_type(champ, valeur)?;
        verifier_les_options(champ, valeur)?;
        verifier_les_regles(champ, valeur)?;
    }

    Ok(())
}

/// **La clé inconnue est un refus, pas un silence** (FR-075).
fn refuser_les_cles_inconnues(champs: &[ChampResolu], reponses: &Map<String, Value>) -> Result<()> {
    for cle in reponses.keys() {
        if !champs.iter().any(|c| &c.code == cle) {
            return Err(refuser(
                cle,
                format!("La question « {cle} » n'existe pas dans ce formulaire."),
            ));
        }
    }
    Ok(())
}

/// Une réponse absente, nulle, ou **vide au sens de la base**.
///
/// Un choix multiple fait exception : un tableau vide est bien une absence de
/// réponse, alors que `->>` en base rend `'[]'` et le laisse passer. C'est la
/// divergence assumée de R17.
fn est_absente(champ: &ChampResolu, valeur: Option<&Value>) -> bool {
    match valeur {
        None | Some(Value::Null) => true,
        Some(Value::String(texte)) => texte.trim().is_empty(),
        Some(Value::Array(elements)) if champ.field_type == "multiple_choice" => {
            elements.is_empty()
        }
        _ => false,
    }
}

/// Les onze natures de champ, et ce que chacune attend.
fn verifier_le_type(champ: &ChampResolu, valeur: &Value) -> Result<()> {
    let code = champ.code.as_str();

    match champ.field_type.as_str() {
        "text" | "long_text" | "phone" | "single_choice" | "taxonomy_term" => {
            exiger_une_chaine(champ, valeur)?;
        }
        "email" => {
            let texte = exiger_une_chaine(champ, valeur)?;
            if !ressemble_a_une_adresse(texte) {
                return Err(refuser(
                    code,
                    "Cette valeur n'est pas une adresse électronique valide.",
                ));
            }
        }
        "number" => {
            if !valeur.is_number() {
                return Err(refuser(code, "Cette valeur n'est pas un nombre."));
            }
        }
        "boolean" => {
            if !valeur.is_boolean() {
                return Err(refuser(code, "Cette valeur n'est ni « oui » ni « non »."));
            }
        }
        "date" => {
            let texte = exiger_une_chaine(champ, valeur)?;
            if !est_une_date(texte) {
                return Err(refuser(code, "Cette valeur n'est pas une date valide."));
            }
        }
        "country" => {
            let texte = exiger_une_chaine(champ, valeur)?;
            // R18 : le code ISO 3166-1 alpha-2, lisible dans un export et stable
            // si la fiche d'un pays est refaite. Un nom de pays en toutes
            // lettres est refusé — un export mêlant les deux formes serait
            // irrécupérable (écart n° 11).
            if texte.trim().len() != 2 {
                return Err(refuser(
                    code,
                    "Le pays s'exprime par son code à deux lettres (par exemple « SN »).",
                ));
            }
        }
        "multiple_choice" => match valeur {
            Value::Array(elements) => {
                for element in elements {
                    if !element.is_string() {
                        return Err(refuser(
                            code,
                            "Cette valeur ne fait pas partie des choix proposés.",
                        ));
                    }
                }
            }
            _ => {
                return Err(refuser(code, "Cette question attend plusieurs réponses."));
            }
        },
        // Une nature inconnue est une donnée d'un modèle plus récent que ce
        // code : on ne refuse pas l'inscrit pour cela, et la trace le dit.
        autre => {
            tracing::warn!(champ = code, nature = autre, "nature de champ inconnue");
        }
    }

    Ok(())
}

fn exiger_une_chaine<'v>(champ: &ChampResolu, valeur: &'v Value) -> Result<&'v str> {
    valeur
        .as_str()
        .ok_or_else(|| refuser(&champ.code, "Cette valeur n'est pas un texte."))
}

/// Une adresse électronique **au sens de ce que l'écran doit refuser**, et non
/// au sens de la RFC : `platform.email` porte le contrôle sérieux en base.
fn ressemble_a_une_adresse(texte: &str) -> bool {
    let texte = texte.trim();
    match texte.split_once('@') {
        Some((avant, apres)) => {
            !avant.is_empty()
                && apres.contains('.')
                && !apres.starts_with('.')
                && !apres.ends_with('.')
                && !apres.contains(' ')
        }
        None => false,
    }
}

/// Une date au format ISO `AAAA-MM-JJ` — celui que le formulaire envoie.
fn est_une_date(texte: &str) -> bool {
    time::Date::parse(
        texte.trim(),
        &time::format_description::well_known::Iso8601::DATE,
    )
    .is_ok()
}

/// L'appartenance aux options, pour les trois natures qui en portent.
///
/// **Le pays suit la même règle**, ses options étant les codes ISO du
/// référentiel : c'est ce qui évite deux mécanismes de comparaison, donc deux
/// occasions de diverger.
fn verifier_les_options(champ: &ChampResolu, valeur: &Value) -> Result<()> {
    let Some(admises) = &champ.options else {
        return Ok(());
    };

    let hors_choix = || {
        refuser(
            &champ.code,
            "Cette valeur ne fait pas partie des choix proposés.",
        )
    };

    match valeur {
        Value::String(texte) => {
            if !admise(admises, texte, champ) {
                return Err(hors_choix());
            }
        }
        Value::Array(elements) => {
            for element in elements {
                let texte = element.as_str().ok_or_else(hors_choix)?;
                if !admise(admises, texte, champ) {
                    return Err(hors_choix());
                }
            }
        }
        _ => return Err(hors_choix()),
    }

    Ok(())
}

/// Le code d'un pays se compare **en majuscules** : « sn » et « SN » désignent
/// le Sénégal, et refuser la casse serait un piège de saisie. Partout ailleurs,
/// le code est celui d'une taxonomie et se compare tel quel.
fn admise(admises: &[String], valeur: &str, champ: &ChampResolu) -> bool {
    let valeur = valeur.trim();
    if champ.field_type == "country" {
        let majuscules = valeur.to_uppercase();
        admises.iter().any(|a| a.eq_ignore_ascii_case(&majuscules))
    } else {
        admises.iter().any(|a| a == valeur)
    }
}

/// Les cinq règles de saisie honorées. **Toute autre clé est ignorée avec une
/// trace**, et une expression régulière fautive aussi : c'est une donnée
/// d'administrateur, pas une faute de l'inscrit (R16, R27).
fn verifier_les_regles(champ: &ChampResolu, valeur: &Value) -> Result<()> {
    let Some(regles) = champ.validation.as_object() else {
        return Ok(());
    };

    for cle in regles.keys() {
        if !REGLES_HONOREES.contains(&cle.as_str()) {
            tracing::warn!(
                champ = %champ.code,
                regle = %cle,
                "règle de saisie inconnue, ignorée"
            );
        }
    }

    if let Some(texte) = valeur.as_str() {
        let longueur = texte.chars().count() as u64;

        if let Some(mini) = regles.get("minLength").and_then(Value::as_u64) {
            if longueur < mini {
                return Err(refuser(
                    &champ.code,
                    format!("Cette réponse doit compter au moins {mini} caractères."),
                ));
            }
        }
        if let Some(maxi) = regles.get("maxLength").and_then(Value::as_u64) {
            if longueur > maxi {
                return Err(refuser(
                    &champ.code,
                    format!("Cette réponse ne doit pas dépasser {maxi} caractères."),
                ));
            }
        }
        if let Some(motif) = regles.get("pattern").and_then(Value::as_str) {
            match regex::Regex::new(motif) {
                Ok(expression) if !expression.is_match(texte) => {
                    return Err(refuser(
                        &champ.code,
                        "Cette réponse n'a pas le format attendu.",
                    ));
                }
                Err(erreur) => tracing::warn!(
                    champ = %champ.code,
                    erreur = %erreur,
                    "motif de validation invalide, ignoré"
                ),
                _ => {}
            }
        }
    }

    if let Some(nombre) = valeur.as_f64() {
        let mini = regles.get("min").and_then(Value::as_f64);
        let maxi = regles.get("max").and_then(Value::as_f64);

        match (mini, maxi) {
            (Some(a), Some(b)) if nombre < a || nombre > b => {
                return Err(refuser(
                    &champ.code,
                    format!("La valeur doit être comprise entre {a} et {b}."),
                ));
            }
            (Some(a), None) if nombre < a => {
                return Err(refuser(
                    &champ.code,
                    format!("La valeur ne peut pas être inférieure à {a}."),
                ));
            }
            (None, Some(b)) if nombre > b => {
                return Err(refuser(
                    &champ.code,
                    format!("La valeur ne peut pas dépasser {b}."),
                ));
            }
            _ => {}
        }
    }

    Ok(())
}

/// Les champs **sensibles auxquels une réponse est donnée**.
///
/// `is_sensitive` est une marque sans effet en base : c'est le service qui en
/// tire le consentement, et il a besoin de savoir **quel** champ nommer dans le
/// refus (R22).
pub fn champs_sensibles_repondus<'c>(
    champs: &'c [ChampResolu],
    reponses: &Map<String, Value>,
) -> Vec<&'c str> {
    champs
        .iter()
        .filter(|c| c.is_sensitive && !est_absente(c, reponses.get(&c.code)))
        .map(|c| c.code.as_str())
        .collect()
}
