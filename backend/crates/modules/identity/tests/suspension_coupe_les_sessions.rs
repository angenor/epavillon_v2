//! FR-033 : une session cesse de valoir dès que la personne est suspendue,
//! exclue ou anonymisée — **sans attendre son échéance**.
//!
//! Le jeton d'accès porte quinze minutes de signature valable ; c'est la
//! relecture de la session en base, à chaque requête, qui rend la coupure
//! immédiate. Un jeton qui porterait ses droits laisserait un quart d'heure de
//! sursis à quelqu'un qu'on vient d'exclure.

mod commun;

use commun::{connexion, semer, Bac, Compte};
use identity::domain::ids::PersonId;
use identity::service::session::{self, Device, RefreshOutcome};
use time::{Duration, OffsetDateTime};

const ADRESSE: &str = "awa.diallo@example.org";

#[tokio::test]
async fn suspension_coupe_les_sessions() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 1);

    // La coupure vit DANS la transaction du changement de statut : un intervalle
    // entre les deux serait exactement la fenêtre qu'on ferme.
    let db = bac.db();
    let ctx = bac.ctx();
    let mut tx = db.write(&ctx).await.expect("transaction");
    sqlx::query!(
        "UPDATE identity.people
            SET status = 'suspended', suspended_until = $2, status_changed_at = now()
          WHERE id = $1",
        personne,
        OffsetDateTime::now_utc() + Duration::days(7)
    )
    .execute(&mut *tx)
    .await
    .expect("suspension");
    let coupees = session::cut_on_status_change(&mut tx, PersonId(personne))
        .await
        .expect("coupure des sessions");
    tx.commit().await.expect("validation");

    assert_eq!(coupees, 1);
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
    assert_eq!(
        commun::sessions(&bac, personne).await[0].1.as_deref(),
        Some("status_changed")
    );

    assert_eq!(
        commun::acteur_resolu(&bac, &ouverte.access_token).await,
        None
    );

    let issue = session::refresh(
        &bac.state,
        &bac.ctx(),
        &ouverte.refresh_token,
        Device::default(),
    )
    .await
    .expect("renouvellement");
    assert!(matches!(issue, RefreshOutcome::Expired));
}

/// Même règle pour un changement de mot de passe : les sessions ouvertes
/// ailleurs tombent avec l'ancien secret.
#[tokio::test]
async fn un_changement_de_mot_de_passe_coupe_les_sessions() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let un = connexion(&bac, ADRESSE).await;
    let deux = connexion(&bac, ADRESSE).await;

    let db = bac.db();
    let ctx = bac.ctx();
    let mut tx = db.write(&ctx).await.expect("transaction");
    let coupees = session::cut_on_password_change(&mut tx, PersonId(personne))
        .await
        .expect("coupure des sessions");
    tx.commit().await.expect("validation");

    assert_eq!(coupees, 2);
    assert_eq!(commun::acteur_resolu(&bac, &un.access_token).await, None);
    assert_eq!(commun::acteur_resolu(&bac, &deux.access_token).await, None);
}

/// L'anonymisation est écrite par la base elle-même : le service ne la refait
/// pas, il vérifie qu'elle produit le même effet.
#[tokio::test]
async fn lanonymisation_coupe_aussi_les_sessions() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;
    let ouverte = connexion(&bac, ADRESSE).await;

    let db = bac.db();
    let ctx = bac.ctx();
    let mut tx = db.write(&ctx).await.expect("transaction");
    sqlx::query!("SELECT identity.anonymize_person($1, 'test')", personne)
        .execute(&mut *tx)
        .await
        .expect("anonymisation");
    tx.commit().await.expect("validation");

    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 0);
    assert_eq!(
        commun::sessions(&bac, personne).await[0].1.as_deref(),
        Some("anonymization")
    );
    assert_eq!(
        commun::acteur_resolu(&bac, &ouverte.access_token).await,
        None
    );
}
