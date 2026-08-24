//! **Cent créations simultanées du même nom dans le même pays → une seule
//! fiche** (SC-005).
//!
//! C'est la base qui tranche, pas une lecture préalable :
//! `ux_organizations_name_country` est un index unique partiel, et le service se
//! contente de traduire son refus en `name_taken` — avec la fiche gagnante.

mod commun;

use commun::{pays, personne, Bac};
use org::domain::ids::PersonId;
use org::domain::organization::{CreateOrganization, CreateOrganizationOutcome};
use org::service::create;

const NOM: &str = "Alliance panafricaine pour l'adaptation côtière";

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cent_creations_simultanees_ne_produisent_quune_fiche() {
    let bac = Bac::monter().await;
    let senegal = pays(&bac, "SEN").await;
    let awa = personne(&bac, "a.sowfall@roac-afrique.org", "Awa", "Sow Fall").await;

    let mut taches = Vec::with_capacity(100);
    for _ in 0..100 {
        let state = bac.state.clone();
        let ctx = bac.ctx().with_actor(awa);
        taches.push(tokio::spawn(async move {
            create::create(
                &state,
                &ctx,
                PersonId(awa),
                CreateOrganization {
                    legal_name: NOM.to_owned(),
                    acronym: None,
                    organization_type_code: "ngo_association".to_owned(),
                    country_id: Some(senegal),
                    city: None,
                    website: None,
                    description: None,
                    job_title: Some("Directrice".to_owned()),
                    acknowledged_match_ids: Vec::new(),
                },
            )
            .await
        }));
    }

    let mut creees = 0;
    let mut refusees = 0;
    for tache in taches {
        match tache.await.expect("la tâche ne panique pas") {
            Ok(CreateOrganizationOutcome::Created { .. }) => creees += 1,
            Ok(CreateOrganizationOutcome::NameTaken { .. }) => refusees += 1,
            Err(e) => panic!(
                "aucune création ne doit sortir en erreur : la contrainte doit être \
                 traduite en refus. {e}"
            ),
        }
    }

    assert_eq!(creees, 1, "une seule fiche créée");
    assert_eq!(refusees, 99, "les autres reçoivent la fiche gagnante");

    let fiches = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.organizations
            WHERE legal_name_normalized = platform.normalize_label($1)"#,
        NOM
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(fiches, 1);
}
