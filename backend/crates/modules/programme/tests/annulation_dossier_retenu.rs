//! **Un dossier retenu puis annulé retire ses activités du programme public.**
//!
//! Ses séances publiées et non closes passent « annulées » avec le motif du
//! dossier s'il en a un, dans la transaction de la décision : le déclencheur émet
//! `programme.session.cancelled`, que le direct et les avis aux inscrits
//! consomment. Une séance terminée ne bouge pas. Et une séance non publiée ne
//! prend aucune inscription, même par une adresse fabriquée.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::auth::Perimeter;
use kernel::error::ErrorCode;
use programme::domain::ids::ProposalId;
use programme::domain::registration::IssueDInscription;
use programme::domain::transitions::ProposalStatus;
use programme::repo::sessions::CancellationImpact;
use programme::service::transition;
use uuid::Uuid;

const MOTIF: &str = "L'organisation renonce à l'activité.";

/// Un dossier retenu, dont la séance est publiée et ouverte aux inscriptions.
async fn dossier_retenu_publie(bac: &Bac, terrain: &Terrain, slug: &str) -> (Uuid, Uuid) {
    let dossier = seances::dossier_pret(bac, terrain, slug, slug, Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();

    let seance = seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id;
    seances::ouvrir_les_inscriptions(bac, seance, None, false).await;
    sqlx::query!(
        "UPDATE programme.sessions SET status = 'scheduled' WHERE id = $1",
        seance
    )
    .execute(bac.pool())
    .await
    .unwrap();
    (dossier.id, seance)
}

async fn annuler(bac: &Bac, dossier: Uuid, motif: Option<&str>) {
    let issue = transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.into(),
        ProposalStatus::Cancelled,
        motif,
    )
    .await
    .expect("la décision aboutit");
    assert!(matches!(issue, transition::Issue::Appliquee(_)));
}

async fn decideur(bac: &Bac, terrain: &Terrain) -> Perimeter {
    let droits = commun::droits(bac, terrain).await;
    commun::perimetre_de(bac, droits.decideur).await
}

async fn impact(bac: &Bac, perimetre: &Perimeter, dossier: Uuid) -> CancellationImpact {
    transition::impact_de_lannulation(&bac.state, perimetre, ProposalId(dossier))
        .await
        .expect("l'aperçu se lit")
}

async fn motif_de_la_seance(bac: &Bac, seance: Uuid) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT cancelled_reason ->> 'fr' FROM programme.sessions WHERE id = $1",
        seance
    )
    .fetch_one(bac.pool())
    .await
    .unwrap()
}

async fn inscrire(
    bac: &Bac,
    seance: Uuid,
    courriel: &str,
) -> kernel::error::Result<IssueDInscription> {
    let personne = commun::personne(bac, courriel, "Awa", "Diallo").await;
    seances::sinscrire(bac, seance, Some(personne), seances::reponses_valides()).await
}

/// 🔴 La séance publiée passe annulée avec le motif, un seul
/// `programme.session.cancelled` part, et l'aperçu comptait juste.
#[tokio::test]
async fn annuler_un_dossier_retenu_annule_sa_seance_publiee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let perimetre = decideur(&bac, &terrain).await;
    let (dossier, seance) = dossier_retenu_publie(&bac, &terrain, "retenu").await;
    inscrire(&bac, seance, "une@example.org").await.unwrap();
    inscrire(&bac, seance, "deux@example.org").await.unwrap();

    assert_eq!(
        impact(&bac, &perimetre, dossier).await,
        CancellationImpact {
            published_sessions: 1,
            registrants: 2,
        }
    );

    annuler(&bac, dossier, Some(MOTIF)).await;

    let apres = seances::seance(&bac, seance).await;
    assert_eq!(apres.status, "cancelled");
    assert!(
        apres.published_at.is_some(),
        "elle reste au programme, rayée"
    );
    assert_eq!(
        motif_de_la_seance(&bac, seance).await.as_deref(),
        Some(MOTIF)
    );

    let annonces = commun::evenements_emis(&bac, seance)
        .await
        .into_iter()
        .filter(|e| e == "programme.session.cancelled")
        .count();
    assert_eq!(annonces, 1, "le déclencheur émet, et une seule fois");

    assert_eq!(
        impact(&bac, &perimetre, dossier).await,
        CancellationImpact {
            published_sessions: 0,
            registrants: 0,
        },
        "plus rien à annuler"
    );
}

/// Le motif est facultatif depuis le 28/10 : sans lui, la séance s'annule
/// quand même, sans motif affiché.
#[tokio::test]
async fn annuler_sans_motif_annule_quand_meme_la_seance() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let (dossier, seance) = dossier_retenu_publie(&bac, &terrain, "sans-motif").await;
    annuler(&bac, dossier, None).await;

    assert_eq!(seances::seance(&bac, seance).await.status, "cancelled");
    assert_eq!(motif_de_la_seance(&bac, seance).await, None);
    assert!(commun::evenements_emis(&bac, seance)
        .await
        .iter()
        .any(|e| e == "programme.session.cancelled"));
}

/// Une séance terminée a eu lieu : ni l'aperçu ni l'annulation n'y touchent.
#[tokio::test]
async fn une_seance_terminee_nest_pas_touchee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let perimetre = decideur(&bac, &terrain).await;
    let (dossier, seance) = dossier_retenu_publie(&bac, &terrain, "termine").await;
    inscrire(&bac, seance, "une@example.org").await.unwrap();
    sqlx::query!(
        "UPDATE programme.sessions SET status = 'completed' WHERE id = $1",
        seance
    )
    .execute(bac.pool())
    .await
    .unwrap();

    assert_eq!(
        impact(&bac, &perimetre, dossier).await,
        CancellationImpact {
            published_sessions: 0,
            registrants: 0,
        }
    );

    annuler(&bac, dossier, Some(MOTIF)).await;

    assert_eq!(seances::seance(&bac, seance).await.status, "completed");
    assert_eq!(motif_de_la_seance(&bac, seance).await, None);
    assert!(!commun::evenements_emis(&bac, seance)
        .await
        .iter()
        .any(|e| e == "programme.session.cancelled"));
}

/// Une séance non publiée refuse l'inscription avec son message ; publiée, elle
/// la prend.
#[tokio::test]
async fn une_seance_non_publiee_refuse_linscription() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let (_, seance) = dossier_retenu_publie(&bac, &terrain, "cachee").await;
    sqlx::query!(
        "UPDATE programme.sessions SET published_at = NULL WHERE id = $1",
        seance
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let erreur = inscrire(&bac, seance, "forgee@example.org")
        .await
        .expect_err("une adresse fabriquée ne suffit pas");
    assert_eq!(erreur.code, ErrorCode::RegistrationNotAccepted);
    assert_eq!(
        erreur.message,
        "Les inscriptions à cette activité ne sont pas encore ouvertes."
    );

    sqlx::query!(
        "UPDATE programme.sessions SET published_at = now() WHERE id = $1",
        seance
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let issue = inscrire(&bac, seance, "publique@example.org")
        .await
        .expect("publiée, elle prend l'inscription");
    assert!(matches!(issue, IssueDInscription::Registered { .. }));
}
