//! **Les six contraintes nommées d'une édition, chacune sur son champ.**
//!
//! Principe VIII : le service ne redouble aucun de ces invariants — il ne
//! vérifie ni que la fin suit le début, ni que l'adresse est libre. Il laisse la
//! base refuser et **traduit** son refus, en branchant sur le **nom de la
//! contrainte** et jamais sur le texte du message, qui est localisé par
//! PostgreSQL et se reformule d'une version à l'autre.
//!
//! Seul un aller-retour réel prouve cette traduction : un test qui se fierait au
//! typage n'éprouverait rien, puisque les contraintes vivent dans PostgreSQL.
//!
//! **Et aucun message technique ne franchit la réponse** : la forme du contrat
//! ne porte qu'un code et un champ. Un nom de table dans un écran est une fuite,
//! pas un diagnostic.

mod commun;

use commun::{auteur, formulaire, serie, Bac};
use event::domain::edition::{EditionErrorCode, EditionFormPayload, EditionSaveResult};
use event::service::edition_write;

async fn ecrire(bac: &Bac, p: EditionFormPayload) -> EditionSaveResult {
    let acteur = auteur(bac).await;
    edition_write::creer(&bac.state, &bac.ctx(), acteur, p)
        .await
        .expect("un refus de contrainte est une réponse, pas une erreur HTTP")
}

fn refuse(r: &EditionSaveResult, code: EditionErrorCode, champ: &str) {
    assert!(!r.ok, "l'écriture devait être refusée");
    assert!(r.edition.is_none(), "rien n'a été écrit");
    assert_eq!(r.errors.len(), 1, "un seul refus : {:?}", r.errors);
    assert_eq!(r.errors[0].code, code);
    assert_eq!(r.errors[0].field.as_deref(), Some(champ));
}

/// `ck_events_period` → `period`, sur `ends_at`.
#[tokio::test]
async fn une_periode_inversee_se_refuse_sur_la_fin() {
    let bac = Bac::monter().await;

    let mut p = formulaire("periode-inversee", "Édition à période inversée");
    p.starts_at = commun::instant("2027-03-04T16:00:00Z");
    p.ends_at = commun::instant("2027-03-02T10:00:00Z");

    refuse(&ecrire(&bac, p).await, EditionErrorCode::Period, "ends_at");
}

/// `ck_events_physical_location` → `physical_location`. Hors ligne, le pays
/// **et** la ville sont exigés ; on nomme celui des deux qui manque, le pays
/// d'abord — c'est l'ordre du formulaire.
#[tokio::test]
async fn une_edition_hors_ligne_sans_lieu_se_refuse_sur_le_pays_puis_la_ville() {
    let bac = Bac::monter().await;
    let bresil = commun::pays(&bac, "BRA").await;

    let mut p = formulaire("sans-lieu", "Édition sans lieu");
    p.participation_mode = "in_person".to_owned();

    refuse(
        &ecrire(&bac, p.clone()).await,
        EditionErrorCode::PhysicalLocation,
        "country_id",
    );

    // Le pays donné, la ville manquante : c'est elle qu'il faut marquer.
    p.country_id = Some(bresil);
    refuse(
        &ecrire(&bac, p).await,
        EditionErrorCode::PhysicalLocation,
        "city",
    );
}

/// `ux_events_slug` → `slug_taken`. L'unicité porte sur **toute la plateforme**,
/// pas sur la série.
#[tokio::test]
async fn une_adresse_deja_prise_se_refuse_sur_son_champ() {
    let bac = Bac::monter().await;

    let premiere = ecrire(&bac, formulaire("meme-adresse", "Première")).await;
    assert!(premiere.ok, "{:?}", premiere.errors);

    refuse(
        &ecrire(&bac, formulaire("meme-adresse", "Seconde")).await,
        EditionErrorCode::SlugTaken,
        "slug",
    );
}

/// `ux_events_series_edition` → `edition_taken`, sur le libellé.
#[tokio::test]
async fn un_millesime_deja_pris_dans_sa_serie_se_refuse_sur_le_libelle() {
    let bac = Bac::monter().await;
    let climat = serie(&bac, "cop_climate").await;

    let mut p = formulaire("cop31-a", "COP31");
    p.series_id = Some(climat);
    p.edition_label = Some("COP31".to_owned());

    let premiere = ecrire(&bac, p.clone()).await;
    assert!(premiere.ok, "{:?}", premiere.errors);

    p.slug = "cop31-b".to_owned();
    refuse(
        &ecrire(&bac, p).await,
        EditionErrorCode::EditionTaken,
        "edition_label",
    );
}

/// `events_edition_year_check` → `year_range`.
#[tokio::test]
async fn un_millesime_hors_bornes_se_refuse_sur_son_champ() {
    let bac = Bac::monter().await;

    let mut p = formulaire("millesime-hors-bornes", "Édition d'un autre siècle");
    p.edition_year = 1999;

    refuse(
        &ecrire(&bac, p).await,
        EditionErrorCode::YearRange,
        "edition_year",
    );
}

/// `ck_events_coordinates` → `coordinates`. **On nomme celle qui a été donnée
/// seule** : c'est elle que l'écran doit marquer, l'autre étant simplement
/// absente.
#[tokio::test]
async fn des_coordonnees_incompletes_se_refusent_sur_celle_qui_est_donnee() {
    let bac = Bac::monter().await;

    let mut p = formulaire("latitude-seule", "Édition à demi située");
    p.latitude = Some(-1.455_754);
    refuse(
        &ecrire(&bac, p).await,
        EditionErrorCode::Coordinates,
        "latitude",
    );

    let mut p = formulaire("longitude-seule", "Édition à demi située");
    p.longitude = Some(-48.503_887);
    refuse(
        &ecrire(&bac, p).await,
        EditionErrorCode::Coordinates,
        "longitude",
    );
}

/// **Un point complet passe.** Sans ce contrôle, une traduction trop large
/// aurait pu refuser les coordonnées valides sans que rien ne le montre.
#[tokio::test]
async fn un_point_complet_est_accepte() {
    let bac = Bac::monter().await;

    let mut p = formulaire("point-complet", "Édition située");
    p.latitude = Some(-1.455_754);
    p.longitude = Some(-48.503_887);

    let r = ecrire(&bac, p).await;
    assert!(r.ok, "{:?}", r.errors);
    let edition = r.edition.expect("l'édition");
    assert_eq!(edition.latitude, Some(-1.455_754));
    assert_eq!(edition.longitude, Some(-48.503_887));
}

/// **Une adresse mal formée est refusée par le DOMAINE, dont le nom de
/// contrainte est celui du domaine et non de la colonne.** C'est le seul refus
/// de cette liste qui ne peut pas se lire sur le nom de contrainte : il se lit
/// sur le nom de type.
#[tokio::test]
async fn une_adresse_mal_formee_se_refuse_sur_son_champ() {
    let bac = Bac::monter().await;

    let p = formulaire("Adresse Avec Majuscules", "Édition mal adressée");
    refuse(&ecrire(&bac, p).await, EditionErrorCode::Required, "slug");
}

/// **Ce que le contrat n'exprime pas sort en erreur HTTP.** Une série inconnue
/// n'a aucun code dans `EditionFormError` : la forcer dedans aurait obligé à en
/// inventer un.
#[tokio::test]
async fn une_serie_inconnue_sort_en_erreur_http_et_non_en_refus_de_formulaire() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let mut p = formulaire("serie-inconnue", "Édition sans série");
    p.series_id = Some(uuid::Uuid::now_v7());

    let erreur = edition_write::creer(&bac.state, &bac.ctx(), acteur, p)
        .await
        .expect_err("une référence inconnue n'est pas un refus de formulaire");

    assert_eq!(erreur.code, kernel::error::ErrorCode::EventUnknownReference);
    assert_eq!(erreur.field.as_deref(), Some("series_id"));
}

/// **Un fuseau que la base de fuseaux de PostgreSQL ne connaît pas** est une
/// référence inconnue, pas une faute de saisie. C'est le même dictionnaire qui
/// vérifie ici et qui convertira les jours civils.
#[tokio::test]
async fn un_fuseau_inconnu_est_refuse_par_la_base_de_fuseaux() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let mut p = formulaire("fuseau-inconnu", "Édition hors du temps");
    p.timezone = "Mars/Olympus_Mons".to_owned();

    let erreur = edition_write::creer(&bac.state, &bac.ctx(), acteur, p)
        .await
        .expect_err("un fuseau inconnu est refusé");

    assert_ne!(
        erreur.code,
        kernel::error::ErrorCode::Internal,
        "un fuseau mal saisi n'est pas une panne : {erreur}"
    );
}
