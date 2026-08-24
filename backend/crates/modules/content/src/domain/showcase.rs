//! Ce que le public lit de la vitrine — la forme servie **sans session**.
//!
//! C'est `ShowcaseRow`, la ligne de `content.v_showcase` telle que
//! `frontend/app/types/views.ts` la déclare. **Aucun nom de champ n'est
//! renégocié** : la vue résout en base la nature, l'attribution, le pays, les
//! trois médias et les thématiques, et le contrat du site reprend ces noms.
//!
//! **Deux colonnes pour la même chose, et c'est voulu**, ici comme partout dans
//! le modèle : `theme_codes` filtre et s'indexe, `themes` porte le libellé et la
//! couleur venus de `reference.taxonomy_terms`. Une pastille thématique ne se
//! traduit jamais dans un fichier i18n — c'est le défaut n° 1 de la v1.

use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

/// Une diapositive de la vitrine, prête à afficher.
///
/// Les champs multilingues (`title`, `quote`, `nature_label`…) sont des
/// `platform.i18n_text` bruts : la résolution avec repli sur le français se fait
/// à l'affichage, jamais en base et jamais ici.
#[derive(Debug, Clone, Serialize)]
pub struct ShowcaseRow {
    pub id: Uuid,
    pub placement: String,
    pub sort_order: i16,

    pub nature_code: String,
    pub nature_label: Option<Value>,
    pub nature_color: Option<String>,
    pub nature_icon: Option<String>,

    pub title: Value,
    pub quote: Option<Value>,
    pub body: Option<Value>,

    pub author_name: Option<String>,
    pub author_title: Option<Value>,
    pub person_id: Option<Uuid>,

    pub organization_id: Option<Uuid>,
    pub organization_name: Option<String>,
    pub organization_acronym: Option<String>,

    pub country_code: Option<String>,
    pub country_name: Option<Value>,

    pub event_id: Option<Uuid>,
    pub event_slug: Option<String>,
    pub event_title: Option<Value>,

    pub session_id: Option<Uuid>,
    pub session_slug: Option<String>,
    pub session_title: Option<Value>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub session_starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub session_ends_at: Option<OffsetDateTime>,
    pub session_timezone: Option<String>,

    pub link_url: Option<String>,
    pub link_label: Option<Value>,

    /// Fond photographique — rôle `banner`, résolu par `media.attached_image()`.
    pub background_image: Option<Value>,
    /// Fond vidéo — rôle `video`, **sorti du même mécanisme** bien qu'il ne
    /// s'agisse pas d'une image : la fonction ne fait aucune hypothèse sur le
    /// type MIME, et le site décide de rendre `<video>` ou `<img>` d'après ce
    /// qu'il reçoit. `null` est courant — un objet encore en traitement n'est
    /// pas servi, et le bandeau se rabat alors sur l'image puis sur l'aplat.
    pub background_video: Option<Value>,
    /// Vignette du rail — rôle `cover`. Sert aussi d'affiche à la vidéo.
    pub thumbnail: Option<Value>,
    pub background_color_hex: Option<String>,

    pub theme_codes: Vec<String>,
    pub themes: Value,

    #[serde(with = "time::serde::rfc3339::option")]
    pub starts_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub ends_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
}

/// Ce que `GET /home` rend — `Pick<HomeScreen, 'hero'>` côté site.
///
/// **Un objet et non un tableau nu.** Le reste de l'accueil — éditions, séances,
/// chiffres — est servi par les modules qui en répondent, et l'écran les
/// compose ; cette enveloppe laisse la place à ce que la vitrine servira plus
/// tard sans changer la forme d'une réponse déjà consommée.
#[derive(Debug, Clone, Serialize)]
pub struct HomeShowcase {
    pub hero: Vec<ShowcaseRow>,
}
