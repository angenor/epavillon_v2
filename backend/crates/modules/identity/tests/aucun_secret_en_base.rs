//! SC-009 : après un cycle complet, la base ne contient **aucun secret
//! utilisable**.
//!
//! Trois tests cherchent déjà le jeton en clair, chacun sur un chemin précis —
//! la table des jetons, l'outbox, la charge utile du rappel de compte existant.
//! Celui-ci ne les redouble pas : il joue le **cycle entier** — inscription,
//! envoi remis, vérification, connexion, réinitialisation — puis fouille
//! **toutes les colonnes textuelles** des schémas `identity` et `platform`, y
//! compris celles que personne n'a pensé à regarder.
//!
//! C'est la différence entre « le jeton n'est pas là où je l'ai cherché » et
//! « le jeton n'est nulle part ».

mod commun;

use commun::{Bac, MOT_DE_PASSE};
use identity::domain::token::VerifyEmailOutcome;
use identity::service::registration::RegisterRequest;
use identity::service::{password_reset, registration};
use kernel::jobs::{self, DEFAULT_QUEUE};
use kernel::RequestContext;
use sqlx::{AssertSqlSafe, Row};

const ADRESSE: &str = "awa.diallo@example.org";
const NOUVEAU_MOT_DE_PASSE: &str = "Kinshasa2028!";

/// Une fonction de fouille, créée dans la base jetable : elle parcourt le
/// catalogue et interroge chaque colonne textuelle. Écrite en SQL parce que
/// c'est le catalogue qui sait quelles colonnes existent — une liste tenue à la
/// main dans le test vieillirait à la première colonne ajoutée, et le test
/// passerait au vert en ne regardant plus au bon endroit.
const FOUILLE: &str = r#"
CREATE FUNCTION public.chercher_le_secret(p_secret text)
RETURNS TABLE (schema_name text, table_name text, column_name text)
LANGUAGE plpgsql AS $$
DECLARE r record; n bigint;
BEGIN
    FOR r IN
        SELECT c.table_schema, c.table_name, c.column_name
          FROM information_schema.columns c
          JOIN information_schema.tables t
            ON t.table_schema = c.table_schema AND t.table_name = c.table_name
         WHERE t.table_type = 'BASE TABLE'
           AND c.table_schema IN ('identity', 'platform')
           AND c.data_type IN ('text', 'character varying', 'jsonb', 'json')
    LOOP
        EXECUTE format('SELECT count(*) FROM %I.%I WHERE %I::text LIKE %L',
                       r.table_schema, r.table_name, r.column_name,
                       '%' || p_secret || '%')
           INTO n;
        IF n > 0 THEN
            schema_name := r.table_schema;
            table_name  := r.table_name;
            column_name := r.column_name;
            RETURN NEXT;
        END IF;
    END LOOP;
END; $$;
"#;

/// La requête n'est pas une macro : la fonction n'existe que dans la base
/// jetable, et SQLx vérifie ses macros contre la base de développement — elle y
/// échouerait à la compilation.
async fn ou_se_trouve(bac: &Bac, secret: &str) -> Vec<String> {
    sqlx::query("SELECT schema_name, table_name, column_name FROM public.chercher_le_secret($1)")
        .bind(secret)
        .fetch_all(bac.base.pool())
        .await
        .expect("fouille du catalogue")
        .into_iter()
        .map(|l| {
            format!(
                "{}.{}.{}",
                l.get::<String, _>("schema_name"),
                l.get::<String, _>("table_name"),
                l.get::<String, _>("column_name")
            )
        })
        .collect()
}

/// Le clair du dernier travail d'envoi d'une tâche donnée.
async fn jeton_en_file(bac: &Bac, tache: &str) -> String {
    sqlx::query_scalar!(
        "SELECT payload ->> 'token' FROM platform.jobs
          WHERE task = $1 ORDER BY created_at DESC LIMIT 1",
        tache
    )
    .fetch_one(bac.base.pool())
    .await
    .expect("lecture du travail d'envoi")
    .expect("le travail porte le jeton en clair")
}

/// Ce que le worker fait d'un envoi réussi : la charge utile est **vidée**.
/// Sans ce passage, le cycle n'est pas complet — le jeton vivrait encore
/// légitimement dans la file, en attente d'envoi.
async fn remettre_tous_les_courriels(bac: &Bac) {
    let db = bac.db();
    let mut tx = db
        .write(&RequestContext::background("test-worker"))
        .await
        .expect("transaction");
    let travaux = jobs::claim(&mut tx, DEFAULT_QUEUE, "worker-de-test", 50)
        .await
        .expect("réservation");
    for travail in travaux {
        jobs::succeed(&mut tx, travail.id)
            .await
            .expect("réussite du travail");
    }
    tx.commit().await.expect("validation");
}

#[tokio::test]
async fn apres_un_cycle_complet_la_base_ne_porte_aucun_secret_utilisable() {
    let bac = Bac::monter().await;
    sqlx::raw_sql(AssertSqlSafe(FOUILLE))
        .execute(bac.base.pool())
        .await
        .expect("création de la fonction de fouille");

    // 1. Inscription — le jeton de vérification naît, en clair, dans la file.
    registration::register(
        &bac.state,
        &bac.ctx(),
        RegisterRequest {
            first_name: "Awa",
            last_name: "Diallo",
            email: ADRESSE,
            country_id: None,
            password: MOT_DE_PASSE,
            preferred_locale: "fr",
            timezone: "Africa/Dakar",
        },
    )
    .await
    .expect("inscription");

    let jeton_verification = jeton_en_file(&bac, "identity.send_verification_email").await;
    remettre_tous_les_courriels(&bac).await;

    // 2. Vérification de l'adresse.
    let issue = registration::verify_email(&bac.state, &bac.ctx(), &jeton_verification)
        .await
        .expect("vérification");
    assert!(matches!(issue, VerifyEmailOutcome::Verified { .. }));

    // 3. Connexion — deux jetons de session en clair sortent vers le client.
    let session = commun::connexion(&bac, ADRESSE).await;

    // 4. Réinitialisation — un second jeton de lien, et un nouveau mot de passe.
    password_reset::request(&bac.state, &bac.ctx(), ADRESSE)
        .await
        .expect("demande de réinitialisation");
    let jeton_reinitialisation = jeton_en_file(&bac, "identity.send_password_reset_email").await;
    remettre_tous_les_courriels(&bac).await;

    password_reset::confirm(
        &bac.state,
        &bac.ctx(),
        &jeton_reinitialisation,
        NOUVEAU_MOT_DE_PASSE,
    )
    .await
    .expect("réinitialisation");

    // ------------------------------------------------------------------
    // Le cycle est joué. Rien de ce qui suit ne doit se trouver en base.
    // ------------------------------------------------------------------
    for (quoi, secret) in [
        ("le mot de passe d'inscription", MOT_DE_PASSE),
        ("le mot de passe réinitialisé", NOUVEAU_MOT_DE_PASSE),
        ("le jeton d'accès", session.access_token.as_str()),
        (
            "le jeton de rafraîchissement",
            session.refresh_token.as_str(),
        ),
        ("le lien de vérification", jeton_verification.as_str()),
        (
            "le lien de réinitialisation",
            jeton_reinitialisation.as_str(),
        ),
    ] {
        let trouve = ou_se_trouve(&bac, secret).await;
        assert!(
            trouve.is_empty(),
            "{quoi} se retrouve en base, dans : {trouve:?}"
        );
    }
}

/// L'autre moitié de SC-009, et celle qui se perd : un travail **réussi** garde
/// sa trace — qu'un courriel est parti, quand, après combien d'essais — sans
/// garder son contenu.
#[tokio::test]
async fn la_charge_utile_dun_travail_reussi_est_videe() {
    let bac = Bac::monter().await;

    registration::register(
        &bac.state,
        &bac.ctx(),
        RegisterRequest {
            first_name: "Awa",
            last_name: "Diallo",
            email: ADRESSE,
            country_id: None,
            password: MOT_DE_PASSE,
            preferred_locale: "fr",
            timezone: "Africa/Dakar",
        },
    )
    .await
    .expect("inscription");

    let jeton = jeton_en_file(&bac, "identity.send_verification_email").await;
    assert!(!jeton.is_empty(), "le jeton voyage bien par la file");

    remettre_tous_les_courriels(&bac).await;

    let charges = sqlx::query!(
        r#"SELECT status::text AS "status!", payload::text AS "payload!", completed_at
             FROM platform.jobs"#
    )
    .fetch_all(bac.base.pool())
    .await
    .expect("relecture de la file");

    assert!(!charges.is_empty());
    for travail in charges {
        assert_eq!(travail.status, "succeeded");
        assert_eq!(
            travail.payload, "{}",
            "la réussite vide la charge utile, qui portait le jeton en clair"
        );
        assert!(
            travail.completed_at.is_some(),
            "la trace reste : on sait qu'un courriel est parti, et quand"
        );
    }
}
