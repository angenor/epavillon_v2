//! Les organisations associées à un dossier — **jamais la ligne du porteur**.
//!
//! # La règle qui gouverne ce fichier
//!
//! `programme.proposals.organization_id` désigne le porteur principal, et
//! `tg_sync_proposal_lead_organization()` en tient la ligne de rôle `lead` :
//! une seule vérité, deux points d'accès. **Le service n'écrit jamais cette
//! ligne** — et il refuse même d'ajouter le porteur comme co-organisateur.
//!
//! Ce refus n'est pas de la coquetterie. Le déclencheur pose la ligne du
//! porteur par `ON CONFLICT (proposal_id, organization_id) DO UPDATE SET role
//! = 'lead'` : une co-organisation posée sur l'organisation porteuse
//! **basculerait en `lead` au premier enregistrement suivant**, en silence, et
//! le dossier perdrait un co-organisateur sans qu'aucune erreur ne le dise.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::ids::ProposalId;

/// Une association à écrire — **jamais `lead`**, le service l'a déjà refusé.
pub struct Association<'a> {
    pub organization_id: Uuid,
    pub role: &'a str,
    pub sort_order: i16,
}

/// Remplacer les co-organisations d'un dossier, **exactement celles-là**.
///
/// Le geste est un remplacement : l'écran envoie la liste entière, et une
/// organisation retirée doit disparaître.
///
/// Rend **les organisations réellement ajoutées** — celles qui n'étaient pas
/// déjà là. C'est ce qui permet d'émettre `programme.coorganization.requested`
/// une fois par organisation et **pas à chaque enregistrement automatique** :
/// un brouillon s'enregistre toutes les deux secondes, et annoncer à chaque
/// fois inviterait la même organisation cent fois.
pub async fn remplacer(
    conn: &mut PgConnection,
    dossier: ProposalId,
    acteur: Uuid,
    associations: &[Association<'_>],
) -> Result<Vec<Uuid>> {
    let ids: Vec<Uuid> = associations.iter().map(|a| a.organization_id).collect();
    let roles: Vec<String> = associations.iter().map(|a| a.role.to_owned()).collect();
    let rangs: Vec<i16> = associations.iter().map(|a| a.sort_order).collect();

    // Le porteur est exclu du retrait par `role <> 'lead'` : sa ligne
    // appartient au déclencheur, et la supprimer la ferait reparaître au
    // prochain enregistrement — en attendant, le dossier serait sans porteur.
    sqlx::query!(
        "DELETE FROM programme.proposal_organizations
          WHERE proposal_id = $1 AND role <> 'lead' AND organization_id <> ALL($2)",
        dossier.as_uuid(),
        &ids
    )
    .execute(&mut *conn)
    .await?;

    if associations.is_empty() {
        return Ok(Vec::new());
    }

    let ajoutees = sqlx::query!(
        r#"INSERT INTO programme.proposal_organizations
               (proposal_id, organization_id, role, sort_order, added_by)
           SELECT $1, e.org, e.role::programme.organization_role, e.rang, $5
             FROM unnest($2::uuid[], $3::text[], $4::smallint[]) AS e(org, role, rang)
           ON CONFLICT (proposal_id, organization_id) DO UPDATE
               SET role = EXCLUDED.role, sort_order = EXCLUDED.sort_order
        RETURNING organization_id, (xmax = 0) AS "nouvelle!""#,
        dossier.as_uuid(),
        &ids,
        &roles,
        &rangs,
        acteur
    )
    .fetch_all(&mut *conn)
    .await?;

    // `xmax = 0` distingue l'insertion de la mise à jour dans un `ON CONFLICT` :
    // c'est la seule façon de le savoir sans relire la table avant d'écrire,
    // donc sans une seconde requête dont le résultat pourrait déjà être périmé.
    Ok(ajoutees
        .into_iter()
        .filter(|l| l.nouvelle)
        .map(|l| l.organization_id)
        .collect())
}

/// Les organisations associées, porteur compris — ce que la lecture d'un
/// dossier rend.
pub async fn lister(conn: &mut PgConnection, dossier: ProposalId) -> Result<Vec<(Uuid, String)>> {
    let lignes = sqlx::query!(
        r#"SELECT organization_id, role::text AS "role!"
             FROM programme.proposal_organizations
            WHERE proposal_id = $1
            ORDER BY role = 'lead' DESC, sort_order, organization_id"#,
        dossier.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| (l.organization_id, l.role))
        .collect())
}

// -----------------------------------------------------------------------------
// La lecture — ce que la fiche du comité et l'espace organisation affichent
// -----------------------------------------------------------------------------

/// Une ligne de rôle, telle que la table la porte — `ProposalOrganization`.
///
/// **Le porteur y figure**, et il le faut : la fiche affiche « porté par », et
/// masquer sa ligne obligerait l'écran à recomposer ce que le déclencheur tient
/// déjà en cohérence avec `proposals.organization_id`.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct LienDOrganisation {
    pub proposal_id: Uuid,
    pub organization_id: Uuid,
    pub role: String,
    /// **Nulle tant que la co-organisation n'est pas confirmée.** Une
    /// co-organisation annoncée engage un tiers : le back-office l'affiche
    /// « en attente » plutôt que de la compter comme acquise.
    #[serde(with = "time::serde::rfc3339::option")]
    pub confirmed_at: Option<time::OffsetDateTime>,
    pub sort_order: i16,
    pub added_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub added_at: time::OffsetDateTime,
}

/// Les organisations associées à un dossier, **porteur compris**, dans l'ordre
/// où le dossier les range.
pub async fn du_dossier<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<LienDOrganisation>> {
    let lignes = sqlx::query!(
        r#"SELECT proposal_id, organization_id, role::text AS "role!",
                  confirmed_at, sort_order, added_by, added_at
             FROM programme.proposal_organizations
            WHERE proposal_id = $1
            ORDER BY (role = 'lead') DESC, sort_order, organization_id"#,
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| LienDOrganisation {
            proposal_id: l.proposal_id,
            organization_id: l.organization_id,
            role: l.role,
            confirmed_at: l.confirmed_at,
            sort_order: l.sort_order,
            added_by: l.added_by,
            added_at: l.added_at,
        })
        .collect())
}
