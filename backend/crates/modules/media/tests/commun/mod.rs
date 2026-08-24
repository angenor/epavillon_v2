//! Ce que les tests d'intégration du module partagent : une base jetable, un
//! état de module, et **de quoi déposer un vrai fichier**.
//!
//! Aucun double en mémoire (principe X) : chaque test travaille sur une vraie
//! base, recopiée d'un modèle chargé depuis `docs/database/`.
//!
//! # Le stockage des tests est le SYSTÈME DE FICHIERS, jamais Garage
//!
//! `make check-db` exécute `down -v`, ce qui efface le layout du stockage
//! objet : des tests qui le frapperaient échoueraient après chaque vérification
//! complète, et l'on prendrait l'habitude de les sauter (B6, R7). Le stockage
//! sur fichiers exerce pourtant **tout le reste** — le service entier, tout le
//! SQL, la déduplication, les quotas, le rattachement, la fabrication des
//! déclinaisons. Seule la signature S3 lui échappe, et elle a ses propres
//! vecteurs d'épreuve (`tests/signature.rs`).
//!
//! `kernel::testing::test_config` pose déjà `MEDIA_STORAGE=filesystem` avec une
//! racine unique par test : deux tests parallèles ne se voient pas.
//!
//! # Les images sont VRAIES, et c'est nécessaire
//!
//! Le relevé des dimensions est ce que ce module éprouve : un PNG écrit à la
//! main ne serait pas décodable, et il n'y aurait rien à mesurer. Elles sont
//! donc encodées par la même bibliothèque que le worker.
//!
//! # Pourquoi la **vraie application** n'est pas montée ici
//!
//! La monter demanderait au crate `media` une dépendance de développement vers
//! `api` — qui dépend d'`identity`, d'`org`, d'`event`, de `programme` et
//! d'`engagement`. Le contrôle bloquant du jalon,
//! `cargo tree -p media | grep -E 'engagement|identity|org|event|programme'`,
//! doit ne **rien** rendre : `cargo tree` liste aussi les dépendances de
//! développement, et cette arête le ferait échouer.
//!
//! Les tests qui frappent les routes sur l'application entière vivent donc dans
//! `crates/api/tests/`, exactement là où B2 a mis les siens.

#![allow(dead_code)]

use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::testing::TestDb;
use kernel::Db;
use media::service::upload::MetadonneesDepot;
use media::state::MediaState;
use std::sync::Arc;
use uuid::Uuid;

pub struct Bac {
    pub base: TestDb,
    pub state: MediaState,
    pub config: Arc<Config>,
}

impl Bac {
    pub async fn monter() -> Self {
        let base = TestDb::new().await;
        let config = Arc::new(kernel::testing::test_config(base.url()));
        let state = MediaState::new(base.db(), config.clone());

        Self {
            base,
            state,
            config,
        }
    }

    pub fn db(&self) -> Db {
        self.base.db()
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        self.base.pool()
    }

    pub fn ctx(&self) -> RequestContext {
        RequestContext::new(format!("test-{}", Uuid::now_v7()), "fr")
    }
}

// -----------------------------------------------------------------------------
// La fabrique — un propriétaire, une édition, et de quoi rattacher
// -----------------------------------------------------------------------------

/// Tout ce qu'un test de dépôt a besoin de connaître.
///
/// Le semis ne fournit **ni objet stocké, ni quota propre** : chaque test les
/// pose. Un objet semé serait un objet dont le propriétaire et l'état seraient
/// partagés entre des tests qui les modifient.
pub struct Terrain {
    /// L'organisation propriétaire des objets déposés — celle dont le quota est
    /// opposable.
    pub organisation: Uuid,
    /// Une personne **référente** de cette organisation : le droit de poser un
    /// logo est celui-là, et pas l'adhésion simple.
    pub referente: Uuid,
    /// Une personne membre **active** mais non référente : c'est elle qui rend
    /// la garde du logo vérifiable.
    pub membre: Uuid,
    /// Une personne sans aucun lien : la cible naturelle d'une URL forgée.
    pub etrangere: Uuid,
    /// L'édition de référence, entité porteuse des trois déclinaisons.
    pub edition: Uuid,
    /// Une personne qui **administre cette édition-là**, et aucune autre. Le
    /// droit de poser le bandeau d'une COP est celui-là : ni une adhésion, ni
    /// une permission globale — la permission **sur la portée de l'édition**,
    /// et le périmètre d'administration qui va avec (règle métier n° 8).
    pub administratrice: Uuid,
}

pub async fn terrain(bac: &Bac) -> Terrain {
    let organisation = organisation(bac, "Réseau ouest-africain climat", "ROAC").await;
    let referente = personne(bac, "referente@example.org", "Awa", "Sow Fall").await;
    let membre = personne(bac, "membre@example.org", "Karim", "Ilboudo").await;
    let etrangere = personne(bac, "etrangere@example.org", "Léa", "Perret").await;
    adherer(bac, organisation, referente, "manager", "active").await;
    adherer(bac, organisation, membre, "member", "active").await;

    let edition = edition_cop31(bac).await;
    let administratrice = personne(bac, "admin@ifdd.org", "Sylvie", "Nomo").await;
    attribuer(bac, administratrice, "admin", "event", Some(edition)).await;

    Terrain {
        organisation,
        referente,
        membre,
        etrangere,
        edition,
        administratrice,
    }
}

/// Une attribution de rôle, avec sa portée. C'est **la portée** qui distingue
/// « administrateur de la plateforme » de « administrateur de la COP31 » : le
/// nom du rôle est le même.
pub async fn attribuer(
    bac: &Bac,
    person_id: Uuid,
    role_code: &str,
    scope_type: &str,
    scope_id: Option<Uuid>,
) {
    sqlx::query!(
        "INSERT INTO identity.role_assignments (person_id, role_code, scope_type, scope_id)
         VALUES ($1, $2, $3::text::identity.scope_type, $4)",
        person_id,
        role_code,
        scope_type,
        scope_id
    )
    .execute(bac.pool())
    .await
    .expect("attribution du rôle");
}

pub async fn organisation(bac: &Bac, nom: &str, sigle: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO org.organizations
               (legal_name, acronym, slug, organization_type_code, status, verified_at)
           VALUES ($1, $2, platform.slugify($1)::platform.slug,
                   'ngo_association', 'active', now())
        RETURNING id"#,
        nom,
        sigle
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de l'organisation")
}

pub async fn personne(bac: &Bac, email: &str, prenom: &str, nom: &str) -> Uuid {
    sqlx::query_scalar!(
        r#"INSERT INTO identity.people
               (primary_email, first_name, last_name, email_verified_at, status)
           VALUES ($1::text::platform.email, $2, $3, now(), 'active')
        RETURNING id"#,
        email,
        prenom,
        nom
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la personne")
}

pub async fn adherer(bac: &Bac, organisation: Uuid, personne: Uuid, role: &str, statut: &str) {
    sqlx::query!(
        // `job_title` renseignée : une adhésion ACTIVE en porte toujours une
        // (`ck_memberships_job_title`).
        "INSERT INTO org.memberships (organization_id, person_id, role, status, job_title, approved_at)
         VALUES ($1, $2, $3::text::org.membership_role, $4::text::org.membership_status,
                 'Chargée de projet', now())",
        organisation,
        personne,
        role,
        statut
    )
    .execute(bac.pool())
    .await
    .expect("insertion de l'adhésion");
}

/// L'édition de référence : COP31, Belém. **Trois heures derrière l'UTC** — le
/// décalage qui rend visible une conversion d'heure murale oubliée.
pub async fn edition_cop31(bac: &Bac) -> Uuid {
    let serie = sqlx::query_scalar!("SELECT id FROM event.event_series WHERE code = 'cop_climate'")
        .fetch_one(bac.pool())
        .await
        .expect("série climat du semis");

    // `ck_events_physical_location` exige un pays et une ville dès que l'édition
    // n'est pas entièrement en ligne : une COP se tient quelque part.
    let bresil = sqlx::query_scalar!("SELECT id FROM reference.countries WHERE iso3 = 'BRA'")
        .fetch_one(bac.pool())
        .await
        .expect("Brésil du référentiel");

    sqlx::query_scalar!(
        r#"INSERT INTO event.events
               (series_id, edition_label, edition_year, title, acronym, slug, description,
                status, participation_mode, timezone, starts_at, ends_at,
                country_id, city, has_pavilion)
           VALUES ($1, 'COP31', 2027,
                   '{"fr":"COP31 — Conférence des Parties","en":"COP31"}'::jsonb,
                   'COP31', 'cop31-belem'::platform.slug,
                   '{"fr":"Pavillon de la Francophonie à la COP31.","en":"Francophonie pavilion."}'::jsonb,
                   'announced', 'hybrid', 'America/Belem'::platform.timezone_name,
                   timestamp '2027-11-09 09:00' AT TIME ZONE 'America/Belem',
                   timestamp '2027-11-20 18:00' AT TIME ZONE 'America/Belem',
                   $2, 'Belém', true)
        RETURNING id"#,
        serie,
        bresil
    )
    .fetch_one(bac.pool())
    .await
    .expect("insertion de la COP31")
}

/// Poser un quota propre à une organisation. Le semis n'en fournit qu'un — la
/// ligne par défaut, à `organization_id` nul.
pub async fn quota(bac: &Bac, organisation: Uuid, max_bytes: i64, max_files: i32) {
    sqlx::query!(
        "INSERT INTO media.storage_quotas (organization_id, max_bytes, max_files)
         VALUES ($1, $2, $3)
         ON CONFLICT (organization_id) DO UPDATE
            SET max_bytes = EXCLUDED.max_bytes, max_files = EXCLUDED.max_files",
        organisation,
        max_bytes,
        max_files
    )
    .execute(bac.pool())
    .await
    .expect("pose du quota");
}

// -----------------------------------------------------------------------------
// Les fichiers d'épreuve — de VRAIES images, et un document
// -----------------------------------------------------------------------------

/// Un fichier tel qu'un client le déposerait.
pub struct Fichier {
    pub nom: &'static str,
    pub mime: &'static str,
    pub octets: Vec<u8>,
}

/// Une image PNG opaque de la taille demandée, réellement encodée.
///
/// Le damier n'est pas un ornement : une image d'une seule couleur se comprime
/// à quelques octets, et l'on ne verrait pas la différence de poids entre
/// l'original et ses déclinaisons — précisément ce que les compteurs de quota
/// mesurent.
pub fn image(nom: &'static str, largeur: u32, hauteur: u32) -> Fichier {
    let mut tampon = image::RgbImage::new(largeur, hauteur);
    for (x, y, pixel) in tampon.enumerate_pixels_mut() {
        let teinte = ((x / 8 + y / 8) % 2) as u8;
        *pixel = image::Rgb([40 + teinte * 180, (x % 251) as u8, (y % 241) as u8]);
    }

    let mut octets = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(tampon)
        .write_to(&mut octets, image::ImageFormat::Png)
        .expect("encodage PNG");

    Fichier {
        nom,
        mime: "image/png",
        octets: octets.into_inner(),
    }
}

/// Une image PNG **avec transparence** : c'est elle qui décide du format des
/// déclinaisons — PNG plutôt que JPEG, un logo aplati sur du blanc étant un
/// défaut visible sur fond sombre.
pub fn image_transparente(nom: &'static str, largeur: u32, hauteur: u32) -> Fichier {
    let mut tampon = image::RgbaImage::new(largeur, hauteur);
    for (x, y, pixel) in tampon.enumerate_pixels_mut() {
        let opacite = if (x + y) % 3 == 0 { 0 } else { 255 };
        *pixel = image::Rgba([(x % 251) as u8, (y % 241) as u8, 120, opacite]);
    }

    let mut octets = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(tampon)
        .write_to(&mut octets, image::ImageFormat::Png)
        .expect("encodage PNG");

    Fichier {
        nom,
        mime: "image/png",
        octets: octets.into_inner(),
    }
}

/// **Le bandeau panoramique, 32:9.** La forme que `event.events / banner` exige,
/// à la tolérance de 2 % près.
pub fn bandeau_32_9() -> Fichier {
    image("bandeau.png", 3200, 900)
}

/// **La couverture, 16:9.** `event.events / cover`.
pub fn couverture_16_9() -> Fichier {
    image("couverture.png", 1920, 1080)
}

/// **La vignette carrée, 1:1.** `event.events / thumbnail`.
pub fn vignette_1_1() -> Fichier {
    image("vignette.png", 800, 800)
}

/// Une image qui ne respecte **aucune** des trois formes : c'est elle qui fait
/// tomber le refus de forme, avec ses trois chiffres.
pub fn image_mal_cadree() -> Fichier {
    image("mal-cadree.png", 1024, 768)
}

/// Un PDF minimal mais **valide** — de quoi éprouver qu'un document n'est ni
/// mesuré, ni décliné, et qu'il est refusé là où une image est attendue.
pub fn document_pdf() -> Fichier {
    let contenu = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n\
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n\
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 595 842]>>endobj\n\
trailer<</Root 1 0 R>>\n%%EOF\n";

    Fichier {
        nom: "programme.pdf",
        mime: "application/pdf",
        octets: contenu.to_vec(),
    }
}

// -----------------------------------------------------------------------------
// Déposer, depuis un test
// -----------------------------------------------------------------------------

/// Le dépôt d'un fichier, sans passer par HTTP.
///
/// Les tests de ce crate appellent les **services** : monter la vraie
/// application demanderait une dépendance de développement vers `api`, et
/// `cargo tree -p media` doit rester sans arête. Le corps composite, lui, est
/// éprouvé dans `crates/api/tests/`.
pub async fn deposer(
    bac: &Bac,
    acteur: Uuid,
    fichier: &Fichier,
    metadonnees: MetadonneesDepot,
) -> kernel::error::Result<media::service::upload::ResultatDepot> {
    let flux = media::service::stream::flux_en_tranches(fichier.octets.clone(), 8192);
    media::service::upload::deposer(&bac.state, &bac.ctx(), acteur, metadonnees, flux).await
}

/// Les métadonnées d'un dépôt d'image, avec son texte alternatif — que la base
/// exige pour qu'une image devienne servable.
pub fn metadonnees(fichier: &Fichier) -> MetadonneesDepot {
    MetadonneesDepot {
        filename: fichier.nom.to_owned(),
        mime_type: fichier.mime.to_owned(),
        byte_size: Some(fichier.octets.len() as i64),
        alt_text: fichier
            .mime
            .starts_with("image/")
            .then(|| serde_json::json!({ "fr": "Une image d'épreuve", "en": "A test image" })),
        ..MetadonneesDepot::default()
    }
}

/// Les mêmes, visant une entité porteuse et un rôle.
pub fn metadonnees_pour(
    fichier: &Fichier,
    owner_schema: &str,
    owner_table: &str,
    owner_id: Uuid,
    role: &str,
) -> MetadonneesDepot {
    MetadonneesDepot {
        owner_schema: Some(owner_schema.to_owned()),
        owner_table: Some(owner_table.to_owned()),
        owner_id: Some(owner_id),
        role: Some(role.to_owned()),
        ..metadonnees(fichier)
    }
}

/// Le contenu réellement présent sur le stockage, à une clé donnée.
pub async fn lire_sur_le_stockage(bac: &Bac, cle: &str) -> Option<Vec<u8>> {
    bac.state.storage().get(cle).await.ok()
}

/// Un MP4 réduit à son en-tête — `ftyp`, puis `moov/mvhd`.
///
/// C'est tout ce que le relevé de durée regarde : elle est en clair dans
/// `mvhd`, sous forme d'un nombre de graduations et d'une cadence. Encoder un
/// vrai flux vidéo demanderait un encodeur que le dépôt n'a pas, pour éprouver
/// exactement la même lecture.
pub fn video_mp4(secondes: f64) -> Fichier {
    let cadence: u32 = 1000;
    let graduations = (secondes * cadence as f64).round() as u32;

    let mut mvhd = Vec::new();
    mvhd.extend_from_slice(&[0, 0, 0, 0]); // version 0, aucun drapeau
    mvhd.extend_from_slice(&0_u32.to_be_bytes()); // création
    mvhd.extend_from_slice(&0_u32.to_be_bytes()); // modification
    mvhd.extend_from_slice(&cadence.to_be_bytes());
    mvhd.extend_from_slice(&graduations.to_be_bytes());
    mvhd.extend_from_slice(&[0_u8; 80]);

    let mut moov = Vec::new();
    moov.extend_from_slice(&((mvhd.len() + 8) as u32).to_be_bytes());
    moov.extend_from_slice(b"mvhd");
    moov.extend_from_slice(&mvhd);

    let mut octets = Vec::new();
    octets.extend_from_slice(&20_u32.to_be_bytes());
    octets.extend_from_slice(b"ftypisom");
    octets.extend_from_slice(&[0_u8; 8]);
    octets.extend_from_slice(&((moov.len() + 8) as u32).to_be_bytes());
    octets.extend_from_slice(b"moov");
    octets.extend_from_slice(&moov);

    Fichier {
        nom: "seance.mp4",
        mime: "video/mp4",
        octets,
    }
}

// -----------------------------------------------------------------------------
// Faire passer le worker, depuis un test
// -----------------------------------------------------------------------------

/// **Un passage du worker, par le vrai chemin.**
///
/// Les travaux sont réservés par `platform.claim_jobs()` sur la file que le
/// gestionnaire déclare, exécutés, puis marqués comme le worker les marque. Ce
/// détour n'est pas de la cérémonie : appeler le gestionnaire directement
/// laisserait passer une file mal nommée — un travail déposé dans une file
/// qu'aucun worker n'écoute s'empile sans erreur et sans trace.
///
/// Rend l'issue de chaque travail exécuté.
pub async fn passer_le_worker(bac: &Bac) -> Vec<kernel::error::Result<()>> {
    let gestionnaires = media::job_handlers(bac.db(), &bac.config);
    executer_les_travaux(bac, &gestionnaires).await
}

/// Le même passage, avec des gestionnaires fournis — pour brancher un moteur
/// d'analyse d'épreuve là où la configuration n'en propose aucun qui trouve
/// quelque chose.
///
/// **La réservation se fait par FILE, jamais par gestionnaire.** Les trois
/// travaux du module nomment la même — « media » —, et réserver au nom du
/// premier lui ferait tenir les travaux des deux autres : réservés, non
/// exécutés, et invisibles au tour suivant. C'est exactement ce que fait le
/// worker, qui parcourt les files et distribue par tâche.
pub async fn executer_les_travaux(
    bac: &Bac,
    gestionnaires: &[Arc<dyn kernel::jobs::JobHandler>],
) -> Vec<kernel::error::Result<()>> {
    let mut issues = Vec::new();

    for file in files_de(gestionnaires) {
        loop {
            let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
            let travaux = kernel::jobs::claim(&mut tx, &file, "test-worker", 10)
                .await
                .expect("réservation");
            tx.commit().await.expect("validation");

            if travaux.is_empty() {
                break;
            }

            for travail in travaux {
                let Some(gestionnaire) = gestionnaires.iter().find(|g| g.task() == travail.task)
                else {
                    continue;
                };
                let issue = gestionnaire.run(&travail).await;

                let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
                match &issue {
                    Ok(()) => kernel::jobs::succeed(&mut tx, travail.id)
                        .await
                        .expect("succès"),
                    Err(e) => kernel::jobs::fail(&mut tx, travail.id, &e.to_string())
                        .await
                        .expect("échec"),
                }
                tx.commit().await.expect("validation");
                issues.push(issue);
            }
        }
    }

    issues
}

/// Les files déclarées, dédoublonnées — le `queues()` du registre du worker.
fn files_de(gestionnaires: &[Arc<dyn kernel::jobs::JobHandler>]) -> Vec<String> {
    let mut files: Vec<String> = gestionnaires.iter().map(|g| g.queue().to_owned()).collect();
    files.sort();
    files.dedup();
    files
}

/// **Le worker tué entre l'exécution et son marquage.**
///
/// Les travaux sont réservés et exécutés, mais **rien n'est marqué** : ils
/// restent `running`, exactement comme après un `Ctrl-C` en cours de lot. C'est
/// le scénario que le point de contrôle du quickstart provoque à la main.
pub async fn worker_tue_apres_le_travail(bac: &Bac) -> Vec<kernel::error::Result<()>> {
    let gestionnaires = media::job_handlers(bac.db(), &bac.config);
    let mut issues = Vec::new();

    for file in files_de(&gestionnaires) {
        let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
        let travaux = kernel::jobs::claim(&mut tx, &file, "test-worker", 10)
            .await
            .expect("réservation");
        tx.commit().await.expect("validation");

        for travail in travaux {
            if let Some(gestionnaire) = gestionnaires.iter().find(|g| g.task() == travail.task) {
                issues.push(gestionnaire.run(&travail).await);
            }
        }
    }

    issues
}

/// **Le worker relancé rend à la file ce que le précédent a laissé réservé.**
///
/// C'est `jobs::reclaim_stalled()`, la fonction du noyau, appelée avec un bail
/// nul pour ne pas attendre trente minutes : le chemin est le vrai, et la
/// charge utile du travail est intacte — ce qui n'est pas le cas d'un travail
/// déjà marqué comme réussi, dont `succeed()` a **vidé** la charge utile.
pub async fn worker_relance(bac: &Bac) {
    let gestionnaires = media::job_handlers(bac.db(), &bac.config);
    let mut tx = bac.db().write(&bac.ctx()).await.expect("transaction");
    for file in files_de(&gestionnaires) {
        kernel::jobs::reclaim_stalled(&mut tx, &file, 0.0)
            .await
            .expect("reprise des travaux bloqués");
    }
    tx.commit().await.expect("validation");
}

/// Le nombre de déclinaisons écrites pour un objet, tous états confondus.
pub async fn compter_declinaisons(bac: &Bac, asset_id: Uuid) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "compte!" FROM media.renditions WHERE asset_id = $1"#,
        asset_id
    )
    .fetch_one(bac.pool())
    .await
    .expect("comptage des déclinaisons")
}

/// La consommation d'espace enregistrée pour une organisation.
pub async fn espace_consomme(bac: &Bac, organisation: Uuid) -> i64 {
    sqlx::query_scalar!(
        "SELECT used_bytes FROM media.storage_quotas WHERE organization_id = $1",
        organisation
    )
    .fetch_optional(bac.pool())
    .await
    .expect("lecture du quota")
    .unwrap_or(0)
}
