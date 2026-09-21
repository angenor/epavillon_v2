//! Se déconnecter ferme **cet** appareil, et lui seul (FR-006).
//!
//! C'est déjà le comportement — la révocation porte l'identifiant de session —,
//! et ce test existe pour qu'il le reste. Une déconnexion qui couperait tout
//! ferait sortir quelqu'un de son téléphone parce qu'il a fermé son navigateur
//! au bureau, et le défaut ne se verrait qu'en salle de négociation.

mod commun;

use commun::{appareil_de_lapplication, appareil_du_site, connexion_avec, semer, Bac, Compte};
use identity::service::session;

const ADRESSE: &str = "awa.diallo@example.org";

#[tokio::test]
async fn la_deconnexion_dun_appareil_laisse_les_autres_ouverts() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;

    let telephone = connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false).await;
    let poste = connexion_avec(&bac, ADRESSE, appareil_du_site(), false).await;
    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 2);

    session::logout(&bac.state, &bac.ctx(), Some(&poste.refresh_token))
        .await
        .expect("déconnexion du poste");

    assert_eq!(commun::sessions_vivantes(&bac, personne).await, 1);
    assert_eq!(
        commun::acteur_resolu(&bac, &telephone.access_token).await,
        Some(personne),
        "le téléphone reste connecté"
    );
    assert_eq!(commun::acteur_resolu(&bac, &poste.access_token).await, None);
}

/// L'inverse, et il n'est pas symétrique par hasard : c'est le geste que fait la
/// personne qui prête son téléphone, et il ne doit rien couper ailleurs.
#[tokio::test]
async fn la_deconnexion_du_telephone_laisse_le_poste_ouvert() {
    let bac = Bac::monter().await;
    let personne = semer(&bac, Compte::actif(ADRESSE)).await;

    let telephone = connexion_avec(&bac, ADRESSE, appareil_de_lapplication(), false).await;
    let poste = connexion_avec(&bac, ADRESSE, appareil_du_site(), false).await;

    session::logout(&bac.state, &bac.ctx(), Some(&telephone.refresh_token))
        .await
        .expect("déconnexion du téléphone");

    assert_eq!(
        commun::acteur_resolu(&bac, &poste.access_token).await,
        Some(personne)
    );
}
