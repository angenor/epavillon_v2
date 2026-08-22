//! Harnais de base jetable, derrière la caractéristique `testing`.
//!
//! Principe X : aucun double en mémoire, aucun mock de base — la moitié des
//! invariants de ce projet vit dans PostgreSQL. Une **base modèle** est chargée
//! une fois depuis `docs/database/`, puis chaque test en fait une copie
//! (`CREATE DATABASE … TEMPLATE …`) qu'il détruit en sortant : quelques
//! dizaines de millisecondes, pour une base identique, triggers compris.

use figment::providers::Serialized;
use figment::Figment;
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgConnectOptions, PgConnection, PgPool, PgPoolOptions};
use sqlx::{AssertSqlSafe, ConnectOptions, Connection, Executor};
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;

/// Le nom du modèle porte l'empreinte du SQL qu'il contient.
///
/// C'est ce qui évite le piège que `CLAUDE.md` signale pour le conteneur :
/// modifier un fichier de `docs/database/` et continuer de tester contre
/// l'ancien schéma, sans qu'aucun message ne le dise. Ici, changer le SQL
/// change le nom, donc reconstruit le modèle — sans consigne à retenir.
const PREFIXE_MODELE: &str = "epavillon_test_template_";
/// Verrou consultatif : `cargo test` lance ses tests en parallèle, et deux
/// d'entre eux ne doivent pas charger le modèle en même temps.
const VERROU_MODELE: i64 = 0x_65_70_61_76_31;

pub struct TestDb {
    nom: String,
    url: String,
    url_admin: String,
    pool: PgPool,
}

impl TestDb {
    /// Crée une base jetable par recopie du modèle.
    ///
    /// Le modèle survit au processus : il vit dans le cluster. Sa présence est
    /// donc vérifiée à chaque appel, sous verrou consultatif — `cargo test`
    /// lance ses tests en parallèle et deux d'entre eux ne doivent pas le
    /// charger en même temps.
    pub async fn new() -> Self {
        // Les tests tournent sans le `.env` chargé par les binaires : le
        // harnais le charge lui-même, sinon chaque test devrait y penser.
        let _ = dotenvy::dotenv();
        let url_base = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL absente : les tests exigent une base réelle (principe X)");
        let url_admin = remplacer_base(&url_base, "postgres");

        let modele = assurer_modele(&url_admin).await;

        let nom = format!("epavillon_test_{}", Uuid::new_v4().simple());
        let mut admin = connecter(&url_admin).await;
        admin
            .execute(AssertSqlSafe(format!(
                r#"CREATE DATABASE "{nom}" TEMPLATE "{modele}""#
            )))
            .await
            .expect("recopie de la base modèle");
        admin.close().await.ok();

        let url = remplacer_base(&url_base, &nom);
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .expect("connexion à la base jetable");

        Self {
            nom,
            url,
            url_admin,
            pool,
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn name(&self) -> &str {
        &self.nom
    }

    /// URL d'administration du cluster — la même que celle dont le harnais se
    /// sert, plutôt qu'une substitution de chaîne qui casse dès que la base ne
    /// s'appelle plus `epavillon`.
    pub fn admin_url(&self) -> &str {
        &self.url_admin
    }

    pub fn db(&self) -> crate::db::Db {
        crate::db::Db::from_pool(self.pool.clone())
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let url_admin = self.url_admin.clone();
        let nom = self.nom.clone();

        // La suppression doit être terminée quand `drop` rend la main : un test
        // qui vérifie la disparition de sa base ne peut pas l'observer sinon.
        //
        // Le pool n'est PAS fermé ici : sa fermeture s'appuie sur le runtime du
        // test, que ce fil bloque — l'attente ne se dénouerait jamais.
        // `DROP DATABASE … WITH (FORCE)` termine les connexions restantes côté
        // serveur, ce qui règle le même problème sans interblocage.
        let _ = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime de nettoyage");
            rt.block_on(async move {
                if let Ok(mut admin) = PgConnection::connect(&url_admin).await {
                    let _ = admin
                        .execute(AssertSqlSafe(format!(
                            r#"DROP DATABASE IF EXISTS "{nom}" WITH (FORCE)"#
                        )))
                        .await;
                    admin.close().await.ok();
                }
            });
        })
        .join();
    }
}

fn remplacer_base(url: &str, base: &str) -> String {
    let options = PgConnectOptions::from_str(url)
        .expect("DATABASE_URL illisible")
        .database(base);
    options.to_url_lossy().to_string()
}

async fn connecter(url: &str) -> PgConnection {
    PgConnection::connect(url)
        .await
        .expect("connexion d'administration")
}

async fn assurer_modele(url_admin: &str) -> String {
    let modele = format!("{PREFIXE_MODELE}{}", empreinte_du_schema());
    let mut admin = connecter(url_admin).await;

    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(VERROU_MODELE)
        .execute(&mut admin)
        .await
        .expect("verrou du modèle");

    let existe: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(&modele)
            .fetch_one(&mut admin)
            .await
            .expect("recherche de la base modèle");

    if !existe {
        supprimer_modeles_perimes(&mut admin, &modele).await;

        admin
            .execute(AssertSqlSafe(format!(r#"CREATE DATABASE "{modele}""#)))
            .await
            .expect("création de la base modèle");

        // CREATE DATABASE n'est pas transactionnel : un chargement interrompu
        // laisserait un modèle amputé que plus rien ne reconstruirait, et le
        // développeur corrigerait son SQL sans jamais voir sa correction.
        let url_modele = remplacer_base(url_admin, &modele);
        if let Err(e) = charger_schema(&url_modele).await {
            admin
                .execute(AssertSqlSafe(format!(
                    r#"DROP DATABASE IF EXISTS "{modele}" WITH (FORCE)"#
                )))
                .await
                .ok();
            panic!("{e}");
        }
    }

    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(VERROU_MODELE)
        .execute(&mut admin)
        .await
        .expect("libération du verrou");
    admin.close().await.ok();

    modele
}

/// Les modèles d'une version antérieure du schéma n'ont plus de lecteur : les
/// garder ferait grossir le cluster d'une base complète par modification du SQL.
async fn supprimer_modeles_perimes(admin: &mut PgConnection, courant: &str) {
    let anciens: Vec<String> = sqlx::query_scalar(
        "SELECT datname FROM pg_database WHERE datname LIKE $1 AND datname <> $2",
    )
    .bind(format!("{PREFIXE_MODELE}%"))
    .bind(courant)
    .fetch_all(&mut *admin)
    .await
    .unwrap_or_default();

    for ancien in anciens {
        admin
            .execute(AssertSqlSafe(format!(
                r#"DROP DATABASE IF EXISTS "{ancien}" WITH (FORCE)"#
            )))
            .await
            .ok();
    }
}

fn empreinte_du_schema() -> String {
    let mut hacheur = Sha256::new();
    for fichier in fichiers_sql() {
        hacheur.update(fichier.file_name().unwrap_or_default().as_encoded_bytes());
        hacheur.update(std::fs::read(&fichier).unwrap_or_default());
    }
    hex_court(&hacheur.finalize())
}

fn hex_court(octets: &[u8]) -> String {
    octets[..6].iter().map(|o| format!("{o:02x}")).collect()
}

fn fichiers_sql() -> Vec<PathBuf> {
    let mut fichiers: Vec<PathBuf> = std::fs::read_dir(repertoire_sql())
        .expect("docs/database/ introuvable")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "sql"))
        .collect();
    fichiers.sort();
    fichiers
}

/// Les fichiers sont joués dans l'ordre de leur numérotation, exactement comme
/// le fait le conteneur de développement. Ce ne sont pas des migrations : faire
/// dépendre les tests d'une coïncidence de nommage avec `sqlx migrate` serait
/// fragile (research.md § R15).
async fn charger_schema(url_modele: &str) -> Result<(), String> {
    let mut conn = connecter(url_modele).await;
    for fichier in fichiers_sql() {
        let sql = std::fs::read_to_string(&fichier)
            .map_err(|e| format!("lecture de {} : {e}", fichier.display()))?;
        sqlx::raw_sql(AssertSqlSafe(sql))
            .execute(&mut conn)
            .await
            .map_err(|e| format!("chargement de {} : {e}", fichier.display()))?;
    }
    conn.close().await.ok();
    Ok(())
}

fn repertoire_sql() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/database")
        .canonicalize()
        .expect("docs/database/ introuvable depuis backend/crates/kernel")
}

/// Racine jetable du stockage sur fichiers, sous le répertoire temporaire du
/// système. Elle n'est pas effacée en sortie : quelques kilo-octets par test,
/// et le contenu déposé est la seule matière de diagnostic d'un test rouge.
fn racine_media_jetable() -> String {
    std::env::temp_dir()
        .join(format!("epavillon-media-{}", Uuid::new_v4().simple()))
        .to_string_lossy()
        .into_owned()
}

/// Configuration minimale d'un test, passée par **la même validation qu'au
/// démarrage** : un test ne doit pas pouvoir s'appuyer sur un réglage que la
/// production refuserait. Seule la base change d'un test à l'autre.
pub fn test_config(database_url: &str) -> crate::config::Config {
    // 43 signes de base64 sans remplissage : les 32 octets que la clé Ed25519
    // exige. Fixe, pour qu'un jeton émis dans un test se vérifie dans le même.
    const CLE_DE_TEST: &str = "ZXBhdmlsbG9uLWNsZS1kZS10ZXN0LWVkMjU1MTktYjE";

    let valeurs = serde_json::json!({
        "database_url": database_url,
        "app_public_url": "http://localhost:3000",
        "auth_signing_key": CLE_DE_TEST,
        "auth_cookie_secure": false,
        "mail_transport": "relay",
        "mail_relay_url": "http://localhost:3000/api/internal/mail",
        "mail_relay_token": "secret-de-test",
        // Le stockage des tests est le SYSTÈME DE FICHIERS, jamais Garage
        // (B6, R7) : `make check-db` fait `down -v`, ce qui efface le layout du
        // stockage objet — des tests qui le frapperaient échoueraient après
        // chaque vérification complète, et on prendrait l'habitude de les
        // sauter. La racine est unique par test : deux tests parallèles ne
        // doivent pas se voir.
        "media_storage": "filesystem",
        "media_fs_root": racine_media_jetable(),
        "media_scanner": "none",
        // Le jeton d'ingestion est RENSEIGNÉ ici : sans lui, la route de
        // délivrabilité n'est pas montée et le test qui la frappe lirait un 404
        // sans savoir si c'est le montage ou le chemin qui manque.
        "mail_webhook_token": "jeton-webhook-de-test",
    });

    crate::config::Config::from_figment(Figment::from(Serialized::defaults(valeurs)))
        .expect("configuration de test")
}
