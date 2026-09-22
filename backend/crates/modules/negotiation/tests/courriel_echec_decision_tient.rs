//! **Un courriel qui ne part pas ne défait pas la décision** — SC-011.
//!
//! La décision et la mise en file naissent dans la même transaction
//! ([`decision_transaction`]), mais l'envoi, lui, se fait plus tard et ailleurs :
//! un relais injoignable est la panne normale, pas l'exception. Ce qui se joue
//! ici est donc l'inverse du test de transaction — l'accès accordé **survit** à
//! l'échec de l'envoi, et le travail revient en file pour être rejoué.
//!
//! La faute que ce test interdit : traiter l'échec d'envoi comme un échec de la
//! décision, et retrancher l'accès. La personne perdrait un droit qu'un
//! administrateur lui a bel et bien donné, pour une raison qu'elle ne peut ni
//! voir ni corriger.

mod commun;

use async_trait::async_trait;
use commun::{a_lacces, admettre, attribuer, demander, mon_acces, Bac};
use kernel::mail::{MailError, Mailer, OutgoingMail, MAIL_RELAY_UNREACHABLE};
use negotiation::domain::access::{AccessState, RequestStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Un relais qu'on met en panne, puis qu'on répare — comme un relais réel le
/// fait tout seul entre deux essais du worker.
#[derive(Default)]
struct RelaisCapricieux {
    en_panne: AtomicBool,
    remis: Mutex<Vec<OutgoingMail>>,
}

impl RelaisCapricieux {
    fn en_panne() -> Arc<Self> {
        Arc::new(Self {
            en_panne: AtomicBool::new(true),
            remis: Mutex::new(Vec::new()),
        })
    }

    fn reparer(&self) {
        self.en_panne.store(false, Ordering::SeqCst);
    }

    fn messages(&self) -> Vec<OutgoingMail> {
        self.remis.lock().expect("boîte lisible").clone()
    }
}

#[async_trait]
impl Mailer for RelaisCapricieux {
    async fn send(&self, mail: &OutgoingMail) -> Result<(), MailError> {
        if self.en_panne.load(Ordering::SeqCst) {
            return Err(MailError::Unreachable("relais éteint pour le test".into()));
        }
        self.remis
            .lock()
            .expect("boîte inscriptible")
            .push(mail.clone());
        Ok(())
    }
}

/// L'état d'un travail tel que l'exploitation le lit.
struct Travail {
    statut: String,
    essais: i16,
    derniere_erreur: Option<String>,
}

async fn travail(bac: &Bac, task: &str) -> Travail {
    let l = sqlx::query!(
        r#"SELECT status::text AS "statut!", attempts AS "essais!", last_error
             FROM platform.jobs WHERE task = $1"#,
        task
    )
    .fetch_one(bac.pool())
    .await
    .expect("lecture du travail");

    Travail {
        statut: l.statut,
        essais: l.essais,
        derniere_erreur: l.last_error,
    }
}

/// Un tour de worker : réserver ce qui est dû, l'exécuter, puis marquer —
/// exactement la boucle de `crates/worker`, réduite aux gestionnaires de ce
/// module.
async fn tour_de_worker(bac: &Bac, mailer: Arc<dyn Mailer>) {
    let gestionnaires = negotiation::job_handlers(bac.db(), &bac.config, mailer);

    let mut files: Vec<&str> = gestionnaires.iter().map(|g| g.queue()).collect();
    files.sort_unstable();
    files.dedup();

    for file in files {
        let mut tx = bac
            .db()
            .write(&bac.ctx_anonyme())
            .await
            .expect("transaction");
        let travaux = kernel::jobs::claim(&mut tx, file, "test-worker", 50)
            .await
            .expect("réservation");
        tx.commit().await.expect("validation");

        for t in travaux {
            let Some(gestionnaire) = gestionnaires.iter().find(|g| g.task() == t.task) else {
                continue;
            };
            let issue = gestionnaire.run(&t).await;

            let mut tx = bac
                .db()
                .write(&bac.ctx_anonyme())
                .await
                .expect("transaction");
            match issue {
                Ok(()) => kernel::jobs::succeed(&mut tx, t.id).await.expect("succès"),
                Err(e) => kernel::jobs::fail(&mut tx, t.id, &e.to_string())
                    .await
                    .expect("échec"),
            }
            tx.commit().await.expect("validation");
        }
    }
}

/// `fail_job` replanifie avec un délai croissant. Le rejeu ne s'observe donc
/// qu'en ramenant l'échéance — ce que le temps fait de lui-même en
/// exploitation, et qu'un test ne peut pas attendre.
async fn ramener_lecheance(bac: &Bac, task: &str) {
    let id: Uuid = sqlx::query_scalar!("SELECT id FROM platform.jobs WHERE task = $1", task)
        .fetch_one(bac.pool())
        .await
        .expect("le travail est en file");

    let mut tx = bac
        .db()
        .write(&bac.ctx_anonyme())
        .await
        .expect("transaction");
    kernel::jobs::reschedule_by(&mut tx, &[id], -3600.0)
        .await
        .expect("replanification");
    tx.commit().await.expect("validation");
}

#[tokio::test]
async fn le_relais_en_panne_ne_retranche_pas_lacces() {
    let bac = Bac::monter().await;
    let d = commun::decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let demande = demander(&bac, d.person_id, Some(d.space_id), None)
        .await
        .expect("demande");
    admettre(&bac, d.admin_id, demande.id, None)
        .await
        .expect("admission");

    let relais = RelaisCapricieux::en_panne();
    tour_de_worker(&bac, relais.clone()).await;

    assert!(
        relais.messages().is_empty(),
        "le relais est éteint : rien n'est parti"
    );

    // Ce que le test défend : la décision tient.
    assert!(
        a_lacces(&bac, d.person_id, Some(d.space_id)).await,
        "l'accès accordé ne dépend pas de l'acheminement d'un courriel"
    );
    let apres = mon_acces(&bac, d.person_id).await;
    assert_eq!(apres.state, AccessState::Granted);
    assert_eq!(
        apres.request.expect("la demande est rendue").status,
        RequestStatus::Approved,
    );

    // Et le travail attend son tour suivant, au lieu d'être perdu.
    let t = travail(&bac, negotiation::jobs::emails::SEND_APPROVED_EMAIL).await;
    assert_eq!(t.statut, "queued", "le travail se rejoue");
    assert_eq!(t.essais, 1);
    assert!(
        t.derniere_erreur
            .as_deref()
            .is_some_and(|e| e.contains(MAIL_RELAY_UNREACHABLE)),
        "l'exploitation lit le code stable, jamais l'adresse : {:?}",
        t.derniere_erreur
    );
}

#[tokio::test]
async fn le_rejeu_envoie_le_courriel_une_seule_fois() {
    let bac = Bac::monter().await;
    let d = commun::decor(&bac).await;
    attribuer(&bac, d.admin_id, "admin", "global", None).await;

    let demande = demander(&bac, d.person_id, Some(d.space_id), None)
        .await
        .expect("demande");
    admettre(&bac, d.admin_id, demande.id, None)
        .await
        .expect("admission");

    let relais = RelaisCapricieux::en_panne();
    tour_de_worker(&bac, relais.clone()).await;

    relais.reparer();
    ramener_lecheance(&bac, negotiation::jobs::emails::SEND_APPROVED_EMAIL).await;
    tour_de_worker(&bac, relais.clone()).await;

    let partis = relais.messages();
    assert_eq!(partis.len(), 1, "{partis:?}");
    assert_eq!(partis[0].to, "awa.diallo@example.org");
    assert!(
        partis[0].text.contains("/guide-nego/ressources/acces"),
        "le lien ramène dans l'application : {}",
        partis[0].text
    );

    let t = travail(&bac, negotiation::jobs::emails::SEND_APPROVED_EMAIL).await;
    assert_eq!(t.statut, "succeeded");
    assert_eq!(t.essais, 2, "un échec, puis une réussite");

    // Un tour de plus ne renvoie rien : le travail réussi n'est plus réservable.
    tour_de_worker(&bac, relais.clone()).await;
    assert_eq!(relais.messages().len(), 1, "aucun doublon chez la personne");

    assert!(a_lacces(&bac, d.person_id, Some(d.space_id)).await);
}
