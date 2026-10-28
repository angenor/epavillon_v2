//! Les gabarits d'un courriel du back-office (28/10).
//!
//! HTML de messagerie : tableaux de présentation, styles en ligne, 600 px de
//! large, aucune feuille de style ni police distante. C'est ce qu'Outlook et
//! Microsoft 365 savent rendre ; un `div` en flex y serait illisible. Couleurs
//! de la charte en hexadécimal (`design-tokens.css`) : un courriel n'a pas
//! accès aux jetons.

/// Bleu nuit de la charte (`--ifdd-bleu-riche`).
const BLEU_NUIT: &str = "#1d1a5b";
/// Cyan foncé (`--ifdd-cyan-700`), 6,2:1 sur blanc : le fond d'un bouton.
const CYAN: &str = "#0c6792";
/// Le liseré aux couleurs du logo : cyan, vert, jaune, rouge, violet.
const LISERE: [&str; 5] = ["#00a1e4", "#8fbf2f", "#ffd500", "#e63132", "#732f85"];
const TEXTE: &str = "#231f20";
const POLICE: &str = "Arial,Helvetica,sans-serif";

pub const GABARITS: [&str; 3] = ["sobre", "institutionnel", "annonce"];

/// Ce qui habille le corps rendu.
pub struct MiseEnPage<'a> {
    pub gabarit: &'a str,
    /// L'objet, déjà rendu : le grand titre de l'annonce.
    pub sujet: &'a str,
    /// Le bouton d'action de l'annonce : texte et lien.
    pub action: Option<(&'a str, &'a str)>,
    /// Le lien `mailto:` du bouton Répondre, s'il est demandé.
    pub repondre: Option<String>,
    /// L'adresse publique du site, pour le logo.
    pub site: &'a str,
}

/// Le courriel complet, gabarit appliqué.
pub fn mettre_en_page(corps: &str, m: &MiseEnPage<'_>) -> String {
    let corps = boutons(corps);
    let mut contenu = String::new();
    if m.gabarit == "annonce" {
        contenu.push_str(&format!(
            "<h1 style=\"margin:0 0 16px;font-family:{POLICE};font-size:26px;line-height:1.25;color:{BLEU_NUIT}\">{}</h1>",
            echapper(m.sujet)
        ));
    }
    contenu.push_str(&corps);
    if m.gabarit == "annonce" {
        if let Some((texte, lien)) = m.action {
            contenu.push_str(&bouton(lien, texte));
        }
    }
    if let Some(lien) = &m.repondre {
        contenu.push_str(&bouton(lien, "Répondre"));
    }
    if m.gabarit == "sobre" {
        contenu.push_str(&signature());
    }

    let mut lignes = String::new();
    if m.gabarit != "sobre" {
        lignes.push_str(&bandeau(m.site));
    }
    lignes.push_str(&format!(
        "<tr><td style=\"padding:28px 32px;font-family:{POLICE};font-size:15px;line-height:1.6;color:{TEXTE}\">{contenu}</td></tr>"
    ));
    if m.gabarit == "institutionnel" {
        lignes.push_str(&pied());
    } else if m.gabarit == "annonce" {
        lignes.push_str(&format!(
            "<tr><td style=\"padding:0 32px 28px;font-family:{POLICE};font-size:15px;color:{TEXTE}\">{}</td></tr>",
            signature()
        ));
        lignes.push_str(&pied());
    }

    format!(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"></head>\
<body style=\"margin:0;padding:0;background:#f4f5f7\">\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"background:#f4f5f7\"><tr><td align=\"center\" style=\"padding:24px 12px\">\
<table role=\"presentation\" width=\"600\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"width:600px;max-width:100%;background:#ffffff;border:1px solid #e3e5ea\">{lignes}</table>\
</td></tr></table></body></html>"
    )
}

fn bandeau(site: &str) -> String {
    let logo = format!("{}/logos/png/epavillon@4x.png", site.trim_end_matches('/'));
    let lisere: String = LISERE
        .iter()
        .map(|c| {
            format!(
                "<td height=\"4\" style=\"background:{c};font-size:0;line-height:0\">&nbsp;</td>"
            )
        })
        .collect();
    format!(
        "<tr><td style=\"background:{BLEU_NUIT};padding:16px 32px\">\
<table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>\
<td style=\"padding-right:12px\"><img src=\"{logo}\" alt=\"ePavillon\" width=\"36\" height=\"46\" style=\"display:block;border:0\"></td>\
<td style=\"font-family:{POLICE};font-size:16px;font-weight:bold;color:#ffffff\">ePavillon de la Francophonie</td>\
</tr></table></td></tr>\
<tr><td style=\"padding:0\"><table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>{lisere}</tr></table></td></tr>"
    )
}

fn pied() -> String {
    format!(
        "<tr><td style=\"background:#f4f5f7;padding:20px 32px;font-family:{POLICE};font-size:12px;line-height:1.5;color:#565554\">\
Institut de la Francophonie pour le développement durable (IFDD), organe subsidiaire de l'Organisation internationale de la Francophonie.<br>\
<a href=\"https://www.ifdd.francophonie.org\" style=\"color:{CYAN}\">ifdd.francophonie.org</a> · \
<a href=\"https://www.francophonie.org\" style=\"color:{CYAN}\">francophonie.org</a></td></tr>"
    )
}

fn signature() -> String {
    format!(
        "<p style=\"margin:24px 0 0;font-family:{POLICE};font-size:15px;color:{TEXTE}\">L'équipe de l'ePavillon<br>\
<span style=\"color:#565554\">Institut de la Francophonie pour le développement durable (IFDD)</span></p>"
    )
}

/// Un bouton « à l'épreuve d'Outlook » : une cellule colorée, un lien dedans.
fn bouton(lien: &str, texte: &str) -> String {
    format!(
        "<table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"margin:20px 0\"><tr>\
<td style=\"background:{CYAN};border-radius:6px\"><a href=\"{}\" style=\"display:inline-block;padding:12px 24px;font-family:{POLICE};font-size:15px;font-weight:bold;color:#ffffff;text-decoration:none\">{}</a></td>\
</tr></table>",
        echapper(lien),
        echapper(texte)
    )
}

/// Les liens `class="bouton"` posés par l'éditeur (« Insérer un bouton »)
/// deviennent des boutons. Le corps est déjà assaini : le lien et son texte y
/// sont sûrs, on ne fait que les replacer.
pub fn boutons(corps: &str) -> String {
    let mut sortie = String::with_capacity(corps.len());
    let mut reste = corps;
    while let Some(debut) = reste.find("<a ") {
        let Some(fin_ouvrante) = reste[debut..].find('>').map(|i| debut + i) else {
            break;
        };
        let ouvrante = &reste[debut..=fin_ouvrante];
        let Some(fermante) = reste[fin_ouvrante..].find("</a>").map(|i| fin_ouvrante + i) else {
            break;
        };
        if ouvrante.contains("class=\"bouton\"") {
            if let Some(lien) = attribut(ouvrante, "href") {
                let texte = &reste[fin_ouvrante + 1..fermante];
                sortie.push_str(&reste[..debut]);
                sortie.push_str(&bouton_brut(&lien, texte));
                reste = &reste[fermante + 4..];
                continue;
            }
        }
        sortie.push_str(&reste[..fermante + 4]);
        reste = &reste[fermante + 4..];
    }
    sortie.push_str(reste);
    sortie
}

/// Comme [`bouton`], pour un lien et un texte déjà échappés par l'assainisseur.
fn bouton_brut(lien: &str, texte: &str) -> String {
    format!(
        "</p><table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"margin:20px 0\"><tr>\
<td style=\"background:{CYAN};border-radius:6px\"><a href=\"{lien}\" style=\"display:inline-block;padding:12px 24px;font-family:{POLICE};font-size:15px;font-weight:bold;color:#ffffff;text-decoration:none\">{texte}</a></td>\
</tr></table><p style=\"margin:0\">"
    )
}

fn attribut(balise: &str, nom: &str) -> Option<String> {
    let motif = format!("{nom}=\"");
    let debut = balise.find(&motif)? + motif.len();
    let fin = balise[debut..].find('"')? + debut;
    Some(balise[debut..fin].to_owned())
}

fn echapper(valeur: &str) -> String {
    super::mailing::echapper_html(valeur)
}

/// Le lien du bouton Répondre : vers l'adresse de réponse, ou la première
/// copie, avec les copies en copie et l'objet repris. `None` : personne à qui
/// répondre que la boîte d'envoi, qui ne se lit pas.
pub fn mailto_reponse(reply_to: Option<&str>, cc: &[String], sujet: &str) -> Option<String> {
    let destinataire = reply_to
        .map(str::to_owned)
        .or_else(|| cc.first().cloned())?;
    let copies: Vec<&String> = cc.iter().filter(|c| **c != destinataire).collect();
    let mut lien = format!(
        "mailto:{destinataire}?subject={}",
        encoder(&format!("RE: {sujet}"))
    );
    if !copies.is_empty() {
        let liste: Vec<&str> = copies.iter().map(|c| c.as_str()).collect();
        lien.push_str(&format!("&cc={}", encoder(&liste.join(","))));
    }
    Some(lien)
}

/// Encodage d'URL : tout ce qui n'est ni lettre, ni chiffre, ni `-._~@` passe
/// en `%XX`, octet par octet.
fn encoder(texte: &str) -> String {
    let mut sortie = String::new();
    for octet in texte.as_bytes() {
        match octet {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'@' => {
                sortie.push(*octet as char)
            }
            _ => sortie.push_str(&format!("%{octet:02X}")),
        }
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_lien_bouton_devient_un_bouton() {
        let corps = r#"<p>Avant</p><p><a href="https://exemple.org/x" class="bouton">S'inscrire</a></p><p><a href="https://a.org">lien</a></p>"#;
        let rendu = boutons(corps);
        assert!(rendu.contains("background:#0c6792"), "{rendu}");
        assert!(
            rendu.contains(r#"<a href="https://exemple.org/x" style="#),
            "{rendu}"
        );
        assert!(rendu.contains(r#"<a href="https://a.org">lien</a>"#));
    }

    #[test]
    fn le_bouton_repondre_garde_les_copies() {
        let cc = vec!["equipe@ifdd.org".to_owned(), "moi@ifdd.org".to_owned()];
        let lien = mailto_reponse(None, &cc, "Programme").unwrap();
        assert_eq!(
            lien,
            "mailto:equipe@ifdd.org?subject=RE%3A%20Programme&cc=moi@ifdd.org"
        );
        assert!(mailto_reponse(None, &[], "x").is_none());
    }

    #[test]
    fn les_trois_gabarits_tiennent_dans_600_pixels() {
        for gabarit in GABARITS {
            let html = mettre_en_page(
                "<p>Bonjour</p>",
                &MiseEnPage {
                    gabarit,
                    sujet: "Objet",
                    action: Some(("Voir", "https://exemple.org")),
                    repondre: None,
                    site: "https://epavillonclimatique.francophonie.org",
                },
            );
            assert!(html.contains("width=\"600\""));
            assert_eq!(
                html.contains("logos/png/epavillon@4x.png"),
                gabarit != "sobre"
            );
            assert_eq!(html.contains(">Voir</a>"), gabarit == "annonce");
        }
    }
}
