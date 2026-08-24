//! **Le formulaire vient de la base, et les réponses sont validées contre lui.**
//!
//! L'écart n° 114 en un test : le déclencheur ne vérifie **rien** lorsque la
//! séance ne porte pas de formulaire attaché — le cas courant.

mod commun;

use commun::seances::{self, Souhaits};
use commun::{Bac, Terrain};
use kernel::error::ErrorCode;
use programme::domain::transitions::ProposalStatus;
use programme::service::{registration, transition};
use serde_json::json;
use uuid::Uuid;

async fn seance_ouverte(bac: &Bac, terrain: &Terrain, slug: &str) -> Uuid {
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

/// 🔴 **Sur une séance SANS formulaire attaché**, celui de l'édition — à défaut
/// de la plateforme — est rendu, ses champs inactifs absents, ses options de
/// taxonomie résolues. Et **une inscription sans le pays est refusée** : ni la
/// base ni personne d'autre ne l'aurait vue (écart n° 114).
#[tokio::test]
async fn le_formulaire_applicable_est_resolu_et_ses_reponses_exigees() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let seance = seance_ouverte(&bac, &terrain, "atelier").await;

    // La séance ne porte AUCUN formulaire attaché : c'est le cas courant.
    let attache = sqlx::query_scalar!(
        "SELECT registration_form_id FROM programme.sessions WHERE id = $1",
        seance
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(attache, None);

    // Un champ désactivé, qui ne doit ni s'afficher ni s'accepter.
    sqlx::query!(
        r#"INSERT INTO programme.registration_form_fields
               (form_id, code, label, field_type, is_active, sort_order)
           SELECT id, 'retire', '{"fr":"Question retirée"}'::jsonb, 'text', false, 90
             FROM programme.registration_forms WHERE code = 'default'"#
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let formulaire = registration::formulaire(&bac.state, seance.into())
        .await
        .expect("le formulaire de la plateforme s'applique");

    let codes: Vec<&str> = formulaire
        .fields
        .iter()
        .filter_map(|f| f.get("code").and_then(|c| c.as_str()))
        .collect();
    assert!(codes.contains(&"country"), "le pays est posé par le modèle");
    assert!(
        !codes.contains(&"retire"),
        "un champ inactif n'est pas rendu"
    );

    // Les options d'un champ adossé à une taxonomie sont **résolues** : sans
    // cela, l'écran rechargerait la taxonomie et les libellés finiraient figés
    // dans le frontend.
    let source = formulaire
        .fields
        .iter()
        .find(|f| f.get("code").and_then(|c| c.as_str()) == Some("referral_source"))
        .expect("le canal d'acquisition est posé par le modèle");
    let valeurs = source
        .get("options")
        .and_then(|o| o.get("values"))
        .and_then(|v| v.as_array())
        .expect("les options sont résolues");
    assert!(!valeurs.is_empty());
    assert!(
        valeurs[0].get("label").is_some(),
        "avec leur libellé traduit"
    );

    // 🔴 Et l'inscription **sans le pays** est refusée. Si elle passait, ni la
    // base ni le service n'auraient vérifié.
    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;
    let erreur = seances::sinscrire(
        &bac,
        seance,
        Some(personne),
        registration::SessionRegisterPayload {
            answers: json!({}),
            ..seances::reponses_valides()
        },
    )
    .await
    .expect_err("le pays est obligatoire, et personne d'autre ne le vérifie");

    assert_eq!(erreur.code, ErrorCode::RegistrationAnswerInvalid);
    assert_eq!(erreur.field.as_deref(), Some("country"));
}

/// **Les six familles de refus, chacune nommant son champ.**
#[tokio::test]
async fn les_six_familles_de_refus_nomment_leur_champ() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let seance = seance_ouverte(&bac, &terrain, "atelier").await;
    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;

    // Deux champs de plus pour couvrir les bornes et le choix multiple.
    sqlx::query!(
        r#"INSERT INTO programme.registration_form_fields
               (form_id, code, label, field_type, is_required, options, validation, sort_order)
           SELECT id, v.code, v.label::jsonb, v.nature::programme.form_field_type,
                  v.obligatoire, v.options::jsonb, v.regles::jsonb, v.rang
             FROM programme.registration_forms f
             CROSS JOIN (VALUES
                 ('participants', '{"fr":"Nombre de participants"}', 'number', false,
                  '{}', '{"min": 1, "max": 50}', 50),
                 ('interets', '{"fr":"Centres d''intérêt"}', 'multiple_choice', true,
                  '{"values":[{"value":"eau","label":{"fr":"Eau"}}]}', '{}', 60)
             ) AS v(code, label, nature, obligatoire, options, regles, rang)
            WHERE f.code = 'default'"#
    )
    .execute(bac.pool())
    .await
    .unwrap();

    let cas = [
        // 1. Obligatoire absent.
        (json!({ "interets": ["eau"] }), "country"),
        // 2. Type incompatible.
        (
            json!({ "country": "SN", "interets": ["eau"], "participants": "beaucoup" }),
            "participants",
        ),
        // 3. Hors options.
        (json!({ "country": "SN", "interets": ["feu"] }), "interets"),
        // 4. Hors bornes.
        (
            json!({ "country": "SN", "interets": ["eau"], "participants": 500 }),
            "participants",
        ),
        // 5. Clé inconnue — refusée, jamais ignorée.
        (
            json!({ "country": "SN", "interets": ["eau"], "fonction": "Directrice" }),
            "fonction",
        ),
        // 6. Choix multiple obligatoire vide — que la base ne voit pas.
        (json!({ "country": "SN", "interets": [] }), "interets"),
    ];

    for (reponses, champ) in cas {
        let erreur = seances::sinscrire(
            &bac,
            seance,
            Some(personne),
            registration::SessionRegisterPayload {
                answers: reponses.clone(),
                ..seances::reponses_valides()
            },
        )
        .await
        .unwrap_err();

        assert_eq!(
            erreur.code,
            ErrorCode::RegistrationAnswerInvalid,
            "{reponses}"
        );
        assert_eq!(erreur.field.as_deref(), Some(champ), "{reponses}");
    }
}

/// **Le pays est le code ISO à deux lettres**, et rien d'autre (écart n° 11,
/// tranché en R18).
#[tokio::test]
async fn le_pays_est_un_code_iso_valide_contre_le_referentiel() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let seance = seance_ouverte(&bac, &terrain, "atelier").await;
    let personne = commun::personne(&bac, "inscrite@example.org", "Ida", "Ndour").await;

    for valeur in ["Sénégal", "ZZ"] {
        let erreur = seances::sinscrire(
            &bac,
            seance,
            Some(personne),
            registration::SessionRegisterPayload {
                answers: json!({ "country": valeur }),
                ..seances::reponses_valides()
            },
        )
        .await
        .unwrap_err();
        assert_eq!(erreur.field.as_deref(), Some("country"), "« {valeur} »");
    }

    seances::sinscrire(
        &bac,
        seance,
        Some(personne),
        registration::SessionRegisterPayload {
            answers: json!({ "country": "SN" }),
            ..seances::reponses_valides()
        },
    )
    .await
    .expect("le code ISO du Sénégal est accepté");
}
