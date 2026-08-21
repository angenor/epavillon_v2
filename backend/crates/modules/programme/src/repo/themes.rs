//! **Écriture hors schéma n° 1 : les thématiques d'un dossier** (R11, écarts
//! n° 3 et n° 94).
//!
//! # Pourquoi cette table n'a pas d'autre porte
//!
//! Les thématiques vivent dans `reference.entity_terms`, table **polymorphe et
//! sans clé étrangère** vers les propositions. Aucun autre module ne peut poser
//! les thématiques d'un dossier, et aucune contrainte référentielle ne les
//! purge. C'est exactement la dérogation que B3 s'est accordée pour les fils de
//! programmation, bornée de la même façon : **un seul fichier, où un ajout se
//! discute**.
//!
//! `reference` n'est pas un module métier : c'est le référentiel partagé de la
//! plateforme, sans crate, sans service et sans règle propre. La frontière que
//! le principe II protège est celle des **modules** — dont un service autonome
//! pourrait un jour se détacher.
//!
//! # Le triplet est écrit LITTÉRALEMENT, jamais reçu
//!
//! `('programme', 'proposals', <id>)` est posé par ce fichier. Accepter les
//! trois champs dans la charge utile permettrait à un client de rattacher des
//! thématiques à **n'importe quelle entité de n'importe quel schéma** : c'est
//! l'écart n° 3 dans son intégralité, et il n'a pas d'autre remède que de ne
//! jamais lire ces trois champs.
//!
//! # Pourquoi la purge est à nous
//!
//! Le commentaire du modèle annonce une fonction de nettoyage qui **n'existe
//! pas** (écart n° 94). Sans la purge de ce fichier, un dossier effacé laisse
//! ses liens derrière lui — et ils ressortent au premier filtre par thématique.

use kernel::error::{ApiError, ErrorCode, Result};
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// L'entité porteuse, telle que `reference.entity_terms` la nomme. **Écrite
/// ici, jamais reçue.**
const SCHEMA: &str = "programme";
const TABLE: &str = "proposals";
/// La taxonomie attendue. **Un code, pas un libellé** : les libellés vivent en
/// base et se modifient au back-office.
const TAXONOMIE: &str = "activity_theme";

/// Poser les thématiques d'un dossier, **exactement celles-là**.
///
/// Le geste est un remplacement et non un ajout : l'écran envoie la liste
/// entière, et une thématique retirée doit disparaître.
///
/// # Un code inconnu est REFUSÉ, et c'est l'inverse de B3
///
/// B3 ignorait les codes inconnus, faute d'un code d'erreur pour le dire. Ici
/// le catalogue en porte un — `PROPOSAL_UNKNOWN_TERM` —, et la classification
/// est une étape à part entière du formulaire : accepter en silence une
/// pastille périmée ferait déposer un dossier que le comité ne retrouverait sur
/// aucun filtre, sans que personne ne soit averti.
pub async fn poser(conn: &mut PgConnection, proposal_id: Uuid, codes: &[String]) -> Result<()> {
    purger(conn, proposal_id).await?;

    // Dédoublonné AVANT l'insertion, et sans trier : le rang de la liste porte
    // l'ordre d'affichage des pastilles. Sans cette étape, deux fois la même
    // pastille rendrait un décompte inférieur à la liste reçue, et le contrôle
    // ci-dessous accuserait un code parfaitement valide.
    let mut codes: Vec<String> = codes.to_vec();
    let mut vus = std::collections::HashSet::new();
    codes.retain(|c| vus.insert(c.clone()));
    let codes = codes.as_slice();

    if codes.is_empty() {
        return Ok(());
    }

    let poses = sqlx::query_scalar!(
        r#"WITH pose AS (
               INSERT INTO reference.entity_terms
                   (entity_schema, entity_table, entity_id, term_id, sort_order)
               SELECT $1, $2, $3, t.id, c.rang
                 FROM unnest($5::text[]) WITH ORDINALITY AS c(code, rang)
                 JOIN reference.taxonomy_terms t
                   ON t.code = c.code AND t.taxonomy_code = $4
               ON CONFLICT DO NOTHING
               RETURNING term_id
           )
           SELECT count(*) AS "n!" FROM pose"#,
        SCHEMA,
        TABLE,
        proposal_id,
        TAXONOMIE,
        codes
    )
    .fetch_one(&mut *conn)
    .await?;

    // Le décompte est la seule façon de savoir qu'un code n'a pas été honoré :
    // la jointure ne rend que ce qui existe, et sans cette comparaison le refus
    // serait silencieux — exactement ce que l'écart n° 3 reproche.
    if poses < codes.len() as i64 {
        return Err(nommer_le_code_refuse(conn, codes).await);
    }

    Ok(())
}

/// Retirer tous les liens de thématique d'un dossier.
///
/// **La purge se borne à la taxonomie des thématiques.** Un dossier peut porter
/// d'autres rattachements un jour ; les effacer tous ferait de ce fichier une
/// porte plus large que ce qu'il déclare.
pub async fn purger(conn: &mut PgConnection, proposal_id: Uuid) -> Result<()> {
    sqlx::query!(
        "DELETE FROM reference.entity_terms
          WHERE entity_schema = $1 AND entity_table = $2 AND entity_id = $3
            AND term_id IN (SELECT id FROM reference.taxonomy_terms
                             WHERE taxonomy_code = $4)",
        SCHEMA,
        TABLE,
        proposal_id,
        TAXONOMIE
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// **Nommer le code refusé**, et non « une thématique est inconnue » : l'écran
/// doit pouvoir retirer la bonne pastille.
async fn nommer_le_code_refuse(conn: &mut PgConnection, codes: &[String]) -> ApiError {
    let connus: Vec<String> = sqlx::query_scalar!(
        "SELECT code FROM reference.taxonomy_terms
          WHERE taxonomy_code = $1 AND code = ANY($2)",
        TAXONOMIE,
        codes
    )
    .fetch_all(conn)
    .await
    .unwrap_or_default();

    match codes.iter().find(|c| !connus.contains(c)) {
        Some(inconnu) => ApiError::with_message(
            ErrorCode::ProposalUnknownTerm,
            format!("La thématique « {inconnu} » n'existe pas."),
        )
        .field("theme_codes"),
        // La liste est dédoublonnée en amont et tous ses codes existent : le
        // seul cas restant est un lien déjà posé par un autre rôle sur la même
        // entité. Il n'y a rien à refuser, mais rien non plus qui doive passer
        // en silence — le refus générique porte alors le champ.
        None => ApiError::new(ErrorCode::ProposalUnknownTerm).field("theme_codes"),
    }
}
