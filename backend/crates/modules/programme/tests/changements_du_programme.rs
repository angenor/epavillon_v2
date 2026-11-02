//! **Ce qui a changé dans le programme publié** : `GET /events/{id}/programme-changes`,
//! sans session, lu dans le journal d'audit.

mod commun;

use actix_web::http::StatusCode;
use actix_web::{test as atest, web, App};
use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use programme::domain::transitions::ProposalStatus;
use programme::service::transition;
use serde_json::Value;
use uuid::Uuid;

async fn seance_placee(bac: &Bac, terrain: &Terrain, titre: &str, slug: &str) -> Uuid {
    let dossier = seances::dossier_pret(bac, terrain, titre, slug, Souhaits::default()).await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    let id = seances::seances_du_dossier(bac, dossier.id)
        .await
        .remove(0)
        .id;
    seances::placer(bac, terrain.edition, id, None, "14:00", "15:00")
        .await
        .unwrap();
    seances::situer(bac, id).await;
    id
}

/// Une instruction, une transaction : chaque écriture a son instant au journal.
async fn executer(bac: &Bac, sql: &'static str, id: Uuid) {
    sqlx::query(sql)
        .bind(id)
        .execute(bac.pool())
        .await
        .expect(sql);
}

async fn changements(bac: &Bac, edition: Uuid) -> Vec<Value> {
    let app = atest::init_service(
        App::new()
            .app_data(web::Data::new(bac.state.clone()))
            .configure(programme::routes::public_schedule::configurer),
    )
    .await;
    let reponse = atest::call_service(
        &app,
        atest::TestRequest::get()
            .uri(&format!("/events/{edition}/programme-changes"))
            .to_request(),
    )
    .await;
    assert_eq!(reponse.status(), StatusCode::OK);
    let corps: Value = atest::read_body_json(reponse).await;
    corps.as_array().cloned().expect("une liste")
}

fn du(lignes: &[Value], id: Uuid) -> Vec<&str> {
    let id = id.to_string();
    lignes
        .iter()
        .filter(|l| l["session_id"] == id.as_str())
        .map(|l| l["kind"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn horaire_titre_annulation_report_et_ajout() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let deplacee = seance_placee(&bac, &terrain, "Déplacée", "deplacee").await;
    let renommee = seance_placee(&bac, &terrain, "Ancien titre", "renommee").await;
    let annulee = seance_placee(&bac, &terrain, "Annulée", "annulee").await;
    let reportee = seance_placee(&bac, &terrain, "Reportée", "reportee").await;
    let ajoutee = seance_placee(&bac, &terrain, "Ajoutée", "ajoutee").await;
    let brouillon = seance_placee(&bac, &terrain, "Brouillon", "brouillon").await;

    // Avant publication : rien de ce qui arrive à une séance n'est public.
    executer(
        &bac,
        "UPDATE programme.sessions SET starts_at = starts_at + interval '1 hour',
                ends_at = ends_at + interval '1 hour' WHERE id = $1",
        deplacee,
    )
    .await;

    // La première publication, d'un seul geste : elle n'est pas une suite d'ajouts.
    sqlx::query(
        "UPDATE programme.sessions SET published_at = now(), status = 'scheduled'
          WHERE id = ANY($1)",
    )
    .bind(vec![deplacee, renommee, annulee, reportee])
    .execute(bac.pool())
    .await
    .unwrap();
    executer(
        &bac,
        "UPDATE event.events SET programme_published_at = now() WHERE id = $1",
        terrain.edition,
    )
    .await;
    assert!(
        changements(&bac, terrain.edition).await.is_empty(),
        "la première publication n'annonce rien"
    );

    executer(
        &bac,
        "UPDATE programme.sessions SET starts_at = starts_at + interval '30 minutes',
                ends_at = ends_at + interval '30 minutes' WHERE id = $1",
        deplacee,
    )
    .await;
    executer(
        &bac,
        "UPDATE programme.sessions SET starts_at = starts_at + interval '2 days',
                ends_at = ends_at + interval '2 days' WHERE id = $1",
        deplacee,
    )
    .await;
    executer(
        &bac,
        r#"UPDATE programme.sessions SET title = '{"fr":"Nouveau titre"}' WHERE id = $1"#,
        renommee,
    )
    .await;
    executer(
        &bac,
        "UPDATE programme.sessions SET status = 'cancelled' WHERE id = $1",
        annulee,
    )
    .await;
    executer(
        &bac,
        "UPDATE programme.sessions SET status = 'postponed' WHERE id = $1",
        reportee,
    )
    .await;
    executer(
        &bac,
        "UPDATE programme.sessions SET published_at = now(), status = 'scheduled' WHERE id = $1",
        ajoutee,
    )
    .await;
    executer(
        &bac,
        r#"UPDATE programme.sessions SET title = '{"fr":"Toujours interne"}' WHERE id = $1"#,
        brouillon,
    )
    .await;

    let lignes = changements(&bac, terrain.edition).await;

    // Le plus récent d'abord ; l'horaire d'avant publication n'y est pas.
    assert_eq!(du(&lignes, deplacee), ["rescheduled", "rescheduled"]);
    let jours: Vec<bool> = lignes
        .iter()
        .filter(|l| l["session_id"] == deplacee.to_string().as_str())
        .map(|l| l["day_changed"].as_bool().unwrap())
        .collect();
    assert_eq!(
        jours,
        [true, false],
        "deux jours plus tard, puis le même jour"
    );

    assert_eq!(du(&lignes, renommee), ["retitled"]);
    let titre = lignes.iter().find(|l| l["kind"] == "retitled").unwrap();
    assert_eq!(titre["previous_title"]["fr"], "Ancien titre");
    assert_eq!(titre["new_title"]["fr"], "Nouveau titre");
    assert_eq!(titre["slug"], "renommee");

    assert_eq!(du(&lignes, annulee), ["cancelled"]);
    assert_eq!(du(&lignes, reportee), ["postponed"]);
    assert_eq!(du(&lignes, ajoutee), ["added"]);
    let ajout = lignes.iter().find(|l| l["kind"] == "added").unwrap();
    assert_eq!(ajout["previous_starts_at"], Value::Null);
    assert!(
        du(&lignes, brouillon).is_empty(),
        "un brouillon n'existe pas"
    );

    assert_eq!(lignes[0]["kind"], "added", "le plus récent en haut");
    assert!(
        lignes.iter().all(|l| l.get("actor_label").is_none()),
        "aucun auteur"
    );
}

#[tokio::test]
async fn une_edition_au_programme_non_paru_ne_dit_rien() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let seance = seance_placee(&bac, &terrain, "Seule", "seule").await;
    executer(
        &bac,
        "UPDATE programme.sessions SET published_at = now(), status = 'scheduled' WHERE id = $1",
        seance,
    )
    .await;
    executer(
        &bac,
        "UPDATE programme.sessions SET status = 'cancelled' WHERE id = $1",
        seance,
    )
    .await;

    assert!(changements(&bac, terrain.edition).await.is_empty());
    assert!(changements(&bac, Uuid::now_v7()).await.is_empty());
}
