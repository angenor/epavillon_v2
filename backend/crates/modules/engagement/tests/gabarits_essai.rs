//! **Courriels d'essai des gabarits, écrits en `.eml`, jamais envoyés** (29/10).
//!
//! Rendus par le vrai code (`composer` du travail d'envoi, `kernel::mail::composer`
//! pour le MIME), sur la CdP31. Les images, pas encore en ligne, sont intégrées
//! au message pour l'essai seulement ; en production, elles partent par leur
//! adresse publique.
//!
//! ```text
//! ESSAI_SORTIE=<dossier> \
//! ESSAI_FOND=<bannière 3:1>.jpeg:1200x400 ESSAI_LOGO=<logo>.jpeg:320x397 \
//! ESSAI_FOND_PERSO=<fond déposé>.jpeg:1200x400 ESSAI_LOGO_PERSO=<logo déposé>.png:LxH \
//! cargo test -p engagement --test gabarits_essai -- --ignored
//! ```

use engagement::domain::gabarits::{self, images_integrees, ImageEdition, Visuels};
use engagement::domain::mailing;
use engagement::jobs::send_mailing::{composer, Apparence};
use engagement::repo::mailings::{Habillage, LigneEnvoi};
use kernel::mail::{Extras, InlineImage};
use std::path::{Path, PathBuf};
use time::macros::date;
use uuid::Uuid;

const CORPS: &str = "<p>Bonjour {{prenom}},</p>\
<p>Le Pavillon de la Francophonie ouvrira ses portes à Antalya pendant la CdP31. \
Vous trouverez le programme des activités sur la page de l'édition.</p>\
<p>Ceci est un message d'essai : il montre la mise en page, rien n'est à faire.</p>";

/// `chemin:LARGEURxHAUTEUR` : l'image et les dimensions du fichier servi.
fn image(variable: &str, cid: &str) -> (ImageEdition, InlineImage) {
    let valeur = std::env::var(variable).unwrap_or_else(|_| panic!("{variable} manquante"));
    let (chemin, taille) = valeur.rsplit_once(':').expect("chemin:LxH");
    let (l, h) = taille.split_once('x').expect("LxH");
    let mime = if chemin.ends_with(".png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    (
        ImageEdition {
            src: format!("cid:{cid}"),
            largeur: l.parse().expect("largeur"),
            hauteur: h.parse().expect("hauteur"),
        },
        InlineImage {
            content_id: cid.to_owned(),
            mime_type: mime.to_owned(),
            content: std::fs::read(chemin).expect("image lisible"),
        },
    )
}

fn envoi(sujet: &str) -> LigneEnvoi {
    let maintenant = time::OffsetDateTime::now_utc();
    LigneEnvoi {
        id: Uuid::nil(),
        event_id: Uuid::nil(),
        status: "draft".into(),
        subject: format!("[Essai] Gabarit « {sujet} »"),
        body_html: CORPS.into(),
        audience: vec![],
        extra_addresses: vec![],
        cc: vec![],
        bcc: vec![],
        reply_to: None,
        duplicated_from: None,
        created_by_name: None,
        queued_by_name: None,
        queued_at: None,
        completed_at: None,
        cancelled_at: None,
        created_at: maintenant,
        updated_at: maintenant,
        habillage: Habillage {
            template: "institutionnel".into(),
            variable_fallbacks: serde_json::json!({ "prenom": "Madame, Monsieur" }),
            reply_button: false,
            cta_label: None,
            cta_url: None,
            mail_template_id: None,
        },
    }
}

fn ecrire(
    sortie: &Path,
    fichier: &str,
    sujet: &str,
    apparence: &Apparence,
    images: &[InlineImage],
) {
    let valeurs = mailing::valeurs(
        &serde_json::json!({ "prenom": "Angenor" }),
        &serde_json::json!({}),
    );
    let message = composer(
        format!("essai-{fichier}"),
        "angenor99@gmail.com",
        &envoi(sujet),
        &valeurs,
        apparence,
    )
    .expect("message rendu");
    let html = message.html.as_deref().unwrap_or_default();
    let mut integrees = images_integrees(&apparence.gabarit, apparence.visuels.as_ref());
    for image in images {
        if html.contains(&format!("cid:{}", image.content_id)) {
            integrees.push(image.clone());
        }
    }
    let extras = Extras {
        inline_images: integrees,
        ..Extras::default()
    };
    let brut = kernel::mail::composer(
        "ePavillon <epavillonclimatique@francophonie.org>"
            .parse()
            .expect("expéditeur"),
        &message,
        &extras,
    )
    .expect("MIME composé")
    .formatted();
    std::fs::write(sortie.join(fichier), brut).expect("fichier écrit");
    // Le HTML seul, pour le regarder au navigateur (images intégrées absentes).
    let apercus = sortie.join("apercus");
    std::fs::create_dir_all(&apercus).expect("dossier des aperçus");
    std::fs::write(apercus.join(fichier.replace(".eml", ".html")), html).expect("HTML écrit");
}

#[test]
#[ignore = "écrit des fichiers d'essai ; lancé à la main"]
fn ecrire_les_courriels_dessai() {
    let sortie = PathBuf::from(std::env::var("ESSAI_SORTIE").expect("ESSAI_SORTIE manquante"));
    std::fs::create_dir_all(&sortie).expect("dossier de sortie");
    let (fond, fond_integre) = image("ESSAI_FOND", "fond-cdp31");
    let (logo, logo_integre) = image("ESSAI_LOGO", "logo-cdp31");
    let (fond_perso, fond_perso_integre) = image("ESSAI_FOND_PERSO", "fond-gabarit");
    let (logo_perso, logo_perso_integre) = image("ESSAI_LOGO_PERSO", "logo-gabarit");
    let images = [
        fond_integre,
        logo_integre,
        fond_perso_integre,
        logo_perso_integre,
    ];

    // Ce que `apparence()` construit pour la CdP31 : nom, période et ville, page
    // publique, variante `courriel` du bandeau et logo.
    let cdp31 = Visuels {
        titre: "31e conférence des Nations Unies sur le climat".into(),
        sous_titre: Some(format!(
            "{}, Antalya",
            gabarits::periode(date!(2026 - 11 - 09), date!(2026 - 11 - 20))
        )),
        lien: "https://epavillonclimatique.francophonie.org/evenements/cdp31-antalya-2026".into(),
        fond: Some(fond),
        logo: Some(logo),
        degrade: true,
    };
    // Un gabarit personnalisé rattaché à la CdP31 : ses textes et ses images
    // remplacent ceux de l'édition, sur la mise en page « texte sur l'image ».
    let perso = Visuels {
        titre: "Pavillon de la Francophonie à la CdP31".into(),
        sous_titre: Some("Appel à propositions : ce qu'il faut savoir".into()),
        fond: Some(fond_perso),
        logo: Some(logo_perso),
        ..cdp31.clone()
    };

    for (fichier, sujet, gabarit, visuels) in [
        (
            "1-banniere-basse.eml",
            "Bannière de l'édition",
            "banniere",
            &cdp31,
        ),
        (
            "2-texte-sur-limage.eml",
            "Texte sur l'image",
            "texte_sur_image",
            &cdp31,
        ),
        (
            "3-institutionnel-logo-de-ledition.eml",
            "Institutionnel + logo de l'édition",
            "institutionnel_logo",
            &cdp31,
        ),
        (
            "4-gabarit-personnalise-cdp31.eml",
            "Gabarit personnalisé CdP31",
            "texte_sur_image",
            &perso,
        ),
    ] {
        let apparence = Apparence {
            gabarit: gabarit.into(),
            visuels: Some(visuels.clone()),
        };
        ecrire(&sortie, fichier, sujet, &apparence, &images);
    }
}
