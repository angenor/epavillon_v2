//! **Le périmètre d'administration, sur les séances** — règle métier n° 8.
//!
//! Toute liste du back-office est filtrée par le périmètre, **y compris quand
//! l'utilisateur forge une URL**. Et le refus ne dit jamais si l'objet existe :
//! un identifiant inexistant et un identifiant hors périmètre sont
//! indiscernables par la forme de la réponse.

mod commun;

use commun::seances::{self, Souhaits};
use commun::Bac;
use kernel::auth::Perimeter;
use kernel::error::ErrorCode;
use programme::domain::ids::{CommentId, EventId, ProposalId, RegistrationId, ReviewId, SessionId};
use programme::domain::transitions::ProposalStatus;
use programme::service::perimeter::{self, Cible};
use programme::service::transition;
use uuid::Uuid;

/// Les trois cas du périmètre restent distincts — **et le troisième est un refus
/// explicite**, jamais une grille vide : les confondre afficherait « rien à
/// arbitrer » à qui n'a aucun droit.
#[tokio::test]
async fn les_trois_cas_du_perimetre_restent_distincts() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;
    let autre = commun::edition_secondaire(&bac).await;

    // 1. Portée globale : les deux éditions.
    let globale = commun::personne(&bac, "globale@ifdd.org", "Gaëlle", "Ndiaye").await;
    commun::attribuer(&bac, globale, "admin", "global", None).await;
    let perimetre = commun::perimetre_de(&bac, globale).await;
    for edition in [terrain.edition, autre] {
        perimeter::edition_dans_le_perimetre(
            bac.pool(),
            &perimetre,
            Cible::Edition(EventId(edition)),
        )
        .await
        .expect("une portée globale ouvre toutes les éditions");
    }

    // 2. Portée ciblée : une seule.
    let detachee = commun::personne(&bac, "detachee@ifdd.org", "Diane", "Kouadio").await;
    commun::attribuer(&bac, detachee, "admin", "event", Some(terrain.edition)).await;
    let perimetre = commun::perimetre_de(&bac, detachee).await;
    perimeter::edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Edition(EventId(terrain.edition)),
    )
    .await
    .expect("son édition s'ouvre");

    let refus = perimeter::edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Edition(EventId(autre)),
    )
    .await
    .expect_err("l'autre édition, jamais");
    assert_eq!(refus.code, ErrorCode::NotFound);

    // 3. Aucun droit : **un refus explicite**, et non une liste vide.
    let sans_droit = commun::personne(&bac, "sans@example.org", "Sam", "Traoré").await;
    let refus = kernel::auth::require_perimeter(bac.pool(), sans_droit)
        .await
        .expect_err("un périmètre vide se refuse");
    assert_eq!(refus.code, ErrorCode::Forbidden);
}

/// **Six identifiants forgés — dont quatre désignent des objets bien réels
/// d'une autre édition — mènent tous au même refus qu'un identifiant
/// inexistant.**
#[tokio::test]
async fn six_identifiants_forges_menent_au_meme_refus() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    seances::grille(&bac, terrain.edition).await;

    // Une seconde édition **complète**, avec son dossier retenu, sa séance et
    // son inscription : ce sont les objets qu'une URL forgée viserait.
    let autre = commun::edition_secondaire(&bac).await;
    let terrain_voisin = commun::Terrain {
        edition: autre,
        appel: commun::appel_ouvert(&bac, autre).await,
        organisation: commun::organisation_verifiee(&bac, "Voisine", "VSN").await,
        deposante: commun::personne(&bac, "voisine@example.org", "Vera", "Sow").await,
    };
    commun::adherer(
        &bac,
        terrain_voisin.organisation,
        terrain_voisin.deposante,
        "active",
    )
    .await;
    seances::grille(&bac, autre).await;

    let dossier_voisin = seances::dossier_pret(
        &bac,
        &terrain_voisin,
        "Chez le voisin",
        "chez-le-voisin",
        Souhaits {
            creneau: Some("2027-03-02 10:00"),
            ..Souhaits::default()
        },
    )
    .await;
    transition::tenter(
        &bac.state,
        &bac.ctx(),
        dossier_voisin.id.into(),
        ProposalStatus::Accepted,
        None,
    )
    .await
    .unwrap();
    let seance_voisine = seances::seances_du_dossier(&bac, dossier_voisin.id)
        .await
        .remove(0)
        .id;

    let inscrit = commun::personne(&bac, "inscrit@example.org", "Ina", "Diop").await;
    let inscription = sqlx::query_scalar!(
        "INSERT INTO programme.registrations (session_id, person_id)
         VALUES ($1, $2) RETURNING id",
        seance_voisine,
        inscrit
    )
    .fetch_one(bac.pool())
    .await
    .expect("inscription chez le voisin");

    // Une personne qui n'administre QUE la COP31.
    let detachee = commun::personne(&bac, "detachee@ifdd.org", "Diane", "Kouadio").await;
    commun::attribuer(&bac, detachee, "admin", "event", Some(terrain.edition)).await;
    let perimetre: Perimeter = commun::perimetre_de(&bac, detachee).await;

    let inexistant = Uuid::now_v7();

    let cibles = [
        // Deux qui n'existent pas.
        Cible::Edition(EventId(inexistant)),
        Cible::Seance(SessionId(inexistant)),
        // Quatre qui existent, mais ailleurs.
        Cible::Edition(EventId(autre)),
        Cible::Dossier(ProposalId(dossier_voisin.id)),
        Cible::Seance(SessionId(seance_voisine)),
        Cible::Inscription(RegistrationId(inscription)),
    ];

    for cible in cibles {
        let refus = perimeter::edition_dans_le_perimetre(bac.pool(), &perimetre, cible)
            .await
            .expect_err("hors périmètre");
        assert_eq!(
            refus.code,
            ErrorCode::NotFound,
            "le même refus, quelle que soit la cible : une URL forgée ne dit rien"
        );
    }

    // Et les deux cibles héritées de B4 se comportent pareil.
    for cible in [
        Cible::Message(CommentId(inexistant)),
        Cible::Revue(ReviewId(inexistant)),
    ] {
        let refus = perimeter::edition_dans_le_perimetre(bac.pool(), &perimetre, cible)
            .await
            .expect_err("inexistant");
        assert_eq!(refus.code, ErrorCode::NotFound);
    }
}
