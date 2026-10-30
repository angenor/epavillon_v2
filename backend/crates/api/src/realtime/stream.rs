//! `GET /realtime/stream?topics=…` : la ligne ouverte (SSE) des personnes
//! connectées.
//!
//! Événements : `ready` (boot, versions, sujets refusés) à l'ouverture ;
//! `signal` (sujets changés, regroupés sur 500 ms) ; `resync` (signaux perdus,
//! tout relire) ; `revoked` (sujets retirés à la revalidation) ; `reauth`
//! (jeton d'accès échu ou session close : rafraîchir puis rouvrir) ; `evicted`
//! (plus de cinq flux pour la session : passer au pouls) ; `restart` (arrêt de
//! l'API : revenir après une gigue). Un commentaire toutes les 25 s tient la
//! ligne sous le `Timeout 60` d'Apache.

use actix_web::http::header;
use actix_web::web::Bytes;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse};
use futures_util::stream;
use identity::AccessTokenCodec;
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::error::{ApiError, Result};
use kernel::realtime::{Audience, Topic, TopicRegistry, Viewer};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::{broadcast, mpsc};
use tokio::time::Instant;
use uuid::Uuid;

use super::hub::{Compteurs, Hub, HubMessage};

/// Le battement sert aussi à voir partir un client : Actix ne lâche le corps
/// qu'à une écriture refusée, donc un flux abandonné vit jusqu'à deux battements
/// (mesuré le 31/10). Le plafond par session ferme d'abord ces plus anciens.
pub const BATTEMENT: Duration = Duration::from_secs(25);
pub const REVALIDATION: Duration = Duration::from_secs(60);
pub const REGROUPEMENT: Duration = Duration::from_millis(500);
/// Délai de reconnexion annoncé au client (`retry:`) ; `useRealtime` a le sien.
const RETRY_MS: u32 = 3000;
const FILE_SORTIE: usize = 64;

#[derive(Debug, Deserialize)]
pub struct TopicsQuery {
    pub topics: String,
}

#[derive(Debug, Serialize)]
struct Ready {
    boot: String,
    versions: BTreeMap<String, u64>,
    denied: Vec<String>,
}

#[utoipa::path(
    get,
    description = "Flux `text/event-stream` des sujets demandés (`topics`, séparés par des virgules, 32 au plus), pour une session connectée. Ne transporte aucune donnée : un `signal` nomme les sujets changés, la page relit sa route. Événements : `ready` (`RealtimeReady`), `signal` (`RealtimeSignal`), `resync`, `revoked`, `reauth`, `evicted`, `restart`. Sujets refusés listés dans `ready` ; aucun accordé : 403. Cinq flux par session, le plus ancien fermé.",
    path = "/realtime/stream",
    tag = "Temps réel",
    operation_id = "realtime_stream",
    params(("topics" = String, Query, description = "Sujets, séparés par des virgules")),
    responses(
        (status = 200, description = "Flux SSE", content_type = "text/event-stream", body = String),
        (status = 401, description = "Aucune session", body = crate::openapi::ApiErrorBody),
        (status = 403, description = "Aucun sujet accordé", body = crate::openapi::ApiErrorBody),
        (status = 422, description = "Sujet mal formé ou trop de sujets", body = crate::openapi::ApiErrorBody),
    )
)]
pub(crate) async fn flux(
    requete: HttpRequest,
    query: web::Query<TopicsQuery>,
    hub: web::Data<Hub>,
    registre: web::Data<TopicRegistry>,
    db: web::Data<Db>,
    codec: web::Data<AccessTokenCodec>,
) -> Result<HttpResponse> {
    let contexte = requete.extensions().get::<RequestContext>().cloned();
    let (personne, session) = contexte
        .and_then(|c| c.actor_id.zip(c.session_id))
        .ok_or_else(ApiError::unauthenticated)?;
    // L'intergiciel a déjà vérifié le jeton : on n'en relit que l'échéance.
    let echeance = requete
        .cookie(identity::COOKIE_ACCES)
        .and_then(|c| codec.verify(c.value()))
        .map(|c| c.expires_at)
        .unwrap_or_else(|| OffsetDateTime::now_utc() + time::Duration::minutes(15));

    let viewer = Viewer {
        person_id: Some(personne),
        session_id: Some(session),
    };
    let sujets = super::lire_sujets(&query.topics)?;
    let (accordes, refuses) =
        super::trier(&registre, db.pool(), &viewer, sujets.clone(), None).await?;
    Compteurs::plus(&hub.compteurs.refus, refuses.len() as u64);
    if accordes.is_empty() {
        return Err(ApiError::forbidden());
    }

    let hub: Arc<Hub> = hub.into_inner();
    let (sortie, entree) = mpsc::channel::<Bytes>(FILE_SORTIE);
    let pret = Ready {
        boot: hub.boot(),
        versions: hub.versions(&accordes),
        denied: refuses,
    };
    let _ = sortie.try_send(Bytes::from(format!("retry: {RETRY_MS}\n\n")));
    let _ = sortie.try_send(evenement("ready", &pret));

    let (id, fermeture) = hub.ouvrir_flux(session);
    let ligne = Ligne {
        hub: hub.clone(),
        session,
        id,
    };
    tracing::info!(personne = %personne, sujets = accordes.len(), "temps réel : flux ouvert");
    let tenue = Tenue {
        ligne,
        pool: db.pool().clone(),
        registre: registre.get_ref().clone(),
        viewer,
        sujets,
        accordes,
        echeance,
        sortie,
        reception: hub.abonner(),
        fermeture,
    };
    tokio::spawn(tenue.tenir());

    let corps = stream::unfold(entree, |mut entree| async move {
        entree
            .recv()
            .await
            .map(|morceau| (Ok::<_, Infallible>(morceau), entree))
    });

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        // `no-transform` : Apache ne compresse pas, donc ne retient rien en tampon.
        .insert_header((header::CACHE_CONTROL, "no-cache, no-transform"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(corps))
}

fn evenement(nom: &str, donnees: &impl Serialize) -> Bytes {
    let json = serde_json::to_string(donnees).unwrap_or_else(|_| "{}".to_owned());
    Bytes::from(format!("event: {nom}\ndata: {json}\n\n"))
}

/// Retire le flux du concentrateur quelle que soit la sortie de la boucle.
struct Ligne {
    hub: Arc<Hub>,
    session: Uuid,
    id: u64,
}

impl Drop for Ligne {
    fn drop(&mut self) {
        self.hub.fermer_flux(self.session, self.id);
    }
}

struct Tenue {
    ligne: Ligne,
    pool: PgPool,
    registre: TopicRegistry,
    viewer: Viewer,
    sujets: Vec<Topic>,
    accordes: BTreeMap<Topic, Audience>,
    echeance: OffsetDateTime,
    sortie: mpsc::Sender<Bytes>,
    reception: broadcast::Receiver<HubMessage>,
    fermeture: Arc<tokio::sync::Notify>,
}

enum Suite {
    Continuer,
    Fermer(&'static str),
}

impl Tenue {
    fn envoyer(&self, morceau: Bytes) -> Suite {
        // Une file pleine est un client qui ne lit plus : on ferme, il
        // reviendra et relira tout.
        match self.sortie.try_send(morceau) {
            Ok(()) => Suite::Continuer,
            Err(_) => Suite::Fermer("client lent ou parti"),
        }
    }

    async fn tenir(mut self) {
        let debut = Instant::now();
        let reste = (self.echeance - OffsetDateTime::now_utc())
            .try_into()
            .unwrap_or(Duration::ZERO);
        let echeance = tokio::time::sleep(reste);
        tokio::pin!(echeance);
        let mut battement = tokio::time::interval_at(debut + BATTEMENT, BATTEMENT);
        let mut revalidation = tokio::time::interval_at(debut + REVALIDATION, REVALIDATION);
        let mut en_attente: BTreeSet<String> = BTreeSet::new();
        let mut envoi_a: Option<Instant> = None;
        let fermeture = self.fermeture.clone();

        let cause = loop {
            let delai = envoi_a.unwrap_or_else(|| Instant::now() + Duration::from_secs(3600));
            let suite = tokio::select! {
                _ = self.sortie.closed() => Suite::Fermer("client parti"),
                _ = fermeture.notified() => {
                    let _ = self.envoyer(evenement("evicted", &json!({})));
                    Suite::Fermer("plus de cinq flux pour la session")
                }
                _ = &mut echeance => {
                    let _ = self.envoyer(evenement("reauth", &json!({})));
                    Suite::Fermer("jeton d'accès échu")
                }
                _ = battement.tick() => self.envoyer(Bytes::from_static(b": battement\n\n")),
                _ = revalidation.tick() => self.revalider().await,
                _ = tokio::time::sleep_until(delai), if envoi_a.is_some() => {
                    envoi_a = None;
                    let sujets: Vec<String> = std::mem::take(&mut en_attente).into_iter().collect();
                    Compteurs::plus(&self.ligne.hub.compteurs.signaux_livres, sujets.len() as u64);
                    self.envoyer(evenement("signal", &json!({ "topics": sujets })))
                }
                recu = self.reception.recv() => match recu {
                    Ok(HubMessage::Signal { topic, audience }) => {
                        let admis = self
                            .accordes
                            .get(topic.as_ref())
                            .is_some_and(|a| a.admits(audience.as_deref()));
                        if admis {
                            en_attente.insert(topic.to_string());
                            envoi_a.get_or_insert_with(|| Instant::now() + REGROUPEMENT);
                        }
                        Suite::Continuer
                    }
                    Ok(HubMessage::Resync) | Err(broadcast::error::RecvError::Lagged(_)) => {
                        en_attente.clear();
                        envoi_a = None;
                        self.envoyer(evenement("resync", &json!({ "boot": self.ligne.hub.boot() })))
                    }
                    Ok(HubMessage::Restart) => {
                        let _ = self.envoyer(evenement("restart", &json!({})));
                        Suite::Fermer("arrêt de l'API")
                    }
                    Err(broadcast::error::RecvError::Closed) => Suite::Fermer("concentrateur fermé"),
                },
            };
            if let Suite::Fermer(cause) = suite {
                break cause;
            }
        };

        tracing::info!(
            personne = ?self.viewer.person_id,
            duree_s = debut.elapsed().as_secs(),
            cause,
            "temps réel : flux fermé"
        );
    }

    /// Session encore valide, sujets encore autorisés (un administrateur retiré
    /// d'une édition perd ses sujets dans la minute).
    async fn revalider(&mut self) -> Suite {
        let Some(session) = self.viewer.session_id else {
            return Suite::Fermer("sans session");
        };
        match identity::repo::sessions::resolve_active(&self.pool, session.into()).await {
            Ok(Some(_)) => {}
            Ok(None) => {
                let _ = self.envoyer(evenement("reauth", &json!({})));
                return Suite::Fermer("session close");
            }
            // Base injoignable : on garde la ligne, la prochaine revalidation tranchera.
            Err(_) => return Suite::Continuer,
        }
        let Ok((accordes, refuses)) = super::trier(
            &self.registre,
            &self.pool,
            &self.viewer,
            self.sujets.clone(),
            None,
        )
        .await
        else {
            return Suite::Continuer;
        };
        let retires: Vec<&String> = refuses
            .iter()
            .filter(|s| self.accordes.contains_key(s.as_str()))
            .collect();
        if !retires.is_empty() {
            if let Suite::Fermer(c) =
                self.envoyer(evenement("revoked", &json!({ "topics": retires })))
            {
                return Suite::Fermer(c);
            }
        }
        // Un sujet refusé à l'ouverture et accordé depuis reste hors du flux :
        // la page le demandera à sa prochaine ouverture.
        self.accordes.retain(|t, _| accordes.contains_key(t));
        if self.accordes.is_empty() {
            return Suite::Fermer("plus aucun sujet accordé");
        }
        Suite::Continuer
    }
}
