//! L'adresse du client, et ce à quoi on a le droit d'y croire.
//!
//! `X-Forwarded-For` est écrit par le client autant que par un mandataire :
//! n'importe qui peut l'annoncer. Il ne vaut donc que lorsque la requête arrive
//! **d'un mandataire déclaré en configuration** ; partout ailleurs, c'est
//! l'adresse du pair qui fait foi, la seule que la pile réseau ait constatée.
//!
//! Le défaut **ferme** : sans `TRUSTED_PROXIES`, aucun mandataire n'est cru.
//! Un déploiement derrière un frontal le déclare, et l'oublier fait enregistrer
//! l'adresse du frontal — visible, donc corrigeable — plutôt qu'une adresse que
//! le client aurait choisie lui-même.

use actix_web::HttpRequest;
use std::net::IpAddr;

pub const FORWARDED_FOR: &str = "x-forwarded-for";

/// Les mandataires dont on accepte l'en-tête. Chaque entrée est une adresse ou
/// un préfixe : `10.0.0.7`, `10.0.0.0/8`, `::1`, `2001:db8::/32`.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    prefixes: Vec<Prefixe>,
}

#[derive(Debug, Clone, Copy)]
struct Prefixe {
    reseau: IpAddr,
    bits: u8,
}

impl TrustedProxies {
    pub fn parse(liste: &str) -> Result<Self, String> {
        let prefixes = liste
            .split(',')
            .map(str::trim)
            .filter(|entree| !entree.is_empty())
            .map(Prefixe::parse)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { prefixes })
    }

    pub fn is_empty(&self) -> bool {
        self.prefixes.is_empty()
    }

    pub fn trusts(&self, ip: IpAddr) -> bool {
        let ip = canoniser(ip);
        self.prefixes.iter().any(|prefixe| prefixe.contient(ip))
    }
}

impl Prefixe {
    fn parse(entree: &str) -> Result<Self, String> {
        let (adresse, bits) = match entree.split_once('/') {
            Some((adresse, bits)) => {
                let bits = bits
                    .parse::<u8>()
                    .map_err(|_| format!("« {entree} » : longueur de préfixe illisible."))?;
                (adresse, Some(bits))
            }
            None => (entree, None),
        };

        let reseau = canoniser(
            adresse
                .parse::<IpAddr>()
                .map_err(|_| format!("« {entree} » n'est pas une adresse IP."))?,
        );

        let maximum = if reseau.is_ipv4() { 32 } else { 128 };
        let bits = bits.unwrap_or(maximum);
        if bits > maximum {
            return Err(format!(
                "« {entree} » : un préfixe ne dépasse pas /{maximum}."
            ));
        }

        Ok(Self { reseau, bits })
    }

    fn contient(&self, ip: IpAddr) -> bool {
        match (self.reseau, ip) {
            (IpAddr::V4(reseau), IpAddr::V4(ip)) => {
                memes_bits(&reseau.octets(), &ip.octets(), self.bits)
            }
            (IpAddr::V6(reseau), IpAddr::V6(ip)) => {
                memes_bits(&reseau.octets(), &ip.octets(), self.bits)
            }
            // Deux familles différentes ne se comparent pas : `10.0.0.0/8` ne
            // contient aucune adresse IPv6, et l'inverse non plus.
            _ => false,
        }
    }
}

fn memes_bits(reseau: &[u8], ip: &[u8], bits: u8) -> bool {
    let entiers = (bits / 8) as usize;
    if reseau[..entiers] != ip[..entiers] {
        return false;
    }
    let reste = bits % 8;
    if reste == 0 {
        return true;
    }
    let masque = 0xffu8 << (8 - reste);
    reseau[entiers] & masque == ip[entiers] & masque
}

/// Une socket à double pile rend `::ffff:127.0.0.1` là où la configuration
/// écrit `127.0.0.1` : sans cette remise à plat, les deux ne se compareraient
/// jamais.
fn canoniser(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(v6)),
        v4 => v4,
    }
}

/// L'adresse du client, telle qu'on a le droit d'y croire.
///
/// Sans mandataire de confiance, c'est l'adresse du pair. Avec, on remonte la
/// chaîne annoncée **de droite à gauche** — chaque saut ajoute son prédécesseur
/// à droite, donc l'adresse la plus à droite est celle que notre propre
/// mandataire a constatée, la seule qu'il ait pu vérifier. On saute les
/// mandataires connus ; le premier qui n'en est pas est le client. Un segment
/// illisible arrête la remontée : au-delà, plus rien n'est vérifiable.
pub fn client_ip(requete: &HttpRequest, mandataires: &TrustedProxies) -> Option<IpAddr> {
    let pair = canoniser(requete.peer_addr()?.ip());
    if !mandataires.trusts(pair) {
        return Some(pair);
    }

    let annonce = requete
        .headers()
        .get(FORWARDED_FOR)
        .and_then(|valeur| valeur.to_str().ok())
        .unwrap_or_default();

    let mut dernier_connu = None;
    for segment in annonce.rsplit(',') {
        let Ok(ip) = segment.trim().parse::<IpAddr>() else {
            break;
        };
        let ip = canoniser(ip);
        if !mandataires.trusts(ip) {
            return Some(ip);
        }
        dernier_connu = Some(ip);
    }

    Some(dernier_connu.unwrap_or(pair))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    fn ip(valeur: &str) -> IpAddr {
        valeur.parse().expect("adresse de test")
    }

    fn requete(pair: &str, annonce: Option<&str>) -> HttpRequest {
        let mut constructeur = TestRequest::default()
            .peer_addr(format!("{pair}:54321").parse().expect("socket de test"));
        if let Some(annonce) = annonce {
            constructeur = constructeur.insert_header((FORWARDED_FOR, annonce));
        }
        constructeur.to_http_request()
    }

    #[test]
    fn sans_mandataire_declare_lentete_ne_vaut_rien() {
        let aucun = TrustedProxies::default();
        let vue = client_ip(&requete("198.51.100.4", Some("203.0.113.9")), &aucun);

        assert_eq!(vue, Some(ip("198.51.100.4")));
    }

    #[test]
    fn un_mandataire_declare_fait_foi() {
        let connus = TrustedProxies::parse("10.0.0.0/8").expect("liste");
        let vue = client_ip(&requete("10.2.3.4", Some("203.0.113.9")), &connus);

        assert_eq!(vue, Some(ip("203.0.113.9")));
    }

    /// Une chaîne de mandataires : on remonte jusqu'au premier maillon qu'on ne
    /// connaît pas, et pas plus loin.
    #[test]
    fn la_chaine_se_remonte_de_droite_a_gauche() {
        let connus = TrustedProxies::parse("10.0.0.0/8, 192.0.2.1").expect("liste");
        let vue = client_ip(
            &requete("10.2.3.4", Some("198.51.100.7, 203.0.113.9, 192.0.2.1")),
            &connus,
        );

        assert_eq!(vue, Some(ip("203.0.113.9")));
    }

    #[test]
    fn un_segment_illisible_arrete_la_remontee() {
        let connus = TrustedProxies::parse("10.0.0.0/8").expect("liste");
        let vue = client_ip(&requete("10.2.3.4", Some("203.0.113.9, unknown")), &connus);

        assert_eq!(vue, Some(ip("10.2.3.4")));
    }

    #[test]
    fn un_mandataire_sans_entete_reste_lui_meme() {
        let connus = TrustedProxies::parse("10.0.0.0/8").expect("liste");

        assert_eq!(
            client_ip(&requete("10.2.3.4", None), &connus),
            Some(ip("10.2.3.4"))
        );
    }

    #[test]
    fn une_adresse_ipv4_projetee_se_compare_a_son_equivalent() {
        let connus = TrustedProxies::parse("127.0.0.1").expect("liste");

        assert!(connus.trusts(ip("::ffff:127.0.0.1")));
        assert!(!connus.trusts(ip("::1")));
    }

    #[test]
    fn les_prefixes_ne_franchissent_pas_les_familles() {
        let connus = TrustedProxies::parse("10.0.0.0/8, 2001:db8::/32").expect("liste");

        assert!(connus.trusts(ip("10.255.255.255")));
        assert!(!connus.trusts(ip("11.0.0.1")));
        assert!(connus.trusts(ip("2001:db8:dead::1")));
        assert!(!connus.trusts(ip("2001:db9::1")));
    }

    #[test]
    fn un_prefixe_qui_ne_tombe_pas_sur_un_octet() {
        let connus = TrustedProxies::parse("192.0.2.0/26").expect("liste");

        assert!(connus.trusts(ip("192.0.2.63")));
        assert!(!connus.trusts(ip("192.0.2.64")));
    }

    #[test]
    fn une_liste_illisible_est_refusee() {
        assert!(TrustedProxies::parse("pas-une-adresse").is_err());
        assert!(TrustedProxies::parse("10.0.0.0/33").is_err());
        assert!(TrustedProxies::parse("").expect("vide").is_empty());
    }
}
