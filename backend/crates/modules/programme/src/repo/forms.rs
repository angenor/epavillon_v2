//! Le formulaire d'inscription **applicable** à une séance, et ses options.
//!
//! # Trois niveaux, et le plus courant n'est pas celui qu'on croit
//!
//! Une séance peut porter un formulaire **attaché** ; à défaut, c'est celui de
//! son édition ; à défaut encore, celui de la plateforme. Le cas courant est le
//! troisième — le modèle sème un formulaire par défaut, et aucune séance ne
//! porte de formulaire attaché.
//!
//! **C'est ce qui rend l'écart n° 114 dangereux** : le contrôle des réponses
//! obligatoires de `tg_validate_registration()` est gardé par
//! `IF v_session.registration_form_id IS NOT NULL`. Sur une séance sans
//! formulaire attaché — donc presque toutes — la base ne vérifie **rien**, alors
//! que l'écran aura posé quatre questions. Sans la résolution qui suit, une
//! inscription sans aucune réponse obligatoire passerait.
//!
//! # Les options se lisent EN UNE FOIS
//!
//! Un formulaire de six questions à choix produirait sinon six requêtes. Les
//! taxonomies visées sont rassemblées, lues d'un coup, puis distribuées (R15).

use kernel::error::Result;
use serde::Serialize;
use sqlx::PgExecutor;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::answers::ChampResolu;
use crate::domain::ids::SessionId;

/// Le formulaire applicable et ses champs actifs — `{ form, fields }`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FormulaireApplicable {
    pub form: serde_json::Value,
    /// **Les champs actifs seulement**, triés comme l'écran les pose. Un champ
    /// désactivé n'est ni affiché ni accepté : c'est ainsi qu'une question
    /// retirée cesse de compter, sans effacer les réponses déjà données.
    pub fields: Vec<serde_json::Value>,
}

/// Un champ, tel que la base le porte — la forme que le service transforme en
/// [`ChampResolu`].
#[derive(Debug, Clone)]
pub struct Champ {
    pub code: String,
    pub field_type: String,
    pub is_required: bool,
    pub is_sensitive: bool,
    pub options: serde_json::Value,
    pub validation: serde_json::Value,
}

/// Le formulaire applicable à une séance. `None` : ni la séance, ni son édition,
/// ni la plateforme n'en portent — la plateforme en sème un, donc ce cas signale
/// une base incomplète plutôt qu'un usage.
pub async fn formulaire_applicable<'e>(
    executor: impl PgExecutor<'e>,
    session_id: SessionId,
) -> Result<Option<(Uuid, bool, serde_json::Value)>> {
    let ligne = sqlx::query!(
        r#"SELECT f.id, f.allows_anonymous, to_jsonb(f) AS "form!"
             FROM programme.sessions s
             JOIN programme.registration_forms f
               ON f.id = s.registration_form_id
              OR (s.registration_form_id IS NULL
                  AND f.is_default
                  AND (f.event_id = s.event_id OR f.event_id IS NULL))
            WHERE s.id = $1
            -- Attaché d'abord, édition ensuite, plateforme en dernier.
            ORDER BY (f.id = s.registration_form_id) DESC,
                     (f.event_id IS NOT NULL) DESC
            LIMIT 1"#,
        session_id.as_uuid()
    )
    .fetch_optional(executor)
    .await?;

    Ok(ligne.map(|l| (l.id, l.allows_anonymous, l.form)))
}

/// Les champs **actifs** d'un formulaire, dans l'ordre d'affichage.
pub async fn champs_actifs<'e>(executor: impl PgExecutor<'e>, form_id: Uuid) -> Result<Vec<Champ>> {
    let lignes = sqlx::query!(
        r#"SELECT code, field_type::text AS "field_type!", is_required,
                  is_sensitive, options, validation
             FROM programme.registration_form_fields
            WHERE form_id = $1 AND is_active
            ORDER BY sort_order, code"#,
        form_id
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Champ {
            code: l.code,
            field_type: l.field_type,
            is_required: l.is_required,
            is_sensitive: l.is_sensitive,
            options: l.options,
            validation: l.validation,
        })
        .collect())
}

/// Les champs actifs **prêts à afficher** : options de taxonomie résolues avec
/// leur libellé traduit, pour que l'écran n'ait pas à recharger la taxonomie.
///
/// C'est le défaut n° 1 de la v1 transposé au formulaire : n'exposer que les
/// codes forcerait chaque écran à refaire la correspondance, et les libellés
/// finiraient figés dans le frontend.
pub async fn champs_affichables<'e>(
    executor: impl PgExecutor<'e> + Copy,
    form_id: Uuid,
) -> Result<Vec<serde_json::Value>> {
    let lignes = sqlx::query!(
        r#"SELECT to_jsonb(f) AS "champ!", f.options ->> 'taxonomy' AS "taxonomie?"
             FROM programme.registration_form_fields f
            WHERE f.form_id = $1 AND f.is_active
            ORDER BY f.sort_order, f.code"#,
        form_id
    )
    .fetch_all(executor)
    .await?;

    let taxonomies: Vec<String> = lignes.iter().filter_map(|l| l.taxonomie.clone()).collect();
    let resolues = termes_des_taxonomies(executor, &taxonomies).await?;

    Ok(lignes
        .into_iter()
        .map(|l| {
            let mut champ = l.champ;
            if let Some(taxonomie) = l.taxonomie {
                let valeurs = resolues.get(&taxonomie).cloned().unwrap_or_default();
                // La clé `taxonomy` reste : l'écran doit pouvoir dire d'où
                // viennent ces options, et un administrateur les modifie en base.
                if let Some(options) = champ.get_mut("options").and_then(|o| o.as_object_mut()) {
                    options.insert("values".to_owned(), serde_json::Value::Array(valeurs));
                }
            }
            champ
        })
        .collect())
}

/// Les termes actifs de plusieurs taxonomies, **en une lecture**, prêts à
/// afficher : `{ value, label }`.
async fn termes_des_taxonomies<'e>(
    executor: impl PgExecutor<'e>,
    taxonomies: &[String],
) -> Result<HashMap<String, Vec<serde_json::Value>>> {
    if taxonomies.is_empty() {
        return Ok(HashMap::new());
    }

    let lignes = sqlx::query!(
        r#"SELECT taxonomy_code, code, label
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = ANY($1) AND is_active
            ORDER BY taxonomy_code, sort_order, code"#,
        taxonomies
    )
    .fetch_all(executor)
    .await?;

    let mut par_taxonomie: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for l in lignes {
        par_taxonomie
            .entry(l.taxonomy_code)
            .or_default()
            .push(serde_json::json!({ "value": l.code, "label": l.label }));
    }

    Ok(par_taxonomie)
}

/// Transformer les champs de la base en champs **résolus**, prêts pour la
/// validation pure.
///
/// Les options viennent de trois origines et repartent sous **une seule** forme :
/// une liste de valeurs admises. Un seul mécanisme de comparaison, donc une seule
/// occasion de diverger.
pub async fn resoudre_pour_validation<'e>(
    executor: impl PgExecutor<'e> + Copy,
    champs: Vec<Champ>,
) -> Result<Vec<ChampResolu>> {
    let taxonomies: Vec<String> = champs
        .iter()
        .filter_map(|c| {
            c.options
                .get("taxonomy")
                .and_then(|t| t.as_str())
                .map(str::to_owned)
        })
        .collect();

    let codes_de_taxonomie = super::cross::codes_de_taxonomies(executor, &taxonomies).await?;
    let mut par_taxonomie: HashMap<String, Vec<String>> = HashMap::new();
    for (taxonomie, code) in codes_de_taxonomie {
        par_taxonomie.entry(taxonomie).or_default().push(code);
    }

    // Les pays ne se lisent QUE si un champ en demande : la table en porte deux
    // cents, et un formulaire sans question de pays n'a pas à la charger.
    let pays = if champs.iter().any(|c| c.field_type == "country") {
        Some(super::cross::codes_pays(executor).await?)
    } else {
        None
    };

    Ok(champs
        .into_iter()
        .map(|c| {
            let options = match c.field_type.as_str() {
                "country" => pays.clone(),
                _ => match c.options.get("taxonomy").and_then(|t| t.as_str()) {
                    Some(taxonomie) => {
                        Some(par_taxonomie.get(taxonomie).cloned().unwrap_or_default())
                    }
                    None => valeurs_explicites(&c.options),
                },
            };

            ChampResolu {
                code: c.code,
                field_type: c.field_type,
                is_required: c.is_required,
                is_sensitive: c.is_sensitive,
                options,
                validation: c.validation,
            }
        })
        .collect())
}

/// Une liste explicite `[{"value":"x","label":{…}}]`. Absente, le champ n'est
/// pas à choix et rien n'est comparé.
fn valeurs_explicites(options: &serde_json::Value) -> Option<Vec<String>> {
    let valeurs = options.get("values")?.as_array()?;

    Some(
        valeurs
            .iter()
            .filter_map(|v| v.get("value").and_then(|v| v.as_str()).map(str::to_owned))
            .collect(),
    )
}
