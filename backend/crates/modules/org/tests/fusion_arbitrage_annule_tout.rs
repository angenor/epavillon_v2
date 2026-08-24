//! **Un arbitrage qui échoue annule la fusion entière** (SC-012).
//!
//! Ni fiche absorbée, ni rattachement déplacé, ni ligne au journal, ni événement
//! dans l'outbox. C'est la garantie que cherchait l'obligation d'A11 — « dans la
//! même transaction » —, et elle est **conservée intacte** : seul l'ordre des
//! deux écritures a changé (research.md § R5).
//!
//! ## Comment on fait échouer un arbitrage
//!
//! L'unicité porte sur **(nom normalisé, pays)**. Trois fiches suffisent :
//!
//! - la **source**, « Coalition sahélienne », au Burkina Faso ;
//! - la **cible**, « ROAC Afrique », au Sénégal ;
//! - une **tierce**, « Coalition sahélienne », au Sénégal elle aussi.
//!
//! Rien n'interdit ces trois fiches — les deux homonymes sont dans deux pays
//! différents. Mais arbitrer le nom légal en faveur de la source ferait porter
//! « Coalition sahélienne » à une fiche **du Sénégal**, où le nom est déjà pris.
//! L'arbitrage échoue, et la fusion avec lui.

mod commun;

use commun::{pays, perimetres, personne, Bac};
use org::domain::ids::PersonId;
use org::domain::merge::{MergePayload, MergeSide};
use org::service::merge;
use std::collections::BTreeMap;
use uuid::Uuid;

const NOM_DISPUTE: &str = "Coalition sahélienne";

async fn semer(bac: &Bac) -> (Uuid, Uuid) {
    let senegal = pays(bac, "SEN").await;
    let burkina = pays(bac, "BFA").await;

    let source = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, slug, organization_type_code, country_id, status)
           VALUES ($2, 'coalition-sahelienne-bf'::platform.slug,
                   'ngo_association', $1, 'active')
        RETURNING id"#,
        burkina,
        NOM_DISPUTE
    )
    .fetch_one(bac.pool())
    .await
    .expect("source");

    let cible = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, slug, organization_type_code, country_id, status)
           VALUES ('ROAC Afrique', 'roac-afrique'::platform.slug,
                   'ngo_association', $1, 'active')
        RETURNING id"#,
        senegal
    )
    .fetch_one(bac.pool())
    .await
    .expect("cible");

    // La tierce : même nom que la source, mais dans le pays de la CIBLE.
    sqlx::query!(
        r#"INSERT INTO org.organizations
               (legal_name, slug, organization_type_code, country_id, status)
           VALUES ($2, 'coalition-sahelienne-sn'::platform.slug,
                   'ngo_association', $1, 'active')"#,
        senegal,
        NOM_DISPUTE
    )
    .execute(bac.pool())
    .await
    .expect("tierce fiche");

    (source, cible)
}

#[tokio::test]
async fn un_arbitrage_qui_echoue_ne_laisse_aucune_trace() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let (source, cible) = semer(&bac).await;

    // Une adhésion côté source : si la fusion passait à moitié, elle aurait
    // basculé.
    let awa = personne(&bac, "awa@roac-afrique.org", "Awa", "Sow Fall").await;
    sqlx::query!(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, job_title, approved_at)
         VALUES ($1, $2, 'member', 'active', 'Chargée de projet', now())",
        source,
        awa
    )
    .execute(bac.pool())
    .await
    .expect("adhésion");

    let mut choix = BTreeMap::new();
    choix.insert("legal_name".to_owned(), MergeSide::Source);

    let refus = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: source,
            target_id: cible,
            pair_id: None,
            reason: "doublon".to_owned(),
            field_choices: choix,
            confirmation_name: NOM_DISPUTE.to_owned(),
        },
    )
    .await;

    assert!(
        refus.is_err(),
        "l'arbitrage se heurte à l'unicité (nom, pays) : la fusion échoue avec lui"
    );

    // **Ni fiche absorbée…**
    let source_intacte = org::repo::organizations::by_id(bac.pool(), source.into())
        .await
        .expect("lecture")
        .expect("la source");
    assert_eq!(source_intacte.status.as_str(), "active");
    assert!(source_intacte.merged_into_id.is_none());
    assert_eq!(source_intacte.legal_name, NOM_DISPUTE);

    // **…ni nom repris sur la survivante…**
    let cible_intacte = org::repo::organizations::by_id(bac.pool(), cible.into())
        .await
        .expect("lecture")
        .expect("la cible");
    assert_eq!(cible_intacte.legal_name, "ROAC Afrique");

    // **…ni rattachement déplacé…**
    let adhesion = org::repo::memberships::by_couple(bac.pool(), source.into(), PersonId(awa))
        .await
        .expect("lecture")
        .expect("l'adhésion n'a pas bougé");
    assert_eq!(adhesion.organization_id.as_uuid(), source);

    // **…ni ligne au journal…**
    let fusions = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.merge_log WHERE source_id = $1"#,
        source
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(fusions, 0);

    // **…ni événement dans l'outbox.** La fonction de base l'avait pourtant
    // émis : la transaction annulée l'emporte avec elle. C'est tout l'intérêt de
    // l'outbox transactionnel.
    let emis = commun::evenements_emis(&bac, cible).await;
    assert!(
        emis.is_empty(),
        "la transaction est annulée, l'événement avec : {emis:?}"
    );
}

/// La même fusion **sans l'arbitrage fautif** aboutit : c'est bien l'arbitrage
/// qui a échoué, pas la fusion.
#[tokio::test]
async fn la_meme_fusion_sans_larbitrage_aboutit() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let (source, cible) = semer(&bac).await;

    let issue = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: source,
            target_id: cible,
            pair_id: None,
            reason: "doublon".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: NOM_DISPUTE.to_owned(),
        },
    )
    .await
    .expect("fusion");

    assert!(matches!(
        issue,
        org::domain::merge::MergeOutcome::Merged { .. }
    ));

    let absorbee = org::repo::organizations::by_id(bac.pool(), source.into())
        .await
        .expect("lecture")
        .expect("la source");
    assert_eq!(absorbee.status.as_str(), "merged");
}
