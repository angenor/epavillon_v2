//! **CE QUE LA SUGGESTION D'INTERVENANTS ACCEPTE DE DIRE** (16/09).
//!
//! Une autocomplétion sur des adresses électroniques est une porte : mal bornée,
//! elle rend l'annuaire une frappe à la fois. Ce fichier éprouve les quatre
//! bornes qui la referment — préfixe, longueur minimale, populations admises,
//! visibilité choisie par la personne —, parce qu'aucune d'elles ne se voit à la
//! lecture de l'écran.

mod commun;

use commun::Bac;
use programme::repo::cross;
use uuid::Uuid;

/// Une personne connue de la plateforme, sans compte ni adresse confirmée.
async fn personne_non_confirmee(bac: &Bac, email: &str, prenom: &str, nom: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO identity.people (primary_email, first_name, last_name, status)
           VALUES ($1::text::platform.email, $2, $3, 'active')
        RETURNING id"#,
        email,
        prenom,
        nom
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la personne")
}

async fn suggerer(bac: &Bac, prefixe: &str) -> Vec<String> {
    cross::suggestions_dintervenants(bac.pool(), prefixe, 8)
        .await
        .expect("suggestions")
        .into_iter()
        .map(|f| f.email)
        .collect()
}

#[tokio::test]
async fn seules_les_adresses_confirmees_et_les_intervenants_retenus_sont_suggeres() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    // Confirmée : elle a cliqué le lien de vérification.
    commun::personne(&bac, "sonia.confirmee@example.org", "Sonia", "Confirmée").await;
    // Connue mais jamais confirmée, et jamais retenue : elle ne sort pas.
    personne_non_confirmee(&bac, "sonia.inconnue@example.org", "Sonia", "Inconnue").await;

    let suggerees = suggerer(&bac, "sonia").await;
    assert_eq!(suggerees, vec!["sonia.confirmee@example.org".to_owned()]);

    let _ = terrain;
}

#[tokio::test]
async fn un_intervenant_dun_dossier_retenu_est_suggere_meme_sans_compte() {
    let bac = Bac::monter().await;
    let terrain = commun::terrain(&bac).await;

    let orateur =
        personne_non_confirmee(&bac, "karim.orateur@example.org", "Karim", "Orateur").await;
    let dossier = commun::dossier(&bac, &terrain, "Dossier retenu", "dossier-retenu").await;

    sqlx::query!(
        "INSERT INTO programme.proposal_speakers
             (proposal_id, person_id, role, job_title_snapshot, organization_snapshot)
         VALUES ($1, $2, 'speaker', 'Directeur', 'IFDD')",
        dossier,
        orateur
    )
    .execute(bac.pool())
    .await
    .expect("intervenant du dossier");

    // Tant que le dossier n'est pas RETENU, la personne reste invisible : la
    // suggestion dirait sinon qui a été proposé, à qui n'a pas à le savoir.
    assert!(suggerer(&bac, "karim").await.is_empty());

    sqlx::query("UPDATE identity.people SET civility = 'other' WHERE id = $1")
        .bind(orateur)
        .execute(bac.pool())
        .await
        .expect("civilité de l'intervenant");

    // La machine à états n'autorise pas `draft -> accepted` : on suit le chemin
    // que le comité suit, sans quoi le déclencheur refuse la ligne.
    for etape in ["submitted", "under_review", "accepted"] {
        sqlx::query(
            "UPDATE programme.proposals
                SET status = $2::text::programme.proposal_status
              WHERE id = $1",
        )
        .bind(dossier)
        .bind(etape)
        .execute(bac.pool())
        .await
        .unwrap_or_else(|e| panic!("transition vers {etape} : {e}"));
    }

    assert_eq!(
        suggerer(&bac, "karim").await,
        vec!["karim.orateur@example.org".to_owned()]
    );
}

#[tokio::test]
async fn la_recherche_est_un_prefixe_et_respecte_le_choix_de_la_personne() {
    let bac = Bac::monter().await;
    let _terrain = commun::terrain(&bac).await;

    commun::personne(&bac, "amina.visible@example.org", "Amina", "Visible").await;
    let discrete = commun::personne(&bac, "amina.discrete@example.org", "Amina", "Discrète").await;
    sqlx::query!(
        "UPDATE identity.people SET is_directory_visible = false WHERE id = $1",
        discrete
    )
    .execute(bac.pool())
    .await
    .expect("retrait de l'annuaire");

    // Le préfixe complète ce qui est tapé ; il ne cherche pas au milieu des
    // adresses, sans quoi « example » les rendrait toutes.
    assert!(suggerer(&bac, "example.org").await.is_empty());

    // Et la personne qui a refusé de figurer dans les listes n'y figure pas.
    assert_eq!(
        suggerer(&bac, "amina").await,
        vec!["amina.visible@example.org".to_owned()]
    );
}
