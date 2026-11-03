//! **Les courriels programmés** (04/11) : rappels d'activité et de journée
//! déjà en file, leurs textes et leur composition, purs.
//!
//! Un « type » est un rappel de journée (tous décalages confondus) ou un rappel
//! d'activité à un décalage ; un « envoi » réunit les destinataires d'un même
//! rappel pour une même cible à la même heure. Le texte retenu à l'envoi est
//! celui de l'envoi, sinon celui du type, sinon le texte par défaut écrit ici.
//!
//! Le corps est du texte brut : une ligne vide sépare deux paragraphes. Les
//! valeurs des variables sont échappées dans le HTML, jamais le texte saisi
//! n'y passe tel quel.

use kernel::mail::OutgoingMail;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::{OffsetDateTime, PrimitiveDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use super::courriel_direct::{assembler, Assemblage, Paragraphe};
use super::dossier::{date_longue, heure_de};
use super::gabarits::Visuels;
use super::mailing::echapper_html;
use super::offsets;
use super::render::{self, ErreurRendu};

pub const ACTIVITE: &str = "activity";
pub const JOURNEE: &str = "day";
pub const LANGUES: [&str; 2] = ["fr", "en"];

const SUJET_MAX: usize = 250;
const CORPS_MAX: usize = 10_000;

/// Le fond des valeurs dans l'aperçu, celui de l'aperçu des envois.
const SURLIGNAGE: &str = "background-color:#e0f3fb;border-radius:2px;padding:0 2px";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nature {
    Activite,
    Journee,
}

impl Nature {
    pub fn code(self) -> &'static str {
        match self {
            Self::Activite => ACTIVITE,
            Self::Journee => JOURNEE,
        }
    }

    pub fn depuis(code: &str) -> Option<Self> {
        match code {
            ACTIVITE => Some(Self::Activite),
            JOURNEE => Some(Self::Journee),
            _ => None,
        }
    }

    pub fn variables(self) -> &'static [&'static str] {
        match self {
            Self::Activite => &["prenom", "titre_activite", "date", "delai", "edition"],
            Self::Journee => &["prenom", "titre_journee", "date", "delai", "edition"],
        }
    }

    fn variable_titre(self) -> &'static str {
        match self {
            Self::Activite => "titre_activite",
            Self::Journee => "titre_journee",
        }
    }
}

/// `fr` ou `en` : la langue d'un destinataire, repli sur le français.
pub fn langue(locale: &str) -> &'static str {
    if locale.starts_with("en") {
        "en"
    } else {
        "fr"
    }
}

// -----------------------------------------------------------------------------
// Les clés
// -----------------------------------------------------------------------------

/// Un type : `day`, ou `activity.<décalage en minutes>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleType {
    pub nature: Nature,
    /// Nul pour le rappel de journée, qui couvre tous ses décalages.
    pub offset_minutes: Option<i32>,
}

impl CleType {
    pub fn texte(&self) -> String {
        match (self.nature, self.offset_minutes) {
            (Nature::Activite, Some(m)) => format!("{ACTIVITE}.{m}"),
            _ => JOURNEE.to_owned(),
        }
    }

    pub fn lire(cle: &str) -> Option<Self> {
        if cle == JOURNEE {
            return Some(Self {
                nature: Nature::Journee,
                offset_minutes: None,
            });
        }
        let minutes: i32 = cle.strip_prefix("activity.")?.parse().ok()?;
        (minutes > 0).then_some(Self {
            nature: Nature::Activite,
            offset_minutes: Some(minutes),
        })
    }
}

/// Un envoi : `activity.<séance>.<décalage>` ou
/// `day.<journée>.<décalage>.<début visé, secondes Unix>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleEnvoi {
    pub nature: Nature,
    pub cible: Uuid,
    pub offset_minutes: i32,
    pub debut: Option<i64>,
}

impl CleEnvoi {
    pub fn texte(&self) -> String {
        match self.debut {
            Some(d) if self.nature == Nature::Journee => {
                format!("{JOURNEE}.{}.{}.{d}", self.cible, self.offset_minutes)
            }
            _ => format!("{ACTIVITE}.{}.{}", self.cible, self.offset_minutes),
        }
    }

    pub fn lire(cle: &str) -> Option<Self> {
        let morceaux: Vec<&str> = cle.split('.').collect();
        let offset_minutes: i32 = morceaux.get(2)?.parse().ok().filter(|m| *m > 0)?;
        let cible: Uuid = morceaux.get(1)?.parse().ok()?;
        match (morceaux[0], morceaux.len()) {
            (ACTIVITE, 3) => Some(Self {
                nature: Nature::Activite,
                cible,
                offset_minutes,
                debut: None,
            }),
            (JOURNEE, 4) => Some(Self {
                nature: Nature::Journee,
                cible,
                offset_minutes,
                debut: Some(morceaux[3].parse().ok()?),
            }),
            _ => None,
        }
    }

    pub fn type_(&self) -> CleType {
        CleType {
            nature: self.nature,
            offset_minutes: (self.nature == Nature::Activite).then_some(self.offset_minutes),
        }
    }

    pub fn debut_vise(&self) -> Option<OffsetDateTime> {
        self.debut
            .and_then(|d| OffsetDateTime::from_unix_timestamp(d).ok())
    }
}

// -----------------------------------------------------------------------------
// Ce que l'écran lit et écrit
// -----------------------------------------------------------------------------

/// Un objet et un message (`ScheduledMailText`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ScheduledMailText {
    pub subject: String,
    pub body: String,
}

/// Les deux langues (`ScheduledMailTexts`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ScheduledMailTexts {
    pub fr: ScheduledMailText,
    pub en: ScheduledMailText,
}

/// Un envoi de la liste (`ScheduledMailBatchRow`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScheduledMailBatchRow {
    pub key: String,
    pub type_key: String,
    pub kind: String,
    pub offset_minutes: i32,
    pub target_id: Uuid,
    #[serde(with = "time::serde::rfc3339::option")]
    pub target_starts_at: Option<OffsetDateTime>,
    /// L'activité ou la journée, dans la langue de l'écran.
    pub title: String,
    #[serde(with = "time::serde::rfc3339")]
    pub run_at: OffsetDateTime,
    pub recipients: i64,
    pub has_own_text: bool,
}

/// Un type et ses envois (`ScheduledMailType`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScheduledMailType {
    pub key: String,
    pub kind: String,
    pub offset_minutes: Option<i32>,
    pub count: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub next_run_at: OffsetDateTime,
    /// Le type a un texte écrit pour l'édition (sinon, le texte par défaut).
    pub has_custom_text: bool,
    pub batches: Vec<ScheduledMailBatchRow>,
}

/// L'onglet « Programmés » (`ScheduledMailScreen`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScheduledMailScreen {
    /// L'heure du serveur au relevé : le décompte s'y cale.
    #[serde(with = "time::serde::rfc3339")]
    pub observed_at: OffsetDateTime,
    pub total: i64,
    pub next: Option<ScheduledMailBatchRow>,
    pub types: Vec<ScheduledMailType>,
}

/// Le panneau d'un envoi (`ScheduledMailBatch`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScheduledMailBatch {
    #[serde(with = "time::serde::rfc3339")]
    pub observed_at: OffsetDateTime,
    pub batch: ScheduledMailBatchRow,
    /// Les courriels de tout le type, dans l'édition.
    pub type_recipients: i64,
    /// Le texte du type, écrit ou par défaut, dans les deux langues.
    pub type_text: ScheduledMailTexts,
    pub type_text_is_custom: bool,
    /// Le texte propre à cet envoi ; nul : il suit le texte du type.
    pub own_text: Option<ScheduledMailTexts>,
    pub variables: Vec<String>,
    /// La langue du premier destinataire, celle de l'aperçu par défaut.
    pub preview_locale: String,
}

/// Le texte d'un type (`ScheduledMailTypeTextPayload`).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScheduledMailTypeTextPayload {
    pub event_id: Uuid,
    pub fr: ScheduledMailText,
    pub en: ScheduledMailText,
}

/// L'aperçu d'un texte en cours d'écriture (`ScheduledMailPreviewPayload`).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScheduledMailPreviewPayload {
    pub locale: String,
    pub subject: String,
    pub body: String,
}

/// Ce qui partira (`ScheduledMailPreview`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScheduledMailPreview {
    pub locale: String,
    pub subject: String,
    pub html: String,
}

/// Les types, dans l'ordre de l'écran : la journée, puis les activités du plus
/// lointain au plus proche ; les envois par heure de départ.
pub fn regrouper(lignes: Vec<(ScheduledMailBatchRow, bool)>) -> Vec<ScheduledMailType> {
    let mut types: Vec<ScheduledMailType> = Vec::new();
    for (envoi, type_ecrit) in lignes {
        let Some(cle) = CleEnvoi::lire(&envoi.key) else {
            continue;
        };
        let cle_type = cle.type_();
        let cle_texte = cle_type.texte();
        let type_ = match types.iter_mut().position(|t| t.key == cle_texte) {
            Some(i) => &mut types[i],
            None => {
                types.push(ScheduledMailType {
                    key: cle_texte,
                    kind: cle_type.nature.code().to_owned(),
                    offset_minutes: cle_type.offset_minutes,
                    count: 0,
                    next_run_at: envoi.run_at,
                    has_custom_text: type_ecrit,
                    batches: Vec::new(),
                });
                types.last_mut().expect("type ajouté")
            }
        };
        type_.count += envoi.recipients;
        type_.next_run_at = type_.next_run_at.min(envoi.run_at);
        type_.batches.push(envoi);
    }
    for t in &mut types {
        t.batches
            .sort_by(|a, b| a.run_at.cmp(&b.run_at).then_with(|| a.title.cmp(&b.title)));
    }
    types.sort_by_key(|t| (t.kind != JOURNEE, std::cmp::Reverse(t.offset_minutes)));
    types
}

// -----------------------------------------------------------------------------
// Les textes
// -----------------------------------------------------------------------------

/// D'où vient le texte qui part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Envoi,
    Type,
}

/// Le texte de l'envoi d'abord, puis celui du type. Nul : le texte par défaut
/// (ou, pour une activité, un modèle publié) prend le relais.
pub fn retenir(
    envoi: Option<ScheduledMailText>,
    type_: Option<ScheduledMailText>,
) -> Option<(ScheduledMailText, Source)> {
    envoi
        .map(|t| (t, Source::Envoi))
        .or_else(|| type_.map(|t| (t, Source::Type)))
}

/// Le texte par défaut, quand personne n'en a écrit.
pub fn texte_par_defaut(nature: Nature, anglais: bool) -> ScheduledMailText {
    let (sujet, corps) = match (nature, anglais) {
        (Nature::Journee, false) => (
            "{{titre_journee}} commence {{delai}}",
            "Bonjour {{prenom}},\n\n« {{titre_journee}} » commence {{delai}}, le {{date}}.\n\nSuivez le direct avec le lien de votre courriel de confirmation, ou depuis la page de la journée.",
        ),
        (Nature::Journee, true) => (
            "{{titre_journee}} starts {{delai}}",
            "Hello {{prenom}},\n\n\"{{titre_journee}}\" starts {{delai}}, on {{date}}.\n\nFollow the live stream with the link in your confirmation email, or from the page of the day.",
        ),
        (Nature::Activite, false) => (
            "{{titre_activite}} commence {{delai}}",
            "Bonjour {{prenom}},\n\n« {{titre_activite}} » commence {{delai}}, le {{date}}.\n\nRetrouvez l'activité et son direct sur sa page.",
        ),
        (Nature::Activite, true) => (
            "{{titre_activite}} starts {{delai}}",
            "Hello {{prenom}},\n\n\"{{titre_activite}}\" starts {{delai}}, on {{date}}.\n\nFind the activity and its live stream on its page.",
        ),
    };
    ScheduledMailText {
        subject: sujet.to_owned(),
        body: corps.to_owned(),
    }
}

/// Un texte recevable : objet et message non vides, dans leurs bornes, et
/// aucune variable que la nature ne fournit pas. Rend le message et le champ.
pub fn valider(nature: Nature, t: &ScheduledMailText) -> Result<(), (String, &'static str)> {
    let sujet = t.subject.trim();
    if sujet.is_empty() {
        return Err(("Écrivez l'objet du courriel.".to_owned(), "subject"));
    }
    if sujet.chars().count() > SUJET_MAX {
        return Err((
            format!("L'objet dépasse {SUJET_MAX} caractères."),
            "subject",
        ));
    }
    let corps = t.body.trim();
    if corps.is_empty() {
        return Err(("Écrivez le message.".to_owned(), "body"));
    }
    if corps.chars().count() > CORPS_MAX {
        return Err((
            format!("Le message dépasse {CORPS_MAX} caractères."),
            "body",
        ));
    }
    for (champ, texte) in [("subject", sujet), ("body", corps)] {
        if let Some(inconnue) = render::variables_citees(texte)
            .into_iter()
            .find(|v| !nature.variables().contains(&v.as_str()))
        {
            return Err((
                format!(
                    "La variable {{{{{inconnue}}}}} n'existe pas pour ce rappel. Variables possibles : {}.",
                    nature
                        .variables()
                        .iter()
                        .map(|v| format!("{{{{{v}}}}}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                champ,
            ));
        }
    }
    Ok(())
}

/// Le texte tel qu'il s'enregistre : espaces de bord retirés, fins de ligne
/// unifiées.
pub fn normaliser(t: &ScheduledMailText) -> ScheduledMailText {
    ScheduledMailText {
        subject: t.subject.trim().to_owned(),
        body: t.body.replace("\r\n", "\n").trim().to_owned(),
    }
}

// -----------------------------------------------------------------------------
// La composition
// -----------------------------------------------------------------------------

/// Ce qu'il faut savoir d'un rappel pour remplir ses variables.
pub struct Contexte<'a> {
    pub nature: Nature,
    pub anglais: bool,
    pub prenom: &'a str,
    pub titre: &'a str,
    pub edition: &'a str,
    /// Le début, dans le fuseau de la cible.
    pub debut_local: PrimitiveDateTime,
    /// La ville qui nomme le fuseau (« Antalya »).
    pub zone: &'a str,
    pub offset_minutes: i64,
    pub lien: &'a str,
}

/// La page publique de la cible, dans la langue du destinataire.
pub fn lien(site: &str, nature: Nature, edition: &str, slug: &str, anglais: bool) -> String {
    let base = site.trim_end_matches('/');
    match (nature, anglais) {
        (Nature::Activite, false) => format!("{base}/programmations/{edition}/{slug}"),
        (Nature::Activite, true) => format!("{base}/en/programmes/{edition}/{slug}"),
        (Nature::Journee, false) => format!("{base}/programmations/{edition}/journees/{slug}"),
        (Nature::Journee, true) => format!("{base}/en/programmes/{edition}/days/{slug}"),
    }
}

/// « demain », « dans 2 jours », « dans 1 heure ».
pub fn delai(minutes: i64, anglais: bool) -> String {
    if minutes == 1440 {
        return if anglais { "tomorrow" } else { "demain" }.to_owned();
    }
    offsets::libelle_delai(i32::try_from(minutes).unwrap_or(i32::MAX), anglais)
}

const JOURS_EN: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const MOIS_EN: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// « dimanche 8 novembre 2026 à 09:00 (heure d'Antalya) », « Sunday
/// 8 November 2026 at 09:00 (Antalya time) ».
pub fn date(debut: PrimitiveDateTime, zone: &str, anglais: bool) -> String {
    let heure = format!("{:02}:{:02}", debut.hour(), debut.minute());
    if anglais {
        let d = debut.date();
        format!(
            "{} {} {} {} at {heure} ({zone} time)",
            JOURS_EN[usize::from(d.weekday().number_days_from_monday())],
            d.day(),
            MOIS_EN[usize::from(u8::from(d.month())) - 1],
            d.year()
        )
    } else {
        format!(
            "{} à {heure} ({})",
            date_longue(debut.date()),
            heure_de(zone)
        )
    }
}

pub fn valeurs(c: &Contexte<'_>) -> HashMap<String, String> {
    HashMap::from([
        ("prenom".to_owned(), c.prenom.to_owned()),
        (c.nature.variable_titre().to_owned(), c.titre.to_owned()),
        ("date".to_owned(), date(c.debut_local, c.zone, c.anglais)),
        ("delai".to_owned(), delai(c.offset_minutes, c.anglais)),
        ("edition".to_owned(), c.edition.to_owned()),
    ])
}

/// Le courriel, dans l'habillage des confirmations d'inscription. `surligner`
/// pose les valeurs sur un fond : l'aperçu seulement, jamais un envoi.
pub fn composer(
    texte: &ScheduledMailText,
    c: &Contexte<'_>,
    visuels: Option<&Visuels>,
    to: &str,
    locale: &str,
    message_id: &str,
    surligner: bool,
) -> Result<OutgoingMail, ErreurRendu> {
    let brutes = valeurs(c);
    let echappees: HashMap<String, String> = brutes
        .iter()
        .map(|(k, v)| {
            let sure = echapper_html(v);
            let v = if surligner {
                format!("<span style=\"{SURLIGNAGE}\">{sure}</span>")
            } else {
                sure
            };
            (k.clone(), v)
        })
        .collect();

    let sujet = render::rendre(texte.subject.trim(), &brutes)?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut paragraphes = Vec::new();
    for bloc in texte.body.replace("\r\n", "\n").split("\n\n") {
        let bloc = bloc.trim();
        if bloc.is_empty() {
            continue;
        }
        paragraphes.push(Paragraphe {
            html: render::rendre(&echapper_html(bloc), &echappees)?.replace('\n', "<br>"),
            texte: render::rendre(bloc, &brutes)?,
        });
    }
    let action = match (c.nature, c.anglais) {
        (Nature::Activite, false) => "Voir l'activité",
        (Nature::Activite, true) => "View the activity",
        (Nature::Journee, false) => "Voir la journée",
        (Nature::Journee, true) => "View the day",
    };
    Ok(assembler(
        &Assemblage {
            anglais: c.anglais,
            sujet: &sujet,
            avant: &paragraphes,
            lien: c.lien,
            action,
            apres: &[],
        },
        visuels,
        to,
        locale,
        message_id,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn contexte<'a>(nature: Nature, anglais: bool, titre: &'a str, lien: &'a str) -> Contexte<'a> {
        Contexte {
            nature,
            anglais,
            prenom: "Awa",
            titre,
            edition: "CdP31",
            debut_local: datetime!(2026-11-08 09:00),
            zone: "Antalya",
            offset_minutes: 2880,
            lien,
        }
    }

    fn texte(sujet: &str, corps: &str) -> ScheduledMailText {
        ScheduledMailText {
            subject: sujet.to_owned(),
            body: corps.to_owned(),
        }
    }

    #[test]
    fn le_texte_de_lenvoi_passe_avant_celui_du_type() {
        let envoi = texte("envoi", "e");
        let type_ = texte("type", "t");
        assert_eq!(
            retenir(Some(envoi.clone()), Some(type_.clone())),
            Some((envoi, Source::Envoi))
        );
        assert_eq!(
            retenir(None, Some(type_.clone())),
            Some((type_, Source::Type))
        );
        assert_eq!(retenir(None, None), None);
    }

    #[test]
    fn le_texte_par_defaut_ne_cite_que_des_variables_fournies() {
        for nature in [Nature::Activite, Nature::Journee] {
            for anglais in [false, true] {
                let t = texte_par_defaut(nature, anglais);
                assert_eq!(valider(nature, &t), Ok(()));
                assert!(!t.body.contains('\u{2014}') && !t.subject.contains('\u{2014}'));
            }
        }
    }

    #[test]
    fn une_variable_dun_autre_rappel_est_refusee_en_la_nommant() {
        let t = texte("{{titre_journee}}", "Bonjour {{prenom}}");
        let (message, champ) = valider(Nature::Activite, &t).unwrap_err();
        assert_eq!(champ, "subject");
        assert!(message.contains("{{titre_journee}}"));
        assert!(message.contains("{{titre_activite}}"));
        assert!(valider(Nature::Activite, &texte(" ", "x")).is_err());
    }

    #[test]
    fn les_valeurs_sont_echappees_et_le_texte_saisi_aussi() {
        let lien = "https://site.org/programmations/cdp31/atelier";
        let c = contexte(Nature::Activite, false, "Climat <b>& finance</b>", lien);
        let t = texte(
            "{{titre_activite}} commence {{delai}}",
            "Bonjour {{prenom}},\n\n<script>x</script> {{titre_activite}} le {{date}}.\nÀ bientôt",
        );
        let m = composer(&t, &c, None, "awa@example.org", "fr", "j", false).unwrap();
        let html = m.html.unwrap();
        assert!(html.contains("Climat &lt;b&gt;&amp; finance&lt;/b&gt;"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("<br>À bientôt"));
        assert!(html.contains(">Voir l&#39;activité</a>"));
        assert_eq!(m.subject, "Climat <b>& finance</b> commence dans 2 jours");
        assert!(m
            .text
            .contains("dimanche 8 novembre 2026 à 09:00 (heure d'Antalya)"));
        assert!(m.text.contains(lien));
        assert!(!html.contains("background-color:#e0f3fb"));
    }

    #[test]
    fn lapercu_surligne_les_valeurs() {
        let c = contexte(Nature::Journee, false, "Journée finance", "https://s.org/x");
        let t = texte_par_defaut(Nature::Journee, false);
        let m = composer(&t, &c, None, "a@b.org", "fr", "apercu", true).unwrap();
        assert!(m.html.unwrap().contains(
            "<span style=\"background-color:#e0f3fb;border-radius:2px;padding:0 2px\">Awa</span>"
        ));
        assert_eq!(m.subject, "Journée finance commence dans 2 jours");
    }

    #[test]
    fn langlais_a_ses_dates_ses_delais_et_son_bouton() {
        let c = Contexte {
            offset_minutes: 1440,
            ..contexte(
                Nature::Journee,
                true,
                "Finance day",
                "https://s.org/en/programmes/cdp31/days/finance",
            )
        };
        let m = composer(
            &texte_par_defaut(Nature::Journee, true),
            &c,
            None,
            "a@b.org",
            "en",
            "j",
            false,
        )
        .unwrap();
        assert_eq!(m.subject, "Finance day starts tomorrow");
        assert!(m
            .text
            .contains("Sunday 8 November 2026 at 09:00 (Antalya time)"));
        assert!(m.html.unwrap().contains(">View the day</a>"));
    }

    #[test]
    fn les_liens_menent_aux_pages_du_site_dans_la_langue() {
        let site = "https://site.org/";
        assert_eq!(
            lien(site, Nature::Activite, "cdp31", "atelier", false),
            "https://site.org/programmations/cdp31/atelier"
        );
        assert_eq!(
            lien(site, Nature::Activite, "cdp31", "atelier", true),
            "https://site.org/en/programmes/cdp31/atelier"
        );
        assert_eq!(
            lien(site, Nature::Journee, "cdp31", "finance", false),
            "https://site.org/programmations/cdp31/journees/finance"
        );
        assert_eq!(
            lien(site, Nature::Journee, "cdp31", "finance", true),
            "https://site.org/en/programmes/cdp31/days/finance"
        );
    }

    #[test]
    fn la_langue_du_destinataire_se_replie_sur_le_francais() {
        assert_eq!(langue("en-GB"), "en");
        assert_eq!(langue("es"), "fr");
    }

    #[test]
    fn les_cles_se_relisent() {
        let id = Uuid::nil();
        let a = CleEnvoi {
            nature: Nature::Activite,
            cible: id,
            offset_minutes: 60,
            debut: None,
        };
        assert_eq!(CleEnvoi::lire(&a.texte()), Some(a));
        assert_eq!(a.type_().texte(), "activity.60");
        let j = CleEnvoi {
            nature: Nature::Journee,
            cible: id,
            offset_minutes: 2880,
            debut: Some(1_794_000_000),
        };
        assert_eq!(CleEnvoi::lire(&j.texte()), Some(j));
        assert_eq!(j.type_().texte(), "day");
        assert_eq!(CleType::lire("day").map(|t| t.offset_minutes), Some(None));
        assert_eq!(
            CleType::lire("activity.30").and_then(|t| t.offset_minutes),
            Some(30)
        );
        assert!(CleEnvoi::lire("day.x.1").is_none());
        assert!(CleType::lire("activity.-3").is_none());
    }

    #[test]
    fn les_envois_se_regroupent_par_type() {
        let ligne = |kind: &str, offset: i32, debut: Option<i64>, heure: i64, n: i64| {
            let cle = CleEnvoi {
                nature: Nature::depuis(kind).unwrap(),
                cible: Uuid::from_u128(heure as u128),
                offset_minutes: offset,
                debut,
            };
            (
                ScheduledMailBatchRow {
                    key: cle.texte(),
                    type_key: cle.type_().texte(),
                    kind: kind.to_owned(),
                    offset_minutes: offset,
                    target_id: cle.cible,
                    target_starts_at: None,
                    title: format!("t{heure}"),
                    run_at: OffsetDateTime::from_unix_timestamp(heure).unwrap(),
                    recipients: n,
                    has_own_text: false,
                },
                false,
            )
        };
        let types = regrouper(vec![
            ligne(ACTIVITE, 60, None, 50, 3),
            ligne(JOURNEE, 1440, Some(9), 30, 4),
            ligne(ACTIVITE, 2880, None, 20, 2),
            ligne(JOURNEE, 2880, Some(9), 10, 4),
            ligne(ACTIVITE, 60, None, 40, 1),
        ]);
        let cles: Vec<&str> = types.iter().map(|t| t.key.as_str()).collect();
        assert_eq!(cles, ["day", "activity.2880", "activity.60"]);
        assert_eq!(types[0].count, 8);
        assert_eq!(types[0].next_run_at.unix_timestamp(), 10);
        assert_eq!(types[2].batches[0].run_at.unix_timestamp(), 40);
    }
}
