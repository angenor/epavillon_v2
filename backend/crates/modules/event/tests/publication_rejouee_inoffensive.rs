//! **Republier est inoffensif** (research.md § R10, SC-019).
//!
//! `UPDATE … WHERE programme_published_at IS NULL` : la clause finale rend la
//! republication sans effet. La date d'origine ne s'écrase pas — c'est elle que
//! la frise d'accueil affiche —, et **aucun second événement n'est émis**, ce qui
//! épargne au consommateur de B5 un rejeu qu'il devrait garder.
//!
//! **Le décompte est le contrôle qui dit quelque chose.** Vérifier qu'un
//! événement est présent après deux publications ne prouverait rien : il l'était
//! déjà après la première.

mod commun;

use commun::Bac;
use event::domain::ids::EventId;
use event::service::publication;

#[tokio::test]
async fn republier_ne_deplace_ni_la_date_ni_lannonce() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let premiere = publication::publier(&bac.state, &bac.ctx(), cop31)
        .await
        .expect("première publication");
    assert!(!premiere.blocked);
    let date_dorigine = premiere.published_at.expect("la date est posée");

    let seconde = publication::publier(&bac.state, &bac.ctx(), cop31)
        .await
        .expect("seconde publication");

    assert!(!seconde.blocked, "republier n'est pas un refus");
    assert_eq!(
        seconde.published_at,
        Some(date_dorigine),
        "la date d'origine ne s'écrase pas"
    );
    assert_eq!(
        seconde.published_count, 0,
        "rien de nouveau n'a été désigné"
    );

    assert_eq!(
        commun::evenements_emis(&bac, editions.cop31).await,
        vec!["event.programme.published".to_owned()],
        "aucun second événement : le consommateur n'a rien à rejouer"
    );

    let en_base = sqlx::query_scalar!(
        "SELECT programme_published_at FROM event.events WHERE id = $1",
        editions.cop31
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();
    assert_eq!(en_base, Some(date_dorigine));
}

/// **Une écriture d'édition ne touche jamais à cette date.** Elle est posée par
/// la publication seule ; le formulaire ne la porte même pas.
#[tokio::test]
async fn enregistrer_ledition_apres_publication_ne_touche_pas_la_date() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let acteur = commun::auteur(&bac).await;
    let cop31 = EventId::from(editions.cop31);

    let publiee = publication::publier(&bac.state, &bac.ctx(), cop31)
        .await
        .expect("publication");
    let date_dorigine = publiee.published_at.expect("la date");

    let mut formulaire = commun::formulaire(commun::seed::SLUG_COP31, "COP31 — titre modifié");
    formulaire.acronym = Some("COP31".to_owned());
    formulaire.has_pavilion = true;
    event::service::edition_write::modifier(&bac.state, &bac.ctx(), acteur, cop31, formulaire)
        .await
        .expect("modification de l'édition");

    let en_base = sqlx::query_scalar!(
        "SELECT programme_published_at FROM event.events WHERE id = $1",
        editions.cop31
    )
    .fetch_one(bac.pool())
    .await
    .unwrap();

    assert_eq!(
        en_base,
        Some(date_dorigine),
        "la date de publication n'appartient pas au formulaire"
    );
}
