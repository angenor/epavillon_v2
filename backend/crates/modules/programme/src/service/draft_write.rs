//! La composition d'un enregistrement — ce qui se décide avant d'écrire.
//!
//! # Ce que ce fichier tient, et que la base ne tient pas
//!
//! Sept règles, toutes listées en `data-model.md` § 3 avec leur écart : les
//! bornes d'intervenants et de durée de l'appel (n° 27), les longueurs des
//! textes (n° 28), l'assainissement du HTML (n° 32), le verrouillage d'identité
//! (n° 31), le contact par défaut (n° 30), le refus d'ajouter le porteur comme
//! co-organisateur, et la plage horaire quotidienne. **Aucune n'est un
//! redoublement** : la base ne les porte pas du tout.
//!
//! # Trois choses qui se jouent à l'écriture, et nulle part ailleurs
//!
//! **Les textes sont en français.** `platform.i18n_text` exige la clé `fr` non
//! vide ; le brouillon porte des chaînes et l'écriture les enveloppe. La
//! traduction anglaise est un travail éditorial de l'IFDD, pas une seconde
//! colonne à remplir par le déposant.
//!
//! **Le créneau est une heure murale**, convertie en base dans le fuseau de
//! **l'édition** (R6). Un créneau saisi à 14:30 à Belém se rouvrirait à 11:30
//! pour qui corrige depuis Dakar, sans qu'aucune erreur ne soit levée.
//!
//! **Une seule annonce, et par organisation ajoutée** :
//! `programme.coorganization.requested`. Rien d'autre n'est émis — le
//! changement d'état, lui, est annoncé par le déclencheur, et le redire
//! enverrait tout en double.

use kernel::context::RequestContext;
use kernel::error::{ApiError, ErrorCode, Result};
use kernel::events::{self, DomainEvent};
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::draft::{self, DraftSpeaker, ProposalDraft, SaveDraftPayload};
use crate::domain::ids::{EventId, ProposalId};
use crate::domain::limits::{self, Borne};
use crate::domain::{ownership, sanitize};
use crate::repo::cross::{ContexteEdition, ReglesDeLAppel};
use crate::repo::proposals::{ChampsDuDossier, Enregistrement, NouveauDossier};
use crate::repo::{cross, organizations, people, proposals, speakers, themes};
use crate::state::ProgrammeState;

/// Enregistrer un brouillon — création au premier appel, mise à jour ensuite.
///
/// **L'état n'est jamais touché** : corriger n'est pas déposer.
pub async fn enregistrer(
    state: &ProgrammeState,
    ctx: &RequestContext,
    acteur: Uuid,
    payload: SaveDraftPayload,
) -> Result<Enregistrement> {
    let (regles, edition, existant) = contexte(state, acteur, &payload).await?;
    let porteur = porteur_du_dossier(&payload, existant.as_ref())?;

    // L'adhésion active est vérifiée sur l'organisation PORTEUSE, résolue en
    // base pour un dossier existant : le corps ne décide pas de qui peut
    // écrire.
    let adhesion = cross::adhesion(state.pool(), porteur, acteur).await?;
    ownership::exiger(adhesion)?;

    let champs = composer(&payload.draft, &regles, &edition, acteur, porteur)?;
    let titre_brut = payload.draft.title.trim().to_owned();

    let mut tx = state.db().write(ctx).await?;

    let (dossier, ligne) = match &existant {
        None => {
            let nouveau = NouveauDossier {
                call_id: regles.call_id,
                event_id: EventId::from(regles.event_id),
                organization_id: porteur,
                submitted_by: acteur,
                titre_brut: titre_brut.clone(),
            };
            let ligne = proposals::creer(&mut tx, &nouveau, &champs).await?;
            (ProposalId(ligne.proposal_id), ligne)
        }
        Some(etat) => {
            let dossier = ProposalId(etat.id);
            // L'adresse suit le titre TANT QUE le dossier est en brouillon, et
            // se fige au dépôt : une adresse déjà communiquée ne change pas
            // sous une correction de titre.
            let refaire = etat.status == "draft";
            let ligne =
                proposals::mettre_a_jour(&mut tx, dossier, &champs, &titre_brut, refaire).await?;
            (dossier, ligne)
        }
    };

    themes::poser(&mut tx, dossier.as_uuid(), &payload.draft.theme_codes).await?;

    let associations = associations(&payload.draft, porteur)?;
    let ajoutees = organizations::remplacer(&mut tx, dossier, acteur, &associations).await?;

    let intervenants = resoudre_intervenants(&mut tx, &payload.draft.speakers).await?;
    let lignes: Vec<speakers::Intervenant<'_>> = intervenants
        .iter()
        .enumerate()
        .map(|(rang, i)| speakers::Intervenant {
            person_id: i.person_id,
            role: &i.role,
            job_title_snapshot: non_vide(&i.job_title),
            organization_snapshot: non_vide(&i.organization_name),
            organization_id: i.organization_id,
            bio: draft::i18n(&i.bio),
            sort_order: rang as i16,
        })
        .collect();
    speakers::remplacer(&mut tx, dossier, &lignes).await?;

    annoncer_les_coorganisations(
        &mut tx,
        &ligne,
        regles.event_id,
        porteur,
        &ajoutees,
        &associations,
    )
    .await?;

    // 🔴 **AUCUN CHAMP N'EST PROPAGÉ VERS UNE SÉANCE PROGRAMMÉE**, et c'est ici
    // que la tentation existe : `programme.sessions` porte `proposal_id`, un
    // titre, un format et un créneau, et il paraît naturel de « tenir la séance
    // à jour » quand le dossier change.
    //
    // Ce serait une faute. Une séance retenue a un créneau **arbitré** par
    // l'IFDD, une salle attribuée, des inscrits prévenus et des rappels
    // programmés. Recopier dessus le créneau *souhaité* d'un dossier corrigé
    // déplacerait une séance à laquelle quarante personnes se sont inscrites,
    // **sans que personne l'ait demandé** (FR-091). Le dossier est la demande,
    // la séance est la décision : corriger la demande ne rejoue pas la
    // décision.
    //
    // C'est aussi pour cela que ce service n'émet aucun événement de
    // modification : rien ne doit pouvoir s'y abonner pour « synchroniser ».

    tx.commit().await?;

    Ok(ligne)
}

// -----------------------------------------------------------------------------
// Ce qu'il faut savoir avant de composer
// -----------------------------------------------------------------------------

type Contexte = (
    ReglesDeLAppel,
    ContexteEdition,
    Option<proposals::EtatDuDossier>,
);

/// L'appel, l'édition et le dossier existant — **tous lus en base**.
///
/// L'appel vient de l'identifiant du corps, mais **l'édition vient de
/// l'appel** : le `event_id` que le front envoie n'est jamais lu. Pour un
/// dossier existant, l'appel vient du dossier — le corps ne peut pas le
/// déplacer d'une campagne à l'autre.
async fn contexte(
    state: &ProgrammeState,
    acteur: Uuid,
    payload: &SaveDraftPayload,
) -> Result<Contexte> {
    let existant = match payload.proposal_id {
        None => None,
        Some(id) => Some(
            proposals::etat(state.pool(), ProposalId(id))
                .await?
                .ok_or_else(ApiError::not_found)?,
        ),
    };

    if let Some(etat) = &existant {
        if !crate::domain::transitions::ProposalStatus::from_db(&etat.status)
            .is_some_and(|s| s.est_modifiable())
        {
            return Err(ApiError::new(ErrorCode::ProposalNotEditable));
        }
    }

    let call_id = existant
        .as_ref()
        .and_then(|e| e.call_id)
        .unwrap_or(payload.call_id);

    let regles = cross::regles_de_lappel(state.pool(), call_id)
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ProposalUnknownReference).field("call_id"))?;

    let edition = cross::contexte_edition(state.pool(), EventId::from(regles.event_id))
        .await?
        .ok_or_else(|| ApiError::new(ErrorCode::ProposalUnknownReference).field("event_id"))?;

    // **Tant que l'ÉDITION n'est pas terminée, on peut corriger** — arbitrage du
    // commanditaire, 17/08. Ce n'est pas la fenêtre de l'appel : elle borne le
    // dépôt, pas la correction, et une organisation qui repère une coquille
    // trois jours avant sa séance doit pouvoir la rectifier.
    //
    // La base ne tient cette règle nulle part : `proposals` n'a aucune
    // contrainte liée à `events.ends_at`.
    if edition.ends_at < OffsetDateTime::now_utc() {
        return Err(ApiError::with_message(
            ErrorCode::ProposalNotEditable,
            "Cette édition est terminée : son dossier ne se modifie plus.",
        ));
    }

    let _ = acteur;
    Ok((regles, edition, existant))
}

/// L'organisation porteuse — **de la base pour un dossier existant**, du corps
/// pour une création. Un dossier déjà créé ne change pas de porteur : le
/// déclencheur de synchronisation le suivrait, et la ligne de rôle `lead`
/// basculerait sans que personne l'ait demandé.
fn porteur_du_dossier(
    payload: &SaveDraftPayload,
    existant: Option<&proposals::EtatDuDossier>,
) -> Result<Uuid> {
    match existant {
        Some(etat) => Ok(etat.organization_id),
        None => payload.draft.organization_id.ok_or_else(|| {
            ApiError::with_message(
                ErrorCode::ValidationFailed,
                "L'organisation porteuse est obligatoire.",
            )
            .field("organization_id")
        }),
    }
}

// -----------------------------------------------------------------------------
// La composition
// -----------------------------------------------------------------------------

fn composer(
    brouillon: &ProposalDraft,
    regles: &ReglesDeLAppel,
    edition: &ContexteEdition,
    deposant: Uuid,
    _porteur: Uuid,
) -> Result<ChampsDuDossier> {
    borner(&brouillon.title, &limits::TITRE)?;
    borner(&brouillon.summary, &limits::RESUME)?;
    borner(&brouillon.objectives, &limits::OBJECTIFS)?;
    borner(&brouillon.expected_outcomes, &limits::RESULTATS)?;
    borner(&brouillon.scheduling_constraints, &limits::CONTRAINTES)?;
    for public in &brouillon.target_audiences {
        borner(public, &limits::PUBLIC_VISE)?;
    }

    // L'assainissement PRÉCÈDE la mesure : un fragment qu'on mesure avant de le
    // nettoyer se ferait refuser pour du balisage qui n'allait pas être écrit.
    let presentation = sanitize::assainir(&brouillon.detailed_presentation);
    if !limits::tient(&sanitize::texte_seul(&presentation), &limits::PRESENTATION) {
        return Err(trop_long(&limits::PRESENTATION));
    }

    let format = format_admis(brouillon, regles)?;
    let duree = duree_admise(brouillon, regles)?;
    let creneau = creneau_admis(brouillon, regles, duree)?;

    Ok(ChampsDuDossier {
        title: draft::i18n_de_brouillon(&brouillon.title, draft::TITRE_PROVISOIRE),
        summary: draft::i18n(&brouillon.summary),
        objectives: draft::i18n_de_brouillon(&brouillon.objectives, draft::TEXTE_PROVISOIRE),
        detailed_presentation: draft::i18n_de_brouillon(
            &sans_balisage_vide(&presentation),
            draft::TEXTE_PROVISOIRE,
        ),
        expected_outcomes: draft::i18n(&brouillon.expected_outcomes),
        target_audiences: draft::i18n_liste(&brouillon.target_audiences),
        format,
        activity_type_code: brouillon.activity_type_code.clone(),
        language_codes: if brouillon.language_codes.is_empty() {
            vec!["fr".to_owned()]
        } else {
            brouillon.language_codes.clone()
        },
        country_id: brouillon.country_id,
        creneau,
        fuseau: edition.timezone.clone(),
        duration_minutes: duree,
        requested_sessions: brouillon.requested_sessions.max(1),
        scheduling_constraints: non_vide(&brouillon.scheduling_constraints).map(str::to_owned),
        // **Le contact du dossier est le déposant par défaut**, et c'est une
        // règle explicite : la colonne est nullable et rien ne la remplit
        // (écart n° 30). Le demander à l'étape des organisations est un geste
        // d'écran, pas d'API.
        contact_person_id: Some(deposant),
    })
}

/// Un fragment que l'éditeur rend « vide » est `<p></p>` et non la chaîne vide.
fn sans_balisage_vide(html: &str) -> String {
    if sanitize::est_vide(html) {
        String::new()
    } else {
        html.to_owned()
    }
}

fn borner(texte: &str, borne: &Borne) -> Result<()> {
    if limits::tient(texte, borne) {
        Ok(())
    } else {
        Err(trop_long(borne))
    }
}

fn trop_long(borne: &Borne) -> ApiError {
    ApiError::with_message(
        ErrorCode::ProposalTextTooLong,
        format!(
            "Ce texte dépasse la longueur autorisée : {} caractères au maximum.",
            borne.max
        ),
    )
    .field(borne.champ)
}

fn non_vide(texte: &str) -> Option<&str> {
    let coupe = texte.trim();
    (!coupe.is_empty()).then_some(coupe)
}

/// Le format doit être **admis par l'appel**. La colonne du dossier accepte les
/// trois ; l'appel, lui, peut n'en offrir qu'un — un cycle de webinaires ne
/// reçoit pas de séance en présentiel.
fn format_admis(brouillon: &ProposalDraft, regles: &ReglesDeLAppel) -> Result<String> {
    let format = brouillon.format.clone().unwrap_or_else(|| {
        regles
            .allowed_formats
            .first()
            .cloned()
            .unwrap_or_else(|| "online".to_owned())
    });

    if regles.allowed_formats.iter().any(|f| f == &format) {
        Ok(format)
    } else {
        Err(ApiError::with_message(
            ErrorCode::ValidationFailed,
            "Cet appel n'accepte pas ce format d'activité.",
        )
        .field("format"))
    }
}

/// La durée doit tenir dans les bornes **de l'appel**, plus serrées que celles
/// de la colonne (15 à 600) : ce sont des règles de campagne, pas de données.
fn duree_admise(brouillon: &ProposalDraft, regles: &ReglesDeLAppel) -> Result<Option<i16>> {
    let Some(duree) = brouillon.duration_minutes else {
        return Ok(None);
    };

    if duree < regles.min_duration_minutes || duree > regles.max_duration_minutes {
        return Err(ApiError::with_message(
            ErrorCode::ValidationFailed,
            format!(
                "La durée doit être comprise entre {} et {} minutes.",
                regles.min_duration_minutes, regles.max_duration_minutes
            ),
        )
        .field("duration_minutes"));
    }

    Ok(Some(duree))
}

/// Le créneau doit tenir dans la **plage horaire quotidienne** de l'appel,
/// **fin comprise** : c'est la contrainte matérielle d'un stand ouvert de 9 h à
/// 17 h, qu'une activité peut atteindre mais pas dépasser.
///
/// La comparaison se fait en heure **murale**, celle que le déposant a saisie
/// et que l'appel décrit — les deux sont dans le fuseau de l'édition. Aucune
/// conversion n'est donc nécessaire, et c'est ce qui la rend sûre.
fn creneau_admis(
    brouillon: &ProposalDraft,
    regles: &ReglesDeLAppel,
    duree: Option<i16>,
) -> Result<Option<(String, String)>> {
    let Some(saisi) = brouillon.preferred_start_at.as_deref() else {
        return Ok(None);
    };
    if saisi.trim().is_empty() {
        return Ok(None);
    }

    let (date, heure) = draft::heure_murale(saisi).ok_or_else(|| {
        ApiError::with_message(
            ErrorCode::ValidationFailed,
            "Le créneau souhaité doit être une date et une heure (AAAA-MM-JJTHH:MM).",
        )
        .field("preferred_start_at")
    })?;

    let debut = time::Time::parse(
        heure,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )
    .map_err(|_| {
        ApiError::with_message(ErrorCode::ValidationFailed, "L'heure saisie est invalide.")
            .field("preferred_start_at")
    })?;

    if debut < regles.daily_start_time {
        return Err(hors_plage(regles));
    }

    if let Some(duree) = duree {
        let fin = debut + time::Duration::minutes(i64::from(duree));
        // Une activité qui franchit minuit sort de la plage par construction :
        // l'addition repasse avant l'heure d'ouverture, et la comparaison le dit.
        if fin < debut || fin > regles.daily_end_time {
            return Err(hors_plage(regles));
        }
    }

    Ok(Some((date.to_owned(), heure.to_owned())))
}

fn hors_plage(regles: &ReglesDeLAppel) -> ApiError {
    ApiError::with_message(
        ErrorCode::ValidationFailed,
        format!(
            "L'activité doit se tenir entre {} et {}, heure du lieu de l'édition.",
            regles.daily_start_time, regles.daily_end_time
        ),
    )
    .field("preferred_start_at")
}

// -----------------------------------------------------------------------------
// Les co-organisations
// -----------------------------------------------------------------------------

/// **Le porteur ne peut pas être co-organisateur de son propre dossier**, et le
/// refuser explicitement n'est pas une précaution de style : le `ON CONFLICT`
/// du déclencheur de synchronisation ferait basculer la ligne en `lead` au
/// prochain enregistrement, en silence, et le dossier perdrait un
/// co-organisateur sans qu'aucune erreur ne le dise.
fn associations(
    brouillon: &ProposalDraft,
    porteur: Uuid,
) -> Result<Vec<organizations::Association<'_>>> {
    let mut vues = std::collections::HashSet::new();
    let mut associations = Vec::with_capacity(brouillon.co_organizations.len());

    for (rang, ligne) in brouillon.co_organizations.iter().enumerate() {
        if ligne.organization_id == porteur {
            return Err(ApiError::with_message(
                ErrorCode::ValidationFailed,
                "L'organisation porteuse est déjà associée au dossier : elle ne peut pas y figurer une seconde fois.",
            )
            .field("co_organizations"));
        }
        if ligne.role == "lead" {
            return Err(ApiError::with_message(
                ErrorCode::ValidationFailed,
                "Le porteur principal ne se désigne pas dans la liste des co-organisations.",
            )
            .field("co_organizations"));
        }
        if !vues.insert(ligne.organization_id) {
            return Err(ApiError::with_message(
                ErrorCode::ValidationFailed,
                "Une même organisation ne peut être associée qu'une fois.",
            )
            .field("co_organizations"));
        }

        associations.push(organizations::Association {
            organization_id: ligne.organization_id,
            role: &ligne.role,
            sort_order: rang as i16,
        });
    }

    Ok(associations)
}

/// **Un événement par organisation ajoutée, et rien d'autre.**
///
/// Le déclencheur d'état annonce déjà les changements d'état ; le service n'en
/// émet aucun. Ce qu'il annonce ici, la base ne l'annonce pas : une
/// co-organisation **engage un tiers**, et le front dit déjà « sera invitée à
/// confirmer sa participation ».
async fn annoncer_les_coorganisations(
    conn: &mut PgConnection,
    ligne: &Enregistrement,
    event_id: Uuid,
    porteur: Uuid,
    ajoutees: &[Uuid],
    associations: &[organizations::Association<'_>],
) -> Result<()> {
    for organisation in ajoutees {
        let role = associations
            .iter()
            .find(|a| &a.organization_id == organisation)
            .map(|a| a.role)
            .unwrap_or("co_organizer");

        events::emit(
            conn,
            DomainEvent {
                aggregate_schema: contracts::programme::AGGREGATE_SCHEMA,
                aggregate_type: contracts::programme::AGGREGATE_PROPOSAL,
                aggregate_id: ligne.proposal_id,
                event_type: contracts::programme::COORGANIZATION_REQUESTED,
                payload: serde_json::to_value(contracts::programme::CoorganizationRequested {
                    proposal_id: ligne.proposal_id,
                    reference_code: ligne.reference_code.clone(),
                    event_id,
                    organization_id: *organisation,
                    role: role.to_owned(),
                    lead_organization_id: porteur,
                })
                .map_err(ApiError::internal)?,
            },
        )
        .await?;
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// Les intervenants
// -----------------------------------------------------------------------------

pub struct IntervenantResolu {
    pub person_id: Uuid,
    pub role: String,
    pub job_title: String,
    pub organization_name: String,
    pub organization_id: Option<Uuid>,
    pub bio: String,
}

/// Rapprocher par l'adresse, créer sinon — et **ne jamais réécrire l'identité
/// de qui possède un compte** (écart n° 31).
///
/// Trois cas, et trois traitements distincts :
///
/// - **inconnue** : la personne est créée avec le prénom et le nom **saisis**.
///   Rien n'est déduit de l'adresse : un « a.diallo » extrait d'un courriel est
///   un nom que plus personne ne corrigera, et il s'afficherait sur toutes ses
///   participations futures (FR-026) ;
/// - **connue, sans compte** : l'identité reste corrigeable, et la correction
///   est écrite — un enregistrement qui réussit sans rien changer serait pire
///   qu'un refus ;
/// - **connue, avec compte** : l'identité appartient à son titulaire. Une
///   différence est **refusée en nommant la personne**, jamais ignorée.
///
/// Les deux instantanés — fonction et organisation au moment de l'activité —
/// restent modifiables **dans tous les cas** : le modèle les distingue
/// explicitement de la fiche.
async fn resoudre_intervenants(
    conn: &mut PgConnection,
    saisis: &[DraftSpeaker],
) -> Result<Vec<IntervenantResolu>> {
    let mut resolus = Vec::with_capacity(saisis.len());

    for saisi in saisis {
        let email = saisi.email.trim().to_lowercase();
        if email.is_empty() {
            return Err(ApiError::with_message(
                ErrorCode::ValidationFailed,
                "L'adresse électronique d'un intervenant est obligatoire : c'est par elle que la plateforme le reconnaît.",
            )
            .field("speakers"));
        }
        let prenom = saisi.first_name.trim();
        let nom = saisi.last_name.trim();
        if prenom.is_empty() || nom.is_empty() {
            return Err(ApiError::with_message(
                ErrorCode::ValidationFailed,
                "Le prénom et le nom d'un intervenant sont obligatoires.",
            )
            .field("speakers"));
        }
        borner(&saisi.bio, &limits::BIOGRAPHIE)?;

        let person_id = match cross::fiche_personne_par_email(&mut *conn, &email).await? {
            None => {
                people::trouver_ou_creer(
                    &mut *conn,
                    people::IdentiteSaisie {
                        email: &email,
                        first_name: prenom,
                        last_name: nom,
                        civility: saisi.civility.as_deref(),
                    },
                )
                .await?
            }
            Some(fiche) => {
                let change = fiche.first_name.trim() != prenom || fiche.last_name.trim() != nom;
                if change && fiche.has_account {
                    return Err(ApiError::with_message(
                        ErrorCode::ProposalSpeakerIdentityLocked,
                        format!(
                            "{} {} possède un compte : son identité lui appartient et ne se modifie pas depuis un dossier.",
                            fiche.first_name, fiche.last_name
                        ),
                    )
                    .field("speakers"));
                }
                if change {
                    people::corriger_identite(
                        &mut *conn,
                        fiche.id,
                        prenom,
                        nom,
                        saisi.civility.as_deref(),
                    )
                    .await?;
                }
                fiche.id
            }
        };

        resolus.push(IntervenantResolu {
            person_id,
            role: saisi.role.clone(),
            job_title: saisi.job_title.clone(),
            organization_name: saisi.organization_name.clone(),
            organization_id: saisi.organization_id,
            bio: saisi.bio.clone(),
        });
    }

    Ok(resolus)
}
