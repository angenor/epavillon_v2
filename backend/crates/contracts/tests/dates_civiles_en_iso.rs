//! **UNE DATE CIVILE TRAVERSE EN `AAAA-MM-JJ`, JAMAIS EN TUPLE.**
//!
//! Ce test ne vérifie pas du code : il verrouille une **option de compilation**.
//! `time::Date` n'a d'attribut de sérialisation nulle part dans le dépôt — les
//! journées d'une édition, les bornes d'un appel, les points des courbes du
//! tableau de bord sont des `Date` nues. Leur forme JSON dépend donc entièrement
//! de la feature `serde-human-readable` de `time`.
//!
//! **Sans elle, `Date` se sérialise en `(année, jour de l'année)`** — `[2026,
//! 251]` — et le contrat ment. C'est arrivé : le 08/09, tout le back-office
//! rendait 500 sur `Invalid time value`, parce que le graphique du tableau de
//! bord recevait un tableau là où il attendait une date. Le piège est double :
//! `serde-well-known` **n'implique pas** `serde-human-readable` bien que les deux
//! activent les mêmes dépendances, et le défaut est **silencieux à la
//! compilation** — rien ne casse, la forme change.
//!
//! Les `OffsetDateTime` n'ont jamais souffert du même mal : ils portent tous
//! `#[serde(with = "time::serde::rfc3339")]`. C'est exactement ce qui a permis au
//! défaut de vivre — toutes les dates *avec heure* étaient correctes.

use time::macros::{date, datetime};

/// Le cas qui a cassé le back-office : la date d'un point de courbe.
#[test]
fn une_date_civile_se_serialise_en_texte_iso() {
    let jour = date!(2026 - 09 - 08);
    assert_eq!(
        serde_json::to_string(&jour).expect("une date se sérialise"),
        "\"2026-09-08\"",
        "sans la feature `serde-human-readable` de `time`, une date part en \
         tuple `[année, jour]` et tout écran qui la lit comme une date échoue"
    );
}

/// Le chemin du retour : un formulaire qui **envoie** une date civile.
#[test]
fn une_date_civile_se_relit_depuis_son_texte_iso() {
    let relue: time::Date = serde_json::from_str("\"2026-09-08\"").expect("une date ISO se relit");
    assert_eq!(relue, date!(2026 - 09 - 08));
}

/// La forme la plus courante du dépôt, pour que le test dise **les deux**
/// contrats — un instant reste en RFC 3339, et ne devient pas un tuple non plus.
#[test]
fn un_instant_reste_en_rfc_3339() {
    #[derive(serde::Serialize)]
    struct Porteur {
        #[serde(with = "time::serde::rfc3339")]
        at: time::OffsetDateTime,
    }

    let json = serde_json::to_string(&Porteur {
        at: datetime!(2026-09-08 14:30:00 UTC),
    })
    .expect("un instant se sérialise");
    assert_eq!(json, r#"{"at":"2026-09-08T14:30:00Z"}"#);
}
