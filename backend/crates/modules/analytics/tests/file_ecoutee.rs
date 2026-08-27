//! **Le worker écoute « analytics » du seul fait que le gestionnaire la
//! nomme.**
//!
//! `platform.claim_jobs()` filtre strictement sur la file : un travail déposé
//! dans une file inécoutée s'empile **sans erreur, sans trace, et sans que rien
//! ne l'exécute jamais**. C'est le seul défaut du jalon qu'aucun message ne
//! signalerait.

mod commun;

use commun::*;

#[tokio::test]
async fn le_gestionnaire_declare_la_file_analytics() {
    let bac = Bac::monter().await;
    let gestionnaires = analytics::job_handlers(bac.db(), &bac.config);

    assert_eq!(gestionnaires.len(), 1, "un travail différé, et un seul");
    let refresh = &gestionnaires[0];
    assert_eq!(refresh.queue(), "analytics");
    assert_eq!(refresh.task(), "analytics.refresh_all");
    assert!(
        !refresh.carries_secret(),
        "la charge utile est la seule matière de diagnostic d'un rafraîchissement mort"
    );
}
