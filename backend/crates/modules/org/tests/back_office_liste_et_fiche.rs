//! La liste et la fiche du back-office : les trois périmètres, le drapeau de
//! restriction, et les facettes comptées sur le jeu affiché.
//!
//! **Une organisation n'appartient à aucune édition.** La règle métier n° 8 ne
//! peut donc pas se lire comme ailleurs : c'est **l'activité déposée ou tenue**
//! qui rattache une fiche à un périmètre.

mod commun;

use commun::{ifdd, perimetres, Bac};
use org::domain::ids::OrganizationId;
use org::service::{admin_detail, admin_list};
use uuid::Uuid;

/// Un dossier déposé par une organisation dans une édition : c'est ce qui la
/// fait entrer dans le périmètre de qui administre cette édition.
async fn deposer(bac: &Bac, organisation: Uuid, evenement: Uuid, qui: Uuid, reference: &str) {
    sqlx::query!(
        r#"INSERT INTO programme.proposals
               (reference_code, event_id, organization_id, submitted_by, title, slug,
                objectives, detailed_presentation, format, submitted_at, status)
           VALUES ($4, $2, $1, $3,
                   jsonb_build_object('fr', 'Atelier de démonstration'),
                   lower($4 || '-atelier')::platform.slug,
                   jsonb_build_object('fr', 'Objectifs'),
                   jsonb_build_object('fr', 'Présentation'),
                   'online', now(), 'submitted')"#,
        organisation,
        evenement,
        qui,
        reference
    )
    .execute(bac.pool())
    .await
    .expect("dépôt du dossier");
}

#[tokio::test]
async fn un_perimetre_global_voit_toutes_les_fiches() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    commun::seed::paire_osed(&bac).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    // **Sans rafraîchir la projection** : la liste doit tout de même tout
    // montrer. Une fiche créée il y a dix secondes n'est pas encore dans la
    // projection, et c'est justement celle que l'équipe vient traiter.
    let ecran = admin_list::screen(bac.pool(), &perimetre)
        .await
        .expect("liste");

    assert!(
        ecran.rows.len() >= 3,
        "l'IFDD et les deux fiches OSED, au moins : {}",
        ecran.rows.len()
    );
    assert!(
        !ecran.scoped_to_events,
        "un périmètre global n'est pas restreint"
    );
}

/// **Un administrateur détaché ne voit que les fiches qui ont déposé ou tenu une
/// activité dans son édition.**
#[tokio::test]
async fn un_perimetre_detache_ne_voit_que_les_fiches_de_son_edition() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;
    let ifdd = ifdd(&bac).await;

    // Seule la fiche OSED complète a déposé dans l'édition administrée.
    deposer(
        &bac,
        osed.complete,
        p.edition_detachee,
        p.detachee,
        "COP31-001",
    )
    .await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.detachee)
        .await
        .expect("périmètre détaché");

    let ecran = admin_list::screen(bac.pool(), &perimetre)
        .await
        .expect("liste");

    let ids: Vec<Uuid> = ecran
        .rows
        .iter()
        .map(|r| r.organization_id.as_uuid())
        .collect();

    assert_eq!(
        ids,
        vec![osed.complete],
        "une seule fiche dans le périmètre"
    );
    assert!(
        !ids.contains(&ifdd),
        "l'IFDD n'a rien déposé dans cette édition"
    );
    assert!(
        ecran.scoped_to_events,
        "l'écran doit dire que la liste est restreinte, plutôt que de laisser \
         croire que la plateforme ne compte que ces fiches"
    );
}

/// Les facettes sont comptées **sur le jeu affiché**, jamais sur la base
/// entière : « Sénégal (3) » doit correspondre à ce qui s'affiche.
#[tokio::test]
async fn les_facettes_sont_comptees_sur_le_jeu_affiche() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    deposer(
        &bac,
        osed.complete,
        p.edition_detachee,
        p.detachee,
        "COP31-001",
    )
    .await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.detachee)
        .await
        .expect("périmètre détaché");
    let ecran = admin_list::screen(bac.pool(), &perimetre)
        .await
        .expect("liste");

    let total_types: i64 = ecran.types.iter().map(|f| f.count).sum();
    let total_pays: i64 = ecran.countries.iter().map(|f| f.count).sum();

    assert_eq!(
        total_types,
        ecran.rows.len() as i64,
        "chaque ligne compte pour une facette de type"
    );
    assert!(
        total_pays <= ecran.rows.len() as i64,
        "une fiche sans pays ne compte dans aucune facette de pays"
    );
    assert_eq!(ecran.types[0].value, "ngo_association");
}

/// La fiche entière, ses huit lectures.
#[tokio::test]
async fn la_fiche_assemble_ses_huit_lectures() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let organisation = ifdd(&bac).await;

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    let fiche = admin_detail::detail(bac.pool(), &perimetre, OrganizationId(organisation))
        .await
        .expect("lecture")
        .expect("la fiche de l'IFDD");

    assert_eq!(
        fiche.legal_name,
        "Institut de la Francophonie pour le développement durable"
    );
    assert!(fiche.verified_at.is_some(), "le sceau");
    assert_eq!(
        fiche.names.len(),
        6,
        "cinq dénominations semées, plus le sigle posé par le trigger"
    );
    assert_eq!(fiche.domains.len(), 2);
    assert_eq!(fiche.members.len(), 1);
    assert!(fiche.merged_into.is_none());
    assert!(fiche.absorbed.is_empty());
    // **La fiche de performance peut manquer**, et c'est normal : la projection
    // n'est rafraîchie que par un travail différé. Une fois rafraîchie, elle est
    // là.
    assert!(fiche.scorecard.is_none() || fiche.scorecard.is_some());

    sqlx::query("REFRESH MATERIALIZED VIEW analytics.mv_organization_scorecard")
        .execute(bac.pool())
        .await
        .expect("rafraîchissement de la projection");

    let apres = admin_detail::detail(bac.pool(), &perimetre, OrganizationId(organisation))
        .await
        .expect("lecture")
        .expect("la fiche");
    assert!(
        apres.scorecard.is_some(),
        "une fois la projection rafraîchie, la fiche de performance est là"
    );

    // **Les dénominations posées par la base sont marquées.** Elles ne se
    // retirent pas à la main : elles suivent la fiche.
    let derivees = fiche.names.iter().filter(|n| n.is_derived).count();
    assert_eq!(
        derivees, 2,
        "le nom légal et le sigle, recopiés par tg_organizations_sync_names"
    );

    // L'historique rend le libellé d'auteur dénormalisé : il reste lisible après
    // anonymisation, quand l'identifiant ne pointe plus vers personne.
    assert!(
        !fiche.history.is_empty(),
        "le journal d'audit porte au moins la création"
    );
}

/// **Une fiche absorbée est consultable et porte son renvoi.**
#[tokio::test]
async fn une_fiche_absorbee_est_consultable_et_porte_son_renvoi() {
    let bac = Bac::monter().await;
    let p = perimetres(&bac).await;
    let osed = commun::seed::paire_osed(&bac).await;

    let db = bac.db();
    let mut tx = db
        .write(&bac.ctx().with_actor(p.globale))
        .await
        .expect("transaction");
    sqlx::query_scalar!(
        "SELECT org.merge_organizations($1, $2, 'doublon manifeste')",
        osed.jumelle,
        osed.complete
    )
    .fetch_one(&mut *tx)
    .await
    .expect("fusion");
    tx.commit().await.expect("validation");

    let perimetre = kernel::auth::require_perimeter(bac.pool(), p.globale)
        .await
        .expect("périmètre global");

    let absorbee = admin_detail::detail(bac.pool(), &perimetre, OrganizationId(osed.jumelle))
        .await
        .expect("lecture")
        .expect("la fiche absorbée s'ouvre normalement");

    let renvoi = absorbee
        .merged_into
        .expect("le renvoi vers la fiche vivante");
    assert_eq!(renvoi.organization_id.as_uuid(), osed.complete);
    assert_eq!(
        renvoi.legal_name,
        "Observatoire du Sahel pour l'environnement et le développement"
    );
    assert!(renvoi.merged_at.is_some());

    // Et la survivante porte la trace de ce qu'elle a absorbé.
    let survivante = admin_detail::detail(bac.pool(), &perimetre, OrganizationId(osed.complete))
        .await
        .expect("lecture")
        .expect("la fiche vivante");
    assert_eq!(survivante.absorbed.len(), 1);
    assert_eq!(survivante.merges.len(), 1, "la fusion est au journal");
    assert_eq!(
        survivante.merges[0].reason.as_deref(),
        Some("doublon manifeste")
    );
}
