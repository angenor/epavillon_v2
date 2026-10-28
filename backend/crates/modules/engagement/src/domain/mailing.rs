//! Ce que les écrans lisent et écrivent d'un envoi du back-office.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

/// Les variables offertes, dans l'ordre où l'écran les propose.
pub const VARIABLES: [&str; 5] = ["prenom", "nom", "organisation", "email", "edition"];

/// Les quatre familles de destinataires, telles que `mailing_audience()` les lit.
pub const KINDS: [&str; 4] = ["submitters", "speakers", "registrants", "roles"];

/// Un critère de destinataires (`MailingCriterion`). Les filtres vides valent
/// « le défaut du critère », écrit dans `engagement.mailing_audience()`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct MailingCriterion {
    pub kind: String,
    #[serde(default)]
    pub proposal_statuses: Vec<String>,
    #[serde(default)]
    pub registration_statuses: Vec<String>,
    #[serde(default)]
    pub role_codes: Vec<String>,
    #[serde(default)]
    pub session_id: Option<Uuid>,
}

/// L'écriture d'un brouillon (`MailingPayload`). Tous les champs sont
/// facultatifs : un brouillon s'enregistre à moitié rempli.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct MailingPayload {
    pub subject: Option<String>,
    pub body_html: Option<String>,
    pub audience: Option<Vec<MailingCriterion>>,
    pub extra_addresses: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    /// Chaîne vide : retirer l'adresse de réponse.
    pub reply_to: Option<String>,
    /// Changer d'édition, tant que c'est un brouillon : gardé sur la nouvelle.
    pub event_id: Option<Uuid>,
    /// `sobre`, `institutionnel` ou `annonce`.
    pub template: Option<String>,
    /// Valeur de secours par variable ; une chaîne vide retire le secours.
    pub variable_fallbacks: Option<HashMap<String, String>>,
    pub reply_button: Option<bool>,
    /// Bouton d'action de l'annonce : les deux, ou deux chaînes vides.
    pub cta_label: Option<String>,
    pub cta_url: Option<String>,
}

/// La création (`CreateMailingPayload`).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateMailingPayload {
    pub event_id: Uuid,
    #[serde(flatten)]
    pub draft: MailingPayload,
}

/// Les compteurs d'un envoi, par état de destinataire (`MailingCounts`).
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct MailingCounts {
    pub total: i64,
    pub queued: i64,
    pub sending: i64,
    pub sent: i64,
    pub failed: i64,
    pub suppressed: i64,
    pub cancelled: i64,
}

/// Une ligne de l'historique (`MailingSummary`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailingSummary {
    pub id: Uuid,
    pub event_id: Uuid,
    /// `draft`, `queued`, `sent`, `cancelled`.
    pub status: String,
    pub subject: String,
    pub created_by_name: Option<String>,
    pub queued_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub queued_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub counts: MailingCounts,
    pub attachment_count: i64,
}

/// L'écran de l'historique (`MailingListScreen`) : les envois, et les réglages
/// que l'écran annonce (copie de l'équipe, variables).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailingListScreen {
    pub rows: Vec<MailingSummary>,
    pub team_cc: Vec<String>,
    pub variables: Vec<String>,
}

/// Un envoi complet (`Mailing`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Mailing {
    pub id: Uuid,
    pub event_id: Uuid,
    pub status: String,
    pub subject: String,
    pub body_html: String,
    pub audience: Vec<MailingCriterion>,
    pub extra_addresses: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Option<String>,
    pub template: String,
    /// Objet nom de variable -> valeur de secours.
    pub variable_fallbacks: serde_json::Value,
    pub reply_button: bool,
    pub cta_label: Option<String>,
    pub cta_url: Option<String>,
    pub duplicated_from: Option<Uuid>,
    pub created_by_name: Option<String>,
    pub queued_by_name: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub queued_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub cancelled_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    pub counts: MailingCounts,
    /// Les destinataires figés, vides tant que l'envoi est un brouillon.
    pub recipients: Vec<MailingRecipient>,
    /// Les pièces jointes, dans l'ordre de leur rattachement.
    pub attachments: Vec<MailingAttachment>,
    pub attachments_total_bytes: i64,
    /// Le poids cumulé permis (`media.attachable_roles.max_total_byte_size`).
    pub attachments_max_bytes: Option<i64>,
}

/// Une pièce jointe (`MailingAttachment`) : un rattachement `attachment` de
/// `media.attachments` sur l'envoi, toujours privé.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailingAttachment {
    pub attachment_id: Uuid,
    pub asset_id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub byte_size: i64,
    /// `media.asset_status` : seul `ready` peut partir.
    pub status: String,
}

/// Un destinataire figé (`MailingRecipient`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailingRecipient {
    pub id: Uuid,
    pub email: String,
    pub person_id: Option<Uuid>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub organization: Option<String>,
    pub sources: Vec<String>,
    /// `queued`, `sending`, `sent`, `failed`, `suppressed`, `cancelled`.
    pub status: String,
    pub last_error: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub sent_at: Option<OffsetDateTime>,
}

/// Une adresse telle qu'elle partirait maintenant (`MailingAudienceEntry`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailingAudienceEntry {
    pub email: String,
    pub person_id: Option<Uuid>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub organization: Option<String>,
    pub sources: Vec<String>,
    pub is_suppressed: bool,
}

/// La liste avant envoi (`MailingAudience`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailingAudience {
    pub total: i64,
    /// Ce qui partira vraiment : le total moins les adresses supprimées.
    pub sendable: i64,
    pub suppressed: i64,
    /// Adresses joignables sans prénom connu : `{{prenom}}` y serait vide.
    pub without_first_name: i64,
    /// Adresses joignables qui ne sont celles d'aucun compte.
    pub without_account: i64,
    /// Destinataires qui recevront au moins une valeur de secours, pour les
    /// variables que l'objet et le corps citent.
    pub fallback_count: i64,
    /// Messages remis au serveur d'envoi : un par destinataire, plus un par
    /// adresse en copie ou en copie cachée, et ce pour chaque message.
    pub envelope_count: i64,
    pub entries: Vec<MailingAudienceEntry>,
}

/// Le courriel tel qu'un destinataire le recevra (`MailingPreview`), gabarit
/// compris. N'envoie rien.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailingPreview {
    pub to: String,
    pub has_account: bool,
    pub subject: String,
    pub html: String,
}

/// La réponse d'un envoi d'essai (`MailingTestResult`).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MailingTestResult {
    pub to: String,
}

/// Le contrôle d'une adresse, celui du domaine `platform.email`. Le vérifier
/// ici rend le refus sur le champ, avec l'adresse fautive.
pub fn adresse_valide(adresse: &str) -> bool {
    let Some((local, domaine)) = adresse.split_once('@') else {
        return false;
    };
    if local.is_empty() || domaine.contains('@') || adresse.chars().any(char::is_whitespace) {
        return false;
    }
    match domaine.rsplit_once('.') {
        Some((nom, tld)) => !nom.is_empty() && tld.chars().count() >= 2,
        None => false,
    }
}

/// Coupe, nettoie et dédoublonne une liste d'adresses saisie. Rend l'adresse
/// fautive en erreur.
pub fn normaliser_adresses(brutes: &[String]) -> Result<Vec<String>, String> {
    let mut sortie: Vec<String> = Vec::new();
    for brute in brutes {
        for morceau in brute.split([',', ';', '\n', ' ', '\t']) {
            let adresse = morceau.trim().trim_matches(['<', '>']).to_lowercase();
            if adresse.is_empty() {
                continue;
            }
            if !adresse_valide(&adresse) {
                return Err(adresse);
            }
            if !sortie.contains(&adresse) {
                sortie.push(adresse);
            }
        }
    }
    Ok(sortie)
}

/// Remplace `&`, `<`, `>`, `"` et `'` : une valeur insérée dans le corps HTML
/// ne doit pas pouvoir y ouvrir une balise.
pub fn echapper_html(valeur: &str) -> String {
    let mut sortie = String::with_capacity(valeur.len());
    for c in valeur.chars() {
        match c {
            '&' => sortie.push_str("&amp;"),
            '<' => sortie.push_str("&lt;"),
            '>' => sortie.push_str("&gt;"),
            '"' => sortie.push_str("&quot;"),
            '\'' => sortie.push_str("&#39;"),
            c => sortie.push(c),
        }
    }
    sortie
}

/// Les valeurs des variables d'un destinataire, toutes présentes : une valeur
/// inconnue est vide plutôt qu'absente, l'écran ayant prévenu avant l'envoi.
pub fn valeurs(
    variables: &serde_json::Value,
    secours: &serde_json::Value,
) -> HashMap<String, String> {
    VARIABLES
        .iter()
        .map(|nom| {
            let lue = variables
                .get(*nom)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .trim();
            // Une personne sans compte n'a ni prénom ni nom : la valeur de
            // secours (« Madame, Monsieur ») prend la place du vide.
            let valeur = if lue.is_empty() {
                secours
                    .get(*nom)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
            } else {
                lue
            };
            ((*nom).to_owned(), valeur.to_owned())
        })
        .collect()
}

/// Le repli texte d'un corps HTML : les fins de bloc deviennent des retours à
/// la ligne, les balises tombent, les entités courantes sont rendues. Un client
/// qui n'affiche pas le HTML montre ce texte, il doit donc rester lisible.
pub fn texte_brut(html: &str) -> String {
    let mut prepare = html.to_owned();
    for fin in [
        "</p>",
        "</h3>",
        "</h4>",
        "</blockquote>",
        "</ul>",
        "</ol>",
        "<hr>",
    ] {
        prepare = prepare.replace(fin, &format!("{fin}\n\n"));
    }
    for fin in ["</li>", "<br>", "<br/>", "<br />"] {
        prepare = prepare.replace(fin, &format!("{fin}\n"));
    }
    prepare = prepare.replace("<li>", "<li>- ");
    let mut sortie = String::with_capacity(prepare.len());
    let mut dans_une_balise = false;
    for c in prepare.chars() {
        match c {
            '<' => dans_une_balise = true,
            '>' => dans_une_balise = false,
            c if !dans_une_balise => sortie.push(c),
            _ => {}
        }
    }
    let sortie = sortie
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    let lignes: Vec<&str> = sortie.lines().map(str::trim).collect();
    let mut texte = lignes.join("\n");
    while texte.contains("\n\n\n") {
        texte = texte.replace("\n\n\n", "\n\n");
    }
    texte.trim().to_owned()
}

/// Le gabarit d'un courriel du back-office : le corps tel qu'écrit, dans une
/// enveloppe qui fixe la police et la largeur de lecture.
pub fn enveloppe_html(corps: &str) -> String {
    format!(
        "<!doctype html><html><body style=\"margin:0;padding:24px;font-family:Arial,Helvetica,sans-serif;font-size:15px;line-height:1.55;color:#1f2933\"><div style=\"max-width:640px\">{corps}</div></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_repli_texte_garde_les_paragraphes() {
        assert_eq!(
            texte_brut("<p>Bonjour <strong>Awa</strong>,</p><p>À demain &amp; merci.</p>"),
            "Bonjour Awa,\n\nÀ demain & merci."
        );
    }

    #[test]
    fn les_adresses_saisies_se_coupent_et_se_dedoublonnent() {
        let brutes = vec![
            "Awa@Example.org, b@example.org; awa@example.org".to_owned(),
            "  <c@example.org>  ".to_owned(),
        ];
        assert_eq!(
            normaliser_adresses(&brutes).unwrap(),
            vec!["awa@example.org", "b@example.org", "c@example.org"]
        );
    }

    #[test]
    fn une_adresse_fautive_est_nommee() {
        let brutes = vec!["a@example.org, pas-une-adresse".to_owned()];
        assert_eq!(normaliser_adresses(&brutes).unwrap_err(), "pas-une-adresse");
        assert!(!adresse_valide("a@b.c"));
        assert!(adresse_valide("a@b.co"));
    }

    #[test]
    fn une_valeur_ne_peut_pas_ouvrir_de_balise() {
        assert_eq!(echapper_html("<b>A&B</b>"), "&lt;b&gt;A&amp;B&lt;/b&gt;");
    }

    #[test]
    fn toutes_les_variables_ont_une_valeur() {
        let secours = serde_json::json!({ "prenom": "Madame, Monsieur" });
        let v = valeurs(&serde_json::json!({ "prenom": "Awa" }), &secours);
        assert_eq!(v.len(), VARIABLES.len());
        assert_eq!(v["prenom"], "Awa");
        assert_eq!(v["nom"], "");
        let sans_compte = valeurs(&serde_json::json!({ "prenom": "" }), &secours);
        assert_eq!(sans_compte["prenom"], "Madame, Monsieur");
    }
}
