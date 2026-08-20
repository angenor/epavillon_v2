//! **C'est le domaine qui décide, pas la volonté de la personne.**
//!
//! Le semis pose l'IFDD avec deux domaines vérifiés : `ifdd.francophonie.org`,
//! ouvert au rattachement automatique, et `francophonie.org`, vérifié mais
//! **non** marqué. Les deux issues doivent différer sans qu'on ait rien semé de
//! plus — c'est ce que ce test éprouve.

mod commun;

use commun::{ifdd, personne, personne_non_verifiee, Bac};
use org::domain::ids::{OrganizationId, PersonId};
use org::domain::membership::{JoinOrganization, JoinOutcome, MembershipStatus};
use org::service::join;

async fn rejoindre(bac: &Bac, person_id: uuid::Uuid, organisation: uuid::Uuid) -> JoinOutcome {
    join::join(
        &bac.state,
        &bac.ctx().with_actor(person_id),
        PersonId(person_id),
        OrganizationId(organisation),
        JoinOrganization {
            organization_id: None,
            job_title: Some("Chargée de programme".to_owned()),
        },
    )
    .await
    .expect("demande de rattachement")
}

#[tokio::test]
async fn un_domaine_verifie_et_marque_rattache_doffice() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let awa = personne(&bac, "awa.diallo@ifdd.francophonie.org", "Awa", "Diallo").await;

    let issue = rejoindre(&bac, awa, organisation).await;

    assert!(
        matches!(issue, JoinOutcome::Joined { .. }),
        "un domaine vérifié et ouvert au rattachement automatique fait entrer d'office : {issue:?}"
    );

    let adhesion =
        org::repo::memberships::by_couple(bac.pool(), organisation.into(), PersonId(awa))
            .await
            .expect("lecture")
            .expect("l'adhésion");
    assert_eq!(adhesion.status, MembershipStatus::Active);
    assert!(
        adhesion.approved_at.is_some(),
        "un rattachement automatique EST une approbation : laisser la colonne nulle \
         ferait croire à une adhésion active que personne n'a validée"
    );

    // La base attribue la primauté : le service ne la calcule jamais.
    assert!(
        adhesion.is_primary,
        "première adhésion active de la personne"
    );
    let rattachement = sqlx::query_scalar!(
        "SELECT primary_organization_id FROM identity.people WHERE id = $1",
        awa
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture de la personne");
    assert_eq!(rattachement, Some(organisation));
}

#[tokio::test]
async fn un_domaine_verifie_mais_non_marque_laisse_en_attente() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    // `francophonie.org` est vérifié dans le semis, mais `auto_join` y vaut faux.
    let claire = personne(&bac, "c.perret@francophonie.org", "Claire", "Perret").await;

    let issue = rejoindre(&bac, claire, organisation).await;

    assert!(
        matches!(issue, JoinOutcome::Pending { .. }),
        "un domaine vérifié SANS rattachement automatique attend un référent : {issue:?}"
    );
}

/// L'adresse doit être **vérifiée**. Sans cela, déclarer une adresse suffirait à
/// entrer dans une organisation qu'on ne connaît pas.
#[tokio::test]
async fn une_adresse_non_verifiee_ne_rattache_pas_doffice() {
    let bac = Bac::monter().await;
    let organisation = ifdd(&bac).await;
    let inconnue = personne_non_verifiee(&bac, "quelquun@ifdd.francophonie.org").await;

    let issue = rejoindre(&bac, inconnue, organisation).await;

    assert!(
        matches!(issue, JoinOutcome::Pending { .. }),
        "une adresse non prouvée n'ouvre rien : {issue:?}"
    );
}

/// Deux fiches sur un même domaine : **la vérifiée l'emporte**. C'est le signal
/// de doublon du modèle, et entre deux fiches, celle que l'IFDD a reconnue est
/// la bonne réponse.
#[tokio::test]
async fn entre_deux_fiches_sur_un_domaine_la_verifiee_lemporte() {
    let bac = Bac::monter().await;
    let osed = commun::seed::paire_osed(&bac).await;

    let revele = org::repo::domains::what_email_reveals(
        bac.pool(),
        &format!("b.ouedraogo@{}", osed.domaine),
    )
    .await
    .expect("lecture du domaine")
    .expect("le domaine révèle une organisation");

    assert_eq!(
        revele.organization.id.as_uuid(),
        osed.complete,
        "la fiche vérifiée doit l'emporter sur la jumelle"
    );
    assert!(revele.can_auto_join);
    assert_eq!(revele.domain, osed.domaine);
}

/// Une demande visant une fiche **absorbée** ouvre l'adhésion sur la fiche
/// vivante (FR-024). C'est la promesse de `org.resolve_organization()`.
#[tokio::test]
async fn une_demande_sur_une_fiche_absorbee_mene_a_la_vivante() {
    let bac = Bac::monter().await;
    let osed = commun::seed::paire_osed(&bac).await;

    let db = bac.db();
    let mut tx = db.write(&bac.ctx()).await.expect("transaction");
    sqlx::query_scalar!(
        "SELECT org.merge_organizations($1, $2, NULL)",
        osed.jumelle,
        osed.complete
    )
    .fetch_one(&mut *tx)
    .await
    .expect("fusion");
    tx.commit().await.expect("validation");

    let karim = personne(&bac, "karim@example.org", "Karim", "Ilboudo").await;
    let issue = rejoindre(&bac, karim, osed.jumelle).await;

    let organisation = match &issue {
        JoinOutcome::Joined { organization, .. } | JoinOutcome::Pending { organization, .. } => {
            organization.id.as_uuid()
        }
        autre => panic!("issue inattendue : {autre:?}"),
    };

    assert_eq!(
        organisation, osed.complete,
        "rejoindre une fiche absorbée doit mener à la fiche vivante"
    );
}
