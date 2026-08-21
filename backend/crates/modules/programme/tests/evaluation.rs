//! **La fiche d'évaluation : ce qui en sort, ce qui n'en sort pas, et ce que
//! la consolidation change.**
//!
//! Quatre choses s'y prouvent, et le voile en est la plus importante :
//!
//! - **le voile, par inspection de la CHARGE UTILE** — pas de l'écran. Ce
//!   n'est pas un filtre : quand il est baissé, la requête des revues des
//!   pairs n'est pas exécutée, et le test le constate en cherchant la note
//!   d'un pair dans la réponse ;
//! - **la consolidation**, dont l'absence est muette : les agrégats rendus
//!   sont comparés à ceux relus en base ;
//! - **l'affectation**, qui garde la notation sans garder la lecture ;
//! - **les trois visibilités**, chacune sur son lecteur, et la demande de
//!   correction qui ressort partagée quoi qu'on ait demandé.

mod commun;

use commun::{Bac, Terrain};
use kernel::auth::{AdminScope, Perimeter};
use kernel::error::ErrorCode;
use programme::domain::desk::EtatDAvancement;
use programme::domain::ids::ProposalId;
use programme::service::comments::{self, PostCommentPayload};
use programme::service::review::{self, RecusalPayload, SaveReviewPayload};
use programme::service::{desk, transition};
use std::collections::BTreeMap;
use uuid::Uuid;

// -----------------------------------------------------------------------------
// La fabrique — un dossier confié à deux membres du comité
// -----------------------------------------------------------------------------

struct Comite {
    dossier: Uuid,
    /// Affectée, n'a pas encore noté : **le voile est baissé pour elle**.
    premiere: Perimeter,
    /// Affectée, a déposé sa revue : elle n'ancre plus personne.
    seconde: Perimeter,
    /// Décide sans noter, **donc n'est pas affectée, donc pas voilée**.
    decideur: Perimeter,
    criteres: Vec<(Uuid, f64, bool)>,
}

async fn perimetre(bac: &Bac, personne: Uuid) -> Perimeter {
    commun::perimetre_de(bac, personne).await
}

async fn noteur(bac: &Bac, terrain: &Terrain, courriel: &str, prenom: &str) -> Uuid {
    let personne = commun::personne(bac, courriel, prenom, "Comite").await;
    commun::attribuer(bac, personne, "reviewer", "event", Some(terrain.edition)).await;
    sqlx::query!(
        "INSERT INTO event.call_reviewers (call_id, person_id) VALUES ($1, $2)",
        terrain.appel,
        personne
    )
    .execute(bac.pool())
    .await
    .expect("inscription au comité");
    personne
}

async fn confier(bac: &Bac, dossier: Uuid, membre: Uuid) {
    sqlx::query!(
        "INSERT INTO programme.review_assignments (proposal_id, reviewer_id) VALUES ($1, $2)",
        dossier,
        membre
    )
    .execute(bac.pool())
    .await
    .expect("affectation");
}

/// La grille par défaut de l'appel : six critères, dont un éliminatoire.
async fn criteres(bac: &Bac, appel: Uuid) -> Vec<(Uuid, f64, bool)> {
    sqlx::query!(
        r#"SELECT id, max_score::float8 AS "max!", is_knockout
             FROM event.review_criteria WHERE call_id = $1 ORDER BY sort_order, code"#,
        appel
    )
    .fetch_all(bac.pool())
    .await
    .expect("grille de l'appel")
    .into_iter()
    .map(|l| (l.id, l.max, l.is_knockout))
    .collect()
}

async fn comite(bac: &Bac, terrain: &Terrain) -> Comite {
    let dossier = commun::dossier(bac, terrain, "Atelier adaptation", "atelier-adaptation").await;
    let premiere = noteur(bac, terrain, "premiere@ifdd.francophonie.org", "Prisca").await;
    let seconde = noteur(bac, terrain, "seconde@ifdd.francophonie.org", "Sophie").await;
    let decideur = commun::personne(bac, "decideur@ifdd.francophonie.org", "Denis", "Kabore").await;
    commun::attribuer(bac, decideur, "admin", "event", Some(terrain.edition)).await;

    confier(bac, dossier, premiere).await;
    confier(bac, dossier, seconde).await;

    Comite {
        dossier,
        premiere: perimetre(bac, premiere).await,
        seconde: perimetre(bac, seconde).await,
        decideur: perimetre(bac, decideur).await,
        criteres: criteres(bac, terrain.appel).await,
    }
}

/// Une charge utile de notation qui pose la même note sur chaque critère.
fn notation(criteres: &[(Uuid, f64, bool)], part: f64, deposer: bool) -> SaveReviewPayload {
    let mut scores = BTreeMap::new();
    for (id, max, _) in criteres {
        scores.insert(*id, (max * part * 100.0).round() / 100.0);
    }

    SaveReviewPayload {
        recommendation: "accept".to_owned(),
        scores,
        comments: BTreeMap::new(),
        strengths: Some("Un sujet bien cadré.".to_owned()),
        weaknesses: None,
        private_note: Some("À suivre en séance.".to_owned()),
        submit: deposer,
    }
}

// -----------------------------------------------------------------------------
// T110 — le voile, par inspection de la charge utile
// -----------------------------------------------------------------------------

/// **Ce qui est masqué n'est pas lu, et la charge utile le prouve.**
///
/// La seconde membre dépose sa revue ; la première ne l'a pas déposée. La fiche
/// de la première ne doit porter **aucune** revue de pair — ni note, ni
/// recommandation, ni nom — mais **doit** porter le décompte : savoir que deux
/// revues existent n'ancre personne, lire leurs notes si.
#[tokio::test]
async fn le_voile_retient_les_revues_des_pairs_mais_pas_leur_decompte() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.seconde,
        ProposalId(comite.dossier),
        notation(&comite.criteres, 0.8, true),
    )
    .await
    .expect("la seconde dépose sa revue");

    let voilee = desk::ouvrir(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
    )
    .await
    .expect("la première ouvre la fiche");

    assert!(voilee.blind_review, "l'appel de la fabrique est en aveugle");
    assert!(voilee.blind_veiled, "affectée et n'ayant pas déposé");
    assert!(
        voilee.peer_reviews.is_empty(),
        "la requête des revues des pairs ne doit pas avoir été exécutée"
    );
    assert_eq!(voilee.veiled_count, 1, "le décompte, lui, est lu");

    // **On balaie la charge utile ENTIÈRE**, et non les champs qu'on soupçonne :
    // un champ oublié dans un type de sortie est exactement le défaut que le
    // voile existe pour rendre impossible.
    let charge = serde_json::to_string(&voilee).expect("sérialisation de la fiche");
    assert!(
        !charge.contains("Un sujet bien cadré."),
        "les points forts d'un pair ne doivent pas sortir"
    );
    assert!(
        !charge.contains("À suivre en séance."),
        "la note personnelle d'un pair ne doit pas sortir"
    );
}

/// **Le voile se lève à la seconde où sa propre revue part**, et il ne se
/// baisse jamais sur qui décide sans noter.
#[tokio::test]
async fn le_voile_se_leve_au_depot_et_ne_touche_pas_qui_decide_sans_noter() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.seconde,
        ProposalId(comite.dossier),
        notation(&comite.criteres, 0.8, true),
    )
    .await
    .expect("la seconde dépose");

    // Un BROUILLON ne lève rien : il ne compte dans aucun agrégat et n'est
    // visible d'aucun pair.
    review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation(&comite.criteres, 0.5, false),
    )
    .await
    .expect("la première enregistre un brouillon");

    let encore_voilee = desk::ouvrir(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
    )
    .await
    .expect("la fiche");
    assert!(
        encore_voilee.blind_veiled,
        "une revue en brouillon ne lève pas le voile"
    );
    assert!(encore_voilee.peer_reviews.is_empty());

    review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation(&comite.criteres, 0.5, true),
    )
    .await
    .expect("la première dépose");

    let levee = desk::ouvrir(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
    )
    .await
    .expect("la fiche");
    assert!(!levee.blind_veiled, "sa revue est déposée");
    assert_eq!(levee.peer_reviews.len(), 1, "elle lit celle de sa collègue");

    // **Qui décide sans noter n'est pas affecté, donc pas voilé** : l'ancrage
    // vise celui qui va poser une note, et masquer les notes à qui doit
    // trancher rendrait la décision impossible.
    let decideur = desk::ouvrir(
        &bac.state,
        &bac.ctx(),
        &comite.decideur,
        ProposalId(comite.dossier),
    )
    .await
    .expect("la fiche du décideur");
    assert!(!decideur.blind_veiled);
    assert_eq!(decideur.peer_reviews.len(), 2, "il lit les deux revues");
    assert!(!decideur.permissions.is_assigned);
    assert!(decideur.permissions.can_decide);
    assert!(
        !decideur.permissions.can_review,
        "le rôle d'administration ne détient pas la permission de noter (écart n° 50)"
    );
}

// -----------------------------------------------------------------------------
// T112 — la consolidation, dont l'absence serait muette
// -----------------------------------------------------------------------------

/// **Les agrégats rendus égalent ceux relus en base.**
///
/// C'est le seul contrôle qui dise quelque chose de l'écart n° 98 : rien
/// n'appelle `refresh_proposal_score()`, et sans appel explicite le classement
/// du comité resterait faux **sans qu'aucune erreur ne le signale**.
#[tokio::test]
async fn les_agregats_rendus_egalent_ceux_relus_en_base() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    let rendu = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation(&comite.criteres, 0.8, true),
    )
    .await
    .expect("dépôt de la revue");

    let relu = sqlx::query!(
        r#"SELECT weighted_score::float8, average_score::float8,
                  review_count, is_knocked_out
             FROM programme.proposals WHERE id = $1"#,
        comite.dossier
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture des agrégats");

    assert_eq!(rendu.review_count, 1);
    assert_eq!(rendu.review_count, relu.review_count);
    assert_eq!(rendu.proposal_weighted_score, relu.weighted_score);
    assert_eq!(rendu.proposal_average_score, relu.average_score);
    assert_eq!(rendu.is_knocked_out, relu.is_knocked_out);
    assert!(
        rendu.proposal_weighted_score.is_some(),
        "sans l'appel à la consolidation, la note resterait nulle — et personne ne le dirait"
    );
    assert!(!rendu.is_knocked_out);

    // Un BROUILLON ne compte dans aucun agrégat : la seconde enregistre sans
    // déposer, le décompte ne bouge pas.
    review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.seconde,
        ProposalId(comite.dossier),
        notation(&comite.criteres, 0.2, false),
    )
    .await
    .expect("brouillon de la seconde");

    let apres = sqlx::query_scalar!(
        "SELECT review_count FROM programme.proposals WHERE id = $1",
        comite.dossier
    )
    .fetch_one(bac.pool())
    .await
    .expect("relecture du décompte");
    assert_eq!(apres, 1, "une revue en brouillon ne compte pas");
}

/// **Un zéro sur un critère éliminatoire marque le dossier.**
///
/// Et c'est pourquoi une note **absente** n'est pas une note à zéro : ne pas
/// avoir encore noté ne disqualifie rien.
#[tokio::test]
async fn un_zero_sur_un_critere_eliminatoire_marque_le_dossier() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    let mut charge = notation(&comite.criteres, 0.8, true);
    let eliminatoire = comite
        .criteres
        .iter()
        .find(|(_, _, ko)| *ko)
        .expect("la grille par défaut porte un critère éliminatoire");
    charge.scores.insert(eliminatoire.0, 0.0);

    let rendu = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        charge,
    )
    .await
    .expect("dépôt de la revue");

    assert!(rendu.is_knocked_out, "zéro sur un éliminatoire disqualifie");
}

// -----------------------------------------------------------------------------
// T111 — la note au-dessus du maximum de SON critère
// -----------------------------------------------------------------------------

#[tokio::test]
async fn une_note_au_dessus_du_maximum_de_son_critere_est_refusee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    let mut charge = notation(&comite.criteres, 0.5, false);
    let (critere, max, _) = comite.criteres[0];
    charge.scores.insert(critere, max + 1.0);

    let refus = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        charge,
    )
    .await
    .expect_err("le déclencheur refuse une note au-dessus du plafond du critère");

    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert!(
        refus.message.contains("maximum") && refus.message.contains("Bornes de cette grille"),
        "le refus reprend le message du déclencheur ET nomme les bornes de la grille — reçu : {}",
        refus.message
    );
    let _ = max;

    // Un critère d'un autre appel est refusé **avant** la base, en le nommant :
    // la clé étrangère rendrait un refus qui ne dit pas lequel.
    let mut etrangere = notation(&comite.criteres, 0.5, false);
    let inconnu = Uuid::now_v7();
    etrangere.scores.insert(inconnu, 1.0);
    let refus = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        etrangere,
    )
    .await
    .expect_err("un critère étranger à la grille est refusé");
    assert_eq!(refus.code, ErrorCode::ProposalUnknownReference);
    assert!(refus.message.contains(&inconnu.to_string()));
}

// -----------------------------------------------------------------------------
// T113 — noter exige une affectation ; lire n'en exige pas
// -----------------------------------------------------------------------------

/// **Les deux règles sont décorrélées, et c'est le sujet.**
///
/// Rien ne lie la permission à l'affectation en base : sans ce contrôle, un
/// membre du comité noterait n'importe quel dossier de son édition.
#[tokio::test]
async fn noter_exige_une_affectation_mais_lire_nen_exige_pas() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    let non_affectee = noteur(&bac, &terrain, "libre@ifdd.francophonie.org", "Lina").await;
    let perimetre_libre = perimetre(&bac, non_affectee).await;

    // Elle LIT le dossier sans difficulté…
    let fiche = desk::ouvrir(
        &bac.state,
        &bac.ctx(),
        &perimetre_libre,
        ProposalId(comite.dossier),
    )
    .await
    .expect("un membre du comité lit un dossier qu'on ne lui a pas confié");
    assert!(fiche.permissions.can_review);
    assert!(!fiche.permissions.is_assigned);
    assert!(fiche.my_review.assignment.is_none());

    // …et ne peut pas le noter.
    let refus = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &perimetre_libre,
        ProposalId(comite.dossier),
        notation(&comite.criteres, 0.5, false),
    )
    .await
    .expect_err("noter sans affectation est refusé");
    assert_eq!(refus.code, ErrorCode::ProposalReviewNotAssigned);
}

// -----------------------------------------------------------------------------
// T115 — le déport, et la pièce sans adresse
// -----------------------------------------------------------------------------

/// **Un déport sans motif est refusé** : la colonne existe pour tracer
/// l'impartialité du comité, et un déport sans motif ne se relit pas six mois
/// plus tard.
#[tokio::test]
async fn un_deport_sans_motif_est_refuse_et_ferme_la_notation() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    let refus = review::se_deporter(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        RecusalPayload {
            reason: "   ".to_owned(),
        },
    )
    .await
    .expect_err("un motif d'espaces n'est pas un motif");
    assert_eq!(refus.code, ErrorCode::ValidationFailed);
    assert_eq!(refus.field.as_deref(), Some("reason"));

    let affectation = review::se_deporter(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        RecusalPayload {
            reason: "Je siège au conseil de cette organisation.".to_owned(),
        },
    )
    .await
    .expect("le déport motivé aboutit");

    // **Le déport n'efface pas l'affectation : il la date.**
    assert!(affectation.recused_at.is_some());
    assert!(affectation.recusal_reason.is_some());

    let refus = review::enregistrer(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
        notation(&comite.criteres, 0.5, false),
    )
    .await
    .expect_err("on ne note pas un dossier dont on s'est déporté");
    assert_eq!(refus.code, ErrorCode::ProposalReviewNotAssigned);

    let fiche = desk::ouvrir(
        &bac.state,
        &bac.ctx(),
        &comite.premiere,
        ProposalId(comite.dossier),
    )
    .await
    .expect("la fiche reste lisible");
    assert!(fiche.permissions.is_recused);
    assert!(!fiche.permissions.is_assigned);
    assert!(
        !fiche.blind_veiled,
        "un membre déporté ne posera plus de note : il n'y a rien à ancrer"
    );

    let sienne = fiche
        .committee
        .iter()
        .find(|m| m.assignment.reviewer_id == comite.premiere.person_id)
        .expect("son affectation reste au tableau du comité");
    assert_eq!(sienne.state, EtatDAvancement::Recused);
}

/// **Une pièce en quarantaine est rendue sans adresse.**
///
/// Le comité doit savoir qu'une pièce manque à son dossier, pas cliquer sur un
/// lien mort : c'est la nullité de l'adresse qui commande l'avertissement
/// plutôt que le bouton.
#[tokio::test]
async fn une_piece_en_quarantaine_est_rendue_sans_adresse() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;

    for (nom, statut, verdict) in [
        ("propre.pdf", "ready", "clean"),
        ("suspect.pdf", "quarantined", "infected"),
    ] {
        let asset = sqlx::query_scalar!(
            r#"INSERT INTO media.assets
                   (bucket, object_key, checksum_sha256, mime_type, byte_size,
                    original_filename, owner_organization_id, status, scan_verdict)
               VALUES ('epavillon', '2027/03/' || gen_random_uuid()::text || '/' || $1,
                       md5(gen_random_uuid()::text) || md5(gen_random_uuid()::text),
                       'application/pdf', 1024, $1, $2,
                       $3::text::media.asset_status, $4::text::media.scan_verdict)
            RETURNING id"#,
            nom,
            terrain.organisation,
            statut,
            verdict
        )
        .fetch_one(bac.pool())
        .await
        .expect("insertion de l'objet stocké");

        sqlx::query!(
            r#"INSERT INTO programme.proposal_documents (proposal_id, asset_id, title)
               VALUES ($1, $2, jsonb_build_object('fr', $3::text))"#,
            comite.dossier,
            asset,
            nom
        )
        .execute(bac.pool())
        .await
        .expect("rattachement de la pièce");
    }

    let fiche = desk::ouvrir(
        &bac.state,
        &bac.ctx(),
        &comite.decideur,
        ProposalId(comite.dossier),
    )
    .await
    .expect("la fiche");

    assert_eq!(fiche.documents.len(), 2);
    let servie = fiche
        .documents
        .iter()
        .find(|d| d.asset.as_ref().is_some_and(|a| a.status == "ready"))
        .expect("la pièce servie");
    assert!(servie.url.is_some());

    let quarantaine = fiche
        .documents
        .iter()
        .find(|d| {
            d.asset
                .as_ref()
                .is_some_and(|a| a.scan_verdict == "infected")
        })
        .expect("la pièce en quarantaine");
    assert!(
        quarantaine.url.is_none(),
        "une pièce non servie n'a pas d'adresse — c'est ce qui commande l'avertissement"
    );
}

// -----------------------------------------------------------------------------
// T114 — les trois visibilités, et la demande de correction forcée
// -----------------------------------------------------------------------------

/// **Chaque visibilité sur son lecteur, filtrée à la source.**
#[tokio::test]
async fn les_trois_visibilites_ne_franchissent_que_ce_quelles_doivent() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;
    let dossier = ProposalId(comite.dossier);

    for (visibilite, corps) in [
        ("committee", "Délibération interne."),
        ("submitter", "Merci de préciser le format."),
        ("private", "Ma note à moi."),
    ] {
        comments::ecrire(
            &bac.state,
            &bac.ctx(),
            comite.premiere.person_id,
            dossier,
            PostCommentPayload {
                parent_id: None,
                visibility: Some(visibilite.to_owned()),
                body: corps.to_owned(),
                is_change_request: false,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("écriture du message {visibilite} : {e}"));
    }

    // L'AUTRICE voit les trois : le fil du comité, le message partagé, sa note.
    let sien = comments::fil_de(&bac.state, comite.premiere.person_id, dossier)
        .await
        .expect("son propre fil");
    assert_eq!(sien.len(), 3);

    // UNE COLLÈGUE ne voit pas la note personnelle de la première.
    let collegue = comments::fil_de(&bac.state, comite.seconde.person_id, dossier)
        .await
        .expect("le fil de la collègue");
    assert_eq!(collegue.len(), 2);
    assert!(!collegue.iter().any(|m| m.visibility == "private"));

    // LE DÉPOSANT ne voit que ce qui lui est adressé.
    let deposante = comments::fil_de(&bac.state, terrain.deposante, dossier)
        .await
        .expect("le fil de la déposante");
    assert_eq!(deposante.len(), 1);
    assert_eq!(deposante[0].visibility, "submitter");
    assert!(!deposante[0].body.contains("Délibération"));
}

/// **Une demande de correction écrite « comité » ressort partagée** (écart
/// n° 99).
///
/// Les deux colonnes sont indépendantes en base. Une demande que le déposant ne
/// verrait pas bloquerait son dossier sans qu'il sache pourquoi.
#[tokio::test]
async fn une_demande_de_correction_est_forcee_en_visibilite_partagee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;
    let dossier = ProposalId(comite.dossier);

    let message = comments::ecrire(
        &bac.state,
        &bac.ctx(),
        comite.premiere.person_id,
        dossier,
        PostCommentPayload {
            parent_id: None,
            visibility: Some("committee".to_owned()),
            body: "Le résumé ne dit pas ce que l'atelier produit.".to_owned(),
            is_change_request: true,
        },
    )
    .await
    .expect("la demande de correction");

    assert_eq!(message.visibility, "submitter", "forcée en partagé");
    assert!(message.is_change_request);

    // **Un seul événement, et sur le message partagé seulement.**
    let emis = commun::evenements_emis(&bac, message.id).await;
    assert_eq!(emis, vec!["programme.comment.shared".to_owned()]);

    // Une réponse du DÉPOSANT est toujours partagée, et jamais une demande.
    let reponse = comments::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        dossier,
        PostCommentPayload {
            parent_id: Some(message.id),
            visibility: Some("private".to_owned()),
            body: "C'est corrigé.".to_owned(),
            is_change_request: true,
        },
    )
    .await
    .expect("la réponse de la déposante");
    assert_eq!(reponse.visibility, "submitter");
    assert!(
        !reponse.is_change_request,
        "une organisation ne se demande pas des corrections à elle-même"
    );

    // Un message de COMITÉ n'émet rien : il ne sort pas du comité.
    let interne = comments::ecrire(
        &bac.state,
        &bac.ctx(),
        comite.premiere.person_id,
        dossier,
        PostCommentPayload {
            parent_id: None,
            visibility: Some("committee".to_owned()),
            body: "Avis partagé en séance.".to_owned(),
            is_change_request: false,
        },
    )
    .await
    .expect("le message de comité");
    assert!(commun::evenements_emis(&bac, interne.id).await.is_empty());
}

// -----------------------------------------------------------------------------
// La fiche entière — l'accusé de lecture et le périmètre
// -----------------------------------------------------------------------------

/// **La réponse dit l'état d'AVANT la visite.**
///
/// La fonction du modèle insère ou incrémente sans distinguer : lue après
/// l'appel, elle dirait toujours « déjà vu », et l'écran ne pourrait plus
/// signaler un dossier qu'on découvre.
#[tokio::test]
async fn laccuse_de_lecture_est_pose_et_la_reponse_dit_letat_davant() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;
    let dossier = ProposalId(comite.dossier);

    let premiere_visite = desk::ouvrir(&bac.state, &bac.ctx(), &comite.decideur, dossier)
        .await
        .expect("la première ouverture");
    assert!(premiere_visite.first_visit);
    assert_eq!(premiere_visite.read_count, 1);

    let seconde_visite = desk::ouvrir(&bac.state, &bac.ctx(), &comite.decideur, dossier)
        .await
        .expect("la seconde ouverture");
    assert!(!seconde_visite.first_visit);
    assert_eq!(
        seconde_visite.read_count, 1,
        "le compteur est COLLECTIF : la même personne ne compte qu'une fois"
    );
}

/// Un dossier hors périmètre se refuse comme un inexistant, et un périmètre
/// vide se refuse **explicitement**.
#[tokio::test]
async fn la_fiche_est_bornee_par_le_perimetre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;
    let autre_edition = commun::edition_secondaire(&bac).await;

    let ailleurs = commun::personne(&bac, "ailleurs@ifdd.francophonie.org", "Alix", "Loin").await;
    commun::attribuer(&bac, ailleurs, "admin", "event", Some(autre_edition)).await;
    let perimetre_ailleurs = commun::perimetre_de(&bac, ailleurs).await;

    let hors = desk::ouvrir(
        &bac.state,
        &bac.ctx(),
        &perimetre_ailleurs,
        ProposalId(comite.dossier),
    )
    .await
    .expect_err("un dossier hors périmètre se refuse");
    let inexistant = desk::ouvrir(
        &bac.state,
        &bac.ctx(),
        &perimetre_ailleurs,
        ProposalId(Uuid::now_v7()),
    )
    .await
    .expect_err("un dossier inexistant se refuse");
    assert_eq!(hors.code, inexistant.code);
    assert_eq!(hors.message, inexistant.message);
    assert_eq!(hors.code, ErrorCode::NotFound);

    let quidam = commun::personne(&bac, "quidam@example.org", "Quid", "Am").await;
    let vide = Perimeter {
        person_id: quidam,
        scope: AdminScope {
            is_global: false,
            event_ids: Vec::new(),
        },
    };
    let refus = desk::ouvrir(&bac.state, &bac.ctx(), &vide, ProposalId(comite.dossier))
        .await
        .expect_err("un périmètre vide se refuse explicitement");
    assert_eq!(refus.code, ErrorCode::Forbidden);
}

/// La fiche porte les actions offertes **à ce lecteur** — l'en-tête en a besoin
/// sans requête de plus.
#[tokio::test]
async fn la_fiche_porte_les_transitions_offertes_a_son_lecteur() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let comite = comite(&bac, &terrain).await;
    let dossier = ProposalId(comite.dossier);

    // Le dossier est en brouillon : il faut le déposer pour que le comité ait
    // quoi que ce soit à décider.
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier,
        programme::domain::transitions::ProposalStatus::Submitted,
        None,
    )
    .await
    .expect("dépôt");

    let fiche = desk::ouvrir(&bac.state, &bac.ctx(), &comite.decideur, dossier)
        .await
        .expect("la fiche du décideur");
    let offertes: Vec<&str> = fiche
        .available_transitions
        .iter()
        .map(|t| t.to_status.as_str())
        .collect();
    assert!(
        offertes.contains(&"under_review"),
        "le décideur peut mettre en évaluation — reçu : {offertes:?}"
    );

    let noteuse = desk::ouvrir(&bac.state, &bac.ctx(), &comite.premiere, dossier)
        .await
        .expect("la fiche de la noteuse");
    let offertes: Vec<&str> = noteuse
        .available_transitions
        .iter()
        .map(|t| t.to_status.as_str())
        .collect();
    assert!(
        offertes.contains(&"changes_requested"),
        "la noteuse demande des corrections — reçu : {offertes:?}"
    );
}
