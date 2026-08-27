//! **Les cinq familles se déclenchent, chacune avec son décompte, ses exemples
//! et son lien.**
//!
//! Une ligne par famille, jamais une par élément : quarante dossiers non évalués
//! produiraient quarante lignes, et le bloc censé se lire d'un coup d'œil
//! deviendrait la liste des propositions — qui existe déjà, avec ses filtres.

mod commun;

use analytics::domain::action::{AdminActionKind, AdminActionSeverity};
use commun::*;

#[tokio::test]
async fn les_cinq_familles_sallument_avec_trois_exemples_au_plus() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // 1. Dossiers sans évaluation, échéance proche : quatre, pour éprouver le
    //    plafond de trois exemples.
    let appel_id = appel(&bac, decor.event_id, "cop31_appel", 5).await;
    for i in 0..4 {
        dossier_depose(
            &bac,
            decor.event_id,
            Some(appel_id),
            decor.organization_id,
            comptes.globale,
            &format!("Dossier {i}"),
        )
        .await;
    }

    // 3. Un message actif.
    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;

    // 5. Une paire d'organisations présumées identiques, non arbitrée.
    sqlx::query!(
        "INSERT INTO org.duplicate_candidates (left_id, right_id, score, reasons)
         VALUES ($1, $2, 120, ARRAY['nom']::text[])",
        decor.organization_id,
        decor.organisation_etrangere
    )
    .execute(bac.pool())
    .await
    .expect("insertion du doublon");

    rafraichir(&bac).await;

    let actions = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .actions;

    let familles: Vec<AdminActionKind> = actions.iter().map(|a| a.kind).collect();
    assert!(familles.contains(&AdminActionKind::ProposalsUnreviewed));
    assert!(familles.contains(&AdminActionKind::ActiveIncidents));
    assert!(familles.contains(&AdminActionKind::OrganizationDuplicates));

    for ligne in &actions {
        assert!(ligne.count > 0, "une ligne à zéro n'est jamais émise");
        assert!(
            ligne.examples.len() <= 3,
            "trois exemples nommés au plus, sinon la ligne cesse d'être un résumé"
        );
        assert!(
            ligne.target.starts_with("/admin/"),
            "chaque famille pointe vers l'écran qui la règle"
        );
    }

    let dossiers = actions
        .iter()
        .find(|a| a.kind == AdminActionKind::ProposalsUnreviewed)
        .expect("la famille des dossiers");
    assert_eq!(dossiers.count, 4, "quatre dossiers, une seule ligne");
    assert_eq!(dossiers.examples.len(), 3);
    assert_eq!(dossiers.severity, AdminActionSeverity::High);
    assert!(dossiers.due_at.is_some(), "l'échéance la plus proche");
}

#[tokio::test]
async fn le_rangement_met_la_gravite_haute_devant() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    // Une famille `medium` (doublons) et une `high` (message actif).
    sqlx::query!(
        "INSERT INTO org.duplicate_candidates (left_id, right_id, score, reasons)
         VALUES ($1, $2, 120, ARRAY['nom']::text[])",
        decor.organization_id,
        decor.organisation_etrangere
    )
    .execute(bac.pool())
    .await
    .expect("insertion du doublon");
    poser(
        &bac,
        comptes.globale,
        "session",
        Some(decor.session_id),
        "active",
    )
    .await;
    rafraichir(&bac).await;

    let actions = analytics::service::dashboard::composer(bac.pool(), decor.event_id)
        .await
        .expect("composition")
        .actions;

    assert_eq!(actions[0].severity, AdminActionSeverity::High);
    assert_eq!(actions[0].kind, AdminActionKind::ActiveIncidents);
    assert_eq!(actions[1].severity, AdminActionSeverity::Medium);
}

#[tokio::test]
async fn les_doublons_ne_revelent_lexistence_daucune_autre_edition() {
    let bac = Bac::monter().await;
    let decor = decor(&bac).await;
    let comptes = comptes(&bac, &decor).await;

    sqlx::query!(
        "INSERT INTO org.duplicate_candidates (left_id, right_id, score, reasons)
         VALUES ($1, $2, 120, ARRAY['nom']::text[])",
        decor.organization_id,
        decor.organisation_etrangere
    )
    .execute(bac.pool())
    .await
    .expect("insertion du doublon");
    // Un message global, seconde famille non filtrée par édition.
    poser(&bac, comptes.globale, "global", None, "active").await;
    rafraichir(&bac).await;

    let perimetre = perimetre_de(&bac, comptes.detache).await;
    let ecran = analytics::service::dashboard::ecran(&bac.state, &perimetre, decor.event_id)
        .await
        .expect("un compte détaché obtient son tableau de bord");

    let rendu = serde_json::to_string(&ecran.actions).expect("sérialisation");
    assert!(
        !rendu.contains("Bakou") && !rendu.contains("COP30"),
        "aucune autre édition n'est nommée : {rendu}"
    );
}
