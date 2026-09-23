//! Les documents : la bibliothèque publique, la chaîne de remplacement, la
//! recherche dans le texte, et les écritures du back-office. Les invariants —
//! une source au plus, un seul successeur, pas de boucle — sont tenus par la
//! base ; ce fichier ne les réimplémente pas.

use std::collections::HashMap;

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::domain::documents::{VocabularyCop, VocabularyTerm};

/// Un document publié, tel que la bibliothèque le lit.
#[derive(Debug, Clone)]
pub struct Publie {
    pub id: Uuid,
    pub slug: String,
    pub version: String,
    pub title: String,
    pub summary: Option<String>,
    pub type_code: String,
    pub themes: Vec<String>,
    pub event_id: Option<Uuid>,
    pub issued_on: Option<Date>,
    pub published_at: OffsetDateTime,
    pub publisher: Option<String>,
    pub locale: String,
    pub asset_id: Option<Uuid>,
    pub external_url: Option<String>,
    pub restricted: bool,
    pub rendu: Option<RenduLu>,
}

/// L'extraction du fichier du document, quand elle le concerne encore.
#[derive(Debug, Clone)]
pub struct RenduLu {
    pub asset_id: Uuid,
    pub status: String,
    pub page_count: Option<i32>,
    pub reading_bytes: Option<i64>,
    pub serve_as_is: bool,
    pub extracted_at: Option<OffsetDateTime>,
}

impl RenduLu {
    /// Lisible : l'extraction de **ce** fichier est prête.
    pub fn pret_pour(&self, asset_id: Option<Uuid>) -> bool {
        self.status == "ready" && Some(self.asset_id) == asset_id
    }
}

pub async fn publies(conn: &mut PgConnection, locale: &str) -> Result<Vec<Publie>> {
    let lignes = sqlx::query!(
        r#"SELECT d.id, d.slug::text AS "slug!", d.version,
                  platform.t(d.title, $1) AS "title!", platform.t(d.summary, $1) AS summary,
                  t.code AS "type_code!",
                  COALESCE((SELECT array_agg(tt.code ORDER BY et.sort_order, tt.code)
                              FROM reference.entity_terms et
                              JOIN reference.taxonomy_terms tt ON tt.id = et.term_id
                             WHERE et.entity_schema = 'negotiation' AND et.entity_table = 'documents'
                               AND et.entity_id = d.id AND tt.taxonomy_code = 'negotiation_theme'
                               AND tt.is_active), '{}') AS "themes!",
                  d.event_id, d.issued_on, d.published_at AS "published_at!", d.publisher,
                  d.locale_code, d.asset_id, d.external_url::text AS external_url, d.is_restricted,
                  r.asset_id AS "rendu_asset?", r.status::text AS "rendu_status?", r.page_count,
                  r.reading_bytes, r.serve_as_is AS "serve_as_is?", r.extracted_at
             FROM negotiation.documents d
             JOIN reference.taxonomy_terms t ON t.id = d.document_type_term_id
             LEFT JOIN negotiation.document_renditions r ON r.document_id = d.id
            WHERE d.published_at IS NOT NULL
            ORDER BY d.published_at DESC, d.id"#,
        locale
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| Publie {
            id: l.id,
            slug: l.slug,
            version: l.version,
            title: l.title,
            summary: l.summary,
            type_code: l.type_code,
            themes: l.themes,
            event_id: l.event_id,
            issued_on: l.issued_on,
            published_at: l.published_at,
            publisher: l.publisher,
            locale: l.locale_code,
            asset_id: l.asset_id,
            external_url: l.external_url,
            restricted: l.is_restricted,
            rendu: l.rendu_asset.map(|asset_id| RenduLu {
                asset_id,
                status: l.rendu_status.unwrap_or_default(),
                page_count: l.page_count,
                reading_bytes: l.reading_bytes,
                serve_as_is: l.serve_as_is.unwrap_or(false),
                extracted_at: l.extracted_at,
            }),
        })
        .collect())
}

#[derive(Debug, Clone)]
pub struct SuccesseurLu {
    pub id: Uuid,
    pub title: String,
    pub published_at: OffsetDateTime,
    pub page_count: Option<i32>,
}

/// Pour chaque document remplacé, le bout **publié** de sa chaîne. Un brouillon
/// ne remplace encore rien ; un maillon dépublié interrompt la chaîne.
pub async fn bouts_de_chaine(
    conn: &mut PgConnection,
    locale: &str,
) -> Result<HashMap<Uuid, SuccesseurLu>> {
    let lignes = sqlx::query!(
        r#"WITH RECURSIVE chaine(origine, id, rang) AS (
               SELECT d.id, s.id, 1
                 FROM negotiation.documents d
                 JOIN negotiation.documents s ON s.supersedes_id = d.id AND s.published_at IS NOT NULL
               UNION ALL
               SELECT c.origine, s.id, c.rang + 1
                 FROM chaine c
                 JOIN negotiation.documents s ON s.supersedes_id = c.id AND s.published_at IS NOT NULL
                WHERE c.rang < 50
           )
           SELECT DISTINCT ON (c.origine)
                  c.origine AS "origine!", s.id AS "id!", platform.t(s.title, $1) AS "title!",
                  s.published_at AS "published_at!", r.page_count
             FROM chaine c
             JOIN negotiation.documents s ON s.id = c.id
             LEFT JOIN negotiation.document_renditions r ON r.document_id = s.id AND r.asset_id = s.asset_id
            ORDER BY c.origine, c.rang DESC"#,
        locale
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| {
            (
                l.origine,
                SuccesseurLu {
                    id: l.id,
                    title: l.title,
                    published_at: l.published_at,
                    page_count: l.page_count,
                },
            )
        })
        .collect())
}

pub async fn vocabulaire(
    conn: &mut PgConnection,
    taxonomie: &str,
    locale: &str,
) -> Result<Vec<VocabularyTerm>> {
    let lignes = sqlx::query!(
        r#"SELECT code AS "code!", platform.t(label, $2) AS "label!", sort_order AS "sort_order!"
             FROM reference.taxonomy_terms
            WHERE taxonomy_code = $1 AND is_active
            ORDER BY sort_order, code"#,
        taxonomie,
        locale
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| VocabularyTerm {
            code: l.code,
            label: l.label,
            sort_order: i32::from(l.sort_order),
        })
        .collect())
}

pub async fn cops(
    conn: &mut PgConnection,
    ids: &[Uuid],
    locale: &str,
) -> Result<Vec<VocabularyCop>> {
    let lignes = sqlx::query!(
        r#"SELECT e.id, COALESCE(e.edition_label, e.acronym, platform.t(e.title, $2)) AS "label!", e.city
             FROM event.events e
            WHERE e.id = ANY($1)
            ORDER BY e.starts_at DESC"#,
        ids,
        locale
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| VocabularyCop {
            id: l.id,
            label: l.label,
            city: l.city,
        })
        .collect())
}

#[derive(Debug, Clone)]
pub struct Trouvaille {
    pub document_id: Uuid,
    pub restricted: bool,
    pub page_index: i32,
    pub label: String,
    pub excerpt: String,
}

/// Les pages des documents publiés dont le texte répond à la requête, les plus
/// pertinentes d'abord. Accents et casse ne comptent pas.
pub async fn rechercher(conn: &mut PgConnection, requete: &str) -> Result<Vec<Trouvaille>> {
    let lignes = sqlx::query!(
        r#"SELECT p.document_id, d.is_restricted, p.page_index, p.label,
                  ts_headline('french', p.plain_text, q,
                              'StartSel="",StopSel="",MaxWords=24,MinWords=10,MaxFragments=1') AS "excerpt!"
             FROM websearch_to_tsquery('french', platform.immutable_unaccent($1)) q,
                  negotiation.document_pages p
             JOIN negotiation.documents d ON d.id = p.document_id AND d.published_at IS NOT NULL
            WHERE p.search_vector @@ q
            ORDER BY ts_rank(p.search_vector, q) DESC, p.document_id, p.page_index
            LIMIT 300"#,
        requete
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| Trouvaille {
            document_id: l.document_id,
            restricted: l.is_restricted,
            page_index: l.page_index,
            label: l.label,
            excerpt: l.excerpt,
        })
        .collect())
}

/// Un document publié, pour la lecture, les images et le compteur.
#[derive(Debug, Clone)]
pub struct Lisible {
    pub id: Uuid,
    pub version: String,
    pub restricted: bool,
    pub asset_id: Option<Uuid>,
    pub external_url: Option<String>,
    pub rendu: Option<RenduLu>,
    pub outline: Option<Value>,
}

pub async fn publie(conn: &mut PgConnection, id: Uuid) -> Result<Option<Lisible>> {
    lisible(conn, id, true).await
}

/// Le même, brouillon compris : l'aperçu du back-office.
pub async fn quelconque(conn: &mut PgConnection, id: Uuid) -> Result<Option<Lisible>> {
    lisible(conn, id, false).await
}

async fn lisible(
    conn: &mut PgConnection,
    id: Uuid,
    publie_seulement: bool,
) -> Result<Option<Lisible>> {
    let ligne = sqlx::query!(
        r#"SELECT d.id, d.version, d.is_restricted, d.asset_id, d.external_url::text AS external_url,
                  r.asset_id AS "rendu_asset?", r.status::text AS "rendu_status?", r.page_count,
                  r.reading_bytes, r.serve_as_is AS "serve_as_is?", r.extracted_at, r.outline
             FROM negotiation.documents d
             LEFT JOIN negotiation.document_renditions r ON r.document_id = d.id
            WHERE d.id = $1 AND (d.published_at IS NOT NULL OR NOT $2)"#,
        id,
        publie_seulement
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| Lisible {
        id: l.id,
        version: l.version,
        restricted: l.is_restricted,
        asset_id: l.asset_id,
        external_url: l.external_url,
        outline: l.outline,
        rendu: l.rendu_asset.map(|asset_id| RenduLu {
            asset_id,
            status: l.rendu_status.unwrap_or_default(),
            page_count: l.page_count,
            reading_bytes: l.reading_bytes,
            serve_as_is: l.serve_as_is.unwrap_or(false),
            extracted_at: l.extracted_at,
        }),
    }))
}

pub async fn compter_un_telechargement(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!(
        "SELECT negotiation.register_document_download($1) AS fait",
        id
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

// -----------------------------------------------------------------------------
// Le back-office
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Lien {
    pub id: Uuid,
    pub title: String,
    pub version: String,
}

/// Un document, quel que soit son état, avec ses textes non résolus.
#[derive(Debug, Clone)]
pub struct Fiche {
    pub id: Uuid,
    pub slug: String,
    pub title: Value,
    pub title_resolu: String,
    pub summary: Option<Value>,
    pub type_code: String,
    pub event_id: Option<Uuid>,
    pub version: String,
    pub issued_on: Option<Date>,
    pub publisher: Option<String>,
    pub locale: String,
    pub asset_id: Option<Uuid>,
    pub external_url: Option<String>,
    pub supersedes: Option<Lien>,
    pub superseded_by: Option<Lien>,
    pub restricted: bool,
    pub rag_eligible: bool,
    pub published_at: Option<OffsetDateTime>,
    pub unpublished_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

pub async fn fiches(conn: &mut PgConnection, id: Option<Uuid>, locale: &str) -> Result<Vec<Fiche>> {
    let lignes = sqlx::query!(
        r#"SELECT d.id, d.slug::text AS "slug!", d.title::jsonb AS "title!", platform.t(d.title, $2) AS "title_resolu!",
                  d.summary::jsonb AS summary, t.code AS "type_code!", d.event_id, d.version, d.issued_on,
                  d.publisher, d.locale_code, d.asset_id, d.external_url::text AS external_url,
                  d.is_restricted, d.is_rag_eligible, d.published_at, d.unpublished_at,
                  d.created_at, d.updated_at,
                  a.id AS "remplace_id?", platform.t(a.title, $2) AS remplace_titre, a.version AS "remplace_version?",
                  s.id AS "successeur_id?", platform.t(s.title, $2) AS successeur_titre,
                  s.version AS "successeur_version?"
             FROM negotiation.documents d
             JOIN reference.taxonomy_terms t ON t.id = d.document_type_term_id
             LEFT JOIN negotiation.documents a ON a.id = d.supersedes_id
             LEFT JOIN negotiation.documents s ON s.supersedes_id = d.id
            WHERE $1::uuid IS NULL OR d.id = $1
            ORDER BY d.updated_at DESC, d.id"#,
        id,
        locale
    )
    .fetch_all(conn)
    .await?;
    Ok(lignes
        .into_iter()
        .map(|l| Fiche {
            id: l.id,
            slug: l.slug,
            title: l.title,
            title_resolu: l.title_resolu,
            summary: l.summary,
            type_code: l.type_code,
            event_id: l.event_id,
            version: l.version,
            issued_on: l.issued_on,
            publisher: l.publisher,
            locale: l.locale_code,
            asset_id: l.asset_id,
            external_url: l.external_url,
            supersedes: l.remplace_id.map(|id| Lien {
                id,
                title: l.remplace_titre.unwrap_or_default(),
                version: l.remplace_version.unwrap_or_default(),
            }),
            superseded_by: l.successeur_id.map(|id| Lien {
                id,
                title: l.successeur_titre.unwrap_or_default(),
                version: l.successeur_version.unwrap_or_default(),
            }),
            restricted: l.is_restricted,
            rag_eligible: l.is_rag_eligible,
            published_at: l.published_at,
            unpublished_at: l.unpublished_at,
            created_at: l.created_at,
            updated_at: l.updated_at,
        })
        .collect())
}

/// L'état d'un document, verrouillé pour la transaction : publier, attacher un
/// fichier ou supprimer se décident sur lui.
#[derive(Debug, Clone)]
pub struct EtatVerrouille {
    pub published_at: Option<OffsetDateTime>,
    pub unpublished_at: Option<OffsetDateTime>,
    pub asset_id: Option<Uuid>,
    pub external_url: Option<String>,
}

impl EtatVerrouille {
    /// Déjà publié une fois : le fichier est figé, le document ne se supprime plus.
    pub fn deja_publie(&self) -> bool {
        self.published_at.is_some() || self.unpublished_at.is_some()
    }
}

pub async fn verrouiller(conn: &mut PgConnection, id: Uuid) -> Result<Option<EtatVerrouille>> {
    let ligne = sqlx::query!(
        "SELECT published_at, unpublished_at, asset_id, external_url::text AS external_url
           FROM negotiation.documents WHERE id = $1 FOR UPDATE",
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(ligne.map(|l| EtatVerrouille {
        published_at: l.published_at,
        unpublished_at: l.unpublished_at,
        asset_id: l.asset_id,
        external_url: l.external_url,
    }))
}

pub async fn type_id(conn: &mut PgConnection, code: &str) -> Result<Option<Uuid>> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM reference.taxonomy_terms
          WHERE taxonomy_code = 'document_type' AND code = $1 AND is_active",
        code
    )
    .fetch_optional(conn)
    .await?;
    Ok(id)
}

pub async fn edition_existe(conn: &mut PgConnection, id: Uuid) -> Result<bool> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM event.events WHERE id = $1) AS "existe!""#,
        id
    )
    .fetch_one(conn)
    .await?;
    Ok(existe)
}

pub struct Nouveau<'a> {
    pub title: &'a Value,
    pub summary: Option<&'a Value>,
    pub type_id: Uuid,
    pub event_id: Option<Uuid>,
    pub version: &'a str,
    pub issued_on: Option<Date>,
    pub publisher: Option<&'a str>,
    pub locale: &'a str,
    pub supersedes_id: Option<Uuid>,
    pub restricted: bool,
    pub rag_eligible: bool,
    pub external_url: Option<&'a str>,
    pub auteur: Uuid,
}

/// Un brouillon. Son identifiant d'URL suit le titre, suffixé pour rester
/// unique quand deux documents portent le même titre.
pub async fn creer(conn: &mut PgConnection, n: &Nouveau<'_>) -> Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO negotiation.documents
             (id, slug, title, summary, document_type_term_id, event_id, version, issued_on,
              publisher, locale_code, supersedes_id, is_restricted, is_rag_eligible, external_url,
              uploaded_by)
         VALUES ($1,
                 (platform.slugify(platform.t($2::jsonb::platform.i18n_text, 'fr')) || '-'
                  || substr(replace($15::text, '-', ''), 25, 8))::platform.slug,
                 $2::jsonb::platform.i18n_text, $3::jsonb::platform.i18n_text, $4, $5, $6, $7,
                 $8, $9, $10, $11, $12, $13::text::platform.url, $14)",
        id,
        n.title,
        n.summary,
        n.type_id,
        n.event_id,
        n.version,
        n.issued_on,
        n.publisher,
        n.locale,
        n.supersedes_id,
        n.restricted,
        n.rag_eligible,
        n.external_url,
        n.auteur,
        id.to_string()
    )
    .execute(conn)
    .await?;
    Ok(id)
}

/// Un changement partiel. `Some(x)` : le champ prend `x` ; `None` : inchangé.
/// Pour un champ facultatif, `Some(None)` le vide.
#[derive(Default)]
pub struct Modification<'a> {
    pub title: Option<&'a Value>,
    pub summary: Option<Option<&'a Value>>,
    pub type_id: Option<Uuid>,
    pub event_id: Option<Option<Uuid>>,
    pub version: Option<&'a str>,
    pub issued_on: Option<Option<Date>>,
    pub publisher: Option<Option<&'a str>>,
    pub locale: Option<&'a str>,
    pub supersedes_id: Option<Option<Uuid>>,
    pub restricted: Option<bool>,
    pub rag_eligible: Option<bool>,
    pub external_url: Option<Option<&'a str>>,
}

pub async fn modifier(conn: &mut PgConnection, id: Uuid, m: &Modification<'_>) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.documents SET
             title = COALESCE($2::jsonb::platform.i18n_text, title),
             summary = CASE WHEN $3 THEN $4::jsonb::platform.i18n_text ELSE summary END,
             document_type_term_id = COALESCE($5, document_type_term_id),
             event_id = CASE WHEN $6 THEN $7 ELSE event_id END,
             version = COALESCE($8, version),
             issued_on = CASE WHEN $9 THEN $10 ELSE issued_on END,
             publisher = CASE WHEN $11 THEN $12 ELSE publisher END,
             locale_code = COALESCE($13, locale_code),
             supersedes_id = CASE WHEN $14 THEN $15 ELSE supersedes_id END,
             is_restricted = COALESCE($16, is_restricted),
             is_rag_eligible = COALESCE($17, is_rag_eligible),
             external_url = CASE WHEN $18 THEN $19::text::platform.url ELSE external_url END
          WHERE id = $1",
        id,
        m.title,
        m.summary.is_some(),
        m.summary.flatten(),
        m.type_id,
        m.event_id.is_some(),
        m.event_id.flatten(),
        m.version,
        m.issued_on.is_some(),
        m.issued_on.flatten(),
        m.publisher.is_some(),
        m.publisher.flatten(),
        m.locale,
        m.supersedes_id.is_some(),
        m.supersedes_id.flatten(),
        m.restricted,
        m.rag_eligible,
        m.external_url.is_some(),
        m.external_url.flatten()
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn attacher_le_fichier(conn: &mut PgConnection, id: Uuid, asset_id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.documents SET asset_id = $2 WHERE id = $1",
        id,
        asset_id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn publier(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.documents SET published_at = now(), unpublished_at = NULL WHERE id = $1",
        id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn depublier(conn: &mut PgConnection, id: Uuid) -> Result<()> {
    sqlx::query!(
        "UPDATE negotiation.documents SET published_at = NULL, unpublished_at = now()
          WHERE id = $1 AND published_at IS NOT NULL",
        id
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Supprime un brouillon jamais publié. Rend faux sinon.
pub async fn supprimer_le_brouillon(conn: &mut PgConnection, id: Uuid) -> Result<bool> {
    let fait = sqlx::query!(
        "DELETE FROM negotiation.documents
          WHERE id = $1 AND published_at IS NULL AND unpublished_at IS NULL",
        id
    )
    .execute(conn)
    .await?;
    Ok(fait.rows_affected() == 1)
}

/// Le brouillon d'une nouvelle version, prérempli et désigné comme remplaçant.
/// La version se saisit : elle porte d'ici là une mention à remplacer.
pub async fn nouvelle_version(
    conn: &mut PgConnection,
    id: Uuid,
    version: &str,
    auteur: Uuid,
) -> Result<Uuid> {
    let nouveau = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO negotiation.documents
             (id, space_id, slug, title, summary, document_type_term_id, track_term_id, event_id,
              version, publisher, locale_code, supersedes_id, is_restricted, is_rag_eligible, uploaded_by)
         SELECT $2, d.space_id, d.slug, d.title, d.summary, d.document_type_term_id, d.track_term_id,
                d.event_id, $3, d.publisher, d.locale_code, d.id, d.is_restricted, d.is_rag_eligible, $4
           FROM negotiation.documents d WHERE d.id = $1",
        id,
        nouveau,
        version,
        auteur
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, role, sort_order)
         SELECT entity_schema, entity_table, $2, term_id, role, sort_order
           FROM reference.entity_terms
          WHERE entity_schema = 'negotiation' AND entity_table = 'documents' AND entity_id = $1",
        id,
        nouveau
    )
    .execute(conn)
    .await?;
    Ok(nouveau)
}

/// Le successeur direct d'un document, quel que soit son état : c'est lui que
/// nomme le refus d'un second remplaçant.
/// La version du remplaçant direct : une nouvelle version garde le titre, qui ne le distingue pas.
pub async fn version_du_successeur(conn: &mut PgConnection, id: Uuid) -> Result<Option<String>> {
    let version = sqlx::query_scalar!(
        "SELECT version FROM negotiation.documents WHERE supersedes_id = $1",
        id
    )
    .fetch_optional(conn)
    .await?;
    Ok(version)
}
