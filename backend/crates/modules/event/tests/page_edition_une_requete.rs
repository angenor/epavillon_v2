//! **La page d'une édition en UNE requête** (research.md § R16, SC-022).
//!
//! Deux écarts consignés se referment ici, et aucun n'a demandé de code écrit
//! pour lui :
//!
//! - **écart n° 25** — la page portait ses images par un appel séparé. La vue
//!   `event.v_public_editions` rend déjà les **trois déclinaisons** résolues par
//!   `media.attached_image()` ; l'aller-retour disparaît ;
//! - **écart n° 26** — le critère de publicité était recopié dans chaque écran.
//!   Il vit dans la vue.
//!
//! L'échéance rendue est **l'échéance effective**, prolongation comprise —
//! `event.effective_deadline()`, appelée et jamais recalculée. Une page qui
//! afficherait la clôture initiale après une prolongation dirait à une
//! organisation qu'elle n'a plus le temps de déposer.

mod commun;

use commun::Bac;
use event::service::public_read;

#[tokio::test]
async fn les_trois_declinaisons_voyagent_avec_la_page() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    commun::seed::images_de_ledition(&bac, editions.cop31).await;

    let page = public_read::edition_par_slug(bac.pool(), commun::seed::SLUG_COP31)
        .await
        .expect("lecture publique")
        .expect("la COP31 est publique");

    for (role, image) in [
        ("banner", &page.banner),
        ("cover", &page.cover),
        ("thumbnail", &page.thumbnail),
    ] {
        let image = image
            .as_ref()
            .unwrap_or_else(|| panic!("la déclinaison « {role} » doit voyager avec la page"));
        assert!(
            image.get("url").and_then(|u| u.as_str()).is_some(),
            "« {role} » doit porter son adresse : {image}"
        );
    }
}

/// **Une édition sans image reste publique**, et les trois clés valent `null` :
/// c'est le cas courant, et chaque écran porte déjà son repli.
#[tokio::test]
async fn une_edition_sans_image_reste_publique() {
    let bac = Bac::monter().await;
    commun::seed::editions(&bac).await;

    let page = public_read::edition_par_slug(bac.pool(), commun::seed::SLUG_COP31)
        .await
        .expect("lecture publique")
        .expect("la COP31 est publique");

    assert!(page.banner.is_none() || page.banner == Some(serde_json::Value::Null));
    assert!(page.cover.is_none() || page.cover == Some(serde_json::Value::Null));
}

/// **L'appel est résolu, et son échéance est l'échéance EFFECTIVE.**
#[tokio::test]
async fn lappel_est_resolu_et_la_prolongation_deplace_lecheance() {
    let bac = Bac::monter().await;
    let editions = commun::seed::editions(&bac).await;
    let enfants = commun::seed::enfants(&bac, editions.cop31).await;

    let avant = public_read::edition_par_slug(bac.pool(), commun::seed::SLUG_COP31)
        .await
        .expect("lecture")
        .expect("publique");

    assert_eq!(avant.call_id, Some(enfants.appel));
    assert_eq!(avant.call_status.as_deref(), Some("open"));
    assert_eq!(
        avant.call_is_open,
        Some(true),
        "statut ET fenêtre : l'appel semé encadre l'instant courant"
    );
    let echeance_initiale = avant.call_deadline.expect("l'échéance de l'appel");

    // On prolonge : l'échéance effective doit suivre.
    sqlx::query!(
        "UPDATE event.calls_for_proposals SET extended_until = closes_at + interval '15 days'
          WHERE id = $1",
        enfants.appel
    )
    .execute(bac.pool())
    .await
    .expect("prolongation");

    let apres = public_read::edition_par_slug(bac.pool(), commun::seed::SLUG_COP31)
        .await
        .expect("lecture")
        .expect("publique");

    let echeance_prolongee = apres.call_deadline.expect("l'échéance prolongée");
    assert!(
        echeance_prolongee > echeance_initiale,
        "la page doit annoncer l'échéance EFFECTIVE, pas la clôture initiale"
    );
}

/// **Le volume du programme publié est joint par la gauche.** Une édition
/// annoncée sans aucune séance publiée doit rester visible, à zéro — c'est la
/// leçon de B2, où une jointure stricte rendait une liste vide sur base neuve.
#[tokio::test]
async fn une_edition_sans_seance_publiee_reste_visible_a_zero() {
    let bac = Bac::monter().await;
    commun::seed::editions(&bac).await;

    let page = public_read::edition_par_slug(bac.pool(), commun::seed::SLUG_COP31)
        .await
        .expect("lecture")
        .expect("publique");

    assert_eq!(page.published_session_count, 0);
    assert_eq!(page.organization_count, 0);
    assert!(page.programme_starts_at.is_none());
}
