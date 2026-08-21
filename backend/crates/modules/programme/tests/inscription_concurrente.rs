//! 🔴 **Cent inscriptions simultanées sur dix places** — l'écart n° 124.
//!
//! `tg_validate_registration()` contrôle la jauge par un `count(*)` **sans
//! verrou**. Sous `READ COMMITTED`, deux inscriptions simultanées lisent toutes
//! deux neuf places prises sur dix et passent toutes deux : onze inscrits sur
//! dix places, ce qui ne se verrait **que le jour de l'activité**. La position
//! en liste d'attente souffre du même défaut, et aucun index unique ne la
//! protège : deux personnes peuvent recevoir le même rang, ce qui ne se verrait
//! **jamais**.
//!
//! Ce n'est pas une cible de débit, c'est une correction : le service prend la
//! ligne de la séance en verrou avant toute écriture d'inscription.

mod commun;

use commun::seances::{self, Souhaits};
use commun::Bac;
use programme::domain::transitions::ProposalStatus;
use programme::service::transition;

const CANDIDATS: usize = 100;
const PLACES: i32 = 10;

#[tokio::test]
async fn cent_inscriptions_simultanees_sur_dix_places() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let dossier = seances::dossier_pret(
        &bac,
        &terrain,
        "Atelier très demandé",
        "atelier-demande",
        Souhaits::default(),
    )
    .await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();

    let seance = seances::seances_du_dossier(&bac, dossier.id)
        .await
        .remove(0)
        .id;
    seances::ouvrir_les_inscriptions(&bac, seance, Some(PLACES), true).await;

    let mut candidats = Vec::with_capacity(CANDIDATS);
    for rang in 0..CANDIDATS {
        candidats.push(
            commun::personne(
                &bac,
                &format!("candidat{rang}@example.org"),
                "Candidat",
                &format!("Numéro{rang}"),
            )
            .await,
        );
    }

    // **Toutes lancées avant qu'aucune n'aboutisse** : c'est ce qui met le
    // contrôle de jauge de la base en concurrence avec lui-même.
    let mut tentatives = Vec::with_capacity(CANDIDATS);
    for personne in candidats {
        let state = bac.state.clone();
        let ctx = bac.ctx().with_actor(personne);

        tentatives.push(tokio::spawn(async move {
            programme::service::registration::sinscrire(
                &state,
                &ctx,
                seance.into(),
                Some(personne),
                None,
                programme::service::registration::RegisterPayload {
                    answers: serde_json::json!({ "country": "SN" }),
                    locale: None,
                    guest: None,
                    sensitive_data_consent: false,
                    organization_id: None,
                },
            )
            .await
            .map(|_| ())
        }));
    }

    let mut abouties = 0;
    for tentative in tentatives {
        if tentative.await.expect("la tâche ne panique pas").is_ok() {
            abouties += 1;
        }
    }
    assert_eq!(abouties, CANDIDATS, "aucune inscription n'échoue");

    // 1. **Exactement dix confirmées.** Onze se verraient le jour de l'activité.
    let confirmees = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.registrations
            WHERE session_id = $1 AND status IN ('registered', 'attended')"#,
        seance
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(confirmees, PLACES as i64, "la jauge tient sous concurrence");

    // 2. **Aucun rang d'attente en double.** Sans verrou, ce défaut-là ne se
    // verrait jamais.
    let doublons = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM (
               SELECT waitlist_position
                 FROM programme.registrations
                WHERE session_id = $1 AND status = 'waitlisted'
                GROUP BY waitlist_position
               HAVING count(*) > 1
           ) AS doublons"#,
        seance
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(doublons, 0, "deux personnes au même rang d'attente");

    // 3. Et les rangs se suivent **sans trou**.
    let rangs: Vec<i32> = sqlx::query_scalar!(
        r#"SELECT waitlist_position AS "rang!"
             FROM programme.registrations
            WHERE session_id = $1 AND status = 'waitlisted'
            ORDER BY waitlist_position"#,
        seance
    )
    .fetch_all(bac.pool())
    .await
    .unwrap();

    assert_eq!(rangs.len(), CANDIDATS - PLACES as usize);
    assert_eq!(
        rangs,
        (1..=rangs.len() as i32).collect::<Vec<_>>(),
        "les positions se suivent sans trou"
    );
}
