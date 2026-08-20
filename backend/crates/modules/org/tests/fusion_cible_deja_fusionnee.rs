//! **Le message du trigger ressort mot pour mot** — obligation n° 3 du principe
//! X, second volet.
//!
//! Deux gardes protègent la même règle, et il a fallu les mesurer pour le
//! savoir : `org.merge_organizations()` refuse elle-même une cible absorbée —
//! « Organisation cible … introuvable ou déjà fusionnée. » — **avant** que
//! `org.tg_forbid_merge_chains()` n'ait l'occasion de se déclencher. Le trigger
//! reste le filet de sécurité d'une écriture directe, et son message —
//! « Cibler la fiche finale » — se vérifie donc à part.
//!
//! Dans les deux cas, le code ne redouble pas la règle : il **traduit** le refus
//! et rend le message tel que le modèle l'écrit. Le reformuler produirait un
//! second libellé pour un même refus, et le second se périmerait à la première
//! évolution du SQL.
//!
//! Le SQLSTATE de ce refus a été **relevé sur la base** : le nom de condition
//! `integrity_constraint_violation` vaut 23000. B1 a payé une fois d'avoir
//! recopié un code depuis un document au lieu de le mesurer.

mod commun;

use commun::{pays, perimetres, Bac};
use org::domain::ids::PersonId;
use org::domain::merge::{MergeOutcome, MergePayload};
use org::service::merge;
use std::collections::BTreeMap;

#[tokio::test]
async fn cibler_une_fiche_deja_fusionnee_rend_le_message_de_la_base() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;
    let ctx = bac.ctx().with_actor(p.globale);

    // Première fusion : la jumelle est absorbée par la fiche complète.
    merge::merge(
        &bac.state,
        &ctx,
        PersonId(p.globale),
        MergePayload {
            source_id: osed.jumelle,
            target_id: osed.complete,
            pair_id: None,
            reason: "doublon".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "OSED Sahel".to_owned(),
        },
    )
    .await
    .expect("première fusion");

    // Une troisième fiche, qu'on tente d'absorber dans la fiche DÉJÀ fusionnée.
    let burkina = pays(&bac, "BFA").await;
    let troisieme = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, slug, organization_type_code, country_id, status)
           VALUES ('Observatoire sahélien', 'observatoire-sahelien'::platform.slug,
                   'ngo_association', $1, 'active')
        RETURNING id"#,
        burkina
    )
    .fetch_one(bac.pool())
    .await
    .expect("troisième fiche");

    let issue = merge::merge(
        &bac.state,
        &ctx,
        PersonId(p.globale),
        MergePayload {
            source_id: troisieme,
            target_id: osed.jumelle,
            pair_id: None,
            reason: "erreur de cible".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "Observatoire sahélien".to_owned(),
        },
    )
    .await
    .expect("le refus de la base est traduit, jamais rendu en erreur interne");

    let (message, finale) = match issue {
        MergeOutcome::AlreadyMerged { message, target } => (message, target),
        autre => panic!("issue inattendue : {autre:?}"),
    };

    assert!(
        message.contains("déjà fusionnée"),
        "le message de la base est repris MOT POUR MOT : « {message} »"
    );
    assert_eq!(
        finale.map(|f| f.as_uuid()),
        Some(osed.complete),
        "le refus porte la fiche finale à viser : c'est ce que l'écran propose"
    );

    // Rien n'a bougé : la troisième fiche est intacte.
    let intacte = org::repo::organizations::by_id(bac.pool(), troisieme.into())
        .await
        .expect("lecture")
        .expect("la fiche");
    assert_eq!(intacte.status.as_str(), "active");
}

/// Une fiche **introuvable** rend `not_found`, jamais une erreur interne.
#[tokio::test]
async fn une_fiche_introuvable_rend_not_found() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let issue = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: uuid::Uuid::now_v7(),
            target_id: osed.complete,
            pair_id: None,
            reason: "erreur".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "peu importe".to_owned(),
        },
    )
    .await
    .expect("pas d'erreur interne");

    assert!(matches!(issue, MergeOutcome::NotFound));
}

/// **La fusion exige la portée globale.** Il n'existe pas de fusion limitée à
/// une édition : elle déplace des rattachements dans toutes, y compris celles
/// qu'on n'administre pas.
#[tokio::test]
async fn un_administrateur_detache_ne_fusionne_pas() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let refus = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.detachee),
        PersonId(p.detachee),
        MergePayload {
            source_id: osed.jumelle,
            target_id: osed.complete,
            pair_id: None,
            reason: "doublon".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "OSED Sahel".to_owned(),
        },
    )
    .await
    .expect_err("un administrateur détaché ne fusionne pas");

    assert_eq!(
        refus.code,
        kernel::error::ErrorCode::OrgMergeGlobalScopeRequired,
        "le code est distinct de FORBIDDEN : l'écran sait dire POURQUOI"
    );
}

/// **L'adresse d'URL n'est pas arbitrable**, et le refus **nomme le champ**.
#[tokio::test]
async fn ladresse_durl_de_la_fiche_absorbee_ne_se_reprend_pas() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let mut choix = BTreeMap::new();
    choix.insert("slug".to_owned(), org::domain::merge::MergeSide::Source);

    let refus = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: osed.jumelle,
            target_id: osed.complete,
            pair_id: None,
            reason: "doublon".to_owned(),
            field_choices: choix,
            confirmation_name: "OSED Sahel".to_owned(),
        },
    )
    .await
    .expect_err("l'adresse d'URL ne se déplace pas");

    assert_eq!(
        refus.code,
        kernel::error::ErrorCode::OrgMergeFieldNotArbitrable
    );
    assert_eq!(
        refus.field.as_deref(),
        Some("slug"),
        "le refus NOMME le champ : sans cela l'opérateur cherche ce qui a échoué"
    );

    // Et rien n'a été fusionné : le refus vient avant tout le reste.
    let intacte = org::repo::organizations::by_id(bac.pool(), osed.jumelle.into())
        .await
        .expect("lecture")
        .expect("la fiche");
    assert_eq!(intacte.status.as_str(), "candidate");
}

/// **Le message du trigger, mot pour mot.**
///
/// `tg_forbid_merge_chains` n'est atteint que par une écriture directe de
/// `merged_into_id` : la fonction de fusion refuse avant lui. Le filet reste
/// posé, et sa traduction se vérifie ici — c'est un invariant de la base, et le
/// code le rend sans le réécrire.
#[tokio::test]
async fn le_message_du_trigger_sort_mot_pour_mot() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: osed.jumelle,
            target_id: osed.complete,
            pair_id: None,
            reason: "doublon".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "OSED Sahel".to_owned(),
        },
    )
    .await
    .expect("première fusion");

    let burkina = pays(&bac, "BFA").await;
    let troisieme = sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, slug, organization_type_code, country_id, status)
           VALUES ('Observatoire sahélien', 'observatoire-sahelien'::platform.slug,
                   'ngo_association', $1, 'active')
        RETURNING id"#,
        burkina
    )
    .fetch_one(bac.pool())
    .await
    .expect("troisième fiche");

    // L'écriture directe que le trigger surveille : pointer vers une fiche
    // elle-même fusionnée.
    let refus = sqlx::query!(
        "UPDATE org.organizations
            SET status = 'merged', merged_into_id = $2, merged_at = now()
          WHERE id = $1",
        troisieme,
        osed.jumelle
    )
    .execute(bac.pool())
    .await
    .expect_err("le trigger refuse la chaîne");

    let traduite = kernel::pg_error::translate(&refus);

    assert!(
        traduite.message.contains("Cibler la fiche finale"),
        "le message du trigger est rendu MOT POUR MOT : « {} »",
        traduite.message
    );
    assert_eq!(
        traduite.code,
        kernel::error::ErrorCode::Conflict,
        "SQLSTATE 23000, relevé sur la base et non recopié d'un document"
    );
}
