//! La table blanche, telle qu'elle se lit — et les trois prédicats qui la
//! consultent.
//!
//! # Ce que cette table déclare, et ce qu'elle ne déclare PAS
//!
//! Elle déclare la **forme** : quels types, quel poids, quel rapport, un seul
//! objet ou plusieurs. Elle ne déclare **pas qui a le droit** — c'est
//! `domain/guards.rs`, et l'écart n° 127.
//!
//! # Pourquoi vérifier avant d'écrire, alors que le trigger refuse déjà
//!
//! Parce que `media.tg_validate_attachment()` lève ses cinq refus par
//! `RAISE EXCEPTION` **sans nom de contrainte**, et que trois d'entre eux
//! partagent `integrity_constraint_violation`. Les distinguer par le texte du
//! message serait la faute que B3 a nommée : un message français se périme au
//! premier ajustement du SQL. Le service se sert donc de ce qu'il a lui-même
//! vérifié pour savoir lequel des trois vient de tomber — c'est le seul moyen
//! de rendre `MEDIA_MIME_NOT_ALLOWED` plutôt que `MEDIA_ROLE_NOT_DECLARED`.
//!
//! **Ce n'est pas une réimplémentation** : le refus reste celui du trigger, et
//! les prédicats ci-dessous ne servent qu'à le nommer.

use serde::Serialize;
use utoipa::ToSchema;

/// Une ligne de `media.attachable_roles`, telle que l'API la rend.
///
/// **Deux champs de plus que `AttachableRoleRule` du front** : la forme
/// attendue et sa tolérance. Le modèle les déclare, le contrat du front ne les
/// porte pas encore — et sans eux l'écran ne peut pas annoncer la forme avant
/// le refus. Un champ ajouté ne casse rien ; l'ajout côté front est inscrit aux
/// obligations de B7.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AttachableRoleRule {
    pub owner_schema: String,
    pub owner_table: String,
    pub role: String,
    pub label: serde_json::Value,
    pub is_multiple: bool,
    /// Préfixes MIME acceptés, motif `*` admis. **Vide = tout accepté.**
    pub allowed_mime_prefixes: Vec<String>,
    pub max_byte_size: Option<i64>,
    /// Largeur ÷ hauteur : 3.5556 pour un 32:9, 1.0 pour un carré. Traversée en
    /// texte — `numeric(6,4)` n'a pas de représentant flottant exact, et le
    /// rapport sert à afficher autant qu'à comparer.
    pub expected_aspect_ratio: Option<String>,
    pub aspect_ratio_tolerance: String,
    pub is_active: bool,
}

/// Le type est-il accepté pour ce rôle ?
///
/// Le motif de la base est un `LIKE` où `*` vaut `%` : `image/*` accepte
/// `image/png`. Un tableau vide accepte tout — c'est le défaut du modèle.
pub fn type_accepte(mime: &str, prefixes: &[String]) -> bool {
    if prefixes.is_empty() {
        return true;
    }
    prefixes.iter().any(|p| correspond(mime, p))
}

fn correspond(mime: &str, motif: &str) -> bool {
    match motif.split_once('*') {
        Some((debut, fin)) => {
            mime.len() >= debut.len() + fin.len() && mime.starts_with(debut) && mime.ends_with(fin)
        }
        None => mime == motif,
    }
}

/// Le poids est-il accepté ? Un plafond absent n'en impose aucun.
pub fn poids_accepte(octets: i64, max: Option<i64>) -> bool {
    max.is_none_or(|m| octets <= m)
}

/// La forme est-elle acceptée ?
///
/// **Ne s'applique qu'aux objets MESURÉS.** `width` et `height` sont nuls pour
/// un document, et un objet image dont le relevé a échoué passe : refuser ici
/// transformerait une panne de traitement en refus de téléversement. C'est mot
/// pour mot la règle du trigger, et l'y contredire produirait deux réponses
/// différentes à la même question.
pub fn forme_acceptee(
    largeur: Option<i32>,
    hauteur: Option<i32>,
    rapport_attendu: Option<f64>,
    tolerance: f64,
) -> bool {
    let (Some(attendu), Some(l), Some(h)) = (rapport_attendu, largeur, hauteur) else {
        return true;
    };
    if h <= 0 {
        return true;
    }
    let obtenu = f64::from(l) / f64::from(h);
    (obtenu - attendu).abs() <= attendu * tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prefixes(valeurs: &[&str]) -> Vec<String> {
        valeurs.iter().map(|v| (*v).to_owned()).collect()
    }

    #[test]
    fn un_tableau_vide_accepte_tout() {
        assert!(type_accepte("application/pdf", &[]));
    }

    #[test]
    fn le_motif_etoile_couvre_la_famille() {
        let regle = prefixes(&["image/*"]);
        assert!(type_accepte("image/png", &regle));
        assert!(!type_accepte("application/pdf", &regle));
    }

    /// La ligne des documents d'un dossier porte deux motifs, dont un exact.
    #[test]
    fn plusieurs_motifs_dont_un_exact() {
        let regle = prefixes(&["application/pdf", "application/vnd.*"]);
        assert!(type_accepte("application/pdf", &regle));
        assert!(type_accepte(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            &regle
        ));
        assert!(!type_accepte("image/png", &regle));
    }

    #[test]
    fn le_poids_sans_plafond_passe_toujours() {
        assert!(poids_accepte(999_999_999, None));
        assert!(poids_accepte(5_242_880, Some(5_242_880)));
        assert!(!poids_accepte(5_242_881, Some(5_242_880)));
    }

    /// La tolérance de 2 % laisse passer un 1600×902 pour un 16:9 et refuse un
    /// 4:3 présenté comme tel — l'exemple du commentaire du modèle.
    #[test]
    fn la_tolerance_du_modele_fait_ce_quelle_annonce() {
        assert!(forme_acceptee(Some(1920), Some(1080), Some(1.7778), 0.02));
        assert!(forme_acceptee(Some(1600), Some(902), Some(1.7778), 0.02));
        assert!(!forme_acceptee(Some(1024), Some(768), Some(1.7778), 0.02));
    }

    /// Le cas qui compte : un objet non mesuré passe. Le relevé a échoué, pas
    /// le cadrage.
    #[test]
    fn un_objet_non_mesure_passe() {
        assert!(forme_acceptee(None, None, Some(1.0), 0.02));
        assert!(forme_acceptee(Some(800), None, Some(1.0), 0.02));
    }

    #[test]
    fn un_role_sans_forme_attendue_nimpose_rien() {
        assert!(forme_acceptee(Some(3), Some(1000), None, 0.02));
    }
}
