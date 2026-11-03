//! Le magasin : onglet → (vu à, zone, connecté), et la courbe de la dernière
//! heure. Tout en mémoire, perdu au redémarrage : c'est voulu.
//!
//! Les heures sont des secondes Unix passées par l'appelant, ce qui rend le
//! magasin testable sans horloge.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, MutexGuard};
use time::OffsetDateTime;

/// Au-delà, un onglet inconnu n'est plus compté : un client qui inventerait des
/// identifiants ne fait pas enfler la mémoire.
pub const ONGLETS_MAX: usize = 50_000;
/// Un onglet visible se signale toutes les 30 s ; muet depuis une minute, il est parti.
pub const DUREE_DE_VIE: i64 = 60;
/// Purge à l'écriture au plus toutes les 5 s : chaque signal ne parcourt pas tout.
const PURGE_TOUTES: i64 = 5;
const MINUTES: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Live,
    Programme,
    Day,
    Activity,
    Organization,
    Admin,
    Negotiation,
    Other,
}

impl Zone {
    /// Liste fermée : un code inconnu compte dans « autres pages ».
    pub fn lire(code: &str) -> Zone {
        match code {
            "live" => Zone::Live,
            "programme" => Zone::Programme,
            "day" => Zone::Day,
            "activity" => Zone::Activity,
            "organization" => Zone::Organization,
            "admin" => Zone::Admin,
            "negotiation" => Zone::Negotiation,
            _ => Zone::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ZoneCount {
    pub zone: Zone,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PresenceSample {
    #[serde(with = "time::serde::rfc3339")]
    pub at: OffsetDateTime,
    pub total: u32,
}

/// Corps de `GET /admin/presence` (`AdminPresence` côté site).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PresenceSnapshot {
    pub total: u32,
    pub signed_in: u32,
    pub visitors: u32,
    /// Effectif décroissant ; une zone vide n'y figure pas.
    pub zones: Vec<ZoneCount>,
    /// Une entrée par minute, la plus ancienne d'abord ; la dernière est en cours.
    pub last_hour: Vec<PresenceSample>,
    pub peak: Option<PresenceSample>,
    #[serde(with = "time::serde::rfc3339")]
    pub observed_at: OffsetDateTime,
}

struct Onglet {
    vu: i64,
    zone: Zone,
    connecte: bool,
}

#[derive(Default)]
struct Etat {
    onglets: HashMap<String, Onglet>,
    /// (minute Unix, plus grand total vu pendant cette minute), minutes consécutives.
    minutes: VecDeque<(i64, u32)>,
    purge: i64,
}

impl Etat {
    fn purger(&mut self, maintenant: i64) {
        let seuil = maintenant - DUREE_DE_VIE;
        self.onglets.retain(|_, o| o.vu > seuil);
        self.purge = maintenant;
    }

    /// Le total retenu pour la minute est le plus grand vu pendant celle-ci.
    /// Une minute sans aucun signal vaut zéro : chaque onglet visible se
    /// signale deux fois par minute.
    fn noter(&mut self, maintenant: i64) {
        let minute = maintenant.div_euclid(60);
        let total = self.onglets.len() as u32;
        match self.minutes.back_mut() {
            Some((m, t)) if *m == minute => *t = (*t).max(total),
            // Horloge reculée : on n'écrit pas dans le passé.
            Some((m, _)) if *m > minute => {}
            dernier => {
                if let Some(&mut (precedente, _)) = dernier {
                    let depuis = (precedente + 1).max(minute - MINUTES as i64 + 1);
                    for vide in depuis..minute {
                        self.minutes.push_back((vide, 0));
                    }
                }
                self.minutes.push_back((minute, total));
            }
        }
        while self.minutes.len() > MINUTES {
            self.minutes.pop_front();
        }
    }
}

#[derive(Default)]
pub struct Presence {
    etat: Mutex<Etat>,
}

impl Presence {
    fn verrou(&self) -> MutexGuard<'_, Etat> {
        // Un comptage n'a pas d'invariant qu'une panique aurait pu rompre.
        self.etat.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Faux si le magasin est plein et l'onglet inconnu : il n'est pas compté.
    pub fn signaler(&self, onglet: &str, zone: Zone, connecte: bool, maintenant: i64) -> bool {
        let mut etat = self.verrou();
        if maintenant - etat.purge >= PURGE_TOUTES {
            etat.purger(maintenant);
        }
        if !etat.onglets.contains_key(onglet) && etat.onglets.len() >= ONGLETS_MAX {
            etat.purger(maintenant);
            if etat.onglets.len() >= ONGLETS_MAX {
                return false;
            }
        }
        etat.onglets.insert(
            onglet.to_owned(),
            Onglet {
                vu: maintenant,
                zone,
                connecte,
            },
        );
        etat.noter(maintenant);
        true
    }

    pub fn releve(&self, maintenant: i64) -> PresenceSnapshot {
        let mut etat = self.verrou();
        etat.purger(maintenant);
        etat.noter(maintenant);

        let mut par_zone: HashMap<Zone, u32> = HashMap::new();
        let mut signed_in = 0;
        for o in etat.onglets.values() {
            *par_zone.entry(o.zone).or_default() += 1;
            if o.connecte {
                signed_in += 1;
            }
        }
        let mut zones: Vec<ZoneCount> = par_zone
            .into_iter()
            .map(|(zone, count)| ZoneCount { zone, count })
            .collect();
        zones.sort_by(|a, b| b.count.cmp(&a.count).then(a.zone.cmp(&b.zone)));

        let minute = maintenant.div_euclid(60);
        let last_hour: Vec<PresenceSample> = etat
            .minutes
            .iter()
            .filter(|(m, _)| *m > minute - MINUTES as i64)
            .map(|&(m, total)| PresenceSample {
                at: instant(m * 60),
                total,
            })
            .collect();
        let peak = last_hour
            .iter()
            .filter(|s| s.total > 0)
            .fold(None::<&PresenceSample>, |pic, s| match pic {
                Some(p) if p.total > s.total => Some(p),
                _ => Some(s),
            })
            .cloned();

        let total = etat.onglets.len() as u32;
        PresenceSnapshot {
            total,
            signed_in,
            visitors: total - signed_in,
            zones,
            last_hour,
            peak,
            observed_at: instant(maintenant),
        }
    }
}

fn instant(secondes: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(secondes).unwrap_or(OffsetDateTime::UNIX_EPOCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_793_000_000 - 1_793_000_000 % 60;

    fn compte(r: &PresenceSnapshot, zone: Zone) -> u32 {
        r.zones
            .iter()
            .find(|z| z.zone == zone)
            .map_or(0, |z| z.count)
    }

    #[test]
    fn zone_inconnue_dans_autres() {
        assert_eq!(Zone::lire("live"), Zone::Live);
        assert_eq!(Zone::lire("negotiation"), Zone::Negotiation);
        assert_eq!(Zone::lire("LIVE"), Zone::Other);
        assert_eq!(Zone::lire("../admin"), Zone::Other);
    }

    #[test]
    fn comptes_par_zone_tries_par_effectif() {
        let p = Presence::default();
        p.signaler("a", Zone::Programme, false, T0);
        p.signaler("b", Zone::Live, false, T0);
        p.signaler("c", Zone::Live, true, T0);
        p.signaler("d", Zone::Admin, true, T0);
        // Le même onglet qui change de page ne compte qu'une fois.
        p.signaler("a", Zone::Live, false, T0 + 1);
        let r = p.releve(T0 + 2);
        assert_eq!(r.total, 4);
        assert_eq!(compte(&r, Zone::Live), 3);
        assert_eq!(compte(&r, Zone::Programme), 0);
        assert_eq!(r.zones[0].zone, Zone::Live);
        assert_eq!(r.zones.len(), 2);
    }

    #[test]
    fn connectes_et_visiteurs() {
        let p = Presence::default();
        p.signaler("a", Zone::Other, true, T0);
        p.signaler("b", Zone::Other, false, T0);
        p.signaler("c", Zone::Other, false, T0);
        let r = p.releve(T0);
        assert_eq!((r.total, r.signed_in, r.visitors), (3, 1, 2));
        // Déconnecté depuis : le dernier signal fait foi.
        p.signaler("a", Zone::Other, false, T0 + 30);
        assert_eq!(p.releve(T0 + 30).signed_in, 0);
    }

    #[test]
    fn purge_apres_une_minute_de_silence() {
        let p = Presence::default();
        p.signaler("a", Zone::Live, false, T0);
        p.signaler("b", Zone::Live, false, T0 + 30);
        assert_eq!(p.releve(T0 + 59).total, 2);
        assert_eq!(p.releve(T0 + 60).total, 1);
        assert_eq!(p.releve(T0 + 90).total, 0);
        assert!(p.releve(T0 + 90).zones.is_empty());
    }

    #[test]
    fn magasin_borne() {
        let p = Presence::default();
        for i in 0..ONGLETS_MAX {
            assert!(p.signaler(&format!("t{i}"), Zone::Other, false, T0));
        }
        assert!(!p.signaler("de-trop", Zone::Other, false, T0 + 1));
        assert_eq!(p.releve(T0 + 1).total, ONGLETS_MAX as u32);
        // Un onglet déjà connu se signale toujours.
        assert!(p.signaler("t0", Zone::Live, false, T0 + 30));
        // Une fois les anciens partis, la place revient.
        assert!(p.signaler("nouveau", Zone::Other, false, T0 + 61));
        assert_eq!(p.releve(T0 + 61).total, 2);
    }

    #[test]
    fn un_echantillon_par_minute_et_les_creux_a_zero() {
        let p = Presence::default();
        p.signaler("a", Zone::Live, false, T0);
        p.signaler("b", Zone::Live, false, T0 + 10);
        p.signaler("a", Zone::Live, false, T0 + 70);
        // Rien pendant trois minutes, puis un retour.
        p.signaler("c", Zone::Live, false, T0 + 300);
        let r = p.releve(T0 + 301);
        let totaux: Vec<u32> = r.last_hour.iter().map(|s| s.total).collect();
        assert_eq!(totaux, vec![2, 1, 0, 0, 0, 1]);
        assert_eq!(r.last_hour[0].at, instant(T0));
        assert_eq!(r.last_hour[5].at, instant(T0 + 300));
    }

    #[test]
    fn soixante_minutes_au_plus() {
        let p = Presence::default();
        for minute in 0..90 {
            p.signaler("a", Zone::Live, false, T0 + minute * 60);
        }
        let r = p.releve(T0 + 89 * 60 + 5);
        assert_eq!(r.last_hour.len(), 60);
        assert_eq!(r.last_hour[0].at, instant(T0 + 30 * 60));
        // Après une longue absence, la courbe ne garde que la dernière heure.
        let r = p.releve(T0 + 300 * 60);
        assert_eq!(r.last_hour.len(), 60);
        assert!(r.last_hour.iter().all(|s| s.total == 0));
        assert_eq!(r.peak, None);
    }

    #[test]
    fn pic_le_plus_recent_a_egalite() {
        let p = Presence::default();
        p.signaler("a", Zone::Live, false, T0);
        p.signaler("b", Zone::Live, false, T0 + 1);
        p.signaler("a", Zone::Live, false, T0 + 61);
        p.signaler("a", Zone::Live, false, T0 + 121);
        p.signaler("b", Zone::Live, false, T0 + 122);
        let r = p.releve(T0 + 130);
        let pic = r.peak.expect("un pic");
        assert_eq!(pic.total, 2);
        assert_eq!(pic.at, instant(T0 + 120));
    }

    #[test]
    fn rien_vu_aucun_pic() {
        let r = Presence::default().releve(T0);
        assert_eq!(r.total, 0);
        assert_eq!(r.peak, None);
        assert_eq!(r.last_hour.len(), 1);
    }
}
