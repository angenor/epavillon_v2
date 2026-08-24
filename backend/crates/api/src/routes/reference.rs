//! Le référentiel : pays, langues, vocabulaires administrables.
//!
//! **Il n'appartient à aucun module.** Le schéma `reference` est transverse — le
//! dépôt d'un dossier y lit ses thématiques, le rattachement à une organisation
//! son type, l'inscription son pays —, et le loger dans un crate de module
//! obligerait les cinq autres à en dépendre. Ces routes vivent donc auprès des
//! routes d'exploitation, comme la permission d'`analytics` de `health.rs`.
//!
//! **Aucune session n'est exigée.** Un formulaire d'inscription a besoin de la
//! liste des pays avant qu'un compte existe, et une page publique d'événement
//! affiche ses pastilles thématiques à qui passe. Rien ici n'est personnel.
//!
//! **Les textes multilingues partent ENTIERS.** La colonne `platform.i18n_text`
//! est rendue telle quelle et le site la résout dans la locale active
//! (`useI18nText`). Résoudre côté API ferait perdre l'autre langue, et le site
//! change de locale sans recharger ses données.

use actix_web::{web, HttpResponse};
use kernel::db::Db;
use kernel::error::Result;
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn configurer(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/reference")
            .route("/countries", web::get().to(pays))
            .route("/locales", web::get().to(langues))
            .route("/taxonomies/{code}/terms", web::get().to(termes)),
    );
}

#[derive(Debug, Serialize)]
pub struct Country {
    pub id: Uuid,
    pub iso2: String,
    pub iso3: String,
    pub numeric_code: Option<String>,
    pub name: Value,
    pub official_name: Option<Value>,
    pub name_normalized: Option<String>,
    pub region_code: Option<String>,
    pub continent: Option<String>,
    pub oif_status: String,
    pub default_timezone: Option<String>,
    pub calling_code: Option<String>,
    pub flag_emoji: Option<String>,
    pub is_active: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
pub struct Locale {
    pub code: String,
    pub native_label: String,
    pub english_label: String,
    pub is_default: bool,
    pub is_active: bool,
    pub sort_order: i16,
    pub text_search_config: String,
}

#[derive(Debug, Serialize)]
pub struct TaxonomyTerm {
    pub id: Uuid,
    pub taxonomy_code: String,
    pub parent_id: Option<Uuid>,
    pub code: String,
    pub label: Value,
    pub description: Option<Value>,
    pub color_hex: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i16,
    pub is_active: bool,
    pub superseded_by: Option<Uuid>,
    pub metadata: Value,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// **Les pays inactifs sont écartés** : ils ne sont pas supprimés — des fiches
/// anciennes les référencent — mais on ne les propose plus au choix.
#[utoipa::path(
    get,
    description = "`Country[]` — les pays actifs, ordonnés par leur nom français. Sans session : un formulaire d'inscription en a besoin avant qu'un compte existe. Les libellés partent en `i18n_text` entier, le site les résout dans sa locale.",
    path = "/reference/countries",
    tag = "Référentiel",
    operation_id = "reference_pays",
    responses((status = 200, description = "Country[]", body = Object))
)]
pub(crate) async fn pays(db: web::Data<Db>) -> Result<HttpResponse> {
    let lignes = sqlx::query_as!(
        Country,
        r#"SELECT id,
                  iso2 AS "iso2!", iso3 AS "iso3!", numeric_code,
                  name AS "name!: Value", official_name AS "official_name?: Value",
                  name_normalized, region_code, continent,
                  oif_status::text AS "oif_status!",
                  default_timezone::text AS "default_timezone?",
                  calling_code, flag_emoji, is_active, created_at, updated_at
             FROM reference.countries
            WHERE is_active
            ORDER BY name ->> 'fr'"#
    )
    .fetch_all(db.pool())
    .await?;

    Ok(HttpResponse::Ok().json(lignes))
}

#[utoipa::path(
    get,
    description = "`Locale[]` — les langues actives, dans leur ordre d'affichage.",
    path = "/reference/locales",
    tag = "Référentiel",
    operation_id = "reference_langues",
    responses((status = 200, description = "Locale[]", body = Object))
)]
pub(crate) async fn langues(db: web::Data<Db>) -> Result<HttpResponse> {
    let lignes = sqlx::query_as!(
        Locale,
        r#"SELECT code AS "code!", native_label, english_label, is_default, is_active,
                  sort_order, text_search_config::text AS "text_search_config!"
             FROM reference.locales
            WHERE is_active
            ORDER BY sort_order, code"#
    )
    .fetch_all(db.pool())
    .await?;

    Ok(HttpResponse::Ok().json(lignes))
}

/// **Une taxonomie inconnue rend une liste vide, pas 404.** L'écran qui demande
/// « organization_type » attend une liste de choix : lui rendre une erreur le
/// ferait afficher une panne pour un vocabulaire qu'un administrateur n'a
/// simplement pas encore garni.
#[utoipa::path(
    get,
    description = "`TaxonomyTerm[]` — les termes ACTIFS d'une taxonomie, dans leur ordre d'affichage. Les libellés et les couleurs viennent de la base, **où un administrateur les modifie** : les figer dans le site est le défaut n° 1 de la v1.",
    path = "/reference/taxonomies/{code}/terms",
    tag = "Référentiel",
    operation_id = "reference_termes",
    params(("code" = String, Path, description = "Code de la taxonomie, ex. `activity_theme`")),
    responses((status = 200, description = "TaxonomyTerm[]", body = Object))
)]
pub(crate) async fn termes(db: web::Data<Db>, chemin: web::Path<String>) -> Result<HttpResponse> {
    let lignes = sqlx::query_as!(
        TaxonomyTerm,
        r#"SELECT id, taxonomy_code, parent_id, code,
                  label AS "label!: Value", description AS "description?: Value",
                  color_hex, icon, sort_order, is_active, superseded_by,
                  metadata AS "metadata!: Value",
                  created_at, updated_at
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = $1 AND is_active
            ORDER BY sort_order, code"#,
        chemin.into_inner()
    )
    .fetch_all(db.pool())
    .await?;

    Ok(HttpResponse::Ok().json(lignes))
}
