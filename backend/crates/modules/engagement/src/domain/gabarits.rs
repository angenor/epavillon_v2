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
/// Le voile du texte posé sur une image, comme `.scrim-fade-bottom` du site
/// (`--color-scrim`, 92 % en bas, 62 % à mi-hauteur, transparent en haut).
const VOILE: &str =
    "linear-gradient(to top,rgba(35,31,32,0.92) 0%,rgba(35,31,32,0.62) 45%,rgba(35,31,32,0) 100%)";

/// Les mises en page : les modèles par défaut, et la base d'un gabarit
/// personnalisé (`engagement.mail_templates.layout`).
pub const GABARITS: [&str; 7] = [
    "sobre",
    "institutionnel",
    "annonce",
    "banniere",
    "texte_sur_image",
    "institutionnel_logo",
    "edition",
];

/// Le symbole inversé de l'ePavillon (« e » blanc), 84 x 80 px affiché en
/// 42 x 40. **Intégré au message**, jamais par adresse : Outlook bloque les
/// images distantes jusqu'au clic, pas les images intégrées.
const LOGO_EPAVILLON: &[u8] = include_bytes!("../../assets/logo-epavillon-symbole-inverse.png");
pub const CID_LOGO: &str = "logo-epavillon";

/// Une image prête à citer : adresse publique (ou `cid:`) et dimensions du
/// fichier servi, d'où se déduisent les attributs `width` et `height` qu'Outlook
/// exige.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageEdition {
    pub src: String,
    pub largeur: u32,
    pub hauteur: u32,
}

/// Ce que les mises en page de l'édition montrent : textes, lien et images de
/// l'édition de l'envoi, remplacés par ceux d'un gabarit personnalisé.
#[derive(Debug, Clone)]
pub struct Visuels {
    /// Le nom de l'édition, ou le titre du gabarit.
    pub titre: String,
    /// « Du 9 au 20 novembre 2026, Antalya », ou le sous-titre du gabarit.
    pub sous_titre: Option<String>,
    /// La page publique de l'édition.
    pub lien: String,
    /// L'image de tête, au rapport 6:1 (affichée 600 x 100).
    pub fond: Option<ImageEdition>,
    pub logo: Option<ImageEdition>,
    /// Le voile sombre sous le texte posé sur l'image.
    pub degrade: bool,
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
    pub visuels: Option<&'a Visuels>,
}

/// L'en-tête que le gabarit prend vraiment : sans l'image qu'elle attend, une
/// mise en page de l'édition retombe sur l'institutionnel.
enum Tete<'a> {
    Aucune,
    Institutionnelle(Option<(&'a Visuels, &'a ImageEdition)>),
    Banniere(&'a Visuels, &'a ImageEdition),
    TexteSurImage(&'a Visuels, &'a ImageEdition),
    Edition(&'a Visuels),
}

fn tete<'a>(gabarit: &str, visuels: Option<&'a Visuels>) -> Tete<'a> {
    let fond = visuels.and_then(|v| v.fond.as_ref().map(|f| (v, f)));
    let logo = visuels.and_then(|v| v.logo.as_ref().map(|l| (v, l)));
    match gabarit {
        "sobre" => Tete::Aucune,
        "banniere" => match fond {
            Some((v, f)) => Tete::Banniere(v, f),
            None => Tete::Institutionnelle(None),
        },
        "texte_sur_image" => match fond {
            Some((v, f)) => Tete::TexteSurImage(v, f),
            None => Tete::Institutionnelle(None),
        },
        "institutionnel_logo" => Tete::Institutionnelle(logo),
        "edition" => match visuels {
            Some(v) => Tete::Edition(v),
            None => Tete::Institutionnelle(None),
        },
        _ => Tete::Institutionnelle(None),
    }
}

/// Les images à intégrer au message pour ce gabarit : le symbole de l'ePavillon
/// quand l'en-tête bleu nuit est dessiné.
pub fn images_integrees(gabarit: &str, visuels: Option<&Visuels>) -> Vec<InlineImage> {
    match tete(gabarit, visuels) {
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

    let mut lignes = match tete(m.gabarit, m.visuels) {
        Tete::Aucune => String::new(),
        Tete::Institutionnelle(logo) => bandeau(logo) + &lisere(),
        Tete::Banniere(v, image) => banniere(v, image) + &lisere(),
        Tete::TexteSurImage(v, image) => texte_sur_image(v, image) + &lisere(),
        Tete::Edition(v) => bande_de_ledition(v) + &lisere(),
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
        "<!doctype html><html lang=\"fr\" xmlns:v=\"urn:schemas-microsoft-com:vml\" xmlns:o=\"urn:schemas-microsoft-com:office:office\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"></head>\
<body style=\"margin:0;padding:0;background:#f4f5f7\">\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"background:#f4f5f7\"><tr><td align=\"center\" style=\"padding:24px 12px\">\
<table role=\"presentation\" width=\"600\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"width:600px;max-width:100%;background:#ffffff;border:1px solid #e3e5ea\">{lignes}</table>\
</td></tr></table></body></html>"
    )
}

/// L'en-tête bleu nuit, 72 px de haut : le symbole, le nom sur deux lignes
/// comme la barre du site, et, pour `institutionnel_logo`, le logo de l'édition
/// à l'extrême droite sur une tuile blanche (un logo foncé disparaîtrait sur le
/// bleu nuit). La tuile (56 px de logo) prend la place des marges intérieures :
/// l'en-tête garde sa hauteur.
fn bandeau(logo: Option<(&Visuels, &ImageEdition)>) -> String {
    let marge = if logo.is_some() {
        "4px 32px"
    } else {
        "16px 32px"
    };
    let tuile = logo
        .map(|(v, image)| {
            let (l, h) = ajuster(image, 56, 180);
            format!(
                "<td align=\"right\" style=\"vertical-align:middle;padding-left:16px\">\
<table role=\"presentation\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" align=\"right\"><tr>\
<td style=\"background:#ffffff;border-radius:6px;padding:4px 8px\">\
<img src=\"{}\" alt=\"{}\" width=\"{l}\" height=\"{h}\" style=\"display:block;border:0;width:{l}px;height:{h}px\"></td>\
</tr></table></td>",
                echapper(&image.src),
                echapper(&v.titre)
            )
        })
        .unwrap_or_default();
    format!(
        "<tr><td style=\"background:{BLEU_NUIT};padding:{marge}\">\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>\
<td width=\"42\" style=\"width:42px;padding-right:12px;vertical-align:middle\"><img src=\"cid:{CID_LOGO}\" alt=\"\" width=\"42\" height=\"40\" style=\"display:block;border:0;width:42px;height:40px\"></td>\
<td style=\"vertical-align:middle;font-family:{POLICE}\">\
<p style=\"margin:0;font-size:20px;line-height:22px;mso-line-height-rule:exactly;letter-spacing:-0.01em;color:#ffffff\"><span style=\"font-weight:300\">e</span><span style=\"font-weight:bold\">Pavillon</span></p>\
<p style=\"margin:5px 0 0;font-size:11px;line-height:13px;mso-line-height-rule:exactly;letter-spacing:0.16em;text-transform:uppercase;color:{BLEU_PALE}\">de la Francophonie</p>\
</td>{tuile}</tr></table></td></tr>"
    )
}

/// Hauteur affichée d'une image de tête 6:1 dans 600 px.
fn hauteur_de_tete(image: &ImageEdition) -> u64 {
    (600 * u64::from(image.hauteur) / u64::from(image.largeur.max(1))).max(1)
}

/// L'image de tête sur toute la largeur (600 x 100), cliquable vers la page de
/// l'édition, puis la bande de l'édition.
fn banniere(v: &Visuels, image: &ImageEdition) -> String {
    let hauteur = hauteur_de_tete(image);
    let lien = echapper(&v.lien);
    let titre = echapper(&v.titre);
    format!(
        "<tr><td style=\"padding:0;font-size:0;line-height:0\"><a href=\"{lien}\" style=\"display:block\">\
<img src=\"{}\" alt=\"{titre}\" width=\"600\" height=\"{hauteur}\" style=\"display:block;border:0;width:100%;max-width:600px;height:auto\"></a></td></tr>{}",
        echapper(&image.src),
        bande_de_ledition(v)
    )
}

/// Une bande blanche : le logo de l'édition à gauche, titre et sous-titre à
/// droite (le titre seul, à gauche, sans logo). Sous l'image de « banniere »,
/// ou seule pour « edition ».
fn bande_de_ledition(v: &Visuels) -> String {
    let titre = echapper(&v.titre);
    let logo = v
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
    let alignement = if v.logo.is_some() { "right" } else { "left" };
    let sous_titre = v
        .sous_titre
        .as_deref()
        .map(|d| {
            format!(
                "<p style=\"margin:4px 0 0;font-size:14px;line-height:18px;mso-line-height-rule:exactly;color:{BLEU_NUIT}\">{}</p>",
                echapper(d)
            )
        })
        .unwrap_or_default();
    format!(
        "<tr><td style=\"background:#ffffff;padding:16px 32px\">\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>{logo}\
<td align=\"{alignement}\" style=\"vertical-align:middle;font-family:{POLICE};text-align:{alignement}\">\
<p style=\"margin:0;font-size:17px;line-height:22px;mso-line-height-rule:exactly;font-weight:bold;color:{BLEU_NUIT}\">{titre}</p>{sous_titre}\
</td></tr></table></td></tr>"
    )
}

/// Le titre et le sous-titre en blanc sur l'image, sur un voile sombre du bas
/// vers le haut. Image de fond en CSS, repli VML pour Outlook bureau (qui
/// ignore le dégradé) ; sans prise en charge des images de fond, un aplat bleu
/// nuit porte le texte.
fn texte_sur_image(v: &Visuels, image: &ImageEdition) -> String {
    let hauteur = hauteur_de_tete(image);
    let src = echapper(&image.src);
    let lien = echapper(&v.lien);
    let voile = if v.degrade {
        format!("background-image:{VOILE};")
    } else {
        String::new()
    };
    let ombre = "text-shadow:0 1px 2px rgba(0,0,0,0.6);";
    let sous_titre = v
        .sous_titre
        .as_deref()
        .map(|d| {
            format!(
                "<p style=\"margin:3px 0 0;font-family:{POLICE};font-size:13px;line-height:16px;mso-line-height-rule:exactly;color:#ffffff;{ombre}\">{}</p>",
                echapper(d)
            )
        })
        .unwrap_or_default();
    // 600 x hauteur px = 450 x (hauteur x 0,75) pt pour le rectangle VML.
    let hauteur_pt = hauteur * 3 / 4;
    format!(
        "<tr><td background=\"{src}\" bgcolor=\"{BLEU_NUIT}\" width=\"600\" height=\"{hauteur}\" valign=\"bottom\" \
style=\"padding:0;height:{hauteur}px;background-color:{BLEU_NUIT};background-image:url('{src}');background-size:cover;background-position:center;background-repeat:no-repeat\">\
<!--[if gte mso 9]><v:rect xmlns:v=\"urn:schemas-microsoft-com:vml\" fill=\"true\" stroke=\"false\" style=\"width:450pt;height:{hauteur_pt}pt;\">\
<v:fill type=\"frame\" src=\"{src}\" color=\"{BLEU_NUIT}\" /><v:textbox inset=\"0,0,0,0\"><![endif]-->\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>\
<td height=\"{hauteur}\" valign=\"bottom\" style=\"height:{hauteur}px;vertical-align:bottom;padding:0 32px 12px;{voile}\">\
<p style=\"margin:0;font-family:{POLICE};font-size:20px;line-height:24px;mso-line-height-rule:exactly;font-weight:bold;color:#ffffff;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;{ombre}\">\
<a href=\"{lien}\" style=\"color:#ffffff;text-decoration:none\">{}</a></p>{sous_titre}\
</td></tr></table>\
<!--[if gte mso 9]></v:textbox></v:rect><![endif]--></td></tr>",
        echapper(&v.titre)
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

/// Les fichiers JPEG ou PNG d'une image telle que `media.attached_image()` la
/// rend : ses variantes, et l'original s'il est l'un des deux. Outlook ne lit
/// pas le WebP.
fn candidats(image: &serde_json::Value, recadrees: bool) -> Vec<ImageEdition> {
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
    let mut liste: Vec<ImageEdition> = image
        .get("sources")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(cle, _)| cle.ends_with("_jpeg") || cle.ends_with("_png"))
        // La variante `courriel` est recadrée : elle ne sert qu'à l'image de tête.
        .filter(|(cle, _)| recadrees || !cle.starts_with("courriel_"))
        .filter_map(|(_, v)| candidat(v))
        .collect();
    if let Some(original) = candidat(image) {
        let chemin = original.src.to_ascii_lowercase();
        let chemin = chemin.split(['?', '#']).next().unwrap_or_default();
        if [".jpg", ".jpeg", ".png"]
            .iter()
            .any(|e| chemin.ends_with(e))
        {
            liste.push(original);
        }
    }
    liste.sort_by_key(|c| u64::from(c.largeur) * u64::from(c.hauteur));
    liste
}

/// La plus petite qui atteint la taille demandée, sinon la plus grande.
fn la_plus_juste(
    mut liste: Vec<ImageEdition>,
    largeur_min: u32,
    hauteur_min: u32,
) -> Option<ImageEdition> {
    match liste
        .iter()
        .position(|c| c.largeur >= largeur_min && c.hauteur >= hauteur_min)
    {
        Some(i) => Some(liste.swap_remove(i)),
        None => liste.pop(),
    }
}

/// Une image quelconque (le logo) : ni flou, ni original de plusieurs Mo.
pub fn choisir_image(
    image: &serde_json::Value,
    largeur_min: u32,
    hauteur_min: u32,
) -> Option<ImageEdition> {
    la_plus_juste(candidats(image, false), largeur_min, hauteur_min)
}

/// L'image de tête : un fichier au rapport 6:1 (à 5 % près), la variante
/// `courriel` du module media (1200 x 200, JPEG compressé) ou un fond déposé
/// déjà recadré. Un bandeau d'une autre forme n'en a pas : la mise en page
/// retombe alors sur l'institutionnel plutôt que d'étirer une image haute.
pub fn choisir_fond(image: &serde_json::Value) -> Option<ImageEdition> {
    let au_rapport: Vec<ImageEdition> = candidats(image, true)
        .into_iter()
        .filter(|c| {
            let r = f64::from(c.largeur) / f64::from(c.hauteur);
            (r - 6.0).abs() <= 0.3
        })
        .collect();
    la_plus_juste(au_rapport, 1200, 0)
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

    fn visuels(fond: bool, logo: bool) -> Visuels {
        Visuels {
            titre: "CdP31 <Antalya>".into(),
            sous_titre: Some("Du 9 au 20 novembre 2026, Antalya".into()),
            lien: "https://site.org/evenements/cdp31".into(),
            fond: fond.then(|| ImageEdition {
                src: "https://site.org/courriel.jpeg".into(),
                largeur: 1200,
                hauteur: 200,
            }),
            logo: logo.then(|| ImageEdition {
                src: "https://site.org/l.png".into(),
                largeur: 320,
                hauteur: 160,
            }),
            degrade: true,
        }
    }

    fn rendre(gabarit: &str, v: Option<&Visuels>) -> String {
        mettre_en_page(
            "<p>Bonjour</p>",
            &MiseEnPage {
                gabarit,
                sujet: "Objet",
                action: Some(("Voir", "https://exemple.org")),
                repondre: None,
                visuels: v,
            },
        )
    }

    #[test]
    fn les_gabarits_tiennent_dans_600_pixels() {
        let v = visuels(true, true);
        for gabarit in GABARITS {
            let html = rendre(gabarit, Some(&v));
            assert!(html.contains("width=\"600\""));
            assert_eq!(html.contains(">Voir</a>"), gabarit == "annonce");
            assert!(!html.contains('\u{2014}'), "tiret cadratin dans {gabarit}");
            let integrees = images_integrees(gabarit, Some(&v));
            assert_eq!(
                html.contains(&format!("cid:{CID_LOGO}")),
                !integrees.is_empty(),
                "{gabarit}"
            );
        }
        assert!(!rendre("sobre", Some(&v)).contains("cid:"));
    }

    #[test]
    fn lentete_porte_le_nom_sur_deux_lignes() {
        let html = rendre("institutionnel", None);
        assert!(html.contains("<span style=\"font-weight:300\">e</span><span style=\"font-weight:bold\">Pavillon</span>"));
        assert!(html.contains("text-transform:uppercase;color:#bac1ea\">de la Francophonie"));
        assert!(html.contains("padding:16px 32px"));
        assert_eq!(images_integrees("institutionnel", None).len(), 1);
    }

    #[test]
    fn la_banniere_basse_et_ses_replis() {
        let v = visuels(true, true);
        let html = rendre("banniere", Some(&v));
        assert!(html.contains("src=\"https://site.org/courriel.jpeg\" alt=\"CdP31 &lt;Antalya&gt;\" width=\"600\" height=\"100\""), "{html}");
        assert!(html.contains("<a href=\"https://site.org/evenements/cdp31\""));
        assert!(html.contains("width=\"96\" height=\"48\""), "{html}");
        assert!(html.contains("Du 9 au 20 novembre 2026, Antalya"));
        assert!(images_integrees("banniere", Some(&v)).is_empty());

        let sans_logo = rendre("banniere", Some(&visuels(true, false)));
        assert!(!sans_logo.contains("l.png"));
        assert!(sans_logo.contains("text-align:left"));

        let repli = rendre("banniere", Some(&visuels(false, true)));
        assert_eq!(repli, rendre("institutionnel", None));
    }

    #[test]
    fn le_texte_sur_limage_a_son_voile_et_son_repli_outlook() {
        let v = visuels(true, false);
        let html = rendre("texte_sur_image", Some(&v));
        assert!(
            html.contains("background-image:url('https://site.org/courriel.jpeg')"),
            "{html}"
        );
        assert!(html.contains("bgcolor=\"#1d1a5b\""));
        assert!(html.contains("<v:fill type=\"frame\" src=\"https://site.org/courriel.jpeg\""));
        assert!(html.contains("height:75pt"));
        assert!(html.contains("font-size:20px;line-height:24px"));
        assert!(html.contains("font-size:13px;line-height:16px"));
        assert!(html.contains("linear-gradient(to top"));
        assert!(html.contains("color:#ffffff;text-decoration:none\">CdP31 &lt;Antalya&gt;</a>"));
        let sans_voile = rendre(
            "texte_sur_image",
            Some(&Visuels {
                degrade: false,
                ..visuels(true, false)
            }),
        );
        assert!(!sans_voile.contains("linear-gradient"));
        assert_eq!(
            rendre("texte_sur_image", Some(&visuels(false, true))),
            rendre("institutionnel", None)
        );
    }

    #[test]
    fn ledition_sans_image_garde_la_bande() {
        let v = visuels(true, true);
        let html = rendre("edition", Some(&v));
        assert!(!html.contains("courriel.jpeg"));
        assert!(html.contains("width=\"96\" height=\"48\""), "{html}");
        assert!(html.contains("font-weight:bold;color:#1d1a5b\">CdP31 &lt;Antalya&gt;</p>"));
        assert!(html.contains("Du 9 au 20 novembre 2026, Antalya"));
        assert!(images_integrees("edition", Some(&v)).is_empty());
        let sans_logo = rendre("edition", Some(&visuels(false, false)));
        assert!(sans_logo.contains("text-align:left"));
        assert_eq!(rendre("edition", None), rendre("institutionnel", None));
    }

    #[test]
    fn le_logo_agrandi_ne_grandit_pas_lentete() {
        let v = visuels(false, true);
        let html = rendre("institutionnel_logo", Some(&v));
        assert!(html.contains("background:#ffffff;border-radius:6px;padding:4px 8px"));
        assert!(html.contains("width=\"112\" height=\"56\""), "{html}");
        // 4 + 56 + 2 x 4 + 4 = 72, comme 16 + 40 + 16.
        assert!(html.contains("padding:4px 32px"));
        assert_eq!(
            rendre("institutionnel_logo", Some(&visuels(true, false))),
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
    fn le_fond_est_un_6_1_leger_jamais_un_webp() {
        let image = serde_json::json!({
            "url": "https://s/o/photo.jpg", "width": 3315, "height": 1860,
            "sources": {
                "lg_jpeg": { "url": "https://s/lg.jpeg", "width": 1600, "height": 898 },
                "md_jpeg": { "url": "https://s/md.jpeg", "width": 800, "height": 449 },
                "courriel_jpeg": { "url": "https://s/courriel.jpeg", "width": 1200, "height": 200 },
                "lg_webp": { "url": "https://s/lg.webp", "width": 1200, "height": 200 }
            }
        });
        assert_eq!(choisir_fond(&image).unwrap().src, "https://s/courriel.jpeg");
        assert_eq!(
            choisir_image(&image, 0, 120).unwrap().src,
            "https://s/md.jpeg"
        );
        // Sans variante 6:1 : pas d'image de tête.
        let haute = serde_json::json!({ "url": "https://s/b.jpg", "width": 1600, "height": 900, "sources": {} });
        assert!(choisir_fond(&haute).is_none());
        // Un fond déposé déjà recadré : l'original.
        let depose = serde_json::json!({ "url": "https://s/fond.jpg", "width": 1200, "height": 200, "sources": {} });
        assert_eq!(choisir_fond(&depose).unwrap().largeur, 1200);
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
