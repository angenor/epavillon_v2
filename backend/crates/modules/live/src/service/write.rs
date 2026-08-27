//! Les quatre écritures — leur ordre de contrôle, leur validation, leur
//! autorisation.
//!
//! # L'ORDRE DES CONTRÔLES, ET POURQUOI IL COMPTE
//!
//! 1. **Périmètre** : non vide, et `from_event_id` dedans → sinon 403 / 404.
//!    C'est le seul refus qui sort en HTTP : il ne figure pas au contrat du site
//!    et ne doit **rien révéler**.
//! 2. **Appartenance de la cible** à cette édition → sinon `missing_target`.
//!    `ck_incidents_scope_target` vérifie la cohérence portée/cible, **jamais
//!    l'appartenance à une édition** : la base est muette là-dessus.
//! 3. **Permission sur la portée VISÉE** → sinon `forbidden`, **en 200**.
//! 4. **Validation** : cohérence portée/cible, message bilingue, fenêtre.
//! 5. **Écriture**, puis publication par `live.publish_incident()` si demandée —
//!    **dans la même transaction**.
//!
//! L'ordre 2 avant 3 n'est pas indifférent : tester la permission avant de savoir
//! sur quelle édition la cible se rattache reviendrait à la tester sur la
//! mauvaise portée.
//!
//! # CE QUE LE SERVICE NE FAIT PAS
//!
//! **Il ne rejoue pas la machine à états de la publication.** Un message jamais
//! publié qu'on tente de retirer n'est pas détecté en amont : l'appel à
//! `live.unpublish_incident()` est fait, et sa levée est traduite en
//! `not_published`. La condition vit à un seul endroit.
//!
//! **Il n'émet aucun événement.** Les deux fonctions de publication le font
//! déjà, dans la transaction de l'appelant.

use kernel::auth::{has_permission, Perimeter};
use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::incident::{IncidentWriteResult, IncidentWriteStatus, ManagedIncident};
use crate::domain::payload::IncidentPayload;
use crate::repo;
use crate::service::authz::{portee_visee, INCIDENT_PUBLISH};
use crate::state::LiveState;

/// Les deux langues que l'API exige d'un message.
///
/// **C'est une règle d'INTERFACE, pas un invariant de base** : `platform.i18n_text`
/// n'exige qu'un document non nul, et c'est voulu — les données reprises de la
/// v1 n'ont qu'une langue. Un bandeau publié maintenant, lui, s'adresse à une
/// COP bilingue.
const LANGUES_EXIGEES: [&str; 2] = ["fr", "en"];

/// Le titre, tel qu'il doit être écrit.
///
/// **Vide dans les deux langues, il s'écrit NUL.** La base accepterait
/// `{"fr":"","en":""}`, que `platform.t()` rendrait comme un titre présent et
/// vide — un bandeau coiffé d'une ligne blanche.
pub fn titre_a_ecrire(titre: Option<&Value>) -> Option<Value> {
    let titre = titre?;
    let porte_un_texte = titre
        .as_object()?
        .values()
        .any(|v| v.as_str().is_some_and(|t| !t.trim().is_empty()));

    porte_un_texte.then(|| titre.clone())
}

/// Valide ce que la base ne porte pas, **et ce qu'elle porte mais ne sait pas
/// nommer**.
///
/// Les deux dernières règles ressemblent à une réimplémentation et n'en sont
/// pas : la barrière reste la contrainte, ce que le service ajoute c'est **la
/// forme du refus** — la base ignore qu'un formulaire a un champ `session_id` et
/// un champ `scope`.
pub fn valider(valeurs: &IncidentPayload) -> Option<IncidentWriteStatus> {
    // Portée et cible : exactement une cible par portée, aucune pour `global`.
    let cibles_posees = [
        valeurs.event_id,
        valeurs.event_day_id,
        valeurs.session_id,
        valeurs.organization_id,
    ]
    .iter()
    .filter(|c| c.is_some())
    .count();

    let attendu = match valeurs.scope.as_str() {
        "global" => 0,
        "event" | "event_day" | "session" | "organization" => 1,
        _ => return Some(IncidentWriteStatus::MissingTarget),
    };
    if cibles_posees != attendu || (attendu == 1 && valeurs.cible().is_none()) {
        return Some(IncidentWriteStatus::MissingTarget);
    }

    // Le message, dans les deux langues.
    let manque = LANGUES_EXIGEES.iter().any(|langue| {
        valeurs
            .message
            .get(*langue)
            .and_then(Value::as_str)
            .is_none_or(|t| t.trim().is_empty())
    });
    if manque {
        return Some(IncidentWriteStatus::MissingMessage);
    }

    // La fenêtre d'affichage. Une fin nulle est légitime — c'est « jusqu'à
    // dépublication explicite », et c'est le vrai danger de la table, que
    // l'interface signale plutôt que le modèle ne l'interdise.
    if let Some(fin) = valeurs.display_until {
        if fin <= valeurs.display_from {
            return Some(IncidentWriteStatus::InvalidWindow);
        }
    }

    None
}

/// Traduit la levée de `live.unpublish_incident()` en issue de contrat.
///
/// La fonction lève `no_data_found` sur un message inexistant **ou** jamais
/// publié. À ce point du chemin, l'existence a déjà été vérifiée dans le
/// périmètre : il ne reste que la seconde cause.
///
/// **Tout autre refus repart tel quel** : il sortira du catalogue, avec son code
/// stable et son message français.
pub fn issue_de_depublication(
    erreur: ApiError,
) -> std::result::Result<IncidentWriteStatus, ApiError> {
    match erreur.code {
        ErrorCode::NotFound | ErrorCode::LiveIncidentNotPublished => {
            Ok(IncidentWriteStatus::NotPublished)
        }
        _ => Err(erreur),
    }
}

// ---------------------------------------------------------------------------
// Les quatre écritures
//
// Elles vivent ICI et non dans les routes, et ce n'est pas une préférence de
// rangement : le périmètre, l'autorisation sur la portée visée et la ligne entre
// un refus HTTP et une issue en 200 sont **le sujet** de ce jalon. Les laisser
// dans un gestionnaire Actix les rendrait inéprouvables sans monter
// l'application entière — donc sans une arête de développement vers `api`, que
// le contrôle de frontière du principe II refuse.
// ---------------------------------------------------------------------------

/// Périmètre vide → **403**, et jamais une liste vide : les trois cas du
/// périmètre restent distincts.
pub fn refuser_un_perimetre_vide(perimetre: &Perimeter) -> Result<()> {
    if perimetre.scope.is_empty() {
        return Err(ApiError::forbidden());
    }
    Ok(())
}

/// Périmètre vide → 403 ; édition hors périmètre → **404**, jamais 403 : un
/// identifiant hors périmètre se refuse comme un identifiant inexistant.
pub fn assurer_le_perimetre(perimetre: &Perimeter, event_id: Uuid) -> Result<()> {
    refuser_un_perimetre_vide(perimetre)?;
    perimetre.ensure(event_id)
}

/// Le message, retrouvé **par la fonction** sur le périmètre — ce qui rend le
/// contrôle et la lecture indissociables.
pub async fn charger_dans_le_perimetre(
    state: &LiveState,
    perimetre: &Perimeter,
    id: Uuid,
) -> Result<Option<ManagedIncident>> {
    let mut tx = repo::lecture(state.pool()).await?;
    let ligne = if perimetre.scope.is_global {
        repo::incidents::n_importe_ou(&mut tx, id).await?
    } else {
        repo::incidents::dans_le_perimetre(&mut tx, id, &perimetre.scope.event_ids).await?
    };
    tx.commit().await?;
    Ok(ligne)
}

/// `live.incident.publish` sur la portée **du message tel qu'il est**.
async fn autorise_sur(
    state: &LiveState,
    perimetre: &Perimeter,
    ligne: &ManagedIncident,
) -> Result<bool> {
    let mut tx = repo::lecture(state.pool()).await?;
    let edition = repo::incidents::edition_du_message(&mut tx, ligne.incident_id).await?;
    tx.commit().await?;

    let Some(portee) = portee_visee(&ligne.scope, edition) else {
        return Ok(false);
    };
    has_permission(state.pool(), perimetre.person_id, INCIDENT_PUBLISH, portee).await
}

/// Rédiger, et publier dans le même geste si c'est demandé.
pub async fn creer(
    state: &LiveState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    depuis: Uuid,
    valeurs: &IncidentPayload,
) -> Result<IncidentWriteResult> {
    assurer_le_perimetre(perimetre, depuis)?;
    ecrire(state, ctx, perimetre, depuis, valeurs, None).await
}

/// Corriger. **La portée peut changer**, et l'autorisation se vérifie alors sur
/// celle d'ARRIVÉE : déplacer un message vers la portée globale exige la
/// permission globale.
pub async fn corriger(
    state: &LiveState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    depuis: Uuid,
    id: Uuid,
    valeurs: &IncidentPayload,
) -> Result<IncidentWriteResult> {
    assurer_le_perimetre(perimetre, depuis)?;

    // La SOURCE d'abord : un message qu'on n'a pas le droit de voir ne se
    // corrige pas, même vers une portée qu'on administre.
    if charger_dans_le_perimetre(state, perimetre, id)
        .await?
        .is_none()
    {
        return Ok(IncidentWriteResult::refuse(IncidentWriteStatus::NotFound));
    }

    ecrire(state, ctx, perimetre, depuis, valeurs, Some(id)).await
}

/// Publier un brouillon, ou rétablir un message retiré.
pub async fn publier(
    state: &LiveState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    id: Uuid,
) -> Result<IncidentWriteResult> {
    refuser_un_perimetre_vide(perimetre)?;

    let Some(ligne) = charger_dans_le_perimetre(state, perimetre, id).await? else {
        return Ok(IncidentWriteResult::refuse(IncidentWriteStatus::NotFound));
    };
    if !autorise_sur(state, perimetre, &ligne).await? {
        return Ok(IncidentWriteResult::refuse(IncidentWriteStatus::Forbidden));
    }

    let mut tx = state.db().write(ctx).await?;
    repo::incidents::publier(&mut tx, id).await?;
    tx.commit().await?;

    let relue = charger_dans_le_perimetre(state, perimetre, id).await?;
    Ok(IncidentWriteResult::abouti(
        IncidentWriteStatus::Published,
        relue,
    ))
}

/// Retirer un bandeau, avec son motif. **Ce n'est pas une suppression.**
pub async fn depublier(
    state: &LiveState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    id: Uuid,
    motif: Option<&str>,
) -> Result<IncidentWriteResult> {
    refuser_un_perimetre_vide(perimetre)?;

    let Some(ligne) = charger_dans_le_perimetre(state, perimetre, id).await? else {
        return Ok(IncidentWriteResult::refuse(IncidentWriteStatus::NotFound));
    };
    if !autorise_sur(state, perimetre, &ligne).await? {
        return Ok(IncidentWriteResult::refuse(IncidentWriteStatus::Forbidden));
    }

    let mut tx = state.db().write(ctx).await?;
    match repo::incidents::depublier(&mut tx, id, motif).await {
        Ok(()) => {
            tx.commit().await?;
            let relue = charger_dans_le_perimetre(state, perimetre, id).await?;
            Ok(IncidentWriteResult::abouti(
                IncidentWriteStatus::Unpublished,
                relue,
            ))
        }
        Err(erreur) => {
            // La transaction est abandonnée : la levée l'a déjà invalidée.
            drop(tx);
            Ok(IncidentWriteResult::refuse(issue_de_depublication(erreur)?))
        }
    }
}

/// Le chemin nominal des deux écritures de contenu : cible, permission,
/// validation, écriture, publication — **dans cet ordre**.
async fn ecrire(
    state: &LiveState,
    ctx: &RequestContext,
    perimetre: &Perimeter,
    depuis: Uuid,
    valeurs: &IncidentPayload,
    corriger: Option<Uuid>,
) -> Result<IncidentWriteResult> {
    let mut tx = repo::lecture(state.pool()).await?;
    let edition_cible =
        repo::incidents::edition_de_la_cible(&mut tx, &valeurs.scope, valeurs.cible(), depuis)
            .await?;
    tx.commit().await?;

    // **La cible doit appartenir à l'édition depuis laquelle on agit.**
    // `ck_incidents_scope_target` vérifie la cohérence portée/cible, jamais
    // l'appartenance à une édition : la base est muette là-dessus. Et le refus
    // ne NOMME rien — citer « COP30 — Bakou » à un compte détaché sur la COP31
    // lui apprendrait qu'elle existe.
    if valeurs.scope != "global" && edition_cible != Some(depuis) {
        return Ok(IncidentWriteResult::refuse(
            IncidentWriteStatus::MissingTarget,
        ));
    }

    let Some(portee) = portee_visee(&valeurs.scope, edition_cible) else {
        return Ok(IncidentWriteResult::refuse(
            IncidentWriteStatus::MissingTarget,
        ));
    };
    if !has_permission(state.pool(), perimetre.person_id, INCIDENT_PUBLISH, portee).await? {
        return Ok(IncidentWriteResult::refuse(IncidentWriteStatus::Forbidden));
    }

    if let Some(refuse) = valider(valeurs) {
        return Ok(IncidentWriteResult::refuse(refuse));
    }

    let titre = titre_a_ecrire(valeurs.title.as_ref());

    let mut tx = state.db().write(ctx).await?;
    let id = match corriger {
        Some(id) => {
            repo::incidents::modifier(&mut tx, id, valeurs, titre.as_ref()).await?;
            id
        }
        None => {
            repo::incidents::creer(&mut tx, valeurs, titre.as_ref(), perimetre.person_id).await?
        }
    };
    // **Dans la MÊME transaction** : un message enregistré sans sa publication
    // laisserait un brouillon là où quelqu'un croit avoir parlé.
    if valeurs.publish {
        repo::incidents::publier(&mut tx, id).await?;
    }
    tx.commit().await?;

    let status = match (corriger.is_some(), valeurs.publish) {
        (_, true) => IncidentWriteStatus::Published,
        (true, false) => IncidentWriteStatus::Updated,
        (false, false) => IncidentWriteStatus::Created,
    };

    let relue = charger_dans_le_perimetre(state, perimetre, id).await?;
    Ok(IncidentWriteResult::abouti(status, relue))
}
