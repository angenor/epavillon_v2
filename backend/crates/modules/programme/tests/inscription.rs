//! **S'inscrire, annuler, promouvoir, rejoindre.**
//!
//! La jauge, la liste d'attente et les quatre fenêtres — dont deux que la base
//! ne vérifie pas.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::registration::IssueDInscription;
use programme::domain::transitions::ProposalStatus;
use programme::service::{registration, transition};
use uuid::Uuid;

async fn seance_ouverte(
    bac: &Bac,
    terrain: &Terrain,
    slug: &str,
    jauge: Option<i32>,
    liste_dattente: bool,
) -> Uuid {
    let dossier = seances::dossier_pret(bac, terrain, slug, slug, Souhaits::default()).await;
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
    seances::ouvrir_les_inscriptions(bac, id, jauge, liste_dattente).await;
    id
}

/// **Une inscription écrit UNE ligne d'outbox, pas deux.** Le déclencheur émet
/// déjà ; un service qui émettrait à son tour produirait deux courriels de
/// confirmation et deux jeux de rappels, et le doublon ne se verrait qu'en
/// production.
#[tokio::test]
async fn une_inscription_ecrit_une_seule_ligne_doutbox() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let seance = seance_ouverte(&bac, &terrain, "atelier", None, false).await;

    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;
    let issue = seances::sinscrire(&bac, seance, Some(personne), seances::reponses_valides())
        .await
        .expect("l'inscription aboutit");

    let inscription = seances::identifiant_de(&issue);
    let emis = commun::evenements_emis(&bac, inscription).await;

    assert_eq!(
        emis,
        vec!["programme.registration.created"],
        "une ligne, celle du déclencheur"
    );
}

/// **La jauge tient, et la bascule est laissée à la base.** Avec liste
/// d'attente : la position suivante, sans trou. Sans : un refus portant le
/// nombre de places, **relu sur la séance** et jamais extrait d'une phrase.
#[tokio::test]
async fn la_jauge_bascule_en_attente_ou_refuse_en_portant_les_places() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    let avec_attente = seance_ouverte(&bac, &terrain, "avec-attente", Some(1), true).await;
    let premiere = commun::personne(&bac, "une@example.org", "Aïda", "Ba").await;
    let seconde = commun::personne(&bac, "deux@example.org", "Bineta", "Fall").await;

    seances::sinscrire(
        &bac,
        avec_attente,
        Some(premiere),
        seances::reponses_valides(),
    )
    .await
    .unwrap();
    let issue = seances::sinscrire(
        &bac,
        avec_attente,
        Some(seconde),
        seances::reponses_valides(),
    )
    .await
    .unwrap();

    match issue {
        IssueDInscription::Waitlisted { position, .. } => assert_eq!(position, 1),
        autre => panic!("attendait une place en liste d'attente : {autre:?}"),
    }

    let sans_attente = seance_ouverte(&bac, &terrain, "sans-attente", Some(1), false).await;
    seances::sinscrire(
        &bac,
        sans_attente,
        Some(premiere),
        seances::reponses_valides(),
    )
    .await
    .unwrap();
    let issue = seances::sinscrire(
        &bac,
        sans_attente,
        Some(seconde),
        seances::reponses_valides(),
    )
    .await
    .unwrap();

    match issue {
        IssueDInscription::Full { capacity } => assert_eq!(
            capacity, 1,
            "le nombre de places est relu sur la séance, jamais extrait du message"
        ),
        autre => panic!("attendait un refus de jauge : {autre:?}"),
    }
}

/// Annuler une inscription **confirmée** promeut **exactement une** personne ;
/// annuler une inscription **en attente** n'en promeut aucune.
#[tokio::test]
async fn lannulation_promeut_exactement_le_nombre_de_places_liberees() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let seance = seance_ouverte(&bac, &terrain, "atelier", Some(1), true).await;

    let confirmee = commun::personne(&bac, "une@example.org", "Aïda", "Ba").await;
    let attente = commun::personne(&bac, "deux@example.org", "Bineta", "Fall").await;
    let attente_bis = commun::personne(&bac, "trois@example.org", "Coura", "Sy").await;

    let premiere = seances::identifiant_de(
        &seances::sinscrire(&bac, seance, Some(confirmee), seances::reponses_valides())
            .await
            .unwrap(),
    );
    let deuxieme = seances::identifiant_de(
        &seances::sinscrire(&bac, seance, Some(attente), seances::reponses_valides())
            .await
            .unwrap(),
    );
    let troisieme = seances::identifiant_de(
        &seances::sinscrire(&bac, seance, Some(attente_bis), seances::reponses_valides())
            .await
            .unwrap(),
    );

    // Annuler une inscription EN ATTENTE ne libère rien.
    let resultat = seances::annuler(&bac, troisieme, seance, None)
        .await
        .unwrap();
    assert_eq!(resultat.promoted, 0);
    assert_eq!(
        seances::statut_dinscription(&bac, deuxieme).await,
        "waitlisted"
    );

    // Annuler la CONFIRMÉE libère une place, et une seule.
    let resultat = seances::annuler(&bac, premiere, seance, Some("Empêchement."))
        .await
        .unwrap();
    assert_eq!(resultat.promoted, 1);
    assert_eq!(
        seances::statut_dinscription(&bac, deuxieme).await,
        "registered",
        "la personne en attente est promue, et la base l'annonce elle-même"
    );
}

/// Une seconde inscription vivante est refusée — **et rendue** : la ligne
/// existante est relue, ce que l'écran affiche tel quel. Une réinscription
/// **après annulation** est acceptée : l'index d'unicité est partiel.
#[tokio::test]
async fn une_seconde_inscription_rend_lexistante_et_la_reinscription_passe() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let seance = seance_ouverte(&bac, &terrain, "atelier", None, false).await;

    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;
    let premiere = seances::identifiant_de(
        &seances::sinscrire(&bac, seance, Some(personne), seances::reponses_valides())
            .await
            .unwrap(),
    );

    let issue = seances::sinscrire(&bac, seance, Some(personne), seances::reponses_valides())
        .await
        .unwrap();
    match issue {
        IssueDInscription::AlreadyRegistered { registration } => {
            assert_eq!(
                registration.get("id").and_then(|v| v.as_str()),
                Some(premiere.to_string().as_str())
            );
        }
        autre => panic!("attendait « déjà inscrit » : {autre:?}"),
    }

    seances::annuler(&bac, premiere, seance, None)
        .await
        .unwrap();

    let issue = seances::sinscrire(&bac, seance, Some(personne), seances::reponses_valides())
        .await
        .unwrap();
    assert!(matches!(issue, IssueDInscription::Registered { .. }));
}

/// **Les quatre fenêtres rendent quatre motifs distincts** — dont deux que la
/// base ne vérifie pas du tout (écart n° 115).
#[tokio::test]
async fn les_quatre_fenetres_rendent_quatre_motifs_distincts() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;

    // 1. Séance annulée.
    let annulee = seance_ouverte(&bac, &terrain, "annulee", None, false).await;
    sqlx::query!(
        r#"UPDATE programme.sessions
              SET status = 'cancelled',
                  cancelled_reason = '{"fr":"Annulée."}'::jsonb
            WHERE id = $1"#,
        annulee
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let erreur = seances::sinscrire(&bac, annulee, Some(personne), seances::reponses_valides())
        .await
        .expect_err("une séance annulée ne prend pas d'inscription");
    assert_eq!(erreur.code, ErrorCode::RegistrationNotAccepted);

    // 2. Séance qui ne prend pas d'inscription — la base l'ignore.
    let sans = seance_ouverte(&bac, &terrain, "sans-inscription", None, false).await;
    sqlx::query!(
        "UPDATE programme.sessions SET registration_required = false WHERE id = $1",
        sans
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let erreur = seances::sinscrire(&bac, sans, Some(personne), seances::reponses_valides())
        .await
        .expect_err("registration_required n'est lu par personne en base");
    assert_eq!(erreur.code, ErrorCode::RegistrationNotAccepted);

    // 3. Pas encore ouvertes — la base ne vérifie que la clôture.
    let plus_tard = seance_ouverte(&bac, &terrain, "plus-tard", None, false).await;
    sqlx::query!(
        "UPDATE programme.sessions
            SET registration_opens_at = now() + interval '7 days'
          WHERE id = $1",
        plus_tard
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let issue = seances::sinscrire(&bac, plus_tard, Some(personne), seances::reponses_valides())
        .await
        .unwrap();
    assert!(matches!(issue, IssueDInscription::NotOpenYet { .. }));

    // 4. Closes.
    let close = seance_ouverte(&bac, &terrain, "close", None, false).await;
    sqlx::query!(
        "UPDATE programme.sessions
            SET registration_opens_at = now() - interval '30 days',
                registration_closes_at = now() - interval '1 day'
          WHERE id = $1",
        close
    )
    .execute(bac.pool())
    .await
    .unwrap();
    let issue = seances::sinscrire(&bac, close, Some(personne), seances::reponses_valides())
        .await
        .unwrap();
    assert!(matches!(issue, IssueDInscription::Closed { .. }));
}

/// **L'inscription sans compte** : la personne est créée sans compte, retrouvée
/// par son adresse à la seconde inscription, et refusée quand le formulaire ne
/// l'admet pas.
#[tokio::test]
async fn linscription_sans_compte_cree_puis_retrouve_la_personne() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let premiere = seance_ouverte(&bac, &terrain, "premiere", None, false).await;
    let seconde = seance_ouverte(&bac, &terrain, "seconde", None, false).await;

    let invitee = registration::Invite {
        email: "invitee@example.org".to_owned(),
        first_name: "Awa".to_owned(),
        last_name: "Sow".to_owned(),
        civility: None,
    };

    let charge = || registration::SessionRegisterPayload {
        guest: Some(registration::Invite {
            email: invitee.email.clone(),
            first_name: invitee.first_name.clone(),
            last_name: invitee.last_name.clone(),
            civility: None,
        }),
        ..seances::reponses_valides()
    };

    seances::sinscrire(&bac, premiere, None, charge())
        .await
        .expect("le formulaire par défaut admet l'anonyme");

    let fiche = commun::fiche(&bac, &invitee.email)
        .await
        .expect("la personne a été créée");
    assert_eq!(
        fiche.1, "Awa",
        "le prénom saisi, jamais déduit de l'adresse"
    );

    seances::sinscrire(&bac, seconde, None, charge())
        .await
        .expect("la seconde inscription aboutit");
    let seconde_fiche = commun::fiche(&bac, &invitee.email).await.unwrap();
    assert_eq!(
        seconde_fiche.0, fiche.0,
        "la même personne, retrouvée par son adresse"
    );

    // Un formulaire qui n'admet pas l'anonyme.
    let ferme = seance_ouverte(&bac, &terrain, "ferme", None, false).await;
    sqlx::query!(
        "UPDATE programme.registration_forms SET allows_anonymous = false WHERE code = 'default'"
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let erreur = seances::sinscrire(&bac, ferme, None, charge())
        .await
        .expect_err("sans compte, l'inscription est refusée");
    assert_eq!(erreur.code, ErrorCode::RegistrationAccountRequired);
}

/// La **première présence** est écrite une seule fois ; « mes inscriptions » ne
/// rend que les siennes.
#[tokio::test]
async fn la_premiere_presence_et_mes_inscriptions() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let seance = seance_ouverte(&bac, &terrain, "atelier", None, false).await;

    let moi = commun::personne(&bac, "moi@example.org", "Moi", "Même").await;
    let autre = commun::personne(&bac, "autre@example.org", "Autre", "Personne").await;

    let mienne = seances::identifiant_de(
        &seances::sinscrire(&bac, seance, Some(moi), seances::reponses_valides())
            .await
            .unwrap(),
    );
    seances::sinscrire(&bac, seance, Some(autre), seances::reponses_valides())
        .await
        .unwrap();

    let premier = registration::rejoindre(&bac.state, &bac.ctx(), mienne.into())
        .await
        .unwrap();
    let second = registration::rejoindre(&bac.state, &bac.ctx(), mienne.into())
        .await
        .unwrap();
    assert_eq!(
        premier.get("joined_at"),
        second.get("joined_at"),
        "un second clic n'écrase pas la première présence"
    );

    let miennes = registration::mes_inscriptions(&bac.state, moi)
        .await
        .unwrap();
    assert_eq!(miennes.len(), 1);
    assert_eq!(
        miennes[0].get("id").and_then(|v| v.as_str()),
        Some(mienne.to_string().as_str())
    );
}

/// 🔴 **L'écart n° 125, éprouvé plutôt que découvert** : le déclencheur revalide
/// à chaque changement d'état, donc **on ne peut pas annuler son inscription à
/// une séance annulée**. Le service traduit en refus nommé, jamais un 500.
#[tokio::test]
async fn annuler_une_inscription_a_une_seance_annulee_est_refuse_par_un_code() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let seance = seance_ouverte(&bac, &terrain, "atelier", None, false).await;

    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;
    let inscription = seances::identifiant_de(
        &seances::sinscrire(&bac, seance, Some(personne), seances::reponses_valides())
            .await
            .unwrap(),
    );

    sqlx::query!(
        r#"UPDATE programme.sessions
              SET status = 'cancelled',
                  cancelled_reason = '{"fr":"Annulée."}'::jsonb
            WHERE id = $1"#,
        seance
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let erreur = seances::annuler(&bac, inscription, seance, None)
        .await
        .expect_err("le déclencheur refuse la modification");

    assert_eq!(
        erreur.code,
        ErrorCode::RegistrationLocked,
        "un refus nommé, et non un 500"
    );
}

/// La liste **nominative** exige la permission de gérer les inscriptions ; un
/// compte qui n'a que celle de planifier est refusé (écart n° 119).
#[tokio::test]
async fn la_liste_nominative_exige_sa_propre_permission() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let seance = seance_ouverte(&bac, &terrain, "atelier", None, false).await;

    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;
    seances::sinscrire(&bac, seance, Some(personne), seances::reponses_valides())
        .await
        .unwrap();

    // Un rôle qui planifie **sans** gérer les inscriptions.
    let programmatrice = commun::personne(&bac, "programmation@ifdd.org", "Paule", "Kone").await;
    commun::attribuer(
        &bac,
        programmatrice,
        "programmer",
        "event",
        Some(terrain.edition),
    )
    .await;

    let planifie = kernel::auth::has_permission(
        bac.pool(),
        programmatrice,
        programme::domain::permissions::SESSION_SCHEDULE,
        kernel::auth::Scope::Event(terrain.edition),
    )
    .await
    .unwrap();
    let gere = kernel::auth::has_permission(
        bac.pool(),
        programmatrice,
        programme::domain::permissions::REGISTRATION_MANAGE,
        kernel::auth::Scope::Event(terrain.edition),
    )
    .await
    .unwrap();

    assert!(planifie, "elle compose la grille");
    assert!(
        !gere,
        "et ne peut pas ouvrir la liste des inscrits : une ligne de la table des droits"
    );

    // La lecture elle-même rend bien les inscrits, une fois la permission
    // accordée à qui la détient.
    let lignes = registration::liste_nominative(&bac.state, seance.into())
        .await
        .unwrap();
    assert_eq!(lignes.len(), 1);
    assert!(lignes[0].get("person").is_some(), "la liste est nominative");
}
