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
//!
//! # LES DOCUMENTS
//!
//! Même piège, autre permission : le rôle `admin` porte
//! `negotiation.document.publish`, et un document peut être rattaché à une COP.
//! Les lectures du back-office s'ouvrent à qui publie **ou** corrige, sur la
//! portée globale ; `exiger_la_lecture` refuse les autres, par un 403.
//!
//! Chaque refus a son témoin : la même permission, tenue sur la portée globale,
//! passe. Sans lui, un code de permission mal orthographié refuserait aussi.

mod commun;

use commun::documents::{administratrice, expert};
use commun::{administre_guide_nego, attribuer, decor, espace, personne, Bac};
use kernel::auth::{has_permission, require_permission, Scope};
use kernel::error::ErrorCode;
use negotiation::domain::permissions::{
    CORRECTION_POST, CORRECTION_WITHDRAW, DOCUMENT_PUBLISH, SPACE_MANAGE,
};
use negotiation::service::admin_documents::{self, Droits};

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

// ---------------------------------------------------------------------------
// Les documents et les notes
// ---------------------------------------------------------------------------

async fn une_edition(bac: &Bac, slug: &str) -> uuid::Uuid {
    sqlx::query_scalar(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2027, jsonb_build_object('fr', 'COP31'), $1::text::platform.slug,
                   jsonb_build_object('fr', 'COP31'), 'online',
                   'America/Belem'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#,
    )
    .bind(slug)
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition")
}

async fn peut(bac: &Bac, qui: uuid::Uuid, permission: &str) -> bool {
    has_permission(bac.pool(), qui, permission, Scope::Global)
        .await
        .expect("lecture de la permission")
}

/// `Droits` ne se débogue pas : `expect_err` n'est pas disponible.
fn refus(issue: kernel::error::Result<Droits>) -> ErrorCode {
    match issue {
        Ok(d) => panic!(
            "lecture ouverte (publier : {}, corriger : {})",
            d.publier, d.corriger
        ),
        Err(e) => e.code,
    }
}

#[tokio::test]
async fn un_administrateur_devenement_ne_publie_ni_ne_corrige_les_documents() {
    let bac = Bac::monter().await;
    let admin_cop = personne(&bac, "admin.cop31@example.org").await;
    let edition = une_edition(&bac, "cop31-documents-test").await;
    attribuer(&bac, admin_cop, "admin", "event", Some(edition)).await;

    assert!(
        has_permission(
            bac.pool(),
            admin_cop,
            DOCUMENT_PUBLISH,
            Scope::Event(edition)
        )
        .await
        .expect("lecture de la permission"),
        "le rôle `admin` publie SUR SON ÉVÉNEMENT — c'est le piège"
    );

    // Témoins : chaque permission qu'il n'a pas se tient sur la portée globale.
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let relectrice = expert(&bac, "experte@example.org").await;
    for (permission, detenteur) in [
        (DOCUMENT_PUBLISH, ifdd),
        (CORRECTION_POST, relectrice),
        (CORRECTION_WITHDRAW, relectrice),
    ] {
        assert!(
            !peut(&bac, admin_cop, permission).await,
            "{permission} sur la portée globale"
        );
        assert!(
            peut(&bac, detenteur, permission).await,
            "{permission} existe et se détient sur la portée globale"
        );
    }

    let droits = admin_documents::droits(&bac.state, admin_cop)
        .await
        .expect("lecture des droits");
    assert!(!droits.publier && !droits.corriger);
    let (publie, corrige) = (
        admin_documents::droits(&bac.state, ifdd)
            .await
            .expect("lecture des droits"),
        admin_documents::droits(&bac.state, relectrice)
            .await
            .expect("lecture des droits"),
    );
    assert!(
        publie.publier && corrige.corriger,
        "les droits suivent la portée globale"
    );

    assert_eq!(
        refus(admin_documents::exiger_la_lecture(&bac.state, admin_cop).await),
        ErrorCode::Forbidden,
        "un refus, jamais une liste vide"
    );
}

#[tokio::test]
async fn sans_permission_les_documents_se_refusent_explicitement() {
    let bac = Bac::monter().await;
    let quidam = personne(&bac, "quidam@example.org").await;

    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let relectrice = expert(&bac, "experte@example.org").await;
    for (permission, detenteur) in [
        (DOCUMENT_PUBLISH, ifdd),
        (CORRECTION_POST, relectrice),
        (CORRECTION_WITHDRAW, relectrice),
    ] {
        let refus = require_permission(bac.pool(), quidam, permission, Scope::Global)
            .await
            .expect_err("une personne sans droit ne passe pas la garde");
        assert_eq!(refus.code, ErrorCode::Forbidden, "{permission}");
        require_permission(bac.pool(), detenteur, permission, Scope::Global)
            .await
            .unwrap_or_else(|e| panic!("{permission} : son détenteur passe ({:?})", e.code));
    }

    assert_eq!(
        refus(admin_documents::exiger_la_lecture(&bac.state, quidam).await),
        ErrorCode::Forbidden
    );
}

#[tokio::test]
async fn les_documents_souvrent_a_qui_publie_ou_corrige_sur_la_portee_globale() {
    let bac = Bac::monter().await;
    let ifdd = administratrice(&bac, "ifdd@example.org").await;
    let relectrice = expert(&bac, "experte@example.org").await;

    let publie = admin_documents::exiger_la_lecture(&bac.state, ifdd)
        .await
        .expect("l'administratrice de la plateforme lit");
    assert!(
        publie.publier && !publie.corriger,
        "elle publie, et ne corrige pas"
    );

    let corrige = admin_documents::exiger_la_lecture(&bac.state, relectrice)
        .await
        .expect("l'experte lit ce qu'elle corrige");
    assert!(
        corrige.corriger && !corrige.publier,
        "elle corrige, et ne publie pas"
    );

    // Retirer une note n'entre pas dans `Droits` : l'experte le tient sur la
    // portée globale, l'administratrice non.
    for (qui, attendus) in [
        (relectrice, [false, true, true]),
        (ifdd, [true, false, false]),
    ] {
        for (permission, attendu) in [DOCUMENT_PUBLISH, CORRECTION_POST, CORRECTION_WITHDRAW]
            .into_iter()
            .zip(attendus)
        {
            assert_eq!(
                peut(&bac, qui, permission).await,
                attendu,
                "{qui} : {permission}"
            );
        }
    }
}
