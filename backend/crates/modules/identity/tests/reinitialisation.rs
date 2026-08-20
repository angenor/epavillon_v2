//! Le cycle complet de réinitialisation, et ce qu'il doit garantir.
//!
//! Trois choses se vérifient ici et nulle part ailleurs : la réponse ne varie
//! pas selon que l'adresse est connue (FR-036), le jeton est **revérifié à
//! l'envoi** et pas seulement à l'affichage (FR-042), et un mot de passe changé
//! coupe tout ce qui tenait à l'ancien (FR-043).

mod commun;

use commun::{semer, sessions_vivantes, Bac, Compte, MOT_DE_PASSE};
use identity::domain::login::LoginOutcome;
use identity::domain::token::{PasswordResetOutcome, TokenCheckOutcome, TokenRejection};
use identity::service::auth::{login, LoginRequest};
use identity::service::password_reset;
use identity::service::session::Device;

const ADRESSE: &str = "awa.diallo@example.org";
const NOUVEAU: &str = "Ouagadougou2027";

/// Le jeton en clair tel qu'il partirait dans le courriel : la file est le seul
/// endroit où il existe — la base n'en garde que l'empreinte (FR-038).
async fn jeton_en_file(bac: &Bac) -> String {
    sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'identity.send_password_reset_email'
          ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail d'envoi")
    .expect("le travail porte le jeton en clair")
}

async fn demander(bac: &Bac, email: &str) {
    let issue = password_reset::request(&bac.state, &bac.ctx(), email)
        .await
        .expect("demande de réinitialisation");
    assert_eq!(issue.status, "sent");
}

async fn confirmer(bac: &Bac, jeton: &str, mot_de_passe: &str) -> PasswordResetOutcome {
    password_reset::confirm(&bac.state, &bac.ctx(), jeton, mot_de_passe)
        .await
        .expect("enregistrement du mot de passe")
}

async fn tenter(bac: &Bac, email: &str, mot_de_passe: &str) -> LoginOutcome {
    login(
        &bac.state,
        &bac.ctx(),
        LoginRequest {
            email,
            password: mot_de_passe,
            remember_me: false,
            device: Device::default(),
        },
    )
    .await
    .expect("connexion")
    .outcome
}

fn refus(issue: &PasswordResetOutcome) -> TokenRejection {
    match issue {
        PasswordResetOutcome::Rejected { reason } => *reason,
        PasswordResetOutcome::Reset { .. } => panic!("le jeton devait être refusé"),
    }
}

#[tokio::test]
async fn le_cycle_complet_remplace_le_mot_de_passe() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    demander(&bac, ADRESSE).await;
    let jeton = jeton_en_file(&bac).await;

    match password_reset::check(&bac.state, &jeton)
        .await
        .expect("contrôle du jeton")
    {
        TokenCheckOutcome::Valid { email } => assert_eq!(email, ADRESSE),
        autre => panic!("le jeton devait être valide : {autre:?}"),
    }

    match confirmer(&bac, &jeton, NOUVEAU).await {
        PasswordResetOutcome::Reset { email } => assert_eq!(email, ADRESSE),
        autre => panic!("issue inattendue : {autre:?}"),
    }

    assert!(
        matches!(
            tenter(&bac, ADRESSE, NOUVEAU).await,
            LoginOutcome::Authenticated { .. }
        ),
        "le nouveau mot de passe doit ouvrir la session"
    );
    assert!(
        matches!(
            tenter(&bac, ADRESSE, MOT_DE_PASSE).await,
            LoginOutcome::InvalidCredentials
        ),
        "l'ancien mot de passe ne vaut plus rien"
    );

    let evenements = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.outbox_events
            WHERE event_type = 'identity.account.password_changed'
              AND payload ->> 'channel' = 'reset'"#
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(evenements, 1);
}

/// FR-036. Le comptage porte sur **l'adresse visée** : `900_seed.sql` sème déjà
/// une personne, et un comptage global mesurerait le semis.
#[tokio::test]
async fn une_adresse_inconnue_rend_la_meme_reponse_sans_rien_mettre_en_file() {
    let bac = Bac::monter().await;

    demander(&bac, "personne.inconnue@example.org").await;

    let en_file = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM platform.jobs
            WHERE task = 'identity.send_password_reset_email'"#
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(
        en_file, 0,
        "aucun courriel ne part vers une adresse inconnue"
    );

    let jetons = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM identity.one_time_tokens
            WHERE purpose = 'password_reset'"#
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("comptage");
    assert_eq!(jetons, 0);
}

/// FR-042, le cas de l'onglet ouvert la veille au soir : le contrôle a dit
/// « valide », l'envoi arrive une heure trop tard.
#[tokio::test]
async fn un_jeton_perime_entre_laffichage_et_lenvoi_est_refuse() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    demander(&bac, ADRESSE).await;
    let jeton = jeton_en_file(&bac).await;

    assert!(matches!(
        password_reset::check(&bac.state, &jeton)
            .await
            .expect("contrôle"),
        TokenCheckOutcome::Valid { .. }
    ));

    sqlx::query!("UPDATE identity.one_time_tokens SET expires_at = now() - interval '1 minute'")
        .execute(bac.base.pool())
        .await
        .expect("péremption forcée");

    assert_eq!(
        refus(&confirmer(&bac, &jeton, NOUVEAU).await),
        TokenRejection::Expired
    );
    assert!(
        matches!(
            tenter(&bac, ADRESSE, MOT_DE_PASSE).await,
            LoginOutcome::Authenticated { .. }
        ),
        "un enregistrement refusé ne touche pas au mot de passe"
    );
}

/// FR-043 : les sessions ouvertes ailleurs tombent avec l'ancien secret.
#[tokio::test]
async fn la_reinitialisation_coupe_toutes_les_sessions() {
    let bac = Bac::monter().await;
    let person_id = semer(&bac, Compte::actif(ADRESSE)).await;

    let session = commun::connexion(&bac, ADRESSE).await;
    commun::connexion(&bac, ADRESSE).await;
    assert_eq!(sessions_vivantes(&bac, person_id).await, 2);

    demander(&bac, ADRESSE).await;
    let jeton = jeton_en_file(&bac).await;
    confirmer(&bac, &jeton, NOUVEAU).await;

    assert_eq!(sessions_vivantes(&bac, person_id).await, 0);
    assert_eq!(
        commun::acteur_resolu(&bac, &session.access_token).await,
        None,
        "le jeton d'accès d'avant ne résout plus personne"
    );

    let motifs: Vec<Option<String>> = commun::sessions(&bac, person_id)
        .await
        .into_iter()
        .map(|(_, motif)| motif)
        .collect();
    assert!(
        motifs
            .iter()
            .all(|m| m.as_deref() == Some("password_changed")),
        "le motif de révocation dit pourquoi : {motifs:?}"
    );
}

/// FR-043, l'autre moitié : quelqu'un qui s'est verrouillé à force d'essayer
/// resterait bloqué un quart d'heure avec le mot de passe qu'il vient de
/// choisir.
#[tokio::test]
async fn le_compteur_dechecs_et_le_verrou_repartent_de_zero() {
    let bac = Bac::monter().await;
    let person_id = semer(&bac, Compte::actif(ADRESSE)).await;

    sqlx::query!(
        "UPDATE identity.accounts
            SET failed_attempts = 5, locked_until = now() + interval '15 minutes'
          WHERE person_id = $1",
        person_id
    )
    .execute(bac.base.pool())
    .await
    .expect("verrou posé");

    demander(&bac, ADRESSE).await;
    let jeton = jeton_en_file(&bac).await;
    confirmer(&bac, &jeton, NOUVEAU).await;

    let compte = sqlx::query!(
        "SELECT failed_attempts, locked_until FROM identity.accounts WHERE person_id = $1",
        person_id
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("relecture du compte");
    assert_eq!(compte.failed_attempts, 0);
    assert!(compte.locked_until.is_none());

    assert!(matches!(
        tenter(&bac, ADRESSE, NOUVEAU).await,
        LoginOutcome::Authenticated { .. }
    ));
}

/// FR-041 : le lien ne sert qu'une fois, et le second clic lit « déjà utilisé »
/// plutôt que « périmé ».
#[tokio::test]
async fn le_jeton_ne_sert_quune_fois() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    demander(&bac, ADRESSE).await;
    let jeton = jeton_en_file(&bac).await;
    confirmer(&bac, &jeton, NOUVEAU).await;

    assert_eq!(
        refus(&confirmer(&bac, &jeton, "Cotonou2027").await),
        TokenRejection::AlreadyUsed
    );
    assert!(
        matches!(
            password_reset::check(&bac.state, &jeton)
                .await
                .expect("contrôle"),
            TokenCheckOutcome::Rejected {
                reason: TokenRejection::AlreadyUsed
            }
        ),
        "le contrôle préalable suit le même ordre de refus"
    );
}

/// FR-040 : demander un second lien tue le premier. Deux liens valides pour le
/// même compte, c'est une surface d'attaque de plus pour aucun service rendu.
#[tokio::test]
async fn un_second_lien_invalide_le_premier() {
    let bac = Bac::monter().await;
    semer(&bac, Compte::actif(ADRESSE)).await;

    demander(&bac, ADRESSE).await;
    let premier = jeton_en_file(&bac).await;

    demander(&bac, ADRESSE).await;
    let second = jeton_en_file(&bac).await;
    assert_ne!(premier, second);

    assert_eq!(
        refus(&confirmer(&bac, &premier, NOUVEAU).await),
        TokenRejection::Expired,
        "l'ancien lien est périmé, pas « déjà utilisé » : un plus récent vient d'arriver"
    );
    assert!(matches!(
        confirmer(&bac, &second, NOUVEAU).await,
        PasswordResetOutcome::Reset { .. }
    ));
}

/// La finalité entre dans le filtre : un jeton de vérification d'adresse
/// présenté à la réinitialisation est **invalide**, jamais « déjà utilisé ».
#[tokio::test]
async fn un_jeton_dune_autre_finalite_est_invalide() {
    let bac = Bac::monter().await;
    let person_id = semer(&bac, Compte::non_verifie(ADRESSE)).await;

    identity::service::registration::resend_verification(&bac.state, &bac.ctx(), ADRESSE)
        .await
        .expect("renvoi du lien de vérification");

    let jeton = sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = 'identity.send_verification_email'
          ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail d'envoi")
    .expect("le travail porte le jeton");

    assert_eq!(
        refus(&confirmer(&bac, &jeton, NOUVEAU).await),
        TokenRejection::Invalid
    );

    let empreinte = sqlx::query_scalar!(
        "SELECT password_hash FROM identity.accounts WHERE person_id = $1",
        person_id
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("relecture du compte");
    assert!(
        bac.state
            .passwords()
            .verify(MOT_DE_PASSE, &empreinte.expect("empreinte")),
        "le mot de passe d'origine est intact"
    );
}
