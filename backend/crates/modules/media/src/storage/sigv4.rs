//! **La signature S3, écrite ici — dans un fichier, et avec un critère de
//! bascule écrit d'avance.**
//!
//! # Pourquoi pas le client d'Amazon
//!
//! `aws-sdk-s3` amène une quarantaine de crates, dont la compilation se paie à
//! **chaque** `make check-back` d'un dépôt sans intégration continue, tenu par
//! une seule personne sous contrainte de temps. Quatre verbes suffisent, en
//! *path-style*, contre notre propre stockage sur le réseau interne.
//!
//! **L'argument qui a fait choisir `ammonia` contre un filtre maison ne vaut pas
//! ici.** Une erreur de signature est **bruyante et totale** — 403 sur le
//! premier dépôt —, jamais silencieuse, à l'inverse d'un filtre HTML dont le
//! premier trou est une injection qu'on découvre des mois plus tard.
//!
//! # LE CRITÈRE DE BASCULE, pour ne pas s'entêter
//!
//! Si la signature n'est pas au vert contre Garage **en une demi-journée**,
//! prendre `aws-sdk-s3` et consigner le changement dans le journal du jour. Ce
//! n'est pas une clause de style : un code qui rend 403 sans dire pourquoi peut
//! coûter deux jours à qui décide de « comprendre ».
//!
//! # Ce qui prouve que la signature est juste, et non la configuration
//!
//! Les vecteurs d'exemple d'AWS, éprouvés en test (`media/tests/signature.rs` et
//! les tests unitaires de ce fichier). Sans eux, un 403 laisse trois causes
//! également plausibles — clé fausse, horloge décalée, signature fautive — et
//! rien ne les départage.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

type HmacSha256 = Hmac<Sha256>;

/// L'algorithme, tel que l'en-tête d'autorisation le nomme.
const ALGORITHME: &str = "AWS4-HMAC-SHA256";

/// Empreinte d'une charge utile vide. Elle sert à `GET`, `HEAD` et `DELETE`,
/// qui n'en portent aucune.
pub const CHARGE_VIDE: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Les identifiants et la portée de la signature.
pub struct Identifiants<'a> {
    pub access_key_id: &'a str,
    pub secret_access_key: &'a str,
    pub region: &'a str,
    /// `s3` en production. Les vecteurs d'exemple d'AWS emploient `service`.
    pub service: &'a str,
}

/// La requête à signer. Le chemin et la chaîne de requête arrivent **déjà
/// encodés** : l'encodage d'une clé d'objet appartient à l'appelant, qui seul
/// sait ce qui est un séparateur et ce qui est un signe.
pub struct Requete<'a> {
    pub methode: &'a str,
    /// Chemin absolu, barre oblique initiale comprise.
    pub chemin: &'a str,
    /// Chaîne de requête canonique — paramètres triés, vide si aucun.
    pub requete: &'a str,
    /// En-têtes à signer, **en minuscules**. `host` en fait toujours partie.
    pub entetes: &'a [(String, String)],
    /// Empreinte hexadécimale de la charge utile, ou [`CHARGE_VIDE`].
    pub charge_sha256: &'a str,
}

/// La valeur de l'en-tête `Authorization`.
pub fn autorisation(req: &Requete<'_>, id: &Identifiants<'_>, instant: OffsetDateTime) -> String {
    let horodatage = horodatage(instant);
    let jour = &horodatage[..8];
    let portee = format!("{}/{}/{}/aws4_request", jour, id.region, id.service);

    let (entetes_canoniques, entetes_signes) = canoniser_entetes(req.entetes);

    let requete_canonique = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        req.methode, req.chemin, req.requete, entetes_canoniques, entetes_signes, req.charge_sha256
    );

    let a_signer = format!(
        "{ALGORITHME}\n{horodatage}\n{portee}\n{}",
        hex(&Sha256::digest(requete_canonique.as_bytes()))
    );

    let signature = hex(&hmac(
        &cle_de_signature(id.secret_access_key, jour, id.region, id.service),
        a_signer.as_bytes(),
    ));

    format!(
        "{ALGORITHME} Credential={}/{portee}, SignedHeaders={entetes_signes}, Signature={signature}",
        id.access_key_id
    )
}

/// `20260821T124500Z` — le format que `x-amz-date` exige.
pub fn horodatage(instant: OffsetDateTime) -> String {
    let utc = instant.to_offset(time::UtcOffset::UTC);
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        utc.year(),
        u8::from(utc.month()),
        utc.day(),
        utc.hour(),
        utc.minute(),
        utc.second()
    )
}

pub fn empreinte(contenu: &[u8]) -> String {
    hex(&Sha256::digest(contenu))
}

/// Les en-têtes rangés, minuscules, valeurs réduites — et la liste de leurs
/// noms, séparés par des points-virgules.
fn canoniser_entetes(entetes: &[(String, String)]) -> (String, String) {
    let mut rangees: Vec<(String, String)> = entetes
        .iter()
        .map(|(n, v)| (n.to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    rangees.sort_by(|a, b| a.0.cmp(&b.0));

    let canoniques = rangees
        .iter()
        .map(|(n, v)| format!("{n}:{v}\n"))
        .collect::<String>();
    let signes = rangees
        .iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join(";");

    (canoniques, signes)
}

/// La chaîne de dérivation : quatre HMAC emboîtés, chacun clé du suivant.
fn cle_de_signature(secret: &str, jour: &str, region: &str, service: &str) -> Vec<u8> {
    let date = hmac(format!("AWS4{secret}").as_bytes(), jour.as_bytes());
    let region = hmac(&date, region.as_bytes());
    let service = hmac(&region, service.as_bytes());
    hmac(&service, b"aws4_request")
}

fn hmac(cle: &[u8], message: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(cle).expect("HMAC accepte toute longueur de clé");
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}

fn hex(octets: &[u8]) -> String {
    octets.iter().map(|o| format!("{o:02x}")).collect()
}

/// Encode un segment de clé d'objet pour le chemin d'une URL.
///
/// **Les barres obliques ne sont PAS encodées** : elles séparent les segments
/// d'une clé, et S3 les attend telles quelles. Tout le reste suit RFC 3986, y
/// compris les signes qu'un nom de fichier normalisé ne produit jamais — la
/// normalisation peut changer, l'encodage doit rester juste.
pub fn encoder_chemin(cle: &str) -> String {
    let mut sortie = String::with_capacity(cle.len());
    for octet in cle.bytes() {
        match octet {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                sortie.push(octet as char)
            }
            autre => sortie.push_str(&format!("%{autre:02X}")),
        }
    }
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn exemple() -> Identifiants<'static> {
        Identifiants {
            access_key_id: "AKIDEXAMPLE",
            secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            region: "us-east-1",
            service: "service",
        }
    }

    /// **`get-vanilla` de la suite d'exemples d'AWS.** C'est le seul moyen de
    /// séparer une erreur de signature d'une erreur de configuration.
    #[test]
    fn le_vecteur_get_vanilla_daws_est_reproduit() {
        let entetes = vec![
            ("host".to_owned(), "example.amazonaws.com".to_owned()),
            ("x-amz-date".to_owned(), "20150830T123600Z".to_owned()),
        ];
        let autorisation = autorisation(
            &Requete {
                methode: "GET",
                chemin: "/",
                requete: "",
                entetes: &entetes,
                charge_sha256: CHARGE_VIDE,
            },
            &exemple(),
            datetime!(2015-08-30 12:36:00 UTC),
        );

        assert_eq!(
            autorisation,
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
             SignedHeaders=host;x-amz-date, \
             Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
        );
    }

    /// `get-vanilla-query-order-key-case` : deux paramètres, déjà rangés.
    #[test]
    fn le_vecteur_avec_parametres_est_reproduit() {
        let entetes = vec![
            ("host".to_owned(), "example.amazonaws.com".to_owned()),
            ("x-amz-date".to_owned(), "20150830T123600Z".to_owned()),
        ];
        let autorisation = autorisation(
            &Requete {
                methode: "GET",
                chemin: "/",
                requete: "Param1=value1&Param2=value2",
                entetes: &entetes,
                charge_sha256: CHARGE_VIDE,
            },
            &exemple(),
            datetime!(2015-08-30 12:36:00 UTC),
        );
        assert!(autorisation.contains(
            "Signature=b97d918cfa904a5beff61c982a1b6f458b799221646efd99d3219ec94cdf2500"
        ));
    }

    #[test]
    fn lempreinte_dune_charge_vide_est_la_constante() {
        assert_eq!(empreinte(b""), CHARGE_VIDE);
    }

    #[test]
    fn lhorodatage_a_la_forme_attendue() {
        assert_eq!(
            horodatage(datetime!(2026-08-21 12:45:00 UTC)),
            "20260821T124500Z"
        );
    }

    /// Les barres obliques séparent les segments d'une clé : les encoder
    /// donnerait un objet nommé « 2026%2F08%2F… ».
    #[test]
    fn les_barres_obliques_ne_sont_pas_encodees() {
        assert_eq!(
            encoder_chemin("2026/08/abc/logo.png"),
            "2026/08/abc/logo.png"
        );
        assert_eq!(encoder_chemin("a b"), "a%20b");
        assert_eq!(encoder_chemin("é"), "%C3%A9");
    }
}
