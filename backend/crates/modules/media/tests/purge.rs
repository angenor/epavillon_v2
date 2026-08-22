//! **Le disque ne se remplit pas tout seul.**
//!
//! Ce que ces tests éprouvent et qu'aucune relecture de code ne remplace :
//!
//! 1. un objet purgé **a quitté le stockage** — vérifié sur le stockage
//!    lui-même, jamais sur une relecture en base : la ligne peut porter son
//!    instant de purge alors que les octets sont toujours là, et c'est
//!    précisément le défaut qu'on cherche ;
//! 2. la suppression d'un objet rattaché à **deux** fiches est refusée **en
//!    disant deux** — la déduplication traverse les propriétaires, et sans ce
//!    refus la première organisation ferait disparaître l'image de la seconde
//!    (écart n° 128) ;
//! 3. les travaux récurrents **se replanifient**, et un second armement n'en
//!    produit pas un second.
//!
//! # L'heure vient de la BASE, jamais de l'horloge de la machine
//!
//! Le conteneur dérive de quelques secondes. Un `run_at` posé depuis Rust peut
//! tomber dans le futur de la base : `claim_jobs()` ne réserve rien, et le test
//! échoue une fois sur deux sur une chaîne qui marche. Pour vieillir un objet,
//! on écrit `now() - interval …` **en SQL** ; pour poser un créneau, on part de
//! l'instant que la base rend.

mod commun;

use commun::Bac;
use kernel::ErrorCode;
use media::domain::attachment::AttachmentPayload;
use uuid::Uuid;

/// Un objet servable, appartenant à l'organisation, **et détaché** : le logo
/// qu'on a remplacé, et dont personne ne se souvient qu'il occupe de la place.
async fn objet_orphelin(bac: &Bac, terrain: &commun::Terrain) -> Uuid {
    let fichier = commun::couverture_16_9();
    let depose = commun::deposer(
        bac,
        terrain.referente,
        &fichier,
        commun::metadonnees_pour(
            &fichier,
            "org",
            "organizations",
            terrain.organisation,
            "logo",
        ),
    )
    .await
    .expect("dépôt");

    commun::passer_le_worker(bac).await;
    rattacher(
        bac,
        terrain.referente,
        terrain.organisation,
        depose.asset.id,
    )
    .await;
    detacher_tout(bac, terrain.referente, depose.asset.id).await;

    depose.asset.id
}

/// Rattache un objet au logo d'une organisation.
///
/// **Le dépôt ne rattache pas** : le rôle qu'il reçoit sert à éprouver la forme
/// du fichier — type, poids, cadrage —, et le rattachement est un geste à part.
async fn rattacher(bac: &Bac, acteur: Uuid, organisation: Uuid, asset_id: Uuid) {
    media::service::attach::poser(
        &bac.state,
        &bac.ctx(),
        acteur,
        &AttachmentPayload {
            owner_schema: "org".to_owned(),
            owner_table: "organizations".to_owned(),
            owner_id: organisation,
            role: "logo".to_owned(),
            asset_id,
            sort_order: None,
            alt_text_override: None,
        },
    )
    .await
    .expect("rattachement");
}

async fn detacher_tout(bac: &Bac, acteur: Uuid, asset_id: Uuid) {
    let rattachements = sqlx::query_scalar!(
        "SELECT id FROM media.attachments WHERE asset_id = $1",
        asset_id
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture des rattachements");

    for id in rattachements {
        media::service::attach::detacher(&bac.state, &bac.ctx(), acteur, id)
            .await
            .expect("détachement");
    }
}

/// Vieillit un objet **en SQL** : l'ancienneté se compte contre l'horloge de la
/// base, et une soustraction faite en Rust tomberait à côté.
async fn vieillir(bac: &Bac, asset_id: Uuid, jours: i32) {
    sqlx::query!(
        "UPDATE media.assets
            SET created_at = now() - make_interval(days => $2)
          WHERE id = $1",
        asset_id,
        jours
    )
    .execute(bac.pool())
    .await
    .expect("vieillissement de l'objet");
}

/// Fait échoir la fenêtre de rétention, toujours en SQL.
async fn echoir(bac: &Bac, asset_id: Uuid) {
    sqlx::query!(
        "UPDATE media.assets SET purge_after = now() - interval '1 hour' WHERE id = $1",
        asset_id
    )
    .execute(bac.pool())
    .await
    .expect("échéance de la fenêtre");
}

/// Pose une occurrence de travail récurrent **au présent de la base**.
async fn armer(bac: &Bac, tache: &str) {
    let maintenant = media::repo::assets::maintenant(bac.pool())
        .await
        .expect("instant de la base");
    let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
    let pose = match tache {
        "purge" => media::jobs::purge::planifier(&mut tx, maintenant).await,
        _ => media::jobs::reconcile::planifier(&mut tx, maintenant).await,
    }
    .expect("planification");
    tx.commit().await.expect("validation");
    assert!(pose, "le créneau n'était pas déjà posé");
}

/// Donne à une personne le droit du back-office : la permission **et** le
/// périmètre global.
async fn administratrice_globale(bac: &Bac) -> Uuid {
    let personne = commun::personne(bac, "quota@ifdd.org", "Paul", "Kaboré").await;
    commun::attribuer(bac, personne, "super_admin", "global", None).await;
    personne
}

// -----------------------------------------------------------------------------
// T192 — les orphelins
// -----------------------------------------------------------------------------

/// **Un objet non rattaché depuis plus d'un mois apparaît ; un objet rattaché
/// n'y apparaît jamais.**
#[tokio::test]
async fn un_objet_detache_et_ancien_apparait_dans_les_orphelins() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let admin = administratrice_globale(&bac).await;

    let orphelin = objet_orphelin(&bac, &terrain).await;
    vieillir(&bac, orphelin, 40).await;

    // Un second objet, rattaché celui-là, et tout aussi vieux.
    let fichier = commun::image("autre-logo.png", 1600, 900);
    let rattache = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees_pour(
            &fichier,
            "org",
            "organizations",
            terrain.organisation,
            "logo",
        ),
    )
    .await
    .expect("dépôt")
    .asset
    .id;
    commun::passer_le_worker(&bac).await;
    rattacher(&bac, terrain.referente, terrain.organisation, rattache).await;
    vieillir(&bac, rattache, 40).await;

    let orphelins = media::service::admin::orphelins(&bac.state, admin, None)
        .await
        .expect("lecture des orphelins");

    let vus: Vec<Uuid> = orphelins.iter().map(|o| o.asset_id).collect();
    assert!(vus.contains(&orphelin), "l'objet détaché doit apparaître");
    assert!(
        !vus.contains(&rattache),
        "un objet rattaché n'est JAMAIS un orphelin, quel que soit son âge"
    );

    // Le poids annoncé comprend les déclinaisons : c'est ce que la purge
    // libérera, et un chiffre qui ne les compterait pas sous-estimerait le gain.
    let ligne = orphelins
        .iter()
        .find(|o| o.asset_id == orphelin)
        .expect("l'orphelin est là");
    assert!(ligne.rendition_bytes > 0, "les déclinaisons sont comptées");
    assert!(ligne.age_days >= 40);
}

/// **Un objet trop jeune n'est pas proposé.** L'ancienneté par défaut vient des
/// réglages, et elle protège d'une purge de ce qui vient d'être déposé.
#[tokio::test]
async fn un_objet_recent_nest_pas_propose_a_la_purge() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let admin = administratrice_globale(&bac).await;

    let orphelin = objet_orphelin(&bac, &terrain).await;

    let orphelins = media::service::admin::orphelins(&bac.state, admin, None)
        .await
        .expect("lecture");
    assert!(
        !orphelins.iter().any(|o| o.asset_id == orphelin),
        "déposé à l'instant, il n'a rien à faire dans la liste"
    );

    // Le même, sans condition d'âge : il y est.
    let sans_delai = media::service::admin::orphelins(&bac.state, admin, Some(0))
        .await
        .expect("lecture");
    assert!(sans_delai.iter().any(|o| o.asset_id == orphelin));
}

/// **Un compte sans périmètre d'administration reçoit un refus**, jamais une
/// liste vide (principe V).
#[tokio::test]
async fn le_back_office_refuse_un_perimetre_vide() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let erreur = media::service::admin::orphelins(&bac.state, terrain.etrangere, None)
        .await
        .expect_err("un compte sans périmètre doit être refusé");
    assert_eq!(erreur.code, ErrorCode::Forbidden);

    let erreur = media::service::admin::quotas(&bac.state, terrain.membre)
        .await
        .expect_err("un membre d'organisation n'administre rien");
    assert_eq!(erreur.code, ErrorCode::Forbidden);
}

// -----------------------------------------------------------------------------
// T193 — la suppression d'un objet encore utilisé
// -----------------------------------------------------------------------------

/// **La suppression d'un objet rattaché à deux fiches est refusée, en disant
/// deux.**
///
/// C'est l'écart n° 128 : le même fichier déposé par deux organisations donne
/// **une** ligne, appartenant à la première. Une suppression par celle-ci ferait
/// disparaître l'image de la seconde.
#[tokio::test]
async fn supprimer_un_objet_rattache_a_deux_fiches_est_refuse_en_disant_deux() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let seconde = commun::organisation(&bac, "Institut du Sahel", "INSAH").await;
    commun::adherer(&bac, seconde, terrain.referente, "manager", "active").await;

    let fichier = commun::couverture_16_9();
    let asset = commun::deposer(
        &bac,
        terrain.referente,
        &fichier,
        commun::metadonnees_pour(
            &fichier,
            "org",
            "organizations",
            terrain.organisation,
            "logo",
        ),
    )
    .await
    .expect("dépôt")
    .asset
    .id;
    commun::passer_le_worker(&bac).await;

    rattacher(&bac, terrain.referente, terrain.organisation, asset).await;
    // La seconde organisation pose **le même objet** sur sa propre fiche : c'est
    // ce que produit la déduplication quand deux organisations déposent le même
    // fichier.
    rattacher(&bac, terrain.referente, seconde, asset).await;

    let erreur = media::service::admin::supprimer(&bac.state, &bac.ctx(), terrain.referente, asset)
        .await
        .expect_err("un objet encore utilisé ne se supprime pas");

    assert_eq!(erreur.code, ErrorCode::MediaAssetInUse);
    assert!(
        erreur.message.contains('2'),
        "le refus doit DIRE combien de fiches l'utilisent : {}",
        erreur.message
    );

    // Et rien n'a bougé : l'objet est toujours servable pour la seconde
    // organisation.
    let objet = media::service::read::objet(&bac.state, asset)
        .await
        .expect("lecture");
    assert_eq!(objet.status, "ready");
    assert!(objet.deleted_at.is_none());
}

/// **Détaché, il se supprime** — et la consommation baisse immédiatement, avant
/// même que la purge ne passe (FR-106).
#[tokio::test]
async fn un_objet_detache_se_supprime_et_la_consommation_baisse_aussitot() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let asset = objet_orphelin(&bac, &terrain).await;
    let avant = commun::espace_consomme(&bac, terrain.organisation).await;
    assert!(
        avant > 0,
        "l'objet et ses déclinaisons occupent de la place"
    );

    let issue = media::service::admin::supprimer(&bac.state, &bac.ctx(), terrain.referente, asset)
        .await
        .expect("suppression");
    assert!(
        issue.scheduled_purge_at.is_some(),
        "la réponse dit jusqu'à quand l'objet reste récupérable"
    );

    assert_eq!(
        commun::espace_consomme(&bac, terrain.organisation).await,
        0,
        "la consommation baisse à la suppression logique, pas à la purge"
    );
    assert!(
        media::service::read::objet(&bac.state, asset)
            .await
            .is_err(),
        "un objet supprimé n'est plus servi"
    );
}

// -----------------------------------------------------------------------------
// T194, T195, T188 — la purge
// -----------------------------------------------------------------------------

/// **Un objet purgé a quitté le stockage, et la consommation a baissé de son
/// poids, déclinaisons comprises.**
///
/// L'absence est vérifiée **sur le stockage lui-même** : une relecture en base
/// dirait seulement que la ligne porte un instant de purge, ce qui est
/// exactement ce qu'on écrirait si l'on oubliait d'appeler le stockage.
#[tokio::test]
async fn un_objet_purge_a_quitte_le_stockage() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let asset = objet_orphelin(&bac, &terrain).await;

    let cle = sqlx::query_scalar!("SELECT object_key FROM media.assets WHERE id = $1", asset)
        .fetch_one(bac.pool())
        .await
        .expect("clé de l'objet");
    let cles_declinaisons = sqlx::query_scalar!(
        "SELECT object_key FROM media.renditions WHERE asset_id = $1",
        asset
    )
    .fetch_all(bac.pool())
    .await
    .expect("clés des déclinaisons");
    assert_eq!(
        cles_declinaisons.len(),
        3,
        "les trois déclinaisons sont sur le stockage"
    );

    let consomme_avant = commun::espace_consomme(&bac, terrain.organisation).await;
    let poids_declinaisons =
        sqlx::query_scalar!(r#"SELECT media.rendition_bytes($1) AS "octets!""#, asset)
            .fetch_one(bac.pool())
            .await
            .expect("poids des déclinaisons");
    let poids_objet =
        sqlx::query_scalar!("SELECT byte_size FROM media.assets WHERE id = $1", asset)
            .fetch_one(bac.pool())
            .await
            .expect("poids de l'objet");

    media::service::admin::supprimer(&bac.state, &bac.ctx(), terrain.referente, asset)
        .await
        .expect("suppression");
    echoir(&bac, asset).await;

    armer(&bac, "purge").await;
    let issues = commun::passer_le_worker(&bac).await;
    assert!(
        issues.iter().all(|i| i.is_ok()),
        "la purge a échoué : {issues:?}"
    );

    // **Sur le stockage**, pas en base.
    assert!(
        commun::lire_sur_le_stockage(&bac, &cle).await.is_none(),
        "l'original est toujours sur le disque"
    );
    for cle in &cles_declinaisons {
        assert!(
            commun::lire_sur_le_stockage(&bac, cle).await.is_none(),
            "la déclinaison {cle} est toujours sur le disque"
        );
    }

    let purge_a = sqlx::query_scalar!("SELECT purged_at FROM media.assets WHERE id = $1", asset)
        .fetch_one(bac.pool())
        .await
        .expect("instant de purge");
    assert!(purge_a.is_some(), "l'instant de la purge est enregistré");

    assert_eq!(
        consomme_avant - commun::espace_consomme(&bac, terrain.organisation).await,
        poids_objet + poids_declinaisons,
        "la consommation baisse du poids de l'objet ET de ses déclinaisons"
    );

    // **L'annonce de la disparition effective est émise**, et elle seule : le
    // modèle a déjà annoncé l'intention.
    let annonces = sqlx::query!(
        r#"SELECT event_type AS "event_type!", payload AS "payload!"
             FROM platform.outbox_events
            WHERE aggregate_id = $1 ORDER BY id"#,
        asset
    )
    .fetch_all(bac.pool())
    .await
    .expect("lecture de l'outbox");

    let types: Vec<&str> = annonces.iter().map(|a| a.event_type.as_str()).collect();
    assert_eq!(
        types,
        vec![
            "media.asset.uploaded",
            "media.asset.purge_scheduled",
            "media.asset.purged"
        ],
        "une annonce par fait, jamais deux pour le même"
    );
    let purgee = annonces.last().expect("l'annonce de purge");
    assert_eq!(purgee.payload["rendition_bytes"], poids_declinaisons);
    assert_eq!(purgee.payload["byte_size"], poids_objet);
}

/// **Un objet dont la fenêtre n'est pas échue n'est pas touché.** C'est toute la
/// valeur de la fenêtre de rétention : une suppression par erreur se rattrape.
#[tokio::test]
async fn un_objet_dont_la_fenetre_court_encore_nest_pas_touche() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let asset = objet_orphelin(&bac, &terrain).await;
    let cle = sqlx::query_scalar!("SELECT object_key FROM media.assets WHERE id = $1", asset)
        .fetch_one(bac.pool())
        .await
        .expect("clé");

    media::service::admin::supprimer(&bac.state, &bac.ctx(), terrain.referente, asset)
        .await
        .expect("suppression");

    armer(&bac, "purge").await;
    commun::passer_le_worker(&bac).await;

    assert!(
        commun::lire_sur_le_stockage(&bac, &cle).await.is_some(),
        "la fenêtre court encore : l'objet doit rester récupérable"
    );
    let purge_a = sqlx::query_scalar!("SELECT purged_at FROM media.assets WHERE id = $1", asset)
        .fetch_one(bac.pool())
        .await
        .expect("lecture");
    assert!(purge_a.is_none());
}

/// **Une purge dont l'objet a déjà disparu du stockage aboutit** (FR-108).
///
/// L'objectif du travail est que l'objet ne soit plus là — pas qu'il ait fallu
/// l'enlever. Faire échouer ce cas laisserait le travail mourir sur un objet
/// déjà parti, et la ligne resterait éternellement « à purger ».
#[tokio::test]
async fn une_purge_dont_lobjet_a_deja_disparu_aboutit() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let asset = objet_orphelin(&bac, &terrain).await;
    let cles = sqlx::query!(
        r#"SELECT (SELECT object_key FROM media.assets WHERE id = $1) AS "objet!",
                  array_agg(r.object_key) AS "declinaisons!"
             FROM media.renditions r WHERE r.asset_id = $1"#,
        asset
    )
    .fetch_one(bac.pool())
    .await
    .expect("clés");

    media::service::admin::supprimer(&bac.state, &bac.ctx(), terrain.referente, asset)
        .await
        .expect("suppression");
    echoir(&bac, asset).await;

    // Quelqu'un a vidé le seau à la main — ou une purge précédente est morte
    // après avoir effacé, avant d'avoir écrit.
    for cle in std::iter::once(&cles.objet).chain(cles.declinaisons.iter()) {
        bac.state
            .storage()
            .delete(cle)
            .await
            .expect("effacement préalable");
    }

    armer(&bac, "purge").await;
    let issues = commun::passer_le_worker(&bac).await;
    assert!(
        issues.iter().all(|i| i.is_ok()),
        "un objet déjà parti n'est pas un échec : {issues:?}"
    );

    let purge_a = sqlx::query_scalar!("SELECT purged_at FROM media.assets WHERE id = $1", asset)
        .fetch_one(bac.pool())
        .await
        .expect("lecture");
    assert!(
        purge_a.is_some(),
        "l'objectif est atteint : la purge doit être datée"
    );
}

// -----------------------------------------------------------------------------
// T196 — la réconciliation
// -----------------------------------------------------------------------------

/// **Après réconciliation, la consommation enregistrée égale la consommation
/// calculée.**
///
/// Le compteur est incrémental — il doit l'être, un quota s'opposant au moment
/// du téléversement. Il dérive donc, et la dérive est ici provoquée à la main :
/// c'est le seul moyen de vérifier que le réalignement fait quelque chose.
#[tokio::test]
async fn la_reconciliation_realigne_le_compteur_sur_le_calcul() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let asset = objet_orphelin(&bac, &terrain).await;
    assert!(asset != Uuid::nil());

    // La dérive : le compteur annonce dix mégaoctets de plus que la réalité.
    sqlx::query!(
        "UPDATE media.storage_quotas SET used_bytes = used_bytes + 10485760, used_files = used_files + 3
          WHERE organization_id = $1",
        terrain.organisation
    )
    .execute(bac.pool())
    .await
    .expect("dérive du compteur");

    armer(&bac, "reconcile").await;
    let issues = commun::passer_le_worker(&bac).await;
    assert!(issues.iter().all(|i| i.is_ok()), "{issues:?}");

    let reel = sqlx::query!(
        r#"SELECT total_bytes AS "total!", file_count AS "fichiers!"
             FROM media.organization_storage_usage($1)"#,
        terrain.organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("consommation calculée");

    let enregistre = sqlx::query!(
        r#"SELECT used_bytes AS "octets!", used_files AS "fichiers!"
             FROM media.storage_quotas WHERE organization_id = $1"#,
        terrain.organisation
    )
    .fetch_one(bac.pool())
    .await
    .expect("consommation enregistrée");

    assert_eq!(enregistre.octets, reel.total);
    assert_eq!(i64::from(enregistre.fichiers), reel.fichiers);
}

// -----------------------------------------------------------------------------
// T197 — les chaînes récurrentes
// -----------------------------------------------------------------------------

/// **Les deux travaux récurrents du module se replanifient**, et un second
/// armement du même créneau n'en produit pas un second.
#[tokio::test]
async fn les_travaux_recurrents_se_replanifient_sans_doublon() {
    let bac = Bac::monter().await;

    for tache in ["purge", "reconcile"] {
        armer(&bac, tache).await;
    }

    // Le démarrage du worker repose le même créneau : la clé d'unicité le
    // confond avec celui qui est déjà là.
    let maintenant = media::repo::assets::maintenant(bac.pool())
        .await
        .expect("instant de la base");
    let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
    assert!(!media::jobs::purge::planifier(&mut tx, maintenant)
        .await
        .expect("replanification"));
    assert!(!media::jobs::reconcile::planifier(&mut tx, maintenant)
        .await
        .expect("replanification"));
    tx.commit().await.expect("validation");

    assert_eq!(compter_travaux(&bac, "media.purge_assets").await, 1);
    assert_eq!(compter_travaux(&bac, "media.reconcile_quotas").await, 1);

    // Le passage du worker les exécute **et pose l'occurrence suivante**.
    let issues = commun::passer_le_worker(&bac).await;
    assert_eq!(issues.len(), 2, "deux travaux étaient en file");
    assert!(issues.iter().all(|i| i.is_ok()), "{issues:?}");

    for tache in ["media.purge_assets", "media.reconcile_quotas"] {
        assert_eq!(
            compter_travaux(&bac, tache).await,
            2,
            "{tache} : l'occurrence exécutée et la suivante"
        );
        let en_attente = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM platform.jobs
                WHERE task = $1 AND status = 'queued' AND run_at > now()"#,
            tache
        )
        .fetch_one(bac.pool())
        .await
        .expect("comptage");
        assert_eq!(en_attente, 1, "{tache} : la chaîne continue");
    }
}

async fn compter_travaux(bac: &Bac, tache: &str) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs WHERE task = $1"#,
        tache
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des travaux")
}
