//! Les quatre verbes, en *path-style*, au-dessus de [`super::sigv4`].
//!
//! `S3_FORCE_PATH_STYLE=true` est déclaré dans `.env.example` depuis le 16/08, et
//! Garage ne sait faire que cela : le nom du bucket vit dans le **chemin**,
//! jamais dans le nom d'hôte. Un stockage en nom d'hôte exigerait un certificat
//! générique et une entrée DNS par bucket, pour aucun gain.

use actix_web::web::Bytes;
use async_trait::async_trait;
use futures_util::StreamExt;
use kernel::config::S3Config;
use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::sigv4::{self, CHARGE_VIDE};
use super::{FluxOctets, ObjectInfo, ObjectStore, StorageError, StorageResult};

/// Tranches de lecture du tampon local. 64 Kio : assez pour que le coût par
/// tranche disparaisse, assez peu pour que la mémoire reste constante.
const TRANCHE: usize = 64 * 1024;

pub struct S3Store {
    client: reqwest::Client,
    endpoint: String,
    bucket: String,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    /// Vrai : le bucket vit dans le **chemin**. C'est le seul style que Garage
    /// accepte, et le défaut de `.env.example` depuis le 16/08. Faux : il vit
    /// dans le nom d'hôte — ce qu'exigerait un fournisseur cloud, et ce qui
    /// demande alors un certificat générique et une entrée DNS par bucket.
    force_path_style: bool,
}

impl S3Store {
    pub fn new(cfg: &S3Config) -> Self {
        Self {
            client: reqwest::Client::builder()
                // Un fond vidéo de deux cents mégaoctets ne se dépose pas en
                // quinze secondes sur une liaison ordinaire.
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("client HTTP"),
            endpoint: cfg.endpoint.trim_end_matches('/').to_owned(),
            bucket: cfg.bucket.clone(),
            region: cfg.region.clone(),
            access_key_id: cfg.access_key_id.clone(),
            secret_access_key: cfg.secret_access_key.expose().to_owned(),
            force_path_style: cfg.force_path_style,
        }
    }

    /// Le chemin signé. Il porte le bucket, ou non, selon le style.
    fn chemin(&self, key: &str) -> String {
        let cle = sigv4::encoder_chemin(key);
        if self.force_path_style {
            format!("/{}/{}", self.bucket, cle)
        } else {
            format!("/{cle}")
        }
    }

    /// Le nom d'hôte signé. **Il doit être exactement celui que le client
    /// enverra** : un `host` signé qui diffère du `host` transmis produit un 403
    /// dont le message ne nomme jamais la cause.
    fn hote(&self) -> String {
        let brut = self
            .endpoint
            .split_once("://")
            .map(|(_, reste)| reste)
            .unwrap_or(&self.endpoint);
        if self.force_path_style {
            brut.to_owned()
        } else {
            format!("{}.{brut}", self.bucket)
        }
    }

    /// L'adresse appelée, cohérente avec [`Self::chemin`] et [`Self::hote`].
    fn url(&self, key: &str) -> String {
        let (schema, brut) = self
            .endpoint
            .split_once("://")
            .unwrap_or(("http", &self.endpoint));
        let _ = brut;
        if self.force_path_style {
            format!("{}{}", self.endpoint, self.chemin(key))
        } else {
            format!("{schema}://{}{}", self.hote(), self.chemin(key))
        }
    }

    /// Bâtit une requête signée. `entetes_sup` porte ce que le verbe ajoute —
    /// le type de contenu d'un dépôt, la source d'une copie.
    /// Une requête signée dont la charge utile tient en mémoire.
    async fn appeler(
        &self,
        methode: reqwest::Method,
        key: &str,
        charge: Option<Vec<u8>>,
        entetes_sup: &[(&str, String)],
    ) -> StorageResult<reqwest::Response> {
        let empreinte = match &charge {
            Some(octets) => sigv4::empreinte(octets),
            None => CHARGE_VIDE.to_owned(),
        };
        let corps = charge.map(|octets| {
            let taille = octets.len() as u64;
            (reqwest::Body::from(octets), taille)
        });

        self.appeler_avec_corps(methode, key, corps, &empreinte, entetes_sup)
            .await
    }

    /// La forme générale : le corps et son empreinte arrivent tout faits.
    ///
    /// La longueur voyage à part et devient `content-length`. **Sans elle**,
    /// `reqwest` enverrait un corps découpé en fragments, que le protocole S3
    /// n'accepte pas sans une signature par fragment — et le refus tomberait en
    /// 501 sur un message qui ne nomme pas la cause.
    async fn appeler_avec_corps(
        &self,
        methode: reqwest::Method,
        key: &str,
        corps: Option<(reqwest::Body, u64)>,
        empreinte: &str,
        entetes_sup: &[(&str, String)],
    ) -> StorageResult<reqwest::Response> {
        let maintenant = OffsetDateTime::now_utc();
        let horodatage = sigv4::horodatage(maintenant);

        let mut entetes: Vec<(String, String)> = vec![
            ("host".to_owned(), self.hote()),
            ("x-amz-date".to_owned(), horodatage.clone()),
            ("x-amz-content-sha256".to_owned(), empreinte.to_owned()),
        ];
        for (nom, valeur) in entetes_sup {
            entetes.push(((*nom).to_owned(), valeur.clone()));
        }

        let chemin = self.chemin(key);
        let autorisation = sigv4::autorisation(
            &sigv4::Requete {
                methode: methode.as_str(),
                chemin: &chemin,
                requete: "",
                entetes: &entetes,
                charge_sha256: empreinte,
            },
            &sigv4::Identifiants {
                access_key_id: &self.access_key_id,
                secret_access_key: &self.secret_access_key,
                region: &self.region,
                service: "s3",
            },
            maintenant,
        );

        let mut requete = self
            .client
            .request(methode, self.url(key))
            .header("Authorization", autorisation);
        for (nom, valeur) in &entetes {
            // `host` est posé par le client HTTP lui-même : le repasser en
            // ferait un doublon, et la signature porterait sur une valeur que la
            // requête n'envoie pas.
            if nom != "host" {
                requete = requete.header(nom.as_str(), valeur.as_str());
            }
        }
        if let Some((corps, longueur)) = corps {
            requete = requete
                .header(reqwest::header::CONTENT_LENGTH, longueur)
                .body(corps);
        }

        requete
            .send()
            .await
            .map_err(|e| StorageError::Unavailable(e.to_string()))
    }

    async fn verifier(reponse: reqwest::Response, key: &str) -> StorageResult<reqwest::Response> {
        let statut = reponse.status();
        if statut.is_success() {
            return Ok(reponse);
        }
        if statut == reqwest::StatusCode::NOT_FOUND {
            return Err(StorageError::NotFound(key.to_owned()));
        }
        let corps = reponse.text().await.unwrap_or_default();
        Err(StorageError::Rejected {
            statut: statut.as_u16(),
            corps,
        })
    }
}

#[async_trait]
impl ObjectStore for S3Store {
    /// # Pourquoi un tampon sur disque, et non un envoi direct du flux
    ///
    /// **SigV4 signe l'empreinte de la charge utile.** Elle doit donc être
    /// connue **avant** d'envoyer le premier octet — ce qui est impossible tant
    /// que le flux n'a pas été lu jusqu'au bout. Les deux contournements du
    /// protocole coûtent plus qu'ils ne rapportent ici : `UNSIGNED-PAYLOAD`
    /// renonce à l'intégrité de bout en bout, et l'envoi en plusieurs parties
    /// demande trois appels de plus et une reprise sur incident.
    ///
    /// Le flux est donc versé dans un fichier temporaire local — **mémoire
    /// constante**, une tranche à la fois —, puis relu en flux vers le stockage.
    /// Le fichier temporaire est effacé dans tous les cas.
    async fn put_stream(
        &self,
        key: &str,
        mime_type: &str,
        mut contenu: FluxOctets,
    ) -> StorageResult<u64> {
        let tampon = std::env::temp_dir().join(format!(
            "epavillon-upload-{}",
            uuid::Uuid::new_v4().simple()
        ));

        let resultat = async {
            let mut fichier = tokio::fs::File::create(&tampon)
                .await
                .map_err(|e| StorageError::Unavailable(e.to_string()))?;
            let mut hacheur = <sha2::Sha256 as sha2::Digest>::new();
            let mut ecrits = 0_u64;

            while let Some(tranche) = contenu.next().await {
                let tranche = tranche?;
                sha2::Digest::update(&mut hacheur, &tranche);
                fichier
                    .write_all(&tranche)
                    .await
                    .map_err(|e| StorageError::Unavailable(e.to_string()))?;
                ecrits += tranche.len() as u64;
            }
            fichier
                .flush()
                .await
                .map_err(|e| StorageError::Unavailable(e.to_string()))?;
            drop(fichier);

            let empreinte: String = sha2::Digest::finalize(hacheur)
                .iter()
                .map(|o| format!("{o:02x}"))
                .collect();

            let corps = reqwest::Body::wrap_stream(lire_par_tranches(tampon.clone()));
            let reponse = self
                .appeler_avec_corps(
                    reqwest::Method::PUT,
                    key,
                    Some((corps, ecrits)),
                    &empreinte,
                    &[("content-type", mime_type.to_owned())],
                )
                .await?;
            Self::verifier(reponse, key).await?;
            Ok(ecrits)
        }
        .await;

        let _ = tokio::fs::remove_file(&tampon).await;
        resultat
    }

    async fn put(&self, key: &str, mime_type: &str, contenu: Vec<u8>) -> StorageResult<()> {
        let reponse = self
            .appeler(
                reqwest::Method::PUT,
                key,
                Some(contenu),
                &[("content-type", mime_type.to_owned())],
            )
            .await?;
        Self::verifier(reponse, key).await.map(|_| ())
    }

    async fn get(&self, key: &str) -> StorageResult<Vec<u8>> {
        let reponse = self.appeler(reqwest::Method::GET, key, None, &[]).await?;
        let reponse = Self::verifier(reponse, key).await?;
        reponse
            .bytes()
            .await
            .map(|o| o.to_vec())
            .map_err(|e| StorageError::Unavailable(e.to_string()))
    }

    async fn head(&self, key: &str) -> StorageResult<ObjectInfo> {
        let reponse = self.appeler(reqwest::Method::HEAD, key, None, &[]).await?;
        let reponse = Self::verifier(reponse, key).await?;
        let byte_size = reponse
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or_default();
        Ok(ObjectInfo { byte_size })
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        let reponse = self
            .appeler(reqwest::Method::DELETE, key, None, &[])
            .await?;
        // S3 rend 204 pour une suppression, y compris d'un objet absent : la
        // purge se rejoue sans échouer, ce que le contrat exige.
        match Self::verifier(reponse, key).await {
            Ok(_) | Err(StorageError::NotFound(_)) => Ok(()),
            Err(autre) => Err(autre),
        }
    }

    async fn rename(&self, de: &str, vers: &str) -> StorageResult<()> {
        // Copie **côté serveur** : les octets ne traversent pas le réseau. La
        // source porte le bucket, comme le protocole l'exige.
        let source = format!("/{}/{}", self.bucket, sigv4::encoder_chemin(de));
        let reponse = self
            .appeler(
                reqwest::Method::PUT,
                vers,
                None,
                &[("x-amz-copy-source", source)],
            )
            .await?;
        Self::verifier(reponse, de).await?;
        self.delete(de).await
    }

    fn engine(&self) -> &'static str {
        "s3"
    }
}

/// Relit un fichier local par tranches, sous forme de flux.
///
/// Écrit ici plutôt que pris à `tokio-util` : une vingtaine de lignes contre une
/// dépendance de plus, pour un besoin qui n'apparaît qu'à cet endroit.
fn lire_par_tranches(
    chemin: std::path::PathBuf,
) -> impl futures_util::Stream<Item = StorageResult<Bytes>> + Send {
    futures_util::stream::unfold(None, move |fichier| {
        let chemin = chemin.clone();
        async move {
            let mut fichier = match fichier {
                Some(f) => f,
                None => match tokio::fs::File::open(&chemin).await {
                    Ok(f) => f,
                    Err(e) => {
                        return Some((Err(StorageError::Unavailable(e.to_string())), None));
                    }
                },
            };

            let mut tampon = vec![0_u8; TRANCHE];
            match fichier.read(&mut tampon).await {
                Ok(0) => None,
                Ok(lus) => {
                    tampon.truncate(lus);
                    Some((Ok(Bytes::from(tampon)), Some(fichier)))
                }
                Err(e) => Some((Err(StorageError::Unavailable(e.to_string())), None)),
            }
        }
    })
}
