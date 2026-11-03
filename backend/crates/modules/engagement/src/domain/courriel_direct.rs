//! Le courriel d'inscription qui mène au direct, journée ou activité (02/11),
//! et les rappels programmés (04/11) qui en reprennent l'habillage.
//!
//! Gabarit « institutionnel + logo de l'édition », un bouton (« Rejoindre le
//! direct », « Voir la journée »…) et le même lien en clair pour la messagerie
//! qui n'affiche pas le bouton. Aucun lien de désinscription : demande du
//! commanditaire. C'est la plateforme qui envoie le lien, jamais Zoom : on suit
//! le direct sur le site, par le SDK.

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

/// Un paragraphe, déjà échappé pour le HTML et brut pour le texte.
pub struct Paragraphe {
    pub html: String,
    pub texte: String,
}

impl Paragraphe {
    pub fn brut(texte: &str) -> Self {
        Self {
            html: echapper_html(texte),
            texte: texte.to_owned(),
        }
    }
}

/// Le courriel avant mise en page : paragraphes, bouton et lien en clair.
pub struct Assemblage<'a> {
    pub anglais: bool,
    pub sujet: &'a str,
    pub avant: &'a [Paragraphe],
    pub lien: &'a str,
    /// Le texte du bouton.
    pub action: &'a str,
    pub apres: &'a [Paragraphe],
}

pub fn composer(
    c: &CourrielDirect,
    visuels: Option<&Visuels>,
    to: &str,
    locale: &str,
    message_id: &str,
) -> OutgoingMail {
    let action = if c.anglais {
        "Join the live stream"
    } else {
        "Rejoindre le direct"
    };
    let avant: Vec<Paragraphe> = c.avant.iter().map(|p| Paragraphe::brut(p)).collect();
    let apres: Vec<Paragraphe> = c.apres.iter().map(|p| Paragraphe::brut(p)).collect();
    assembler(
        &Assemblage {
            anglais: c.anglais,
            sujet: &c.sujet,
            avant: &avant,
            lien: &c.lien,
            action,
            apres: &apres,
        },
        visuels,
        to,
        locale,
        message_id,
    )
}

pub fn assembler(
    a: &Assemblage<'_>,
    visuels: Option<&Visuels>,
    to: &str,
    locale: &str,
    message_id: &str,
) -> OutgoingMail {
    let (repli, signature) = if a.anglais {
        (
            "If the button does not work, use the following link:",
            [
                "The ePavillon team",
                "Institut de la Francophonie pour le développement durable (IFDD)",
            ],
        )
    } else {
        (
            "Si le bouton ne fonctionne pas, utilisez le lien suivant :",
            [
                "L'équipe de l'ePavillon",
                "Institut de la Francophonie pour le développement durable (IFDD)",
            ],
        )
    };
    let paragraphe = |p: &Paragraphe| format!("<p style=\"margin:0 0 16px\">{}</p>", p.html);
    let lien = echapper_html(a.lien);

    let mut corps: String = a.avant.iter().map(paragraphe).collect();
    corps.push_str(&bouton(a.lien, a.action));
    corps.push_str(&format!(
        "<p style=\"margin:0 0 16px;font-size:13px;color:#565554\">{}<br>\
<a href=\"{lien}\" style=\"color:#0c6792;word-break:break-all\">{lien}</a></p>",
        echapper_html(repli)
    ));
    corps.extend(a.apres.iter().map(paragraphe));
    corps.push_str(&format!(
        "<p style=\"margin:24px 0 0\">{}<br><span style=\"color:#565554\">{}</span></p>",
        echapper_html(signature[0]),
        echapper_html(signature[1])
    ));
    let html = mettre_en_page(
        &corps,
        &MiseEnPage {
            gabarit: GABARIT,
            sujet: a.sujet,
            action: None,
            repondre: None,
            visuels,
        },
    );

    let mut texte = a
        .avant
        .iter()
        .map(|p| p.texte.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let deux_points = if a.anglais { ":" } else { " :" };
    texte.push_str(&format!("\n\n{}{deux_points}\n{}\n\n", a.action, a.lien));
    for p in a.apres {
        texte.push_str(&p.texte);
        texte.push_str("\n\n");
    }
    texte.push_str(&signature.join("\n"));
    texte.push('\n');

    OutgoingMail {
        message_id: message_id.to_owned(),
        to: to.to_owned(),
        locale: locale.to_owned(),
        subject: a.sujet.to_owned(),
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
