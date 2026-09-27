//! **S'inscrire** (US2) — inscrite, liste d'attente et sa position, complet,
//! fenêtre close, annulée, rejeu d'une même référence, réinscription, nouvelle
//! venue derrière une liste non vide, concurrence sur la dernière place
//! (SC-003), promotion à la désinscription, désinscription tardive, lien de
//! visioconférence, `403` et `401`.

#[macro_use]
mod reunions;

use actix_web::http::header::IF_NONE_MATCH;
use actix_web::http::StatusCode;
use reunions::{desinscrire, inscrire, mes_inscriptions, Decor, Reunion, LIEN};
use serde_json::{json, Value};
use std::time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

struct Personnes {
    awa: Uuid,
    bea: Uuid,
    chloe: Uuid,
}

async fn trois(d: &Decor) -> Personnes {
    Personnes {
        awa: d.personne("awa@example.org", true).await,
        bea: d.personne("bea@example.org", true).await,
        chloe: d.personne("chloe@example.org", true).await,
    }
}

fn complete(capacite: i32, attente: bool) -> Reunion {
    Reunion {
        capacite: Some(capacite),
        attente,
        ..Default::default()
    }
}

#[tokio::test]
async fn sinscrire_puis_la_liste_dattente_avec_sa_position() {
    let d = Decor::monter().await;
    let app = application!(d);
    let p = trois(&d).await;
    let reunion = d.reunion(complete(1, true)).await;

    let r = frapper!(&app, inscrire(Some(p.awa), reunion, Uuid::now_v7()));
    assert_eq!(r.statut, StatusCode::OK, "{}", r.brut);
    assert_eq!(
        r.corps,
        json!({ "status": "registered", "waitlist_position": null })
    );

    let r = frapper!(&app, inscrire(Some(p.bea), reunion, Uuid::now_v7()));
    assert_eq!(
        r.corps,
        json!({ "status": "waitlisted", "waitlist_position": 1 })
    );
    let r = frapper!(&app, inscrire(Some(p.chloe), reunion, Uuid::now_v7()));
    assert_eq!(
        r.corps,
        json!({ "status": "waitlisted", "waitlist_position": 2 })
    );
    assert_eq!(
        d.inscrites(reunion).await,
        1,
        "la capacité n'est jamais dépassée"
    );
}

#[tokio::test]
async fn les_refus_de_la_base_sont_traduits() {
    let d = Decor::monter().await;
    let app = application!(d);
    let p = trois(&d).await;

    let pleine = d.reunion(complete(1, false)).await;
    frapper!(&app, inscrire(Some(p.awa), pleine, Uuid::now_v7()));
    let r = frapper!(&app, inscrire(Some(p.bea), pleine, Uuid::now_v7()));
    assert_eq!(r.statut, StatusCode::CONFLICT);
    assert_eq!(r.corps["code"], "NEGOTIATION_MEETING_FULL");
    assert!(!r.corps["message"]
        .as_str()
        .unwrap_or_default()
        .starts_with("meeting_"));

    let maintenant = OffsetDateTime::now_utc();
    let close = d
        .reunion(Reunion {
            ouverture: Some(maintenant - time::Duration::days(3)),
            fermeture: Some(maintenant - time::Duration::days(1)),
            ..Default::default()
        })
        .await;
    let r = frapper!(&app, inscrire(Some(p.awa), close, Uuid::now_v7()));
    assert_eq!(r.statut, StatusCode::CONFLICT);
    assert_eq!(r.corps["code"], "NEGOTIATION_MEETING_CLOSED");

    let libre = d
        .reunion(Reunion {
            inscription: false,
            ..Default::default()
        })
        .await;
    let r = frapper!(&app, inscrire(Some(p.awa), libre, Uuid::now_v7()));
    assert_eq!(r.corps["code"], "NEGOTIATION_MEETING_CLOSED");

    for statut in ["cancelled", "draft"] {
        let reunion = d
            .reunion(Reunion {
                statut,
                ..Default::default()
            })
            .await;
        let r = frapper!(&app, inscrire(Some(p.awa), reunion, Uuid::now_v7()));
        assert_eq!(r.statut, StatusCode::CONFLICT, "{statut}");
        assert_eq!(
            r.corps["code"], "NEGOTIATION_MEETING_UNAVAILABLE",
            "{statut}"
        );
    }

    let r = frapper!(&app, inscrire(Some(p.awa), Uuid::now_v7(), Uuid::now_v7()));
    assert_eq!(r.statut, StatusCode::NOT_FOUND);
    assert_eq!(r.corps["code"], "NEGOTIATION_MEETING_UNKNOWN");
}

#[tokio::test]
async fn un_geste_rejoue_ne_sinscrit_quune_fois_et_une_reference_neuve_reinscrit() {
    let d = Decor::monter().await;
    let app = application!(d);
    let p = trois(&d).await;
    let reunion = d.reunion(Reunion::default()).await;
    let geste = Uuid::now_v7();

    for _ in 0..3 {
        let r = frapper!(&app, inscrire(Some(p.awa), reunion, geste));
        assert_eq!(r.statut, StatusCode::OK);
        assert_eq!(r.corps["status"], "registered");
    }
    assert_eq!(d.lignes(reunion).await, 1);

    let r = frapper!(&app, desinscrire(Some(p.awa), reunion));
    assert_eq!(r.statut, StatusCode::NO_CONTENT);
    let r = frapper!(&app, desinscrire(Some(p.awa), reunion));
    assert_eq!(r.statut, StatusCode::NO_CONTENT, "idempotente");

    let r = frapper!(&app, inscrire(Some(p.awa), reunion, geste));
    assert_eq!(
        r.corps["status"], "cancelled",
        "l'ancien geste rejoué ne réinscrit pas"
    );
    assert_eq!(
        d.ligne(reunion, p.awa).await.map(|l| l.0),
        Some("cancelled".into())
    );

    let neuf = Uuid::now_v7();
    let r = frapper!(&app, inscrire(Some(p.awa), reunion, neuf));
    assert_eq!(r.corps["status"], "registered");
    assert_eq!(d.lignes(reunion).await, 1, "se réinscrire est un UPDATE");

    let r = frapper!(&app, mes_inscriptions(Some(p.awa)));
    assert_eq!(r.corps["registrations"][0]["client_ref"], json!(neuf));
}

#[tokio::test]
async fn une_nouvelle_venue_passe_derriere_une_liste_dattente_non_vide() {
    let d = Decor::monter().await;
    let app = application!(d);
    let p = trois(&d).await;
    let reunion = d.reunion(complete(1, true)).await;
    frapper!(&app, inscrire(Some(p.awa), reunion, Uuid::now_v7()));
    frapper!(&app, inscrire(Some(p.bea), reunion, Uuid::now_v7()));
    sqlx::query("UPDATE negotiation.meetings SET capacity = 5 WHERE id = $1")
        .bind(reunion)
        .execute(d.pool())
        .await
        .expect("capacité relevée, sans promotion");

    let r = frapper!(&app, inscrire(Some(p.chloe), reunion, Uuid::now_v7()));
    assert_eq!(
        r.corps,
        json!({ "status": "waitlisted", "waitlist_position": 2 })
    );
}

#[tokio::test]
async fn deux_inscriptions_simultanees_sur_la_derniere_place_par_la_route() {
    let d = Decor::monter().await;
    let app = application!(d);
    let p = trois(&d).await;

    for attente in [true, false] {
        let reunion = d.reunion(complete(1, attente)).await;
        // Une troisième connexion tient la réunion : les deux requêtes
        // s'y empilent et repartent ensemble.
        let mut verrou = d.pool().begin().await.expect("verrou");
        sqlx::query("SELECT 1 FROM negotiation.meetings WHERE id = $1 FOR UPDATE")
            .bind(reunion)
            .execute(&mut *verrou)
            .await
            .expect("réunion tenue");

        let (a, b, ()) = tokio::join!(
            async { frapper!(&app, inscrire(Some(p.awa), reunion, Uuid::now_v7())) },
            async { frapper!(&app, inscrire(Some(p.bea), reunion, Uuid::now_v7())) },
            async {
                tokio::time::sleep(Duration::from_millis(300)).await;
                verrou.commit().await.expect("libération");
            }
        );

        let statuts: Vec<Value> = [&a, &b]
            .iter()
            .map(|r| {
                if r.statut == StatusCode::OK {
                    r.corps["status"].clone()
                } else {
                    r.corps["code"].clone()
                }
            })
            .collect();
        let inscrites = statuts.iter().filter(|s| *s == "registered").count();
        assert_eq!(inscrites, 1, "jamais deux inscrites : {statuts:?}");
        let autre = if attente {
            "waitlisted"
        } else {
            "NEGOTIATION_MEETING_FULL"
        };
        assert!(statuts.iter().any(|s| s == autre), "{statuts:?}");
        assert_eq!(d.inscrites(reunion).await, 1);
    }
}

#[tokio::test]
async fn deux_connexions_sur_la_derniere_place_la_base_seule_tient_la_jauge() {
    let d = Decor::monter().await;
    let p = trois(&d).await;
    let reunion = d.reunion(complete(1, true)).await;
    let ecrire = "INSERT INTO negotiation.meeting_registrations (meeting_id, person_id)
                  VALUES ($1, $2) RETURNING status::text";

    let mut premiere = d.pool().begin().await.expect("première connexion");
    let mut seconde = d.pool().begin().await.expect("seconde connexion");
    let statut_a: String = sqlx::query_scalar(ecrire)
        .bind(reunion)
        .bind(p.awa)
        .fetch_one(&mut *premiere)
        .await
        .expect("première inscription");
    assert_eq!(statut_a, "registered");

    let (statut_b, attendu) = tokio::join!(
        async {
            let s: String = sqlx::query_scalar(ecrire)
                .bind(reunion)
                .bind(p.bea)
                .fetch_one(&mut *seconde)
                .await
                .expect("seconde inscription");
            seconde.commit().await.expect("seconde validée");
            (s, std::time::Instant::now())
        },
        async {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let a = std::time::Instant::now();
            premiere.commit().await.expect("première validée");
            a
        }
    );
    assert!(
        statut_b.1 >= attendu,
        "la seconde a attendu le verrou de la réunion"
    );
    assert_eq!(statut_b.0, "waitlisted");
    assert_eq!(d.inscrites(reunion).await, 1);
}

#[tokio::test]
async fn se_desinscrire_fait_passer_la_premiere_en_attente() {
    let d = Decor::monter().await;
    let app = application!(d);
    let p = trois(&d).await;
    let reunion = d.reunion(complete(1, true)).await;
    for qui in [p.awa, p.bea, p.chloe] {
        frapper!(&app, inscrire(Some(qui), reunion, Uuid::now_v7()));
    }

    let r = frapper!(&app, desinscrire(Some(p.awa), reunion));
    assert_eq!(r.statut, StatusCode::NO_CONTENT, "{}", r.brut);
    assert_eq!(
        d.ligne(reunion, p.awa).await,
        Some(("cancelled".into(), None))
    );
    assert_eq!(
        d.ligne(reunion, p.bea).await,
        Some(("registered".into(), None))
    );
    assert_eq!(
        d.ligne(reunion, p.chloe).await,
        Some(("waitlisted".into(), Some(1)))
    );
    assert_eq!(d.inscrites(reunion).await, 1);
}

#[tokio::test]
async fn se_desinscrire_apres_le_debut_est_refuse() {
    let d = Decor::monter().await;
    let app = application!(d);
    let p = trois(&d).await;
    let reunion = d.reunion(Reunion::default()).await;
    frapper!(&app, inscrire(Some(p.awa), reunion, Uuid::now_v7()));
    d.commencer(reunion).await;

    let r = frapper!(&app, desinscrire(Some(p.awa), reunion));
    assert_eq!(r.statut, StatusCode::CONFLICT);
    assert_eq!(r.corps["code"], "NEGOTIATION_MEETING_UNAVAILABLE");
    assert_eq!(
        d.ligne(reunion, p.awa).await.map(|l| l.0),
        Some("registered".into())
    );

    let r = frapper!(&app, desinscrire(Some(p.bea), reunion));
    assert_eq!(
        r.statut,
        StatusCode::NO_CONTENT,
        "sans inscription, rien à refuser"
    );
}

#[tokio::test]
async fn le_lien_de_visioconference_nest_servi_qua_qui_y_a_droit() {
    let d = Decor::monter().await;
    let app = application!(d);
    let p = trois(&d).await;
    let sans_acces = d.personne("dina@example.org", false).await;
    let reunion = d.reunion(complete(1, true)).await;
    let libre = d
        .reunion(Reunion {
            inscription: false,
            lien: Some("https://visio.example.org/ouverte"),
            debut: OffsetDateTime::now_utc() + time::Duration::days(8),
            ..Default::default()
        })
        .await;
    frapper!(&app, inscrire(Some(p.awa), reunion, Uuid::now_v7()));
    frapper!(&app, inscrire(Some(p.bea), reunion, Uuid::now_v7()));

    let awa = frapper!(&app, mes_inscriptions(Some(p.awa)));
    assert_eq!(awa.statut, StatusCode::OK, "{}", awa.brut);
    assert_eq!(awa.cache.as_deref(), Some("private, no-store"));
    assert_eq!(
        awa.corps["video"],
        json!([
            { "meeting_id": reunion, "url": LIEN },
            { "meeting_id": libre, "url": "https://visio.example.org/ouverte" }
        ])
    );
    assert_eq!(awa.corps["registrations"][0]["status"], "registered");

    let bea = frapper!(&app, mes_inscriptions(Some(p.bea)));
    assert_eq!(bea.corps["registrations"][0]["status"], "waitlisted");
    assert_eq!(bea.corps["registrations"][0]["waitlist_position"], 1);
    assert!(!bea.brut.contains(LIEN), "jamais pour la liste d'attente");

    let chloe = frapper!(&app, mes_inscriptions(Some(p.chloe)));
    assert!(!chloe.brut.contains(LIEN), "ni pour une autre");
    assert_eq!(chloe.corps["video"][0]["meeting_id"], json!(libre));

    let dina = frapper!(&app, mes_inscriptions(Some(sans_acces)));
    assert_eq!(dina.corps["video"], json!([]), "sans accès, aucun lien");
    assert_eq!(dina.corps["registrations"], json!([]));

    let autre = d.personne("eve@example.org", false).await;
    let eve = frapper!(&app, mes_inscriptions(Some(autre)));
    assert_eq!(dina.brut, eve.brut);
    assert_ne!(dina.etag, eve.etag, "l'empreinte est propre à la personne");
    let encore = frapper!(
        &app,
        mes_inscriptions(Some(p.awa)).insert_header((IF_NONE_MATCH, awa.etag.expect("ETag")))
    );
    assert_eq!(encore.statut, StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn sans_acces_403_et_sans_compte_401() {
    let d = Decor::monter().await;
    let app = application!(d);
    let sans_acces = d.personne("dina@example.org", false).await;
    let reunion = d.reunion(Reunion::default()).await;

    let r = frapper!(&app, inscrire(Some(sans_acces), reunion, Uuid::now_v7()));
    assert_eq!(r.statut, StatusCode::FORBIDDEN);
    assert_eq!(r.corps["code"], "NEGOTIATION_MEETING_FORBIDDEN");
    assert_eq!(d.lignes(reunion).await, 0);

    for r in [
        frapper!(&app, inscrire(None, reunion, Uuid::now_v7())),
        frapper!(&app, desinscrire(None, reunion)),
        frapper!(&app, mes_inscriptions(None)),
    ] {
        assert_eq!(r.statut, StatusCode::UNAUTHORIZED);
    }
}
