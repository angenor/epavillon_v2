//! **LA TABLE DE GARDES** — le fichier le plus important du module.
//!
//! # Aucune permission `media.*` n'existe, et ce n'est pas un oubli
//!
//! Le modèle déclare des permissions pour dix modules ; le schéma `media` n'en
//! porte **aucune** (écart n° 127). Le droit de poser un fichier découle donc du
//! **droit d'écrire sur ce qu'il illustre** — ce qui est aussi la règle la plus
//! juste : qui peut modifier une fiche d'organisation peut en changer le logo,
//! et personne d'autre.
//!
//! `media.attachable_roles` déclare la **forme** — types, poids, rapport,
//! multiplicité. Elle ne déclare **pas qui a le droit**. C'est ici, et nulle
//! part ailleurs.
//!
//! # Toute combinaison non associée est REFUSÉE
//!
//! Jamais autorisée par défaut. Une table blanche est faite pour s'allonger : le
//! jour où un module y ajoute une ligne sans passer par ici, la garde manquante
//! serait une **porte ouverte** — et rien à la compilation ne la signalerait.
//! Un test d'intégration lit donc `media.attachable_roles` **en base** et échoue
//! sur toute ligne que ce fichier ne nomme pas.
//!
//! # Ce fichier ne lit rien : il DÉCLARE
//!
//! La résolution — aller chercher l'adhésion, la permission, le périmètre — vit
//! dans le service, qui lit par `repo/cross.rs`. Séparer les deux permet
//! d'éprouver la table sans base, et de la lire d'un coup d'œil.

/// Ce qui autorise à poser ou retirer un fichier sur une entité porteuse.
///
/// Chaque variante porte la permission qui **remplace** la relation directe : le
/// modèle veut qu'un administrateur puisse toujours agir, sans être membre de
/// l'organisation ni être la personne concernée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Garde {
    /// Référent de l'organisation visée, **ou** la permission.
    ///
    /// Référent et non simple membre : le logo engage la fiche publique.
    OrganisationProprietaire { permission: &'static str },

    /// La permission **sur la portée de l'édition**, ET le périmètre
    /// d'administration — règle métier n° 8, y compris quand l'utilisateur
    /// forge une URL.
    EditionAdministree { permission: &'static str },

    /// Adhésion **active** à l'organisation porteuse du dossier, **ou** la
    /// permission sur l'édition du dossier.
    ///
    /// Adhésion simple et non référence : c'est la règle d'`ownership` posée par
    /// B4 — toute personne dont l'adhésion est active peut agir sur les dossiers
    /// de son organisation.
    OrganisationDuDossier { permission: &'static str },

    /// Adhésion **active** à l'organisation qui anime la séance, **ou** la
    /// permission sur l'édition.
    OrganisationDeLaSeance { permission: &'static str },

    /// La personne elle-même, **ou** la permission. Une photo de profil
    /// appartient à qui elle représente.
    PersonneElleMeme { permission: &'static str },

    /// La permission, sur la portée de l'édition que le contenu met en avant —
    /// globale quand le contenu n'en vise aucune.
    PorteeDuContenu { permission: &'static str },

    /// La permission, sur la portée globale. Aucune relation directe n'ouvre
    /// l'accès.
    PermissionGlobale { permission: &'static str },

    /// **Refus explicite, et le motif est écrit.**
    ///
    /// Ce n'est pas une garde oubliée : c'est une garde qui ne peut pas encore
    /// s'écrire, parce que le modèle ne porte aucune permission pour ce module.
    /// La distinction compte — une ligne absente de cette table est un défaut,
    /// une ligne fermée est une décision.
    Fermee { motif: &'static str },
}

/// La table, dans l'ordre où `media.attachable_roles` sème ses lignes.
///
/// **Les huit couples de la table blanche y sont**, et pas seulement les six que
/// la conception avait relevés : `075_programme_sessions.sql` et
/// `125_training.sql` en sèment chacun un que le plan n'avait pas vus. Les
/// omettre aurait laissé deux portes sans garde — exactement ce que ce fichier
/// existe pour empêcher.
const TABLE: &[((&str, &str), Garde)] = &[
    (
        ("org", "organizations"),
        Garde::OrganisationProprietaire {
            permission: "org.organization.manage",
        },
    ),
    (
        ("event", "events"),
        Garde::EditionAdministree {
            permission: "event.event.manage",
        },
    ),
    (
        ("programme", "proposals"),
        Garde::OrganisationDuDossier {
            permission: "programme.proposal.decide",
        },
    ),
    (
        ("programme", "sessions"),
        Garde::OrganisationDeLaSeance {
            permission: "programme.session.schedule",
        },
    ),
    (
        ("identity", "people"),
        Garde::PersonneElleMeme {
            permission: "identity.person.manage",
        },
    ),
    (
        ("content", "highlights"),
        Garde::PorteeDuContenu {
            permission: "content.highlight.manage",
        },
    ),
    (
        // Le module est fermé par drapeau ; la garde existe quand même, sans
        // quoi le jour de son ouverture serait le jour d'une porte ouverte.
        ("publication", "articles"),
        Garde::PermissionGlobale {
            permission: "publication.article.write",
        },
    ),
    (
        ("training", "trainings"),
        Garde::Fermee {
            motif: "Le module Formations ne déclare aucune permission dans le modèle : \
                    aucune garde ne peut s'écrire tant qu'il n'en porte pas. Le rattachement \
                    est refusé, et le refus est une décision, pas un oubli.",
        },
    ),
];

/// La garde d'un couple (schéma, table). `None` **refuse** : une combinaison
/// non déclarée n'est jamais autorisée par défaut.
pub fn garde_pour(owner_schema: &str, owner_table: &str) -> Option<Garde> {
    TABLE
        .iter()
        .find(|((s, t), _)| *s == owner_schema && *t == owner_table)
        .map(|(_, g)| *g)
}

/// Les couples déclarés, pour le test qui les confronte à la base.
pub fn couples_declares() -> impl Iterator<Item = (&'static str, &'static str)> {
    TABLE.iter().map(|((s, t), _)| (*s, *t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_combinaison_inconnue_est_refusee_et_non_autorisee() {
        assert!(garde_pour("negotiation", "documents").is_none());
        assert!(garde_pour("org", "memberships").is_none());
        // Le schéma existe, la table aussi, mais le couple n'est pas déclaré.
        assert!(garde_pour("event", "rooms").is_none());
    }

    #[test]
    fn les_huit_couples_de_la_table_blanche_ont_leur_garde() {
        for (schema, table) in [
            ("org", "organizations"),
            ("event", "events"),
            ("programme", "proposals"),
            ("programme", "sessions"),
            ("identity", "people"),
            ("content", "highlights"),
            ("publication", "articles"),
            ("training", "trainings"),
        ] {
            assert!(
                garde_pour(schema, table).is_some(),
                "{schema}.{table} sans garde"
            );
        }
    }

    /// Aucun doublon : deux lignes pour un même couple rendraient la première,
    /// et la seconde ne serait jamais lue.
    #[test]
    fn aucun_couple_nest_declare_deux_fois() {
        let mut vus: Vec<(&str, &str)> = couples_declares().collect();
        let total = vus.len();
        vus.sort_unstable();
        vus.dedup();
        assert_eq!(vus.len(), total);
    }
}
