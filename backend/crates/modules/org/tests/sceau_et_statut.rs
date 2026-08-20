//! **Le sceau n'est pas le statut.**
//!
//! `verified_at` dit que l'IFDD a reconnu l'organisation ; `status` dit où elle
//! en est de son cycle de vie. Une fiche peut être active sans sceau — elle
//! soumet, mais rien ne s'affiche à côté de son nom. Les mélanger ferait
//! disparaître d'un écran une organisation qu'on voulait seulement ne pas mettre
//! en avant.

mod commun;

use commun::{pays, perimetres, Bac};
use org::domain::admin::{OrganizationVerification, OrganizationWriteOutcome};
use org::domain::ids::{OrganizationId, PersonId};
use org::service::admin_write;
use uuid::Uuid;

async fn fiche_candidate(bac: &Bac) -> Uuid {
    let burkina = pays(bac, "BFA").await;
    sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, slug, organization_type_code, country_id, status)
           VALUES ('Réseau des jeunes pour le climat', 'rjc'::platform.slug,
                   'ngo_association', $1, 'candidate')
        RETURNING id"#,
        burkina
    )
    .fetch_one(bac.pool())
    .await
    .expect("fiche candidate")
}

#[tokio::test]
async fn poser_le_sceau_admet_une_fiche_candidate_du_meme_geste() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let organisation = fiche_candidate(&bac).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    let issue = admin_write::set_verification(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        &perimetre,
        PersonId(p.globale),
        OrganizationId(organisation),
        OrganizationVerification {
            organization_id: Some(organisation),
            verified: true,
        },
    )
    .await
    .expect("pose du sceau");

    let fiche = match issue {
        OrganizationWriteOutcome::Saved { detail } => detail,
        autre => panic!("issue inattendue : {autre:?}"),
    };

    assert!(fiche.verified_at.is_some(), "le sceau est posé");
    assert_eq!(
        fiche.status, "active",
        "poser le sceau admet la fiche du même geste"
    );
    assert!(
        fiche.verified_by_name.is_some(),
        "on sait qui a reconnu l'organisation"
    );
}

/// **Retirer le sceau ne déclasse pas la fiche.**
#[tokio::test]
async fn retirer_le_sceau_ne_change_pas_le_statut() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let organisation = fiche_candidate(&bac).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    let ctx = bac.ctx().with_actor(p.globale);
    let demande = |verified: bool| OrganizationVerification {
        organization_id: Some(organisation),
        verified,
    };

    admin_write::set_verification(
        &bac.state,
        &ctx,
        &perimetre,
        PersonId(p.globale),
        OrganizationId(organisation),
        demande(true),
    )
    .await
    .expect("pose");

    let issue = admin_write::set_verification(
        &bac.state,
        &ctx,
        &perimetre,
        PersonId(p.globale),
        OrganizationId(organisation),
        demande(false),
    )
    .await
    .expect("retrait");

    let fiche = match issue {
        OrganizationWriteOutcome::Saved { detail } => detail,
        autre => panic!("issue inattendue : {autre:?}"),
    };

    assert!(fiche.verified_at.is_none(), "le sceau est retiré");
    assert_eq!(
        fiche.status, "active",
        "la fiche reste active : elle cesse d'être certifiée, elle ne redevient \
         pas candidate"
    );
}

/// Les deux gestes émettent **deux événements distincts**. Un événement nommé
/// « vérifiée » portant « non » est un mensonge que personne ne relit
/// correctement.
#[tokio::test]
async fn les_deux_gestes_emettent_deux_evenements_distincts() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let organisation = fiche_candidate(&bac).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    for verified in [true, false] {
        admin_write::set_verification(
            &bac.state,
            &bac.ctx().with_actor(p.globale),
            &perimetre,
            PersonId(p.globale),
            OrganizationId(organisation),
            OrganizationVerification {
                organization_id: Some(organisation),
                verified,
            },
        )
        .await
        .expect("écriture");
    }

    let emis = commun::evenements_emis(&bac, organisation).await;
    assert_eq!(
        emis,
        vec![
            "org.organization.verified".to_owned(),
            "org.organization.unverified".to_owned()
        ]
    );
}

/// Le score de confiance suit : le sceau vaut quarante points, et le travail
/// différé est mis en file par l'écriture qui le rend nécessaire.
#[tokio::test]
async fn poser_le_sceau_met_le_recalcul_du_score_en_file() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let organisation = fiche_candidate(&bac).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    admin_write::set_verification(
        &bac.state,
        &bac.ctx().with_actor(p.globale),
        &perimetre,
        PersonId(p.globale),
        OrganizationId(organisation),
        OrganizationVerification {
            organization_id: Some(organisation),
            verified: true,
        },
    )
    .await
    .expect("pose du sceau");

    let taches: Vec<String> = commun::travaux(&bac)
        .await
        .into_iter()
        .map(|(t, _)| t)
        .collect();

    assert!(taches.contains(&"org.trust_score.recompute".to_owned()));
    assert!(taches.contains(&"org.scorecard.refresh".to_owned()));
}
