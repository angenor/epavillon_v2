//! Les lectures hors schéma des messages adressés et des accusés de lecture :
//! qui, parmi des personnes, se tient du côté du comité, et sous quel nom.
//!
//! « Du côté du comité » reprend l'ordre de `perimeter::acces_au_dossier` :
//! la voie de l'organisation passe d'abord. Un administrateur qui est aussi
//! membre de l'organisation porteuse lit le dossier en déposant, donc ne voit
//! aucun message du comité : le lui adresser serait écrire dans le vide.

use kernel::error::Result;
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::domain::ids::ProposalId;
use crate::repo::comments::{CommentPerson, CommentReader};

/// Parmi `ids`, les personnes actives qui ne lisent pas ce dossier en
/// déposant, par nom.
pub async fn comite_parmi<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    ids: &[Uuid],
) -> Result<Vec<CommentPerson>> {
    let lignes = sqlx::query!(
        r#"SELECT p.id, p.display_name AS "display_name!"
             FROM identity.people p
             JOIN programme.proposals d ON d.id = $1
            WHERE p.id = ANY($2)
              AND p.status = 'active'
              AND NOT EXISTS (
                    SELECT 1 FROM org.memberships m
                     WHERE m.organization_id = d.organization_id
                       AND m.person_id = p.id
                       AND (m.status = 'active'
                            OR (p.id = d.submitted_by AND m.status = 'pending' AND m.invited_at IS NULL))
              )
            ORDER BY p.display_name, p.id"#,
        dossier.as_uuid(),
        ids
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| CommentPerson {
            person_id: l.id,
            display_name: l.display_name,
        })
        .collect())
}

/// Les lectures de ces messages, chacune avec le côté d'où elle vient.
pub async fn lecteurs<'e>(
    executor: impl PgExecutor<'e>,
    messages: &[Uuid],
) -> Result<Vec<(Uuid, CommentReader)>> {
    let lignes = sqlx::query!(
        r#"SELECT r.comment_id, r.person_id, p.display_name AS "display_name!", r.read_at,
                  EXISTS (
                      SELECT 1 FROM org.memberships m
                       WHERE m.organization_id = d.organization_id
                         AND m.person_id = r.person_id
                         AND (m.status = 'active'
                              OR (r.person_id = d.submitted_by AND m.status = 'pending' AND m.invited_at IS NULL))
                  ) AS "organisation!"
             FROM programme.proposal_comment_reads r
             JOIN programme.proposal_comments c ON c.id = r.comment_id
             JOIN programme.proposals d ON d.id = c.proposal_id
             JOIN identity.people p ON p.id = r.person_id
            WHERE r.comment_id = ANY($1)
            ORDER BY r.read_at, p.display_name"#,
        messages
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| {
            (
                l.comment_id,
                CommentReader {
                    person_id: l.person_id,
                    display_name: l.display_name,
                    read_at: l.read_at,
                    side: if l.organisation {
                        "organisation"
                    } else {
                        "committee"
                    },
                },
            )
        })
        .collect())
}

/// Ce que l'avis d'un message adressé nomme : le titre du dossier et l'auteur.
pub struct ContexteDAvis {
    pub titre_fr: String,
    pub titre_en: String,
    pub auteur: String,
}

pub async fn contexte_davis<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
    auteur: Uuid,
) -> Result<Option<ContexteDAvis>> {
    let ligne = sqlx::query!(
        r#"SELECT COALESCE(d.title->>'fr', d.reference_code) AS "titre_fr!",
                  COALESCE(d.title->>'en', d.title->>'fr', d.reference_code) AS "titre_en!",
                  p.display_name AS "auteur!"
             FROM programme.proposals d
             JOIN identity.people p ON p.id = $2
            WHERE d.id = $1"#,
        dossier.as_uuid(),
        auteur
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| ContexteDAvis {
        titre_fr: l.titre_fr,
        titre_en: l.titre_en,
        auteur: l.auteur,
    }))
}

/// Les destinataires de tous les fils d'un dossier, par nom.
pub async fn destinataires_du_dossier<'e>(
    executor: impl PgExecutor<'e>,
    dossier: ProposalId,
) -> Result<Vec<(Uuid, CommentPerson)>> {
    let lignes = sqlx::query!(
        r#"SELECT d.comment_id, d.person_id, p.display_name AS "display_name!"
             FROM programme.proposal_comment_recipients d
             JOIN programme.proposal_comments c ON c.id = d.comment_id
             JOIN identity.people p ON p.id = d.person_id
            WHERE c.proposal_id = $1
            ORDER BY d.added_at, p.display_name"#,
        dossier.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| {
            (
                l.comment_id,
                CommentPerson {
                    person_id: l.person_id,
                    display_name: l.display_name,
                },
            )
        })
        .collect())
}
