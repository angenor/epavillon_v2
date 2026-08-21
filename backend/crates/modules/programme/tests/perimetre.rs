//! **Un dossier hors périmètre se refuse exactement comme un dossier
//! inexistant**, sur les trois niveaux d'ascendance (R13, principe V).
//!
//! Ce que ce test prouve, et que rien d'autre ne prouverait : une URL forgée ne
//! dit pas à qui la forge si l'objet existe. Le refus est le même — même code,
//! même forme, même absence de détail.
//!
//! Les trois niveaux sont éprouvés séparément parce qu'ils empruntent trois
//! requêtes différentes : le dossier remonte d'un saut, le message et la revue
//! de deux.

mod commun;

use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::{CommentId, EventId, ProposalId, ReviewId};
use programme::service::perimeter::{self, Cible};
use uuid::Uuid;

/// Une administratrice détachée sur l'édition secondaire — la COP31 lui est
/// donc étrangère, et c'est la cible naturelle d'une URL forgée.
async fn detachee(bac: &Bac, edition_permise: Uuid) -> kernel::auth::Perimeter {
    let personne = commun::personne(bac, "detache@ifdd.francophonie.org", "Détaché", "Test").await;
    commun::attribuer(bac, personne, "admin", "event", Some(edition_permise)).await;
    commun::perimetre_de(bac, personne).await
}

/// Un message du comité sur un dossier.
async fn message(bac: &Bac, dossier: Uuid, auteur: Uuid) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO programme.proposal_comments (proposal_id, author_id, visibility, body)
         VALUES ($1, $2, 'committee', 'Un point à vérifier.')
      RETURNING id",
        dossier,
        auteur
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion du message")
}

/// Une revue en brouillon sur un dossier.
async fn revue(bac: &Bac, dossier: Uuid, membre: Uuid) -> Uuid {
    sqlx::query_scalar!(
        "INSERT INTO programme.reviews (proposal_id, reviewer_id) VALUES ($1, $2) RETURNING id",
        dossier,
        membre
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la revue")
}

#[tokio::test]
async fn le_dossier_hors_perimetre_se_refuse_comme_un_inexistant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let autre_edition = commun::edition_secondaire(&bac).await;
    let perimetre = detachee(&bac, autre_edition).await;

    let dossier = commun::dossier(&bac, &terrain, "Atelier adaptation", "atelier-adaptation").await;

    let hors_perimetre = perimeter::edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Dossier(ProposalId(dossier)),
    )
    .await
    .expect_err("un dossier de la COP31 doit être refusé à qui n'administre que l'autre édition");

    let inexistant = perimeter::edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Dossier(ProposalId(Uuid::now_v7())),
    )
    .await
    .expect_err("un dossier inexistant doit être refusé");

    // C'EST LA COMPARAISON QUI COMPTE, pas le code pris isolément : deux refus
    // différents diraient à qui forge une URL que le dossier existe.
    assert_eq!(hors_perimetre.code, inexistant.code);
    assert_eq!(hors_perimetre.message, inexistant.message);
    assert_eq!(hors_perimetre.field, inexistant.field);
    assert_eq!(hors_perimetre.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn le_message_remonte_deux_niveaux_avant_de_refuser() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let autre_edition = commun::edition_secondaire(&bac).await;
    let perimetre = detachee(&bac, autre_edition).await;

    let dossier = commun::dossier(&bac, &terrain, "Table ronde", "table-ronde").await;
    let message = message(&bac, dossier, terrain.deposante).await;

    let hors_perimetre = perimeter::edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Message(CommentId(message)),
    )
    .await
    .expect_err("un message d'un dossier hors périmètre doit être refusé");

    let inexistant = perimeter::edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Message(CommentId(Uuid::now_v7())),
    )
    .await
    .expect_err("un message inexistant doit être refusé");

    assert_eq!(hors_perimetre.code, inexistant.code);
    assert_eq!(hors_perimetre.message, inexistant.message);
}

#[tokio::test]
async fn la_revue_remonte_deux_niveaux_avant_de_refuser() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let autre_edition = commun::edition_secondaire(&bac).await;
    let perimetre = detachee(&bac, autre_edition).await;

    let dossier = commun::dossier(&bac, &terrain, "Panel jeunesse", "panel-jeunesse").await;
    let membre = commun::personne(&bac, "membre@ifdd.francophonie.org", "Awa", "Sow").await;
    let revue = revue(&bac, dossier, membre).await;

    let hors_perimetre =
        perimeter::edition_dans_le_perimetre(bac.pool(), &perimetre, Cible::Revue(ReviewId(revue)))
            .await
            .expect_err("une revue d'un dossier hors périmètre doit être refusée");

    let inexistant = perimeter::edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Revue(ReviewId(Uuid::now_v7())),
    )
    .await
    .expect_err("une revue inexistante doit être refusée");

    assert_eq!(hors_perimetre.code, inexistant.code);
    assert_eq!(hors_perimetre.message, inexistant.message);
}

#[tokio::test]
async fn ledition_du_perimetre_est_rendue_et_non_celle_quon_annonce() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let perimetre = detachee(&bac, terrain.edition).await;

    let dossier = commun::dossier(&bac, &terrain, "Atelier eau", "atelier-eau").await;

    let edition = perimeter::edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Dossier(ProposalId(dossier)),
    )
    .await
    .expect("le dossier est dans le périmètre");

    // L'édition vient de l'ASCENDANCE EN BASE, jamais du corps de requête : le
    // front envoie encore `event_id` dans ses charges utiles, et c'est un droit
    // déclaré par le client.
    assert_eq!(edition, EventId(terrain.edition));
}

#[tokio::test]
async fn le_dossier_efface_est_traite_comme_absent() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let perimetre = detachee(&bac, terrain.edition).await;

    let dossier = commun::dossier(&bac, &terrain, "Dossier retiré", "dossier-retire").await;
    sqlx::query!(
        "UPDATE programme.proposals SET deleted_at = now() WHERE id = $1",
        dossier
    )
    .execute(bac.pool())
    .await
    .expect("effacement logique");

    let refus = perimeter::edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Dossier(ProposalId(dossier)),
    )
    .await
    .expect_err("un dossier effacé ne donne plus accès à rien");

    assert_eq!(refus.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn laction_groupee_ecarte_le_dossier_sans_echouer_lensemble() {
    // Une sélection de douze peut traverser deux éditions : le périmètre
    // s'applique à chacune, et un dossier hors périmètre devient un ÉCART de
    // l'action groupée, pas un refus global.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let autre_edition = commun::edition_secondaire(&bac).await;
    let perimetre = detachee(&bac, autre_edition).await;

    let dossier = commun::dossier(&bac, &terrain, "Hors périmètre", "hors-perimetre").await;

    let issue =
        perimeter::edition_si_dans_le_perimetre(bac.pool(), &perimetre, ProposalId(dossier))
            .await
            .expect("la lecture aboutit, c'est le périmètre qui écarte");

    assert!(issue.is_none());
}

#[tokio::test]
async fn la_voie_de_lorganisation_ne_passe_pas_par_le_perimetre() {
    // L'espace organisation est borné par l'ADHÉSION ACTIVE, pas par le
    // périmètre : une organisation n'administre rien. Les deux voies sont
    // distinctes, et ce test le prouve en obtenant l'édition SANS aucun rôle.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let dossier = commun::dossier(&bac, &terrain, "Dossier porté", "dossier-porte").await;

    let edition = perimeter::edition_du_dossier_sans_garde(bac.pool(), ProposalId(dossier))
        .await
        .expect("l'édition du dossier se résout sans périmètre");

    assert_eq!(edition, EventId(terrain.edition));
}

/// Le terrain, une fois monté, doit vraiment permettre de déposer : c'est ce
/// que la fabrique promet, et le seul moyen de le savoir est de le vérifier.
#[tokio::test]
async fn la_fabrique_rend_un_terrain_utilisable() {
    let bac = Bac::monter().await;
    let Terrain {
        edition,
        appel,
        organisation,
        deposante,
    } = commun::terrain(&bac).await;

    let ouvert = sqlx::query_scalar!(r#"SELECT event.is_call_open($1) AS "ouvert!""#, appel)
        .fetch_one(bac.pool())
        .await
        .expect("état de l'appel");
    assert!(ouvert, "l'appel doit être ouvert au sens du modèle");

    let criteres = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM event.review_criteria WHERE call_id = $1"#,
        appel
    )
    .fetch_one(bac.pool())
    .await
    .expect("grille");
    assert_eq!(criteres, 6, "la grille par défaut compte six critères");

    let fuseau = sqlx::query_scalar!(
        r#"SELECT timezone::text AS "tz!" FROM event.events WHERE id = $1"#,
        edition
    )
    .fetch_one(bac.pool())
    .await
    .expect("fuseau");
    assert_eq!(fuseau, commun::FUSEAU_COP31);

    let adhesion = sqlx::query_scalar!(
        r#"SELECT status::text AS "s!" FROM org.memberships
            WHERE organization_id = $1 AND person_id = $2"#,
        organisation,
        deposante
    )
    .fetch_one(bac.pool())
    .await
    .expect("adhésion");
    assert_eq!(adhesion, "active");
}
