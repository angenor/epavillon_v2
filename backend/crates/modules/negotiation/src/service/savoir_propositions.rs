//! Les termes proposés : les proposer (tout compte), les accepter ou les
//! refuser (l'expert). Le courriel des auteurs part à la publication de
//! l'entrée née, dans sa transaction (R11) ; le regroupement par forme
//! normalisée est tenu par `ux_glossary_proposals_pending`.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::postgres::PgConnection;
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::admin_savoir::{AdminGlossaryEntry, AdminGlossaryInput};
use crate::domain::savoir_propositions::{
    motif_valide, proposition_valide, AdminProposal, AdminProposalContext,
    AdminProposalRejectInput, IssueProposition, ProposalInput, ProposalReceipt,
    PLAFOND_PROPOSITIONS,
};
use crate::jobs::emails;
use crate::repo::savoir_propositions as repo;
use crate::service::savoir_admin::{creer_terme_dans, fiche_terme, Droits};
use crate::state::NegotiationState;

pub async fn proposer(
    state: &NegotiationState,
    ctx: &RequestContext,
    personne: Uuid,
    entree: &ProposalInput,
) -> Result<IssueProposition> {
    let mut tx = state.db().write(ctx).await?;
    repo::verrouiller(&mut tx, personne).await?;
    if let Some(recu) = repo::recu(&mut tx, personne, entree.client_ref).await? {
        return Ok(IssueProposition::Rejouee(recu));
    }
    let (term, context) = proposition_valide(entree)?;
    let forme = repo::forme(&mut tx, &term).await?;
    if forme.normalisee.is_none() {
        return Err(ApiError::validation(
            "Un terme porte au moins une lettre ou un chiffre.",
            "term",
        ));
    }
    if let Some(slug) = forme.au_lexique {
        return Ok(IssueProposition::DejaAuLexique { slug });
    }
    if repo::envoyees_aujourdhui(&mut tx, personne).await? >= PLAFOND_PROPOSITIONS {
        return Err(ApiError::new(ErrorCode::NegotiationProposalLimit));
    }
    let cible = match repo::cible(&mut tx, &term).await? {
        Some(c) => c,
        None => match repo::inserer(&mut tx, &term).await? {
            Some(c) => c,
            None => repo::cible(&mut tx, &term)
                .await?
                .ok_or_else(|| ApiError::internal("proposition jumelle introuvable"))?,
        },
    };
    let (created_at, nouvelle) = match repo::ajouter_l_auteur(
        &mut tx,
        cible.id,
        personne,
        entree.client_ref,
        context.as_deref(),
    )
    .await?
    {
        Some(le) => (le, true),
        // Le même terme, déjà proposé par la même personne : rien ne naît.
        None => (
            repo::auteur_depuis(&mut tx, cible.id, personne)
                .await?
                .ok_or_else(|| ApiError::internal("auteur de la proposition introuvable"))?,
            false,
        ),
    };
    let recu = ProposalReceipt {
        id: cible.id,
        client_ref: entree.client_ref,
        term: cible.term,
        created_at,
    };
    if !nouvelle {
        return Ok(IssueProposition::Rejouee(recu));
    }
    tx.commit().await?;
    Ok(IssueProposition::Nouvelle(recu))
}

async fn composer(conn: &mut PgConnection, id: Option<Uuid>) -> Result<Vec<AdminProposal>> {
    let lignes = repo::lignes(conn, id).await?;
    let ids: Vec<Uuid> = lignes.iter().map(|l| l.id).collect();
    let mut contextes: HashMap<Uuid, Vec<AdminProposalContext>> = HashMap::new();
    for c in repo::contextes(conn, &ids).await? {
        contextes
            .entry(c.proposal_id)
            .or_default()
            .push(AdminProposalContext {
                context: c.context,
                created_at: c.created_at,
            });
    }
    let mut proches: HashMap<Uuid, Vec<_>> = HashMap::new();
    for p in repo::proches(conn, &ids).await? {
        proches.entry(p.proposal_id).or_default().push(p.entree);
    }
    Ok(lignes
        .into_iter()
        .map(|l| AdminProposal {
            contexts: contextes.remove(&l.id).unwrap_or_default(),
            nearby: proches.remove(&l.id).unwrap_or_default(),
            id: l.id,
            term: l.term,
            status: l.status,
            authors_count: l.authors_count,
            glossary_entry_id: l.glossary_entry_id,
            glossary_entry_slug: l.glossary_entry_slug,
            rejection_reason: l.rejection_reason,
            created_at: l.created_at,
            handled_at: l.handled_at,
        })
        .collect())
}

pub async fn file(state: &NegotiationState) -> Result<Vec<AdminProposal>> {
    let mut conn = state.pool().acquire().await?;
    composer(&mut conn, None).await
}

async fn relue(conn: &mut PgConnection, id: Uuid) -> Result<AdminProposal> {
    composer(conn, Some(id))
        .await?
        .pop()
        .ok_or_else(ApiError::not_found)
}

pub async fn une(state: &NegotiationState, id: Uuid) -> Result<AdminProposal> {
    let mut conn = state.pool().acquire().await?;
    relue(&mut conn, id).await
}

async fn en_attente(conn: &mut PgConnection, id: Uuid) -> Result<String> {
    let p = repo::a_trancher(conn, id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    if p.status != "pending" {
        return Err(ApiError::new(ErrorCode::NegotiationQueueItemClosed));
    }
    Ok(p.term)
}

/// L'entrée naît en brouillon ; ses auteurs sont prévenus à sa publication.
pub async fn accepter(
    state: &NegotiationState,
    ctx: &RequestContext,
    droits: &Droits,
    expert: Uuid,
    id: Uuid,
    entree: &AdminGlossaryInput,
    locale: &str,
) -> Result<AdminGlossaryEntry> {
    let mut tx = state.db().write(ctx).await?;
    let term = en_attente(&mut tx, id).await?;
    let mut entree = entree.clone();
    if entree.term.is_none() {
        entree.term = Some(term);
    }
    let entree_id = creer_terme_dans(&mut tx, ctx, &entree).await?;
    repo::accepter(&mut tx, id, entree_id, expert).await?;
    tx.commit().await?;
    fiche_terme(state, droits, entree_id, locale).await
}

pub async fn rejeter(
    state: &NegotiationState,
    ctx: &RequestContext,
    expert: Uuid,
    id: Uuid,
    entree: &AdminProposalRejectInput,
) -> Result<AdminProposal> {
    let motif = motif_valide(entree)?;
    let mut tx = state.db().write(ctx).await?;
    en_attente(&mut tx, id).await?;
    repo::rejeter(&mut tx, id, &motif, expert).await?;
    let proposition = relue(&mut tx, id).await?;
    tx.commit().await?;
    Ok(proposition)
}

/// Appelée dans la transaction qui publie l'entrée. La clé porte la
/// proposition et l'auteur : une republication ne renvoie rien.
pub(crate) async fn prevenir_les_auteurs(conn: &mut PgConnection, entree: Uuid) -> Result<()> {
    for a in repo::auteurs_a_prevenir(conn, entree).await? {
        emails::mettre_en_file_publication(
            conn,
            a.proposal_id,
            a.person_id,
            &emails::Destinataire {
                email: &a.email,
                locale: &a.locale,
                first_name: &a.first_name,
            },
            &a.term,
            &a.slug,
        )
        .await?;
    }
    Ok(())
}
