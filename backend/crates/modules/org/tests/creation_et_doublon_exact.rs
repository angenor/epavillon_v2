//! **Rien n'est bloqué, sauf le doublon exact que la base refuse.**
//!
//! Et ce refus n'est pas une erreur : il sort en 200 avec son discriminant et
//! porte la fiche en cause, sous la forme d'un résultat de recherche — de quoi
//! la rejoindre. C'est la différence entre prévenir un doublon et refuser une
//! création.

mod commun;

use commun::{pays, personne, Bac};
use org::domain::ids::PersonId;
use org::domain::membership::{MembershipRole, MembershipStatus};
use org::domain::organization::{
    CreateOrganization, CreateOrganizationOutcome, OrganizationStatus,
};
use org::service::create;
use uuid::Uuid;

fn demande(nom: &str, pays: Option<Uuid>) -> CreateOrganization {
    CreateOrganization {
        legal_name: nom.to_owned(),
        acronym: Some("RCA".to_owned()),
        organization_type_code: "ngo_association".to_owned(),
        country_id: pays,
        city: Some("Dakar".to_owned()),
        website: None,
        description: None,
        job_title: Some("Coordinatrice".to_owned()),
        acknowledged_match_ids: Vec::new(),
    }
}

async fn creer(bac: &Bac, qui: Uuid, demande: CreateOrganization) -> CreateOrganizationOutcome {
    create::create(
        &bac.state,
        &bac.ctx().with_actor(qui),
        PersonId(qui),
        demande,
    )
    .await
    .expect("création")
}

#[tokio::test]
async fn une_fiche_nait_candidate_et_son_createur_en_est_referent() {
    let bac = Bac::monter().await;
    let senegal = pays(&bac, "SEN").await;
    let awa = personne(&bac, "a.sowfall@roac-afrique.org", "Awa", "Sow Fall").await;

    let issue = creer(&bac, awa, demande("Réseau climat africain", Some(senegal))).await;

    let (fiche, role) = match issue {
        CreateOrganizationOutcome::Created {
            organization, role, ..
        } => (organization, role),
        autre => panic!("issue inattendue : {autre:?}"),
    };

    assert_eq!(
        fiche.status,
        OrganizationStatus::Candidate,
        "une fiche née d'un formulaire public n'est pas une fiche de référence"
    );
    assert_eq!(role, MembershipRole::Manager);
    assert_eq!(fiche.slug, "reseau-climat-africain");
    assert_eq!(fiche.created_by.map(|p| p.as_uuid()), Some(awa));

    let adhesion = org::repo::memberships::by_couple(bac.pool(), fiche.id, PersonId(awa))
        .await
        .expect("lecture")
        .expect("l'adhésion du créateur");
    assert_eq!(adhesion.role, MembershipRole::Manager);
    assert_eq!(
        adhesion.status,
        MembershipStatus::Active,
        "le créateur n'a personne pour l'approuver"
    );
    assert!(
        adhesion.is_primary,
        "la fiche devient son rattachement principal s'il n'en avait pas"
    );

    // Le nom légal et le sigle sont devenus cherchables **sans écriture du
    // service** : c'est `tg_organizations_sync_names` qui les recopie.
    let denominations: Vec<(String, String)> = sqlx::query!(
        r#"SELECT name, kind::text AS "kind!" FROM org.organization_names
            WHERE organization_id = $1 ORDER BY kind"#,
        fiche.id.as_uuid()
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des dénominations")
    .into_iter()
    .map(|l| (l.name, l.kind))
    .collect();

    assert_eq!(
        denominations,
        vec![
            ("Réseau climat africain".to_owned(), "legal".to_owned()),
            ("RCA".to_owned(), "acronym".to_owned()),
        ]
    );
}

/// Le doublon exact sort en **200 avec son discriminant**, jamais en erreur, et
/// il porte la fiche en cause.
#[tokio::test]
async fn un_nom_deja_pris_dans_le_meme_pays_rend_name_taken_avec_la_fiche() {
    let bac = Bac::monter().await;
    let senegal = pays(&bac, "SEN").await;
    let awa = personne(&bac, "a.sowfall@roac-afrique.org", "Awa", "Sow Fall").await;
    let karim = personne(&bac, "karim.ilboudo@example.org", "Karim", "Ilboudo").await;

    let premiere = creer(&bac, awa, demande("Réseau climat africain", Some(senegal))).await;
    let attendue = match premiere {
        CreateOrganizationOutcome::Created { organization, .. } => organization.id.as_uuid(),
        autre => panic!("issue inattendue : {autre:?}"),
    };

    // La casse et les accents ne comptent pas : la normalisation de la base les
    // efface, et c'est bien le même nom.
    let seconde = creer(
        &bac,
        karim,
        demande("RESEAU CLIMAT AFRICAIN", Some(senegal)),
    )
    .await;

    match seconde {
        CreateOrganizationOutcome::NameTaken { existing } => {
            assert_eq!(existing.organization_id.as_uuid(), attendue);
            assert_eq!(existing.legal_name, "Réseau climat africain");
            assert!(
                existing.member_count >= 1,
                "le résultat porte de quoi reconnaître la fiche"
            );
        }
        autre => panic!("issue inattendue : {autre:?}"),
    }

    let fiches = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.organizations
            WHERE legal_name_normalized = platform.normalize_label('Réseau climat africain')"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(fiches, 1, "aucune seconde fiche n'a été créée");
}

/// **Une simple ressemblance ne bloque rien.** L'écran l'a montrée, la personne
/// a maintenu : c'est une revue humaine que ça mérite, pas un refus.
#[tokio::test]
async fn une_ressemblance_ne_bloque_pas_la_creation() {
    let bac = Bac::monter().await;
    let senegal = pays(&bac, "SEN").await;
    let awa = personne(&bac, "a.sowfall@roac-afrique.org", "Awa", "Sow Fall").await;

    let premiere = creer(&bac, awa, demande("Réseau climat africain", Some(senegal))).await;
    let voisine = match premiere {
        CreateOrganizationOutcome::Created { organization, .. } => organization.id.as_uuid(),
        autre => panic!("issue inattendue : {autre:?}"),
    };

    let mut seconde = demande("Réseau climat africain de l'Ouest", Some(senegal));
    // La personne a vu la fiche voisine et a maintenu sa création.
    seconde.acknowledged_match_ids = vec![voisine];

    let issue = creer(&bac, awa, seconde).await;
    assert!(
        matches!(issue, CreateOrganizationOutcome::Created { .. }),
        "une ressemblance ne bloque jamais : {issue:?}"
    );

    // Le chiffre des fiches montrées part avec l'événement : la revue le lit.
    let charge = sqlx::query_scalar!(
        r#"SELECT payload AS "payload!" FROM platform.outbox_events
            WHERE event_type = 'org.organization.created'
            ORDER BY occurred_at DESC LIMIT 1"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de l'outbox");
    assert_eq!(charge["acknowledged_matches"], 1);
}

/// Le même nom dans **un autre pays** n'est pas un doublon : deux réseaux
/// homonymes dans deux pays sont deux organisations.
#[tokio::test]
async fn le_meme_nom_dans_un_autre_pays_nest_pas_un_doublon() {
    let bac = Bac::monter().await;
    let senegal = pays(&bac, "SEN").await;
    let burkina = pays(&bac, "BFA").await;
    let awa = personne(&bac, "a.sowfall@roac-afrique.org", "Awa", "Sow Fall").await;

    creer(&bac, awa, demande("Réseau climat africain", Some(senegal))).await;
    let ailleurs = creer(&bac, awa, demande("Réseau climat africain", Some(burkina))).await;

    assert!(
        matches!(ailleurs, CreateOrganizationOutcome::Created { .. }),
        "l'unicité porte sur (nom normalisé, pays) : {ailleurs:?}"
    );
}
