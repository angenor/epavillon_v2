//! **Une édition sans aucune séance publie** — avec zéro séance et une liste de
//! contrôle vide.
//!
//! Ce n'est pas un conflit, et le dire explicitement évite la tentation
//! symétrique : refuser de publier « puisqu'il n'y a rien à publier ». Une
//! édition annoncée dont la programmation est vide a une page publique, et la
//! marquer publiée est un état légitime — c'est même le premier que l'équipe
//! rencontre en préparant une COP.

mod commun;

use commun::Bac;
use event::domain::ids::EventId;
use event::service::publication;

#[tokio::test]
async fn une_edition_sans_seance_publie_avec_zero_seance() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let points = publication::controle(bac.pool(), cop31)
        .await
        .expect("contrôle");
    assert!(points.is_empty(), "rien à régler : {points:?}");

    let publiee = publication::publier(&bac.state, &bac.ctx(), cop31)
        .await
        .expect("la publication aboutit");

    assert!(
        !publiee.blocked,
        "une programmation vide n'est pas un conflit"
    );
    assert_eq!(publiee.published_count, 0);
    assert!(publiee.published_at.is_some(), "la date est bien posée");
    assert!(publiee.issues.is_empty());

    assert_eq!(
        commun::evenements_emis(&bac, editions.cop31).await,
        vec!["event.programme.published".to_owned()],
        "l'annonce part quand même : c'est l'édition qui devient publiée"
    );
}

/// **Le décompte porte sur le prédicat annoncé, et pas sur autre chose.** Une
/// séance déjà publiée n'y figure pas ; une séance annulée non plus — et le
/// modèle exige d'ailleurs qu'une annulation porte son motif
/// (`ck_sessions_cancelled_reason`), ce que le semis respecte.
#[tokio::test]
async fn le_decompte_suit_exactement_le_predicat_annonce() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    for (slug, statut, deja_publiee) in [
        ("a-publier", "planned", false),
        ("deja-publiee", "scheduled", true),
        ("annulee", "cancelled", false),
    ] {
        sqlx::query!(
            r#"INSERT INTO programme.sessions
                   (event_id, title, slug, format, starts_at, ends_at, timezone,
                    status, location_note, published_at, cancelled_reason)
               VALUES ($1, '{"fr":"Séance"}'::jsonb, $2::text::platform.slug, 'online',
                       ('2027-11-10 14:00')::timestamp AT TIME ZONE $3,
                       ('2027-11-10 15:00')::timestamp AT TIME ZONE $3,
                       $3::text::platform.timezone_name,
                       $4::text::programme.session_status,
                       '{"fr":"En ligne"}'::jsonb,
                       CASE WHEN $5 THEN now() END,
                       CASE WHEN $4 = 'cancelled'
                            THEN '{"fr":"Report à une autre édition."}'::jsonb END)"#,
            editions.cop31,
            slug,
            commun::seed::FUSEAU_COP31,
            statut,
            deja_publiee
        )
        .execute(bac.pool())
        .await
        .expect("insertion de la séance");
    }

    let publiee = publication::publier(&bac.state, &bac.ctx(), cop31)
        .await
        .expect("publication");

    assert_eq!(
        publiee.published_count, 1,
        "seule la séance planifiée et non publiée est désignée"
    );
}
