//! **La régénération n'écrase aucun contenu éditorial** (FR-036, FR-037).
//!
//! Titre, adresse de page, couleur et mise en avant appartiennent à l'équipe.
//! La génération ne connaît que des **dates** et des **rangs** : elle crée ce qui
//! manque, et ne touche à rien d'autre.
//!
//! **Une journée générée porte sa date et rien d'autre.** Inventer « Jour 3 »
//! produirait un titre que personne n'a écrit et qui s'afficherait tel quel sur
//! la page publique.
//!
//! Le test compare **champ à champ**, et non par un décompte : une régénération
//! qui réécrirait le titre en gardant le bon nombre de lignes passerait un test
//! qui se contenterait de compter.

mod commun;

use commun::Bac;
use event::domain::ids::{EventDayId, EventId};
use event::domain::tabs::EditionDayPayload;
use event::service::days as service_journees;
use serde_json::json;

#[tokio::test]
async fn regenerer_ne_touche_ni_titre_ni_adresse_ni_couleur_ni_mise_en_avant() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    // Première génération : les douze journées de la période.
    let premiere = service_journees::generer(&bac.state, &bac.ctx(), cop31, false)
        .await
        .expect("la génération aboutit");
    assert!(premiere.ok);
    assert_eq!(
        commun::journees(&bac, editions.cop31).await.len(),
        commun::seed::JOURS_COP31 as usize
    );

    // On habille la première journée, comme l'équipe le ferait.
    let journee = premiere
        .detail
        .expect("la composition")
        .days
        .into_iter()
        .next()
        .expect("la première journée");

    service_journees::habiller(
        &bac.state,
        &bac.ctx(),
        cop31,
        EventDayId::from(journee.id),
        EditionDayPayload {
            id: None,
            title: Some(json!({ "fr": "Ouverture du pavillon", "en": "Pavilion opening" })),
            slug: Some("ouverture".to_owned()),
            description: Some(json!({ "fr": "Cérémonie d'ouverture." })),
            is_featured: true,
            color_hex: Some("#00A1E4".to_owned()),
        },
    )
    .await
    .expect("l'habillage aboutit");

    let avant = commun::journees_habillees(&bac, editions.cop31).await;

    // Seconde génération, sur la même période : rien à créer, rien à écraser.
    let seconde = service_journees::generer(&bac.state, &bac.ctx(), cop31, false)
        .await
        .expect("la régénération aboutit");
    assert!(seconde.ok);

    let apres = commun::journees_habillees(&bac, editions.cop31).await;
    assert_eq!(
        avant, apres,
        "titre, adresse, couleur et mise en avant comparés champ à champ"
    );
}

/// **Une journée générée porte sa date et rien d'autre.**
#[tokio::test]
async fn une_journee_generee_ne_porte_aucun_titre_invente() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    service_journees::generer(&bac.state, &bac.ctx(), EventId::from(editions.cop31), false)
        .await
        .expect("génération");

    for (date, titre, slug, en_avant, couleur) in
        commun::journees_habillees(&bac, editions.cop31).await
    {
        assert!(titre.is_none(), "aucun titre inventé le {date}");
        assert!(slug.is_none(), "aucune adresse inventée le {date}");
        assert!(!en_avant, "aucune mise en avant décidée à notre place");
        assert!(couleur.is_none(), "aucune couleur choisie à notre place");
    }
}

/// **Élargir la période ne crée que ce qui manque**, et laisse l'habillage
/// existant intact.
#[tokio::test]
async fn elargir_la_periode_najoute_que_les_dates_absentes() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    service_journees::generer(&bac.state, &bac.ctx(), cop31, false)
        .await
        .expect("première génération");

    // La période gagne deux jours à la fin.
    sqlx::query!(
        "UPDATE event.events SET ends_at = ends_at + interval '2 days' WHERE id = $1",
        editions.cop31
    )
    .execute(bac.pool())
    .await
    .expect("élargissement");

    let plan = service_journees::plan(bac.pool(), cop31)
        .await
        .expect("plan")
        .expect("édition");
    assert_eq!(plan.to_create.len(), 2, "deux journées manquent");
    assert_eq!(plan.unchanged, commun::seed::JOURS_COP31 as usize);

    service_journees::generer(&bac.state, &bac.ctx(), cop31, false)
        .await
        .expect("seconde génération");

    assert_eq!(
        commun::journees(&bac, editions.cop31).await.len(),
        commun::seed::JOURS_COP31 as usize + 2
    );
}
