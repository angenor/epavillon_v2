//! Le brouillon tel qu'il circule, et ses conversions (R6).
//!
//! # Trois décisions de contrat, et chacune vient du modèle
//!
//! **Les textes se saisissent en français.** `platform.i18n_text` exige la clé
//! `fr` non vide : un dossier rédigé en anglais seul serait refusé par la base,
//! pas par l'écran. Le brouillon porte donc des `String`, et l'écriture les
//! enveloppe en `{ "fr": … }`. La traduction anglaise est un travail éditorial
//! de l'IFDD, pas une seconde colonne de formulaire à remplir par le déposant
//! (écart n° 29 — arbitrage du commanditaire, pas ligne de code).
//!
//! **Le créneau est une heure MURALE, pas un instant.** On saisit « le 12
//! novembre à 14:30 à Belém » ; la conversion en `timestamptz` se fait **en
//! base**, avec le fuseau de l'ÉDITION. Le coût d'une erreur est parfaitement
//! concret : un créneau saisi à 14:30 à Belém se rouvre à 11:30 pour qui
//! corrige depuis Dakar, **sans qu'aucune erreur ne soit levée**. Aucune
//! arithmétique de fuseau n'est écrite en Rust — c'est la décision R5 de B3,
//! reprise pour la même raison.
//!
//! **Un intervenant EST une personne.** `proposal_speakers.person_id` est
//! `NOT NULL` : le brouillon ne porte donc pas un identifiant mais **de quoi la
//! retrouver ou la créer**.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Une organisation associée, telle que l'étape 1 la retient.
///
/// **Jamais `lead`** : le porteur principal est `ProposalDraft.organization_id`,
/// et sa ligne de rôle est posée par déclencheur. Accepter `lead` ici le ferait
/// basculer en silence par le `ON CONFLICT` du déclencheur de synchronisation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DraftOrganization {
    pub organization_id: Uuid,
    /// `co_organizer`, `partner` ou `sponsor`.
    pub role: String,
}

/// Un intervenant en cours de saisie.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DraftSpeaker {
    /// Personne existante retenue, ou absente pour quelqu'un que ce dossier
    /// fait connaître à la plateforme.
    #[serde(default)]
    pub person_id: Option<Uuid>,
    pub civility: Option<String>,
    pub first_name: String,
    pub last_name: String,
    /// Clé de rapprochement — `identity.people.primary_email`.
    pub email: String,
    /// `proposal_speakers.job_title_snapshot` : la fonction **au moment** de
    /// l'activité. Distincte de la fiche de la personne, et modifiable même
    /// quand l'identité est verrouillée.
    #[serde(default)]
    pub job_title: String,
    /// `proposal_speakers.organization_snapshot`.
    #[serde(default)]
    pub organization_name: String,
    #[serde(default)]
    pub organization_id: Option<Uuid>,
    pub role: String,
    #[serde(default)]
    pub bio: String,
}

/// L'état complet du formulaire. Chaque champ porte le nom de sa colonne quand
/// elle existe : c'est ce qui rend le raccordement mécanique.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProposalDraft {
    // — Étape 1 : organisations
    pub organization_id: Option<Uuid>,
    /// **Jamais sérialisé.** À la relecture d'un dossier, `BrouillonRecompose`
    /// rend une forme ENRICHIE des co-organisations — leur dénomination, leur
    /// pays — sous la même clé. Sans ce `skip`, le JSON portait deux fois
    /// `co_organizations` : la liste vide d'ici, puis la vraie, et seule la
    /// dernière écrite survivait. Cela fonctionnait par accident.
    #[serde(default, skip_serializing)]
    pub co_organizations: Vec<DraftOrganization>,

    // — Étape 2 : présentation, en français
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub objectives: String,
    /// HTML restreint produit par l'éditeur du front. Assaini à l'écriture.
    #[serde(default)]
    pub detailed_presentation: String,
    #[serde(default)]
    pub expected_outcomes: String,
    /// Publics visés, **un par entrée**. Une chaîne unique « Ministères, ONG,
    /// journalistes » ne se réaffiche pas : elle s'imprime telle quelle, ne se
    /// compte pas, ne se filtre pas, et se découpe à la virgule par quiconque
    /// essaie — ce que la v1 faisait dans ses gabarits.
    #[serde(default)]
    pub target_audiences: Vec<String>,

    // — Étape 3 : classification
    /// Codes de `reference.taxonomy_terms`, taxonomie `activity_theme`. **Les
    /// codes seuls** : le triplet d'entité est posé littéralement par le
    /// service, jamais reçu (R11).
    #[serde(default)]
    pub theme_codes: Vec<String>,
    #[serde(default)]
    pub activity_type_code: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub language_codes: Vec<String>,
    #[serde(default)]
    pub country_id: Option<Uuid>,

    // — Étape 4 : intervenants
    /// **Jamais sérialisé**, même motif que `co_organizations` ci-dessus : la
    /// relecture rend une forme enrichie du verrouillage d'identité sous la même
    /// clé, et les deux se marchaient dessus.
    #[serde(default, skip_serializing)]
    pub speakers: Vec<DraftSpeaker>,

    // — Étape 5 : créneau souhaité
    /// Heure **murale** `AAAA-MM-JJTHH:MM`, dans le fuseau de l'édition. Ni
    /// instant, ni décalage : voir l'en-tête de ce fichier.
    #[serde(default)]
    pub preferred_start_at: Option<String>,
    #[serde(default)]
    pub duration_minutes: Option<i16>,
    #[serde(default = "une_seance")]
    pub requested_sessions: i16,
    #[serde(default)]
    pub scheduling_constraints: String,
}

fn une_seance() -> i16 {
    1
}

/// Un texte français prêt pour `platform.i18n_text`, ou rien quand il est vide.
///
/// **La colonne nullable et la colonne obligatoire ne se traitent pas
/// pareil** : `summary` accepte l'absence, `objectives` non. Rendre `null` pour
/// une chaîne vide laisse la base trancher — ce qui est exactement ce qu'on
/// veut, plutôt que d'écrire `{"fr": ""}` que le domaine refuserait avec un
/// message moins clair.
pub fn i18n(texte: &str) -> Option<serde_json::Value> {
    let coupe = texte.trim();
    (!coupe.is_empty()).then(|| serde_json::json!({ "fr": coupe }))
}

/// Le même, mais pour un champ obligatoire : la chaîne vide passe telle quelle
/// et **c'est le domaine qui refuse**, en nommant le champ fautif par le nom du
/// domaine violé. On ne redouble pas l'invariant, on le laisse parler.
pub fn i18n_obligatoire(texte: &str) -> serde_json::Value {
    serde_json::json!({ "fr": texte.trim() })
}

/// Les publics visés, un document multilingue **par entrée**. Les entrées vides
/// disparaissent : l'exigence de français porte élément par élément, et une
/// ligne blanche laissée par l'écran ferait échouer l'enregistrement entier.
pub fn i18n_liste(entrees: &[String]) -> Vec<serde_json::Value> {
    entrees.iter().filter_map(|e| i18n(e)).collect()
}

/// La résolution française d'un document multilingue, avec repli — la même
/// règle que `platform.t()`.
pub fn fr(document: &serde_json::Value) -> String {
    document
        .get("fr")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// L'heure murale, découpée pour la base : `('2027-11-12', '14:30')`.
///
/// La conversion elle-même est un `timestamp AT TIME ZONE <fuseau>` écrit dans
/// la requête ; ce qui est fait ici est **du découpage de chaîne, pas du
/// calendrier**. Une valeur mal formée rend `None`, et le service la refuse en
/// nommant le champ — plutôt que d'envoyer à PostgreSQL une chaîne dont il
/// rendrait une erreur de syntaxe en anglais.
pub fn heure_murale(valeur: &str) -> Option<(&str, &str)> {
    let (date, reste) = valeur.split_once('T')?;
    let heure = reste.get(..5)?;
    (forme(date, "0000-00-00") && forme(heure, "00:00")).then_some((date, heure))
}

/// Le gabarit d'une date ou d'une heure : `0` pour un chiffre, tout autre
/// caractère pour lui-même. Cela évite de dépendre d'une expression régulière
/// pour vérifier deux formes fixes — et surtout d'accepter `12/11/2027`, qui
/// compte pourtant dix signes.
fn forme(valeur: &str, gabarit: &str) -> bool {
    valeur.len() == gabarit.len()
        && valeur.chars().zip(gabarit.chars()).all(|(c, attendu)| {
            if attendu == '0' {
                c.is_ascii_digit()
            } else {
                c == attendu
            }
        })
}

/// Les deux écritures du formulaire, telles que le contrat les nomme.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SaveDraftPayload {
    /// Nul au tout premier enregistrement : c'est lui qui crée la ligne.
    #[serde(default)]
    pub proposal_id: Option<Uuid>,
    pub call_id: Uuid,
    /// **Ignoré.** L'édition vient de l'appel, lu en base. Le champ est accepté
    /// parce que le front l'envoie ; le lire serait croire ce que le client
    /// déclare — même motif que `actorId` et `organization_id`, écartés depuis
    /// B1.
    #[serde(default)]
    pub event_id: Option<Uuid>,
    pub draft: ProposalDraft,
}

// ---------------------------------------------------------------------------
// Les trois textes obligatoires d'un dossier, et le brouillon qui n'en a aucun
// ---------------------------------------------------------------------------

/// **`platform.i18n_text` refuse un français vide**, et `title`, `objectives`
/// et `detailed_presentation` sont `NOT NULL`. Or le premier enregistrement
/// automatique a lieu **avant l'étape 2** — le formulaire commence par les
/// organisations —, quand ces trois champs n'ont jamais été touchés.
///
/// Le repli de l'adresse d'URL (R5) réglait `slug` ; **il ne réglait pas les
/// trois textes**, qui butent sur le même vide pour la même raison. Le service
/// pose donc un texte provisoire, remplacé dès la première frappe, et **refusé
/// au dépôt** : un dossier ne part pas au comité en s'appelant « Dossier sans
/// titre ».
///
/// La recomposition du brouillon les rend à la chaîne vide, de sorte que le
/// formulaire ne les affiche jamais.
pub const TITRE_PROVISOIRE: &str = "Dossier sans titre";
pub const TEXTE_PROVISOIRE: &str = "À compléter";

/// Un texte obligatoire de brouillon : sa valeur, ou son repli.
pub fn i18n_de_brouillon(texte: &str, repli: &str) -> serde_json::Value {
    let coupe = texte.trim();
    serde_json::json!({ "fr": if coupe.is_empty() { repli } else { coupe } })
}

/// Ce texte est-il l'un des deux replis ? Sert à les refuser au dépôt et à les
/// effacer à la recomposition.
pub fn est_provisoire(texte: &str) -> bool {
    let coupe = texte.trim();
    coupe == TITRE_PROVISOIRE || coupe == TEXTE_PROVISOIRE
}

/// La résolution française d'un document, **repli effacé**.
pub fn fr_sans_repli(document: &serde_json::Value) -> String {
    let texte = fr(document);
    if est_provisoire(&texte) {
        String::new()
    } else {
        texte
    }
}
