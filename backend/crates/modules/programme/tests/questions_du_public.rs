//! **Le public questionne une séance publiée** — et la page de la séance dit
//! si elle le permet.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::ids::{EventId, QuestionId, SessionId};
use programme::domain::transitions::ProposalStatus;
use programme::service::{public_schedule, questions, transition};
use uuid::Uuid;

async fn seance(bac: &Bac, terrain: &Terrain, titre: &str, slug: &str) -> Uuid {
    let dossier = seances::dossier_pret(bac, terrain, titre, slug, Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id
}

async fn publier(bac: &Bac, session_id: Uuid) {
    sqlx::query!(
        "UPDATE programme.sessions
            SET published_at = now(), status = 'scheduled',
                description = '{\"fr\":\"Le détail de l''atelier\"}'::jsonb
          WHERE id = $1",
        session_id
    )
    .execute(bac.pool())
    .await
    .expect("publication posée à la main");
}

fn id_de(question: &serde_json::Value) -> QuestionId {
    QuestionId(question["id"].as_str().unwrap().parse().unwrap())
}

#[tokio::test]
async fn la_page_dit_si_la_seance_prend_des_questions() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;
    publier(&bac, id).await;
    let adresse = seances::seance(&bac, id).await.slug;

    let detail = public_schedule::seance(bac.pool(), EventId(terrain.edition), &adresse)
        .await
        .unwrap();

    assert!(detail.allows_questions);
    assert_eq!(detail.description.unwrap()["fr"], "Le détail de l'atelier");
    let intervenant = &detail.speakers[0];
    assert!(
        intervenant.get("avatar").is_some(),
        "la photo, nulle sans image"
    );
    assert!(intervenant.get("person_id").is_none());
    assert!(intervenant.get("email").is_none());
}

#[tokio::test]
async fn une_question_posee_se_lit_sans_son_auteur() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;
    publier(&bac, id).await;
    let auteur = commun::personne(&bac, "public@example.org", "Awa", "Sow").await;

    let posee = questions::poser(
        &bac.state,
        &bac.ctx().with_actor(auteur),
        SessionId(id),
        auteur,
        "  Quel financement pour l'adaptation ?  ",
    )
    .await
    .expect("la question est posée");
    assert_eq!(posee["body"], "Quel financement pour l'adaptation ?");
    assert_eq!(posee["is_mine"], true);

    let anonymes = questions::lire(&bac.state, SessionId(id), None)
        .await
        .unwrap();
    assert_eq!(anonymes.len(), 1);
    assert_eq!(anonymes[0]["is_mine"], false);
    assert_eq!(anonymes[0]["vote_count"], 0);
    assert!(
        anonymes[0].get("person_id").is_none(),
        "l'auteur ne sort pas"
    );

    let trop_courte = questions::poser(
        &bac.state,
        &bac.ctx().with_actor(auteur),
        SessionId(id),
        auteur,
        " ok ",
    )
    .await
    .expect_err("deux caractères ne font pas une question");
    assert_eq!(trop_courte.code, ErrorCode::ValidationFailed);
    assert_eq!(trop_courte.field.as_deref(), Some("body"));
}

#[tokio::test]
async fn une_seance_fermee_ou_non_publiee_refuse_les_questions() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let fermee = seance(&bac, &terrain, "Fermée", "fermee").await;
    let interne = seance(&bac, &terrain, "Interne", "interne").await;
    publier(&bac, fermee).await;
    sqlx::query!(
        "UPDATE programme.sessions SET allows_questions = false WHERE id = $1",
        fermee
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let auteur = commun::personne(&bac, "public@example.org", "Awa", "Sow").await;
    let ctx = bac.ctx().with_actor(auteur);

    let refus = questions::poser(
        &bac.state,
        &ctx,
        SessionId(fermee),
        auteur,
        "Une question ?",
    )
    .await
    .expect_err("la séance ne prend pas de questions");
    assert_eq!(refus.code, ErrorCode::Conflict);

    let cachee = questions::poser(
        &bac.state,
        &ctx,
        SessionId(interne),
        auteur,
        "Une question ?",
    )
    .await
    .expect_err("une séance non publiée n'existe pas pour le public");
    assert_eq!(cachee.code, ErrorCode::NotFound);
    let lecture = questions::lire(&bac.state, SessionId(interne), None)
        .await
        .expect_err("même refus en lecture");
    assert_eq!(lecture.code, ErrorCode::NotFound);
}

#[tokio::test]
async fn un_soutien_par_personne_et_les_plus_soutenues_dabord() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let id = seance(&bac, &terrain, "Atelier", "atelier").await;
    publier(&bac, id).await;
    let auteur = commun::personne(&bac, "public@example.org", "Awa", "Sow").await;
    let votante = commun::personne(&bac, "votante@example.org", "Rokia", "Traoré").await;
    let ctx_auteur = bac.ctx().with_actor(auteur);
    let ctx_votante = bac.ctx().with_actor(votante);
    let seance = SessionId(id);

    let premiere = questions::poser(&bac.state, &ctx_auteur, seance, auteur, "Première question")
        .await
        .unwrap();
    let seconde = id_de(
        &questions::poser(&bac.state, &ctx_auteur, seance, auteur, "Seconde question")
            .await
            .unwrap(),
    );

    let soutenue = questions::voter(&bac.state, &ctx_votante, seance, seconde, votante)
        .await
        .expect("le soutien est pris");
    assert_eq!(soutenue["vote_count"], 1);
    assert_eq!(soutenue["has_voted"], true);

    let double = questions::voter(&bac.state, &ctx_votante, seance, seconde, votante)
        .await
        .expect_err("un seul soutien par personne");
    assert_eq!(double.code, ErrorCode::Conflict);

    let liste = questions::lire(&bac.state, seance, Some(votante))
        .await
        .unwrap();
    assert_eq!(id_de(&liste[0]), seconde, "la plus soutenue d'abord");
    assert_eq!(liste[0]["has_voted"], true);
    assert_eq!(liste[1]["has_voted"], false);

    let upvotes = sqlx::query_scalar!(
        "SELECT upvotes FROM programme.session_questions WHERE id = $1",
        seconde.as_uuid()
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(upvotes, 1, "la colonne de tri suit les soutiens");

    let retiree = questions::retirer_le_vote(&bac.state, &ctx_votante, seance, seconde, votante)
        .await
        .unwrap();
    assert_eq!(retiree["vote_count"], 0);
    questions::retirer_le_vote(&bac.state, &ctx_votante, seance, seconde, votante)
        .await
        .expect("retirer deux fois ne refuse rien");

    sqlx::query!(
        "UPDATE programme.session_questions SET is_visible = false WHERE id = $1",
        id_de(&premiere).as_uuid()
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let masquee = questions::voter(&bac.state, &ctx_votante, seance, id_de(&premiere), votante)
        .await
        .expect_err("une question masquée ne se soutient pas");
    assert_eq!(masquee.code, ErrorCode::NotFound);
    assert_eq!(
        questions::lire(&bac.state, seance, None)
            .await
            .unwrap()
            .len(),
        1,
        "ni ne se lit"
    );
}
