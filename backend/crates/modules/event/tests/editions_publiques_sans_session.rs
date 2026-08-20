//! **Ce que le public voit, et ce qu'il ne voit pas** (FR-084, écart n° 26).
//!
//! Le critère de publicité est celui du **modèle** : ni brouillon, ni annulée.
//! Il vit dans `event.v_public_editions` et n'est recopié dans aucun écran.
//!
//! Deux cas comptent autant que les exclusions :
//!
//! - une édition **annoncée** dont le programme n'est pas publié **est
//!   publique** — sa page existe, elle annonce ses échéances, et c'est
//!   précisément là qu'on dépose un dossier ;
//! - une édition **hors série** est publique elle aussi : la vue joint la série
//!   par la gauche, et une jointure stricte ferait disparaître de l'historique
//!   tout rendez-vous ponctuel.

mod commun;

use commun::Bac;
use event::service::public_read;

/// Une édition dans un statut donné, sans série ni pays : le minimum publiable.
async fn edition(bac: &Bac, slug: &str, statut: &str, avec_serie: bool) -> uuid::Uuid {
    let serie = if avec_serie {
        Some(commun::serie(bac, "cop_climate").await)
    } else {
        None
    };

    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_year, title, slug, description, status,
                participation_mode, timezone, starts_at, ends_at)
           VALUES ($1, 2027, '{"fr":"Édition"}'::jsonb, $2::text::platform.slug,
                   '{"fr":"Description."}'::jsonb, $3::text::event.event_status,
                   'online', 'Africa/Dakar'::platform.timezone_name,
                   now() + interval '30 days', now() + interval '32 days')
        RETURNING id"#,
        serie,
        slug,
        statut
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'édition")
}

#[tokio::test]
async fn le_brouillon_et_lannulee_sont_absents_lannoncee_est_presente() {
    let bac = Bac::monter().await;

    edition(&bac, "edition-brouillon", "draft", true).await;
    edition(&bac, "edition-annulee", "cancelled", true).await;
    let annoncee = edition(&bac, "edition-annoncee", "announced", true).await;
    let hors_serie = edition(&bac, "edition-hors-serie", "announced", false).await;

    let publiques = public_read::editions(bac.pool())
        .await
        .expect("lecture publique");

    let vues: Vec<uuid::Uuid> = publiques.iter().map(|e| e.id).collect();

    assert!(
        vues.contains(&annoncee),
        "une édition ANNONCÉE est publique, même sans programme publié"
    );
    assert!(
        vues.contains(&hors_serie),
        "une édition HORS SÉRIE reste dans l'historique — la jointure est par la gauche"
    );
    assert_eq!(
        publiques.len(),
        2,
        "le brouillon et l'annulée n'y sont pas : {vues:?}"
    );

    // La série est **résolue**, ou absente : jamais un identifiant nu.
    let annoncee = publiques.iter().find(|e| e.id == annoncee).unwrap();
    assert!(annoncee.series_kind.is_some(), "le genre de la série");
    let hors_serie = publiques.iter().find(|e| e.id == hors_serie).unwrap();
    assert!(hors_serie.series_id.is_none());
}

/// **Aucune session n'est ouverte dans ce test**, et c'est le point : la lecture
/// publique ne consulte ni périmètre ni permission.
#[tokio::test]
async fn la_page_dune_edition_se_lit_par_son_adresse() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;

    let page = public_read::edition_par_slug(bac.pool(), commun::seed::SLUG_COP31)
        .await
        .expect("lecture publique")
        .expect("la COP31 est annoncée, donc publique");

    assert_eq!(page.id, editions.cop31);
    assert_eq!(page.acronym.as_deref(), Some("COP31"));
    assert_eq!(page.country_code.as_deref(), Some("BR"));
    assert_eq!(page.city.as_deref(), Some("Belém"));
}

/// **Brouillon, annulée et adresse inconnue sont indiscernables.** Sans cela,
/// l'adresse d'une édition en préparation se devinerait par la forme de la
/// réponse.
#[tokio::test]
async fn le_brouillon_lannulee_et_ladresse_inconnue_rendent_la_meme_chose() {
    let bac = Bac::monter().await;

    edition(&bac, "en-preparation", "draft", true).await;
    edition(&bac, "abandonnee", "cancelled", true).await;

    for adresse in ["en-preparation", "abandonnee", "nexiste-pas-du-tout"] {
        let reponse = public_read::edition_par_slug(bac.pool(), adresse)
            .await
            .expect("lecture publique");
        assert!(reponse.is_none(), "« {adresse} » ne doit rien rendre");
    }
}
