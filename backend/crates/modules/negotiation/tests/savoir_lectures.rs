//! **Les lectures de la FAQ** : un compteur par entrée et par jour, sans auteur,
//! qui nourrit « les plus lues » (R12). Le `204` sans session se prouve en
//! HTTP, dans `crates/api/tests/routes_negotiation_savoir.rs`.

mod commun;

use commun::savoir::{entree_faq, statut_faq};
use commun::{personne, Bac};
use negotiation::service::savoir_lectures::compter_une_lecture;
use negotiation::service::savoir_paquet::paquet;
use uuid::Uuid;

async fn lectures(bac: &Bac, id: Uuid) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(sum(count), 0)::bigint FROM negotiation.faq_reads WHERE entry_id = $1",
    )
    .bind(id)
    .fetch_one(bac.pool())
    .await
    .expect("lectures")
}

#[tokio::test]
async fn une_lecture_se_compte_par_jour_sans_auteur() {
    let bac = Bac::monter().await;
    let expert = personne(&bac, "expert@example.org").await;
    let entree = entree_faq(&bac, expert, "Où retirer mon badge ?", "published").await;

    for _ in 0..3 {
        compter_une_lecture(&bac.state, &bac.ctx_anonyme(), entree)
            .await
            .expect("lecture");
    }
    let (jours, total): (i64, i64) = sqlx::query_as(
        "SELECT count(*), sum(count)::bigint FROM negotiation.faq_reads WHERE entry_id = $1",
    )
    .bind(entree)
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!((jours, total), (1, 3), "une ligne du jour, trois lectures");
}

#[tokio::test]
async fn un_brouillon_ou_une_entree_inconnue_ne_se_compte_pas_et_ne_refuse_pas() {
    let bac = Bac::monter().await;
    let expert = personne(&bac, "expert@example.org").await;
    let brouillon = entree_faq(&bac, expert, "Brouillon", "draft").await;

    compter_une_lecture(&bac.state, &bac.ctx_anonyme(), brouillon)
        .await
        .expect("brouillon : aucun refus");
    compter_une_lecture(&bac.state, &bac.ctx_anonyme(), Uuid::now_v7())
        .await
        .expect("inconnue : aucun refus");
    assert_eq!(lectures(&bac, brouillon).await, 0);
}

#[tokio::test]
async fn une_entree_a_revoir_reste_lue_et_compte() {
    let bac = Bac::monter().await;
    let expert = personne(&bac, "expert@example.org").await;
    let entree = entree_faq(&bac, expert, "Qu'est-ce qu'un document L ?", "published").await;
    statut_faq(&bac, entree, "to_review").await;

    compter_une_lecture(&bac.state, &bac.ctx_anonyme(), entree)
        .await
        .expect("lecture");
    assert_eq!(lectures(&bac, entree).await, 1);
}

#[tokio::test]
async fn les_plus_lues_suivent_les_lectures() {
    let bac = Bac::monter().await;
    let expert = personne(&bac, "expert@example.org").await;
    let peu = entree_faq(&bac, expert, "Peu lue", "published").await;
    let beaucoup = entree_faq(&bac, expert, "Beaucoup lue", "published").await;

    compter_une_lecture(&bac.state, &bac.ctx_anonyme(), peu)
        .await
        .unwrap();
    for _ in 0..2 {
        compter_une_lecture(&bac.state, &bac.ctx_anonyme(), beaucoup)
            .await
            .unwrap();
    }
    let (lu, _) = paquet(&bac.state, "fr", None).await.expect("paquet");
    assert_eq!(lu.most_read, [beaucoup, peu]);
}
