//! **Les six événements attendus — et l'absence d'événement là où rien ne doit
//! être émis** ([`contracts/events.md`]).
//!
//! ## Le constat qui gouverne ce test
//!
//! **Aucun déclencheur de `060_events.sql` n'émet d'événement de domaine.** Le
//! fichier ne porte que deux déclencheurs d'audit — sur l'édition et sur
//! l'appel — et cinq horodatages.
//!
//! C'est l'**inverse** du piège rencontré deux fois : `identity.anonymize_person()`
//! émettait déjà son événement, et `org.merge_organizations()` émet le sien
//! **et** marque la paire. Dans les deux cas, un service zélé aurait produit un
//! doublon. Ici la conséquence est symétrique : **le service émet tout lui-même,
//! et rien n'émet à sa place**. Un changement d'état non annoncé par le code
//! n'est annoncé par personne.
//!
//! ## La seconde moitié compte autant
//!
//! Journées, fils, lieux, salles, canaux et comité **n'émettent rien**, et c'est
//! une soustraction délibérée : aucun autre module n'a à y réagir. Émettre
//! « pour plus tard » remplit la file de messages que personne ne lit et qu'il
//! faudra un jour retirer. Un test qui ne vérifierait que les présences
//! laisserait cette dette s'installer sans bruit.

mod commun;

use commun::{
    formulaire, formulaire_appel, formulaire_canal, formulaire_fil, formulaire_lieu, Bac,
};
use event::domain::ids::{CallId, ChannelId, EventId, TrackId, VenueId};
use event::domain::tabs::{CommitteePayload, CommitteeSeat};
use event::service::{
    call as service_appel, channels as service_canaux, committee as service_comite,
    days as service_journees, edition_write, publication, tracks as service_fils,
    venues as service_lieux,
};

/// Tous les événements de l'outbox, quel que soit leur agrégat.
async fn tous_les_evenements(bac: &Bac) -> Vec<String> {
    sqlx::query_scalar!("SELECT event_type FROM platform.outbox_events ORDER BY occurred_at, id")
        .fetch_all(bac.pool())
        .await
        .expect("lecture de l'outbox")
}

#[tokio::test]
async fn les_deux_evenements_dune_edition_sont_emis() {
    let bac = Bac::monter().await;
    let acteur = commun::auteur(&bac).await;

    let creee = edition_write::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        formulaire("edition-annoncee", "Édition annoncée"),
    )
    .await
    .expect("création");
    let edition = creee.edition.expect("l'édition créée");

    assert_eq!(
        commun::evenements_emis(&bac, edition.id).await,
        vec!["event.edition.created".to_owned()]
    );

    edition_write::modifier(
        &bac.state,
        &bac.ctx(),
        acteur,
        EventId::from(edition.id),
        formulaire("edition-annoncee", "Édition renommée"),
    )
    .await
    .expect("modification");

    assert_eq!(
        commun::evenements_emis(&bac, edition.id).await,
        vec![
            "event.edition.created".to_owned(),
            "event.edition.updated".to_owned()
        ]
    );
}

/// **Les trois événements de l'appel**, chacun sur son changement d'état.
#[tokio::test]
async fn louverture_la_cloture_et_la_prolongation_sont_annoncees() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = commun::auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    // Un appel créé en brouillon n'annonce rien : rien n'a changé d'état.
    let brouillon = service_appel::creer(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        formulaire_appel(editions.cop31, "cop31"),
    )
    .await
    .expect("création");
    let appel = brouillon.call.expect("l'appel créé");
    let call_id = CallId::from(appel.id);

    assert!(
        commun::evenements_emis(&bac, appel.id).await.is_empty(),
        "un brouillon n'ouvre rien"
    );

    // Ouverture.
    let mut ouverture = formulaire_appel(editions.cop31, "cop31");
    ouverture.status = "open".to_owned();
    service_appel::modifier(&bac.state, &bac.ctx(), acteur, cop31, call_id, ouverture)
        .await
        .expect("ouverture");

    // Prolongation.
    let mut prolongation = formulaire_appel(editions.cop31, "cop31");
    prolongation.status = "open".to_owned();
    prolongation.extended_until = Some(prolongation.closes_at + time::Duration::days(15));
    service_appel::modifier(&bac.state, &bac.ctx(), acteur, cop31, call_id, prolongation)
        .await
        .expect("prolongation");

    // Clôture.
    let mut cloture = formulaire_appel(editions.cop31, "cop31");
    cloture.status = "closed".to_owned();
    service_appel::modifier(&bac.state, &bac.ctx(), acteur, cop31, call_id, cloture)
        .await
        .expect("clôture");

    assert_eq!(
        commun::evenements_emis(&bac, appel.id).await,
        vec![
            "event.call.opened".to_owned(),
            "event.call.deadline_extended".to_owned(),
            "event.call.closed".to_owned()
        ]
    );
}

/// **L'échéance initiale voyage avec la nouvelle.** C'est celle qui a été
/// annoncée aux organisations, et un rappel qui l'ignore dit une contre-vérité.
#[tokio::test]
async fn la_prolongation_porte_lecheance_initiale_avec_la_nouvelle() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = commun::auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let mut ouverture = formulaire_appel(editions.cop31, "cop31");
    ouverture.status = "open".to_owned();
    let cloture_annoncee = ouverture.closes_at;
    let appel = service_appel::creer(&bac.state, &bac.ctx(), acteur, cop31, ouverture)
        .await
        .expect("ouverture")
        .call
        .expect("l'appel");

    let mut prolongation = formulaire_appel(editions.cop31, "cop31");
    prolongation.status = "open".to_owned();
    prolongation.extended_until = Some(cloture_annoncee + time::Duration::days(15));
    service_appel::modifier(
        &bac.state,
        &bac.ctx(),
        acteur,
        cop31,
        CallId::from(appel.id),
        prolongation,
    )
    .await
    .expect("prolongation");

    let charge = sqlx::query_scalar!(
        "SELECT payload FROM platform.outbox_events
          WHERE aggregate_id = $1 AND event_type = 'event.call.deadline_extended'",
        appel.id
    )
    .fetch_one(bac.pool())
    .await
    .expect("l'annonce de prolongation");

    assert!(
        charge.get("initial_deadline").is_some(),
        "l'échéance initiale voyage : {charge}"
    );
    assert!(charge.get("new_deadline").is_some());
    assert_ne!(charge["initial_deadline"], charge["new_deadline"]);
}

/// **Ce qui n'émet rien.** Six écritures, aucun événement — et c'est une
/// soustraction délibérée, pas un oubli.
#[tokio::test]
async fn les_journees_fils_lieux_salles_canaux_et_comite_nemettent_rien() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = commun::auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    // Un appel ouvert, pour pouvoir composer un comité. Son ouverture est le
    // SEUL événement attendu de tout ce test.
    let mut ouverture = formulaire_appel(editions.cop31, "cop31");
    ouverture.status = "open".to_owned();
    let appel = service_appel::creer(&bac.state, &bac.ctx(), acteur, cop31, ouverture)
        .await
        .expect("ouverture")
        .call
        .expect("l'appel");

    let attendus = vec!["event.call.opened".to_owned()];
    assert_eq!(tous_les_evenements(&bac).await, attendus);

    // 1. Journées du calendrier — génération et habillage.
    service_journees::generer(&bac.state, &bac.ctx(), cop31, false)
        .await
        .expect("génération");

    // 2. Fil de programmation — création puis suppression.
    let fil = service_fils::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        formulaire_fil(editions.cop31, "journee_finance"),
    )
    .await
    .expect("création du fil")
    .detail
    .expect("la composition")
    .tracks
    .remove(0);
    service_fils::supprimer(&bac.state, &bac.ctx(), cop31, TrackId::from(fil.id))
        .await
        .expect("suppression du fil");

    // 3 et 4. Lieu et salle.
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
    service_lieux::enregistrer_salle(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        commun::formulaire_salle(lieu.id, "baobab"),
    )
    .await
    .expect("création de la salle");
    service_lieux::supprimer_lieu(&bac.state, &bac.ctx(), cop31, VenueId::from(lieu.id))
        .await
        .expect("suppression du lieu");

    // 5. Canal de diffusion — création puis retrait.
    let canal = service_canaux::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        None,
        formulaire_canal(editions.cop31, "cop31_direct", true),
    )
    .await
    .expect("création du canal")
    .detail
    .expect("la composition")
    .channels
    .into_iter()
    .find(|c| c.code == "cop31_direct")
    .expect("le canal créé");
    service_canaux::retirer(&bac.state, &bac.ctx(), cop31, ChannelId::from(canal.id))
        .await
        .expect("retrait du canal");

    // 6. Comité de sélection.
    let membre = commun::personne(&bac, "membre@ifdd.francophonie.org", "Yann", "Corbeil").await;
    service_comite::enregistrer(
        &bac.state,
        &bac.ctx(),
        cop31,
        CallId::from(appel.id),
        CommitteePayload {
            call_id: None,
            members: vec![CommitteeSeat {
                person_id: membre,
                is_lead: true,
                workload_cap: None,
            }],
        },
    )
    .await
    .expect("composition du comité");

    assert_eq!(
        tous_les_evenements(&bac).await,
        attendus,
        "six écritures d'onglet, et pas un événement de plus : aucun autre module \
         n'a à y réagir, et émettre « pour plus tard » remplit la file de messages \
         que personne ne lit"
    );
}

/// **Le sixième événement** : la publication de la programmation.
#[tokio::test]
async fn la_publication_annonce_le_predicat_exact() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    publication::publier(&bac.state, &bac.ctx(), EventId::from(editions.cop31))
        .await
        .expect("publication");

    let charge = sqlx::query_scalar!(
        "SELECT payload FROM platform.outbox_events
          WHERE aggregate_id = $1 AND event_type = 'event.programme.published'",
        editions.cop31
    )
    .fetch_one(bac.pool())
    .await
    .expect("l'annonce de publication");

    let selection = charge
        .get("selection")
        .expect("le prédicat voyage avec l'annonce");
    assert_eq!(
        selection["statuses"],
        serde_json::json!(["planned", "scheduled"]),
        "le consommateur publie CE prédicat et pas un autre"
    );
    assert_eq!(selection["only_unpublished"], serde_json::json!(true));
    assert!(charge.get("published_count").is_some());
}

/// **Les six types du contrat, et pas un de plus.** Le format à trois segments
/// est tenu par `ck_outbox_event_type_format` ; ce que ce test ajoute, c'est
/// qu'aucun type n'a été inventé en chemin.
#[tokio::test]
async fn aucun_type_devenement_hors_du_contrat() {
    use contracts::event as contrat;

    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = commun::auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let mut ouverture = formulaire_appel(editions.cop31, "cop31");
    ouverture.status = "open".to_owned();
    service_appel::creer(&bac.state, &bac.ctx(), acteur, cop31, ouverture)
        .await
        .expect("ouverture");
    publication::publier(&bac.state, &bac.ctx(), cop31)
        .await
        .expect("publication");

    let contrat_complet = [
        contrat::EDITION_CREATED,
        contrat::EDITION_UPDATED,
        contrat::CALL_OPENED,
        contrat::CALL_CLOSED,
        contrat::CALL_DEADLINE_EXTENDED,
        contrat::PROGRAMME_PUBLISHED,
    ];

    for emis in tous_les_evenements(&bac).await {
        assert!(
            contrat_complet.contains(&emis.as_str()),
            "« {emis} » n'est pas au contrat du module"
        );
    }
}
