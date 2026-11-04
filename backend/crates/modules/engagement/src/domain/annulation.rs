//! Le courriel à l'équipe quand une organisation annule son activité retenue
//! (04/11/2026), pur.
//!
//! Il part aux adresses du réglage `engagement.mail.automatic_cc` (la première
//! en destinataire, les autres en copie, le relais n'en admettant qu'une), la
//! personne qui annule en copie visible : une réponse à tous lui parvient.
//! Gabarit institutionnel des courriels du direct, bouton vers le dossier dans
//! le back-office, aucun lien de désinscription. Une annulation par l'équipe
//! n'en déclenche aucun.

use kernel::mail::OutgoingMail;
use uuid::Uuid;

use super::courriel_direct::{assembler, Assemblage, Paragraphe};
use super::dossier::{date_longue, heure_de, heures, SeanceRetenue};
use super::gabarits::Visuels;
use super::mailing::echapper_html;

/// Le créneau de l'activité annulée, tel qu'on peut encore le lire.
#[derive(Debug, Clone)]
pub enum Creneau {
    /// Sa première séance (publiée, donc annulée maintenant).
    Seance(SeanceRetenue),
    /// Aucune séance publiée : le créneau souhaité du dossier.
    Souhaite(SeanceRetenue),
    Aucun,
}

#[derive(Debug, Clone)]
pub struct AnnulationParLorganisation {
    pub proposal_id: Uuid,
    pub event_id: Uuid,
    pub reference_code: String,
    pub titre: String,
    /// « Nom légal (SIGLE) ».
    pub organisation: String,
    pub auteur: Option<String>,
    pub auteur_email: Option<String>,
    /// « 04/11/2026 à 14:32 », dans le fuseau de l'édition.
    pub annulee_le: String,
    /// Le nom qui désigne le fuseau de l'édition (sa ville).
    pub fuseau: String,
    pub motif: Option<String>,
    pub creneau: Creneau,
}

/// Le courriel et ses copies. `None` sans adresse d'équipe : personne à qui
/// écrire.
pub fn courriel(
    a: &AnnulationParLorganisation,
    equipe: &[String],
    site: &str,
    message_id: &str,
    visuels: Option<&Visuels>,
) -> Option<(OutgoingMail, Vec<String>)> {
    let equipe: Vec<&str> = equipe
        .iter()
        .map(|e| e.trim())
        .filter(|e| !e.is_empty())
        .collect();
    let (premier, autres) = equipe.split_first()?;

    let mut copies: Vec<String> = Vec::new();
    let mut ajouter = |adresse: &str| {
        let deja = premier.eq_ignore_ascii_case(adresse)
            || copies.iter().any(|c| c.eq_ignore_ascii_case(adresse));
        if !deja {
            copies.push(adresse.to_owned());
        }
    };
    for adresse in autres {
        ajouter(adresse);
    }
    if let Some(auteur) = a.auteur_email.as_deref().map(str::trim) {
        if !auteur.is_empty() {
            ajouter(auteur);
        }
    }

    let sujet = format!("Activité annulée par l'organisation : {}", a.reference_code);
    let lien = format!(
        "{}/admin/propositions/{}",
        site.trim_end_matches('/'),
        a.proposal_id
    );

    let mut avant = vec![
        Paragraphe::brut(&format!(
            "{} a annulé son activité retenue {}. Les séances publiées sont annulées et \
             leurs inscrits prévenus ; le motif n'est pas affiché sur le site.",
            a.organisation, a.reference_code
        )),
        Paragraphe::brut(&format!("Activité : {}", a.titre)),
    ];
    match &a.creneau {
        Creneau::Seance(s) => {
            avant.push(Paragraphe::brut(&format!(
                "Date : {}, {}",
                date_longue(s.debut.date()),
                heures(s)
            )));
            avant.push(Paragraphe::brut(&format!(
                "Salle : {}",
                s.salle.as_deref().unwrap_or("non attribuée")
            )));
        }
        Creneau::Souhaite(s) => {
            avant.push(Paragraphe::brut(&format!(
                "Date : aucune séance publiée ; créneau souhaité par l'organisation : {}, {}",
                date_longue(s.debut.date()),
                heures(s)
            )));
            avant.push(Paragraphe::brut("Salle : non attribuée"));
        }
        Creneau::Aucun => {
            avant.push(Paragraphe::brut("Date : aucune séance programmée"));
            avant.push(Paragraphe::brut("Salle : non attribuée"));
        }
    }
    avant.push(Paragraphe::brut(&format!(
        "Organisation : {}",
        a.organisation
    )));
    let qui = match (a.auteur.as_deref(), a.auteur_email.as_deref()) {
        (Some(nom), Some(adresse)) => format!("{nom} ({adresse})"),
        (Some(nom), None) => nom.to_owned(),
        (None, Some(adresse)) => adresse.to_owned(),
        (None, None) => "personne inconnue".to_owned(),
    };
    avant.push(Paragraphe::brut(&format!(
        "Annulée par : {qui}, le {}, {}",
        a.annulee_le,
        heure_de(&a.fuseau)
    )));
    let motif = a
        .motif
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or("non renseigné");
    avant.push(Paragraphe {
        html: format!("Motif : {}", echapper_html(motif).replace('\n', "<br>")),
        texte: format!("Motif : {motif}"),
    });

    let mail = assembler(
        &Assemblage {
            anglais: false,
            sujet: &sujet,
            avant: &avant,
            lien: &lien,
            action: "Ouvrir le dossier",
            apres: &[],
        },
        visuels,
        premier,
        "fr",
        message_id,
    );
    Some((mail, copies))
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn annulation(creneau: Creneau) -> AnnulationParLorganisation {
        AnnulationParLorganisation {
            proposal_id: Uuid::nil(),
            event_id: Uuid::nil(),
            reference_code: "CDP31-0042".to_owned(),
            titre: "Finance climat & adaptation".to_owned(),
            organisation: "Réseau climat Afrique (RCA)".to_owned(),
            auteur: Some("Awa Sow".to_owned()),
            auteur_email: Some("awa@rca.org".to_owned()),
            annulee_le: "04/11/2026 à 14:32".to_owned(),
            fuseau: "Antalya".to_owned(),
            motif: Some("Intervenante principale indisponible.\nNous reviendrons.".to_owned()),
            creneau,
        }
    }

    fn seance() -> Creneau {
        Creneau::Seance(SeanceRetenue {
            debut: datetime!(2026-11-12 09:50),
            fin: datetime!(2026-11-12 10:40),
            timezone: "Europe/Istanbul".to_owned(),
            ville: Some("Antalya".to_owned()),
            salle: Some("Pavillon de la Francophonie".to_owned()),
        })
    }

    fn equipe() -> Vec<String> {
        vec![
            "angenor99@gmail.com".to_owned(),
            "issa.bado@francophonie.org".to_owned(),
        ]
    }

    #[test]
    fn l_equipe_recoit_et_l_auteur_est_en_copie() {
        let (m, copies) = courriel(
            &annulation(seance()),
            &equipe(),
            "https://site.org/",
            "j",
            None,
        )
        .expect("un courriel");
        assert_eq!(m.to, "angenor99@gmail.com");
        assert_eq!(copies, vec!["issa.bado@francophonie.org", "awa@rca.org"]);
        assert_eq!(
            m.subject,
            "Activité annulée par l'organisation : CDP31-0042"
        );
    }

    #[test]
    fn le_corps_dit_tout_et_mene_au_back_office() {
        let (m, _) = courriel(
            &annulation(seance()),
            &equipe(),
            "https://site.org/",
            "j",
            None,
        )
        .expect("un courriel");
        let html = m.html.expect("html");
        for attendu in [
            "Activité : Finance climat & adaptation",
            "jeudi 12 novembre 2026, 09:50 à 10:40, heure d'Antalya",
            "Salle : Pavillon de la Francophonie",
            "Organisation : Réseau climat Afrique (RCA)",
            "Annulée par : Awa Sow (awa@rca.org), le 04/11/2026 à 14:32, heure d'Antalya",
            "Motif : Intervenante principale indisponible.\nNous reviendrons.",
            "https://site.org/admin/propositions/00000000-0000-0000-0000-000000000000",
        ] {
            assert!(m.text.contains(attendu), "texte sans « {attendu} »");
        }
        assert!(html.contains("Finance climat &amp; adaptation"));
        assert!(html.contains("indisponible.<br>Nous reviendrons."));
        assert!(html.contains(">Ouvrir le dossier</a>"));
        for corps in [&m.text, &html] {
            assert!(!corps.contains('\u{2014}'));
            assert!(!corps.to_lowercase().contains("désinscri"));
            assert!(!corps.to_lowercase().contains("unsubscribe"));
        }
        assert!(!m.subject.contains('\u{2014}'));
    }

    #[test]
    fn sans_seance_le_creneau_souhaite_est_dit() {
        let souhaite = Creneau::Souhaite(SeanceRetenue {
            debut: datetime!(2026-11-12 09:50),
            fin: datetime!(2026-11-12 10:40),
            timezone: "Europe/Istanbul".to_owned(),
            ville: Some("Antalya".to_owned()),
            salle: None,
        });
        let (m, _) = courriel(
            &annulation(souhaite),
            &equipe(),
            "https://site.org",
            "j",
            None,
        )
        .unwrap();
        assert!(m.text.contains("créneau souhaité par l'organisation"));
        assert!(m.text.contains("Salle : non attribuée"));
        let (m, _) = courriel(
            &annulation(Creneau::Aucun),
            &equipe(),
            "https://site.org",
            "j",
            None,
        )
        .unwrap();
        assert!(m.text.contains("Date : aucune séance programmée"));
    }

    #[test]
    fn pas_de_doublon_ni_d_envoi_sans_equipe() {
        let mut a = annulation(seance());
        a.auteur_email = Some("ISSA.BADO@francophonie.org".to_owned());
        let (_, copies) = courriel(&a, &equipe(), "https://site.org", "j", None).unwrap();
        assert_eq!(copies, vec!["issa.bado@francophonie.org"]);
        assert!(courriel(&a, &[], "https://site.org", "j", None).is_none());
        assert!(courriel(&a, &["  ".to_owned()], "https://site.org", "j", None).is_none());
    }
}
