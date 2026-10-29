//! Les gabarits d'un courriel du back-office (28/10).
//!
//! HTML de messagerie : tableaux de présentation, styles en ligne, 600 px de
//! large, aucune feuille de style ni police distante. C'est ce qu'Outlook et
//! Microsoft 365 savent rendre ; un `div` en flex y serait illisible. Couleurs
//! de la charte en hexadécimal (`design-tokens.css`) : un courriel n'a pas
//! accès aux jetons.

use kernel::mail::InlineImage;

/// Bleu nuit de la charte (`--ifdd-bleu-riche`).
const BLEU_NUIT: &str = "#1d1a5b";
/// Cyan foncé (`--ifdd-cyan-700`), 6,2:1 sur blanc : le fond d'un bouton.
const CYAN: &str = "#0c6792";
/// Le liseré aux couleurs du logo : cyan, vert, jaune, rouge, violet.
const LISERE: [&str; 5] = ["#00a1e4", "#8fbf2f", "#ffd500", "#e63132", "#732f85"];
/// La seconde ligne du nom, « de la Francophonie » (`--color-text-on-inverse-muted`).
const BLEU_PALE: &str = "#bac1ea";
const TEXTE: &str = "#231f20";
const POLICE: &str = "Arial,Helvetica,sans-serif";

pub const GABARITS: [&str; 5] = [
    "sobre",
    "institutionnel",
    "annonce",
    "banniere_edition",
    "institutionnel_edition",
];

/// Le symbole inversé de l'ePavillon (« e » blanc), 84 x 80 px affiché en
/// 42 x 40. **Intégré au message**, jamais par adresse : Outlook bloque les
/// images distantes jusqu'au clic, pas les images intégrées.
const LOGO_EPAVILLON: &[u8] = include_bytes!("../../assets/logo-epavillon-symbole-inverse.png");
pub const CID_LOGO: &str = "logo-epavillon";

/// Une image de l'édition prête à citer : adresse publique (ou `cid:`) et
/// dimensions du fichier servi, d'où se déduisent les attributs `width` et
/// `height` qu'Outlook exige.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageEdition {
    pub src: String,
    pub largeur: u32,
    pub hauteur: u32,
}

/// L'édition de l'envoi, telle que ses deux gabarits la montrent.
#[derive(Debug, Clone)]
pub struct Edition {
    pub nom: String,
    /// « Du 9 au 20 novembre 2026, Antalya ».
    pub dates: Option<String>,
    /// La page publique de l'édition.
    pub lien: String,
    pub banniere: Option<ImageEdition>,
    pub logo: Option<ImageEdition>,
}

/// Ce qui habille le corps rendu.
pub struct MiseEnPage<'a> {
    pub gabarit: &'a str,
    /// L'objet, déjà rendu : le grand titre de l'annonce.
    pub sujet: &'a str,
    /// Le bouton d'action de l'annonce : texte et lien.
    pub action: Option<(&'a str, &'a str)>,
    /// Le lien `mailto:` du bouton Répondre, s'il est demandé.
    pub repondre: Option<String>,
    /// L'édition de l'envoi : sa bannière et son logo, pour ses deux gabarits.
    pub edition: Option<&'a Edition>,
}

/// L'en-tête que le gabarit prend vraiment : sans l'image qu'il attend, un
/// gabarit de l'édition retombe sur l'institutionnel.
enum Tete<'a> {
    Aucune,
    Institutionnelle(Option<(&'a Edition, &'a ImageEdition)>),
    Banniere(&'a Edition, &'a ImageEdition),
}

fn tete<'a>(gabarit: &str, edition: Option<&'a Edition>) -> Tete<'a> {
    match gabarit {
        "sobre" => Tete::Aucune,
        "banniere_edition" => match edition.and_then(|e| e.banniere.as_ref().map(|b| (e, b))) {
            Some((e, b)) => Tete::Banniere(e, b),
            None => Tete::Institutionnelle(None),
        },
        "institutionnel_edition" => {
            Tete::Institutionnelle(edition.and_then(|e| e.logo.as_ref().map(|l| (e, l))))
        }
        _ => Tete::Institutionnelle(None),
    }
}

/// Les images à intégrer au message pour ce gabarit : le symbole de l'ePavillon
/// quand l'en-tête bleu nuit est dessiné.
pub fn images_integrees(gabarit: &str, edition: Option<&Edition>) -> Vec<InlineImage> {
    match tete(gabarit, edition) {
        Tete::Institutionnelle(_) => vec![InlineImage {
            content_id: CID_LOGO.to_owned(),
            mime_type: "image/png".to_owned(),
            content: LOGO_EPAVILLON.to_vec(),
        }],
        _ => vec![],
    }
}

/// L'aperçu de l'écran n'a pas les parties du message : le symbole y est cité
/// en `data:`, le reste est identique.
pub fn pour_apercu(html: &str) -> String {
    use base64::Engine;
    let donnees = base64::engine::general_purpose::STANDARD.encode(LOGO_EPAVILLON);
    html.replace(
        &format!("cid:{CID_LOGO}"),
        &format!("data:image/png;base64,{donnees}"),
    )
}

/// Le courriel complet, gabarit appliqué.
pub fn mettre_en_page(corps: &str, m: &MiseEnPage<'_>) -> String {
    let corps = boutons(corps);
    let annonce = m.gabarit == "annonce";
    let mut contenu = String::new();
    if annonce {
        contenu.push_str(&format!(
            "<h1 style=\"margin:0 0 16px;font-family:{POLICE};font-size:26px;line-height:1.25;color:{BLEU_NUIT}\">{}</h1>",
            echapper(m.sujet)
        ));
    }
    contenu.push_str(&corps);
    if annonce {
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

    let mut lignes = match tete(m.gabarit, m.edition) {
        Tete::Aucune => String::new(),
        Tete::Institutionnelle(logo) => bandeau(logo) + &lisere(),
        Tete::Banniere(edition, image) => banniere(edition, image) + &lisere(),
    };
    lignes.push_str(&format!(
        "<tr><td style=\"padding:28px 32px;font-family:{POLICE};font-size:15px;line-height:1.6;color:{TEXTE}\">{contenu}</td></tr>"
    ));
    if annonce {
        lignes.push_str(&format!(
            "<tr><td style=\"padding:0 32px 28px;font-family:{POLICE};font-size:15px;color:{TEXTE}\">{}</td></tr>",
            signature()
        ));
    }
    if m.gabarit != "sobre" {
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

/// L'en-tête bleu nuit : le symbole, le nom sur deux lignes comme la barre du
/// site, et, pour le gabarit de l'édition, son logo à l'extrême droite sur une
/// tuile blanche (un logo foncé disparaîtrait sur le bleu nuit).
fn bandeau(logo: Option<(&Edition, &ImageEdition)>) -> String {
    let tuile = logo
        .map(|(edition, image)| {
            let (l, h) = ajuster(image, 40, 140);
            format!(
                "<td align=\"right\" style=\"vertical-align:middle;padding-left:16px\">\
<table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" align=\"right\"><tr>\
<td style=\"background:#ffffff;border-radius:6px;padding:6px 10px\">\
<img src=\"{}\" alt=\"{}\" width=\"{l}\" height=\"{h}\" style=\"display:block;border:0;width:{l}px;height:{h}px\"></td>\
</tr></table></td>",
                echapper(&image.src),
                echapper(&edition.nom)
            )
        })
        .unwrap_or_default();
    format!(
        "<tr><td style=\"background:{BLEU_NUIT};padding:16px 32px\">\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>\
<td width=\"42\" style=\"width:42px;padding-right:12px;vertical-align:middle\"><img src=\"cid:{CID_LOGO}\" alt=\"\" width=\"42\" height=\"40\" style=\"display:block;border:0;width:42px;height:40px\"></td>\
<td style=\"vertical-align:middle;font-family:{POLICE}\">\
<p style=\"margin:0;font-size:20px;line-height:22px;mso-line-height-rule:exactly;letter-spacing:-0.01em;color:#ffffff\"><span style=\"font-weight:300\">e</span><span style=\"font-weight:bold\">Pavillon</span></p>\
<p style=\"margin:5px 0 0;font-size:11px;line-height:13px;mso-line-height-rule:exactly;letter-spacing:0.16em;text-transform:uppercase;color:{BLEU_PALE}\">de la Francophonie</p>\
</td>{tuile}</tr></table></td></tr>"
    )
}

/// La bannière de l'édition sur toute la largeur, cliquable vers sa page, puis
/// une bande blanche : son logo à gauche, son nom et ses dates à droite.
fn banniere(edition: &Edition, image: &ImageEdition) -> String {
    let hauteur = (600 * u64::from(image.hauteur) / u64::from(image.largeur.max(1))).max(1);
    let lien = echapper(&edition.lien);
    let nom = echapper(&edition.nom);
    let logo = edition
        .logo
        .as_ref()
        .map(|l| {
            let (lg, h) = ajuster(l, 48, 160);
            format!(
                "<td width=\"{lg}\" style=\"width:{lg}px;padding-right:16px;vertical-align:middle\">\
<img src=\"{}\" alt=\"\" width=\"{lg}\" height=\"{h}\" style=\"display:block;border:0;width:{lg}px;height:{h}px\"></td>",
                echapper(&l.src)
            )
        })
        .unwrap_or_default();
    let alignement = if edition.logo.is_some() {
        "right"
    } else {
        "left"
    };
    let dates = edition
        .dates
        .as_deref()
        .map(|d| {
            format!(
                "<p style=\"margin:4px 0 0;font-size:14px;line-height:18px;mso-line-height-rule:exactly;color:{BLEU_NUIT}\">{}</p>",
                echapper(d)
            )
        })
        .unwrap_or_default();
    format!(
        "<tr><td style=\"padding:0;font-size:0;line-height:0\"><a href=\"{lien}\" style=\"display:block\">\
<img src=\"{}\" alt=\"{nom}\" width=\"600\" height=\"{hauteur}\" style=\"display:block;border:0;width:100%;max-width:600px;height:auto\"></a></td></tr>\
<tr><td style=\"background:#ffffff;padding:16px 32px\">\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>{logo}\
<td align=\"{alignement}\" style=\"vertical-align:middle;font-family:{POLICE};text-align:{alignement}\">\
<p style=\"margin:0;font-size:17px;line-height:22px;mso-line-height-rule:exactly;font-weight:bold;color:{BLEU_NUIT}\">{nom}</p>{dates}\
</td></tr></table></td></tr>",
        echapper(&image.src)
    )
}

fn lisere() -> String {
    let cellules: String = LISERE
        .iter()
        .map(|c| {
            format!(
                "<td height=\"4\" style=\"background:{c};font-size:0;line-height:0\">&nbsp;</td>"
            )
        })
        .collect();
    format!(
        "<tr><td style=\"padding:0\"><table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>{cellules}</tr></table></td></tr>"
    )
}

/// Largeur et hauteur d'affichage : la hauteur visée, sans dépasser la largeur
/// permise (un logo très allongé rapetisse plutôt que de pousser le texte).
fn ajuster(image: &ImageEdition, hauteur: u32, largeur_max: u32) -> (u32, u32) {
    let (l, h) = (
        u64::from(image.largeur.max(1)),
        u64::from(image.hauteur.max(1)),
    );
    let largeur = (l * u64::from(hauteur) / h).max(1);
    if largeur <= u64::from(largeur_max) {
        (largeur as u32, hauteur)
    } else {
        (largeur_max, (h * u64::from(largeur_max) / l).max(1) as u32)
    }
}

/// L'image d'un rôle de l'édition, telle que `media.attached_image()` la rend,
/// réduite au fichier à citer : une variante JPEG ou PNG (Outlook ne lit pas le
/// WebP), ou l'original s'il est l'un des deux. La plus petite qui atteint la
/// taille demandée, sinon la plus grande : ni flou, ni original de plusieurs Mo.
pub fn choisir_image(
    image: &serde_json::Value,
    largeur_min: u32,
    hauteur_min: u32,
) -> Option<ImageEdition> {
    let candidat = |v: &serde_json::Value| -> Option<ImageEdition> {
        let src = v.get("url")?.as_str()?.to_owned();
        let largeur = u32::try_from(v.get("width")?.as_u64()?).ok()?;
        let hauteur = u32::try_from(v.get("height")?.as_u64()?).ok()?;
        (largeur > 0 && hauteur > 0).then_some(ImageEdition {
            src,
            largeur,
            hauteur,
        })
    };
    let mut candidats: Vec<ImageEdition> = image
        .get("sources")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(cle, _)| cle.ends_with("_jpeg") || cle.ends_with("_png"))
        .filter_map(|(_, v)| candidat(v))
        .collect();
    if let Some(original) = candidat(image) {
        let chemin = original.src.to_ascii_lowercase();
        let chemin = chemin.split(['?', '#']).next().unwrap_or_default();
        if [".jpg", ".jpeg", ".png"]
            .iter()
            .any(|e| chemin.ends_with(e))
        {
            candidats.push(original);
        }
    }
    candidats.sort_by_key(|c| u64::from(c.largeur) * u64::from(c.hauteur));
    let suffisant = candidats
        .iter()
        .position(|c| c.largeur >= largeur_min && c.hauteur >= hauteur_min);
    match suffisant {
        Some(i) => Some(candidats.swap_remove(i)),
        None => candidats.pop(),
    }
}

/// La période d'une édition en toutes lettres : « Du 9 au 20 novembre 2026 ».
pub fn periode(debut: time::Date, fin: time::Date) -> String {
    let jour = |d: time::Date| match d.day() {
        1 => "1er".to_owned(),
        n => n.to_string(),
    };
    let mois = |d: time::Date| MOIS[usize::from(u8::from(d.month())) - 1];
    if debut == fin {
        return format!("Le {} {} {}", jour(debut), mois(debut), debut.year());
    }
    if debut.year() != fin.year() {
        return format!(
            "Du {} {} {} au {} {} {}",
            jour(debut),
            mois(debut),
            debut.year(),
            jour(fin),
            mois(fin),
            fin.year()
        );
    }
    if debut.month() != fin.month() {
        return format!(
            "Du {} {} au {} {} {}",
            jour(debut),
            mois(debut),
            jour(fin),
            mois(fin),
            fin.year()
        );
    }
    format!(
        "Du {} au {} {} {}",
        jour(debut),
        jour(fin),
        mois(fin),
        fin.year()
    )
}

const MOIS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];

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

    fn edition(banniere: bool, logo: bool) -> Edition {
        Edition {
            nom: "CdP31 <Antalya>".into(),
            dates: Some("Du 9 au 20 novembre 2026, Antalya".into()),
            lien: "https://site.org/evenements/cdp31".into(),
            banniere: banniere.then(|| ImageEdition {
                src: "https://site.org/b.jpeg".into(),
                largeur: 1600,
                hauteur: 900,
            }),
            logo: logo.then(|| ImageEdition {
                src: "https://site.org/l.png".into(),
                largeur: 320,
                hauteur: 160,
            }),
        }
    }

    fn rendre(gabarit: &str, edition: Option<&Edition>) -> String {
        mettre_en_page(
            "<p>Bonjour</p>",
            &MiseEnPage {
                gabarit,
                sujet: "Objet",
                action: Some(("Voir", "https://exemple.org")),
                repondre: None,
                edition,
            },
        )
    }

    #[test]
    fn les_gabarits_tiennent_dans_600_pixels() {
        let e = edition(true, true);
        for gabarit in GABARITS {
            let html = rendre(gabarit, Some(&e));
            assert!(html.contains("width=\"600\""));
            assert_eq!(html.contains(">Voir</a>"), gabarit == "annonce");
            assert!(!html.contains('\u{2014}'), "tiret cadratin dans {gabarit}");
            let integrees = images_integrees(gabarit, Some(&e));
            assert_eq!(
                html.contains(&format!("cid:{CID_LOGO}")),
                !integrees.is_empty(),
                "{gabarit}"
            );
        }
        assert!(!rendre("sobre", Some(&e)).contains("cid:"));
    }

    #[test]
    fn lentete_porte_le_nom_sur_deux_lignes() {
        let html = rendre("institutionnel", None);
        assert!(html.contains("<span style=\"font-weight:300\">e</span><span style=\"font-weight:bold\">Pavillon</span>"));
        assert!(html.contains("text-transform:uppercase;color:#bac1ea\">de la Francophonie"));
        assert!(!html.contains("epavillon@4x.png"));
        assert_eq!(images_integrees("institutionnel", None).len(), 1);
    }

    #[test]
    fn la_banniere_de_ledition_et_ses_replis() {
        let e = edition(true, true);
        let html = rendre("banniere_edition", Some(&e));
        assert!(html.contains("src=\"https://site.org/b.jpeg\" alt=\"CdP31 &lt;Antalya&gt;\" width=\"600\" height=\"337\""), "{html}");
        assert!(html.contains("<a href=\"https://site.org/evenements/cdp31\""));
        assert!(html.contains("width=\"96\" height=\"48\""), "{html}");
        assert!(html.contains("Du 9 au 20 novembre 2026, Antalya"));
        assert!(images_integrees("banniere_edition", Some(&e)).is_empty());

        // Sans logo : le nom seul, à gauche.
        let sans_logo = rendre("banniere_edition", Some(&edition(true, false)));
        assert!(!sans_logo.contains("l.png"));
        assert!(sans_logo.contains("text-align:left"));

        // Sans bannière : l'institutionnel.
        let repli = rendre("banniere_edition", Some(&edition(false, true)));
        assert_eq!(repli, rendre("institutionnel", None));
    }

    #[test]
    fn le_logo_de_ledition_sur_sa_tuile() {
        let e = edition(false, true);
        let html = rendre("institutionnel_edition", Some(&e));
        assert!(html.contains("background:#ffffff;border-radius:6px"));
        assert!(html.contains("src=\"https://site.org/l.png\" alt=\"CdP31 &lt;Antalya&gt;\" width=\"80\" height=\"40\""), "{html}");
        assert_eq!(
            rendre("institutionnel_edition", Some(&edition(true, false))),
            rendre("institutionnel", None)
        );
    }

    #[test]
    fn lapercu_cite_le_symbole_en_donnees() {
        let html = pour_apercu(&rendre("institutionnel", None));
        assert!(!html.contains("cid:"));
        assert!(html.contains("src=\"data:image/png;base64,iVBORw0KGgo"));
    }

    #[test]
    fn limage_choisie_est_un_jpeg_ou_un_png_a_la_bonne_taille() {
        let image = serde_json::json!({
            "url": "https://s/o/photo.webp", "width": 3315, "height": 1860,
            "sources": {
                "lg_jpeg": { "url": "https://s/lg.jpeg", "width": 1600, "height": 898 },
                "md_jpeg": { "url": "https://s/md.jpeg", "width": 800, "height": 449 },
                "thumb_jpeg": { "url": "https://s/thumb.jpeg", "width": 320, "height": 180 },
                "lg_webp": { "url": "https://s/lg.webp", "width": 1200, "height": 673 }
            }
        });
        assert_eq!(
            choisir_image(&image, 1200, 0).unwrap().src,
            "https://s/lg.jpeg"
        );
        assert_eq!(
            choisir_image(&image, 0, 120).unwrap().src,
            "https://s/thumb.jpeg"
        );
        // Un petit logo sans variante : l'original, s'il est un PNG.
        let petit = serde_json::json!({ "url": "https://s/logo.PNG", "width": 200, "height": 100, "sources": {} });
        assert_eq!(choisir_image(&petit, 0, 120).unwrap().largeur, 200);
        let webp = serde_json::json!({ "url": "https://s/logo.webp", "width": 200, "height": 100, "sources": {} });
        assert!(choisir_image(&webp, 0, 120).is_none());
    }

    #[test]
    fn la_periode_en_toutes_lettres() {
        use time::macros::date;
        assert_eq!(
            periode(date!(2026 - 11 - 09), date!(2026 - 11 - 20)),
            "Du 9 au 20 novembre 2026"
        );
        assert_eq!(
            periode(date!(2025 - 10 - 30), date!(2025 - 11 - 01)),
            "Du 30 octobre au 1er novembre 2025"
        );
        assert_eq!(
            periode(date!(2026 - 12 - 30), date!(2027 - 01 - 02)),
            "Du 30 décembre 2026 au 2 janvier 2027"
        );
        assert_eq!(
            periode(date!(2026 - 08 - 01), date!(2026 - 08 - 01)),
            "Le 1er août 2026"
        );
    }
}
