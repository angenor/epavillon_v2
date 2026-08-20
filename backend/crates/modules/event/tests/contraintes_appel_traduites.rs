//! **Les six contraintes nommées d'un appel, chacune sur son champ** — plus les
//! deux règles de grille que le service ajoute.
//!
//! Principe VIII : le service ne redouble aucun de ces invariants. Il laisse la
//! base refuser et **traduit** son refus, en branchant sur le nom de la
//! contrainte et jamais sur le texte du message.
//!
//! `ck_calls_duration_bounds` mérite une mention : elle porte **trois conditions
//! sous un seul nom**. Le service compare les trois valeurs pour désigner le
//! champ le plus probablement fautif — sans réimplémenter la vérification, c'est
//! la base qui a refusé.

mod commun;

use commun::{auteur, formulaire_appel, Bac};
use event::domain::call::{CallErrorCode, CallSaveResult, EditionCallPayload};
use event::domain::ids::EventId;
use event::service::call as service_appel;

async fn ecrire(bac: &Bac, event_id: uuid::Uuid, p: EditionCallPayload) -> CallSaveResult {
    let acteur = auteur(bac).await;
    service_appel::creer(&bac.state, &bac.ctx(), acteur, EventId::from(event_id), p)
        .await
        .expect("un refus de contrainte est une réponse, pas une erreur HTTP")
}

fn refuse(r: &CallSaveResult, code: CallErrorCode, champ: Option<&str>) {
    assert!(!r.ok, "l'écriture devait être refusée");
    assert!(r.call.is_none(), "rien n'a été écrit");
    assert_eq!(r.errors.len(), 1, "un seul refus : {:?}", r.errors);
    assert_eq!(r.errors[0].code, code);
    assert_eq!(r.errors[0].field.as_deref(), champ);
}

/// `ck_calls_window` → `window`, sur `closes_at`.
#[tokio::test]
async fn une_fenetre_inversee_se_refuse_sur_la_cloture() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    p.opens_at = commun::instant("2027-06-01T00:00:00Z");
    p.closes_at = commun::instant("2027-05-01T00:00:00Z");

    refuse(
        &ecrire(&bac, editions.cop31, p).await,
        CallErrorCode::Window,
        Some("closes_at"),
    );
}

/// `ck_calls_extension` → `extension`. Une prolongation qui n'allonge rien n'est
/// pas une prolongation.
#[tokio::test]
async fn une_prolongation_anterieure_a_la_cloture_se_refuse() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    p.extended_until = Some(p.opens_at);

    refuse(
        &ecrire(&bac, editions.cop31, p).await,
        CallErrorCode::Extension,
        Some("extended_until"),
    );
}

/// `ck_calls_speakers` → `speakers`, sur le maximum.
#[tokio::test]
async fn un_maximum_dintervenants_sous_le_minimum_se_refuse() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    p.min_speakers = 4;
    p.max_speakers = 2;

    refuse(
        &ecrire(&bac, editions.cop31, p).await,
        CallErrorCode::Speakers,
        Some("max_speakers"),
    );
}

/// `ck_calls_duration_bounds` → `duration_bounds`. **Trois conditions, un seul
/// nom** : on nomme la durée par défaut quand c'est elle qui sort des bornes,
/// et la borne fautive sinon.
#[tokio::test]
async fn une_duree_par_defaut_hors_bornes_se_refuse_sur_elle_meme() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    p.default_duration_minutes = 300;

    refuse(
        &ecrire(&bac, editions.cop31, p).await,
        CallErrorCode::DurationBounds,
        Some("default_duration_minutes"),
    );
}

#[tokio::test]
async fn une_borne_basse_hors_limite_se_refuse_sur_elle_meme() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    p.min_duration_minutes = 5;
    p.default_duration_minutes = 60;
    p.max_duration_minutes = 150;

    refuse(
        &ecrire(&bac, editions.cop31, p).await,
        CallErrorCode::DurationBounds,
        Some("min_duration_minutes"),
    );
}

/// `ck_calls_daily_window` → `daily_window`, sur la fermeture du pavillon.
#[tokio::test]
async fn une_plage_daccueil_inversee_se_refuse_sur_la_fermeture() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    p.daily_start_time = "18:00:00".to_owned();
    p.daily_end_time = "09:00:00".to_owned();

    refuse(
        &ecrire(&bac, editions.cop31, p).await,
        CallErrorCode::DailyWindow,
        Some("daily_end_time"),
    );
}

/// `ux_calls_code` → `code_taken`. Deux appels d'une même édition ne peuvent
/// partager un code — même quand le premier est annulé : l'unicité du code ne
/// porte pas la même exclusion que la cardinalité.
#[tokio::test]
async fn un_code_deja_pris_sur_ledition_se_refuse_sur_le_code() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let mut premier = formulaire_appel(editions.cop31, "cop31");
    premier.status = "cancelled".to_owned();
    let cree = service_appel::creer(&bac.state, &bac.ctx(), acteur, cop31, premier)
        .await
        .expect("création");
    assert!(cree.ok, "{:?}", cree.errors);

    refuse(
        &ecrire(
            &bac,
            editions.cop31,
            formulaire_appel(editions.cop31, "cop31"),
        )
        .await,
        CallErrorCode::CodeTaken,
        Some("code"),
    );
}

/// **Une grille vide n'évalue rien.** Aucune contrainte ne la refuse : c'est une
/// règle du service, et elle est bornée à l'enregistrement d'une campagne.
#[tokio::test]
async fn une_grille_vide_se_refuse_sans_designer_de_champ() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    p.criteria.clear();

    refuse(
        &ecrire(&bac, editions.cop31, p).await,
        CallErrorCode::CriteriaEmpty,
        None,
    );
}

/// **Deux codes identiques désignent le RANG de la ligne fautive.** L'index
/// `ux_review_criteria` refuserait aussi, mais sans dire quelle ligne de l'écran
/// est en cause — et c'est ce rang que le contrat du front attend.
#[tokio::test]
async fn deux_codes_identiques_designent_le_rang_de_la_seconde_ligne() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    p.criteria = vec![
        commun::critere("relevance", 2.0),
        commun::critere("impact", 1.5),
        commun::critere("relevance", 1.0),
    ];

    let refus = ecrire(&bac, editions.cop31, p).await;
    assert!(!refus.ok);
    assert_eq!(refus.errors.len(), 1, "{:?}", refus.errors);
    assert_eq!(refus.errors[0].code, CallErrorCode::CriterionCodeDuplicate);
    assert_eq!(refus.errors[0].criterion_index, Some(2));
}

/// **Aucun message technique ne franchit la réponse** : la forme du contrat ne
/// porte qu'un code, un champ et un rang.
#[tokio::test]
async fn aucun_nom_de_contrainte_ne_franchit_la_reponse() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let mut p = formulaire_appel(editions.cop31, "cop31");
    p.closes_at = p.opens_at;

    let refus = ecrire(&bac, editions.cop31, p).await;
    let rendu = serde_json::to_string(&refus).expect("sérialisation");

    for fuite in ["ck_calls", "calls_for_proposals", "SQLSTATE", "23514"] {
        assert!(
            !rendu.contains(fuite),
            "« {fuite} » ne doit pas franchir la réponse : {rendu}"
        );
    }
}
