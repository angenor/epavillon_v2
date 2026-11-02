//! Le courriel d'inscription qui mène au direct, journée ou activité (02/11).
//!
//! Gabarit « institutionnel + logo de l'édition », un bouton « Rejoindre le
//! direct » et le même lien en clair pour la messagerie qui n'affiche pas le
//! bouton. Aucun lien de désinscription : demande du commanditaire. C'est la
//! plateforme qui envoie le lien, jamais Zoom : on suit le direct sur le site,
//! par le SDK.

use kernel::mail::OutgoingMail;

use super::gabarits::{bouton, mettre_en_page, MiseEnPage, Visuels};
use super::mailing::echapper_html;

pub const GABARIT: &str = "institutionnel_logo";

pub struct CourrielDirect {
    pub anglais: bool,
    pub sujet: String,
    /// Avant le bouton, en texte brut : chaque entrée fait un paragraphe.
    pub avant: Vec<String>,
    pub lien: String,
    /// Après le lien en clair.
    pub apres: Vec<String>,
}

pub fn composer(
    c: &CourrielDirect,
    visuels: Option<&Visuels>,
    to: &str,
    locale: &str,
    message_id: &str,
) -> OutgoingMail {
    let (action, repli, signature) = if c.anglais {
        (
            "Join the live stream",
            "If the button does not work, use the following link:",
            [
                "The ePavillon team",
                "Institut de la Francophonie pour le développement durable (IFDD)",
            ],
        )
    } else {
        (
            "Rejoindre le direct",
            "Si le bouton ne fonctionne pas, utilisez le lien suivant :",
            [
                "L'équipe de l'ePavillon",
                "Institut de la Francophonie pour le développement durable (IFDD)",
            ],
        )
    };
    let paragraphe = |t: &str| format!("<p style=\"margin:0 0 16px\">{}</p>", echapper_html(t));
    let lien = echapper_html(&c.lien);

    let mut corps: String = c.avant.iter().map(|p| paragraphe(p)).collect();
    corps.push_str(&bouton(&c.lien, action));
    corps.push_str(&format!(
        "<p style=\"margin:0 0 16px;font-size:13px;color:#565554\">{}<br>\
<a href=\"{lien}\" style=\"color:#0c6792;word-break:break-all\">{lien}</a></p>",
        echapper_html(repli)
    ));
    corps.extend(c.apres.iter().map(|p| paragraphe(p)));
    corps.push_str(&format!(
        "<p style=\"margin:24px 0 0\">{}<br><span style=\"color:#565554\">{}</span></p>",
        echapper_html(signature[0]),
        echapper_html(signature[1])
    ));
    let html = mettre_en_page(
        &corps,
        &MiseEnPage {
            gabarit: GABARIT,
            sujet: &c.sujet,
            action: None,
            repondre: None,
            visuels,
        },
    );

    let mut texte = c.avant.join("\n\n");
    let deux_points = if c.anglais { ":" } else { " :" };
    texte.push_str(&format!("\n\n{action}{deux_points}\n{}\n\n", c.lien));
    for p in &c.apres {
        texte.push_str(p);
        texte.push_str("\n\n");
    }
    texte.push_str(&signature.join("\n"));
    texte.push('\n');

    OutgoingMail {
        message_id: message_id.to_owned(),
        to: to.to_owned(),
        locale: locale.to_owned(),
        subject: c.sujet.clone(),
        text: texte,
        html: Some(html),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_lien_suit_le_bouton_en_clair_et_rien_ne_desinscrit() {
        let c = CourrielDirect {
            anglais: false,
            sujet: "Inscription confirmée : Atelier".to_owned(),
            avant: vec!["Bonjour Awa,".to_owned()],
            lien: "https://site.org/direct?jeton=a&b".to_owned(),
            apres: vec![],
        };
        let m = composer(&c, None, "awa@example.org", "fr", "j");
        let html = m.html.unwrap();
        assert!(html.contains(">Rejoindre le direct</a>"));
        assert!(html.contains("Si le bouton ne fonctionne pas, utilisez le lien suivant :"));
        assert!(html.contains("https://site.org/direct?jeton=a&amp;b"));
        assert!(m
            .text
            .contains("Rejoindre le direct :\nhttps://site.org/direct?jeton=a&b"));
        assert!(!m.text.to_lowercase().contains("désinscri"));
        assert!(!html.contains('\u{2014}'));
    }
}
