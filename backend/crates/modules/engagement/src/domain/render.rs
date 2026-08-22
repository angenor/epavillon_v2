//! La substitution de variables nommées, **pure** — et rien d'autre.
//!
//! `{{variable}}`, remplacement littéral. Aucune condition, aucune boucle,
//! aucun filtre. Un langage de gabarit complet serait une dépendance d'ampleur
//! pour des courriels transactionnels de dix lignes.
//!
//! # Une variable manquante FAIT ÉCHOUER l'envoi, en la nommant
//!
//! Le modèle l'écrit lui-même, en commentaire de `template_versions.variables` :
//! *« le worker refuse le rendu si une variable manque : mieux vaut un job en
//! échec visible qu'un email “Bonjour  ,” envoyé à 2 000 personnes »*.
//!
//! Un travail en échec porte son message dans `platform.jobs.last_error` et se
//! reprend ; un courriel amputé est parti pour toujours.

use std::collections::{BTreeSet, HashMap};

/// Ce que le rendu a refusé de faire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErreurRendu {
    /// La variable est citée par le gabarit et absente à l'exécution. Le nom
    /// voyage : c'est la seule information qui permette de corriger.
    VariableManquante(String),
}

impl std::fmt::Display for ErreurRendu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VariableManquante(nom) => {
                write!(f, "variable « {nom} » absente des valeurs fournies")
            }
        }
    }
}

/// Les variables qu'un gabarit cite, dédoublonnées et rangées.
///
/// Sert à deux choses : refuser une publication qui citerait une variable que
/// le type ne promet pas (FR-083), et prévenir avant l'envoi plutôt qu'après.
pub fn variables_citees(gabarit: &str) -> BTreeSet<String> {
    let mut citees = BTreeSet::new();
    let mut reste = gabarit;
    while let Some(debut) = reste.find("{{") {
        let apres = &reste[debut + 2..];
        let Some(fin) = apres.find("}}") else { break };
        let nom = apres[..fin].trim();
        if est_un_nom(nom) {
            citees.insert(nom.to_owned());
        }
        reste = &apres[fin + 2..];
    }
    citees
}

/// Le rendu. Échoue en nommant la **première** variable manquante, dans l'ordre
/// alphabétique — un ordre stable rend le message reproductible d'un essai à
/// l'autre, ce qu'un ordre d'apparition ne garantit pas quand le gabarit change.
pub fn rendre(gabarit: &str, valeurs: &HashMap<String, String>) -> Result<String, ErreurRendu> {
    if let Some(manquante) = variables_citees(gabarit)
        .into_iter()
        .find(|nom| !valeurs.contains_key(nom))
    {
        return Err(ErreurRendu::VariableManquante(manquante));
    }

    let mut sortie = String::with_capacity(gabarit.len());
    let mut reste = gabarit;
    while let Some(debut) = reste.find("{{") {
        let apres = &reste[debut + 2..];
        let Some(fin) = apres.find("}}") else { break };
        let nom = apres[..fin].trim();
        match valeurs.get(nom).filter(|_| est_un_nom(nom)) {
            Some(valeur) => {
                sortie.push_str(&reste[..debut]);
                sortie.push_str(valeur);
            }
            // Ce qui n'est pas un nom de variable est du texte : `{{ 1 + 1 }}`
            // n'est pas une expression, c'est une accolade qu'on recopie.
            None => sortie.push_str(&reste[..debut + 2 + fin + 2]),
        }
        reste = &apres[fin + 2..];
    }
    sortie.push_str(reste);
    Ok(sortie)
}

/// Un nom de variable : minuscules, chiffres, soulignés. C'est la grammaire de
/// `notification_types.expected_variables`, où le modèle écrit `{prenom}` et
/// `{titre_session}`.
fn est_un_nom(nom: &str) -> bool {
    !nom.is_empty()
        && nom.len() <= 64
        && nom
            .bytes()
            .all(|o| o.is_ascii_lowercase() || o.is_ascii_digit() || o == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valeurs(paires: &[(&str, &str)]) -> HashMap<String, String> {
        paires
            .iter()
            .map(|(c, v)| ((*c).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn la_substitution_est_litterale() {
        let rendu = rendre(
            "Bonjour {{prenom}}, « {{titre_session}} » commence {{delai}}.",
            &valeurs(&[
                ("prenom", "Awa"),
                ("titre_session", "Financer l'adaptation"),
                ("delai", "dans 1 heure"),
            ]),
        );
        assert_eq!(
            rendu.unwrap(),
            "Bonjour Awa, « Financer l'adaptation » commence dans 1 heure."
        );
    }

    /// LA règle du modèle : mieux vaut un travail en échec visible qu'un
    /// « Bonjour  , » envoyé à deux mille personnes.
    #[test]
    fn une_variable_manquante_fait_echouer_en_la_nommant() {
        let erreur = rendre("Bonjour {{prenom}}.", &valeurs(&[])).unwrap_err();
        assert_eq!(erreur, ErreurRendu::VariableManquante("prenom".to_owned()));
        assert!(erreur.to_string().contains("prenom"));
    }

    #[test]
    fn une_variable_repetee_est_remplacee_partout() {
        let rendu = rendre("{{prenom}} — {{prenom}}", &valeurs(&[("prenom", "Awa")]));
        assert_eq!(rendu.unwrap(), "Awa — Awa");
    }

    #[test]
    fn les_espaces_autour_du_nom_sont_tolerés() {
        let rendu = rendre("{{ prenom }}", &valeurs(&[("prenom", "Awa")]));
        assert_eq!(rendu.unwrap(), "Awa");
    }

    /// Une accolade qui n'ouvre rien de nommable reste du texte : refuser ici
    /// ferait échouer un courriel pour une accolade légitime.
    #[test]
    fn ce_qui_nest_pas_un_nom_reste_du_texte() {
        assert_eq!(rendre("{{ 1 + 1 }}", &valeurs(&[])).unwrap(), "{{ 1 + 1 }}");
        assert_eq!(rendre("{{}}", &valeurs(&[])).unwrap(), "{{}}");
        assert_eq!(rendre("{{Prenom}}", &valeurs(&[])).unwrap(), "{{Prenom}}");
    }

    #[test]
    fn une_accolade_non_fermee_ne_boucle_pas() {
        assert_eq!(
            rendre("début {{prenom", &valeurs(&[("prenom", "Awa")])).unwrap(),
            "début {{prenom"
        );
    }

    #[test]
    fn les_variables_citees_sont_dedoublonnees_et_rangees() {
        let citees = variables_citees("{{b}} {{a}} {{b}} {{ c }}");
        assert_eq!(
            citees.into_iter().collect::<Vec<_>>(),
            vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]
        );
    }

    /// Le cas réel : la variable vit dans un attribut HTML.
    #[test]
    fn une_variable_dans_un_attribut_est_citee_comme_une_autre() {
        let gabarit = r#"<a href="{{lien_participation}}">Rejoindre</a>"#;
        assert!(variables_citees(gabarit).contains("lien_participation"));
        let rendu = rendre(
            gabarit,
            &valeurs(&[("lien_participation", "https://x.fr/s/1")]),
        );
        assert_eq!(
            rendu.unwrap(),
            r#"<a href="https://x.fr/s/1">Rejoindre</a>"#
        );
    }
}
