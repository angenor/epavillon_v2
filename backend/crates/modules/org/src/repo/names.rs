//! Les dénominations : confirmer, déconfirmer.
//!
//! **`is_confirmed` ne décide que de l'affichage.** Confirmée ou non, une
//! dénomination sert la recherche : c'est ce qui permet de retrouver une fiche
//! par une faute d'orthographe connue sans jamais l'afficher sous ce nom.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;

use crate::domain::ids::{OrganizationId, OrganizationNameId};

/// La dénomination est-elle **posée par la base** ?
///
/// Le nom légal et le sigle sont recopiés par `tg_organizations_sync_names` :
/// les retirer à la main ne servirait à rien — le trigger les repose à la
/// première modification de la fiche — et l'API le refuse plutôt que de laisser
/// croire qu'un geste a eu un effet.
///
/// La comparaison porte sur le nom normalisé **et le genre** : une faute
/// d'orthographe connue peut avoir le même nom normalisé que le nom légal sans
/// être pour autant posée par la base.
pub async fn est_derivee<'e>(
    executor: impl PgExecutor<'e>,
    id: OrganizationNameId,
) -> Result<Option<bool>> {
    let derivee = sqlx::query_scalar!(
        r#"SELECT ((n.kind = 'legal'   AND n.name_normalized = o.legal_name_normalized)
                OR (n.kind = 'acronym' AND n.name_normalized = o.acronym_normalized))
                      AS "derivee!"
             FROM org.organization_names n
             JOIN org.organizations o ON o.id = n.organization_id
            WHERE n.id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(derivee)
}

/// Confirme ou déconfirme une dénomination.
pub async fn set_confirmed(
    conn: &mut PgConnection,
    organization_id: OrganizationId,
    name_id: OrganizationNameId,
    confirmee: bool,
) -> Result<bool> {
    let touchees = sqlx::query!(
        "UPDATE org.organization_names SET is_confirmed = $3
          WHERE id = $2 AND organization_id = $1",
        organization_id.as_uuid(),
        name_id.as_uuid(),
        confirmee
    )
    .execute(conn)
    .await?
    .rows_affected();

    Ok(touchees == 1)
}
