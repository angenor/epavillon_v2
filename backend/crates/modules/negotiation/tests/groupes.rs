//! **« Mon groupe »** — le patron des thématiques : remplacement en bloc,
//! empreinte sur les codes, `412` sur état changé, code inconnu nommé ; la
//! liste vide est un choix, et chaque écriture nomme son auteur.

mod commun;

use commun::{personne, Bac};
use kernel::error::{ErrorCode, Result};
use negotiation::domain::groups::MyGroups;
use negotiation::service::groups::{self, Remplacement};
use uuid::Uuid;

async fn choisir(bac: &Bac, qui: Uuid, codes: &[&str], si: Option<&str>) -> Result<MyGroups> {
    let codes: Vec<String> = codes.iter().map(|c| (*c).to_owned()).collect();
    groups::remplacer(
        &bac.state,
        &bac.ctx(qui),
        Remplacement {
            person_id: qui,
            codes: &codes,
            si_correspond: si,
        },
    )
    .await
}

async fn mes_groupes(bac: &Bac, qui: Uuid) -> MyGroups {
    groups::mes_groupes(&bac.state, qui)
        .await
        .expect("lecture des groupes")
}

async fn lignes(bac: &Bac, qui: Uuid) -> Vec<(String, bool)> {
    sqlx::query_as(
        "SELECT t.code, s.left_at IS NOT NULL
           FROM negotiation.group_subscriptions s
           JOIN reference.taxonomy_terms t ON t.id = s.group_term_id
          WHERE s.person_id = $1 ORDER BY s.followed_at, t.code",
    )
    .bind(qui)
    .fetch_all(bac.pool())
    .await
    .expect("lignes de suivi")
}

#[tokio::test]
async fn nominal_rejeu_et_liste_vide() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    let vide = mes_groupes(&bac, awa).await;
    assert!(vide.groups.is_empty());

    let rendu = choisir(&bac, awa, &["ldc", "african_group", "ldc"], None)
        .await
        .expect("choix");
    let mut attendus = rendu.groups.clone();
    attendus.sort();
    assert_eq!(attendus, ["african_group", "ldc"]);
    assert_eq!(rendu.etag, mes_groupes(&bac, awa).await.etag);
    assert_ne!(rendu.etag, vide.etag);

    let avant = lignes(&bac, awa).await;
    let rejeu = choisir(&bac, awa, &["african_group", "ldc"], Some(&rendu.etag))
        .await
        .expect("rejeu");
    assert_eq!(rejeu.etag, rendu.etag);
    assert_eq!(lignes(&bac, awa).await, avant, "le rejeu n'écrit rien");

    let aucun = choisir(&bac, awa, &[], Some(&rendu.etag))
        .await
        .expect("« aucun groupe » est un choix");
    assert!(aucun.groups.is_empty());
    assert_eq!(aucun.etag, vide.etag);
    assert!(
        lignes(&bac, awa).await.iter().all(|(_, fermee)| *fermee),
        "fermées, jamais supprimées"
    );
}

#[tokio::test]
async fn une_empreinte_perimee_est_refusee_sans_rien_ecrire() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    let de_10h = choisir(&bac, awa, &["aosis"], None)
        .await
        .expect("10 h")
        .etag;
    choisir(&bac, awa, &["grulac"], None).await.expect("11 h");
    let lignes_de_11h = lignes(&bac, awa).await;

    let refus = choisir(&bac, awa, &["aosis", "ldc"], Some(&de_10h))
        .await
        .expect_err("intention en retard");
    assert_eq!(refus.code, ErrorCode::NegotiationGroupsStale);
    assert_eq!(refus.code.status().as_u16(), 412);
    assert_eq!(lignes(&bac, awa).await, lignes_de_11h);
    assert_eq!(mes_groupes(&bac, awa).await.groups, ["grulac"]);
}

#[tokio::test]
async fn un_code_inconnu_ou_d_un_autre_vocabulaire_est_nomme() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    for code in ["g99", "adaptation"] {
        let refus = choisir(&bac, awa, &["ldc", code], None)
            .await
            .expect_err("code refusé");
        assert_eq!(refus.code, ErrorCode::NegotiationGroupUnknown);
        assert_eq!(refus.code.status().as_u16(), 400);
        assert!(refus.message.contains(code), "{}", refus.message);
        assert_eq!(refus.field.as_deref(), Some("groups"));
    }
    assert!(lignes(&bac, awa).await.is_empty());
}

#[tokio::test]
async fn chaque_ecriture_nomme_la_personne() {
    let bac = Bac::monter().await;
    let awa = personne(&bac, "awa.diallo@example.org").await;

    choisir(&bac, awa, &["eu", "eig"], None)
        .await
        .expect("ouverture");
    choisir(&bac, awa, &["eig"], None).await.expect("fermeture");

    let traces: Vec<(String, Option<Uuid>)> = sqlx::query_as(
        "SELECT a.action, a.actor_id
           FROM platform.audit_log a
           JOIN negotiation.group_subscriptions s ON s.id = a.entity_id
          WHERE a.entity_table = 'group_subscriptions' AND s.person_id = $1
          ORDER BY a.occurred_at",
    )
    .bind(awa)
    .fetch_all(bac.pool())
    .await
    .expect("audit");
    assert_eq!(traces.len(), 3, "{traces:?}");
    assert!(traces.iter().all(|(_, acteur)| *acteur == Some(awa)));
}
