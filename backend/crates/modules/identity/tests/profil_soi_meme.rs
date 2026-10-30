//! Le profil se modifie par la personne elle-même, et par elle seule. Un
//! identifiant forgé dans l'URL ne change pas la fiche d'un autre.

mod commun;

use commun::{semer, Bac, Compte};
use identity::domain::person::ProfileChanges;
use identity::service::profile;
use kernel::error::ErrorCode;

fn changements(prenom: &str) -> ProfileChanges {
    ProfileChanges {
        civility: Some("mme".into()),
        first_name: format!("  {prenom}  "),
        last_name: "Diallo".into(),
        job_title: Some("".into()),
        phone: Some("+221 77 000 00 00".into()),
        country_id: None,
        city: None,
        preferred_locale: "en".into(),
        timezone: "Africa/Dakar".into(),
        is_directory_visible: false,
    }
}

#[tokio::test]
async fn la_personne_modifie_son_profil() {
    let bac = Bac::monter().await;
    let moi = semer(&bac, Compte::actif("aissatou@example.org")).await;

    let fiche = profile::update(&bac.state, &bac.ctx(), moi, moi, &changements("Aïssatou"))
        .await
        .expect("son propre profil");

    assert_eq!(
        fiche.first_name, "Aïssatou",
        "les blancs autour du prénom tombent"
    );
    assert_eq!(fiche.display_name, "Aïssatou Diallo");
    assert_eq!(fiche.job_title, None, "un texte facultatif vide part nul");
    assert_eq!(fiche.preferred_locale, "en");
    assert_eq!(fiche.timezone, "Africa/Dakar");
    assert!(!fiche.is_directory_visible);
}

#[tokio::test]
async fn le_profil_dun_autre_est_refuse() {
    let bac = Bac::monter().await;
    let moi = semer(&bac, Compte::actif("moi@example.org")).await;
    let autre = semer(&bac, Compte::actif("autre@example.org")).await;

    let refus = profile::update(&bac.state, &bac.ctx(), moi, autre, &changements("Intrus"))
        .await
        .expect_err("la fiche d'un autre");

    assert_eq!(refus.code, ErrorCode::Forbidden);
}

#[tokio::test]
async fn un_fuseau_inconnu_est_nomme() {
    let bac = Bac::monter().await;
    let moi = semer(&bac, Compte::actif("fuseau@example.org")).await;

    let mut faux = changements("Awa");
    faux.timezone = "Mars/Olympus".into();
    let refus = profile::update(&bac.state, &bac.ctx(), moi, moi, &faux)
        .await
        .expect_err("fuseau inconnu");

    assert_eq!(refus.field.as_deref(), Some("timezone"));
}
