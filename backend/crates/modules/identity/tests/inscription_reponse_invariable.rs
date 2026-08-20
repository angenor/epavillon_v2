//! FR-035 : adresse libre et adresse déjà prise rendent **la même réponse**.
//!
//! C'est le formulaire d'inscription qui, dans la v1, disait qui avait un
//! compte. Ici la différence est reportée sur le courriel — un lien de
//! vérification d'un côté, un rappel de compte existant de l'autre — et le test
//! vérifie les deux moitiés : la réponse identique, et le courriel différent.

mod commun;

use commun::{semer, Bac, Compte, MOT_DE_PASSE};
use identity::service::registration::{self, RegisterRequest};
use kernel::error::ErrorCode;

const ADRESSE: &str = "awa.diallo@example.org";

fn demande(email: &str) -> RegisterRequest<'_> {
    RegisterRequest {
        first_name: "Awa",
        last_name: "Diallo",
        email,
        country_id: None,
        password: MOT_DE_PASSE,
        preferred_locale: "fr",
        timezone: "Africa/Dakar",
    }
}

/// Les travaux mis en file, leur tâche et ce qu'ils portent.
async fn travaux(bac: &Bac) -> Vec<(String, serde_json::Value)> {
    sqlx::query!("SELECT task, payload FROM platform.jobs ORDER BY created_at")
        .fetch_all(bac.base.pool())
        .await
        .expect("lecture de la file")
        .into_iter()
        .map(|l| (l.task, l.payload))
        .collect()
}

#[tokio::test]
async fn une_adresse_libre_et_une_adresse_prise_rendent_la_meme_reponse() {
    let bac = Bac::monter().await;

    let libre = registration::register(&bac.state, &bac.ctx(), demande(ADRESSE))
        .await
        .expect("inscription sur adresse libre");

    let prise = registration::register(&bac.state, &bac.ctx(), demande(ADRESSE))
        .await
        .expect("inscription sur adresse prise");

    assert_eq!(
        serde_json::to_value(&libre).unwrap(),
        serde_json::to_value(&prise).unwrap(),
        "les deux réponses doivent être indiscernables, champ pour champ"
    );
    assert_eq!(libre.status, "verification_sent");

    // Et rien n'a été créé une seconde fois.
    let personnes = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM identity.people WHERE primary_email = $1::text::platform.email",
        ADRESSE
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage des personnes");
    assert_eq!(personnes, 1);
}

/// La différence est dans le courriel, et elle est entière : la seconde
/// tentative ne produit **aucun lien**. Sans quoi n'importe qui ferait envoyer
/// un lien vers un compte qui n'est pas le sien en le « réinscrivant ».
#[tokio::test]
async fn le_courriel_differe_et_le_rappel_ne_porte_aucun_lien() {
    let bac = Bac::monter().await;

    registration::register(&bac.state, &bac.ctx(), demande(ADRESSE))
        .await
        .expect("première inscription");
    registration::register(&bac.state, &bac.ctx(), demande(ADRESSE))
        .await
        .expect("seconde inscription");

    let files = travaux(&bac).await;
    let taches: Vec<_> = files.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        taches,
        vec![
            "identity.send_verification_email",
            "identity.send_existing_account_notice"
        ]
    );

    assert!(
        files[0].1.get("token").is_some(),
        "la vérification porte le jeton en clair — c'est le seul chemin qu'il emprunte"
    );
    assert!(
        files[1].1.get("token").is_none(),
        "le rappel de compte existant ne doit porter aucun jeton"
    );
}

/// Le jeton en clair ne vit que dans la charge utile du travail : ni la table
/// des jetons, ni l'outbox ne le connaissent.
#[tokio::test]
async fn aucun_jeton_en_clair_hors_de_la_file() {
    let bac = Bac::monter().await;

    registration::register(&bac.state, &bac.ctx(), demande(ADRESSE))
        .await
        .expect("inscription");

    let files = travaux(&bac).await;
    let clair = files[0].1["token"]
        .as_str()
        .expect("jeton en clair")
        .to_owned();

    let dans_les_jetons = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM identity.one_time_tokens
          WHERE payload::text LIKE '%' || $1 || '%'",
        clair
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("recherche dans les jetons");
    assert_eq!(dans_les_jetons, 0, "la base ne garde que l'empreinte");

    let dans_loutbox = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM platform.outbox_events
          WHERE payload::text LIKE '%' || $1 || '%'",
        clair
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("recherche dans l'outbox");
    assert_eq!(
        dans_loutbox, 0,
        "l'outbox est durable et rejouable : aucun secret n'y entre"
    );

    let evenements = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM platform.outbox_events
          WHERE event_type = 'identity.person.registered'"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage des événements");
    assert_eq!(evenements, 1, "un changement d'état, un événement");
}

/// Un mot de passe trop court est une faute de saisie : elle sort en 422 et ne
/// divulgue rien. C'est le seul refus que l'inscription rend.
#[tokio::test]
async fn un_mot_de_passe_trop_faible_est_refuse_avant_tout() {
    let bac = Bac::monter().await;

    let refus = registration::register(
        &bac.state,
        &bac.ctx(),
        RegisterRequest {
            password: "court",
            ..demande(ADRESSE)
        },
    )
    .await
    .expect_err("un mot de passe trop faible est refusé");

    assert_eq!(refus.code, ErrorCode::IdentityPasswordTooWeak);
    assert_eq!(refus.field.as_deref(), Some("password"));

    // Compté sur l'adresse visée : `900_seed.sql` sème le compte technique
    // pivot, et un comptage global mesurerait le semis.
    let personnes = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM identity.people
          WHERE primary_email = $1::text::platform.email",
        ADRESSE
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(personnes, 0, "rien n'a été créé");
}

/// Une adresse connue **sans compte mot de passe** — quelqu'un saisi comme
/// intervenant, jamais inscrit — suit le même chemin : réponse invariable, et
/// rappel plutôt que création. La distinction personne / compte ne change rien
/// à ce que le formulaire dit.
#[tokio::test]
async fn une_personne_sans_compte_recoit_le_rappel_elle_aussi() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::sans_mot_de_passe(ADRESSE)).await;

    let reponse = registration::register(&bac.state, &bac.ctx(), demande(ADRESSE))
        .await
        .expect("inscription sur une personne connue sans compte");
    assert_eq!(reponse.status, "verification_sent");

    let taches: Vec<_> = travaux(&bac).await.into_iter().map(|(t, _)| t).collect();
    assert_eq!(taches, vec!["identity.send_existing_account_notice"]);

    let comptes = sqlx::query_scalar!(
        "SELECT count(*) AS \"n!\" FROM identity.accounts a
           JOIN identity.people p ON p.id = a.person_id
          WHERE p.primary_email = $1::text::platform.email",
        ADRESSE
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage des comptes");
    assert_eq!(
        comptes, 0,
        "aucun compte n'est créé dans le dos de personne"
    );
}
