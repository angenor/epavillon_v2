# Décisions : 2026-10-31, le temps réel

> Extrait de la [progression](../../PROGRESSION.md). Ce qui a été tranché en cours de route, et pourquoi.

**Statut : proposé.** Étude seule, rien n'est écrit ni tranché. Une fois arbitré, le choix d'architecture rejoint `CADRAGE.md` § 6 comme ADR.

**La demande (30/10).** Plusieurs données doivent se mettre à jour sans recharger. Les deux premières : les échanges d'un dossier (un message de l'organisation apparaît dans la fiche du back-office, compteur de non-lus compris, et réciproquement dans l'espace organisation) ; l'envoi d'un courriel du back-office (statut et nombre envoyé à jour, notification brève « envoi parti » ou « problème », même si la personne a changé de page). Le commanditaire suggère « un mécanisme WebSocket efficace et résilient ». D'autres usages suivront : direct, inscriptions, planificateur à plusieurs, peut-être Guide Négo.

---

## 1. État des lieux

**Aucun flux vers le navigateur n'existe.** Ni SSE, ni WebSocket, ni `EventSource` dans `frontend/app`, ni route de flux dans `backend/`. `actix-ws` n'est pas une dépendance.

**Le sondage existe d'un seul côté.** `composables/usePolling.ts` relit les échanges dans l'espace organisation (`pages/organization/proposal/[id].vue`, fenêtre `components/workspace/CommentsDesk.vue`) : 15 s fenêtre ouverte, 60 s fermée, rien quand l'onglet est caché. Côté back-office, `pages/admin/propositions/[id]/index.vue` et `components/admin/review/CommentsDesk.vue` ne relisent rien : les messages ne changent qu'au rechargement ou après un envoi de la personne elle-même. Les envois de courriels (`pages/admin/courriels/[id].vue`, `index.vue`) ne relisent rien non plus.

**Aucun toast.** Rien dans `components/ui/`, rien dans le guide de style (`docs/guide-de-style-epavillon.html`, `pages/style-guide.vue`). Les retours d'action passent par un `notice` local à la page et `UiAlert`. Le composant est à dessiner.

**L'outbox et son relais.** `platform.emit_event()` écrit dans `platform.outbox_events` dans la transaction du changement et appelle `pg_notify('platform_outbox', id)`. Le worker (`crates/worker/src/outbox.rs`) écoute ce canal par `PgListener` et balaie toutes les 10 s en filet. C'est un bus de **faits métier** à consommateurs idempotents, pas un bus d'affichage : `programme.comment.shared` n'est émis que pour un message visible du déposant (un message interne au comité n'émet rien), et un lot de dix courriels n'émet rien.

**`LISTEN/NOTIFY` est déjà prévu pour le temps réel, sans consommateur.** `docs/database/100_negotiations.sql` (`tg_channel_message_fanout`) émet `pg_notify('negotiation_channel_message', …)` à chaque message d'un canal de Guide Négo, avec le commentaire « Le serveur WebSocket écoute ce canal LISTEN et relaie aux membres connectés ». Personne ne l'écoute encore.

**Valkey tourne et ne sert à rien.** Conteneur présent en local et en production (`ops/docker-compose.prod.yml`), `VALKEY_URL` passé à l'API et au worker, mais aucun client Redis ou Valkey dans le code ni dans `Cargo.toml`. Conforme au cadrage (§ 5.2 : « mis à contribution que lorsque la mesure le justifie »).

**Une seule instance de l'API**, une du worker, une base. Pool de 16 connexions pour l'API, 8 pour le worker. Mise à jour par `./deploy.sh update` : `docker compose build` puis `up -d api worker front`, donc le conteneur de l'API est recréé et toute connexion ouverte tombe.

**Sessions et cookies** (`crates/modules/identity/src/routes/cookies.rs`, `crates/api/src/middleware/auth.rs`) : jeton d'accès `epavillon_at`, `HttpOnly`, `SameSite=Lax`, chemin `/`, **15 minutes** ; jeton de rafraîchissement `epavillon_rt`, `SameSite=Strict`, limité à `/api/auth`. La session est relue en base **à chaque requête** (révocation immédiate). Le contrôle d'origine (`middleware/origin.rs`) ne porte que sur les écritures ; CORS n'autorise que `APP_PUBLIC_URL`. Le site rafraîchit la session par `useApi().refreshSession`.

**Apache** (`ops/apache-epavillon.conf`), seul frontal, partagé avec `financedurable.francophonie.org` en production :
- `ProxyPass /api/ http://127.0.0.1:8080/api/`, **sans `upgrade=websocket`** ni règle `ws://` : une mise à niveau WebSocket **ne passe pas aujourd'hui**. Il faudrait modifier le vhost, et vérifier que `mod_proxy_wstunnel` (ou Apache 2.4.47 et plus) est chargé.
- `ProxyTimeout 120` : un flux qui n'émet rien pendant 120 s est coupé. Un battement toutes les 25 s suffit.
- Apache **compresse en brotli** (mesuré le 22/09, `DEPLOIEMENT.md` § 11) et respecte `Cache-Control: no-transform`, déjà employé pour les PDF. Un flux SSE compressé serait retenu en tampon : il faut cet en-tête.
- ModSecurity (CRS de cPanel) : seule la règle des méthodes est retirée pour `/api/`. Un GET de flux passe ; l'inspection des réponses ne vise pas `text/event-stream` par défaut, à vérifier.
- **Inconnus, à mesurer sur le serveur** : le MPM (`httpd -V | grep -i mpm`), `MaxRequestWorkers`, HTTP/2 (`curl -sI --http2 https://epavillonclimatique.francophonie.org/ | head -1`), modules chargés (`httpd -M | grep -E 'proxy_wstunnel|http2|brotli|deflate|security'`). **C'est le vrai plafond** : avec la configuration actuelle, chaque connexion longue (SSE comme WebSocket) occupe un fil ou un processus d'Apache pendant toute sa durée, pris sur le même réservoir que le site voisin.

**Le nom `live` est pris.** Schéma `live`, crate `live`, `useLiveSession`, `UiLiveBadge` désignent le direct (Zoom, YouTube). Un `useLive()` pour le temps réel créerait une confusion durable : proposition `useRealtime()`, route `/api/realtime`.

**Envoi de courriels : le « problème » n'a pas de représentation.** `send_mailing.rs` rend une erreur quand le serveur d'envoi est injoignable ; `platform.fail_job()` replanifie à 2, 4, 8, 16, 32 s puis passe le travail en `dead`. L'envoi reste alors `queued` (« en cours ») pour toujours, sans erreur au niveau de l'envoi : seule `mailing_recipients.last_error` existe, par destinataire. **Une coupure d'environ une minute du serveur de courriel bloque un envoi sans que rien ne le dise.** À corriger avec l'étape 3 (modèle d'abord : une colonne ou un état sur `engagement.mailings`), sinon la notification « problème » n'a rien à lire.

---

## 2. Proposition

### 2.1 SSE plutôt que WebSocket

Les besoins sont **descendants** : le serveur annonce, le navigateur relit. Toute écriture continue de passer par les routes REST, qui portent déjà l'autorisation par permission et portée, l'audit (`app.actor_id`), le catalogue d'erreurs français et la règle « `send` jamais rejoué ».

| | SSE (`text/event-stream`) | WebSocket |
|---|---|---|
| Apache | GET ordinaire : une règle dédiée, `no-transform`, battement | mise à niveau à ouvrir dans le vhost, module à vérifier, ModSecurity sur la poignée de main |
| Session | cookie `epavillon_at` et `SessionResolver` tels quels | cookie lu à la poignée de main, hors intergiciels habituels |
| Sécurité | CORS limite déjà la lecture à l'origine du site | détournement inter-sites (CSWSH) : `Origin` à vérifier à la main, `OriginCheck` ne voit que les écritures |
| Reconnexion | prévue par le protocole | à écrire (ping, pong, reprise) |
| Changer d'abonnements | rouvrir le flux (ou une route annexe) | message sur la même connexion |
| Limite du navigateur | **6 connexions par origine en HTTP/1.1**, tous onglets confondus | pas dans ce compte |
| Tests | `curl -N`, client de test d'Actix | client WebSocket de test, protocole à décrire |

**Recommandation : SSE**, lu par `fetch` en flux plutôt que par `EventSource` : `EventSource` ne rend pas le code d'une réponse en échec, donc ne distingue pas une session expirée (401, à rafraîchir) d'une panne. Le transport reste caché derrière `useRealtime()` : si Guide Négo exige un jour présence et « en train d'écrire », un WebSocket remplace le flux sans toucher les pages.

La limite de six connexions est réelle (un administrateur ouvre volontiers dix dossiers en onglets). Parade sans complexité : **le flux se ferme quand l'onglet est caché** depuis 30 s et se rouvre au retour, avec relecture. Seuls les onglets visibles tiennent une connexion, donc au plus une par fenêtre. Si HTTP/2 est actif sur Apache, la limite disparaît (une centaine de flux par connexion). Le partage d'un flux entre onglets (Web Locks, `BroadcastChannel`) reste une option si la mesure l'impose.

### 2.2 Un signal, pas une donnée

Le flux ne transporte **jamais de contenu** : il dit « tel sujet a changé », et le navigateur relit par la route REST qu'il appelle déjà. Trois conséquences :
- **L'autorisation de la donnée reste où elle est** : la relecture repasse par `perimeter::acces_au_dossier` ou `has_permission`, comme aujourd'hui, URL forgée comprise (règle 8).
- **Pas de numéro de séquence ni de relecture d'historique** : à chaque (re)connexion, chaque abonné relit son sujet. Un signal perdu pendant une coupure est rattrapé par cette relecture, jamais rejoué.
- Le texte affiché reste résolu par le site et l'API (i18n, `i18n_text`), pas sérialisé dans un message.

Forme d'un signal : `{"topic":"proposal:<uuid>","kind":"comments","audience":"submitter"}`, moins de 8000 octets (limite de `pg_notify`).

### 2.3 Transport : `LISTEN/NOTIFY`, ni outbox, ni Valkey

```
API ou worker, dans la transaction du changement
   SELECT platform.signal('proposal:…', '{"kind":"comments",…}')   ← pg_notify, livré AU COMMIT
                         │
                 canal platform_signal
                         ▼
API : un PgListener dédié → concentrateur en mémoire (tokio broadcast)
   → chaque connexion filtre selon ses sujets autorisés → coalescence 500 ms → flux SSE
```

- **`pg_notify` est transactionnel** : livré au commit, jamais pour une transaction annulée. Même principe que l'outbox (« l'état et son annonce ensemble ou pas du tout »), sans en alourdir la table ni le relais.
- **Le worker publie sans dépendance nouvelle** : il partage la base. Pas de client Valkey à ajouter au worker.
- **Plusieurs instances de l'API** : chacune écoute, `NOTIFY` atteint tous les auditeurs. Valkey ne devient utile qu'avec un volume que `NOTIFY` porterait mal (messagerie de Guide Négo à grande échelle) ou un besoin de relecture d'historique (flux Valkey) ; ni l'un ni l'autre aujourd'hui.
- **Pas l'outbox** : elle ne couvre pas les messages internes au comité, et un signal par lot de dix courriels encombrerait une table durable faite pour être rejouée. Les deux mécanismes cohabitent : l'outbox pour les faits métier, `platform.signal` pour l'écran.
- **SQL d'abord** : `platform.signal(topic text, payload jsonb)` dans `docs/database/010_platform.sql` (nom du canal écrit une fois, contrôle de taille), migration dans `ops/migrations/`. Le canal `negotiation_channel_message` existant pourra s'y rabattre quand Guide Négo s'en servira.
- **Coalescence côté API** : plusieurs signaux d'un même sujet en moins de 500 ms partent en un seul. Protège des rafales (un envoi de mille adresses) quelle que soit la source.
- Coût de `NOTIFY` : verrou global bref au commit. Négligeable à quelques dizaines par seconde.

### 2.4 Sujets et autorisation

| Sujet | Qui s'abonne | Contrôle à l'abonnement | Émis par |
|---|---|---|---|
| `person:<id>` | soi-même, toujours | identité de la session | fin ou problème d'un envoi qu'on a lancé ; lectures faites dans un autre onglet |
| `proposal:<id>` | fiche du dossier, des deux côtés | `perimeter::acces_au_dossier` (même garde que `fil_de`), qui rend aussi le **côté** : comité ou déposant | écriture d'un message, résolution d'une demande de correction |
| `organization:<id>` | espace organisation (pastilles des onglets) | appartenance à l'organisation | message visible du déposant sur un de ses dossiers |
| `mailing:<id>` | page d'un envoi | `engagement.mailing.send` sur `Scope::Event(event_id)` | fin de chaque lot, clôture, arrêt, problème |
| `event:<id>:mailings` | liste des envois | même permission, même portée | idem, pour la liste |

- **Chaque module déclare l'autorisation de ses sujets** par un trait du `kernel` (`TopicAuthorizer`, sur le modèle de `ConsumerRegistry`) : `programme` pour `proposal:`, `engagement` pour `mailing:`. Aucun crate de module ne dépend d'un autre ; le concentrateur vit dans `api`, le type de sujet et `signal()` dans `kernel`.
- **À chaque message** : un signal marqué `audience: committee` ne part jamais vers une connexion abonnée côté déposant. Sans ce filtre, une organisation saurait qu'on parle de son dossier en interne.
- **Revalidation toutes les 60 s** : session encore valide (révocation, suspension) et sujets encore autorisés (un administrateur retiré d'un événement). Un sujet devenu interdit est retiré, la connexion le dit (`event: revoked`). Coût : une requête par connexion et par minute.
- **Fin du jeton d'accès** : le flux se ferme à l'expiration du jeton (15 min au plus) par `event: reauth` ; le client rafraîchit la session par le chemin existant et rouvre.
- **Sujet refusé** : les sujets autorisés sont servis, les refusés listés dans le premier événement (`event: ready`). Un seul sujet invalide ne coupe pas les autres.
- **Personnes connectées seulement.** Le public anonyme (direct sur `/programmations`) reste en relecture périodique d'une route mise en cache tant que la charge d'une COP n'est pas mesurée : des milliers de connexions longues sur l'Apache partagé seraient le risque principal.

### 2.5 Résilience

- **Client** : attente croissante avec gigue complète (1, 2, 4, 8, 16, 30 s au plus), remise à zéro après 60 s de connexion stable ; reprise immédiate sur `online` et au retour de l'onglet. À chaque ouverture, **relecture de tous les sujets** (`onResync`). Après cinq échecs, repli sur `usePolling` à 60 s, comme aujourd'hui, sans message à l'écran.
- **Serveur** : battement (commentaire SSE) toutes les 25 s, sous les 120 s d'Apache ; champ `retry:` ; file bornée par connexion (64), une connexion trop lente reçoit `resync` au lieu d'accumuler.
- **Perte de l'écoute Postgres** : `PgListener` se reconnecte, mais les signaux de l'intervalle sont perdus. Après chaque reconnexion, `resync` à toutes les connexions.
- **Déploiement** : à l'arrêt (SIGTERM), l'API envoie `event: restart` puis ferme ; les clients reviennent avec une gigue de 0 à 5 s pour ne pas arriver tous ensemble. Le délai d'arrêt de Docker (10 s) suffit si les flux sont fermés activement, sinon Actix attend jusqu'à 30 s des réponses qui ne finissent jamais.
- **Notification manquée** : un envoi terminé pendant que l'onglet était caché est retrouvé à la relecture. Le site garde la liste des envois lancés et pas encore clos (`sessionStorage`) et compare leur statut au retour : la notification apparaît alors, en retard mais pas perdue.

### 2.6 Coût et limites

- API : une tâche tokio et une file bornée par connexion, quelques kilo-octets ; mille connexions tiennent en quelques mégaoctets. Une connexion Postgres de plus (l'écoute), hors du pool de 16.
- **Apache est le plafond** : un fil (MPM `event` ou `worker`) ou un processus (`prefork`) par connexion ouverte, pris sur le réservoir partagé avec `financedurable`. Ordre de grandeur attendu : quelques dizaines d'administrateurs et quelques centaines de déposants connectés pendant l'appel. Plafond par personne (5 flux, le plus ancien fermé) et fermeture des onglets cachés tiennent le nombre bas. À confronter à `MaxRequestWorkers` avant la mise en ligne.
- Règle dédiée dans le vhost, **avant** `/api/` : `ProxyPass /api/realtime http://127.0.0.1:8080/api/realtime flushpackets=on`, et côté API `Content-Type: text/event-stream`, `Cache-Control: no-cache, no-transform`.

### 2.7 Observabilité

Traces `tracing` à l'ouverture et à la fermeture (personne, nombre de sujets, durée, cause), compteurs en mémoire (connexions ouvertes, signaux reçus, livrés, coalescés, refus, reconnexions de l'écoute) journalisés toutes les cinq minutes ; une route réservée au super-administrateur pour les lire si le besoin apparaît. Pas de collecteur en production : le journal suffit.

### 2.8 Tests

- Rust, sur base réelle : un message écrit est signalé en moins de deux secondes à l'abonné du dossier ; un message interne au comité n'atteint pas le côté déposant ; une organisation ne s'abonne pas au dossier d'une autre ; un administrateur d'un seul événement ne s'abonne pas à un envoi d'un autre (règle 8) ; session révoquée, flux fermé à la revalidation ; écoute coupée (`pg_terminate_backend`), `resync` reçu.
- Front (Vitest) : attente croissante, relecture à la reconnexion, fermeture à l'onglet caché, repli sur sondage.
- Production, à la main : `curl -N` à travers Apache par l'adresse publique (battement visible toutes les 25 s, aucun `Content-Encoding`), puis une reprise après `./deploy.sh update`.

### 2.9 Côté site

- **`useRealtime()`**, client seulement (plugin `realtime.client.ts`) : `subscribe(topic, { onSignal, onResync })` rendu par un composable de page, désabonné au démontage. Une seule connexion par onglet ; l'ensemble des sujets change à la navigation, la connexion est rouverte après 300 ms d'accalmie (coût : une relecture des sujets, négligeable).
- **Store** `realtime` : seulement des chaînes et des nombres (`state: 'connecting' | 'open' | 'degraded'`), aucune instance de classe (règle du payload de rendu serveur).
- Branchement : `refreshComments()` et le `refresh()` de la fiche sur `proposal:<id>` ; `usePolling` de l'espace organisation devient le repli ; `refresh()` de `pages/admin/courriels/[id].vue` sur `mailing:<id>`.
- **Toasts** : `UiToastRegion` dans `layouts/admin.vue` (et `public.vue` si l'espace organisation en reçoit), `useToasts()` (objets simples), `UiToast` aux couleurs d'état du guide (vert parti, rouge problème, jaune en cours), annonce `aria-live="polite"`, croix de 44 px, lien vers l'envoi. Placement en haut à droite sous la barre : le bas à droite porte déjà les boutons des fenêtres flottantes (`UiFloatingPanel`). Composant à ajouter au guide de style vivant et à faire valider.

---

## 3. Découpage

| Étape | Contenu | Estimation |
|---|---|---|
| 0. Mesures | MPM, `MaxRequestWorkers`, HTTP/2, modules sur le serveur ; `curl -N` d'un flux d'essai à travers Apache | 0,5 j, accès au serveur requis |
| 1. Socle | `platform.signal()` et migration ; `kernel` (sujets, `signal()`, `TopicAuthorizer`) ; concentrateur, écoute, route SSE, revalidation, arrêt propre dans `api` ; règle Apache ; `useRealtime()`, store, `UiToast` et sa région, i18n ; tests | 3 j |
| 2. Échanges d'un dossier | signal à l'écriture et à la résolution, autorisation `proposal:` et `organization:` ; fiche du back-office et espace organisation branchés, compteur de non-lus et pastilles ; sondage en repli | 1,5 j |
| 3. Envois de courriels | état « problème » dans le modèle d'abord (et sortie de l'impasse du travail `dead`) ; signaux par lot, à la clôture, à l'arrêt ; page et liste branchées ; toasts sur toutes les pages du back-office, rattrapage au retour d'onglet | 1,5 à 2 j |

Environ six à sept jours. Chaque étape se livre seule ; l'étape 1 n'a rien de visible et se met en ligne avec la 2.

---

## 4. Questions au commanditaire

Une à la fois, dans cet ordre.

**Question 1. Qui voit la notification d'un envoi de courriels ?** Quand un envoi se termine ou rencontre un problème, une petite notification apparaît quelques secondes dans un coin de l'écran.
- A. Seulement la personne qui a lancé l'envoi. Les autres voient le statut changer s'ils ont la page de l'envoi ouverte, sans être dérangés.
- B. Toutes les personnes qui gèrent l'édition, où qu'elles soient dans le back-office. Plus de bruit : avec trois envois par jour et cinq collègues, chacun reçoit quinze notifications.
- *Recommandation : A*, c'est la personne qui attend la réponse.

**Question 2. Quand prévenir pour un envoi ?**
- A. À la fin seulement : « Envoi parti : 312 messages envoyés, 3 en échec ». Et tout de suite en cas de problème.
- B. Au départ (« L'envoi a commencé ») puis à la fin. Une notification de plus, qui dit surtout ce que la personne vient de faire.
- *Recommandation : A*, le départ se voit déjà à l'écran au moment du clic.

**Question 3. Combien de temps reste une notification ?**
- A. Une bonne nouvelle disparaît seule après six secondes ; un problème reste jusqu'à ce qu'on le ferme, avec un lien vers l'envoi.
- B. Toutes disparaissent seules. Un problème peut alors passer inaperçu si la personne regardait ailleurs.
- *Recommandation : A.*

**Question 4. Un nouveau message d'une organisation doit-il être signalé ailleurs que sur la fiche du dossier ?** Ce qui est demandé : sur la fiche ouverte, le message apparaît et le compteur augmente sans recharger.
- A. Rien de plus pour l'instant.
- B. Une notification dans tout le back-office (« Nouveau message de l'organisation X sur le dossier COP31-042 »). Il faut alors décider qui la reçoit (évaluateurs du dossier ? toute l'équipe de l'édition ?), sinon chacun reçoit les messages de tous les dossiers.
- *Recommandation : A*, puis B pour les seuls évaluateurs du dossier si le besoin se confirme à l'usage.

**Question 5. Que montrer quand la mise à jour automatique est coupée** (réseau instable, mise en ligne d'une nouvelle version) ?
- A. Rien : la page se remet à jour toute seule au retour, et entre-temps elle se relit chaque minute comme aujourd'hui dans l'espace organisation.
- B. Un petit bandeau « Mise à jour automatique interrompue ». Plus honnête, mais il apparaîtra quelques secondes à chaque mise en ligne.
- *Recommandation : A.*

**Question 6. Peut-on toucher au réglage du serveur web partagé ?** Le serveur qui reçoit les visiteurs sert aussi le site Finance durable. Garder une page ouverte « à l'écoute » occupe une petite place sur ce serveur tant qu'elle reste ouverte. Il faut d'abord regarder combien de places il offre, puis ajouter une ligne de réglage pour l'ePavillon.
- A. Oui : une demi-journée de mesure et de réglage, avec un accès au serveur, avant la mise en ligne.
- B. Non : on garde la relecture régulière (toutes les 15 secondes quand la fenêtre est ouverte) partout. Plus simple, mais un message met jusqu'à 15 secondes à apparaître et l'envoi de courriels se met à jour avec le même délai.
- *Recommandation : A*, c'est la condition de tout le reste ; sans elle, on ne peut pas garantir que le site voisin ne sera jamais ralenti.
