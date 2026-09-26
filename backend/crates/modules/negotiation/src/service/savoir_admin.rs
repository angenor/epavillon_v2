//! Le back-office du savoir : FAQ, lexique, parcours. **Portée globale
//! partout.** Rédiger et publier demandent `negotiation.knowledge.publish` ;
//! dater une vérification, `negotiation.knowledge.review` ; lire, l'une ou
//! l'autre. Les invariants — vérification avant publication, entrée publiée
//! indestructible, lien d'étape cohérent — sont portés par la base : le code
//! traduit ses refus.

use kernel::auth::{has_permission, Scope};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::admin_savoir::{
    statut_filtre, AdminFaqEntry, AdminFaqInput, AdminFaqList, AdminFaqVerifyInput,
    AdminGlossaryEntry, AdminGlossaryInput, AdminGlossaryList, AdminPathway,
    AdminPathwayGroupInput, AdminPathwayLinkInput, AdminPathwayOrderInput, AdminPathwayStepInput,
    FiltreSavoir,
};
use crate::domain::permissions::{KNOWLEDGE_PUBLISH, KNOWLEDGE_REVIEW};
use crate::domain::savoir::KnowledgeStatus;
use crate::repo::savoir_faq as faq;
use crate::repo::savoir_lexique as lexique;
use crate::repo::savoir_parcours as parcours;
use crate::repo::savoir_sources::{self as sources, Proprietaire};
use crate::service::savoir_propositions;
use crate::state::NegotiationState;

pub struct Droits {
    pub publier: bool,
    pub verifier: bool,
}

pub async fn droits(state: &NegotiationState, personne: Uuid) -> Result<Droits> {
    Ok(Droits {
        publier: has_permission(state.pool(), personne, KNOWLEDGE_PUBLISH, Scope::Global).await?,
        verifier: has_permission(state.pool(), personne, KNOWLEDGE_REVIEW, Scope::Global).await?,
    })
}

pub async fn exiger_la_lecture(state: &NegotiationState, personne: Uuid) -> Result<Droits> {
    let d = droits(state, personne).await?;
    if d.publier || d.verifier {
        Ok(d)
    } else {
        Err(ApiError::forbidden())
    }
}

/// Le passage demandé à une entrée de FAQ ou du lexique.
#[derive(Clone, Copy)]
pub enum Transition {
    Publier,
    ARevoir,
    Depublier,
}

/// Le statut d'arrivée, ou le refus. Publier et dépublier valent de partout ;
/// « À revoir » ne vaut que pour une entrée publiée.
fn arrivee(depart: KnowledgeStatus, t: Transition) -> Result<&'static str> {
    match (t, depart) {
        (Transition::Publier, _) => Ok("published"),
        (Transition::Depublier, _) => Ok("draft"),
        (Transition::ARevoir, KnowledgeStatus::Draft) => Err(ApiError::with_message(
            ErrorCode::Conflict,
            "Seule une entrée publiée peut être mise « À revoir ».",
        )),
        (Transition::ARevoir, _) => Ok("to_review"),
    }
}

fn texte_valide(v: &Value, champ: &str) -> Result<()> {
    let fr = v.get("fr").and_then(Value::as_str).unwrap_or("").trim();
    let objet_de_chaines = v
        .as_object()
        .is_some_and(|o| o.values().all(Value::is_string));
    if fr.is_empty() || !objet_de_chaines {
        return Err(ApiError::validation(
            "Le texte en français est obligatoire.",
            champ,
        ));
    }
    Ok(())
}

fn texte_facultatif(v: &Option<Option<Value>>, champ: &str) -> Result<()> {
    match v {
        Some(Some(t)) => texte_valide(t, champ),
        _ => Ok(()),
    }
}

fn chaine(v: &Option<Option<String>>) -> Option<Option<&str>> {
    v.as_ref()
        .map(|t| t.as_deref().map(str::trim).filter(|t| !t.is_empty()))
}

fn variantes(v: &[String]) -> Vec<String> {
    let mut propres: Vec<String> = v
        .iter()
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
        .collect();
    propres.dedup();
    propres
}

// ---------------------------------------------------------------------------
// FAQ
// ---------------------------------------------------------------------------

fn faq_introuvable() -> ApiError {
    ApiError::new(ErrorCode::NegotiationFaqNotFound)
}

pub async fn liste_faq(
    state: &NegotiationState,
    droits: &Droits,
    filtre: &FiltreSavoir,
    locale: &str,
) -> Result<AdminFaqList> {
    let mut conn = state.pool().acquire().await?;
    let q = filtre.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
    let rubrique = filtre.section.as_deref().filter(|s| !s.is_empty());
    let entries = faq::lignes(
        &mut conn,
        locale,
        q,
        rubrique,
        statut_filtre(filtre.status.as_deref()),
    )
    .await?;
    Ok(AdminFaqList {
        entries,
        can_publish: droits.publier,
        can_review: droits.verifier,
    })
}

pub async fn fiche_faq(
    state: &NegotiationState,
    droits: &Droits,
    id: Uuid,
    locale: &str,
) -> Result<AdminFaqEntry> {
    let mut conn = state.pool().acquire().await?;
    let f = faq::fiche(&mut conn, id)
        .await?
        .ok_or_else(faq_introuvable)?;
    Ok(AdminFaqEntry {
        sources: sources::lire(&mut conn, locale, Proprietaire::Faq(id)).await?,
        related: faq::liees(&mut conn, locale, id).await?,
        feedback: faq::retours(&mut conn, id).await?,
        reports: faq::signalements(&mut conn, id).await?,
        id: f.id,
        section_code: f.section_code,
        question: f.question,
        answer: f.answer,
        status: KnowledgeStatus::depuis(&f.status),
        verified_on: f.verified_on,
        verified_by_name: f.verified_by_name,
        editorial_rank: f.editorial_rank,
        origin_question_id: f.origin_question_id,
        first_published_at: f.first_published_at,
        created_at: f.created_at,
        updated_at: f.updated_at,
        can_publish: droits.publier,
        can_review: droits.verifier,
    })
}

async fn rubrique(conn: &mut PgConnection, code: &str) -> Result<Uuid> {
    faq::rubrique_id(conn, code)
        .await?
        .ok_or_else(|| ApiError::validation("Cette rubrique n'existe pas.", "section_code"))
}

pub async fn creer_faq(
    state: &NegotiationState,
    ctx: &RequestContext,
    entree: &AdminFaqInput,
) -> Result<Uuid> {
    let auteur = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let question = entree
        .question
        .as_ref()
        .ok_or_else(|| ApiError::validation("La question est obligatoire.", "question"))?;
    texte_valide(question, "question")?;
    texte_facultatif(&entree.answer, "answer")?;
    let code = entree
        .section_code
        .as_deref()
        .ok_or_else(|| ApiError::validation("La rubrique est obligatoire.", "section_code"))?;

    let mut tx = state.db().write(ctx).await?;
    let section_id = rubrique(&mut tx, code).await?;
    let id = faq::creer(
        &mut tx,
        &faq::Nouvelle {
            section_id,
            question,
            answer: entree.answer.as_ref().and_then(Option::as_ref),
            editorial_rank: entree.editorial_rank.flatten(),
            auteur,
        },
    )
    .await?;
    if let Some(s) = &entree.sources {
        sources::remplacer(&mut tx, Proprietaire::Faq(id), s).await?;
    }
    if let Some(l) = &entree.related_ids {
        faq::remplacer_liees(&mut tx, id, l).await?;
    }
    tx.commit().await?;
    Ok(id)
}

pub async fn modifier_faq(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    entree: &AdminFaqInput,
) -> Result<()> {
    if let Some(q) = &entree.question {
        texte_valide(q, "question")?;
    }
    texte_facultatif(&entree.answer, "answer")?;

    let mut tx = state.db().write(ctx).await?;
    faq::statut_verrouille(&mut tx, id)
        .await?
        .ok_or_else(faq_introuvable)?;
    let section_id = match entree.section_code.as_deref() {
        Some(code) => Some(rubrique(&mut tx, code).await?),
        None => None,
    };
    faq::modifier(
        &mut tx,
        id,
        &faq::Modification {
            section_id,
            question: entree.question.as_ref(),
            answer: entree.answer.as_ref().map(Option::as_ref),
            editorial_rank: entree.editorial_rank,
        },
    )
    .await?;
    if let Some(s) = &entree.sources {
        sources::remplacer(&mut tx, Proprietaire::Faq(id), s).await?;
    }
    if let Some(l) = &entree.related_ids {
        faq::remplacer_liees(&mut tx, id, l).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn verifier_faq(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    entree: &AdminFaqVerifyInput,
) -> Result<()> {
    let expert = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let mut tx = state.db().write(ctx).await?;
    faq::statut_verrouille(&mut tx, id)
        .await?
        .ok_or_else(faq_introuvable)?;
    let le = match entree.verified_on {
        Some(d) => d,
        None => faq::aujourdhui(&mut tx).await?,
    };
    faq::verifier(&mut tx, id, le, expert).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn changer_faq(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    t: Transition,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    let depart = faq::statut_verrouille(&mut tx, id)
        .await?
        .ok_or_else(faq_introuvable)?;
    let statut = arrivee(KnowledgeStatus::depuis(&depart), t)?;
    faq::poser_le_statut(&mut tx, id, statut).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn supprimer_faq(state: &NegotiationState, ctx: &RequestContext, id: Uuid) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    if faq::supprimer(&mut tx, id).await? == 0 {
        return Err(faq_introuvable());
    }
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Lexique
// ---------------------------------------------------------------------------

fn terme_introuvable() -> ApiError {
    ApiError::new(ErrorCode::NegotiationGlossaryNotFound)
}

pub async fn liste_lexique(
    state: &NegotiationState,
    droits: &Droits,
    filtre: &FiltreSavoir,
    locale: &str,
) -> Result<AdminGlossaryList> {
    let mut conn = state.pool().acquire().await?;
    let q = filtre.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
    let famille = filtre.family.as_deref().filter(|s| !s.is_empty());
    let entries = lexique::lignes(
        &mut conn,
        locale,
        q,
        famille,
        statut_filtre(filtre.status.as_deref()),
    )
    .await?;
    Ok(AdminGlossaryList {
        entries,
        can_publish: droits.publier,
        can_review: droits.verifier,
    })
}

pub async fn fiche_terme(
    state: &NegotiationState,
    droits: &Droits,
    id: Uuid,
    locale: &str,
) -> Result<AdminGlossaryEntry> {
    let mut conn = state.pool().acquire().await?;
    let g = lexique::fiche(&mut conn, id)
        .await?
        .ok_or_else(terme_introuvable)?;
    Ok(AdminGlossaryEntry {
        sources: sources::lire(&mut conn, locale, Proprietaire::Lexique(id)).await?,
        related: lexique::liees(&mut conn, id).await?,
        id: g.id,
        slug: g.slug,
        family_code: g.family_code,
        term: g.term,
        acronym: g.acronym,
        variants: g.variants,
        translation: g.translation,
        definition: g.definition,
        heard_in_room: g.heard_in_room,
        status: KnowledgeStatus::depuis(&g.status),
        first_published_at: g.first_published_at,
        created_at: g.created_at,
        updated_at: g.updated_at,
        can_publish: droits.publier,
        can_review: droits.verifier,
    })
}

async fn famille(conn: &mut PgConnection, code: &str) -> Result<Uuid> {
    lexique::famille_id(conn, code)
        .await?
        .ok_or_else(|| ApiError::validation("Cette famille n'existe pas.", "family_code"))
}

pub async fn creer_terme(
    state: &NegotiationState,
    ctx: &RequestContext,
    entree: &AdminGlossaryInput,
) -> Result<Uuid> {
    let mut tx = state.db().write(ctx).await?;
    let id = creer_terme_dans(&mut tx, ctx, entree).await?;
    tx.commit().await?;
    Ok(id)
}

/// Le brouillon, écrit dans la transaction de l'appelant : la création directe
/// et l'acceptation d'un terme proposé.
pub(crate) async fn creer_terme_dans(
    conn: &mut PgConnection,
    ctx: &RequestContext,
    entree: &AdminGlossaryInput,
) -> Result<Uuid> {
    let auteur = ctx.actor_id.ok_or_else(ApiError::unauthenticated)?;
    let term = entree
        .term
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| ApiError::validation("Le terme est obligatoire.", "term"))?;
    let exiger = |v: &Option<Value>, champ: &'static str| -> Result<Value> {
        let t = v
            .clone()
            .ok_or_else(|| ApiError::validation("Le texte en français est obligatoire.", champ))?;
        texte_valide(&t, champ)?;
        Ok(t)
    };
    let translation = exiger(&entree.translation, "translation")?;
    let definition = exiger(&entree.definition, "definition")?;
    let code = entree
        .family_code
        .as_deref()
        .ok_or_else(|| ApiError::validation("La famille est obligatoire.", "family_code"))?;
    let variants = variantes(entree.variants.as_deref().unwrap_or_default());

    let family_id = famille(conn, code).await?;
    let id = lexique::creer(
        conn,
        &lexique::Nouveau {
            family_id,
            term,
            acronym: chaine(&entree.acronym).flatten(),
            variants: &variants,
            translation: &translation,
            definition: &definition,
            heard_in_room: chaine(&entree.heard_in_room).flatten(),
            auteur,
        },
    )
    .await?;
    if let Some(s) = &entree.sources {
        sources::remplacer(conn, Proprietaire::Lexique(id), s).await?;
    }
    if let Some(l) = &entree.related_ids {
        lexique::remplacer_liees(conn, id, l).await?;
    }
    Ok(id)
}

pub async fn modifier_terme(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    entree: &AdminGlossaryInput,
) -> Result<()> {
    for (v, champ) in [
        (&entree.translation, "translation"),
        (&entree.definition, "definition"),
    ] {
        if let Some(t) = v {
            texte_valide(t, champ)?;
        }
    }
    let term = match entree.term.as_deref().map(str::trim) {
        Some("") => return Err(ApiError::validation("Le terme est obligatoire.", "term")),
        autre => autre,
    };
    let variants = entree.variants.as_deref().map(variantes);

    let mut tx = state.db().write(ctx).await?;
    lexique::statut_verrouille(&mut tx, id)
        .await?
        .ok_or_else(terme_introuvable)?;
    let family_id = match entree.family_code.as_deref() {
        Some(code) => Some(famille(&mut tx, code).await?),
        None => None,
    };
    lexique::modifier(
        &mut tx,
        id,
        &lexique::Modification {
            family_id,
            term,
            acronym: chaine(&entree.acronym),
            variants: variants.as_deref(),
            translation: entree.translation.as_ref(),
            definition: entree.definition.as_ref(),
            heard_in_room: chaine(&entree.heard_in_room),
        },
    )
    .await?;
    if let Some(s) = &entree.sources {
        sources::remplacer(&mut tx, Proprietaire::Lexique(id), s).await?;
    }
    if let Some(l) = &entree.related_ids {
        lexique::remplacer_liees(&mut tx, id, l).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn changer_terme(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    t: Transition,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    let depart = lexique::statut_verrouille(&mut tx, id)
        .await?
        .ok_or_else(terme_introuvable)?;
    let statut = arrivee(KnowledgeStatus::depuis(&depart), t)?;
    lexique::poser_le_statut(&mut tx, id, statut).await?;
    if matches!(t, Transition::Publier) {
        savoir_propositions::prevenir_les_auteurs(&mut tx, id).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn supprimer_terme(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    if lexique::supprimer(&mut tx, id).await? == 0 {
        return Err(terme_introuvable());
    }
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Parcours
// ---------------------------------------------------------------------------

fn groupe_introuvable() -> ApiError {
    ApiError::with_message(ErrorCode::NotFound, "Ce groupe du parcours n'existe pas.")
}

fn etape_introuvable() -> ApiError {
    ApiError::new(ErrorCode::NegotiationPathwayStepNotFound)
}

pub async fn parcours(
    state: &NegotiationState,
    droits: &Droits,
    locale: &str,
) -> Result<AdminPathway> {
    let mut conn = state.pool().acquire().await?;
    let mut groups = parcours::groupes(&mut conn).await?;
    for e in parcours::etapes(&mut conn, locale).await? {
        if let Some(g) = groups.iter_mut().find(|g| g.id == e.group_id) {
            g.steps.push(e);
        }
    }
    Ok(AdminPathway {
        groups,
        can_publish: droits.publier,
    })
}

pub async fn creer_groupe(
    state: &NegotiationState,
    ctx: &RequestContext,
    entree: &AdminPathwayGroupInput,
) -> Result<Uuid> {
    let label = entree
        .label
        .as_ref()
        .ok_or_else(|| ApiError::validation("Le libellé est obligatoire.", "label"))?;
    texte_valide(label, "label")?;
    let mut tx = state.db().write(ctx).await?;
    let id = parcours::creer_groupe(&mut tx, label, entree.is_published.unwrap_or(false)).await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn modifier_groupe(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    entree: &AdminPathwayGroupInput,
) -> Result<()> {
    if let Some(l) = &entree.label {
        texte_valide(l, "label")?;
    }
    let mut tx = state.db().write(ctx).await?;
    if parcours::modifier_groupe(&mut tx, id, entree.label.as_ref(), entree.is_published).await?
        == 0
    {
        return Err(groupe_introuvable());
    }
    tx.commit().await?;
    Ok(())
}

pub async fn supprimer_groupe(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    if parcours::supprimer_groupe(&mut tx, id).await? == 0 {
        return Err(groupe_introuvable());
    }
    tx.commit().await?;
    Ok(())
}

/// Le genre dit quelle colonne reçoit la cible ; la base refuse le reste
/// (`ck_pathway_steps_link`).
fn lien(l: &AdminPathwayLinkInput) -> Result<parcours::Lien<'_>> {
    if let Some(t) = &l.label {
        texte_valide(t, "link")?;
    }
    let mut lien = parcours::Lien {
        page: l.page,
        section: l
            .section
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
        label: l.label.as_ref(),
        ..Default::default()
    };
    match l.kind.as_str() {
        "document" => {
            lien.kind = Some("document");
            lien.document_id = Some(l.target_id);
        }
        "faq" => {
            lien.kind = Some("faq");
            lien.faq_id = Some(l.target_id);
        }
        "glossary" => {
            lien.kind = Some("glossary");
            lien.glossary_id = Some(l.target_id);
        }
        _ => return Err(ApiError::new(ErrorCode::NegotiationPathwayLinkInvalid).field("link")),
    }
    Ok(lien)
}

pub async fn creer_etape(
    state: &NegotiationState,
    ctx: &RequestContext,
    entree: &AdminPathwayStepInput,
) -> Result<Uuid> {
    let group_id = entree
        .group_id
        .ok_or_else(|| ApiError::validation("Le groupe est obligatoire.", "group_id"))?;
    let label = entree
        .label
        .as_ref()
        .ok_or_else(|| ApiError::validation("Le libellé est obligatoire.", "label"))?;
    texte_valide(label, "label")?;
    texte_facultatif(&entree.detail, "detail")?;
    texte_facultatif(&entree.origin_label, "origin_label")?;
    let lien = match entree.link.as_ref().and_then(Option::as_ref) {
        Some(l) => lien(l)?,
        None => parcours::Lien::default(),
    };

    let mut tx = state.db().write(ctx).await?;
    let id = parcours::creer_etape(
        &mut tx,
        &parcours::Etape {
            group_id,
            label,
            detail: entree.detail.as_ref().and_then(Option::as_ref),
            origin_label: entree.origin_label.as_ref().and_then(Option::as_ref),
            lien,
            publie: entree.is_published.unwrap_or(false),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(id)
}

pub async fn modifier_etape(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
    entree: &AdminPathwayStepInput,
) -> Result<()> {
    if let Some(l) = &entree.label {
        texte_valide(l, "label")?;
    }
    texte_facultatif(&entree.detail, "detail")?;
    texte_facultatif(&entree.origin_label, "origin_label")?;
    let nouveau_lien = match &entree.link {
        None => None,
        Some(None) => Some(parcours::Lien::default()),
        Some(Some(l)) => Some(lien(l)?),
    };

    let mut tx = state.db().write(ctx).await?;
    parcours::groupe_de(&mut tx, id)
        .await?
        .ok_or_else(etape_introuvable)?;
    parcours::modifier_etape(
        &mut tx,
        id,
        &parcours::ModificationEtape {
            group_id: entree.group_id,
            label: entree.label.as_ref(),
            detail: entree.detail.as_ref().map(Option::as_ref),
            origin_label: entree.origin_label.as_ref().map(Option::as_ref),
            lien: nouveau_lien,
            publie: entree.is_published,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Une étape cochée se dépublie : la supprimer effacerait les coches.
pub async fn supprimer_etape(
    state: &NegotiationState,
    ctx: &RequestContext,
    id: Uuid,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    parcours::groupe_de(&mut tx, id)
        .await?
        .ok_or_else(etape_introuvable)?;
    if parcours::est_cochee(&mut tx, id).await? {
        return Err(ApiError::with_message(
            ErrorCode::NegotiationKnowledgePublishedUndeletable,
            "Des comptes ont déjà coché cette étape : dépubliez-la plutôt que de la supprimer.",
        ));
    }
    parcours::supprimer_etape(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}

/// Les groupes prennent le rang de leur place ; chaque étape, celle de sa
/// place dans son groupe.
pub async fn ordonner(
    state: &NegotiationState,
    ctx: &RequestContext,
    entree: &AdminPathwayOrderInput,
) -> Result<()> {
    let mut tx = state.db().write(ctx).await?;
    for (i, g) in entree.groups.iter().enumerate() {
        let rang = i16::try_from(i).unwrap_or(i16::MAX);
        if parcours::ranger_le_groupe(&mut tx, g.id, rang).await? == 0 {
            return Err(groupe_introuvable().field("groups"));
        }
        for (j, etape) in g.step_ids.iter().enumerate() {
            let rang = i16::try_from(j).unwrap_or(i16::MAX);
            if parcours::ranger_l_etape(&mut tx, *etape, g.id, rang).await? == 0 {
                return Err(etape_introuvable().field("groups"));
            }
        }
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_revoir_ne_vaut_que_pour_une_entree_publiee() {
        assert!(arrivee(KnowledgeStatus::Draft, Transition::ARevoir).is_err());
        assert_eq!(
            arrivee(KnowledgeStatus::Published, Transition::ARevoir).unwrap(),
            "to_review"
        );
        assert_eq!(
            arrivee(KnowledgeStatus::ToReview, Transition::Publier).unwrap(),
            "published"
        );
        assert_eq!(
            arrivee(KnowledgeStatus::ToReview, Transition::Depublier).unwrap(),
            "draft"
        );
    }
}
