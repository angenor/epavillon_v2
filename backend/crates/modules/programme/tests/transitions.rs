//! **La machine à états, lue et jamais réécrite.**
//!
//! Quatorze chemins déclarés en données ; le code n'en recopie aucun. Ce qui est
//! éprouvé ici est ce que le service fait de ces règles : les **offrir** selon
//! qui regarde, et **tenter** sans rejouer le graphe.
//!
//! Deux vérifications sortent du lot, et chacune répond à un défaut qui ne se
//! verrait pas autrement :
//!
//! - **une transition acceptée écrit UNE ligne d'outbox, pas deux.** Le
//!   déclencheur émet déjà ; un service zélé enverrait tout en double, et le
//!   doublon ne se verrait qu'en production ;
//! - **le journal garde chaque motif, la colonne n'en garde qu'un.** Une
//!   transition suivante l'écrase, et une transition sans motif l'efface.

mod commun;

use commun::{Bac, Droits, Terrain};
use programme::domain::ids::ProposalId;
use programme::domain::transitions::ProposalStatus;
use programme::repo::transitions as repo;
use programme::service::transition::{self, ChangeStatusPayload, Issue, RaisonDEcart};

/// Un dossier **déposé**, prêt à être arbitré.
async fn dossier_depose(bac: &Bac, terrain: &Terrain) -> ProposalId {
    let mut brouillon = commun::brouillon(terrain, "Atelier adaptation");
    brouillon.speakers = vec![commun::intervenant("awa.sow@example.org", "Awa", "Sow")];

    let cree = programme::service::draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(terrain, brouillon.clone()),
    )
    .await
    .expect("création du brouillon");

    programme::service::submit::deposer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        ProposalId(cree.proposal_id),
        commun::charge(terrain, brouillon),
    )
    .await
    .expect("dépôt");

    ProposalId(cree.proposal_id)
}

/// Le contexte d'écriture d'un acteur donné : c'est lui qui pose `app.actor_id`,
/// donc l'auteur de la ligne de journal.
fn ctx_de(bac: &Bac, acteur: uuid::Uuid) -> kernel::context::RequestContext {
    bac.ctx().with_actor(acteur)
}

fn cibles(offertes: &[programme::domain::transitions::AvailableTransition]) -> Vec<&'static str> {
    offertes.iter().map(|t| t.to_status.as_str()).collect()
}

// -----------------------------------------------------------------------------
// T073 — le même dossier vu par trois personnes de droits différents
// -----------------------------------------------------------------------------

#[tokio::test]
async fn trois_lecteurs_voient_trois_menus_differents() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let Droits {
        deposante,
        noteur,
        decideur,
    } = commun::droits(&bac, &terrain).await;

    let dossier = dossier_depose(&bac, &terrain).await;
    // Le décideur met le dossier en évaluation : c'est l'état où les trois
    // menus divergent le plus.
    transition::tenter(
        &bac.state,
        &ctx_de(&bac, decideur),
        dossier,
        ProposalStatus::UnderReview,
        None,
    )
    .await
    .expect("mise en évaluation");

    // La DÉPOSANTE : elle peut retirer, et rien d'autre. Le retrait ne nomme
    // aucune permission — c'est `allowed_for_owner` qui l'ouvre, et le tester
    // par la permission le rendrait impossible.
    let siennes = repo::offertes(bac.pool(), dossier, deposante)
        .await
        .expect("transitions de la déposante");
    assert_eq!(cibles(&siennes), vec!["withdrawn"]);
    assert!(
        siennes[0].requires_reason,
        "un retrait en évaluation exige un motif"
    );

    // Le NOTEUR : il demande des corrections, avec motif. Il ne décide pas.
    let siennes = repo::offertes(bac.pool(), dossier, noteur)
        .await
        .expect("transitions du noteur");
    assert_eq!(cibles(&siennes), vec!["changes_requested"]);
    assert!(siennes[0].requires_reason);

    // Le DÉCIDEUR : il retient sans motif, rejette avec motif. **Il ne peut pas
    // demander de corrections** — le rôle d'administration ne détient pas
    // `programme.review.write` (écart n° 50).
    let siennes = repo::offertes(bac.pool(), dossier, decideur)
        .await
        .expect("transitions du décideur");
    assert_eq!(cibles(&siennes), vec!["accepted", "rejected"]);
    assert!(!siennes[0].requires_reason, "retenir n'exige pas de motif");
    assert!(siennes[1].requires_reason, "rejeter exige un motif");
}

#[tokio::test]
async fn un_administrateur_dune_autre_edition_ne_voit_rien() {
    // **La portée est celle de l'ÉDITION du dossier**, pas la portée globale :
    // c'est ce qui fait qu'un responsable détaché sur un webinaire ne décide pas
    // sur la COP31.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let autre_edition = commun::edition_secondaire(&bac).await;

    let etranger = commun::personne(&bac, "ailleurs@ifdd.francophonie.org", "Léa", "Mbaye").await;
    commun::attribuer(&bac, etranger, "admin", "event", Some(autre_edition)).await;

    let dossier = dossier_depose(&bac, &terrain).await;

    let siennes = repo::offertes(bac.pool(), dossier, etranger)
        .await
        .expect("transitions");
    assert!(
        siennes.is_empty(),
        "un administrateur d'une autre édition ne se voit offrir aucune transition"
    );
}

#[tokio::test]
async fn la_table_des_regles_est_rendue_telle_quelle() {
    let bac = Bac::monter().await;

    let regles = repo::regles(bac.pool()).await.expect("les règles");

    // Quatorze chemins déclarés en données. Le compte est écrit : une ligne
    // ajoutée en base doit être un choix, pas une surprise.
    assert_eq!(regles.len(), 14);

    let retrait = regles
        .iter()
        .find(|r| {
            r.from_status == ProposalStatus::Draft && r.to_status == ProposalStatus::Withdrawn
        })
        .expect("brouillon → retiré est déclaré");
    assert!(retrait.allowed_for_owner);
    assert!(retrait.required_permission.is_none());
    assert!(!retrait.requires_reason);
}

// -----------------------------------------------------------------------------
// T074 — UNE ligne d'outbox, pas deux (avertissement n° 1)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn une_transition_acceptee_ecrit_une_seule_ligne_doutbox() {
    // `tg_guard_proposal_status()` émet DÉJÀ. Un service qui émettrait à son
    // tour produirait deux avis de décision par transition — et le doublon ne
    // se verrait qu'en production. **Compter est le seul contrôle qui en dise
    // quelque chose** : vérifier la présence n'en dit rien.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = dossier_depose(&bac, &terrain).await;

    let avant = commun::evenements_emis(&bac, dossier.as_uuid()).await;
    let deja = avant
        .iter()
        .filter(|t| *t == "programme.proposal.under_review")
        .count();
    assert_eq!(deja, 0);

    transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        dossier,
        ProposalStatus::UnderReview,
        None,
    )
    .await
    .expect("mise en évaluation");

    let apres = commun::evenements_emis(&bac, dossier.as_uuid()).await;
    let emis = apres
        .iter()
        .filter(|t| *t == "programme.proposal.under_review")
        .count();
    assert_eq!(
        emis, 1,
        "UNE ligne, pas deux : le déclencheur émet déjà, le service n'émet rien"
    );

    // Et le dépôt, lui aussi, n'a été annoncé qu'une fois.
    let depots = apres
        .iter()
        .filter(|t| *t == "programme.proposal.submitted")
        .count();
    assert_eq!(depots, 1);
}

#[tokio::test]
async fn le_journal_gagne_une_ligne_avec_son_auteur_et_son_motif() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = dossier_depose(&bac, &terrain).await;

    transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        dossier,
        ProposalStatus::UnderReview,
        None,
    )
    .await
    .expect("mise en évaluation");

    let issue = transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        dossier,
        ProposalStatus::Rejected,
        Some("Hors des thématiques de l'édition."),
    )
    .await
    .expect("rejet");

    let ligne = match issue {
        Issue::Appliquee(ligne) => ligne,
        autre => panic!("attendu une transition appliquée, reçu {autre:?}"),
    };
    assert_eq!(ligne.from_status.as_deref(), Some("under_review"));
    assert_eq!(ligne.to_status, "rejected");
    assert_eq!(ligne.actor_id, Some(droits.decideur));
    assert_eq!(
        ligne.reason.as_deref(),
        Some("Hors des thématiques de l'édition.")
    );

    // Quatre lignes : ouverture, dépôt, évaluation, rejet.
    let journal = commun::journal_complet(&bac, dossier.as_uuid()).await;
    assert_eq!(journal.len(), 4);
    assert_eq!(
        journal[0].0, None,
        "la ligne d'ouverture n'a pas d'état de départ"
    );
}

// -----------------------------------------------------------------------------
// T075 — deux refus, deux codes distincts, tous deux en 200
// -----------------------------------------------------------------------------

#[tokio::test]
async fn une_transition_non_declaree_rend_le_message_du_declencheur() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = dossier_depose(&bac, &terrain).await;

    // « déposé → retenu » n'est pas un chemin déclaré : la décision passe par
    // l'évaluation.
    let issue = transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        dossier,
        ProposalStatus::Accepted,
        None,
    )
    .await
    .expect("un refus du garde est une RÉPONSE, pas une erreur");

    match issue {
        Issue::TransitionInterdite(message) => {
            // **Le message français du déclencheur, repris mot pour mot.** Le
            // reformuler produirait deux libellés pour un même refus, et le
            // second se périmerait à la première évolution du SQL.
            assert!(message.contains("Transition interdite"), "{message}");
            assert!(message.contains("submitted"), "{message}");
        }
        autre => panic!("attendu transition_not_allowed, reçu {autre:?}"),
    }

    // Rien n'a bougé, et rien n'a été annoncé.
    assert_eq!(
        commun::ligne(&bac, dossier.as_uuid()).await.status,
        "submitted"
    );
    let emis = commun::evenements_emis(&bac, dossier.as_uuid()).await;
    assert!(!emis.iter().any(|t| t == "programme.proposal.accepted"));
}

#[tokio::test]
async fn un_motif_manquant_rend_un_code_distinct() {
    // Le garde lève `restrict_violation` pour une transition non déclarée et
    // `not_null_violation` pour un motif manquant : **on les distingue par le
    // moment, jamais par le texte**. Trois messages français, dont deux
    // interpolent des valeurs, changeraient à la première reformulation du SQL.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = dossier_depose(&bac, &terrain).await;

    transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        dossier,
        ProposalStatus::UnderReview,
        None,
    )
    .await
    .expect("mise en évaluation");

    // « en évaluation → non retenu » exige un motif.
    let issue = transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        dossier,
        ProposalStatus::Rejected,
        None,
    )
    .await
    .expect("un motif manquant est une réponse");

    assert!(
        matches!(issue, Issue::MotifExige),
        "attendu reason_required, reçu {issue:?}"
    );

    // Un motif d'espaces n'en est pas un — c'est ce que `btrim` vérifie côté
    // déclencheur, et le service ne le devance pas : il l'envoie tel quel.
    let issue = transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        dossier,
        ProposalStatus::Rejected,
        Some("   "),
    )
    .await
    .expect("un motif d'espaces est refusé de la même façon");
    assert!(matches!(issue, Issue::MotifExige));

    assert_eq!(
        commun::ligne(&bac, dossier.as_uuid()).await.status,
        "under_review"
    );
}

// -----------------------------------------------------------------------------
// T076 — le motif écrase la colonne, le journal les garde tous (écart n° 97)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn la_colonne_de_decision_est_ecrasee_mais_le_journal_garde_tout() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = dossier_depose(&bac, &terrain).await;

    // 1. Le comité demande des corrections, avec son motif.
    transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.noteur),
        dossier,
        ProposalStatus::ChangesRequested,
        Some("Préciser le budget."),
    )
    .await
    .expect("demande de correction");
    assert_eq!(
        commun::motif_en_colonne(&bac, dossier.as_uuid())
            .await
            .as_deref(),
        Some("Préciser le budget.")
    );

    // 2. L'organisation renvoie son dossier — **sans motif**. La colonne est
    //    EFFACÉE : c'est le comportement du modèle, et c'est précisément
    //    pourquoi un écran ne doit pas la lire.
    transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.deposante),
        dossier,
        ProposalStatus::Submitted,
        None,
    )
    .await
    .expect("renvoi");
    assert_eq!(
        commun::motif_en_colonne(&bac, dossier.as_uuid()).await,
        None,
        "une transition sans motif efface la colonne"
    );

    // 3. L'organisation retire son dossier, avec son propre motif.
    transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.deposante),
        dossier,
        ProposalStatus::Withdrawn,
        Some("Erreur de dépôt."),
    )
    .await
    .expect("retrait motivé");
    assert_eq!(
        commun::motif_en_colonne(&bac, dossier.as_uuid())
            .await
            .as_deref(),
        Some("Erreur de dépôt.")
    );

    // **Le journal, lui, garde chacun avec son auteur.** C'est lui qu'un écran
    // doit lire : la colonne ne dit que le dernier.
    let journal = commun::journal_complet(&bac, dossier.as_uuid()).await;
    let motifs: Vec<Option<&str>> = journal.iter().map(|l| l.2.as_deref()).collect();
    assert!(motifs.contains(&Some("Préciser le budget.")));
    assert!(motifs.contains(&Some("Erreur de dépôt.")));

    // Et les auteurs sont distincts : le comité a demandé, l'organisation a
    // retiré.
    let demande = journal
        .iter()
        .find(|l| l.1 == "changes_requested")
        .expect("la demande est journalisée");
    assert_eq!(demande.3, Some(droits.noteur));
    let retrait = journal
        .iter()
        .find(|l| l.1 == "withdrawn")
        .expect("le retrait est journalisé");
    assert_eq!(retrait.3, Some(droits.deposante));
}

// -----------------------------------------------------------------------------
// T071 — l'action groupée, évaluée dossier par dossier
// -----------------------------------------------------------------------------

#[tokio::test]
async fn laction_groupee_nomme_chaque_ecart() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;

    // Trois dossiers : un déposé (la transition s'applique), un encore en
    // brouillon (elle ne s'applique pas), et un identifiant qui n'existe pas.
    let depose = dossier_depose(&bac, &terrain).await;
    let brouillon = programme::service::draft_write::enregistrer(
        &bac.state,
        &bac.ctx(),
        terrain.deposante,
        commun::charge(&terrain, commun::brouillon(&terrain, "Encore un brouillon")),
    )
    .await
    .expect("brouillon");
    let inexistant = uuid::Uuid::now_v7();

    let resultat = transition::changer_en_groupe(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        &perimetre,
        ChangeStatusPayload {
            proposal_ids: vec![depose.as_uuid(), brouillon.proposal_id, inexistant],
            to_status: ProposalStatus::UnderReview,
            reason: None,
        },
    )
    .await
    .expect("action groupée");

    assert_eq!(resultat.applied, vec![depose.as_uuid()]);
    assert_eq!(resultat.skipped.len(), 2);

    let ecart = |id: uuid::Uuid| {
        resultat
            .skipped
            .iter()
            .find(|e| e.proposal_id == id)
            .unwrap_or_else(|| panic!("écart attendu pour {id}"))
    };
    assert!(matches!(
        ecart(brouillon.proposal_id).reason,
        RaisonDEcart::TransitionNotAllowed
    ));
    // Le dossier écarté porte son NUMÉRO : c'est ce que l'écran affiche.
    assert!(!ecart(brouillon.proposal_id).reference_code.is_empty());
    assert!(matches!(ecart(inexistant).reason, RaisonDEcart::NotFound));
}

#[tokio::test]
async fn un_dossier_hors_perimetre_est_un_ecart_introuvable_pas_un_refus_global() {
    // Une sélection de douze peut traverser deux éditions. Un dossier hors
    // périmètre rend **le même écart qu'un dossier inexistant** : le refus ne
    // dit pas à qui forge une sélection que le dossier existe ailleurs.
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;

    let autre_edition = commun::edition_secondaire(&bac).await;
    let detache = commun::personne(&bac, "detache@ifdd.francophonie.org", "Détaché", "Test").await;
    commun::attribuer(&bac, detache, "admin", "event", Some(autre_edition)).await;
    let perimetre = commun::perimetre_de(&bac, detache).await;

    let depose = dossier_depose(&bac, &terrain).await;
    let _ = droits;

    let resultat = transition::changer_en_groupe(
        &bac.state,
        &ctx_de(&bac, detache),
        &perimetre,
        ChangeStatusPayload {
            proposal_ids: vec![depose.as_uuid()],
            to_status: ProposalStatus::UnderReview,
            reason: None,
        },
    )
    .await
    .expect("l'action groupée aboutit, c'est le dossier qui est écarté");

    assert!(resultat.applied.is_empty());
    assert!(matches!(resultat.skipped[0].reason, RaisonDEcart::NotFound));
    // Aucun numéro de dossier ne fuit : l'écart ne dit rien de plus qu'un
    // identifiant inexistant.
    assert!(resultat.skipped[0].reference_code.is_empty());
}

#[tokio::test]
async fn laction_groupee_ecarte_ce_qui_exige_un_motif_absent() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let perimetre = commun::perimetre_de(&bac, droits.decideur).await;

    let dossier = dossier_depose(&bac, &terrain).await;
    transition::tenter(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        dossier,
        ProposalStatus::UnderReview,
        None,
    )
    .await
    .expect("mise en évaluation");

    let resultat = transition::changer_en_groupe(
        &bac.state,
        &ctx_de(&bac, droits.decideur),
        &perimetre,
        ChangeStatusPayload {
            proposal_ids: vec![dossier.as_uuid()],
            to_status: ProposalStatus::Rejected,
            reason: None,
        },
    )
    .await
    .expect("action groupée");

    assert!(resultat.applied.is_empty());
    assert!(matches!(
        resultat.skipped[0].reason,
        RaisonDEcart::ReasonRequired
    ));
    // **Écarté AVANT d'y toucher** : rien n'a été tenté, donc rien n'a été
    // annoncé.
    let emis = commun::evenements_emis(&bac, dossier.as_uuid()).await;
    assert!(!emis.iter().any(|t| t == "programme.proposal.rejected"));
}

// -----------------------------------------------------------------------------
// L'accès au dossier — les deux voies, distinctes
// -----------------------------------------------------------------------------

#[tokio::test]
async fn le_journal_se_lit_par_ladhesion_ou_par_le_perimetre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let droits = commun::droits(&bac, &terrain).await;
    let dossier = dossier_depose(&bac, &terrain).await;

    // Voie n° 1 : la déposante, membre actif — **aucun périmètre**.
    let sien = transition::journal_de(&bac.state, droits.deposante, dossier)
        .await
        .expect("la déposante lit le journal de son dossier");
    assert_eq!(sien.len(), 2);

    // Voie n° 2 : le décideur, lecture générale dans le périmètre — membre
    // d'aucune organisation.
    let sien = transition::journal_de(&bac.state, droits.decideur, dossier)
        .await
        .expect("le décideur lit le journal dans son périmètre");
    assert_eq!(sien.len(), 2);

    // Ni l'une ni l'autre : refusé comme un dossier inexistant.
    let etranger = commun::personne(&bac, "etranger@example.org", "Sans", "Droit").await;
    let refus = transition::journal_de(&bac.state, etranger, dossier)
        .await
        .expect_err("ni membre, ni administrateur");
    assert_eq!(refus.code, kernel::error::ErrorCode::NotFound);
}
