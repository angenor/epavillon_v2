//! **Un administrateur d'événement ne voit rien du back-office de Guide Négo**
//! — FR-044, SC-008, principe V.
//!
//! Les **douze** routes du back-office — sept sur les codes, trois sur les
//! demandes, deux sur le mode d'admission — exigent
//! `negotiation.space.manage` **sur la portée globale**, et rien d'autre. Un
//! identifiant forgé se refuse donc **avant toute lecture** : la garde tombe
//! sans que la route ait touché la base.
//!
//! # POURQUOI CE FICHIER LIT AUSSI LE CODE SOURCE
//!
//! Le piège que SC-008 nomme n'est pas qu'une route soit ouverte : c'est
//! qu'elle *paraisse* gardée. Le rôle `admin` porte
//! `negotiation.space.manage` et s'attribue **aussi sur un événement** ;
//! `RequiresAnyScope<SpaceManage>` compilerait, se lirait comme une garde, et
//! laisserait entrer l'administrateur d'une seule édition — alors qu'aucun
//! espace de négociation n'est rattaché à une édition.
//!
//! Monter la vraie application pour l'éprouver en HTTP demanderait à ce crate
//! une dépendance de développement vers `api`, que `cargo tree` refuse
//! (principe II, voir `frontieres.rs`). Le contrôle porte donc sur deux choses
//! qui, ensemble, ferment le cas : **ce que la base répond** à un administrateur
//! d'événement, et **quel extracteur les douze routes déclarent**.

mod commun;

use commun::{administre_guide_nego, attribuer, decor, Bac};
use kernel::auth::Scope;
use negotiation::domain::permissions::SPACE_MANAGE;

const ADMIN_CODES: &str = include_str!("../src/routes/admin_codes.rs");
const ADMIN_DEMANDES: &str = include_str!("../src/routes/admin_requests.rs");
const ADMIN_ADMISSION: &str = include_str!("../src/routes/admin_admission.rs");

/// Les douze routes, telles que `contracts/api-admin.md` les énumère.
const ROUTES: [&str; 12] = [
    "/admin/negotiation/invitation-codes",
    "/admin/negotiation/invitation-codes",
    "/admin/negotiation/invitation-codes/{id}",
    "/admin/negotiation/invitation-codes/{id}/revoke",
    "/admin/negotiation/invitation-codes/{id}/uses",
    "/admin/negotiation/invitation-codes/{id}/uses/{person_id}/revoke-access",
    "/admin/negotiation/invitation-codes/{id}/revoke-all-access",
    "/admin/negotiation/access-requests",
    "/admin/negotiation/access-requests/{id}/approve",
    "/admin/negotiation/access-requests/{id}/reject",
    "/admin/negotiation/admission",
    "/admin/negotiation/admission",
];

fn sources() -> [&'static str; 3] {
    [ADMIN_CODES, ADMIN_DEMANDES, ADMIN_ADMISSION]
}

/// Le code seul, commentaires retirés.
///
/// Les fichiers de ce module **expliquent le piège dans leur en-tête**, et y
/// nomment donc `RequiresAnyScope` et `Perimeter` pour dire de ne pas s'en
/// servir. Compter sur le texte brut ferait échouer le contrôle sur
/// l'explication qui le justifie.
fn sans_commentaires(source: &str) -> String {
    source
        .lines()
        .filter(|ligne| !ligne.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn les_douze_routes_sont_declarees() {
    let tout: String = sources().map(sans_commentaires).concat();
    for route in ROUTES {
        assert!(
            tout.contains(&format!("\"{route}\"")),
            "la route {route} n'est montée nulle part"
        );
    }

    // Douze `web::` dans les trois `configurer()` : ni plus — une treizième
    // route serait une porte que personne n'a éprouvée —, ni moins.
    let montees: usize = sources()
        .map(|source| sans_commentaires(source).matches(".route(").count())
        .iter()
        .sum();
    assert_eq!(montees, 12, "douze routes, pas une de plus");
}

#[test]
fn chaque_gestionnaire_exige_la_portee_globale() {
    for source in sources().map(sans_commentaires) {
        let gestionnaires = source.matches("pub(crate) async fn ").count();
        let gardes = source.matches("Requires<SpaceManage>").count();
        assert_eq!(
            gestionnaires, gardes,
            "chaque gestionnaire déclare sa garde, et c'est la même pour tous"
        );
    }
}

#[test]
fn aucune_route_nemprunte_une_garde_plus_large() {
    for source in sources().map(sans_commentaires) {
        // **`RequiresAnyScope` est le piège**, et il compilerait : le rôle
        // `admin` porte la permission sur un événement.
        assert!(
            !source.contains("RequiresAnyScope"),
            "« n'importe quelle portée » ouvrirait le back-office à un administrateur d'édition"
        );
        // `Perimeter` ne conviendrait pas davantage :
        // `identity.administered_events()` ne rend que des portées `event`,
        // quand un code porte `global` ou `negotiation_space`.
        assert!(
            !source.contains("Perimeter"),
            "le périmètre d'édition n'a rien à dire d'un espace de négociation"
        );
    }
}

#[tokio::test]
async fn ladministrateur_dune_edition_ne_passe_aucune_garde() {
    let bac = Bac::monter().await;
    let d = decor(&bac).await;

    let edition = sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (edition_year, title, slug, description, participation_mode,
                timezone, starts_at, ends_at)
           VALUES (2027, jsonb_build_object('fr', 'COP31'),
                   'cop31-url-forgee'::text::platform.slug,
                   jsonb_build_object('fr', 'COP31'), 'online',
                   'America/Belem'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '40 days')
           RETURNING id"#
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition");

    attribuer(&bac, d.admin_id, "admin", "event", Some(edition)).await;

    // La permission existe **sur son édition** : c'est ce qui rend le piège
    // crédible.
    assert!(kernel::auth::has_permission(
        bac.pool(),
        d.admin_id,
        SPACE_MANAGE,
        Scope::Event(edition)
    )
    .await
    .expect("lecture de la permission"));

    // Et elle n'existe pas sur la portée globale, la seule que la garde teste.
    assert!(!administre_guide_nego(&bac, d.admin_id).await);

    // Pas davantage sur l'espace de négociation dont il forgerait l'adresse.
    assert!(!kernel::auth::has_permission(
        bac.pool(),
        d.admin_id,
        SPACE_MANAGE,
        Scope::NegotiationSpace(d.space_id)
    )
    .await
    .expect("lecture de la permission"));
}
