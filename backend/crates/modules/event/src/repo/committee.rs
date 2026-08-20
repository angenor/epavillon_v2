//! Lectures et écritures de `event.call_reviewers` — **qui siège**, et rien
//! d'autre.
//!
//! **Cette table dit la composition, pas le droit d'accès.** L'autorisation
//! reste portée par `identity.role_assignments` : siéger n'accorde rien.
//! Le nom, l'adresse, l'organisation, la charge et la détention effective de la
//! permission d'évaluer vivent hors du schéma `event` et se lisent dans
//! `repo/cross.rs`.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use sqlx::PgExecutor;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::ids::CallId;

/// Un siège, tel que la table le porte.
#[derive(Debug, Clone)]
pub struct Siege {
    pub person_id: Uuid,
    pub is_lead: bool,
    pub workload_cap: Option<i16>,
    pub added_at: OffsetDateTime,
}

/// La composition d'un appel. Les responsables d'abord, comme l'écran l'affiche.
pub async fn de_l_appel<'e>(executor: impl PgExecutor<'e>, call_id: CallId) -> Result<Vec<Siege>> {
    let lignes = sqlx::query!(
        "SELECT person_id, is_lead, workload_cap, added_at
           FROM event.call_reviewers
          WHERE call_id = $1
          ORDER BY is_lead DESC, added_at",
        call_id.as_uuid()
    )
    .fetch_all(executor)
    .await?;

    Ok(lignes
        .into_iter()
        .map(|l| Siege {
            person_id: l.person_id,
            is_lead: l.is_lead,
            workload_cap: l.workload_cap,
            added_at: l.added_at,
        })
        .collect())
}

/// Poser la composition **d'un seul geste**.
///
/// Trois ordres dans une transaction : retirer ce qui n'est plus là, mettre à
/// jour les plafonds et les responsables, ajouter les nouveaux. La charge utile
/// est **dédoublonnée par le service** avant d'arriver ici — la clé primaire
/// `(call_id, person_id)` ne doit jamais remonter.
pub async fn remplacer(
    conn: &mut PgConnection,
    call_id: CallId,
    sieges: &[(Uuid, bool, Option<i16>)],
) -> std::result::Result<(), sqlx::Error> {
    let (personnes, responsables, plafonds) = eclater(sieges);

    sqlx::query!(
        "DELETE FROM event.call_reviewers
          WHERE call_id = $1 AND person_id <> ALL($2::uuid[])",
        call_id.as_uuid(),
        &personnes
    )
    .execute(&mut *conn)
    .await?;

    if personnes.is_empty() {
        return Ok(());
    }

    sqlx::query!(
        "INSERT INTO event.call_reviewers (call_id, person_id, is_lead, workload_cap)
         SELECT $1, s.person_id, s.is_lead, s.workload_cap
           FROM unnest($2::uuid[], $3::boolean[], $4::int2[])
                AS s(person_id, is_lead, workload_cap)
         ON CONFLICT (call_id, person_id) DO UPDATE
            SET is_lead = EXCLUDED.is_lead, workload_cap = EXCLUDED.workload_cap",
        call_id.as_uuid(),
        &personnes,
        &responsables,
        &plafonds as &[Option<i16>]
    )
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Trois colonnes plutôt qu'un tableau de couples : `unnest` ne sait pas
/// déplier un type composite anonyme, et déclarer un type en base pour trois
/// valeurs serait une modification du modèle sans nécessité.
fn eclater(sieges: &[(Uuid, bool, Option<i16>)]) -> (Vec<Uuid>, Vec<bool>, Vec<Option<i16>>) {
    let mut personnes = Vec::with_capacity(sieges.len());
    let mut responsables = Vec::with_capacity(sieges.len());
    let mut plafonds = Vec::with_capacity(sieges.len());

    for (personne, responsable, plafond) in sieges {
        personnes.push(*personne);
        responsables.push(*responsable);
        plafonds.push(*plafond);
    }

    (personnes, responsables, plafonds)
}
