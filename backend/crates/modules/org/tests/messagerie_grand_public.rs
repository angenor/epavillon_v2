//! **Une adresse Gmail ne propose rien, et deux organisations ne se
//! rapprochent pas pour autant.**
//!
//! Les domaines génériques ne prouvent aucune appartenance. Les exclure de tout
//! rapprochement automatique évite de fusionner deux ONG parce que leurs
//! référents utilisent la même messagerie — c'est le commentaire du § 3 du
//! modèle, et ce test le tient.
//!
//! La liste est **lue en base** (`org.public_email_domains`, vingt entrées) et
//! jamais recopiée en Rust : une liste en dur se périmerait le jour où l'IFDD en
//! ajoute une.

mod commun;

use commun::{ifdd, personne, Bac};
use org::domain::ids::{OrganizationId, PersonId};
use org::domain::membership::{JoinOrganization, JoinOutcome};
use org::repo::domains;
use org::service::{join, search::SearchQuery};
use uuid::Uuid;

#[tokio::test]
async fn une_adresse_de_messagerie_grand_public_ne_revele_rien() {
    let bac = Bac::monter().await;

    for adresse in [
        "awa.diallo@gmail.com",
        "awa.diallo@yahoo.fr",
        "awa.diallo@outlook.com",
        "awa.diallo@proton.me",
    ] {
        let revele = domains::what_email_reveals(bac.pool(), adresse)
            .await
            .unwrap_or_else(|e| panic!("{adresse} : {e}"));
        assert!(
            revele.is_none(),
            "{adresse} ne doit rien proposer : {revele:?}"
        );
    }
}

/// Deux fiches dont les référents utilisent Gmail **ne se rapprochent pas**.
/// C'est la fonction du modèle qui neutralise le domaine générique, et ce test
/// vérifie qu'aucune de nos deux lectures ne le rattrape.
#[tokio::test]
async fn deux_fiches_sur_gmail_ne_se_rapprochent_pas() {
    let bac = Bac::monter().await;
    let burkina = commun::pays(&bac, "BFA").await;

    let une =
        fiche_avec_domaine(&bac, "Coopérative maraîchère de Bobo", "coop-bobo", burkina).await;
    let autre = fiche_avec_domaine(&bac, "Atelier du fleuve Niger", "atelier-niger", burkina).await;

    // Le même domaine générique des deux côtés.
    for id in [une, autre] {
        sqlx::query!(
            "INSERT INTO org.organization_domains (organization_id, domain) VALUES ($1, 'gmail.com')",
            id
        )
        .execute(bac.pool())
        .await
        .expect("insertion du domaine générique");
    }

    // Même la lecture de revue, qui ne filtre rien, ne doit pas les rapprocher :
    // c'est la fonction du modèle qui neutralise le domaine, pas notre filtre.
    let resultats = org::service::search::similar_for_review(
        bac.pool(),
        SearchQuery {
            name: "Atelier du fleuve Niger".to_owned(),
            email: Some("quelquun@gmail.com".to_owned()),
            ..Default::default()
        },
    )
    .await
    .expect("recherche de revue");

    assert!(
        !resultats.iter().any(|r| r.organization_id.as_uuid() == une),
        "la coopérative ne doit pas remonter : elle ne partage qu'une messagerie \
         grand public avec l'atelier"
    );
}

/// Et par conséquent, une personne sur Gmail ne se rattache jamais d'office.
#[tokio::test]
async fn une_personne_sur_gmail_ne_se_rattache_pas_doffice() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let awa = personne(&bac, "awa.diallo@gmail.com", "Awa", "Diallo").await;

    let issue = join::join(
        &bac.state,
        &bac.ctx().with_actor(awa),
        PersonId(awa),
        OrganizationId(organisation),
        JoinOrganization {
            organization_id: None,
            job_title: Some("Chargée de projet".to_owned()),
        },
    )
    .await
    .expect("demande");

    assert!(
        matches!(issue, JoinOutcome::Pending { .. }),
        "une messagerie grand public ne prouve rien : {issue:?}"
    );
}

async fn fiche_avec_domaine(bac: &Bac, nom: &str, slug: &str, pays: Uuid) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, slug, organization_type_code, country_id, status)
           VALUES ($1, $2::text::platform.slug, 'ngo_association', $3, 'active')
        RETURNING id"#,
        nom,
        slug,
        pays
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la fiche")
}
