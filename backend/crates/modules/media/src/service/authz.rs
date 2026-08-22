//! **La résolution des gardes** — l'application de [`crate::domain::guards`].
//!
//! La table déclare *quoi* ; ce fichier va *chercher*. Séparer les deux permet
//! d'éprouver la table sans base, et de la lire d'un coup d'œil.
//!
//! # Le refus est un 404, jamais un 403
//!
//! Un identifiant hors périmètre se refuse **comme un identifiant inexistant** :
//! c'est la règle du principe IX, et elle ne souffre aucune exception ici. Un
//! 403 dirait à qui forge une URL que l'entité existe — et les six entités
//! porteuses de la table blanche sont précisément celles qu'on devine.
//!
//! **L'exception, et elle est écrite** : un compte sans **aucun** périmètre
//! d'administration reçoit un refus explicite sur les listes du back-office
//! (principe V). Cela ne concerne pas ce fichier, qui garde des entités.

use kernel::auth::{has_permission, Scope};
use kernel::error::{ApiError, Result};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::guards::{garde_pour, Garde};
use crate::repo::cross;

/// L'entité porteuse visée par un dépôt ou un rattachement.
#[derive(Debug, Clone, Copy)]
pub struct Porteuse<'a> {
    pub owner_schema: &'a str,
    pub owner_table: &'a str,
    pub owner_id: Uuid,
}

/// Cette personne peut-elle poser ou retirer un fichier sur cette entité ?
///
/// **Rend l'organisation propriétaire de l'objet à venir**, quand l'entité en
/// désigne une : c'est elle dont le quota est opposable, et la résoudre ici
/// évite de relire le dossier une seconde fois.
pub async fn exiger_le_droit(
    pool: &PgPool,
    acteur: Uuid,
    porteuse: Porteuse<'_>,
) -> Result<Option<Uuid>> {
    // Une combinaison non déclarée est REFUSÉE, jamais autorisée par défaut.
    let Some(garde) = garde_pour(porteuse.owner_schema, porteuse.owner_table) else {
        return Err(ApiError::not_found().detail(format!(
            "aucune garde déclarée pour {}.{} — voir domain/guards.rs",
            porteuse.owner_schema, porteuse.owner_table
        )));
    };

    match garde {
        Garde::OrganisationProprietaire { permission } => {
            if !cross::organisation_existe(pool, porteuse.owner_id).await? {
                return Err(ApiError::not_found());
            }
            let adhesion = cross::adhesion(pool, acteur, porteuse.owner_id).await?;
            let referent = adhesion.is_some_and(|a| a.active && a.referent);
            let admin = has_permission(
                pool,
                acteur,
                permission,
                Scope::Organization(porteuse.owner_id),
            )
            .await?;
            refuser_sauf(referent || admin)?;
            Ok(Some(porteuse.owner_id))
        }

        Garde::EditionAdministree { permission } => {
            if !cross::edition_existe(pool, porteuse.owner_id).await? {
                return Err(ApiError::not_found());
            }
            // **Permission ET périmètre**, règle métier n° 8 : un administrateur
            // d'une seule édition ne doit pas pouvoir illustrer celle d'à côté,
            // y compris en forgeant l'URL.
            let autorise =
                has_permission(pool, acteur, permission, Scope::Event(porteuse.owner_id)).await?;
            let perimetre = kernel::auth::administered_events(pool, acteur).await?;
            refuser_sauf(autorise && perimetre.allows(porteuse.owner_id))?;
            // Une édition n'appartient à aucune organisation : l'objet
            // appartiendra à la personne qui le dépose.
            Ok(None)
        }

        Garde::OrganisationDuDossier { permission } => {
            let Some(dossier) = cross::dossier(pool, porteuse.owner_id).await? else {
                return Err(ApiError::not_found());
            };
            let acces = acces_par_organisation_ou_edition(
                pool,
                acteur,
                permission,
                dossier.organization_id,
                dossier.event_id,
            )
            .await?;
            refuser_sauf(acces)?;
            Ok(dossier.organization_id)
        }

        Garde::OrganisationDeLaSeance { permission } => {
            let Some(seance) = cross::seance(pool, porteuse.owner_id).await? else {
                return Err(ApiError::not_found());
            };
            let acces = acces_par_organisation_ou_edition(
                pool,
                acteur,
                permission,
                seance.organization_id,
                seance.event_id,
            )
            .await?;
            refuser_sauf(acces)?;
            Ok(seance.organization_id)
        }

        Garde::PersonneElleMeme { permission } => {
            if !cross::personne_existe(pool, porteuse.owner_id).await? {
                return Err(ApiError::not_found());
            }
            let soi_meme = porteuse.owner_id == acteur;
            let admin = has_permission(pool, acteur, permission, Scope::Global).await?;
            refuser_sauf(soi_meme || admin)?;
            // Une photo de profil appartient à la personne, pas à son
            // organisation : aucun quota d'organisation ne s'y oppose.
            Ok(None)
        }

        Garde::PorteeDuContenu { permission } => {
            let Some(edition) = cross::contenu(pool, porteuse.owner_id).await? else {
                return Err(ApiError::not_found());
            };
            // Un contenu de vitrine peut ne viser aucune édition : sa garde
            // s'exerce alors sur la portée globale.
            let portee = edition.map_or(Scope::Global, Scope::Event);
            refuser_sauf(has_permission(pool, acteur, permission, portee).await?)?;
            Ok(None)
        }

        Garde::PermissionGlobale { permission } => {
            refuser_sauf(has_permission(pool, acteur, permission, Scope::Global).await?)?;
            Ok(None)
        }

        // Un refus déclaré, avec son motif dans la trace : le distinguer d'une
        // garde oubliée est le seul intérêt de cette variante.
        Garde::Fermee { motif } => Err(ApiError::not_found().detail(motif)),
    }
}

/// Adhésion **active** à l'organisation porteuse, **ou** permission sur
/// l'édition. C'est la règle posée par B4 : une organisation n'administre rien,
/// et son accès passe par l'adhésion, pas par un périmètre.
async fn acces_par_organisation_ou_edition(
    pool: &PgPool,
    acteur: Uuid,
    permission: &str,
    organization_id: Option<Uuid>,
    event_id: Uuid,
) -> Result<bool> {
    if let Some(organisation) = organization_id {
        if cross::adhesion(pool, acteur, organisation)
            .await?
            .is_some_and(|a| a.active)
        {
            return Ok(true);
        }
    }
    has_permission(pool, acteur, permission, Scope::Event(event_id)).await
}

fn refuser_sauf(autorise: bool) -> Result<()> {
    if autorise {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}
