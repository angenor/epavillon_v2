//! Les formes que le front consomme déjà pour une édition.
//!
//! **Aucun nom de champ n'est renégocié** (FR-003) : leur source unique est
//! `frontend/app/types/admin-events.ts`. Ce qui est ajouté l'est de façon
//! **additive**, et c'est dit là où ça se voit.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// Une ligne de la liste des éditions — `EditionListRow`.
///
/// Elle porte sa **série résolue** et son pays résolu, pas seulement leurs
/// identifiants : une ligne qui n'afficherait que l'année obligerait à savoir
/// par cœur quelle COP tombe quand. Elle ne porte **ni** la description **ni**
/// le message d'accueil (FR-021) — deux paragraphes par édition, dont la liste
/// se passe.
#[derive(Debug, Clone, Serialize)]
pub struct EditionListRow {
    pub id: Uuid,
    pub title: Value,
    pub acronym: Option<String>,
    pub slug: String,
    pub series_id: Option<Uuid>,
    pub series_name: Option<Value>,
    pub series_kind: Option<String>,
    pub edition_label: Option<String>,
    pub edition_year: i16,
    pub status: String,
    pub participation_mode: String,
    pub timezone: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub country_id: Option<Uuid>,
    pub country_name: Option<Value>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub has_pavilion: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub programme_published_at: Option<OffsetDateTime>,

    // -- Décomptes joints, jamais des colonnes de `event.events` -------------
    pub proposal_count: i64,
    pub session_count: i64,
    pub scheduled_session_count: i64,
    pub call_status: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub call_deadline: Option<OffsetDateTime>,
    pub day_count: i64,
}

/// Ce que le formulaire envoie — `EditionFormPayload`.
///
/// **La charge utile est totale** (research.md § R13) : elle porte tous les
/// champs modifiables, et la mise à jour est donc un `UPDATE` complet, écrit
/// une fois et vérifié à la compilation. Une charge partielle demanderait soit
/// un `UPDATE` par combinaison de champs, soit du SQL composé — la seule
/// entorse au principe VI que B2 ait dû s'accorder, et qui n'a pas lieu d'être
/// ici.
///
/// **`programme_published_at` n'y est pas**, et c'est délibéré : elle est posée
/// par la publication seule, et une écriture d'édition ne doit jamais la
/// toucher.
#[derive(Debug, Clone, Deserialize)]
pub struct EditionFormPayload {
    /// Nul à la création. **Ignoré à la modification** : l'identifiant fait foi
    /// dans l'adresse, jamais dans le corps.
    #[serde(default)]
    pub id: Option<Uuid>,
    pub series_id: Option<Uuid>,
    pub edition_label: Option<String>,
    pub edition_year: i16,
    pub title: Value,
    pub acronym: Option<String>,
    pub slug: String,
    pub description: Value,
    pub status: String,
    pub participation_mode: String,
    pub timezone: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub has_pavilion: bool,
    #[serde(default)]
    pub highlights: Option<Value>,
    /// **Acceptées et non posées** (research.md § R17). Le rattachement média
    /// est polymorphe et appartient au module Média : ce jalon **lit** les trois
    /// déclinaisons résolues et accepte sans les écrire les identifiants
    /// d'objet que le formulaire envoie. Obligation inscrite pour B6.
    #[serde(default)]
    pub images: Option<Value>,
}

impl EditionFormPayload {
    /// Le libellé français, dont la valeur de sigle proposée se dérive. Repli
    /// sur l'anglais, puis sur rien : le domaine `platform.i18n_text` exige le
    /// français, mais ce chemin s'exécute **avant** que la base ne le vérifie.
    pub fn titre_pour_proposition(&self) -> &str {
        self.title
            .get("fr")
            .and_then(Value::as_str)
            .or_else(|| self.title.get("en").and_then(Value::as_str))
            .unwrap_or_default()
    }
}

/// Un refus de sauvegarde, tel que la base le formule — `EditionErrorCode`.
///
/// Ce ne sont pas des erreurs de réseau mais des **réponses** : chaque code
/// correspond à une contrainte nommée de `060_events.sql`, et l'écran les rend
/// au champ concerné. Le service ne réimplémente pas l'invariant, il **traduit**
/// le refus de la base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EditionErrorCode {
    /// `ck_events_period` — la fin doit suivre le début.
    Period,
    /// `ck_events_physical_location` — hors ligne, pays et ville sont exigés.
    PhysicalLocation,
    /// `ux_events_slug` — l'adresse est unique sur toute la plateforme.
    SlugTaken,
    /// `ux_events_series_edition` — cette série a déjà ce millésime et ce libellé.
    EditionTaken,
    /// `events_edition_year_check`.
    YearRange,
    /// `ck_events_coordinates` — les deux, ou aucun.
    Coordinates,
    /// Champ obligatoire non renseigné — **et la règle du sigle**, qui emprunte
    /// cette forme faute de variante dédiée dans le contrat du front. Le champ
    /// nommé suffit à désigner le fautif, et le sigle étant facultatif en base,
    /// il n'y a aucune ambiguïté avec un `NOT NULL`.
    Required,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditionFormError {
    pub code: EditionErrorCode,
    /// Champ du formulaire à marquer. `null` quand le refus porte sur l'ensemble.
    pub field: Option<String>,
}

impl EditionFormError {
    pub fn new(code: EditionErrorCode, field: &str) -> Self {
        Self {
            code,
            field: Some(field.to_owned()),
        }
    }
}

/// La réponse d'une sauvegarde — `EditionSaveResult`.
///
/// **Elle dit ce qui est arrivé au calendrier.** Une sauvegarde silencieuse
/// laisserait découvrir la conséquence au planificateur, deux écrans plus loin.
/// Ici, `days_removed` et `sessions_detached` valent **toujours zéro** : un
/// enregistrement d'édition ne supprime aucune journée (FR-033). Le retrait est
/// un geste séparé, et explicite.
#[derive(Debug, Clone, Serialize)]
pub struct EditionSaveResult {
    pub ok: bool,
    pub edition: Option<EditionListRow>,
    pub errors: Vec<EditionFormError>,
    pub days_created: i64,
    pub days_removed: i64,
    pub sessions_detached: i64,
    /// **Champ additif** (research.md § R1). La forme actuelle du contrat
    /// exprime le refus — `{ code: 'required', field: 'acronym' }` — mais n'a
    /// pas de place pour une suggestion. Un refus qui ne propose rien fait
    /// chercher une convention que personne n'a écrite. Ignoré par le front
    /// jusqu'à B7 ; inscrit aux obligations.
    pub suggested_acronym: Option<String>,
}

impl EditionSaveResult {
    pub fn refuse(errors: Vec<EditionFormError>) -> Self {
        Self {
            ok: false,
            edition: None,
            errors,
            days_created: 0,
            days_removed: 0,
            sessions_detached: 0,
            suggested_acronym: None,
        }
    }

    pub fn avec_sigle_propose(mut self, propose: Option<String>) -> Self {
        self.suggested_acronym = propose;
        self
    }
}

/// Une série proposée au filtre et au formulaire — `EditionSeriesOption`.
#[derive(Debug, Clone, Serialize)]
pub struct EditionSeriesOption {
    pub id: Uuid,
    pub name: Value,
    pub kind: String,
    pub is_active: bool,
    /// Éditions déjà rattachées : ce qui distingue une série vive d'une coquille.
    pub edition_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountryOption {
    pub id: Uuid,
    pub name: Value,
    pub iso2: String,
}

/// Un fuseau proposé au formulaire — `TimeZoneOption`.
///
/// **C'est une commodité de saisie, pas un vocabulaire fermé.**
/// `platform.timezone_name` valide un identifiant IANA contre la base de fuseaux
/// de PostgreSQL, sans en tenir la liste : la liste offerte ici vient donc de
/// `pg_timezone_names`, **la même** que celle contre laquelle le domaine vérifie.
/// La recopier ailleurs les ferait diverger.
#[derive(Debug, Clone, Serialize)]
pub struct TimeZoneOption {
    /// Identifiant IANA : `America/Belem`.
    pub value: String,
    /// Dernier segment de l'identifiant, tirets bas rendus aux espaces.
    /// **Sans accent** : IANA écrit « Belem », et inventer « Belém » ferait de
    /// cette commodité une seconde vérité sur les noms de villes.
    pub city: String,
    /// Décalage courant, pour lever l'ambiguïté entre deux fuseaux voisins.
    pub offset_label: String,
}

/// Ce qu'il faut savoir avant d'ouvrir le formulaire — `EditionFormOptions`.
/// Servi **à part** de la liste : le référentiel des pays ne repart pas à
/// chaque affichage du tableau (FR-025).
#[derive(Debug, Clone, Serialize)]
pub struct EditionFormOptions {
    pub series: Vec<EditionSeriesOption>,
    pub countries: Vec<CountryOption>,
    pub timezones: Vec<TimeZoneOption>,
    /// Statuts atteignables, **lus dans l'énuméré du modèle** et donc dans
    /// l'ordre où il les déclare : le recopier ici ferait un second vocabulaire.
    pub statuses: Vec<String>,
}

/// Tout l'écran de la liste en une réponse — `EditionListScreen` (FR-018).
///
/// **Les facettes se comptent sur le même jeu de lignes que la liste.** Les
/// demander à part ferait diverger le « 2027 (4) » du filtre de ce qui
/// s'affiche : c'est la leçon de B2, où la même règle vaut pour les pays et les
/// types d'organisation.
#[derive(Debug, Clone, Serialize)]
pub struct EditionListScreen {
    pub rows: Vec<EditionListRow>,
    /// Séries **présentes dans les lignes**, avec le nombre d'éditions qu'elles
    /// y portent — et non le catalogue entier, qui proposerait au filtre des
    /// séries dont aucune édition n'est visible.
    pub series: Vec<EditionSeriesOption>,
    /// Millésimes présents dans les lignes, décroissants.
    pub years: Vec<i16>,
    /// **Vrai quand la personne administre la plateforme entière** (FR-014) :
    /// sans cela, l'écran ne peut pas distinguer un filtrage d'une absence.
    pub is_global_scope: bool,
}

/// La table `event.events` telle quelle — `EventEdition`.
///
/// C'est la forme que sert le **sélecteur d'édition** du back-office : une
/// liste courte, sans décompte ni série résolue, dont le front ne tire qu'un
/// libellé et un identifiant. La ligne de liste (`EditionListRow`) est une autre
/// forme, pour un autre écran — les confondre ferait charger deux paragraphes
/// par édition dans un menu déroulant.
#[derive(Debug, Clone, Serialize)]
pub struct EventEdition {
    pub id: Uuid,
    pub series_id: Option<Uuid>,
    pub edition_label: Option<String>,
    pub edition_year: i16,
    pub title: Value,
    pub acronym: Option<String>,
    pub slug: String,
    pub description: Value,
    pub status: String,
    pub participation_mode: String,
    pub timezone: String,
    #[serde(with = "time::serde::rfc3339")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub ends_at: OffsetDateTime,
    pub country_id: Option<Uuid>,
    pub city: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub has_pavilion: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub programme_published_at: Option<OffsetDateTime>,
    pub highlights: Option<Value>,
    pub created_by: Option<Uuid>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}
