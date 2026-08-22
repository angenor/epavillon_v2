//! **La signature S3, éprouvée contre les vecteurs d'exemple d'AWS.**
//!
//! C'est le seul moyen de séparer une erreur de signature d'une erreur de
//! configuration : contre un vrai stockage, un 403 laisse trois causes également
//! plausibles — clé fausse, horloge décalée, signature fautive — et rien ne les
//! départage. Ici, la clé, l'instant et la requête sont ceux d'AWS, et la seule
//! variable restante est le code.
//!
//! Ce fichier n'ouvre **aucune connexion** : ni base, ni réseau.

use media::storage::sigv4::{self, Identifiants, Requete, CHARGE_VIDE};
use time::macros::datetime;

/// Les identifiants de la suite d'exemples d'AWS. Ils n'ouvrent rien.
fn identifiants() -> Identifiants<'static> {
    Identifiants {
        access_key_id: "AKIDEXAMPLE",
        secret_access_key: "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
        region: "us-east-1",
        service: "service",
    }
}

fn entetes_de_base() -> Vec<(String, String)> {
    vec![
        ("host".to_owned(), "example.amazonaws.com".to_owned()),
        ("x-amz-date".to_owned(), "20150830T123600Z".to_owned()),
    ]
}

fn signature(req: &Requete<'_>) -> String {
    let autorisation =
        sigv4::autorisation(req, &identifiants(), datetime!(2015-08-30 12:36:00 UTC));
    autorisation
        .rsplit_once("Signature=")
        .expect("l'en-tête porte une signature")
        .1
        .to_owned()
}

/// `get-vanilla`.
#[test]
fn get_vanilla() {
    let entetes = entetes_de_base();
    assert_eq!(
        signature(&Requete {
            methode: "GET",
            chemin: "/",
            requete: "",
            entetes: &entetes,
            charge_sha256: CHARGE_VIDE,
        }),
        "5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31"
    );
}

/// `get-vanilla-query-order-key-case` : deux paramètres, rangés.
#[test]
fn get_vanilla_query_order_key_case() {
    let entetes = entetes_de_base();
    assert_eq!(
        signature(&Requete {
            methode: "GET",
            chemin: "/",
            requete: "Param1=value1&Param2=value2",
            entetes: &entetes,
            charge_sha256: CHARGE_VIDE,
        }),
        "b97d918cfa904a5beff61c982a1b6f458b799221646efd99d3219ec94cdf2500"
    );
}

/// `get-header-key-duplicate` : un en-tête de plus, et la casse ne compte pas.
///
/// Le cas importe pour de vrai : `reqwest` transmet les noms d'en-tête en
/// minuscules, et signer `X-Amz-Date` là où la requête envoie `x-amz-date`
/// produirait un 403 dont le message ne nomme jamais la cause.
#[test]
fn la_casse_des_entetes_ne_change_pas_la_signature() {
    let minuscules = entetes_de_base();
    let melangees = vec![
        ("Host".to_owned(), "example.amazonaws.com".to_owned()),
        ("X-Amz-Date".to_owned(), "20150830T123600Z".to_owned()),
    ];

    let requete = |entetes: &[(String, String)]| {
        sigv4::autorisation(
            &Requete {
                methode: "GET",
                chemin: "/",
                requete: "",
                entetes,
                charge_sha256: CHARGE_VIDE,
            },
            &identifiants(),
            datetime!(2015-08-30 12:36:00 UTC),
        )
    };

    assert_eq!(requete(&minuscules), requete(&melangees));
}

/// L'ordre dans lequel on donne les en-têtes ne change rien : la
/// canonicalisation les range.
#[test]
fn lordre_des_entetes_ne_change_pas_la_signature() {
    let ordre_a = entetes_de_base();
    let mut ordre_b = entetes_de_base();
    ordre_b.reverse();

    let requete = |entetes: &[(String, String)]| {
        sigv4::autorisation(
            &Requete {
                methode: "GET",
                chemin: "/",
                requete: "",
                entetes,
                charge_sha256: CHARGE_VIDE,
            },
            &identifiants(),
            datetime!(2015-08-30 12:36:00 UTC),
        )
    };

    assert_eq!(requete(&ordre_a), requete(&ordre_b));
}

/// Une charge utile différente change la signature — sans quoi un dépôt
/// pourrait être rejoué avec un autre contenu.
#[test]
fn la_charge_utile_entre_dans_la_signature() {
    let entetes = entetes_de_base();
    let vide = signature(&Requete {
        methode: "PUT",
        chemin: "/epavillon/2026/08/abc/logo.png",
        requete: "",
        entetes: &entetes,
        charge_sha256: CHARGE_VIDE,
    });
    let pleine = signature(&Requete {
        methode: "PUT",
        chemin: "/epavillon/2026/08/abc/logo.png",
        requete: "",
        entetes: &entetes,
        charge_sha256: &sigv4::empreinte(b"des octets"),
    });
    assert_ne!(vide, pleine);
}
