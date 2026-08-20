//! La liste du back-office — **une requête**, facettes comprises.
//!
//! Elle compose trois choses :
//!
//! 1. **la table vivante**, qui décide de ce qui figure dans la liste, et d'où
//!    sont relues les quatre colonnes qui bougent au geste de l'opérateur —
//!    statut, sceau, score de confiance, pointeur de fusion (FR-048) ;
//! 2. la **projection analytique** `analytics.mv_organization_scorecard`, qui
//!    porte les compteurs, jointe **par la gauche** ;
//! 3. le **filtre de périmètre**, qui lit le schéma `programme`.
//!
//! **La jointure est par la gauche, et ce n'est pas une précaution de style.**
//! La projection n'est rafraîchie que par un travail différé : une fiche créée
//! il y a dix secondes n'y figure pas encore. Une jointure interne la ferait
//! disparaître de la liste du back-office — c'est-à-dire précisément de l'écran
//! où l'équipe va la chercher pour la traiter. Ses compteurs valent alors zéro,
//! ce qui est vrai : une fiche que rien ne référence encore n'a rien à compter.
//!
//! **Cette lecture franchit une frontière de schéma, et c'est décidé** (R14) :
//! une organisation n'appartient à aucune édition — c'est l'activité déposée ou
//! tenue qui la rattache à un périmètre. Demander l'information au module
//! `programme` serait un appel direct d'un module à un autre, que le principe IV
//! interdit sans détour. Le registre `org.organization_references` déclare
//! d'ailleurs ces mêmes colonnes depuis le premier jour : le modèle a prévu
//! qu'`org` connaisse qui le référence.
//!
//! **Les facettes sont comptées dans la même requête**, sur le même jeu de
//! lignes (FR-046) : les demander à part ferait diverger « Sénégal (3) » de ce
//! qui s'affiche.

use kernel::auth::AdminScope;
use kernel::error::Result;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::admin::{OrganizationFacet, OrganizationListRow};
use crate::domain::ids::OrganizationId;

/// La liste, filtrée par périmètre.
pub async fn rows(pool: &PgPool, perimetre: &AdminScope) -> Result<Vec<OrganizationListRow>> {
    let lignes = sqlx::query_as!(
        Ligne,
        r#"SELECT o.id                     AS "organization_id!",
                  o.legal_name             AS "legal_name!",
                  o.acronym,
                  o.slug::text             AS "slug!",
                  -- Les quatre colonnes vives, relues sur la table : la
                  -- projection a le retard de son dernier rafraîchissement.
                  o.status::text           AS "statut!",
                  (o.verified_at IS NOT NULL) AS "est_verifiee!",
                  o.verified_at,
                  o.trust_score            AS "score_confiance!",
                  o.merged_into_id,
                  o.organization_type_code AS "organization_type_code!",
                  t.label::jsonb           AS "organization_type_label?",
                  t.color_hex              AS "organization_type_color?",
                  o.country_id,
                  c.iso3                   AS "pays_iso3?",
                  c.name::jsonb            AS "pays_nom?",
                  COALESCE(s.statut_oif, 'none')       AS "statut_oif!",
                  COALESCE(s.membres_actifs, 0)        AS "membres_actifs!",
                  COALESCE(s.membres_en_attente, 0)    AS "membres_en_attente!",
                  COALESCE(s.referents, 0)             AS "referents!",
                  COALESCE(s.propositions_deposees, 0) AS "propositions_deposees!",
                  COALESCE(s.propositions_acceptees, 0) AS "propositions_acceptees!",
                  COALESCE(s.propositions_rejetees, 0) AS "propositions_rejetees!",
                  s.ratio_acceptation::float8          AS "ratio_acceptation?",
                  COALESCE(s.sessions_programmees, 0)  AS "sessions_programmees!",
                  COALESCE(s.sessions_realisees, 0)    AS "sessions_realisees!",
                  (SELECT count(*) FROM org.duplicate_candidates d
                    WHERE (d.left_id = o.id OR d.right_id = o.id)
                      AND d.reviewed_at IS NULL)  AS "pending_duplicate_count!",
                  (SELECT count(*) FROM org.organizations a
                    WHERE a.merged_into_id = o.id) AS "absorbed_count!",
                  COALESCE(s.derniere_activite, o.updated_at) AS "derniere_activite?",
                  o.created_at             AS "inscrite_le!"
             FROM org.organizations o
             LEFT JOIN analytics.mv_organization_scorecard s ON s.organization_id = o.id
             LEFT JOIN reference.countries c ON c.id = o.country_id
             LEFT JOIN reference.taxonomy_terms t
                    ON t.taxonomy_code = 'organization_type'
                   AND t.code = o.organization_type_code
            WHERE $1::boolean
               OR EXISTS (
                      SELECT 1 FROM programme.proposals p
                       WHERE p.organization_id = o.id
                         AND p.deleted_at IS NULL
                         AND p.event_id = ANY($2::uuid[])
                  )
               OR EXISTS (
                      SELECT 1 FROM programme.proposal_organizations po
                       JOIN programme.proposals p ON p.id = po.proposal_id
                       WHERE po.organization_id = o.id
                         AND p.deleted_at IS NULL
                         AND p.event_id = ANY($2::uuid[])
                  )
               OR EXISTS (
                      SELECT 1 FROM programme.sessions ses
                       WHERE ses.organization_id = o.id
                         AND ses.event_id = ANY($2::uuid[])
                  )
            ORDER BY o.legal_name"#,
        perimetre.is_global,
        &perimetre.event_ids
    )
    .fetch_all(pool)
    .await?;

    Ok(lignes.into_iter().map(OrganizationListRow::from).collect())
}

/// Les facettes, comptées **sur le même jeu de lignes**.
///
/// Elles se dérivent des lignes déjà lues plutôt que d'une seconde requête :
/// c'est le seul moyen que « Sénégal (3) » corresponde à ce qui s'affiche, et le
/// coût est nul — la liste tient en mémoire, elle est déjà bornée par le
/// périmètre.
pub fn facettes(
    lignes: &[OrganizationListRow],
) -> (Vec<OrganizationFacet>, Vec<OrganizationFacet>) {
    use std::collections::BTreeMap;

    let mut pays: BTreeMap<String, (Option<Value>, i64)> = BTreeMap::new();
    let mut types: BTreeMap<String, (Option<Value>, i64)> = BTreeMap::new();

    for ligne in lignes {
        if let Some(id) = ligne.country_id {
            let entree = pays
                .entry(id.to_string())
                .or_insert((ligne.pays_nom.clone(), 0));
            entree.1 += 1;
        }
        let entree = types
            .entry(ligne.organization_type_code.clone())
            .or_insert((ligne.organization_type_label.clone(), 0));
        entree.1 += 1;
    }

    let en_facettes = |m: BTreeMap<String, (Option<Value>, i64)>| {
        let mut v: Vec<OrganizationFacet> = m
            .into_iter()
            .map(|(value, (label, count))| OrganizationFacet {
                value,
                label,
                count,
            })
            .collect();
        // Les plus fournies d'abord : c'est l'ordre qu'un filtre se lit.
        v.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
        v
    };

    (en_facettes(pays), en_facettes(types))
}

/// Les paires non arbitrées, toutes fiches confondues : la pastille de la file.
///
/// Elle n'est **pas filtrée par périmètre**, et ce n'est pas un oubli : une
/// paire ne relève d'aucune édition. L'écran ne l'affiche qu'à qui peut ouvrir
/// la file, c'est-à-dire à la portée globale.
pub async fn paires_ouvertes(pool: &PgPool) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM org.duplicate_candidates WHERE reviewed_at IS NULL"#
    )
    .fetch_one(pool)
    .await?;

    Ok(n)
}

struct Ligne {
    organization_id: Uuid,
    legal_name: String,
    acronym: Option<String>,
    slug: String,
    statut: String,
    est_verifiee: bool,
    verified_at: Option<time::OffsetDateTime>,
    score_confiance: i16,
    merged_into_id: Option<Uuid>,
    organization_type_code: String,
    organization_type_label: Option<Value>,
    organization_type_color: Option<String>,
    country_id: Option<Uuid>,
    pays_iso3: Option<String>,
    pays_nom: Option<Value>,
    statut_oif: String,
    membres_actifs: i64,
    membres_en_attente: i64,
    referents: i64,
    propositions_deposees: i64,
    propositions_acceptees: i64,
    propositions_rejetees: i64,
    ratio_acceptation: Option<f64>,
    sessions_programmees: i64,
    sessions_realisees: i64,
    pending_duplicate_count: i64,
    absorbed_count: i64,
    derniere_activite: Option<time::OffsetDateTime>,
    inscrite_le: time::OffsetDateTime,
}

impl From<Ligne> for OrganizationListRow {
    fn from(l: Ligne) -> Self {
        Self {
            organization_id: OrganizationId(l.organization_id),
            legal_name: l.legal_name,
            acronym: l.acronym,
            slug: l.slug,
            statut: l.statut,
            organization_type_code: l.organization_type_code,
            organization_type_label: l.organization_type_label,
            organization_type_color: l.organization_type_color,
            country_id: l.country_id,
            pays_iso3: l.pays_iso3,
            pays_nom: l.pays_nom,
            statut_oif: l.statut_oif,
            est_verifiee: l.est_verifiee,
            verified_at: l.verified_at,
            score_confiance: l.score_confiance,
            merged_into_id: l.merged_into_id.map(OrganizationId),
            membres_actifs: l.membres_actifs,
            membres_en_attente: l.membres_en_attente,
            referents: l.referents,
            propositions_deposees: l.propositions_deposees,
            propositions_acceptees: l.propositions_acceptees,
            propositions_rejetees: l.propositions_rejetees,
            ratio_acceptation: l.ratio_acceptation,
            sessions_programmees: l.sessions_programmees,
            sessions_realisees: l.sessions_realisees,
            pending_duplicate_count: l.pending_duplicate_count,
            absorbed_count: l.absorbed_count,
            derniere_activite: l.derniere_activite,
            inscrite_le: l.inscrite_le,
        }
    }
}
