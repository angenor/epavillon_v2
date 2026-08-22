//! **L'administrateur décide ce qui part, et voit ce qui va partir.**
//!
//! Cette histoire passe **avant** le calendrier et les envois, contre l'ordre
//! des priorités : rien ne sème de règle de rappel, et sans écriture les deux
//! autres ne se démontreraient qu'en posant une ligne à la main en SQL — ce qui
//! prouverait la lecture sans prouver le chemin réel.
//!
//! Ce que ces tests éprouvent et qu'aucune relecture ne remplace : **la règle de
//! séance remplace celle de l'édition sans s'y ajouter**, mesuré sur le nombre
//! de décalages rendus **et** sur l'origine ; et **une seconde écriture modifie
//! la première**, mesuré sur `count(*)`.

mod commun;

use commun::Bac;
use engagement::service::rules::{self, ReminderRulePayload};
use kernel::ErrorCode;
use uuid::Uuid;

/// Le défaut du modèle : 2 jours, 1 jour, 1 heure, 30 minutes. **Cumulés.**
const DEFAUT: [i32; 4] = [2880, 1440, 60, 30];

fn pour_ledition(edition: Uuid, minutes: &[i32]) -> ReminderRulePayload {
    ReminderRulePayload {
        event_id: Some(edition),
        session_id: None,
        offsets: Some(minutes.to_vec()),
        channels: None,
        type_code: None,
        template_id: None,
        is_active: None,
    }
}

fn pour_la_seance(seance: Uuid, minutes: &[i32]) -> ReminderRulePayload {
    ReminderRulePayload {
        event_id: None,
        session_id: Some(seance),
        offsets: Some(minutes.to_vec()),
        channels: None,
        type_code: None,
        template_id: None,
        is_active: None,
    }
}

async fn compter_les_regles(bac: &Bac) -> i64 {
    sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM engagement.reminder_rules"#)
        .fetch_one(bac.pool())
        .await
        .expect("comptage des règles")
}

/// **Une règle s'écrit avec ses quatre décalages et se relit en minutes.**
///
/// En minutes, et non en texte : `'1 day'` et `'24 hours'` sont le même
/// intervalle pour la base et deux chaînes différentes pour un écran, ce qui
/// suffirait à afficher deux fois le même rappel.
#[tokio::test]
async fn une_regle_secrit_avec_ses_quatre_decalages_et_se_relit_en_minutes() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let regle = rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &pour_ledition(terrain.edition, &DEFAUT),
    )
    .await
    .expect("écriture de la règle d'édition");

    assert_eq!(regle.event_id, Some(terrain.edition));
    assert_eq!(regle.session_id, None);
    // **Rangés du plus lointain au plus proche**, l'ordre dans lequel le modèle
    // écrit son défaut et celui dans lequel l'écran les lit.
    assert_eq!(regle.offsets, DEFAUT.to_vec());
    assert_eq!(regle.channels, vec!["email".to_owned()]);
    assert!(regle.is_active);
    assert_eq!(regle.created_by, Some(terrain.administratrice));

    // La lecture d'ensemble la retrouve.
    let regles = rules::lister(&bac.state, terrain.administratrice, terrain.edition)
        .await
        .expect("lecture des règles de l'édition");
    assert_eq!(regles.len(), 1);
    assert_eq!(regles[0].id, regle.id);
}

/// **Le défaut du modèle s'applique quand aucun décalage n'est fourni.**
#[tokio::test]
async fn sans_decalage_fourni_le_defaut_du_modele_sapplique() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let regle = rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &ReminderRulePayload {
            event_id: Some(terrain.edition),
            session_id: None,
            offsets: None,
            channels: None,
            type_code: None,
            template_id: None,
            is_active: None,
        },
    )
    .await
    .expect("écriture sans décalage");

    assert_eq!(regle.offsets, DEFAUT.to_vec());
}

/// **LA RÈGLE DE SÉANCE REMPLACE CELLE DE L'ÉDITION, SANS S'Y AJOUTER.**
///
/// Deux décalages posés sur la séance, **deux** rendus — et non six. L'origine
/// le dit : sans elle, une règle de séance à deux décalages ne se distingue pas
/// d'une règle d'édition qu'on aurait tronquée.
#[tokio::test]
async fn la_regle_de_seance_remplace_celle_de_ledition_sans_sy_ajouter() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &pour_ledition(terrain.edition, &DEFAUT),
    )
    .await
    .expect("la règle d'édition");

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &pour_la_seance(terrain.seance, &[1440, 60]),
    )
    .await
    .expect("la règle de séance");

    let applicable = rules::applicable(&bac.state, terrain.seance)
        .await
        .expect("lecture de la règle applicable")
        .expect("une règle s'applique");

    assert_eq!(applicable.offsets, vec![1440, 60]);
    assert_eq!(applicable.origin, "session");
    assert_eq!(applicable.origin_id, terrain.seance);
}

/// **Une séance sans règle propre rend celle de son édition**, avec l'origine
/// « édition » et l'identifiant de l'édition.
#[tokio::test]
async fn une_seance_sans_regle_propre_rend_celle_de_son_edition() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &pour_ledition(terrain.edition, &DEFAUT),
    )
    .await
    .expect("la règle d'édition");

    let applicable = rules::applicable(&bac.state, terrain.seance)
        .await
        .expect("lecture")
        .expect("celle de l'édition s'applique");

    assert_eq!(applicable.offsets, DEFAUT.to_vec());
    assert_eq!(applicable.origin, "event");
    assert_eq!(applicable.origin_id, terrain.edition);
}

/// **Une séance dont ni elle ni son édition n'ont de règle rend explicitement
/// qu'aucune ne s'applique** (FR-076).
///
/// C'est le cas courant sur une base neuve : rien ne sème de règle. Une liste
/// vide muette se confondrait avec « tout est parti ».
#[tokio::test]
async fn sans_aucune_regle_la_lecture_le_dit_explicitement() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let applicable = rules::applicable(&bac.state, terrain.seance)
        .await
        .expect("lecture");

    assert!(applicable.is_none(), "aucune règle ne doit s'appliquer");
}

/// **Une règle coupée ne s'applique plus**, sans être supprimée pour autant.
#[tokio::test]
async fn une_regle_inactive_ne_sapplique_plus() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &ReminderRulePayload {
            is_active: Some(false),
            ..pour_ledition(terrain.edition, &DEFAUT)
        },
    )
    .await
    .expect("écriture d'une règle coupée");

    assert!(rules::applicable(&bac.state, terrain.seance)
        .await
        .expect("lecture")
        .is_none());
    assert_eq!(compter_les_regles(&bac).await, 1, "elle existe toujours");
}

/// **Les refus de portée et de décalages, chacun sur son champ.**
#[tokio::test]
async fn les_refus_portent_chacun_sur_leur_champ() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Trois listes fautives, toutes refusées sur `offsets`.
    for (cas, minutes) in [
        ("liste vide", vec![]),
        ("décalage négatif", vec![1440, -30]),
        ("neuf décalages", (1..=9).collect::<Vec<i32>>()),
        ("décalage répété", vec![1440, 60, 1440]),
    ] {
        let erreur = rules::ecrire(
            &bac.state,
            &bac.ctx(),
            terrain.administratrice,
            &pour_ledition(terrain.edition, &minutes),
        )
        .await
        .expect_err(cas);

        assert_eq!(
            erreur.code,
            ErrorCode::EngagementReminderOffsetsInvalid,
            "{cas}"
        );
        assert_eq!(erreur.field.as_deref(), Some("offsets"), "{cas}");
        assert!(erreur.detail.is_some(), "{cas} doit dire lequel des cas");
    }

    // Portée double, puis portée absente : les deux sur `scope`.
    for (cas, event_id, session_id) in [
        ("portée double", Some(terrain.edition), Some(terrain.seance)),
        ("portée absente", None, None),
    ] {
        let erreur = rules::ecrire(
            &bac.state,
            &bac.ctx(),
            terrain.administratrice,
            &ReminderRulePayload {
                event_id,
                session_id,
                offsets: Some(DEFAUT.to_vec()),
                channels: None,
                type_code: None,
                template_id: None,
                is_active: None,
            },
        )
        .await
        .expect_err(cas);

        assert_eq!(
            erreur.code,
            ErrorCode::EngagementReminderScopeInvalid,
            "{cas}"
        );
        assert_eq!(erreur.field.as_deref(), Some("scope"), "{cas}");
    }

    assert_eq!(compter_les_regles(&bac).await, 0, "aucun refus n'a écrit");
}

/// **Une seconde écriture pour la même édition MODIFIE la première**, et
/// `count(*)` reste à un (FR-073).
///
/// Rendre un conflit dirait « impossible » là où l'administrateur voulait
/// simplement changer ses décalages.
#[tokio::test]
async fn une_seconde_ecriture_modifie_la_premiere() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let premiere = rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &pour_ledition(terrain.edition, &DEFAUT),
    )
    .await
    .expect("première écriture");

    let seconde = rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &pour_ledition(terrain.edition, &[1440, 30]),
    )
    .await
    .expect("seconde écriture");

    assert_eq!(premiere.id, seconde.id, "la même ligne a été modifiée");
    assert_eq!(seconde.offsets, vec![1440, 30]);
    assert_eq!(compter_les_regles(&bac).await, 1);
}

/// **Couper une règle annule les rappels encore à traiter qu'elle
/// gouvernait**, et en rend le nombre (FR-078).
///
/// Sans cette annulation, les rappels **déjà matérialisés** partiraient quand
/// même, et l'administrateur qui vient de retirer la règle les verrait arriver
/// sans comprendre.
#[tokio::test]
async fn couper_une_regle_annule_les_rappels_encore_a_traiter() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let regle = rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &pour_ledition(terrain.edition, &DEFAUT),
    )
    .await
    .expect("la règle");

    // **La fonction du modèle**, appelée telle quelle : c'est elle qui met un
    // travail par rappel en file, et ce module ne la redouble jamais.
    let crees = commun::materialiser_les_rappels(&bac, terrain.seance).await;
    assert_eq!(
        crees as usize,
        terrain.inscrits.len() * DEFAUT.len(),
        "un rappel par inscrit et par décalage"
    );

    let annules = rules::supprimer(&bac.state, &bac.ctx(), terrain.administratrice, regle.id)
        .await
        .expect("coupure de la règle");

    assert_eq!(annules, crees as i64);
    assert_eq!(
        commun::compter_rappels(&bac, terrain.seance, "cancelled").await,
        crees as i64
    );
    assert_eq!(compter_les_regles(&bac).await, 0);
}

/// **Un compte détaché sur une édition ne paramètre pas les rappels d'une
/// autre.**
///
/// Règle métier n° 8, y compris quand l'utilisateur forge une URL. Le refus
/// prend la forme d'une absence : un 403 dirait que l'édition existe.
#[tokio::test]
async fn un_compte_detache_ne_parametre_pas_une_autre_edition() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Une seconde édition, qu'elle n'administre pas. Écrite ici plutôt que par
    // le harnais : celui-ci pose l'adresse d'URL de la COP31, unique par nature.
    let autre_edition = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at, has_pavilion)
           SELECT s.id, 'COP32', 2028,
                  '{"fr":"COP32","en":"COP32"}'::jsonb, 'COP32', 'cop32'::platform.slug,
                  '{"fr":"Édition suivante.","en":"Next edition."}'::jsonb,
                  'announced', 'online', 'UTC'::platform.timezone_name,
                  now() + interval '2 years', now() + interval '2 years 10 days', false
             FROM event.event_series s WHERE s.code = 'cop_climate'
        RETURNING id"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("seconde édition");

    for (cas, payload) in [
        ("l'édition d'à côté", pour_ledition(autre_edition, &DEFAUT)),
        (
            "une édition inexistante",
            pour_ledition(Uuid::now_v7(), &DEFAUT),
        ),
        (
            "une séance inexistante",
            pour_la_seance(Uuid::now_v7(), &DEFAUT),
        ),
    ] {
        let erreur = rules::ecrire(&bac.state, &bac.ctx(), terrain.administratrice, &payload)
            .await
            .expect_err(cas);
        assert_eq!(erreur.code, ErrorCode::NotFound, "{cas}");
    }

    // La lecture est gardée de la même façon.
    let lecture = rules::lister(&bac.state, terrain.administratrice, autre_edition)
        .await
        .expect_err("la lecture aussi est bornée par le périmètre");
    assert_eq!(lecture.code, ErrorCode::NotFound);

    // Et l'animatrice, qui n'administre rien, n'écrit pas non plus.
    let sans_droit = rules::ecrire(
        &bac.state,
        &bac.ctx(),
        terrain.animatrice,
        &pour_ledition(terrain.edition, &DEFAUT),
    )
    .await
    .expect_err("une adhésion n'administre rien");
    assert_eq!(sans_droit.code, ErrorCode::NotFound);

    assert_eq!(compter_les_regles(&bac).await, 0);
}
