//! Règle métier n° 8, tenue par l'API et non plus seulement par l'écran.
//!
//! Un administrateur détaché sur une édition ne doit rien atteindre d'une
//! autre, **y compris en forgeant un identifiant dans l'URL**. C'est la
//! deuxième obligation du principe X : chaque route paramétrée est éprouvée,
//! pas seulement celle qui a servi d'exemple.

mod commun;

use commun::{attribuer, semer, semer_evenement, Bac, Compte};
use identity::domain::ids::PersonId;
use identity::domain::permissions::PERSON_READ;
use identity::service::{admin_users, rbac};
use kernel::error::ErrorCode;

/// Deux éditions, deux administrateurs détachés, une personne rattachée à
/// chacune. Le décor minimal où « une autre édition » veut dire quelque chose.
struct Decor {
    bac: Bac,
    admin_a: uuid::Uuid,
    admin_b: uuid::Uuid,
}

async fn planter() -> Decor {
    let bac = Bac::monter().await;

    let cop_a = semer_evenement(&bac, "cop31-belem", "COP31 Belém").await;
    let cop_b = semer_evenement(&bac, "cop16-riyad", "COP16 Riyad").await;

    let admin_a = semer(&bac, Compte::actif("admin.a@example.org")).await;
    let admin_b = semer(&bac, Compte::actif("admin.b@example.org")).await;

    attribuer(&bac, admin_a, "admin", "event", Some(cop_a)).await;
    attribuer(&bac, admin_b, "admin", "event", Some(cop_b)).await;

    Decor {
        bac,
        admin_a,
        admin_b,
    }
}

#[tokio::test]
async fn la_liste_ne_montre_que_le_perimetre_confie() {
    let decor = planter().await;
    let perimetre = commun::perimetre(&decor.bac, decor.admin_a)
        .await
        .expect("périmètre de l'administrateur A");

    let ecran = admin_users::list_screen(decor.bac.base.pool(), "fr", &perimetre)
        .await
        .expect("liste des utilisateurs");

    let vus: Vec<_> = ecran.rows.iter().map(|r| r.person_id.as_uuid()).collect();
    assert!(
        vus.contains(&decor.admin_a),
        "on se voit dans sa propre liste"
    );
    assert!(
        !vus.contains(&decor.admin_b),
        "l'administrateur d'une autre édition n'a rien à faire dans cette liste"
    );
    assert!(
        ecran.scoped_to_events,
        "la liste doit dire qu'elle a été restreinte"
    );
}

/// Le contrat ne cache pas la fiche hors périmètre : il la marque. La taire
/// ferait croire à la disparition de quelqu'un dont on sait qu'il existe ; la
/// rendre modifiable ouvrirait l'édition voisine.
#[tokio::test]
async fn une_fiche_hors_perimetre_sort_en_lecture_seule() {
    let decor = planter().await;
    let perimetre = commun::perimetre(&decor.bac, decor.admin_a)
        .await
        .expect("périmètre de l'administrateur A");

    let sienne = admin_users::detail(
        decor.bac.base.pool(),
        "fr",
        &perimetre,
        PersonId(decor.admin_a),
    )
    .await
    .expect("fiche de soi")
    .expect("la fiche existe");
    assert!(sienne.in_scope);

    let autre = admin_users::detail(
        decor.bac.base.pool(),
        "fr",
        &perimetre,
        PersonId(decor.admin_b),
    )
    .await
    .expect("fiche forgée")
    .expect("la fiche existe");
    assert!(
        !autre.in_scope,
        "une fiche d'une autre édition ne doit jamais sortir dans le périmètre"
    );
}

/// « Soi-même » se décide par la session. Sans la permission de lire les
/// personnes, l'identifiant d'un autre ne mène nulle part — quelle que soit la
/// route paramétrée qui le porte.
#[tokio::test]
async fn sans_permission_lidentifiant_dun_autre_est_refuse() {
    let decor = planter().await;
    let quidam = semer(&decor.bac, Compte::actif("quidam@example.org")).await;

    let refus =
        kernel::auth::require_permission_anywhere(decor.bac.base.pool(), quidam, PERSON_READ)
            .await
            .expect_err("un compte sans rôle ne lit pas les personnes");
    assert_eq!(refus.code, ErrorCode::Forbidden);

    // Et sur soi-même, la même lecture passe : c'est la session qui l'ouvre,
    // jamais un paramètre.
    let sien = rbac::administered_events(decor.bac.base.pool(), PersonId(quidam))
        .await
        .expect("son propre périmètre");
    assert!(!sien.is_global);
    assert!(sien.event_ids.is_empty());
}

/// Les permissions effectives d'un administrateur détaché ne portent que sa
/// propre édition : c'est ce qui borne, plus tard, ce qu'il pourra écrire.
#[tokio::test]
async fn les_permissions_effectives_portent_leur_portee() {
    let decor = planter().await;

    let effectives = rbac::effective_permissions(decor.bac.base.pool(), PersonId(decor.admin_a))
        .await
        .expect("permissions effectives");

    assert!(
        !effectives.is_empty(),
        "un administrateur d'édition détient des permissions"
    );
    assert!(
        effectives
            .iter()
            .all(|p| p.scope_type == kernel::auth::ScopeType::Event),
        "aucune permission d'un administrateur détaché ne vaut globalement"
    );

    let perimetre = commun::perimetre(&decor.bac, decor.admin_a)
        .await
        .expect("périmètre");
    let vue =
        rbac::effective_permissions_view(decor.bac.base.pool(), "fr", PersonId(decor.admin_a))
            .await
            .expect("écran d'explication");

    assert_eq!(vue.administered.event_ids, perimetre.event_ids);
    assert!(
        vue.groups
            .iter()
            .flat_map(|g| &g.rows)
            .all(|r| !r.is_global),
        "l'écran d'explication ne doit annoncer aucune permission globale"
    );
    assert!(
        vue.total > 0 && !vue.missing.is_empty(),
        "l'écran dit ce que la personne peut ET ce qu'elle ne peut pas"
    );
}
