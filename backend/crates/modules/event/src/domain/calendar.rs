//! Le plan de génération du calendrier — **fonction pure** (research.md § R4).
//!
//! **Aucun déclencheur du modèle ne dérive les journées d'une édition** : rien
//! en base n'en crée quand la période s'élargit, rien n'en supprime quand elle
//! se resserre. La génération est donc un comportement d'application, et un
//! **geste explicite**.
//!
//! Le plan et son exécution partagent cette fonction, et l'exécution la
//! **recalcule dans sa transaction** : jamais elle ne fait confiance au plan que
//! le client lui renvoie. Sans quoi deux onglets ouverts côte à côte suffiraient
//! à faire supprimer une journée qu'on croyait hors période.
//!
//! Elle ne connaît que des dates : les identifiants, les décomptes de séances et
//! le rang se posent au-dessus. C'est ce qui la rend éprouvable sans base.

use std::collections::HashSet;
use time::{Date, Duration};

/// Ce que la génération **ferait**, sans rien écrire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Dates de la période qui n'ont pas encore de journée, croissantes.
    pub to_create: Vec<Date>,
    /// Journées existantes dont la date sort de la période, croissantes. Elles
    /// sont **signalées, jamais retirées d'office** : une soirée d'ouverture la
    /// veille est un cas légitime, et le choix appartient à l'équipe (FR-035).
    pub to_review: Vec<Date>,
    /// Journées déjà en place et dans la période : rien à faire.
    pub unchanged: usize,
}

/// Le plan, à partir de la période civile et des journées déjà créées.
///
/// `premier_jour` et `dernier_jour` sont les dates **civiles dans le fuseau de
/// l'édition**, calculées en base : les recalculer ici demanderait une base de
/// fuseaux qui n'est pas celle de PostgreSQL (research.md § R5).
pub fn plan(premier_jour: Date, dernier_jour: Date, journees_existantes: &[Date]) -> Plan {
    let existantes: HashSet<Date> = journees_existantes.iter().copied().collect();

    let mut to_create = Vec::new();
    let mut unchanged = 0;

    // Une période inversée ne se produit pas — `ck_events_period` l'interdit —
    // mais elle ne doit pas produire de boucle sans fin si elle se produisait.
    let mut jour = premier_jour;
    while jour <= dernier_jour {
        if existantes.contains(&jour) {
            unchanged += 1;
        } else {
            to_create.push(jour);
        }
        match jour.checked_add(Duration::days(1)) {
            Some(suivant) => jour = suivant,
            None => break,
        }
    }

    let mut to_review: Vec<Date> = journees_existantes
        .iter()
        .copied()
        .filter(|d| *d < premier_jour || *d > dernier_jour)
        .collect();
    to_review.sort_unstable();
    to_review.dedup();

    Plan {
        to_create,
        to_review,
        unchanged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn une_periode_vierge_se_cree_en_entier() {
        let p = plan(date!(2027 - 11 - 09), date!(2027 - 11 - 20), &[]);
        assert_eq!(p.to_create.len(), 12, "du 9 au 20 inclus");
        assert_eq!(p.to_create.first(), Some(&date!(2027 - 11 - 09)));
        assert_eq!(p.to_create.last(), Some(&date!(2027 - 11 - 20)));
        assert!(p.to_review.is_empty());
        assert_eq!(p.unchanged, 0);
    }

    #[test]
    fn une_periode_identique_ne_propose_rien() {
        let existantes = [
            date!(2027 - 11 - 09),
            date!(2027 - 11 - 10),
            date!(2027 - 11 - 11),
        ];
        let p = plan(date!(2027 - 11 - 09), date!(2027 - 11 - 11), &existantes);
        assert!(p.to_create.is_empty());
        assert!(p.to_review.is_empty());
        assert_eq!(p.unchanged, 3);
    }

    #[test]
    fn une_periode_elargie_ne_propose_que_les_dates_absentes() {
        let existantes = [date!(2027 - 11 - 10), date!(2027 - 11 - 11)];
        let p = plan(date!(2027 - 11 - 09), date!(2027 - 11 - 12), &existantes);
        assert_eq!(
            p.to_create,
            vec![date!(2027 - 11 - 09), date!(2027 - 11 - 12)]
        );
        assert_eq!(p.unchanged, 2);
        assert!(p.to_review.is_empty());
    }

    /// Une période resserrée **signale** et ne retire pas : c'est FR-035, et
    /// c'est ce qui distingue le plan de son exécution.
    #[test]
    fn une_periode_resserree_signale_les_journees_hors_bornes() {
        let existantes = [
            date!(2027 - 11 - 09),
            date!(2027 - 11 - 10),
            date!(2027 - 11 - 11),
            date!(2027 - 11 - 12),
        ];
        let p = plan(date!(2027 - 11 - 10), date!(2027 - 11 - 11), &existantes);
        assert!(p.to_create.is_empty());
        assert_eq!(
            p.to_review,
            vec![date!(2027 - 11 - 09), date!(2027 - 11 - 12)]
        );
        assert_eq!(p.unchanged, 2);
    }

    /// Le cas du cycle de webinaires : **plus de trois cents journées
    /// annoncées**, et aucune écrite. C'est ce qui rend l'arbitrage possible
    /// plutôt que de le devancer.
    #[test]
    fn une_periode_dun_an_annonce_plus_de_trois_cents_journees() {
        let p = plan(date!(2027 - 01 - 01), date!(2027 - 12 - 31), &[]);
        assert_eq!(p.to_create.len(), 365);
        assert!(p.to_create.len() > 300);
    }

    #[test]
    fn une_periode_bissextile_compte_son_jour_de_plus() {
        let p = plan(date!(2028 - 01 - 01), date!(2028 - 12 - 31), &[]);
        assert_eq!(p.to_create.len(), 366);
    }

    /// Elle ne se produit pas — la base l'interdit — mais elle ne doit pas
    /// boucler si elle se produisait.
    #[test]
    fn une_periode_inversee_ne_cree_rien_et_signale_tout() {
        let existantes = [date!(2027 - 11 - 10)];
        let p = plan(date!(2027 - 11 - 20), date!(2027 - 11 - 09), &existantes);
        assert!(p.to_create.is_empty());
        assert_eq!(p.to_review, vec![date!(2027 - 11 - 10)]);
        assert_eq!(p.unchanged, 0);
    }
}
