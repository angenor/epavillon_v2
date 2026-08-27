//! Les six indicateurs, les deux courbes, les deux répartitions.
//!
//! # `null` N'EST JAMAIS ZÉRO
//!
//! C'est la distinction qui coûte le plus cher ici. Un taux d'acceptation nul
//! signifie qu'aucun dossier n'a été tranché ; affiché « 0 % », il ferait passer
//! un comité qui n'a pas commencé pour un comité qui a tout refusé.
//!
//! # LA VARIATION HEBDOMADAIRE EST NULLE SOUS QUATORZE JOURS
//!
//! Comparer sept jours à une semaine tronquée est un artefact, pas une
//! tendance : l'indicateur n'affiche alors aucune variation plutôt qu'une
//! fausse.

use kernel::error::Result;
use sqlx::postgres::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::figures::{
    BreakdownSlice, DashboardFigures, DashboardKpi, DashboardKpiKey, DashboardKpiTone, TrendPoint,
};
use crate::repo::cross::{event, programme};
use crate::repo::{health, projections, reference};

/// Longueur de l'étincelle : trois semaines, ce que la carte peut montrer sans
/// devenir un graphique.
const ETINCELLE: usize = 21;

/// Sous cette longueur de série, la variation hebdomadaire n'est pas calculée.
const SERIE_MINIMALE: usize = 14;

/// Nombre de parts d'une répartition avant regroupement.
const PARTS: usize = 8;

pub async fn composer(
    conn: &mut PgConnection,
    event_id: Uuid,
    call_id: Option<Uuid>,
) -> Result<DashboardFigures> {
    let funnel = projections::entonnoir(&mut *conn, event_id, call_id).await?;
    let depots = projections::depots_par_jour(&mut *conn, event_id).await?;
    let inscriptions = projections::inscriptions_par_jour(&mut *conn, event_id).await?;
    let charge = projections::charge_du_comite(&mut *conn, event_id).await?;

    let deadline = match call_id {
        Some(id) => event::echeance(&mut *conn, id).await?,
        None => None,
    };
    let call_opens_at = funnel.as_ref().and_then(|f| f.appel_ouvre_le);

    let maintenant = maintenant(&mut *conn).await?;

    let submissions: Vec<TrendPoint> = depots
        .iter()
        .map(|j| TrendPoint {
            jour: j.jour,
            valeur: j.valeur,
            cumul: j.cumul,
            moyenne_7j: j.moyenne_7j,
        })
        .collect();
    let registrations: Vec<TrendPoint> = inscriptions
        .iter()
        .map(|j| TrendPoint {
            jour: j.jour,
            valeur: j.valeur,
            cumul: j.cumul,
            moyenne_7j: j.moyenne_7j,
        })
        .collect();

    let assignees: i64 = charge.iter().map(|c| c.propositions_assignees).sum();
    let rendues: i64 = charge.iter().map(|c| c.revues_soumises).sum();
    let en_retard: i64 = charge.iter().map(|c| c.revues_en_retard).sum();

    let kpis = vec![
        indicateur_de_serie(DashboardKpiKey::Submissions, &submissions),
        echeance_restante(deadline, maintenant),
        avancement_du_comite(rendues, assignees, en_retard),
        taux_dacceptation(funnel.as_ref()),
        activites_programmees(funnel.as_ref()),
        indicateur_de_serie(DashboardKpiKey::Registrations, &registrations),
    ];

    let pays = reference::pays(&mut *conn).await?;
    let thematiques = reference::thematiques(&mut *conn).await?;

    let by_country = repartir(
        programme::par_pays(&mut *conn, event_id).await?,
        |cle| pays.get(cle).cloned(),
        |_| None,
    );
    let by_theme = repartir(
        programme::par_thematique(&mut *conn, event_id).await?,
        |cle| thematiques.get(cle).map(|t| t.label.clone()),
        |cle| thematiques.get(cle).and_then(|t| t.color_hex.clone()),
    );

    Ok(DashboardFigures {
        kpis,
        funnel,
        submissions,
        registrations,
        deadline,
        call_opens_at,
        by_country,
        by_theme,
        refreshed_at: health::rafraichi_le(&mut *conn).await?,
    })
}

async fn maintenant(conn: &mut PgConnection) -> Result<OffsetDateTime> {
    let instant = sqlx::query_scalar!(r#"SELECT now() AS "maintenant!""#)
        .fetch_one(conn)
        .await?;
    Ok(instant)
}

/// Un indicateur adossé à une série quotidienne : le cumul, sa variation
/// hebdomadaire et son étincelle.
fn indicateur_de_serie(key: DashboardKpiKey, serie: &[TrendPoint]) -> DashboardKpi {
    if serie.is_empty() {
        return DashboardKpi {
            key,
            value: None,
            out_of: None,
            delta: None,
            at: None,
            spark: Vec::new(),
            tone: DashboardKpiTone::Neutral,
        };
    }

    let cumul = serie.last().map(|p| p.cumul as f64);
    let delta = variation_hebdomadaire(serie);
    let spark = serie
        .iter()
        .rev()
        .take(ETINCELLE)
        .rev()
        .map(|p| p.valeur)
        .collect();

    DashboardKpi {
        key,
        value: cumul,
        out_of: None,
        delta,
        at: None,
        spark,
        tone: DashboardKpiTone::Neutral,
    }
}

/// **Nulle sous quatorze jours de série** : une comparaison sur une semaine
/// tronquée est un artefact.
fn variation_hebdomadaire(serie: &[TrendPoint]) -> Option<f64> {
    if serie.len() < SERIE_MINIMALE {
        return None;
    }
    let n = serie.len();
    let derniers: i64 = serie[n - 7..].iter().map(|p| p.valeur).sum();
    let precedents: i64 = serie[n - 14..n - 7].iter().map(|p| p.valeur).sum();
    Some((derniers - precedents) as f64)
}

/// Les jours restants avant l'échéance qui fait foi. **Négatif quand elle est
/// passée** — l'écran doit pouvoir dire « échue depuis trois jours ».
fn echeance_restante(deadline: Option<OffsetDateTime>, maintenant: OffsetDateTime) -> DashboardKpi {
    let jours = deadline.map(|d| (d - maintenant).whole_days() as f64);
    let tone = match jours {
        Some(j) if j < 0.0 => DashboardKpiTone::Danger,
        Some(j) if j <= 7.0 => DashboardKpiTone::Warning,
        Some(_) => DashboardKpiTone::Accent,
        None => DashboardKpiTone::Neutral,
    };

    DashboardKpi {
        key: DashboardKpiKey::Deadline,
        value: jours,
        out_of: None,
        delta: None,
        at: deadline,
        spark: Vec::new(),
        tone,
    }
}

/// Revues rendues sur revues attendues, **retards à part**.
fn avancement_du_comite(rendues: i64, assignees: i64, en_retard: i64) -> DashboardKpi {
    // Aucune affectation : le comité n'a pas commencé, ce n'est pas « 0 sur 0 ».
    if assignees == 0 {
        return DashboardKpi {
            key: DashboardKpiKey::ReviewProgress,
            value: None,
            out_of: None,
            delta: None,
            at: None,
            spark: Vec::new(),
            tone: DashboardKpiTone::Neutral,
        };
    }

    let tone = if en_retard > 0 {
        DashboardKpiTone::Warning
    } else if rendues >= assignees {
        DashboardKpiTone::Success
    } else {
        DashboardKpiTone::Neutral
    };

    DashboardKpi {
        key: DashboardKpiKey::ReviewProgress,
        value: Some(rendues as f64),
        out_of: Some(assignees as f64),
        delta: None,
        at: None,
        spark: Vec::new(),
        tone,
    }
}

/// **Sur les dossiers TRANCHÉS**, jamais sur les dépôts : c'est la sélectivité
/// du comité, pas le rendement de l'appel — la projection porte les deux, et les
/// confondre change le sens du chiffre.
fn taux_dacceptation(funnel: Option<&crate::domain::figures::ProposalFunnelRow>) -> DashboardKpi {
    let value = funnel.and_then(|f| f.taux_acceptation);
    DashboardKpi {
        key: DashboardKpiKey::AcceptanceRate,
        value,
        out_of: None,
        delta: None,
        at: None,
        spark: Vec::new(),
        tone: DashboardKpiTone::Neutral,
    }
}

/// Activités retenues déjà placées au calendrier. **Aucun dénominateur** : le
/// contrat le dit, et « 12 sur 40 » laisserait croire que quarante créneaux
/// existent.
fn activites_programmees(
    funnel: Option<&crate::domain::figures::ProposalFunnelRow>,
) -> DashboardKpi {
    let value = funnel.map(|f| f.sessions_programmees as f64);
    DashboardKpi {
        key: DashboardKpiKey::Scheduled,
        value,
        out_of: None,
        delta: None,
        at: None,
        spark: Vec::new(),
        tone: DashboardKpiTone::Neutral,
    }
}

/// **Huit parts au plus, la queue regroupée si elle en compte AU MOINS DEUX.**
///
/// Regrouper une seule part sous « autres » lui retirerait son nom sans rien
/// simplifier — la part serait aussi grosse, et on ne saurait plus de qui il
/// s'agit.
fn repartir<L, C>(parts: Vec<programme::Part>, libelle: L, couleur: C) -> Vec<BreakdownSlice>
where
    L: Fn(&str) -> Option<serde_json::Value>,
    C: Fn(&str) -> Option<String>,
{
    let total: i64 = parts.iter().map(|p| p.compte).sum();
    if total == 0 {
        return Vec::new();
    }

    let queue = parts.len().saturating_sub(PARTS);
    let (tete, reste) = if queue >= 2 {
        parts.split_at(PARTS)
    } else {
        (parts.as_slice(), &[] as &[programme::Part])
    };

    let mut tranches: Vec<BreakdownSlice> = tete
        .iter()
        .map(|p| BreakdownSlice {
            key: p.cle.clone(),
            label: libelle(&p.cle)
                .unwrap_or_else(|| serde_json::json!({ "fr": p.cle, "en": p.cle })),
            color: couleur(&p.cle),
            count: p.compte,
            // **Calculée à la source** : recalculée à l'écran, elle finirait par
            // ne plus coïncider avec la somme affichée à côté.
            share: p.compte as f64 / total as f64,
        })
        .collect();

    if !reste.is_empty() {
        let compte: i64 = reste.iter().map(|p| p.compte).sum();
        tranches.push(BreakdownSlice {
            key: "__other__".to_owned(),
            label: serde_json::json!({ "fr": "Autres", "en": "Other" }),
            color: None,
            count: compte,
            share: compte as f64 / total as f64,
        });
    }

    tranches
}
