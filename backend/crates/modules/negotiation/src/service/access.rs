//! « Mon accès », le verrou et le parcours d'entrée : **une lecture, un écran**.
//!
//! Les quatre morceaux — le mode d'admission, l'accès en cours, les réseaux, la
//! demande — se lisent ensemble et se composent ici. Les laisser à l'écran
//! obligerait chaque page à connaître l'ordre de priorité des états, et la
//! première qui l'oublierait afficherait « visiteuse » à une personne dont
//! l'accès vient d'être retiré.

use kernel::auth::{has_permission, Scope};
use kernel::error::Result;

use crate::domain::access::MyAccess;
use crate::domain::permissions::REPORT_VALIDATE;
use crate::repo::{access, reports, settings};
use crate::state::NegotiationState;

pub async fn mon_acces(
    state: &NegotiationState,
    person_id: uuid::Uuid,
    locale: &str,
) -> Result<MyAccess> {
    let mut conn = state.pool().acquire().await?;

    let mode = settings::mode_dadmission(&mut conn).await?;
    let accorde = access::accorde(&mut conn, person_id, locale).await?;
    let reseaux = access::reseaux(&mut conn, person_id, locale).await?;
    let demande = access::derniere_demande(&mut conn, person_id).await?;

    // Le retrait ne se cherche que si rien n'est accordé : une personne dont
    // l'accès a été retiré puis rendu est admise, et l'ancien retrait n'a plus
    // rien à dire.
    let retire = match accorde {
        Some(_) => false,
        None => access::a_ete_retire(&mut conn, person_id).await?,
    };

    let mut etat = MyAccess::composer(mode, accorde, retire, reseaux, demande);
    if has_permission(state.pool(), person_id, REPORT_VALIDATE, Scope::Global).await? {
        etat.can_validate_reports = true;
        etat.reports_to_review = Some(reports::a_relire(&mut conn).await?);
    }
    Ok(etat)
}
