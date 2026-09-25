//! Ce qu'une lecture change à ce qui est en base (research R6, R7) : pur, sans
//! base ni horloge. Une session sans écart n'apparaît pas dans le résultat.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use super::source::EchecLecture;

/// Absente de ce nombre de lectures réussies de suite, une session est annulée.
pub const ABSENCES_AVANT_ANNULATION: i16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Etat {
    Prevue,
    Annulee(Motif),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motif {
    Source,
    Reportee,
    Retiree,
}

impl Motif {
    pub fn code(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Reportee => "postponed",
            Self::Retiree => "removed",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Self> {
        Some(match code {
            "source" => Self::Source,
            "postponed" => Self::Reportee,
            "removed" => Self::Retiree,
            _ => return None,
        })
    }
}

impl Etat {
    pub fn motif(self) -> Option<&'static str> {
        match self {
            Self::Prevue => None,
            Self::Annulee(m) => Some(m.code()),
        }
    }

    fn en_json(self) -> Value {
        match self {
            Self::Prevue => json!({ "status": "scheduled" }),
            Self::Annulee(m) => json!({ "status": "cancelled", "reason": m.code() }),
        }
    }
}

/// Les valeurs comparées d'une session.
#[derive(Debug, Clone, PartialEq)]
pub struct Fiche {
    pub debut: OffsetDateTime,
    pub fin: Option<OffsetDateTime>,
    pub salle: Option<String>,
    pub titre: String,
    pub type_code: String,
    pub acces_ouvert: Option<bool>,
    pub point: Option<String>,
    pub etat: Etat,
}

pub struct EnBase {
    pub id: Uuid,
    pub cle: String,
    pub fiche: Fiche,
    pub absences: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Changement {
    pub champ: &'static str,
    pub avant: Value,
    pub apres: Value,
}

#[derive(Debug, PartialEq)]
pub enum Ecart {
    /// `lue` : l'indice dans la lecture.
    Apparue { lue: usize },
    /// Changée, ou reparue (`reparue` : ses absences retombent à zéro).
    Changee {
        id: Uuid,
        lue: usize,
        changements: Vec<Changement>,
        reparue: bool,
    },
    Absente {
        id: Uuid,
        absences: i16,
        changements: Vec<Changement>,
    },
}

pub struct Comparaison {
    /// Une entrée par session touchée : c'est le compte des écarts.
    pub ecarts: Vec<Ecart>,
    /// Les sessions en base que la lecture contient, écart ou non.
    pub presentes: Vec<Uuid>,
    /// Indices des entrées retenues : une clé lue deux fois ne compte qu'une fois.
    pub retenues: Vec<usize>,
}

/// Une lecture vide est une lecture manquée : pendant une COP, la source est
/// en panne, pas vide.
pub fn comparer(lues: &[(String, Fiche)], base: &[EnBase]) -> Result<Comparaison, EchecLecture> {
    if lues.is_empty() {
        return Err(EchecLecture::AucuneReunion);
    }
    let par_cle: HashMap<&str, &EnBase> = base.iter().map(|b| (b.cle.as_str(), b)).collect();
    let mut vues = HashSet::new();
    let mut resultat = Comparaison {
        ecarts: Vec::new(),
        presentes: Vec::new(),
        retenues: Vec::new(),
    };

    for (i, (cle, fiche)) in lues.iter().enumerate() {
        if !vues.insert(cle.as_str()) {
            continue;
        }
        resultat.retenues.push(i);
        let Some(existante) = par_cle.get(cle.as_str()) else {
            resultat.ecarts.push(Ecart::Apparue { lue: i });
            continue;
        };
        resultat.presentes.push(existante.id);
        let changements = changements(&existante.fiche, fiche);
        let reparue = existante.absences > 0;
        if !changements.is_empty() || reparue {
            resultat.ecarts.push(Ecart::Changee {
                id: existante.id,
                lue: i,
                changements,
                reparue,
            });
        }
    }

    for b in base.iter().filter(|b| !vues.contains(b.cle.as_str())) {
        if b.absences >= ABSENCES_AVANT_ANNULATION {
            continue;
        }
        let absences = b.absences + 1;
        let changements = if absences == ABSENCES_AVANT_ANNULATION && b.fiche.etat == Etat::Prevue {
            vec![Changement {
                champ: "status",
                avant: Etat::Prevue.en_json(),
                apres: Etat::Annulee(Motif::Retiree).en_json(),
            }]
        } else {
            Vec::new()
        };
        resultat.ecarts.push(Ecart::Absente {
            id: b.id,
            absences,
            changements,
        });
    }

    Ok(resultat)
}

fn changements(avant: &Fiche, apres: &Fiche) -> Vec<Changement> {
    let mut sortie = Vec::new();
    let mut noter = |champ, a: Value, b: Value| {
        if a != b {
            sortie.push(Changement {
                champ,
                avant: a,
                apres: b,
            });
        }
    };
    noter(
        "start",
        instant(Some(avant.debut)),
        instant(Some(apres.debut)),
    );
    noter("end", instant(avant.fin), instant(apres.fin));
    noter("venue", json!(avant.salle), json!(apres.salle));
    noter("title", json!(avant.titre), json!(apres.titre));
    noter("type", json!(avant.type_code), json!(apres.type_code));
    noter(
        "access",
        json!(avant.acces_ouvert),
        json!(apres.acces_ouvert),
    );
    noter("agenda_item", json!(avant.point), json!(apres.point));
    noter("status", avant.etat.en_json(), apres.etat.en_json());
    sortie
}

fn instant(moment: Option<OffsetDateTime>) -> Value {
    moment
        .and_then(|m| m.to_offset(time::UtcOffset::UTC).format(&Rfc3339).ok())
        .map_or(Value::Null, Value::String)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn fiche() -> Fiche {
        Fiche {
            debut: datetime!(2026-11-10 08:00 UTC),
            fin: Some(datetime!(2026-11-10 09:00 UTC)),
            salle: Some("Meeting Room 01".into()),
            titre: "CMA 8 (a) Global goal on adaptation - Informal consultation".into(),
            type_code: "informal_consultations".into(),
            acces_ouvert: Some(true),
            point: Some("CMA 8 (a)".into()),
            etat: Etat::Prevue,
        }
    }

    fn en_base(cle: &str, fiche: Fiche, absences: i16) -> EnBase {
        EnBase {
            id: Uuid::now_v7(),
            cle: cle.into(),
            fiche,
            absences,
        }
    }

    #[test]
    fn une_lecture_vide_est_manquee() {
        assert_eq!(
            comparer(&[], &[en_base("1", fiche(), 0)]).err(),
            Some(EchecLecture::AucuneReunion)
        );
    }

    #[test]
    fn identique_rien_a_ecrire() {
        let base = [en_base("1", fiche(), 0)];
        let r = comparer(&[("1".into(), fiche())], &base).expect("comparaison");
        assert!(r.ecarts.is_empty());
        assert_eq!(r.presentes, [base[0].id]);
    }

    #[test]
    fn apparue_et_changee_champ_par_champ() {
        let base = [en_base("1", fiche(), 0)];
        let mut deplacee = fiche();
        deplacee.debut = datetime!(2026-11-10 07:00 UTC);
        deplacee.salle = Some("Meeting Room 03".into());
        let r =
            comparer(&[("1".into(), deplacee), ("2".into(), fiche())], &base).expect("comparaison");
        assert_eq!(r.ecarts.len(), 2);
        let Ecart::Changee {
            changements,
            reparue,
            ..
        } = &r.ecarts[0]
        else {
            panic!("changée attendue");
        };
        assert!(!reparue);
        assert_eq!(
            changements.iter().map(|c| c.champ).collect::<Vec<_>>(),
            ["start", "venue"]
        );
        assert_eq!(changements[0].avant, json!("2026-11-10T08:00:00Z"));
        assert_eq!(changements[0].apres, json!("2026-11-10T07:00:00Z"));
        assert_eq!(r.ecarts[1], Ecart::Apparue { lue: 1 });
    }

    fn absence(r: &Comparaison) -> (i16, Vec<&'static str>) {
        r.ecarts
            .iter()
            .find_map(|e| match e {
                Ecart::Absente {
                    absences,
                    changements,
                    ..
                } => Some((*absences, changements.iter().map(|c| c.champ).collect())),
                _ => None,
            })
            .expect("absente attendue")
    }

    #[test]
    fn absente_une_fois_puis_annulee_a_la_seconde() {
        let lue = [("2".into(), fiche())];
        let premiere = comparer(&lue, &[en_base("1", fiche(), 0)]).expect("comparaison");
        assert_eq!(absence(&premiere), (1, vec![]));

        let seconde = comparer(&lue, &[en_base("1", fiche(), 1)]).expect("comparaison");
        assert_eq!(absence(&seconde), (2, vec!["status"]));

        let mut annulee = fiche();
        annulee.etat = Etat::Annulee(Motif::Retiree);
        let troisieme = comparer(&lue, &[en_base("1", annulee, 2)]).expect("comparaison");
        assert_eq!(
            troisieme.ecarts,
            [Ecart::Apparue { lue: 0 }],
            "l'annulée n'est plus touchée"
        );
    }

    #[test]
    fn reparue_redevient_prevue() {
        let mut annulee = fiche();
        annulee.etat = Etat::Annulee(Motif::Retiree);
        let r =
            comparer(&[("1".into(), fiche())], &[en_base("1", annulee, 2)]).expect("comparaison");
        let Ecart::Changee {
            changements,
            reparue,
            ..
        } = &r.ecarts[0]
        else {
            panic!("changée attendue");
        };
        assert!(reparue);
        assert_eq!(changements[0].champ, "status");
        assert_eq!(changements[0].apres, json!({ "status": "scheduled" }));

        let r =
            comparer(&[("1".into(), fiche())], &[en_base("1", fiche(), 1)]).expect("comparaison");
        assert!(
            matches!(&r.ecarts[0], Ecart::Changee { changements, reparue: true, .. } if changements.is_empty())
        );
    }

    #[test]
    fn une_cle_lue_deux_fois_compte_une_fois() {
        let r =
            comparer(&[("1".into(), fiche()), ("1".into(), fiche())], &[]).expect("comparaison");
        assert_eq!(r.retenues, [0]);
        assert_eq!(r.ecarts, [Ecart::Apparue { lue: 0 }]);
    }
}
