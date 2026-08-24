//! La fiche complète — **huit lectures dans une seule transaction de lecture**,
//! assemblées en Rust.
//!
//! Pourquoi pas une requête unique rendant du JSON : elle serait illisible, non
//! vérifiée colonne par colonne à la compilation, et impossible à faire évoluer
//! sans la relire en entier. La fiche n'est pas un chemin chaud — elle s'ouvre à
//! la main, une fois. C'est le parti déjà retenu en B1 pour la fiche d'un
//! utilisateur (research.md § R15).
//!
//! Deux lectures franchissent une frontière de schéma : les activités et le nom
//! des personnes. Même règle qu'en R14 — un module **lit** hors de son schéma
//! quand la question porte sur ses propres entités.

use kernel::error::Result;
use serde_json::Value;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

use crate::domain::admin::{
    MergedRef, OrganizationActivityRow, OrganizationDetail, OrganizationDomainRow,
    OrganizationHistoryEntry, OrganizationMemberRow, OrganizationMergeEntry, OrganizationNameRow,
    OrganizationRef,
};
use crate::domain::ids::{OrganizationDomainId, OrganizationId, OrganizationNameId, PersonId};

/// L'identité de la fiche, sceau et renvoi de fusion compris.
pub async fn identite(
    conn: &mut PgConnection,
    id: OrganizationId,
) -> Result<Option<OrganizationDetail>> {
    let ligne = sqlx::query!(
        r#"SELECT o.id, o.legal_name, o.acronym, o.slug::text AS "slug!",
                  o.status::text AS "statut!", o.organization_type_code,
                  t.label::jsonb AS "type_label?",
                  o.country_id, c.name::jsonb AS "country_name?", o.city,
                  o.description::jsonb AS "description?",
                  o.website::text AS "website?",
                  o.contact_email::text AS "contact_email?",
                  o.contact_phone,
                  o.verified_at,
                  v.display_name AS "verified_by_name?",
                  o.trust_score, o.created_at,
                  cr.display_name AS "created_by_name?",
                  o.merged_into_id, o.merged_at,
                  cible.legal_name AS "merged_into_name?"
             FROM org.organizations o
             LEFT JOIN reference.taxonomy_terms t
                    ON t.taxonomy_code = 'organization_type' AND t.code = o.organization_type_code
             LEFT JOIN reference.countries c ON c.id = o.country_id
             LEFT JOIN identity.people v  ON v.id = o.verified_by
             LEFT JOIN identity.people cr ON cr.id = o.created_by
             LEFT JOIN org.organizations cible ON cible.id = o.merged_into_id
            WHERE o.id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne.map(|l| OrganizationDetail {
        organization_id: OrganizationId(l.id),
        legal_name: l.legal_name,
        acronym: l.acronym,
        slug: l.slug,
        status: l.statut,
        organization_type_code: l.organization_type_code,
        organization_type_label: l.type_label,
        country_id: l.country_id,
        country_name: l.country_name,
        city: l.city,
        description: l.description,
        website: l.website,
        contact_email: l.contact_email,
        contact_phone: l.contact_phone,
        verified_at: l.verified_at,
        verified_by_name: l.verified_by_name,
        trust_score: l.trust_score,
        created_at: l.created_at,
        created_by_name: l.created_by_name,
        merged_into: l.merged_into_id.map(|cible| MergedRef {
            organization_id: OrganizationId(cible),
            legal_name: l.merged_into_name.unwrap_or_default(),
            merged_at: l.merged_at,
        }),
        absorbed: Vec::new(),
        scorecard: None,
        names: Vec::new(),
        domains: Vec::new(),
        members: Vec::new(),
        activities: Vec::new(),
        history: Vec::new(),
        merges: Vec::new(),
        duplicates: Vec::new(),
    }))
}

/// La fiche de performance — **jamais nulle pour une organisation qui existe**.
///
/// La projection est MATÉRIALISÉE : une organisation créée depuis le dernier
/// rafraîchissement n'y figure pas encore, et la fiche du back-office s'ouvrait
/// alors sur une valeur nulle qu'aucun écran n'attendait. Ses compteurs valent
/// zéro, ce qui est vrai — c'est déjà ce que fait la liste, par `COALESCE` sur
/// chaque colonne (`repo/admin_list.rs`).
///
/// `ratio_acceptation` reste NUL, et c'est délibéré : une organisation qui n'a
/// jamais rien déposé n'a pas un taux d'acceptation de zéro, elle n'en a pas.
/// Le modèle le dit dans le `COMMENT ON COLUMN` de la vue.
pub async fn scorecard(conn: &mut PgConnection, id: OrganizationId) -> Result<Option<Value>> {
    let ligne = sqlx::query_scalar!(
        r#"SELECT COALESCE(
                      to_jsonb(s),
                      jsonb_build_object(
                          'organization_id',           o.id,
                          'legal_name',                o.legal_name,
                          'acronym',                   o.acronym,
                          'slug',                      o.slug,
                          'statut',                    o.status::text,
                          'organization_type_code',    o.organization_type_code,
                          'country_id',                o.country_id,
                          'pays_iso3',                 c.iso3,
                          'pays_nom',                  platform.t(c.name),
                          'statut_oif',                COALESCE(c.oif_status::text, 'none'),
                          'est_verifiee',              (o.verified_at IS NOT NULL),
                          'verified_at',               o.verified_at,
                          'score_confiance',           o.trust_score,
                          'merged_into_id',            o.merged_into_id,
                          'membres_actifs',            0,
                          'membres_en_attente',        0,
                          'referents',                 0,
                          'propositions_deposees',     0,
                          'propositions_en_brouillon', 0,
                          'propositions_acceptees',    0,
                          'propositions_rejetees',     0,
                          'propositions_retirees',     0,
                          'evenements_couverts',       0,
                          'note_moyenne_obtenue',      NULL,
                          'ratio_acceptation',         NULL,
                          'sessions_programmees',      0,
                          'sessions_realisees',        0,
                          'sessions_annulees',         0,
                          'inscrits_a_ses_sessions',   0,
                          'presents_a_ses_sessions',   0,
                          'articles_publies',          0,
                          'articles_en_moderation',    0,
                          'octets_stockes',            0,
                          'derniere_activite',         o.updated_at,
                          'inscrite_le',               o.created_at
                      )
                  ) AS "fiche!"
             FROM org.organizations o
             LEFT JOIN reference.countries c ON c.id = o.country_id
             LEFT JOIN analytics.mv_organization_scorecard s ON s.organization_id = o.id
            WHERE o.id = $1"#,
        id.as_uuid()
    )
    .fetch_optional(conn)
    .await?;

    Ok(ligne)
}

/// Les dénominations. `is_derived` compare la dénomination normalisée au nom
/// légal et au sigle de la fiche : ce sont celles que
/// `tg_organizations_sync_names` pose, et l'API refuse de les retirer.
pub async fn denominations(
    conn: &mut PgConnection,
    id: OrganizationId,
) -> Result<Vec<OrganizationNameRow>> {
    let lignes = sqlx::query!(
        r#"SELECT n.id, n.name, n.kind::text AS "kind!", n.locale, n.is_confirmed,
                  p.display_name AS "created_by_name?", n.created_at,
                  -- **Le genre entre dans la comparaison.** Sans lui, une faute
                  -- d'orthographe connue — « Developpement » sans accent — porte
                  -- le même nom normalisé que le nom légal et se ferait passer
                  -- pour une dénomination posée par la base : l'API refuserait
                  -- alors de retirer une ligne que rien ne régénère.
                  ((n.kind = 'legal'   AND n.name_normalized = o.legal_name_normalized)
                OR (n.kind = 'acronym' AND n.name_normalized = o.acronym_normalized))
                      AS "is_derived!"
             FROM org.organization_names n
             JOIN org.organizations o ON o.id = n.organization_id
             LEFT JOIN identity.people p ON p.id = n.created_by
            WHERE n.organization_id = $1
            ORDER BY n.kind, n.name"#,
        id.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OrganizationNameRow {
            id: OrganizationNameId(l.id),
            name: l.name,
            kind: l.kind,
            locale: l.locale,
            is_confirmed: l.is_confirmed,
            created_by_name: l.created_by_name,
            created_at: l.created_at,
            is_derived: l.is_derived,
        })
        .collect())
}

/// Les domaines, **et les fiches qui les partagent**. C'est le signal de doublon
/// le plus fiable du modèle, et il se voit d'abord ici.
pub async fn domaines(
    conn: &mut PgConnection,
    id: OrganizationId,
) -> Result<Vec<OrganizationDomainRow>> {
    let lignes = sqlx::query!(
        r#"SELECT d.id, d.domain, d.verified_at, d.verification_method, d.auto_join,
                  d.created_at,
                  COALESCE(
                      (SELECT jsonb_agg(jsonb_build_object(
                                  'organization_id', autre.organization_id,
                                  'legal_name', o.legal_name))
                         FROM org.organization_domains autre
                         JOIN org.organizations o ON o.id = autre.organization_id
                        WHERE autre.domain = d.domain
                          AND autre.organization_id <> d.organization_id),
                      '[]'::jsonb)  AS "shared_with!"
             FROM org.organization_domains d
            WHERE d.organization_id = $1
            ORDER BY d.verified_at DESC NULLS LAST, d.domain"#,
        id.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OrganizationDomainRow {
            id: OrganizationDomainId(l.id),
            domain: l.domain,
            verified_at: l.verified_at,
            verification_method: l.verification_method,
            auto_join: l.auto_join,
            created_at: l.created_at,
            shared_with: partages(&l.shared_with),
        })
        .collect())
}

fn partages(valeur: &Value) -> Vec<OrganizationRef> {
    valeur
        .as_array()
        .map(|entrees| {
            entrees
                .iter()
                .filter_map(|e| {
                    Some(OrganizationRef {
                        organization_id: OrganizationId(
                            e.get("organization_id")?.as_str()?.parse().ok()?,
                        ),
                        legal_name: e.get("legal_name")?.as_str()?.to_owned(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Les membres, **révoqués compris** : une adhésion retirée fait partie de
/// l'histoire de la fiche.
pub async fn membres(
    conn: &mut PgConnection,
    id: OrganizationId,
) -> Result<Vec<OrganizationMemberRow>> {
    let lignes = sqlx::query!(
        r#"SELECT m.id, m.person_id, p.display_name AS "display_name!",
                  p.primary_email::text AS "primary_email!",
                  m.role::text AS "role!", m.status::text AS "statut!",
                  m.is_primary, m.job_title, m.invited_at, m.approved_at,
                  m.revoked_at, m.created_at
             FROM org.memberships m
             JOIN identity.people p ON p.id = m.person_id
            WHERE m.organization_id = $1
            ORDER BY m.status, m.role, p.display_name"#,
        id.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OrganizationMemberRow {
            id: l.id,
            person_id: PersonId(l.person_id),
            display_name: l.display_name,
            primary_email: l.primary_email,
            role: l.role,
            status: l.statut,
            is_primary: l.is_primary,
            job_title: l.job_title,
            invited_at: l.invited_at,
            approved_at: l.approved_at,
            revoked_at: l.revoked_at,
            created_at: l.created_at,
        })
        .collect())
}

/// Les activités — dossiers déposés ou co-organisés, et séances tenues.
///
/// **Lecture qui franchit la frontière du schéma `programme`** : la question
/// porte sur une entité d'`org`, et demander l'information au module `programme`
/// serait un appel direct d'un module à un autre.
pub async fn activites(
    conn: &mut PgConnection,
    id: OrganizationId,
) -> Result<Vec<OrganizationActivityRow>> {
    let lignes = sqlx::query!(
        r#"SELECT 'proposal'::text            AS "kind!",
                  p.id                        AS "id!",
                  p.reference_code            AS "reference_code?",
                  p.title::jsonb              AS "title!",
                  p.event_id                  AS "event_id!",
                  e.title::jsonb              AS "event_name!",
                  e.edition_year              AS "edition_year!",
                  COALESCE(po.role::text, 'lead') AS "role!",
                  p.status::text              AS "statut!",
                  COALESCE(p.submitted_at, p.created_at) AS "occurred_at?"
             FROM programme.proposals p
             JOIN event.events e ON e.id = p.event_id
             LEFT JOIN programme.proposal_organizations po
                    ON po.proposal_id = p.id AND po.organization_id = $1
            WHERE p.deleted_at IS NULL
              AND (p.organization_id = $1 OR po.organization_id = $1)

            UNION ALL

           SELECT 'session'::text, s.id, NULL, s.title::jsonb, s.event_id,
                  e.title::jsonb, e.edition_year, 'lead', s.status::text, s.starts_at
             FROM programme.sessions s
             JOIN event.events e ON e.id = s.event_id
            WHERE s.organization_id = $1

            ORDER BY 10 DESC NULLS LAST"#,
        id.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OrganizationActivityRow {
            kind: l.kind,
            id: l.id,
            reference_code: l.reference_code,
            title: l.title,
            event_id: l.event_id,
            event_name: l.event_name,
            edition_year: l.edition_year,
            role: l.role,
            status: l.statut,
            occurred_at: l.occurred_at,
        })
        .collect())
}

/// L'historique champ par champ. **Ce n'est pas une table** : c'est un
/// sous-produit du journal d'audit, et `actor_label` y est dénormalisé —
/// il reste lisible après anonymisation.
pub async fn historique(
    conn: &mut PgConnection,
    id: OrganizationId,
) -> Result<Vec<OrganizationHistoryEntry>> {
    let lignes = sqlx::query!(
        r#"SELECT occurred_at AS "occurred_at!", actor_id, actor_label,
                  action AS "action!", field, old_value, new_value
             FROM platform.entity_history('org', 'organizations', $1)"#,
        id.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OrganizationHistoryEntry {
            occurred_at: l.occurred_at,
            actor_id: l.actor_id.map(PersonId),
            actor_label: l.actor_label,
            action: l.action,
            field: l.field,
            old_value: l.old_value,
            new_value: l.new_value,
        })
        .collect())
}

/// Les fusions où la fiche apparaît, dans un sens ou dans l'autre.
pub async fn fusions(
    conn: &mut PgConnection,
    id: OrganizationId,
) -> Result<Vec<OrganizationMergeEntry>> {
    let lignes = sqlx::query!(
        r#"SELECT m.id, m.source_id, m.target_id,
                  COALESCE(src.legal_name, m.source_snapshot ->> 'legal_name', '')
                      AS "source_name!",
                  COALESCE(cible.legal_name, '') AS "target_name!",
                  p.display_name AS "performed_by_name?",
                  m.performed_at, m.rows_reassigned, m.reason
             FROM org.merge_log m
             LEFT JOIN org.organizations src   ON src.id = m.source_id
             LEFT JOIN org.organizations cible ON cible.id = m.target_id
             LEFT JOIN identity.people p       ON p.id = m.performed_by
            WHERE m.source_id = $1 OR m.target_id = $1
            ORDER BY m.performed_at DESC"#,
        id.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| OrganizationMergeEntry {
            id: l.id,
            source_id: OrganizationId(l.source_id),
            source_name: l.source_name,
            target_id: OrganizationId(l.target_id),
            target_name: l.target_name,
            performed_by_name: l.performed_by_name,
            performed_at: l.performed_at,
            rows_reassigned: l.rows_reassigned,
            reason: l.reason,
        })
        .collect())
}

/// Les fiches que celle-ci a absorbées.
pub async fn absorbees(conn: &mut PgConnection, id: OrganizationId) -> Result<Vec<MergedRef>> {
    let lignes = sqlx::query!(
        "SELECT id, legal_name, merged_at FROM org.organizations
          WHERE merged_into_id = $1 ORDER BY merged_at DESC",
        id.as_uuid()
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| MergedRef {
            organization_id: OrganizationId(l.id),
            legal_name: l.legal_name,
            merged_at: l.merged_at,
        })
        .collect())
}

/// Le nom d'une fiche, pour un renvoi. Utilisé par les refus qui doivent
/// **nommer** la fiche en cause.
pub async fn nom_de(conn: &mut PgConnection, id: Uuid) -> Result<Option<String>> {
    let nom = sqlx::query_scalar!("SELECT legal_name FROM org.organizations WHERE id = $1", id)
        .fetch_optional(conn)
        .await?;

    Ok(nom)
}
