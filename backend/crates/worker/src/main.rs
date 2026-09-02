//! Relais d'outbox et file de travaux — `cargo run -p worker`.

mod consumers;
mod jobs;
mod outbox;
mod registry;

use kernel::config::Config;
use kernel::context::RequestContext;
use kernel::db::Db;
use kernel::events::ConsumerRegistry;
use time::OffsetDateTime;

use crate::consumers::telemetry::TelemetryConsumer;
use crate::registry::JobRegistry;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("Configuration refusée au démarrage : {e}");
        std::process::exit(1);
    });

    let _traces = kernel::telemetry::init(&config.telemetry).unwrap_or_else(|e| {
        eprintln!("Télémétrie : {e}");
        std::process::exit(1);
    });

    let db = Db::connect(config.database_url.expose(), 8)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("base injoignable : {e}");
            std::process::exit(1);
        });

    // **Le mailer est ENVELOPPÉ**, exactement comme dans l'API : la liste de
    // suppression et le journal d'expédition s'appliquent donc aussi aux
    // courriels que les travaux différés de B1 et B2 envoient, **sans qu'aucun
    // de ces modules ne change d'une ligne** (écart n° 133).
    let courrier =
        engagement::GardedMailer::envelopper(&config.mail, db.clone()).unwrap_or_else(|e| {
            tracing::error!("courriel : {e}");
            std::process::exit(1);
        });

    let consommateurs = ConsumerRegistry::new()
        .register(TelemetryConsumer)
        // `programme` reçoit l'annonce de publication de `event` et rend
        // publiques les séances désignées — l'autre moitié d'un geste partagé
        // entre deux schémas (B5).
        .register_all(programme::event_consumers())
        // **Les deux plus gros consommateurs du dépôt.** Le premier écoute les
        // inscriptions et les séances et branche sur le STATUT porté par la
        // charge utile — `programme.registration.confirmed`, que le modèle
        // nomme lui-même, N'EXISTE PAS, et un consommateur écrit d'après ce
        // commentaire ne serait jamais réveillé (écart n° 126). Le second
        // écoute tout : la correspondance entre un événement et un avis est une
        // DONNÉE, portée par `notification_types.code`.
        .register_all(engagement::event_consumers(db.clone(), courrier.clone()));

    let travaux = JobRegistry::new()
        .register_all(identity::job_handlers(
            db.clone(),
            &config,
            courrier.clone(),
        ))
        .register_all(org::job_handlers(db.clone(), &config, courrier.clone()))
        // Ce module n'envoie aucun courriel : son unique travail clôt les
        // appels échus.
        .register_all(event::job_handlers(db.clone(), &config))
        // Trois travaux : le traitement d'un objet déposé — mis en file par le
        // DÉCLENCHEUR du modèle, jamais par le service —, la purge et la
        // réconciliation des compteurs de quota. Les trois déclarent la file
        // « media », que le worker n'écoute que pour cette raison.
        .register_all(media::job_handlers(db.clone(), &config))
        // Deux travaux : l'envoi d'un rappel — mis en file par la FONCTION du
        // modèle — et la préparation des partitions mensuelles.
        .register_all(engagement::job_handlers(db.clone(), &config, courrier))
        // **Un seul travail, et c'est ce geste qui fait écouter la file
        // « analytics ».** `JobRegistry::queues()` est construite à partir des
        // files que les gestionnaires nomment, et `platform.claim_jobs()` filtre
        // strictement : sans cette ligne, les demandes de rafraîchissement
        // s'empileraient sans erreur, sans trace, et sans que rien ne les
        // exécute jamais.
        .register_all(analytics::job_handlers(db.clone(), &config));

    // Les travaux récurrents se replanifient eux-mêmes ; le démarrage ne fait
    // que **réarmer** la chaîne, au cas où sa dernière occurrence serait morte
    // avant d'avoir posé la suivante. La clé d'unicité porte le jour : dix
    // redémarrages dans la journée n'en produisent pas dix purges.
    armer_les_recurrents(&db, &config).await;

    tracing::info!(worker = %config.worker_id, "worker démarré");

    let mut relais = tokio::spawn(outbox::run(db.clone(), consommateurs));
    let mut file = tokio::spawn(jobs::run(db.clone(), config.worker_id.clone(), travaux));

    tokio::select! {
        _ = tokio::signal::ctrl_c() => tracing::info!("arrêt demandé"),
        r = &mut relais => tracing::error!(?r, "relais d'outbox arrêté"),
        r = &mut file => tracing::error!(?r, "file de travaux arrêtée"),
    }

    // Abandonner les deux tâches AVANT de fermer le pool : le relais tient une
    // connexion d'écoute pour la vie du processus, et `close()` attend que
    // toutes les connexions soient rendues — sans cet abandon, Ctrl-C ne rend
    // jamais la main.
    relais.abort();
    file.abort();
    db.close().await;
}

async fn armer_les_recurrents(db: &Db, config: &Config) {
    let resultat = async {
        let mut tx = db.write(&RequestContext::background("jobs")).await?;
        let maintenant = OffsetDateTime::now_utc();
        let mut armees = Vec::new();

        if identity::jobs::purge::planifier(&mut tx, maintenant).await? {
            armees.push("purge des jetons");
        }
        // Le balayage des doublons se réarme de la même façon, et pour la même
        // raison : sa dernière tranche a pu mourir avant d'avoir posé la
        // suivante. La clé d'unicité porte le jour — dix redémarrages n'en
        // produisent pas dix.
        if org::jobs::duplicates::planifier(&mut tx, maintenant).await? {
            armees.push("balayage des doublons");
        }
        // Troisième chaîne, même patron. Sa grille est horaire plutôt que
        // journalière : la clé d'unicité porte le créneau visé, pas le jour.
        if event::jobs::autoclose::planifier(
            &mut tx,
            event::jobs::autoclose::prochaine_occurrence(
                maintenant,
                config.event.call_autoclose_interval,
            ),
        )
        .await?
        {
            armees.push("clôture des appels échus");
        }
        // Les trois chaînes de B6. La purge est la seule du dépôt qui touche au
        // stockage : la manquer laisse des octets payés pour rien sur un disque
        // fini, et c'est tout le sujet d'US9.
        if media::jobs::purge::planifier(
            &mut tx,
            media::jobs::purge::prochaine_occurrence(maintenant, config.media.purge_interval),
        )
        .await?
        {
            armees.push("purge des objets");
        }
        if media::jobs::reconcile::planifier(
            &mut tx,
            media::jobs::reconcile::prochaine_occurrence(
                maintenant,
                config.media.reconcile_interval,
            ),
        )
        .await?
        {
            armees.push("réconciliation des quotas");
        }
        if engagement::jobs::partitions::planifier(
            &mut tx,
            engagement::jobs::partitions::prochaine_occurrence(
                maintenant,
                config.engagement.partition_interval,
            ),
        )
        .await?
        {
            armees.push("partitions du journal d'expédition");
        }
        // Septième chaîne, et la seule qui ne pose PAS son propre créneau : la
        // mise en file passe par `analytics.enqueue_refresh()`, dont la clé
        // d'anti-rebond porte la tranche de temps. Dix redémarrages dans la même
        // tranche n'arment donc pas dix rafraîchissements — et l'intervalle doit
        // dépasser cette fenêtre, ce que la configuration vérifie au démarrage.
        if analytics::jobs::refresh::planifier(
            &mut tx,
            config.analytics.refresh_interval,
            config.analytics.refresh_debounce,
        )
        .await?
        {
            armees.push("rafraîchissement des projections");
        }

        tx.commit().await?;
        Ok::<_, kernel::error::ApiError>(armees)
    }
    .await;

    match resultat {
        Ok(armees) if armees.is_empty() => {
            tracing::debug!("travaux récurrents : toutes les chaînes étaient déjà posées")
        }
        Ok(armees) => tracing::info!(chaines = ?armees, "travaux récurrents armés"),
        Err(e) => tracing::error!(erreur = %e, "planification des travaux récurrents impossible"),
    }
}
