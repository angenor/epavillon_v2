//! **Une fusion écrit UN événement, pas deux** — obligation n° 4 du principe X.
//!
//! Ce test **compte** ; il ne vérifie pas la présence. C'est toute la
//! différence : `org.merge_organizations()` appelle elle-même
//! `platform.emit_event()`, et un service qui émettrait après elle en écrirait
//! deux **sans qu'aucune erreur ne le signale**. L'outbox accepte les deux, et
//! un consommateur idempotent traiterait la première ligne puis ignorerait la
//! mauvaise. Le défaut ne se voit qu'en relisant l'outbox d'un agrégat qui
//! aurait deux fois la même histoire.
//!
//! C'est le piège n° 1 du module `identity` — `identity.anonymize_person()` —
//! répété à l'identique. La règle générale, apprise deux fois maintenant :
//! **avant d'émettre après un appel de fonction SQL, lire la fonction.**

mod commun;

use commun::{perimetres, Bac};
use org::domain::ids::PersonId;
use org::domain::merge::MergePayload;
use org::service::merge;
use std::collections::BTreeMap;

#[tokio::test]
async fn une_fusion_necrit_quun_seul_evenement() {
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
            reason: "doublon manifeste".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "OSED Sahel".to_owned(),
        },
    )
    .await
    .expect("fusion");

    let fusions = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events
            WHERE event_type = 'org.organization.merged'"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");

    assert_eq!(
        fusions, 1,
        "UN événement. La fonction de base l'émet ; le service ne le refait pas."
    );

    // Et il porte bien l'agrégat de la fiche survivante.
    let emis = commun::evenements_emis(&bac, osed.complete).await;
    assert_eq!(emis, vec!["org.organization.merged".to_owned()]);
}

/// Deux fusions **successives** en écrivent deux, une par fusion : le compte
/// suit les gestes, pas le hasard.
#[tokio::test]
async fn deux_fusions_ecrivent_deux_evenements() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    // Une troisième fiche, à absorber ensuite par la même survivante.
    let burkina = commun::pays(&bac, "BFA").await;
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

    let ctx = bac.ctx().with_actor(p.globale);

    for (source, nom) in [
        (osed.jumelle, "OSED Sahel"),
        (troisieme, "Observatoire sahélien"),
    ] {
        merge::merge(
            &bac.state,
            &ctx,
            PersonId(p.globale),
            MergePayload {
                source_id: source,
                target_id: osed.complete,
                pair_id: None,
                reason: "doublon".to_owned(),
                field_choices: BTreeMap::new(),
                confirmation_name: nom.to_owned(),
            },
        )
        .await
        .expect("fusion");
    }

    let emis = commun::evenements_emis(&bac, osed.complete).await;
    assert_eq!(
        emis,
        vec![
            "org.organization.merged".to_owned(),
            "org.organization.merged".to_owned()
        ],
        "deux gestes, deux événements — et pas quatre"
    );
}
