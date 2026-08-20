//! **Le chiffre annoncé égale le chiffre réel** (research.md § R8, SC-017).
//!
//! Toute suppression qui détache des séances les compte **dans la même
//! transaction, avant l'ordre de suppression**. Après l'ordre, le lien n'existe
//! plus — la clé est `ON DELETE SET NULL` — : le décompte rendrait **zéro**, et
//! l'écran annoncerait sereinement qu'il n'a rien déplacé.
//!
//! Retirer un **lieu** emporte ses salles par cascade : le chiffre doit couvrir
//! **toutes** ses salles, et non seulement celle que l'écran affichait.
//!
//! Aucun déclencheur du modèle ne touche à ces liens — les seuls déclencheurs du
//! module sont deux audits et cinq horodatages. Le chiffre pris avant est donc
//! exact, et ce test peut l'exiger **au chiffre près**.

mod commun;

use commun::{formulaire_lieu, formulaire_salle, Bac};
use event::domain::ids::{EventId, RoomId, VenueId};
use event::service::venues as service_lieux;
use uuid::Uuid;

/// Une séance placée dans une salle donnée, à une heure donnée.
async fn seance_dans(bac: &Bac, event_id: Uuid, room_id: Uuid, heure: &str) {
    sqlx::query!(
        r#"INSERT INTO programme.sessions
               (event_id, title, slug, format, starts_at, ends_at, timezone, room_id)
           VALUES ($1, '{"fr":"Séance"}'::jsonb,
                   ('seance-' || gen_random_uuid())::platform.slug, 'in_person',
                   ('2027-11-10 ' || $4)::timestamp AT TIME ZONE $2,
                   ('2027-11-10 ' || $4)::timestamp AT TIME ZONE $2 + interval '1 hour',
                   $2::text::platform.timezone_name, $3)"#,
        event_id,
        commun::seed::FUSEAU_COP31,
        room_id,
        heure
    )
    .execute(bac.pool())
    .await
    .expect("insertion de la séance");
}

/// Le nombre de séances de l'édition **sans salle** : ce que le détachement a
/// réellement produit.
async fn seances_sans_salle(bac: &Bac, event_id: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM programme.sessions
            WHERE event_id = $1 AND room_id IS NULL"#,
        event_id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn retirer_une_salle_annonce_le_nombre_exact_de_seances_deplacees() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let lieu = service_lieux::enregistrer_lieu(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        formulaire_lieu(editions.cop31, "Pavillon"),
    )
    .await
    .expect("création du lieu")
    .detail
    .expect("la composition")
    .venues
    .remove(0);

    let mut salles = Vec::new();
    for code in ["baobab", "fromager"] {
        let detail = service_lieux::enregistrer_salle(
            &bac.state,
            &bac.ctx(),
            cop31,
            None,
            formulaire_salle(lieu.id, code),
        )
        .await
        .expect("création de la salle")
        .detail
        .expect("la composition");

        salles = detail.venues[0].rooms.clone();
    }
    assert_eq!(salles.len(), 2);

    // Trois séances dans la première salle, une dans la seconde : le chiffre
    // annoncé doit être **celui de la salle retirée**, pas celui du lieu.
    let baobab = salles.iter().find(|s| s.code == "baobab").unwrap();
    let fromager = salles.iter().find(|s| s.code == "fromager").unwrap();
    for heure in ["09:00", "11:00", "14:00"] {
        seance_dans(&bac, editions.cop31, baobab.id, heure).await;
    }
    seance_dans(&bac, editions.cop31, fromager.id, "16:00").await;

    let resultat =
        service_lieux::supprimer_salle(&bac.state, &bac.ctx(), cop31, RoomId::from(baobab.id))
            .await
            .expect("le retrait aboutit");

    assert!(resultat.ok);
    assert_eq!(
        resultat.sessions_detached, 3,
        "trois séances étaient installées dans cette salle"
    );
    assert_eq!(
        seances_sans_salle(&bac, editions.cop31).await,
        3,
        "le chiffre annoncé égale le chiffre réel"
    );
}

/// **Retirer un lieu compte les séances de TOUTES ses salles.** La cascade
/// emporte les salles ; un décompte qui n'en couvrirait qu'une sous-estimerait
/// ce que l'équipe s'apprête à défaire.
#[tokio::test]
async fn retirer_un_lieu_annonce_les_seances_de_toutes_ses_salles() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let lieu = service_lieux::enregistrer_lieu(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        formulaire_lieu(editions.cop31, "Pavillon"),
    )
    .await
    .expect("création du lieu")
    .detail
    .expect("la composition")
    .venues
    .remove(0);

    let mut salles = Vec::new();
    for code in ["baobab", "fromager"] {
        salles = service_lieux::enregistrer_salle(
            &bac.state,
            &bac.ctx(),
            cop31,
            None,
            formulaire_salle(lieu.id, code),
        )
        .await
        .expect("création de la salle")
        .detail
        .expect("la composition")
        .venues[0]
            .rooms
            .clone();
    }

    for (rang, salle) in salles.iter().enumerate() {
        seance_dans(
            &bac,
            editions.cop31,
            salle.id,
            if rang == 0 { "09:00" } else { "11:00" },
        )
        .await;
    }

    let resultat =
        service_lieux::supprimer_lieu(&bac.state, &bac.ctx(), cop31, VenueId::from(lieu.id))
            .await
            .expect("le retrait aboutit");

    assert!(resultat.ok);
    assert_eq!(
        resultat.sessions_detached, 2,
        "une séance par salle, deux salles"
    );
    assert_eq!(
        seances_sans_salle(&bac, editions.cop31).await,
        2,
        "le chiffre annoncé égale le chiffre réel"
    );

    let salles_restantes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM event.rooms WHERE venue_id = $1"#,
        lieu.id
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(salles_restantes, 0, "les salles partent avec leur lieu");
}

/// **Sans séance, le chiffre est zéro** — et non une valeur par défaut posée au
/// hasard. Un retrait sans conséquence doit se voir comme tel.
#[tokio::test]
async fn retirer_une_salle_vide_nannonce_aucun_deplacement() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let lieu = service_lieux::enregistrer_lieu(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        formulaire_lieu(editions.cop31, "Pavillon"),
    )
    .await
    .expect("création")
    .detail
    .expect("la composition")
    .venues
    .remove(0);

    let salle = service_lieux::enregistrer_salle(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        formulaire_salle(lieu.id, "vide"),
    )
    .await
    .expect("création")
    .detail
    .expect("la composition")
    .venues[0]
        .rooms[0]
        .clone();

    let resultat =
        service_lieux::supprimer_salle(&bac.state, &bac.ctx(), cop31, RoomId::from(salle.id))
            .await
            .expect("le retrait aboutit");

    assert_eq!(resultat.sessions_detached, 0);
}
