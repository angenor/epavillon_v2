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

    /// La garde d'[`Garde::OrganisationDuDossier`], sur le dossier de la ligne
    /// d'intervenant visée : qui peut modifier le dossier en illustre les
    /// intervenants, qu'ils aient un compte ou non.
    OrganisationDuDossierDeLaLigne { permission: &'static str },

    /// Adhésion **active** à l'organisation qui anime la séance, **ou** la
    /// permission sur l'édition.
    OrganisationDeLaSeance { permission: &'static str },

    /// La permission **sur la portée de l'édition de la séance**, ET le
    /// périmètre d'administration (règle métier n° 8). Aucune adhésion n'ouvre
    /// l'accès : c'est le geste de l'équipe qui tient l'édition.
    SeanceAdministree { permission: &'static str },

    /// La permission **sur la portée de l'édition de l'envoi**, ET le périmètre
    /// d'administration : les pièces jointes d'un courriel du back-office. Le
    /// dépôt est toujours privé : un fichier joint n'est jamais servi au public.
    EnvoiAdministre { permission: &'static str },

    /// La permission **sur la portée de l'édition du gabarit de courriel**, ET
    /// le périmètre d'administration ; la portée globale pour un gabarit commun
    /// à toutes les éditions. Ses images (fond, logo) partent par leur adresse
    /// publique dans les courriels : le dépôt n'est pas privé.
    GabaritDeCourriel { permission: &'static str },

    /// La personne elle-même, **ou** la permission. Une photo de profil
    /// appartient à qui elle représente.
    PersonneElleMeme { permission: &'static str },

    /// La permission, sur la portée de l'édition que le contenu met en avant —
    /// globale quand le contenu n'en vise aucune.
    PorteeDuContenu { permission: &'static str },

    /// La permission, sur la portée globale. Aucune relation directe n'ouvre
    /// l'accès.
    PermissionGlobale { permission: &'static str },

    /// La permission, sur la portée globale, pour un fichier qui n'illustre pas
    /// l'entité mais **est** sa donnée : des types fermés, et un dépôt toujours
    /// privé, dans le bucket fermé au web. L'entité le désigne par sa propre
    /// colonne, pas par `media.attachments` : aucune ligne de la table blanche
    /// ne le décrit, et c'est donc la garde qui porte les types.
    FichierPrive {
        permission: &'static str,
        types: &'static [&'static str],
    },

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
/// **Les neuf couples de la table blanche y sont**, et pas seulement les six que
/// la conception avait relevés : `075_programme_sessions.sql` et
/// `125_training.sql` en sèment chacun un que le plan n'avait pas vus. Les
/// omettre aurait laissé deux portes sans garde, exactement ce que ce fichier
/// existe pour empêcher. La photo d'un intervenant sur sa ligne a suivi.
/// S'y ajoute le PDF des documents de négociation, que
/// la table blanche ne décrit pas : il n'est pas rattaché, il est la donnée.
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
        // Le PDF d'un document de Guide Négo (specs/011 R5). Portée globale :
        // pour Guide Négo, « son périmètre » veut dire toute la plateforme.
        ("negotiation", "documents"),
        Garde::FichierPrive {
            permission: "negotiation.document.publish",
            types: &["application/pdf"],
        },
    ),
    (
        // Photo d'un intervenant, sur sa ligne du dossier (29/10) : l'organisation
        // la pose pour tout intervenant, avec ou sans compte. L'avatar d'une
        // personne reste à elle seule.
        ("programme", "proposal_speakers"),
        Garde::OrganisationDuDossierDeLaLigne {
            permission: "programme.proposal.decide",
        },
    ),
    (
        // Pièces jointes d'un envoi de courriels (28/10). Même droit que l'envoi.
        ("engagement", "mailings"),
        Garde::EnvoiAdministre {
            permission: "engagement.mailing.send",
        },
    ),
    (
        // Fond et logo d'un gabarit de courriel personnalisé (29/10).
        ("engagement", "mail_templates"),
        Garde::GabaritDeCourriel {
            permission: "engagement.mailing.send",
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

/// Les gardes écrites pour **un rôle**, plus étroites que celle de leur couple.
///
/// Une entité peut confier un rôle à plus de monde qu'un autre : la couverture
/// d'une séance est posée par l'organisation qui l'anime, sa galerie (D7) par
/// l'équipe de l'IFDD seule. Une ligne ici **remplace** la garde du couple pour
/// ce rôle ; un rôle absent d'ici garde celle du couple. Lister sans rôle reste
/// gardé par le couple : ce que l'on peut changer, on peut le lire.
const TABLE_PAR_ROLE: &[((&str, &str, &str), Garde)] = &[(
    // Galerie d'une activité. Même permission que le planificateur qui
    // modifie la séance, mais sans la porte de l'adhésion : les organisations
    // n'ajoutent pas de photo, pour l'instant.
    ("programme", "sessions", "gallery"),
    Garde::SeanceAdministree {
        permission: "programme.session.schedule",
    },
)];

impl Garde {
    /// Les types que la garde admet seule, sans rôle. `None` : c'est la ligne de
    /// `attachable_roles` qui les dit.
    pub fn types_admis(self) -> Option<&'static [&'static str]> {
        match self {
            Garde::FichierPrive { types, .. } => Some(types),
            _ => None,
        }
    }

    /// Vrai : l'objet est déposé privé, quoi que dise la demande.
    pub fn impose_le_prive(self) -> bool {
        matches!(
            self,
            Garde::FichierPrive { .. } | Garde::EnvoiAdministre { .. }
        )
    }
}

/// La garde d'un couple (schéma, table). `None` **refuse** : une combinaison
/// non déclarée n'est jamais autorisée par défaut.
pub fn garde_pour(owner_schema: &str, owner_table: &str) -> Option<Garde> {
    TABLE
        .iter()
        .find(|((s, t), _)| *s == owner_schema && *t == owner_table)
        .map(|(_, g)| *g)
}

/// La garde d'un geste sur un rôle précis : celle du rôle si elle est écrite,
/// celle du couple sinon. `role` nul (une lecture de tous les rôles, un dépôt
/// sans rôle) : celle du couple.
pub fn garde_pour_role(owner_schema: &str, owner_table: &str, role: Option<&str>) -> Option<Garde> {
    role.and_then(|role| {
        TABLE_PAR_ROLE
            .iter()
            .find(|((s, t, r), _)| *s == owner_schema && *t == owner_table && *r == role)
            .map(|(_, g)| *g)
    })
    .or_else(|| garde_pour(owner_schema, owner_table))
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
        assert!(garde_pour("negotiation", "spaces").is_none());
        assert!(garde_pour("org", "memberships").is_none());
        // Le schéma existe, la table aussi, mais le couple n'est pas déclaré.
        assert!(garde_pour("event", "rooms").is_none());
    }

    #[test]
    fn les_neuf_couples_de_la_table_blanche_ont_leur_garde() {
        for (schema, table) in [
            ("org", "organizations"),
            ("event", "events"),
            ("programme", "proposals"),
            ("programme", "sessions"),
            ("programme", "proposal_speakers"),
            ("identity", "people"),
            ("content", "highlights"),
            ("publication", "articles"),
            ("training", "trainings"),
            ("engagement", "mailings"),
        ] {
            assert!(
                garde_pour(schema, table).is_some(),
                "{schema}.{table} sans garde"
            );
        }
    }

    /// Le PDF d'un document de négociation : la permission de publier, un PDF
    /// seul, et toujours privé — un document réservé ne touche jamais le
    /// bucket ouvert au web.
    #[test]
    fn un_document_de_negociation_est_un_pdf_prive_garde_par_la_publication() {
        let garde = garde_pour("negotiation", "documents").expect("garde déclarée");
        assert_eq!(
            garde,
            Garde::FichierPrive {
                permission: "negotiation.document.publish",
                types: &["application/pdf"],
            }
        );
        assert_eq!(garde.types_admis(), Some(&["application/pdf"][..]));
        assert!(garde.impose_le_prive());
        assert!(!garde_pour("event", "events").unwrap().impose_le_prive());
    }

    /// Les pièces jointes d'un envoi : la permission d'envoyer, sur l'édition de
    /// l'envoi, et un dépôt toujours privé.
    #[test]
    fn une_piece_jointe_dun_envoi_est_privee_et_gardee_par_lenvoi() {
        let garde =
            garde_pour_role("engagement", "mailings", Some("attachment")).expect("garde déclarée");
        assert_eq!(
            garde,
            Garde::EnvoiAdministre {
                permission: "engagement.mailing.send",
            }
        );
        assert!(garde.impose_le_prive());
        assert!(garde.types_admis().is_none());
    }

    /// Le fond et le logo d'un gabarit de courriel : la permission d'envoyer,
    /// sur l'édition du gabarit ; un dépôt public, servi dans les courriels.
    #[test]
    fn les_images_dun_gabarit_de_courriel_sont_gardees_par_lenvoi() {
        for role in ["background", "logo"] {
            let garde = garde_pour_role("engagement", "mail_templates", Some(role))
                .expect("garde déclarée");
            assert_eq!(
                garde,
                Garde::GabaritDeCourriel {
                    permission: "engagement.mailing.send",
                }
            );
            assert!(!garde.impose_le_prive());
        }
    }

    /// La galerie d'une séance n'a pas la porte de l'adhésion que sa couverture
    /// garde : les organisations n'y ajoutent rien.
    #[test]
    fn la_galerie_dune_seance_est_reservee_a_lequipe() {
        assert_eq!(
            garde_pour_role("programme", "sessions", Some("gallery")),
            Some(Garde::SeanceAdministree {
                permission: "programme.session.schedule",
            })
        );
        assert_eq!(
            garde_pour_role("programme", "sessions", Some("cover")),
            garde_pour("programme", "sessions")
        );
        assert_eq!(
            garde_pour_role("programme", "sessions", None),
            garde_pour("programme", "sessions")
        );
        // Une garde de rôle ne crée pas de couple : sans garde de couple, refus.
        assert!(garde_pour_role("event", "rooms", Some("gallery")).is_none());
    }

    /// La photo d'un intervenant se garde comme le dossier de sa ligne, avec la
    /// même permission ; l'avatar d'une personne reste à elle.
    #[test]
    fn la_ligne_dun_intervenant_se_garde_comme_son_dossier() {
        assert_eq!(
            garde_pour_role("programme", "proposal_speakers", Some("avatar")),
            Some(Garde::OrganisationDuDossierDeLaLigne {
                permission: "programme.proposal.decide",
            })
        );
        assert!(matches!(
            garde_pour("identity", "people"),
            Some(Garde::PersonneElleMeme { .. })
        ));
    }

    /// Chaque garde de rôle s'appuie sur un couple déclaré : sinon, le test qui
    /// confronte la table blanche à la base ne la verrait pas.
    #[test]
    fn toute_garde_de_role_a_son_couple() {
        for ((schema, table, _), _) in TABLE_PAR_ROLE {
            assert!(garde_pour(schema, table).is_some(), "{schema}.{table}");
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
