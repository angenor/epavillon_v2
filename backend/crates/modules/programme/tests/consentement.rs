//! **Le consentement aux réponses sensibles, et sa preuve.**
//!
//! `is_sensitive` est une marque **sans effet** en base : c'est le service qui
//! en tire le consentement, et la preuve vit dans la transaction de la donnée
//! qu'elle couvre — sinon on refuse sans preuve, ou l'on accepte et la preuve se
//! perd si le relais meurt.

mod commun;

use commun::seances::{self, Souhaits};
use commun::Bac;
use kernel::error::ErrorCode;
use programme::domain::transitions::ProposalStatus;
use programme::service::{registration, transition};
use serde_json::json;
use uuid::Uuid;

/// Un champ **sensible** ajouté au formulaire par défaut.
async fn champ_sensible(bac: &Bac) {
    sqlx::query!(
        r#"INSERT INTO programme.registration_form_fields
               (form_id, code, label, field_type, is_required, is_sensitive, sort_order)
           SELECT id, 'besoin_accessibilite',
                  '{"fr":"Besoin d''accessibilité"}'::jsonb, 'text', false, true, 70
             FROM programme.registration_forms WHERE code = 'default'"#
    )
    .execute(bac.pool())
    .await
    .expect("champ sensible");
}

async fn seance_ouverte(bac: &Bac, terrain: &commun::Terrain, slug: &str) -> Uuid {
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
    seances::ouvrir_les_inscriptions(bac, id, None, false).await;
    id
}

/// **Sans consentement, un refus qui nomme le champ** ; avec, l'inscription
/// aboutit et la preuve est **relue** dans le registre des consentements.
#[tokio::test]
async fn une_reponse_sensible_exige_un_consentement_dont_la_preuve_est_conservee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    champ_sensible(&bac).await;

    let seance = seance_ouverte(&bac, &terrain, "atelier").await;
    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;

    let avec_donnee_sensible = || registration::SessionRegisterPayload {
        answers: json!({ "country": "SN", "besoin_accessibilite": "Boucle magnétique" }),
        ..seances::reponses_valides()
    };

    let erreur = seances::sinscrire(&bac, seance, Some(personne), avec_donnee_sensible())
        .await
        .expect_err("sans accord, la réponse sensible est refusée");
    assert_eq!(erreur.code, ErrorCode::RegistrationConsentRequired);
    assert_eq!(
        erreur.field.as_deref(),
        Some("besoin_accessibilite"),
        "l'écran doit savoir quelle case afficher"
    );

    let preuves_avant = preuves(&bac, personne).await;
    assert_eq!(preuves_avant, 0, "aucune preuve n'a été écrite au passage");

    seances::sinscrire(
        &bac,
        seance,
        Some(personne),
        registration::SessionRegisterPayload {
            sensitive_data_consent: true,
            ..avec_donnee_sensible()
        },
    )
    .await
    .expect("avec l'accord, l'inscription aboutit");

    let ligne = sqlx::query!(
        r#"SELECT purpose, is_granted, policy_version, source
             FROM identity.consents
            WHERE person_id = $1"#,
        personne
    )
    .fetch_one(bac.pool())
    .await
    .expect("la preuve est conservée");

    assert_eq!(ligne.purpose, "registration_sensitive_data");
    assert!(ligne.is_granted);
    assert_eq!(
        ligne.policy_version, "2026-01",
        "la preuve nomme le texte accepté"
    );
    assert_eq!(ligne.source.as_deref(), Some("registration_form"));
}

/// **Une seule finalité, quel que soit le nombre de champs sensibles** :
/// multiplier les finalités rendrait le retrait ingérable — retirer lequel ?
#[tokio::test]
async fn une_seule_finalite_quel_que_soit_le_nombre_de_champs() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    champ_sensible(&bac).await;

    sqlx::query!(
        r#"INSERT INTO programme.registration_form_fields
               (form_id, code, label, field_type, is_sensitive, sort_order)
           SELECT id, 'regime_alimentaire',
                  '{"fr":"Régime alimentaire"}'::jsonb, 'text', true, 80
             FROM programme.registration_forms WHERE code = 'default'"#
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let seance = seance_ouverte(&bac, &terrain, "atelier").await;
    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;

    seances::sinscrire(
        &bac,
        seance,
        Some(personne),
        registration::SessionRegisterPayload {
            answers: json!({
                "country": "SN",
                "besoin_accessibilite": "Boucle magnétique",
                "regime_alimentaire": "Végétarien"
            }),
            sensitive_data_consent: true,
            ..seances::reponses_valides()
        },
    )
    .await
    .expect("l'inscription aboutit");

    assert_eq!(preuves(&bac, personne).await, 1, "une ligne, une finalité");
}

/// Un accord **déjà donné** vaut : redemander à chaque inscription ferait cocher
/// la même case dix fois pour la même personne.
#[tokio::test]
async fn un_accord_deja_donne_dispense_de_le_redemander() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    champ_sensible(&bac).await;

    let premiere = seance_ouverte(&bac, &terrain, "premiere").await;
    let seconde = seance_ouverte(&bac, &terrain, "seconde").await;
    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;

    let charge = |accord: bool| registration::SessionRegisterPayload {
        answers: json!({ "country": "SN", "besoin_accessibilite": "Boucle magnétique" }),
        sensitive_data_consent: accord,
        ..seances::reponses_valides()
    };

    seances::sinscrire(&bac, premiere, Some(personne), charge(true))
        .await
        .unwrap();

    seances::sinscrire(&bac, seconde, Some(personne), charge(false))
        .await
        .expect("l'accord donné à la première inscription vaut pour la seconde");

    assert_eq!(
        preuves(&bac, personne).await,
        1,
        "et aucune seconde preuve n'est écrite"
    );
}

async fn preuves(bac: &Bac, personne: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.consents WHERE person_id = $1"#,
        personne
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture des consentements")
}
