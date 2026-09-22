//! **Sans la permission sur la portée globale, le back-office refuse** — et il
//! refuse explicitement, jamais par une liste vide (FR-044, SC-008).
//!
//! Une liste vide dirait « il n'y a aucun code » à quelqu'un qui n'a pas le
//! droit de savoir s'il y en a. Le refus est un 403, rendu par l'extracteur
//! `Requires<SpaceManage>` avant toute lecture.
//!
//! # LE PIÈGE QUE CE FICHIER GARDE
//!
//! Le rôle `admin` porte `negotiation.space.manage` **et s'attribue aussi sur
//! un événement**. Une garde « sur n'importe quelle portée » laisserait donc
//! entrer l'administrateur d'une seule édition — alors qu'aucun espace de
//! négociation n'est rattaché à un événement, et qu'il n'a rien à y voir.
//!
//! `identity.administered_events()` n'est pas appelée par ce module, et ne
//! pourrait pas l'être : elle ne rend que des portées `event`, quand un code
//! porte `global` ou `negotiation_space`.

mod commun;

use commun::{administre_guide_nego, attribuer, decor, espace, personne, Bac};
use kernel::auth::{require_permission, Scope};
use kernel::error::ErrorCode;
use negotiation::domain::permissions::SPACE_MANAGE;

#[tokio::test]
async fn un_administrateur_devenement_nadministre_pas_guide_nego() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    // Une édition, et l'administration de cette seule édition : exactement le
    // cas PACO, celui qui a motivé la portée `event` du rôle `admin`.
    let edition = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2027, jsonb_build_object('fr', 'COP31'),
                   'cop31-edition-test'::text::platform.slug,
                   jsonb_build_object('fr', 'COP31'), 'online',
                   'America/Belem'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition");

    attribuer(&bac, d.admin_id, "admin", "event", Some(edition)).await;

    assert!(
        kernel::auth::has_permission(bac.pool(), d.admin_id, SPACE_MANAGE, Scope::Event(edition))
            .await
            .expect("lecture de la permission"),
        "le rôle `admin` porte bien la permission SUR SON ÉVÉNEMENT — c'est le piège"
    );

    assert!(
        !administre_guide_nego(&bac, d.admin_id).await,
        "mais il ne l'a pas sur la portée globale, et c'est elle que la garde exige"
    );
}

#[tokio::test]
async fn aucune_permission_donne_un_refus_explicite() {
    let bac = Bac::monter().await;
    let quidam = personne(&bac, "quidam@example.org").await;

    let refus = require_permission(bac.pool(), quidam, SPACE_MANAGE, Scope::Global)
        .await
        .expect_err("une personne sans droit ne passe pas la garde");

    assert_eq!(
        refus.code,
        ErrorCode::Forbidden,
        "un refus, jamais une liste vide"
    );
}

#[tokio::test]
async fn ladministrateur_global_administre_tous_les_espaces() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;
    let _troisieme = espace(&bac, "cop29-test", "COP29 — test").await;

    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    assert!(administre_guide_nego(&bac, d.admin_id).await);
    // La portée globale couvre tous les espaces, présents et à venir : le
    // back-office de Guide Négo ne connaît qu'un seul périmètre.
    for espace_id in [d.space_id, d.autre_space_id] {
        assert!(kernel::auth::has_permission(
            bac.pool(),
            d.admin_id,
            SPACE_MANAGE,
            Scope::NegotiationSpace(espace_id)
        )
        .await
        .expect("lecture de la permission"));
    }
}
