//! **L'URL forgée, sur chacune des routes paramétrées.**
//!
//! Le principe est celui de research.md § R2, et il ne se négocie pas :
//! **résoudre l'ascendance, puis vérifier le périmètre, puis agir**. Vérifier
//! d'abord reviendrait à croire l'édition que le client annonce ; agir d'abord
//! laisserait un effet de bord — un déclencheur d'audit — sur une édition qu'on
//! n'a pas le droit de toucher.
//!
//! **Ce fichier éprouve le garde, pas les gestionnaires.** Six routes du
//! back-office sont paramétrées par l'identifiant d'un **enfant** de l'édition,
//! et elles arrivent au fil des phases 5 à 9 ; toutes passent par
//! `edition_dans_le_perimetre`, l'unique porte, et c'est elle qui est éprouvée
//! ici sur ses six cibles. Un gestionnaire qui ne l'appellerait pas serait un
//! défaut de revue, pas un trou de ce test.
//!
//! Les identifiants forgés visent la **COP31**, hors du périmètre du compte
//! détaché — c'est ce qui en fait la cible naturelle.

mod commun;

use commun::{perimetres, seed, Bac};
use event::domain::ids::{CallId, ChannelId, EventDayId, EventId, RoomId, TrackId, VenueId};
use event::service::{canal_dans_le_perimetre, edition_dans_le_perimetre, CanalCible, Cible};
use kernel::error::ErrorCode;
use uuid::Uuid;

/// **Les six cibles, et l'édition elle-même** : sept refus, tous de la même
/// forme. La boucle nomme la cible fautive — un `assert` muet obligerait à
/// relancer six fois pour savoir laquelle est passée.
#[tokio::test]
async fn les_six_ascendances_refusent_et_le_refus_est_celui_dun_introuvable() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    let p = perimetres(&bac, &editions).await;

    let perimetre = commun::perimetre_de(&bac, p.detache).await;

    let cibles = [
        ("l'édition", Cible::Edition(EventId::from(editions.cop31))),
        ("le fil", Cible::Fil(TrackId::from(enfants.fil))),
        ("le lieu", Cible::Lieu(VenueId::from(enfants.lieu))),
        ("la salle", Cible::Salle(RoomId::from(enfants.salle))),
        ("l'appel", Cible::Appel(CallId::from(enfants.appel))),
        (
            "la journée",
            Cible::Journee(EventDayId::from(enfants.journee)),
        ),
    ];

    for (quoi, cible) in cibles {
        let refus = edition_dans_le_perimetre(bac.pool(), &perimetre, cible)
            .await
            .err()
            .unwrap_or_else(|| panic!("{quoi} de la COP31 aurait dû être refusé"));

        assert_eq!(
            refus.code,
            ErrorCode::NotFound,
            "{quoi} : le refus de périmètre doit être celui d'un identifiant inexistant"
        );
    }
}

/// **Un identifiant inexistant et un identifiant hors périmètre sont
/// indiscernables** (principe IX). Sans cela, une URL forgée dirait à qui la
/// forge si l'objet existe.
#[tokio::test]
async fn un_identifiant_inexistant_rend_le_meme_refus() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    let p = perimetres(&bac, &editions).await;

    let perimetre = commun::perimetre_de(&bac, p.detache).await;
    let inconnu = Uuid::now_v7();

    let hors_perimetre = edition_dans_le_perimetre(
        bac.pool(),
        &perimetre,
        Cible::Fil(TrackId::from(enfants.fil)),
    )
    .await
    .expect_err("hors périmètre");

    let inexistant =
        edition_dans_le_perimetre(bac.pool(), &perimetre, Cible::Fil(TrackId::from(inconnu)))
            .await
            .expect_err("inexistant");

    assert_eq!(hors_perimetre.code, inexistant.code);
    assert_eq!(hors_perimetre.message, inexistant.message);
}

/// Le compte détaché **passe** sur sa propre édition : un garde qui refuserait
/// tout serait vert au test précédent sans rien protéger.
#[tokio::test]
async fn le_compte_detache_passe_sur_son_edition() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let siennes = seed::enfants(&bac, editions.sans_pavillon).await;
    let p = perimetres(&bac, &editions).await;

    let perimetre = commun::perimetre_de(&bac, p.detache).await;

    let cibles = [
        Cible::Edition(EventId::from(editions.sans_pavillon)),
        Cible::Fil(TrackId::from(siennes.fil)),
        Cible::Lieu(VenueId::from(siennes.lieu)),
        Cible::Salle(RoomId::from(siennes.salle)),
        Cible::Appel(CallId::from(siennes.appel)),
        Cible::Journee(EventDayId::from(siennes.journee)),
    ];

    for cible in cibles {
        let edition = edition_dans_le_perimetre(bac.pool(), &perimetre, cible)
            .await
            .expect("son édition passe");
        assert_eq!(edition.as_uuid(), editions.sans_pavillon);
    }
}

/// **Le canal a une issue de plus que les six autres.** Un canal général de la
/// plateforme n'appartient à aucune édition : il n'est ni introuvable ni hors
/// périmètre, et son refus est celui du contrat, pas celui du garde.
#[tokio::test]
async fn le_canal_general_nest_ni_introuvable_ni_hors_perimetre() {
    let bac = Bac::monter().await;
    let editions = seed::editions(&bac).await;
    let enfants = seed::enfants(&bac, editions.cop31).await;
    let p = perimetres(&bac, &editions).await;

    let perimetre = commun::perimetre_de(&bac, p.detache).await;

    let hors_perimetre =
        canal_dans_le_perimetre(bac.pool(), &perimetre, ChannelId::from(enfants.canal))
            .await
            .expect_err("le canal de la COP31 est hors périmètre");
    assert_eq!(hors_perimetre.code, ErrorCode::NotFound);

    // Celui du semis : `ifdd_principal`, sans édition.
    let general = sqlx::query_scalar!(
        "SELECT id FROM event.broadcast_channels WHERE event_id IS NULL AND code = 'ifdd_principal'"
    )
    .fetch_one(bac.pool())
    .await
    .expect("le canal général est semé par 900_seed.sql");

    let cible = canal_dans_le_perimetre(bac.pool(), &perimetre, ChannelId::from(general))
        .await
        .expect("un canal général se résout, il ne se refuse pas ici");

    assert!(matches!(cible, CanalCible::Plateforme));
}
