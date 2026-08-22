//! **Le seul fichier du module qui lise hors de son schéma.**
//!
//! # La règle de frontière, posée par B2 et resserrée ici
//!
//! *Un module lit hors de son schéma quand la question porte sur ses propres
//! entités, il n'y écrit jamais, et il n'appelle jamais un autre module.*
//!
//! Ce module va plus loin que B3, B4 et B5 : **il n'écrit nulle part hors de
//! `media`**, et le contrôle mécanique du quickstart porte aussi sur `reference`
//! et `content`. Une seule ligne d'écriture ici ferait échouer ce contrôle.
//!
//! # La liste exhaustive de ce qui est lu, et la question posée
//!
//! | Lecture | Question |
//! |---|---|
//! | `org.memberships`, `org.organizations` | qui peut poser un fichier sur cette fiche, quelle organisation le possède, et sous quel nom le tableau des quotas la désigne |
//! | `event.events` | à quelle édition appartient l'entité visée, pour vérifier le périmètre |
//! | `programme.proposals` | à quelle organisation appartient le dossier visé |
//! | `programme.sessions` | quelle organisation anime la séance visée |
//! | `identity.people` | l'objet vise-t-il la personne connectée |
//! | `content.highlights` | à quelle édition se rattache le contenu visé |
//!
//! Rien d'autre. Une lecture ajoutée hors de cette liste appartient à ce
//! fichier, ou elle n'appartient pas à ce module.

use kernel::error::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// L'adhésion d'une personne à une organisation, réduite à ce qui décide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adhesion {
    pub active: bool,
    /// Référent de la fiche — c'est le droit qu'exige le logo, qui engage la
    /// fiche publique.
    pub referent: bool,
}

/// L'adhésion d'une personne à une organisation. `None` : aucune ligne.
pub async fn adhesion(
    pool: &PgPool,
    person_id: Uuid,
    organization_id: Uuid,
) -> Result<Option<Adhesion>> {
    let ligne = sqlx::query!(
        r#"SELECT m.status::text AS "statut!", m.role::text AS "role!"
             FROM org.memberships m
            WHERE m.person_id = $1 AND m.organization_id = $2"#,
        person_id,
        organization_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| Adhesion {
        active: l.statut == "active",
        referent: l.role == "manager",
    }))
}

/// L'organisation existe-t-elle ? Un identifiant inconnu se refuse comme un
/// identifiant hors périmètre — principe IX, sans exception.
pub async fn organisation_existe(pool: &PgPool, organization_id: Uuid) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM org.organizations o WHERE o.id = $1) AS "existe!""#,
        organization_id
    )
    .fetch_one(pool)
    .await?;
    Ok(existe)
}

/// L'édition existe-t-elle ?
pub async fn edition_existe(pool: &PgPool, event_id: Uuid) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM event.events e WHERE e.id = $1) AS "existe!""#,
        event_id
    )
    .fetch_one(pool)
    .await?;
    Ok(existe)
}

/// Le dossier visé : son édition et l'organisation qui le porte.
#[derive(Debug, Clone, Copy)]
pub struct DossierVise {
    pub event_id: Uuid,
    /// L'organisation porteuse — la colonne `proposals.organization_id`, qui
    /// fait foi ; la ligne « porteur » de `proposal_organizations` en découle
    /// par déclencheur. Nulle pour une séance que l'IFDD a programmée
    /// directement, sans dossier.
    pub organization_id: Option<Uuid>,
}

pub async fn dossier(pool: &PgPool, proposal_id: Uuid) -> Result<Option<DossierVise>> {
    let ligne = sqlx::query!(
        "SELECT p.event_id, p.organization_id FROM programme.proposals p WHERE p.id = $1",
        proposal_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| DossierVise {
        event_id: l.event_id,
        organization_id: Some(l.organization_id),
    }))
}

/// La séance visée : son édition et l'organisation qui l'anime, via le dossier
/// dont elle est issue. Nulle quand l'IFDD programme directement.
pub async fn seance(pool: &PgPool, session_id: Uuid) -> Result<Option<DossierVise>> {
    let ligne = sqlx::query!(
        r#"SELECT s.event_id,
                  (SELECT p.organization_id
                     FROM programme.proposals p
                    WHERE p.id = s.proposal_id) AS organization_id
             FROM programme.sessions s
            WHERE s.id = $1"#,
        session_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(ligne.map(|l| DossierVise {
        event_id: l.event_id,
        organization_id: l.organization_id,
    }))
}

/// La personne existe-t-elle ?
pub async fn personne_existe(pool: &PgPool, person_id: Uuid) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM identity.people p WHERE p.id = $1) AS "existe!""#,
        person_id
    )
    .fetch_one(pool)
    .await?;
    Ok(existe)
}

/// Le contenu de vitrine visé, et l'édition qu'il met en avant. `event_id` nul :
/// le contenu ne vise aucune édition, et sa garde s'exerce alors sur la portée
/// globale.
pub async fn contenu(pool: &PgPool, highlight_id: Uuid) -> Result<Option<Option<Uuid>>> {
    let ligne = sqlx::query!(
        "SELECT h.event_id FROM content.highlights h WHERE h.id = $1",
        highlight_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(ligne.map(|l| l.event_id))
}

/// Les dénominations des organisations d'un tableau de quotas.
///
/// **Une organisation porte plusieurs dénominations** (règle métier n° 1) ; le
/// tableau du back-office affiche la principale, celle que porte la fiche.
pub async fn noms_dorganisations(pool: &PgPool, ids: &[Uuid]) -> Result<Vec<(Uuid, String)>> {
    let lignes = sqlx::query!(
        "SELECT id, legal_name FROM org.organizations WHERE id = ANY($1)",
        ids
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes.into_iter().map(|l| (l.id, l.legal_name)).collect())
}
