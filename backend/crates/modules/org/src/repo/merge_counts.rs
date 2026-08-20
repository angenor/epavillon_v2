//! **LE SEUL FICHIER DU MODULE QUI COMPOSE SON SQL.** Voici pourquoi.
//!
//! Le décompte de transfert doit parcourir `org.organization_references`, qui
//! est un **registre** : dix-huit lignes aujourd'hui, davantage quand les
//! modules hors jalon vivront. Énumérer ces tables en Rust est exactement ce que
//! le registre existe pour éviter — le décompte deviendrait faux au premier
//! module qui s'y déclare, **et personne ne le verrait**.
//!
//! Surtout, `org.merge_organizations()` compose déjà son SQL **de la même façon
//! et à partir de la même source**. Le décompte annoncé avant et le décompte
//! rendu après sont alors calculés par le même raisonnement, ce qui est la seule
//! façon de tenir SC-010 : l'aperçu ne peut pas mentir sans que la fusion mente
//! aussi.
//!
//! **Une seule requête, et non dix-huit.** Dix-huit allers-retours coûteraient
//! une dizaine de millisecondes pour un écran qui se recharge à chaque inversion
//! du sens.
//!
//! **Le risque d'injection est nul et néanmoins couvert.** Les identifiants
//! viennent d'une table alimentée par le DDL, jamais par un utilisateur — et ils
//! sont cités par `quote_ident`, parce qu'une table de configuration reste une
//! table.
//!
//! ## Un écart, mesuré et borné : les lignes qu'un trigger déplace en chemin
//!
//! Le décompte est pris **sur l'état d'avant**. Or la fonction de fusion parcourt
//! le registre dans l'ordre, et certaines de ses écritures en déclenchent
//! d'autres : réaffecter une adhésion réveille `tg_memberships_sync_primary`,
//! qui déplace `identity.people.primary_organization_id` **avant** que la boucle
//! n'arrive à cette ligne du registre. Le journal compte alors moins de lignes
//! que l'aperçu n'en annonçait, et l'écart n'est pas une erreur de calcul : les
//! lignes ont bien été déplacées, simplement pas par l'ordre qui les comptait.
//!
//! **Une seule ligne du registre est concernée aujourd'hui** —
//! `identity.people.primary_organization_id`, la seule qu'un trigger d'un autre
//! agrégat touche. Les dix-sept autres sont exactes au chiffre près.
//!
//! Le corriger demanderait de reproduire l'effet du trigger dans le décompte,
//! c'est-à-dire de réimplémenter un invariant de la base — ce que le principe
//! VIII interdit. L'autre issue serait de simuler la fusion sous un point de
//! reprise puis de l'annuler : exact par construction, mais c'est exécuter une
//! fonction `SECURITY DEFINER` « pour voir » à chaque inversion du sens à
//! l'écran. Le point est consigné plutôt que tranché à la légère.
//!
//! Alternative écartée : ajouter une fonction `org.count_merge_transfers()` au
//! modèle. Ce serait la plus élégante, et elle demande une modification du SQL
//! que rien n'impose — le besoin est entièrement satisfait depuis l'application.
//! À reproposer si un second appelant apparaît (research.md § R4).

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::Row;
use uuid::Uuid;

use crate::domain::merge::MergeTransferLine;

/// Une entrée du registre.
pub struct Reference {
    pub ref_schema: String,
    pub ref_table: String,
    pub ref_column: String,
    pub strategy: String,
    pub dedupe_on: Vec<String>,
}

/// Le registre entier, dans l'ordre où il se lit à l'écran.
pub async fn registre(conn: &mut PgConnection) -> Result<Vec<Reference>> {
    let lignes = sqlx::query!(
        "SELECT ref_schema, ref_table, ref_column, strategy, dedupe_on
           FROM org.organization_references
          ORDER BY ref_schema, ref_table, ref_column"
    )
    .fetch_all(conn)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Reference {
            ref_schema: l.ref_schema,
            ref_table: l.ref_table,
            ref_column: l.ref_column,
            strategy: l.strategy,
            dedupe_on: l.dedupe_on,
        })
        .collect())
}

/// Chiffre le transfert, **pour un sens donné**.
///
/// Une branche `UNION ALL` par ligne du registre, rendant les trois chiffres :
/// lignes qui basculeront, lignes supprimées parce que la cible porte déjà la
/// valeur, lignes supprimées par stratégie.
pub async fn chiffrer(
    conn: &mut PgConnection,
    source: Uuid,
    cible: Uuid,
) -> Result<Vec<MergeTransferLine>> {
    let references = registre(conn).await?;
    if references.is_empty() {
        return Ok(Vec::new());
    }

    let mut branches = Vec::with_capacity(references.len());
    for (rang, r) in references.iter().enumerate() {
        branches.push(branche(rang, r, conn).await?);
    }

    let requete = branches.join("\n UNION ALL\n");

    // `AssertSqlSafe` est le garde-fou de SQLx sur le SQL composé, et il demande
    // une affirmation explicite. Elle est fondée : chaque identifiant de cette
    // requête sort de `org.organization_references`, table alimentée par le DDL
    // et par lui seul, et chacun est passé par `quote_ident`. Les deux seules
    // valeurs — la source et la cible — sont des paramètres liés.
    let lignes = sqlx::query(sqlx::AssertSqlSafe(requete))
        .bind(source)
        .bind(cible)
        .fetch_all(conn)
        .await?;

    let mut chiffres: Vec<(usize, i64, i64)> = lignes
        .iter()
        .map(|l| {
            (
                l.get::<i32, _>("rang") as usize,
                l.get::<i64, _>("total"),
                l.get::<i64, _>("en_conflit"),
            )
        })
        .collect();
    chiffres.sort_by_key(|(rang, _, _)| *rang);

    Ok(references
        .into_iter()
        .enumerate()
        .map(|(rang, r)| {
            let (_, total, en_conflit) = chiffres
                .iter()
                .find(|(i, _, _)| *i == rang)
                .copied()
                .unwrap_or((rang, 0, 0));

            // **Trois sorts, trois chiffres.** Une ligne `delete` disparaît
            // entièrement ; une ligne `reassign` bascule, sauf celles dont la
            // valeur existe déjà côté cible — celles-là sont supprimées avant la
            // bascule, sans quoi l'unicité ferait échouer la fusion.
            let (reassigned, deduped, deleted) = if r.strategy == "delete" {
                (0, 0, total)
            } else {
                (total - en_conflit, en_conflit, 0)
            };

            MergeTransferLine {
                ref_schema: r.ref_schema,
                ref_table: r.ref_table,
                ref_column: r.ref_column,
                strategy: r.strategy,
                dedupe_on: r.dedupe_on,
                reassigned,
                deduped,
                deleted,
            }
        })
        .collect())
}

/// Une branche de l'union : le total des lignes de la source, et combien
/// d'entre elles trouvent leur pareille côté cible.
///
/// Les identifiants sont cités par `quote_ident` — la base le fait elle-même,
/// c'est la seule façon de citer comme elle cite.
async fn branche(rang: usize, r: &Reference, conn: &mut PgConnection) -> Result<String> {
    let schema = quote_ident(conn, &r.ref_schema).await?;
    let table = quote_ident(conn, &r.ref_table).await?;
    let colonne = quote_ident(conn, &r.ref_column).await?;

    // Sans colonne de dédoublonnage, aucune ligne n'est en conflit : la bascule
    // ne peut pas produire de doublon.
    let en_conflit = if r.dedupe_on.is_empty() {
        "0::bigint".to_owned()
    } else {
        let mut egalites = Vec::with_capacity(r.dedupe_on.len());
        for col in &r.dedupe_on {
            let cite = quote_ident(conn, col).await?;
            egalites.push(format!("t.{cite} IS NOT DISTINCT FROM s.{cite}"));
        }
        format!(
            "count(*) FILTER (WHERE EXISTS (
                 SELECT 1 FROM {schema}.{table} t
                  WHERE t.{colonne} = $2 AND {}))",
            egalites.join(" AND ")
        )
    };

    Ok(format!(
        "SELECT {rang}::int AS rang,
                count(*)::bigint AS total,
                ({en_conflit})::bigint AS en_conflit
           FROM {schema}.{table} s
          WHERE s.{colonne} = $1"
    ))
}

/// La citation, faite **par la base**. Réécrire la règle en Rust ferait diverger
/// d'un cas particulier — une majuscule, un mot réservé — et le SQL composé
/// deviendrait invalide sans qu'on sache pourquoi.
async fn quote_ident(conn: &mut PgConnection, identifiant: &str) -> Result<String> {
    let cite = sqlx::query_scalar!(r#"SELECT quote_ident($1) AS "cite!""#, identifiant)
        .fetch_one(conn)
        .await?;

    Ok(cite)
}
