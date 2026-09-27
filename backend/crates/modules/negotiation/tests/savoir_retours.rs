//! **Retours et signalements** du lecteur, par le service, sur base réelle :
//! une voix par personne, `from_feedback` une fois, rejeu par `client_ref`,
//! plafond du jour, motifs requis — et l'entrée jamais touchée (FR-017 à FR-019).

mod commun;

use commun::documents::expert;
use commun::savoir::{entree_faq, statut_faq};
use commun::{personne, Bac};
use kernel::error::ErrorCode;
use negotiation::domain::savoir_retours::{FaqFeedbackInput, FaqReportInput, PLAFOND_SIGNALEMENTS};
use negotiation::service::savoir_retours::{mes_voix, signaler, voter};
use time::OffsetDateTime;
use uuid::Uuid;

fn retour(helpful: bool, motif: Option<&str>) -> FaqFeedbackInput {
    serde_json::from_value(serde_json::json!({ "helpful": helpful, "missing_reason": motif }))
        .expect("retour")
}

fn signalement(client_ref: Uuid, motifs: &[&str], details: Option<&str>) -> FaqReportInput {
    serde_json::from_value(
        serde_json::json!({ "client_ref": client_ref, "reasons": motifs, "details": details }),
    )
    .expect("signalement")
}

async fn entree_telle_quelle(bac: &Bac, id: Uuid) -> (String, OffsetDateTime) {
    sqlx::query_as(
        "SELECT status::text || ':' || answer::text, updated_at FROM negotiation.faq_entries WHERE id = $1",
    )
    .bind(id)
    .fetch_one(bac.pool())
    .await
    .expect("entrée")
}

async fn signalements(bac: &Bac, entree: Uuid, qui: Uuid) -> (i64, i64) {
    sqlx::query_as(
        "SELECT count(*) FILTER (WHERE from_feedback), count(*) FILTER (WHERE NOT from_feedback)
           FROM negotiation.faq_reports WHERE entry_id = $1 AND reporter_id = $2",
    )
    .bind(entree)
    .bind(qui)
    .fetch_one(bac.pool())
    .await
    .expect("compte des signalements")
}

#[tokio::test]
async fn une_personne_ne_compte_quune_fois_et_la_derniere_voix_gagne() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let faq = entree_faq(
        &bac,
        experte,
        "Qu'est-ce qu'un groupe de contact ?",
        "published",
    )
    .await;
    let avant = entree_telle_quelle(&bac, faq).await;
    let ctx = bac.ctx(moussa);

    let (vide, empreinte_vide) = mes_voix(&bac.state, moussa).await.unwrap();
    assert!(vide.feedback.is_empty());

    voter(&bac.state, &ctx, moussa, faq, &retour(true, None))
        .await
        .expect("oui");
    let voix = voter(
        &bac.state,
        &ctx,
        moussa,
        faq,
        &retour(false, Some("too_vague")),
    )
    .await
    .expect("non, trop vague");
    assert!(!voix.helpful);
    assert_eq!(voix.missing_reason.as_deref(), Some("too_vague"));

    let (mes, empreinte) = mes_voix(&bac.state, moussa).await.unwrap();
    assert_eq!(mes.feedback.len(), 1, "une voix, la dernière");
    assert_eq!(mes.feedback[0].missing_reason.as_deref(), Some("too_vague"));
    assert_ne!(empreinte, empreinte_vide);
    assert_eq!(
        signalements(&bac, faq, moussa).await,
        (0, 0),
        "trop vague ne part pas"
    );
    assert_eq!(
        entree_telle_quelle(&bac, faq).await,
        avant,
        "l'entrée n'a pas bougé"
    );

    // Une entrée repassée en brouillon sort des voix rendues.
    statut_faq(&bac, faq, "draft").await;
    assert!(mes_voix(&bac.state, moussa)
        .await
        .unwrap()
        .0
        .feedback
        .is_empty());
}

#[tokio::test]
async fn depassee_ou_fausse_ouvre_un_signalement_une_seule_fois() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let voisine = personne(&bac, "voisine@example.org").await;
    let faq = entree_faq(&bac, experte, "Qui préside ?", "to_review").await;
    let avant = entree_telle_quelle(&bac, faq).await;

    for voix in [
        retour(false, Some("outdated")),
        retour(false, Some("outdated")),
        retour(true, None),
        retour(false, Some("outdated")),
    ] {
        voter(&bac.state, &bac.ctx(moussa), moussa, faq, &voix)
            .await
            .expect("voix");
    }
    assert_eq!(
        signalements(&bac, faq, moussa).await,
        (1, 0),
        "un seul, sans motif"
    );

    // Clos entre-temps, il ne se rouvre pas.
    sqlx::query(
        "UPDATE negotiation.faq_reports SET status = 'closed', outcome = 'confirmed',
                handled_by = $2, handled_at = now() WHERE entry_id = $1",
    )
    .bind(faq)
    .bind(experte)
    .execute(bac.pool())
    .await
    .expect("clôture");
    voter(
        &bac.state,
        &bac.ctx(moussa),
        moussa,
        faq,
        &retour(false, Some("outdated")),
    )
    .await
    .expect("encore");
    assert_eq!(signalements(&bac, faq, moussa).await, (1, 0));

    voter(
        &bac.state,
        &bac.ctx(voisine),
        voisine,
        faq,
        &retour(false, Some("outdated")),
    )
    .await
    .expect("voisine");
    assert_eq!(
        signalements(&bac, faq, voisine).await,
        (1, 0),
        "une par personne"
    );
    assert_eq!(entree_telle_quelle(&bac, faq).await, avant);
}

#[tokio::test]
async fn un_retour_mal_forme_ou_sur_un_brouillon_est_refuse() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let brouillon = entree_faq(&bac, experte, "Brouillon", "draft").await;
    let publiee = entree_faq(&bac, experte, "Publiée", "published").await;
    let ctx = bac.ctx(moussa);

    for (entree, voix, code) in [
        (
            brouillon,
            retour(true, None),
            ErrorCode::NegotiationFaqNotFound,
        ),
        (
            Uuid::now_v7(),
            retour(true, None),
            ErrorCode::NegotiationFaqNotFound,
        ),
        (
            publiee,
            retour(true, Some("too_vague")),
            ErrorCode::ValidationFailed,
        ),
        (
            publiee,
            retour(false, Some("boring")),
            ErrorCode::ValidationFailed,
        ),
    ] {
        let refus = voter(&bac.state, &ctx, moussa, entree, &voix)
            .await
            .unwrap_err();
        assert_eq!(refus.code, code);
    }
    let rien: i64 = sqlx::query_scalar("SELECT count(*) FROM negotiation.faq_feedback")
        .fetch_one(bac.pool())
        .await
        .unwrap();
    assert_eq!(rien, 0);
}

#[tokio::test]
async fn un_signalement_exige_un_motif_et_une_precision_courte() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let faq = entree_faq(&bac, experte, "Publiée", "published").await;
    let brouillon = entree_faq(&bac, experte, "Brouillon", "draft").await;
    let ctx = bac.ctx(moussa);
    let long = "é".repeat(601);

    for (entree, s, code) in [
        (
            faq,
            signalement(Uuid::now_v7(), &[], None),
            ErrorCode::NegotiationReportReasonRequired,
        ),
        (
            faq,
            signalement(Uuid::now_v7(), &["pirate"], None),
            ErrorCode::ValidationFailed,
        ),
        (
            faq,
            signalement(Uuid::now_v7(), &["wrong"], Some(&long)),
            ErrorCode::NegotiationTextTooLong,
        ),
        (
            brouillon,
            signalement(Uuid::now_v7(), &["wrong"], None),
            ErrorCode::NegotiationFaqNotFound,
        ),
    ] {
        assert_eq!(
            signaler(&bac.state, &ctx, moussa, entree, &s)
                .await
                .unwrap_err()
                .code,
            code
        );
    }
    assert_eq!(signalements(&bac, faq, moussa).await, (0, 0));

    let juste = "é".repeat(600);
    let (recu, nouveau) = signaler(
        &bac.state,
        &ctx,
        moussa,
        faq,
        &signalement(
            Uuid::now_v7(),
            &["source_mismatch", "rule_changed"],
            Some(&juste),
        ),
    )
    .await
    .expect("deux motifs, six cents caractères");
    assert!(nouveau);
    let motifs: Vec<String> =
        sqlx::query_scalar("SELECT reasons FROM negotiation.faq_reports WHERE id = $1")
            .bind(recu.id)
            .fetch_one(bac.pool())
            .await
            .unwrap();
    assert_eq!(motifs, ["rule_changed", "source_mismatch"]);
}

#[tokio::test]
async fn un_signalement_rejoue_rend_le_meme_recu_sans_seconde_ligne() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let voisine = personne(&bac, "voisine@example.org").await;
    let faq = entree_faq(&bac, experte, "Publiée", "published").await;
    let avant = entree_telle_quelle(&bac, faq).await;
    let reference = Uuid::now_v7();
    let s = signalement(
        reference,
        &["rule_changed", "wrong"],
        Some("Décision 1/CMA.6"),
    );

    let (premier, nouveau) = signaler(&bac.state, &bac.ctx(moussa), moussa, faq, &s)
        .await
        .expect("premier envoi");
    assert!(nouveau);
    let (second, nouveau) = signaler(&bac.state, &bac.ctx(moussa), moussa, faq, &s)
        .await
        .expect("rejeu");
    assert!(!nouveau, "rejoué : 200, pas 201");
    assert_eq!(second.id, premier.id);
    assert_eq!(signalements(&bac, faq, moussa).await, (0, 1));

    // La même référence chez une autre personne est un autre envoi.
    let (autre, nouveau) = signaler(&bac.state, &bac.ctx(voisine), voisine, faq, &s)
        .await
        .expect("voisine");
    assert!(nouveau && autre.id != premier.id);
    assert_eq!(
        entree_telle_quelle(&bac, faq).await,
        avant,
        "l'entrée n'a pas bougé"
    );
}

#[tokio::test]
async fn le_plafond_du_jour_rend_429_sans_empecher_le_rejeu() {
    let bac = Bac::monter().await;
    let experte = expert(&bac, "experte@example.org").await;
    let moussa = personne(&bac, "moussa@example.org").await;
    let faq = entree_faq(&bac, experte, "Publiée", "published").await;
    let ctx = bac.ctx(moussa);

    // Un signalement venu d'un retour ne compte pas.
    voter(
        &bac.state,
        &ctx,
        moussa,
        faq,
        &retour(false, Some("outdated")),
    )
    .await
    .expect("retour");
    let premier = Uuid::now_v7();
    for i in 0..PLAFOND_SIGNALEMENTS {
        let reference = if i == 0 { premier } else { Uuid::now_v7() };
        signaler(
            &bac.state,
            &ctx,
            moussa,
            faq,
            &signalement(reference, &["wrong"], None),
        )
        .await
        .unwrap_or_else(|e| panic!("signalement {i} : {e:?}"));
    }
    let refus = signaler(
        &bac.state,
        &ctx,
        moussa,
        faq,
        &signalement(Uuid::now_v7(), &["wrong"], None),
    )
    .await
    .unwrap_err();
    assert_eq!(refus.code, ErrorCode::NegotiationReportLimit);
    assert_eq!(refus.code.status().as_u16(), 429);

    let (_, nouveau) = signaler(
        &bac.state,
        &ctx,
        moussa,
        faq,
        &signalement(premier, &["wrong"], None),
    )
    .await
    .expect("le rejeu passe le plafond");
    assert!(!nouveau);

    // Hier ne compte plus.
    sqlx::query("UPDATE negotiation.faq_reports SET created_at = now() - interval '2 days' WHERE reporter_id = $1")
        .bind(moussa)
        .execute(bac.pool())
        .await
        .unwrap();
    signaler(
        &bac.state,
        &ctx,
        moussa,
        faq,
        &signalement(Uuid::now_v7(), &["wrong"], None),
    )
    .await
    .expect("un nouveau jour");
}
