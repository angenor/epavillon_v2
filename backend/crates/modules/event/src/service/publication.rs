//! **Le seul contrôle bloquant du module.**
//!
//! Partout ailleurs, le système détecte et signale sans jamais refuser : les
//! chevauchements de créneaux s'écrivent librement, l'équipe arbitre (règle
//! métier n° 2). Ici, et ici seulement, un point de gravité bloquante retient
//! l'écriture — parce qu'une programmation rendue publique avec deux activités
//! dans la même salle au même moment n'est pas un brouillon de travail, c'est
//! une information fausse donnée au public.
//!
//! La publication fait trois choses, dans une seule transaction (research.md
//! § R10) :
//!
//! 1. **contrôler** — `programme.publication_readiness()`, appelée et jamais
//!    réécrite ;
//! 2. **estampiller** — `WHERE programme_published_at IS NULL`, ce qui rend la
//!    republication inoffensive ;
//! 3. **annoncer** — `event.programme.published`, avec **le prédicat exact** des
//!    séances à publier.
//!
//! ## Pourquoi l'annonce et non l'écriture
//!
//! La vue de la programmation publique filtre sur `published_at` **de chaque
//! séance**, pas sur la date de l'édition. Rendre le programme public exige donc
//! deux écritures, dans deux schémas. Écrire dans `programme` depuis ici
//! romprait la frontière ; tout confier à B5 la romprait dans l'autre sens, B5
//! écrivant alors dans `event.events`. L'outbox est la troisième voie, et c'est
//! exactement ce que le principe IV décrit.

use kernel::context::RequestContext;
use kernel::error::{ApiError, Result};
use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::domain::ids::EventId;
use crate::repo::cross::{self, PointDeControle};
use crate::repo::editions;
use crate::state::EventState;

/// La réponse de la publication — `PublishProgrammeResult`.
#[derive(Debug, Clone, Serialize)]
pub struct PublishProgrammeResult {
    /// Vrai dès qu'un point de gravité **bloquante** subsiste : rien n'est
    /// écrit, et `issues` dit quoi régler.
    pub blocked: bool,
    /// Séances rendues publiques. **Zéro quand `blocked` vaut vrai.**
    pub published_count: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub published_at: Option<OffsetDateTime>,
    pub issues: Vec<PointDeControle>,
}

/// **Le contrôle préalable, en lecture seule** — consultable avant toute
/// tentative.
///
/// C'est ce qui distingue un refus utile d'un refus sec : l'équipe voit ce qui
/// bloque avant d'essayer, plutôt que de découvrir la liste après un clic.
pub async fn controle(pool: &PgPool, event_id: EventId) -> Result<Vec<PointDeControle>> {
    cross::controle_de_publication(pool, event_id).await
}

/// Publier la programmation d'une édition.
pub async fn publier(
    state: &EventState,
    ctx: &RequestContext,
    event_id: EventId,
) -> Result<PublishProgrammeResult> {
    let mut tx = state.db().write(ctx).await?;

    let issues = cross::controle_de_publication(&mut *tx, event_id).await?;

    // **Les avertissements ne retiennent pas.** Une séance sans intervenant
    // déclaré mérite d'être signalée ; interdire tout un programme pour cela
    // ferait de l'avertissement un refus, et l'équipe cesserait de le lire.
    if issues.iter().any(PointDeControle::est_bloquant) {
        // Rien n'est écrit : la transaction s'annule en sortant de portée.
        return Ok(PublishProgrammeResult {
            blocked: true,
            published_count: 0,
            published_at: None,
            issues,
        });
    }

    // **Compté sous l'instantané de la transaction**, avec le prédicat même que
    // l'annonce va porter : le chiffre annoncé et l'effet obtenu viennent du
    // même raisonnement.
    let published_count = cross::seances_a_publier(&mut *tx, event_id).await?;

    // **`WHERE programme_published_at IS NULL`** : republier n'écrase pas la
    // date d'origine, et n'annonce rien de plus.
    let Some(published_at) = editions::estampiller_la_publication(&mut tx, event_id).await? else {
        // Déjà publiée. On rend la date d'origine, intacte, et **aucun second
        // événement n'est émis** : le consommateur de B5 n'a rien à rejouer.
        tx.rollback().await?;
        let published_at = editions::date_de_publication(state.pool(), event_id).await?;

        return Ok(PublishProgrammeResult {
            blocked: false,
            published_count: 0,
            published_at,
            issues,
        });
    };

    annoncer(&mut tx, event_id, published_at, published_count).await?;

    tx.commit().await?;

    Ok(PublishProgrammeResult {
        blocked: false,
        published_count,
        published_at: Some(published_at),
        issues,
    })
}

/// L'annonce, **dans la même transaction que l'estampille**.
///
/// Elle porte le **prédicat exact** des séances à publier, plutôt que de laisser
/// le consommateur le redéduire : un consommateur qui recalculerait « les
/// séances de l'édition » publierait autre chose que ce qui a été annoncé.
async fn annoncer(
    conn: &mut sqlx::postgres::PgConnection,
    event_id: EventId,
    published_at: OffsetDateTime,
    published_count: i64,
) -> Result<()> {
    use contracts::event as contrat;
    use kernel::events::{emit, DomainEvent};

    let charge = serde_json::to_value(contrat::ProgrammePublished {
        event_id: event_id.as_uuid(),
        published_at,
        selection: contrat::SessionSelection {
            event_id: event_id.as_uuid(),
            statuses: cross::STATUTS_A_PUBLIER
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            only_unpublished: true,
        },
        published_count,
    })
    .map_err(ApiError::internal)?;

    emit(
        conn,
        DomainEvent {
            aggregate_schema: contrat::AGGREGATE_SCHEMA,
            aggregate_type: contrat::AGGREGATE_PROGRAMME,
            aggregate_id: event_id.as_uuid(),
            event_type: contrat::PROGRAMME_PUBLISHED,
            payload: charge,
        },
    )
    .await?;

    Ok(())
}
