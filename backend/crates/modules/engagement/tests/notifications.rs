//! **Chacun choisit ce qu'il reçoit — et ce qui est critique part quand même.**
//!
//! Ce que ces tests éprouvent et qu'aucune relecture ne remplace :
//!
//! - **couper un type critique est sans effet, et la lecture le DIT** : sans ce
//!   dernier point, l'écran afficherait un interrupteur éteint pour un avis qui
//!   part quand même, et la personne croirait s'être désabonnée ;
//! - **trois faits de même nature forment UNE ligne portant un compte** — le
//!   modèle le décrit et aucune fonction ne le fait ;
//! - **le lien de rebond est relatif** : un nom d'hôte de préproduction ne doit
//!   pas entrer en base.

mod commun;

use commun::Bac;
use engagement::domain::notification::NotificationPreferencePayload;
use engagement::service::notifications::{self, Audience, BroadcastPayload, FilQuery};
use serde_json::json;
use uuid::Uuid;

/// Le type de l'annonce de plateforme : deux canaux, criticité **basse**. C'est
/// le seul type semé qui se coupe et qui porte les deux canaux — donc le seul
/// sur lequel « coupé ici, servi là » se mesure.
const ANNONCE: &str = "engagement.announcement.general";
/// Un type **critique** : une séance annulée se dit, préférence ou non.
const SEANCE_ANNULEE: &str = "programme.session.cancelled";

fn fil() -> FilQuery {
    FilQuery {
        unread_only: false,
        limit: None,
        before: None,
    }
}

async fn couper(bac: &Bac, personne: Uuid, type_code: &str, canal: &str) {
    notifications::ecrire_les_preferences(
        &bac.state,
        &bac.ctx(),
        personne,
        "fr",
        &[NotificationPreferencePayload {
            type_code: type_code.to_owned(),
            channel: canal.to_owned(),
            is_enabled: false,
        }],
    )
    .await
    .expect("écriture de la préférence");
}

async fn deplacer_dune_heure(bac: &Bac, seance: Uuid) {
    sqlx::query!(
        "UPDATE programme.sessions
            SET starts_at = starts_at + interval '1 hour',
                ends_at   = ends_at   + interval '1 hour'
          WHERE id = $1",
        seance
    )
    .execute(bac.pool())
    .await
    .expect("séance déplacée");
}

async fn lignes_du_fil(bac: &Bac, personne: Uuid) -> usize {
    notifications::fil(&bac.state, personne, &fil())
        .await
        .expect("le fil")
        .items
        .len()
}

async fn diffuser(bac: &Bac, diffuseuse: Uuid) -> notifications::BroadcastResult {
    notifications::diffuser(
        &bac.state,
        &bac.ctx(),
        diffuseuse,
        &BroadcastPayload {
            title: json!({ "fr": "Le pavillon ouvre lundi", "en": "The pavilion opens Monday" }),
            body: json!({ "fr": "Rendez-vous à 9 h.", "en": "See you at 9." }).into(),
            link_path: Some("/event/cop31-belem".to_owned()),
            audience: Audience::All,
        },
    )
    .await
    .expect("diffusion de l'annonce")
}

// -----------------------------------------------------------------------------
// T176 — couper un canal, garder l'autre
// -----------------------------------------------------------------------------

/// 🔴 **Couper un canal pour un type non critique supprime l'envoi sur ce canal
/// et le laisse sur l'autre.**
///
/// Les deux canaux se consultent **séparément** : les confondre ferait d'un
/// « je ne veux plus de courriels » un « je ne veux plus rien savoir », et la
/// personne manquerait ce qui l'attend à l'écran.
#[tokio::test]
async fn couper_un_canal_laisse_lautre() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let diffuseuse = commun::redactrice(&bac).await;

    // L'animatrice coupe le courriel ; l'étrangère coupe l'écran.
    couper(&bac, terrain.animatrice, ANNONCE, "email").await;
    couper(&bac, terrain.etrangere, ANNONCE, "in_app").await;

    let issue = diffuser(&bac, diffuseuse).await;

    assert_eq!(
        lignes_du_fil(&bac, terrain.animatrice).await,
        1,
        "elle garde l'écran"
    );
    assert_eq!(
        lignes_du_fil(&bac, terrain.etrangere).await,
        0,
        "elle a coupé l'écran"
    );

    let adresses: Vec<String> = bac.boite.messages().into_iter().map(|m| m.to).collect();
    assert!(
        !adresses.contains(&"animatrice@example.org".to_owned()),
        "elle a coupé le courriel : {adresses:?}"
    );
    assert!(
        adresses.contains(&"etrangere@example.org".to_owned()),
        "elle ne l'a pas coupé : {adresses:?}"
    );
    assert!(issue.recipients > 0 && issue.emailed > 0);
}

// -----------------------------------------------------------------------------
// T177 — couper un type critique est sans effet
// -----------------------------------------------------------------------------

/// 🔴 **Couper un type critique est sans effet, et la lecture le dit.**
///
/// L'écriture est **enregistrée** — refuser laisserait l'écran sans réponse à
/// donner, et l'interrupteur reviendrait à sa position sans explication. C'est
/// `is_overridable` qui porte l'information, et sans lui la personne croirait
/// s'être désabonnée d'une annulation de séance.
#[tokio::test]
async fn couper_un_type_critique_est_sans_effet_et_la_lecture_le_dit() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    couper(&bac, terrain.animatrice, SEANCE_ANNULEE, "in_app").await;

    // L'arbitrage est bien **enregistré** en base.
    let pose = sqlx::query_scalar!(
        r#"SELECT is_enabled AS "actif!" FROM engagement.notification_preferences
            WHERE person_id = $1 AND type_code = $2 AND channel = 'in_app'"#,
        terrain.animatrice,
        SEANCE_ANNULEE
    )
    .fetch_one(bac.pool())
    .await
    .expect("la préférence posée");
    assert!(!pose, "l'écriture n'est jamais refusée");

    // Et il n'oppose rien.
    assert!(
        engagement::repo::delivery::canal_autorise(
            bac.pool(),
            terrain.animatrice,
            SEANCE_ANNULEE,
            "in_app"
        )
        .await
        .expect("consultation du canal"),
        "un type critique ignore la préférence"
    );

    let lignes = notifications::preferences(&bac.state, terrain.animatrice, "fr")
        .await
        .expect("les préférences");
    let critique = lignes
        .iter()
        .find(|l| l.type_code == SEANCE_ANNULEE && l.channel == "in_app")
        .expect("la ligne du type critique");
    assert!(
        !critique.is_overridable,
        "la lecture DIT que l'arbitrage n'oppose rien"
    );
    assert!(
        !critique.is_enabled,
        "et elle rend tout de même l'arbitrage posé, pour que l'écran ne mente pas sur ce qui est enregistré"
    );

    let ordinaire = lignes
        .iter()
        .find(|l| l.type_code == ANNONCE && l.channel == "email")
        .expect("la ligne d'un type ordinaire");
    assert!(ordinaire.is_overridable);
    assert!(
        ordinaire.is_enabled,
        "sans arbitrage, le repli est celui des canaux par défaut du type"
    );
}

// -----------------------------------------------------------------------------
// T179 et T180 — le regroupement, et le lien relatif
// -----------------------------------------------------------------------------

/// 🔴 **Trois faits de même nature forment UNE ligne portant un compte**, tant
/// qu'elle n'est pas lue (FR-092).
///
/// Le modèle le décrit — *« le worker incrémente `group_count` […] plutôt que
/// d'en créer une autre »* — et aucune fonction ne le fait. Sans lui, l'écran
/// affiche une pile qui se répète et le badge annonce trois avis pour un seul
/// fait.
///
/// **Une fois lue, elle ne se regroupe plus** : l'index d'unicité est partiel, et
/// c'est le comportement voulu — un fait nouveau après lecture est une ligne
/// nouvelle.
#[tokio::test]
async fn trois_faits_de_meme_cle_forment_une_ligne() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Trois annonces de séance déplacée : même type, même séance, donc même clé.
    for _ in 0..3 {
        deplacer_dune_heure(&bac, terrain.seance).await;
    }
    commun::relayer(&bac, "programme.session.rescheduled").await;

    let fil_lu = notifications::fil(&bac.state, terrain.inscrits[0], &fil())
        .await
        .expect("le fil");
    assert_eq!(fil_lu.items.len(), 1, "une ligne, jamais trois");
    assert_eq!(fil_lu.items[0].group_count, 3, "et elle porte le compte");
    assert_eq!(
        fil_lu.unread_count, 1,
        "le badge compte les lignes, pas les faits"
    );

    // **Le lien de rebond est RELATIF** : un nom d'hôte de préproduction ne doit
    // pas entrer en base. La contrainte du modèle l'exige déjà ; ce test le
    // mesure sur la charge servie.
    let lien = fil_lu.items[0]
        .link_path
        .as_deref()
        .expect("un lien de rebond");
    assert!(lien.starts_with('/'), "{lien}");
    assert!(!lien.contains("http"), "{lien}");
    assert!(!lien.contains("localhost"), "{lien}");

    // Lue, la ligne cesse de se regrouper : le fait suivant est un fait nouveau.
    notifications::marquer_lues(
        &bac.state,
        &bac.ctx(),
        terrain.inscrits[0],
        &notifications::MarquagePayload { ids: None },
    )
    .await
    .expect("marquage");

    deplacer_dune_heure(&bac, terrain.seance).await;
    commun::relayer(&bac, "programme.session.rescheduled").await;

    let apres = notifications::fil(&bac.state, terrain.inscrits[0], &fil())
        .await
        .expect("le fil");
    assert_eq!(apres.items.len(), 2, "une ligne lue ne se regroupe plus");
    assert_eq!(apres.unread_count, 1);
}

/// **Le fil ne rend que les siennes**, et l'archivage range sans détruire.
#[tokio::test]
async fn le_fil_ne_rend_que_les_siennes_et_larchivage_range() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;
    let diffuseuse = commun::redactrice(&bac).await;

    diffuser(&bac, diffuseuse).await;

    let fil_dun_inscrit = notifications::fil(&bac.state, terrain.inscrits[0], &fil())
        .await
        .expect("le fil");
    assert_eq!(fil_dun_inscrit.items.len(), 1);

    let ids: Vec<Uuid> = fil_dun_inscrit.items.iter().map(|n| n.id).collect();
    let archivees = notifications::archiver(
        &bac.state,
        &bac.ctx(),
        terrain.inscrits[0],
        &notifications::ArchivagePayload { ids: ids.clone() },
    )
    .await
    .expect("archivage");
    assert_eq!(archivees, 1);

    let apres = notifications::fil(&bac.state, terrain.inscrits[0], &fil())
        .await
        .expect("le fil");
    assert!(apres.items.is_empty(), "rangée, elle ne s'affiche plus");
    assert_eq!(apres.unread_count, 0, "et elle ne pèse plus sur le badge");

    // Rangée, jamais détruite : la trace reste en base.
    let restantes = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM engagement.notifications WHERE person_id = $1"#,
        terrain.inscrits[0]
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage");
    assert_eq!(restantes, 1);

    // **Archiver la notification d'un autre ne fait rien** : le filtre porte sur
    // le compte de l'appelant, jamais sur la seule liste reçue.
    let volee = notifications::archiver(
        &bac.state,
        &bac.ctx(),
        terrain.etrangere,
        &notifications::ArchivagePayload { ids },
    )
    .await
    .expect("archivage d'autrui");
    assert_eq!(volee, 0);
}

/// **La diffusion est gardée par sa permission**, sur la portée globale : une
/// annonce s'adresse à la plateforme, pas à une édition.
#[tokio::test]
async fn la_diffusion_est_gardee() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let refus = notifications::diffuser(
        &bac.state,
        &bac.ctx(),
        terrain.administratrice,
        &BroadcastPayload {
            title: json!({ "fr": "Annonce" }),
            body: None,
            link_path: None,
            audience: Audience::All,
        },
    )
    .await
    .expect_err("l'administratrice d'une édition ne diffuse pas à la plateforme");
    assert_eq!(refus.code, kernel::ErrorCode::Forbidden);

    // **Un lien absolu est refusé** : un nom d'hôte n'entre pas en base.
    let diffuseuse = commun::redactrice(&bac).await;
    let refus = notifications::diffuser(
        &bac.state,
        &bac.ctx(),
        diffuseuse,
        &BroadcastPayload {
            title: json!({ "fr": "Annonce" }),
            body: None,
            link_path: Some("https://recette.epavillon.org/event/cop31".to_owned()),
            audience: Audience::All,
        },
    )
    .await
    .expect_err("un lien absolu doit être refusé");
    assert_eq!(refus.code, kernel::ErrorCode::ValidationFailed);
}
