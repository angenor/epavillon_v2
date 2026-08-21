//! **La déduction des transitions d'un dossier repris de la v1** (écart n° 37,
//! R20).
//!
//! # Ce que la reprise laisse
//!
//! Des dossiers dans leur **état final**, sans aucune ligne de journal.
//! L'insertion échappe au garde d'état — il n'est posé que sur la mise à jour
//! de `status` —, et le déclencheur d'insertion ne journalise que l'état de
//! départ, qui est ici l'état d'arrivée. La frise d'un dossier repris est donc
//! **vide**, alors que le dossier a été déposé, instruit et décidé.
//!
//! # Ce que la déduction sème, et rien de plus
//!
//! | Ligne | Instant | Condition |
//! |---|---|---|
//! | → brouillon | date de création | toujours |
//! | brouillon → déposé | date de dépôt | si elle existe |
//! | déposé → état final | date de décision | si elle existe **et** que l'état est retenu, non retenu, annulé ou retiré |
//!
//! **Ce qu'elle ne fait pas** : deviner un passage par l'évaluation, ni une
//! demande de correction. Ce qui n'est pas dans les dates du dossier n'est pas
//! déductible, et l'inventer serait pire qu'un trou — c'est ce que le front a
//! évité en franchissant l'étape d'évaluation dès qu'une décision existe.
//!
//! # 🔴 ELLE N'ÉMET AUCUN ÉVÉNEMENT, ET C'EST TOUT LE SUJET
//!
//! Elle écrit dans `proposal_transitions` **sans passer par la mise à jour de
//! l'état** : le déclencheur d'état ne s'éveille donc pas, et rien n'est émis.
//!
//! Émettre huit mille événements de dossiers décidés il y a deux ans
//! déclencherait autant de courriels — **le pire effet possible d'une
//! reprise**. C'est ce qui rend cette opération sûre, et c'est vérifié.
//!
//! # Rejouable, et la condition est DANS la requête
//!
//! « Journal vide » est une clause de l'insertion, pas un contrôle préalable :
//! deux exécutions simultanées ne peuvent pas semer deux fois le même dossier,
//! et une seconde exécution rend zéro.
//!
//! # Synchrone, et non un travail différé
//!
//! Elle est ponctuelle, et son résultat doit être **lu par celui qui la
//! lance** — « 3 812 dossiers, 11 436 lignes semées », pas un identifiant de
//! tâche.

use kernel::context::RequestContext;
use kernel::error::Result;
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::ProgrammeState;

/// Ce que la déduction rend — **des nombres, pas un identifiant de tâche**.
#[derive(Debug, Clone, Serialize, ToSchema, Default)]
pub struct ResultatDeReprise {
    /// Dossiers dont le journal était vide et qui ont reçu des lignes.
    pub proposals: i64,
    /// Lignes de journal semées, toutes natures confondues.
    pub transitions: i64,
}

/// Semer les transitions déductibles des dossiers **au journal vide**.
///
/// **Portée globale exigée** — vérifiée par la route : une reprise porte sur
/// tout le corpus, et la borner à une édition n'aurait aucun sens.
pub async fn deduire(state: &ProgrammeState, ctx: &RequestContext) -> Result<ResultatDeReprise> {
    let mut tx = state.db().write(ctx).await?;

    // Les trois lignes sont semées par **une seule** requête : la condition
    // « journal vide » y est, et deux requêtes la rendraient fausse entre
    // elles — la première ligne semée peuplerait le journal, et la deuxième
    // requête ne verrait plus aucun dossier à traiter.
    let semees = sqlx::query_scalar!(
        r#"WITH sans_journal AS (
               SELECT p.id, p.status, p.created_at, p.submitted_at, p.decided_at
                 FROM programme.proposals p
                WHERE p.deleted_at IS NULL
                  AND NOT EXISTS (
                      SELECT 1 FROM programme.proposal_transitions t
                       WHERE t.proposal_id = p.id
                  )
           ),
           lignes AS (
               -- 1. L'ouverture du dossier. Toujours : un dossier a été créé.
               --
               --    L'instant est BORNÉ par les deux autres dates, et ce n'est
               --    pas une précaution théorique : une reprise qui ne recopie
               --    pas la date d'origine laisse `created_at` à l'instant de
               --    l'IMPORT, c'est-à-dire après le dépôt et la décision. La
               --    frise afficherait alors « dossier créé » en dernier — faux,
               --    et visible du premier coup d'œil.
               --
               --    Quand la date de création est postérieure au dépôt, on la
               --    ramène **une microseconde avant** : c'est une convention
               --    assumée, pas une invention de date. L'alternative — deux
               --    lignes au même instant — laisserait l'ordre au hasard, les
               --    identifiants v7 n'étant pas ordonnés à l'intérieur d'une
               --    milliseconde.
               SELECT id AS proposal_id, 1 AS rang,
                      NULL::programme.proposal_status AS depuis,
                      'draft'::programme.proposal_status AS vers,
                      CASE
                          WHEN created_at < coalesce(submitted_at, decided_at, created_at + interval '1 second')
                          THEN created_at
                          ELSE coalesce(submitted_at, decided_at) - interval '1 microsecond'
                      END AS instant
                 FROM sans_journal
               UNION ALL
               -- 2. Le dépôt, si le dossier porte sa date.
               SELECT id, 2, 'draft'::programme.proposal_status,
                      'submitted'::programme.proposal_status, submitted_at
                 FROM sans_journal WHERE submitted_at IS NOT NULL
               UNION ALL
               -- 3. La décision, si elle existe ET que l'état est final. On ne
               --    devine NI le passage par l'évaluation, NI une demande de
               --    correction : ce qui n'est pas dans les dates n'est pas
               --    déductible, et l'inventer serait pire qu'un trou.
               SELECT id, 3, 'submitted'::programme.proposal_status, status, decided_at
                 FROM sans_journal
                WHERE decided_at IS NOT NULL
                  AND status IN ('accepted', 'rejected', 'cancelled', 'withdrawn')
           ),
           inserees AS (
               INSERT INTO programme.proposal_transitions
                   (proposal_id, from_status, to_status, actor_id, reason, occurred_at)
               SELECT proposal_id, depuis, vers, NULL, NULL, instant
                 FROM lignes
                ORDER BY proposal_id, rang
               RETURNING proposal_id
           )
           SELECT count(*) AS "semees!" FROM inserees"#
    )
    .fetch_one(&mut *tx)
    .await?;

    // Le décompte des dossiers se relit sur les lignes semées : compter avant
    // rendrait le nombre de dossiers *candidats*, qui n'est pas le même quand
    // deux exécutions se croisent.
    let dossiers = sqlx::query_scalar!(
        r#"SELECT count(DISTINCT proposal_id) AS "n!"
             FROM programme.proposal_transitions
            WHERE actor_id IS NULL AND reason IS NULL"#
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(ResultatDeReprise {
        proposals: if semees > 0 { dossiers } else { 0 },
        transitions: semees,
    })
}
