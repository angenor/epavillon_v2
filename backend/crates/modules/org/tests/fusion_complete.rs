//! **La fusion, de bout en bout** (SC-011).
//!
//! La fiche absorbée survit et pointe vers la vivante, la paire passe
//! « fusionnée », et **l'ancien nom trouve toujours la bonne fiche** — c'est la
//! promesse de la fusion, et elle tient parce que les dénominations sont
//! déplacées avant que la source ne soit close.

mod commun;

use commun::{perimetres, Bac};
use org::domain::ids::PersonId;
use org::domain::merge::{MergeOutcome, MergePayload};
use org::service::{merge, search::SearchQuery};
use std::collections::BTreeMap;

#[tokio::test]
async fn la_fusion_conserve_tout_et_ne_perd_rien() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    // Un membre de chaque côté : la fiche absorbée en apporte un.
    let boureima =
        commun::personne(&bac, "b.ouedraogo@osed-sahel.org", "Boureima", "Ouédraogo").await;
    sqlx::query!(
        "INSERT INTO org.memberships (organization_id, person_id, role, status, job_title, approved_at)
         VALUES ($1, $2, 'manager', 'active', 'Chargée de projet', now())",
        osed.jumelle,
        boureima
    )
    .execute(bac.pool())
    .await
    .expect("adhésion côté absorbée");

    let issue = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: osed.jumelle,
            target_id: osed.complete,
            pair_id: None,
            reason: "Deux fiches pour la même maison, même domaine".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "OSED Sahel".to_owned(),
        },
    )
    .await
    .expect("fusion");

    let rows = match issue {
        MergeOutcome::Merged {
            target,
            rows_reassigned,
            ..
        } => {
            assert_eq!(target.as_uuid(), osed.complete);
            rows_reassigned
        }
        autre => panic!("issue inattendue : {autre:?}"),
    };

    // **La fiche absorbée survit et pointe vers la vivante.**
    let absorbee = org::repo::organizations::by_id(bac.pool(), osed.jumelle.into())
        .await
        .expect("lecture")
        .expect("elle survit");
    assert_eq!(absorbee.status.as_str(), "merged");
    assert_eq!(
        absorbee.merged_into_id.map(|id| id.as_uuid()),
        Some(osed.complete)
    );
    assert!(absorbee.merged_at.is_some());

    // **L'ancien nom trouve toujours la bonne fiche.**
    let trouvees = org::service::search::similar_for_person(
        bac.pool(),
        SearchQuery {
            name: "OSED Sahel".to_owned(),
            ..Default::default()
        },
    )
    .await
    .expect("recherche");

    let ids: Vec<_> = trouvees
        .iter()
        .map(|r| r.organization_id.as_uuid())
        .collect();
    assert!(
        ids.contains(&osed.complete),
        "l'ancien nom mène à la fiche vivante"
    );
    assert!(
        !ids.contains(&osed.jumelle),
        "la fiche absorbée ne remonte plus"
    );

    // L'adhésion a suivi.
    let adhesion =
        org::repo::memberships::by_couple(bac.pool(), osed.complete.into(), PersonId(boureima))
            .await
            .expect("lecture")
            .expect("l'adhésion a basculé");
    assert_eq!(
        adhesion.status,
        org::domain::membership::MembershipStatus::Active
    );

    // Le décompte réel porte la ligne des adhésions.
    assert!(
        rows.get("org.memberships.organization_id").is_some(),
        "le décompte réel vient du journal : {rows}"
    );
}

/// **La paire de la file passe « fusionnée » — et c'est la base qui l'écrit.**
#[tokio::test]
async fn la_paire_de_la_file_est_marquee_par_la_base() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let pair_id = sqlx::query_scalar!(
        "INSERT INTO org.duplicate_candidates (left_id, right_id, score, reasons)
         VALUES (LEAST($1::uuid, $2::uuid), GREATEST($1::uuid, $2::uuid), 145.0, ARRAY['name_similarity','shared_domain'])
         RETURNING id",
        osed.complete,
        osed.jumelle
    )
    .fetch_one(bac.pool())
    .await
    .expect("paire");

    merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: osed.jumelle,
            target_id: osed.complete,
            pair_id: Some(pair_id),
            reason: "doublon".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "OSED Sahel".to_owned(),
        },
    )
    .await
    .expect("fusion");

    let paire = sqlx::query!(
        "SELECT decision, reviewed_at, reviewed_by FROM org.duplicate_candidates WHERE id = $1",
        pair_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("la paire");

    assert_eq!(paire.decision.as_deref(), Some("merged"));
    assert!(paire.reviewed_at.is_some());
    assert_eq!(
        paire.reviewed_by,
        Some(p.globale),
        "la fonction de base lit l'acteur dans le contexte de transaction : \
         c'est pourquoi elle DOIT être appelée par la porte d'écriture du noyau"
    );
}

/// **Le nom de confirmation est revérifié.** Masquer un bouton n'a jamais
/// empêché une requête.
#[tokio::test]
async fn un_nom_de_confirmation_faux_refuse_la_fusion() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let issue = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: osed.jumelle,
            target_id: osed.complete,
            pair_id: None,
            reason: "doublon".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "Pas le bon nom".to_owned(),
        },
    )
    .await
    .expect("le refus n'est pas une erreur");

    assert!(matches!(issue, MergeOutcome::ConfirmationMismatch));

    // Rien n'a bougé.
    let intacte = org::repo::organizations::by_id(bac.pool(), osed.jumelle.into())
        .await
        .expect("lecture")
        .expect("la fiche");
    assert_eq!(intacte.status.as_str(), "candidate");
}

/// La casse, les accents et le sigle : **la normalisation de la base tranche**,
/// et c'est la même que partout ailleurs.
#[tokio::test]
async fn le_nom_de_confirmation_ignore_la_casse_et_accepte_le_sigle() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;

    for saisi in ["osed sahel", "OSED SAHEL", "OSED-S"] {
        let bac = Bac::monter().await;
        let p2 = perimetres(&bac).await;
        let osed = commun::seed::paire_osed(&bac).await;

        let issue = merge::merge(
            &bac.state,
            &bac.ctx().with_actor(p2.globale),
            PersonId(p2.globale),
            MergePayload {
                source_id: osed.jumelle,
                target_id: osed.complete,
                pair_id: None,
                reason: "doublon".to_owned(),
                field_choices: BTreeMap::new(),
                confirmation_name: saisi.to_owned(),
            },
        )
        .await
        .expect("fusion");

        assert!(
            matches!(issue, MergeOutcome::Merged { .. }),
            "« {saisi} » désigne sans ambiguïté la fiche absorbée : {issue:?}"
        );
    }

    let _ = p;
}

/// Une organisation ne se fusionne pas avec elle-même. Le service le dit
/// **avant** la fonction de base, pour nommer le champ.
#[tokio::test]
async fn une_fiche_ne_se_fusionne_pas_avec_elle_meme() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let refus = merge::merge(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        PersonId(p.globale),
        MergePayload {
            source_id: osed.complete,
            target_id: osed.complete,
            pair_id: None,
            reason: "erreur de saisie".to_owned(),
            field_choices: BTreeMap::new(),
            confirmation_name: "OSED".to_owned(),
        },
    )
    .await
    .expect_err("refus");

    assert_eq!(
        refus.code,
        kernel::error::ErrorCode::OrgMergeSameOrganization
    );
    assert_eq!(refus.field.as_deref(), Some("target_id"));
}
