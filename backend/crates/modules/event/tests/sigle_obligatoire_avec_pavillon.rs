//! **Écart n° 9 — une édition dont le pavillon est tenu porte un sigle.**
//!
//! Ce que la règle sert : `programme.tg_assign_reference_code()` préfixe le
//! numéro de dossier par le sigle de l'édition, et à défaut par les **huit
//! premiers caractères de l'adresse d'URL**. Une édition `cop31-test` sans
//! sigle produit « COP31-TE-00001 », un numéro qu'aucune organisation ne peut
//! épeler au téléphone.
//!
//! **Les quatre chemins d'écriture sont éprouvés, et le troisième est celui
//! qu'on oublie.** Créer sans sigle ; créer sans pavillon et sans sigle ;
//! **basculer** en pavillon sans en fournir ; **retirer** le sigle d'une édition
//! à pavillon. Un service qui ne vérifierait qu'à la création laisserait passer
//! les deux derniers.

mod commun;

use commun::{auteur, formulaire, serie, Bac};
use event::domain::edition::{EditionErrorCode, EditionSaveResult};
use event::domain::ids::EventId;
use event::service::edition_write;

async fn creer(
    bac: &Bac,
    acteur: uuid::Uuid,
    p: event::domain::edition::EditionFormPayload,
) -> EditionSaveResult {
    edition_write::creer(&bac.state, &bac.ctx(), acteur, p)
        .await
        .expect("l'écriture ne lève pas : elle refuse dans sa réponse")
}

async fn modifier(
    bac: &Bac,
    acteur: uuid::Uuid,
    id: EventId,
    p: event::domain::edition::EditionFormPayload,
) -> EditionSaveResult {
    edition_write::modifier(&bac.state, &bac.ctx(), acteur, id, p)
        .await
        .expect("l'écriture ne lève pas : elle refuse dans sa réponse")
}

fn refuse_sur_le_sigle(r: &EditionSaveResult) {
    assert!(!r.ok, "le refus est un résultat, pas une erreur HTTP");
    assert_eq!(r.errors.len(), 1, "un seul refus : {:?}", r.errors);
    assert_eq!(r.errors[0].code, EditionErrorCode::Required);
    assert_eq!(r.errors[0].field.as_deref(), Some("acronym"));
    assert!(r.edition.is_none(), "rien n'a été écrit");
}

/// Chemin 1 — créer une édition à pavillon **sans sigle**.
#[tokio::test]
async fn creer_avec_pavillon_sans_sigle_est_refuse_avec_une_valeur_proposee() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let mut p = formulaire("cop31-belem", "COP31 — Conférence des Parties");
    p.has_pavilion = true;

    let r = creer(&bac, acteur, p).await;
    refuse_sur_le_sigle(&r);

    // FR-029 : un refus qui ne propose rien fait chercher une convention que
    // personne n'a écrite.
    let propose = r
        .suggested_acronym
        .clone()
        .expect("le refus porte une valeur proposée");
    assert!(
        propose.starts_with("COP31"),
        "la proposition se dérive du libellé : {propose}"
    );
}

/// **La valeur proposée doit être utilisable telle quelle.** Une proposition que
/// la deuxième tentative refuserait ferait tourner l'équipe en rond.
#[tokio::test]
async fn la_valeur_proposee_est_acceptee_sans_retouche() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let mut p = formulaire("cop31-belem", "COP31 — Conférence des Parties");
    p.has_pavilion = true;

    let refus = creer(&bac, acteur, p.clone()).await;
    p.acronym = refus.suggested_acronym.clone();

    let r = creer(&bac, acteur, p).await;
    assert!(r.ok, "la valeur proposée passe : {:?}", r.errors);
}

/// Chemin 2 — avec un sigle, l'écriture passe.
#[tokio::test]
async fn creer_avec_pavillon_et_sigle_est_accepte() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;
    let climat = serie(&bac, "cop_climate").await;

    let mut p = formulaire("cop31-belem", "COP31 — Conférence des Parties");
    p.has_pavilion = true;
    p.acronym = Some("COP31".to_owned());
    p.series_id = Some(climat);
    p.edition_label = Some("COP31".to_owned());

    let r = creer(&bac, acteur, p).await;
    assert!(r.ok, "{:?}", r.errors);
    let edition = r.edition.expect("l'édition telle qu'elle est devenue");
    assert_eq!(edition.acronym.as_deref(), Some("COP31"));
    assert!(edition.has_pavilion);
}

/// Chemin 3 — **le cas PACO** : sans pavillon, aucun sigle n'est réclamé, et
/// aucun n'est inventé d'office (FR-030). C'est ce que la règle doit préserver.
#[tokio::test]
async fn creer_sans_pavillon_et_sans_sigle_est_accepte() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let r = creer(&bac, acteur, formulaire("paco-2027", "Rendez-vous du PACO")).await;

    assert!(r.ok, "{:?}", r.errors);
    let edition = r.edition.expect("l'édition");
    assert_eq!(
        edition.acronym, None,
        "aucun sigle n'est inventé pour une édition qui n'en a pas besoin"
    );
}

/// Chemin 4 — **basculer** en pavillon sans fournir de sigle. La règle porte sur
/// l'état RÉSULTANT, pas sur l'état antérieur.
#[tokio::test]
async fn basculer_en_pavillon_sans_sigle_est_refuse() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let cree = creer(&bac, acteur, formulaire("paco-2027", "Rendez-vous du PACO")).await;
    let id = EventId::from(cree.edition.expect("l'édition").id);

    let mut p = formulaire("paco-2027", "Rendez-vous du PACO");
    p.has_pavilion = true;

    refuse_sur_le_sigle(&modifier(&bac, acteur, id, p).await);
}

/// Chemin 5 — **retirer** le sigle d'une édition à pavillon. C'est celui qu'on
/// oublie le plus souvent, et un service qui ne regarderait que la charge utile
/// entrante le laisserait passer.
#[tokio::test]
async fn retirer_le_sigle_dune_edition_a_pavillon_est_refuse() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    let mut p = formulaire("cop31-belem", "COP31 — Conférence des Parties");
    p.has_pavilion = true;
    p.acronym = Some("COP31".to_owned());

    let cree = creer(&bac, acteur, p.clone()).await;
    let id = EventId::from(cree.edition.expect("l'édition").id);

    p.acronym = None;
    refuse_sur_le_sigle(&modifier(&bac, acteur, id, p.clone()).await);

    // Un sigle réduit à des espaces n'en est pas un.
    p.acronym = Some("   ".to_owned());
    refuse_sur_le_sigle(&modifier(&bac, acteur, id, p).await);
}

/// Les bornes et le jeu de caractères, éprouvés **de bout en bout** : les tests
/// unitaires du domaine tiennent la règle, celui-ci tient le fait qu'elle est
/// bien appliquée par le service, sur le bon champ.
#[tokio::test]
async fn un_sigle_mal_forme_est_refuse_sur_son_champ() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    for (rang, mauvais) in ["A", "TREIZECARACTE", "COP 31", "COP31é", "COP_31"]
        .iter()
        .enumerate()
    {
        let mut p = formulaire(&format!("edition-{rang}"), "COP31 — Conférence");
        p.has_pavilion = true;
        p.acronym = Some((*mauvais).to_owned());

        let r = creer(&bac, acteur, p).await;
        assert!(!r.ok, "« {mauvais} » ne devrait pas passer");
        assert_eq!(
            r.errors[0].field.as_deref(),
            Some("acronym"),
            "« {mauvais} »"
        );
    }
}

/// Les bornes qui **passent** : douze caractères, deux caractères, un tiret.
/// Sans elles, une règle trop stricte se serait cachée derrière les refus.
#[tokio::test]
async fn les_bornes_acceptables_passent() {
    let bac = Bac::monter().await;
    let acteur = auteur(&bac).await;

    for (rang, bon) in ["AB", "COP-31", "DOUZECARACTE"].iter().enumerate() {
        let mut p = formulaire(&format!("acceptable-{rang}"), "COP31 — Conférence");
        p.has_pavilion = true;
        p.acronym = Some((*bon).to_owned());

        let r = creer(&bac, acteur, p).await;
        assert!(r.ok, "« {bon} » devrait passer : {:?}", r.errors);
    }
}
