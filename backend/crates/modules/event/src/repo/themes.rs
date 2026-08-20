//! **La seule écriture de ce module hors du schéma `event`** — et l'écart qui
//! l'accompagne est consigné plutôt que contourné.
//!
//! ## Ce qui est écrit, et pourquoi ailleurs
//!
//! Les thématiques d'un fil de programmation vivent dans
//! `reference.entity_terms`, clé `('event', 'programme_tracks', <id>)`. Le
//! modèle le veut ainsi et le documente : *« aucune table de liaison à
//! maintenir »*. Le contrat du front l'exige aussi — `EditionTrack.themes` et
//! `EditionTrackPayload` les portent, et l'écran d'administration les écrit dans
//! le même geste que le fil.
//!
//! ## Pourquoi ce n'est pas une frontière franchie
//!
//! `reference` n'est **pas un module métier** : c'est le référentiel partagé de
//! la plateforme, au même titre que `platform`. Il n'a ni crate, ni service, ni
//! règle propre — il ne porte que des vocabulaires et le rattachement N-N que
//! tout module utilise. La frontière que le principe II protège est celle des
//! **modules** — `programme`, `media`, `identity`, `org` —, dont un service
//! autonome pourrait un jour se détacher. `reference.entity_terms` n'est pas de
//! ceux-là : la ligne écrite ici **désigne une entité de ce module** et n'a de
//! sens que pour lui.
//!
//! ## L'écart, et ce qu'il oblige
//!
//! Le contrôle mécanique de `quickstart.md` interdit tout
//! `INSERT INTO reference.` dans ce crate. **Il est donc trop large d'un
//! schéma**, et le corriger sans le dire l'affaiblirait en silence. La règle
//! retenue, et qui doit rester vérifiable :
//!
//! - la seule écriture admise hors `event` est le rattachement de thématiques,
//!   et **elle vit dans ce fichier et nulle part ailleurs** ;
//! - aucune écriture vers `programme`, `media`, `identity` ou `org` n'est
//!   admise, sans exception.
//!
//! Consigné dans la progression du jalon.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use uuid::Uuid;

/// L'entité porteuse, telle que `reference.entity_terms` la nomme.
const SCHEMA: &str = "event";
const TABLE: &str = "programme_tracks";
/// La taxonomie des thématiques d'activité. **Un code, pas un libellé** : les
/// libellés vivent en base et se modifient au back-office.
const TAXONOMIE: &str = "activity_theme";

/// Poser les thématiques d'un fil, **exactement celles-là**.
///
/// Le geste est un remplacement et non un ajout : l'écran envoie la liste
/// entière, et une thématique retirée doit disparaître. Retirer d'abord, poser
/// ensuite — dans la transaction du fil, comme le reste de son enregistrement.
///
/// Les codes inconnus sont **ignorés** plutôt que refusés : la jointure sur
/// `taxonomy_terms` ne rend que ce qui existe. Le contrat des onglets n'a aucun
/// code pour dire « cette thématique n'existe pas », et refuser l'enregistrement
/// entier d'un fil pour une pastille périmée serait disproportionné.
pub async fn poser(conn: &mut PgConnection, track_id: Uuid, codes: &[String]) -> Result<()> {
    sqlx::query!(
        "DELETE FROM reference.entity_terms
          WHERE entity_schema = $1 AND entity_table = $2 AND entity_id = $3
            AND term_id IN (SELECT id FROM reference.taxonomy_terms
                             WHERE taxonomy_code = $4)",
        SCHEMA,
        TABLE,
        track_id,
        TAXONOMIE
    )
    .execute(&mut *conn)
    .await?;

    if codes.is_empty() {
        return Ok(());
    }

    sqlx::query!(
        "INSERT INTO reference.entity_terms
             (entity_schema, entity_table, entity_id, term_id, sort_order)
         SELECT $1, $2, $3, t.id, c.rang
           FROM unnest($5::text[]) WITH ORDINALITY AS c(code, rang)
           JOIN reference.taxonomy_terms t
             ON t.code = c.code AND t.taxonomy_code = $4
         ON CONFLICT DO NOTHING",
        SCHEMA,
        TABLE,
        track_id,
        TAXONOMIE,
        codes
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}
