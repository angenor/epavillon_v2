//! **Courriels d'essai des gabarits, écrits en `.eml`, jamais envoyés** (29/10).
//!
//! Rendus par le vrai code (`composer` du travail d'envoi, `kernel::mail::composer`
//! pour le MIME), sur la CdP31. Les images de l'édition, pas encore en ligne,
//! sont intégrées au message pour l'essai seulement ; en production, elles
//! partent par leur adresse publique.
//!
//! ```text
//! ESSAI_SORTIE=<dossier> \
//! ESSAI_BANNIERE=<banniere.jpeg>:1600x897 ESSAI_LOGO=<logo.jpeg>:320x397 \
//! cargo test -p engagement --test gabarits_essai -- --ignored
//! ```

use engagement::domain::gabarits::{self, images_integrees, Edition, ImageEdition};
use engagement::domain::mailing;
use engagement::jobs::send_mailing::composer;
use engagement::repo::mailings::{Habillage, LigneEnvoi};
use kernel::mail::{Extras, InlineImage};
use std::path::PathBuf;
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

fn envoi(gabarit: &str, nom: &str) -> LigneEnvoi {
    let maintenant = time::OffsetDateTime::now_utc();
    LigneEnvoi {
        id: Uuid::nil(),
        event_id: Uuid::nil(),
        status: "draft".into(),
        subject: format!("[Essai] Gabarit « {nom} »"),
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
            template: gabarit.into(),
            variable_fallbacks: serde_json::json!({ "prenom": "Madame, Monsieur" }),
            reply_button: false,
            cta_label: None,
            cta_url: None,
        },
    }
}

#[test]
#[ignore = "écrit des fichiers d'essai ; lancé à la main"]
fn ecrire_les_courriels_dessai() {
    let sortie = PathBuf::from(std::env::var("ESSAI_SORTIE").expect("ESSAI_SORTIE manquante"));
    std::fs::create_dir_all(&sortie).expect("dossier de sortie");
    let (banniere, banniere_integree) = image("ESSAI_BANNIERE", "banniere-cdp31");
    let (logo, logo_integre) = image("ESSAI_LOGO", "logo-cdp31");
    let edition = Edition {
        nom: "31e conférence des Nations Unies sur le climat".into(),
        dates: Some(format!(
            "{}, Antalya",
            gabarits::periode(date!(2026 - 11 - 09), date!(2026 - 11 - 20))
        )),
        lien: "https://epavillonclimatique.francophonie.org/evenements/cdp31-antalya-2026".into(),
        banniere: Some(banniere),
        logo: Some(logo),
    };
    let valeurs = mailing::valeurs(
        &serde_json::json!({ "prenom": "Angenor" }),
        &serde_json::json!({}),
    );

    for (gabarit, nom, fichier) in [
        (
            "banniere_edition",
            "Bannière de l'édition",
            "1-banniere-de-ledition.eml",
        ),
        (
            "institutionnel_edition",
            "Institutionnel + logo de l'édition",
            "2-institutionnel-logo-de-ledition.eml",
        ),
        ("institutionnel", "Institutionnel", "3-institutionnel.eml"),
    ] {
        let e = envoi(gabarit, nom);
        let message = composer(
            format!("essai-{gabarit}"),
            "angenor99@gmail.com",
            &e,
            &valeurs,
            Some(&edition),
        )
        .expect("message rendu");
        let html = message.html.as_deref().unwrap_or_default();
        let mut images = images_integrees(gabarit, Some(&edition));
        for integree in [&banniere_integree, &logo_integre] {
            if html.contains(&format!("cid:{}", integree.content_id)) {
                images.push(integree.clone());
            }
        }
        let extras = Extras {
            inline_images: images,
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
}
