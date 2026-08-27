//! **Les six indicateurs, et `null` n'est jamais zéro.**
//!
//! C'est la distinction qui coûte le plus cher ici. Un taux d'acceptation nul
//! signifie qu'aucun dossier n'a été tranché ; affiché « 0 % », il ferait passer
//! un comité qui n'a pas commencé pour un comité qui a tout refusé.

mod commun;

use analytics::domain::figures::DashboardKpiKey;
use commun::*;

#[tokio::test]
async fn les_six_indicateurs_sont_toujours_rendus_dans_leur_ordre() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    rafraichir(&bac).await;

    let kpis = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures
        .kpis;

    let cles: Vec<DashboardKpiKey> = kpis.iter().map(|k| k.key).collect();
    assert_eq!(
        cles,
        vec![
            DashboardKpiKey::Submissions,
            DashboardKpiKey::Deadline,
            DashboardKpiKey::ReviewProgress,
            DashboardKpiKey::AcceptanceRate,
            DashboardKpiKey::Scheduled,
            DashboardKpiKey::Registrations,
        ],
        "l'ordre où l'écran les pose"
    );
}

#[tokio::test]
async fn un_taux_dacceptation_sur_zero_dossier_tranche_est_absent_pas_zero() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    let appel_id = appel(&bac, decor.event_id, "cop31_appel", 60).await;
    dossier_depose(
        &bac,
        decor.event_id,
        Some(appel_id),
        decor.organization_id,
        comptes.globale,
        "Dossier en attente",
    )
    .await;
    rafraichir(&bac).await;

    let kpis = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures
        .kpis;

    let taux = kpis
        .iter()
        .find(|k| k.key == DashboardKpiKey::AcceptanceRate)
        .expect("le taux");
    assert!(
        taux.value.is_none(),
        "aucun dossier tranché : le taux est ABSENT, pas nul — sinon un comité qui n'a pas commencé passerait pour un comité qui a tout refusé"
    );
}

#[tokio::test]
async fn lavancement_du_comite_est_absent_tant_quaucune_affectation_nexiste() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    rafraichir(&bac).await;

    let kpis = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures
        .kpis;

    let avancement = kpis
        .iter()
        .find(|k| k.key == DashboardKpiKey::ReviewProgress)
        .expect("l'avancement");
    assert!(avancement.value.is_none(), "le comité n'a pas commencé");
    assert!(avancement.out_of.is_none(), "ce n'est pas « 0 sur 0 »");
}

#[tokio::test]
async fn les_activites_programmees_nont_aucun_denominateur() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    appel(&bac, decor.event_id, "cop31_appel", 60).await;
    rafraichir(&bac).await;

    let kpis = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures
        .kpis;

    let programmees = kpis
        .iter()
        .find(|k| k.key == DashboardKpiKey::Scheduled)
        .expect("les activités programmées");
    assert!(
        programmees.out_of.is_none(),
        "« 12 sur 40 » laisserait croire que quarante créneaux existent"
    );
}

#[tokio::test]
async fn lecheance_porte_son_instant_et_ses_jours_restants() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    appel(&bac, decor.event_id, "cop31_appel", 12).await;
    rafraichir(&bac).await;

    let kpis = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .figures
        .kpis;

    let echeance = kpis
        .iter()
        .find(|k| k.key == DashboardKpiKey::Deadline)
        .expect("l'échéance");
    assert!(
        echeance.at.is_some(),
        "l'instant, pour la carte qui décompte"
    );
    let jours = echeance.value.expect("les jours restants");
    assert!(
        (10.0..=12.0).contains(&jours),
        "environ douze jours : {jours}"
    );
}
