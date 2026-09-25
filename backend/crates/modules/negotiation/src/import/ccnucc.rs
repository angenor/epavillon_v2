//! Le format du calendrier de conférence de la CCNUCC, commun aux deux lecteurs
//! (research R2) : un tableau JSON, une entrée par réunion.
//!
//! Ce que la source ne dit pas en clair se lit dans le titre : l'annulation et
//! le report en préfixe, le point de l'ordre du jour en tête (COP30) ou en
//! queue (COP29), fautes de frappe comprises.

use regex::Regex;
use serde::Deserialize;
use std::sync::LazyLock;
use time::macros::format_description;
use time::{Duration, PrimitiveDateTime};

use super::source::{AnnulationSource, EchecLecture, PointLu, SessionLue};

#[derive(Deserialize)]
struct Entree {
    id: serde_json::Value,
    title: String,
    #[serde(default)]
    date: Vec<Plage>,
    #[serde(default)]
    room: Vec<Valeur>,
    #[serde(default)]
    typeofevent: Vec<String>,
    #[serde(default)]
    access: Vec<Valeur>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize)]
struct Plage {
    value: String,
    #[serde(default)]
    end_value: Option<String>,
}

#[derive(Deserialize)]
struct Valeur {
    value: String,
}

/// Lit le JSON du calendrier. `correction_minutes` s'ajoute aux heures naïves :
/// la source publie l'heure locale moins une heure.
pub fn analyser(texte: &str, correction_minutes: i64) -> Result<Vec<SessionLue>, EchecLecture> {
    let texte = texte.trim_start_matches('\u{feff}').trim();
    if texte.is_empty() {
        return Err(EchecLecture::Illisible(
            "la source a rendu une page vide".into(),
        ));
    }
    if texte.starts_with('<') {
        return Err(EchecLecture::Illisible(
            "la source a rendu une page web au lieu des données, protection anti-robot probable"
                .into(),
        ));
    }
    let valeurs: Vec<serde_json::Value> = serde_json::from_str(texte)
        .map_err(|e| EchecLecture::Illisible(format!("JSON mal formé ({e})")))?;

    let correction = Duration::minutes(correction_minutes);
    let sessions: Vec<SessionLue> = valeurs
        .iter()
        .filter_map(|v| serde_json::from_value::<Entree>(v.clone()).ok())
        .filter_map(|e| session(e, correction))
        .collect();

    if !valeurs.is_empty() && sessions.is_empty() {
        return Err(EchecLecture::Illisible(
            "aucune entrée au format du calendrier de conférence".into(),
        ));
    }
    Ok(sessions)
}

fn session(e: Entree, correction: Duration) -> Option<SessionLue> {
    let cle = match e.id {
        serde_json::Value::String(s) if !s.trim().is_empty() => s.trim().to_owned(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => return None,
    };
    let url = e.url.map(|u| u.trim().to_owned())?;
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return None;
    }
    let plage = e.date.first()?;
    let debut = heure(&plage.value)? + correction;
    let fin = plage
        .end_value
        .as_deref()
        .and_then(heure)
        .map(|f| f + correction)
        .filter(|f| *f > debut);

    let (annulation, titre) = retirer_prefixe(&resserrer(&e.title));
    Some(SessionLue {
        cle,
        point: extraire_point(&titre),
        titre,
        annulation,
        categories: e.typeofevent,
        debut,
        fin,
        salle: e
            .room
            .first()
            .map(|r| resserrer(&r.value))
            .filter(|s| !s.is_empty()),
        acces_ouvert: e.access.first().and_then(|a| match a.value.trim() {
            "0" => Some(true),
            "1" => Some(false),
            _ => None,
        }),
        url,
    })
}

fn heure(valeur: &str) -> Option<PrimitiveDateTime> {
    PrimitiveDateTime::parse(
        valeur.trim(),
        format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]"),
    )
    .ok()
}

fn resserrer(texte: &str) -> String {
    texte.split_whitespace().collect::<Vec<_>>().join(" ")
}

static PREFIXE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\*+\s*)?(CANCELL?ED|POSTPONED)\s*(\*+|[-:–])?\s*").expect("motif valide")
});

/// « ** POSTPONED ** Titre », « CANCELLED - Titre ». Un préfixe sans étoiles ni
/// séparateur n'en est pas un : « Cancelled projects… » reste un titre.
pub fn retirer_prefixe(titre: &str) -> (Option<AnnulationSource>, String) {
    if let Some(m) = PREFIXE.captures(titre) {
        if m.get(1).is_some() || m.get(3).is_some() {
            let annulation = if m[2].eq_ignore_ascii_case("postponed") {
                AnnulationSource::Reportee
            } else {
                AnnulationSource::Annulee
            };
            let reste = titre[m.get(0).map_or(0, |t| t.end())..].trim();
            if !reste.is_empty() {
                return (Some(annulation), reste.to_owned());
            }
        }
    }
    (None, titre.to_owned())
}

const ORGANE: &str = r"(?:COP|CMP|CMA|SBI|SBSTA|APA)";

static CODE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"({ORGANE})\s+(\d{{1,2}})(?:\s*\(\s*([a-z])\s*\))?"
    ))
    .expect("motif valide")
});

static SEQUENCE: LazyLock<Regex> = LazyLock::new(|| {
    let un = format!(r"{ORGANE}\s+\d{{1,2}}(?:\s*\(\s*[a-z]\s*\))?");
    Regex::new(&format!(r"\b{un}(?:\s*/\s*{un})*")).expect("motif valide")
});

static CODE_SUIVANT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"\s*/\s*{ORGANE}\s+\d")).expect("motif valide"));

static SEPARATEUR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+[-–]\s*|\s*[-–]\s+").expect("motif valide"));

static NATURE_AVANT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:[:\-–]|\b(?:consultations?|group|informals?))\s*$").expect("motif valide")
});

/// Le point de l'ordre du jour cité par un titre, dans ses deux ordres :
/// « CMA 8 (a) Global goal on adaptation - Informal consultation » (COP30),
/// « New collective quantified goal - Informal consultation: CMA 11 (a) »
/// (COP29). « of COP 30 » n'en est pas un : en queue, le code suit la nature
/// de la réunion ou un séparateur.
pub fn extraire_point(titre: &str) -> Option<PointLu> {
    for m in SEQUENCE.find_iter(titre) {
        if titre[m.end()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '.')
        {
            continue;
        }
        let code = normaliser_code(m.as_str());
        let intitule = if m.start() == 0 {
            intitule_en_tete(&titre[m.end()..])
        } else if NATURE_AVANT.is_match(&titre[..m.start()]) {
            intitule_en_queue(&titre[..m.start()], &titre[m.end()..])
        } else {
            continue;
        };
        let intitule = if intitule.is_empty() {
            titre.to_owned()
        } else {
            intitule
        };
        return Some(PointLu { code, intitule });
    }
    None
}

fn normaliser_code(sequence: &str) -> String {
    CODE.captures_iter(sequence)
        .map(|c| match c.get(3) {
            Some(lettre) => format!("{} {} ({})", &c[1], &c[2], lettre.as_str()),
            None => format!("{} {}", &c[1], &c[2]),
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn intitule_en_tete(reste: &str) -> String {
    let reste = match CODE_SUIVANT.find(reste) {
        Some(m) => &reste[..m.start()],
        None => reste,
    };
    let reste = reste.trim_start_matches(|c: char| c.is_whitespace() || ":.-–".contains(c));
    avant_la_nature(reste)
}

fn intitule_en_queue(avant: &str, apres: &str) -> String {
    let queue = apres
        .trim_start_matches(|c: char| c.is_whitespace() || ":-–".contains(c))
        .trim();
    if !queue.is_empty() {
        return queue.to_owned();
    }
    avant_la_nature(avant.trim_end_matches(|c: char| c.is_whitespace() || ":-–".contains(c)))
}

/// Ce qui précède le dernier séparateur « - » : la nature de la réunion suit.
/// Un trait d'union collé (« el-Sheikh ») n'en est pas un.
fn avant_la_nature(texte: &str) -> String {
    match SEPARATEUR.find_iter(texte).last() {
        Some(m) => texte[..m.start()].trim().to_owned(),
        None => texte.trim().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn point(titre: &str) -> Option<(String, String)> {
        extraire_point(&resserrer(titre)).map(|p| (p.code, p.intitule))
    }

    fn attendu(code: &str, intitule: &str) -> Option<(String, String)> {
        Some((code.to_owned(), intitule.to_owned()))
    }

    #[test]
    fn le_point_en_tete_cop30() {
        assert_eq!(
            point("CMA 8 (a) Global goal on adaptation  - Informal consultation"),
            attendu("CMA 8 (a)", "Global goal on adaptation")
        );
        assert_eq!(
            point("CMA 10 (g ) Sharm el-Sheikh dialogue on the scope of Article 2  - Informal consultation"),
            attendu("CMA 10 (g)", "Sharm el-Sheikh dialogue on the scope of Article 2")
        );
        assert_eq!(
            point("COP 12/ CMA 14/ CMP 8 Matters relating to the forum - Informal consultation"),
            attendu("COP 12/CMA 14/CMP 8", "Matters relating to the forum")
        );
        assert_eq!(
            point("CMA 10 (h) UAE dialogue referred to in paragraph 97 of decision 1/CMA.5- Informal consultation"),
            attendu("CMA 10 (h)", "UAE dialogue referred to in paragraph 97 of decision 1/CMA.5")
        );
    }

    #[test]
    fn deux_points_dans_un_titre_le_premier_seul() {
        assert_eq!(
            point("COP  8 (c)/CMA 10 (b) Report of the Green Climate Fund /  COP 8 (d)/CMA 10 (c) Report of the GEF - Informal consultation"),
            attendu("COP 8 (c)/CMA 10 (b)", "Report of the Green Climate Fund")
        );
    }

    #[test]
    fn le_point_en_queue_cop29() {
        assert_eq!(
            point("New collective quantified goal on climate finance  - Informal consultation: CMA 11 (a)"),
            attendu("CMA 11 (a)", "New collective quantified goal on climate finance")
        );
        assert_eq!(
            point("Poznan Strategic Programme on Technology Transfer- Informal Consultation - SBI 14(c)"),
            attendu("SBI 14 (c)", "Poznan Strategic Programme on Technology Transfer")
        );
        assert_eq!(
            point("Report of the Green Climate Fund - Contact group CMA 11 (c)"),
            attendu("CMA 11 (c)", "Report of the Green Climate Fund")
        );
        assert_eq!(
            point("Research and systematic observation - Informal consultation: SBSTA  4"),
            attendu("SBSTA 4", "Research and systematic observation")
        );
    }

    #[test]
    fn un_code_suivi_d_un_intitule_court() {
        assert_eq!(
            point("HoDs draftng group CMA 11 (a) -NCQG"),
            attendu("CMA 11 (a)", "NCQG")
        );
    }

    #[test]
    fn un_code_qui_n_est_pas_un_point() {
        assert_eq!(point("High-level segment (HLS) of COP 30"), None);
        assert_eq!(point("COP30 AA: Activation Group Stocktake (KO3)"), None);
        assert_eq!(point("guidance in decision 2/CMA.3"), None);
        assert_eq!(point("EIG Coordination Meeting"), None);
    }

    #[test]
    fn les_prefixes_de_la_source() {
        assert_eq!(
            retirer_prefixe("** POSTPONED ** COP 29 opening press conference"),
            (
                Some(AnnulationSource::Reportee),
                "COP 29 opening press conference".into()
            )
        );
        assert_eq!(
            retirer_prefixe("CANCELLED - COP 29 Presidency and UN Climate Change"),
            (
                Some(AnnulationSource::Annulee),
                "COP 29 Presidency and UN Climate Change".into()
            )
        );
        assert_eq!(
            retirer_prefixe("Cancelled projects in review"),
            (None, "Cancelled projects in review".into())
        );
    }

    #[test]
    fn une_entree_reelle_corrigee_d_une_heure() {
        let json = r#"[{"id":"654006","title":"CMA 8 (a) Global goal on adaptation  - Informal consultation",
            "date":[{"value":"2025-11-18T10:00:00","end_value":"2025-11-18T11:00:00"}],
            "room":[{"value":"Meeting Room 01"}],"typeofevent":["Negotiations"],"access":[{"value":"0"}],
            "body":[],"url":"https://unfccc.int/event/cma-8-a-global-goal-on-adaptation-informal-consultation-1","time":"10:00"}]"#;
        let lues = analyser(json, 60).expect("lecture");
        assert_eq!(lues.len(), 1);
        let s = &lues[0];
        // La fiche de la source affiche « 18 Nov. 2025 11:00h - 12:00h BRT ».
        assert_eq!(s.debut, datetime!(2025-11-18 11:00));
        assert_eq!(s.fin, Some(datetime!(2025-11-18 12:00)));
        assert_eq!(s.cle, "654006");
        assert_eq!(
            s.titre,
            "CMA 8 (a) Global goal on adaptation - Informal consultation"
        );
        assert_eq!(s.acces_ouvert, Some(true));
        assert_eq!(s.salle.as_deref(), Some("Meeting Room 01"));
        assert_eq!(s.point.as_ref().map(|p| p.code.as_str()), Some("CMA 8 (a)"));
    }

    #[test]
    fn acces_limite_et_fin_absente() {
        let json = r#"[{"id":1,"title":"EIG Coordination Meeting","date":[{"value":"2025-11-17T08:00:00"}],
            "room":[],"typeofevent":["Coordination meetings"],"access":[{"value":"1"}],"url":"https://unfccc.int/e"}]"#;
        let s = &analyser(json, 60).expect("lecture")[0];
        assert_eq!(s.acces_ouvert, Some(false));
        assert_eq!(s.fin, None);
        assert_eq!(s.salle, None);
        assert_eq!(s.cle, "1");
    }

    #[test]
    fn ce_qui_n_est_pas_le_calendrier_est_illisible() {
        assert!(matches!(analyser("", 60), Err(EchecLecture::Illisible(_))));
        assert!(matches!(
            analyser("<html><body></body></html>", 60),
            Err(EchecLecture::Illisible(_))
        ));
        assert!(matches!(
            analyser("[{\"id\": ", 60),
            Err(EchecLecture::Illisible(_))
        ));
        assert!(matches!(
            analyser(r#"{"events": []}"#, 60),
            Err(EchecLecture::Illisible(_))
        ));
        assert!(matches!(
            analyser(r#"[{"nom": "autre chose"}]"#, 60),
            Err(EchecLecture::Illisible(_))
        ));
        assert_eq!(analyser("[]", 60), Ok(vec![]));
    }
}
