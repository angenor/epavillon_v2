# Phase 0 — Recherche et décisions techniques

**Fonctionnalité** : Socle technique et Identité (B1) · **Date** : 2026-08-20

Seize décisions. Chacune donne ce qui a été retenu, pourquoi, et ce qui a été écarté. Aucune ne subsiste à l'état de `NEEDS CLARIFICATION`.

Les cinq premières sont celles dont une décision tacite aurait produit un défaut difficile à rattraper. La seizième est apparue en découpant les tâches, et compte autant : elle décide de ce que les cinq modules suivants pourront réutiliser.

---

## R1 — Forme du jeton d'accès : **c'est le modèle qui tranche**

**Décision** : jeton **signé** (JWT, Ed25519), durée **15 minutes**, portant uniquement l'identifiant de personne, l'identifiant de session, l'échéance et un identifiant de clé. **Aucune permission n'y figure.**

**Pourquoi** : `identity.sessions` porte `refresh_token_hash bytea NOT NULL UNIQUE` — et **rien d'autre**. Il n'existe aucune colonne où loger l'empreinte d'un jeton d'accès. Un jeton d'accès opaque exigerait donc d'ajouter une colonne au modèle, ce que le principe I interdit sans justification, et qu'aucune fonctionnalité ne réclame. Le modèle a donc décidé de la forme : ce qui n'est pas stocké doit être auto-porteur, donc signé.

Le corollaire compte autant : **les permissions ne voyagent pas dans le jeton**. Un jeton portant ses droits les fige pour sa durée de vie ; un rôle retiré continuerait de valoir un quart d'heure. Comme toute requête autorisée doit de toute façon interroger `identity.has_permission()` avec sa portée (principe V), la lecture en base ne coûte rien de plus, et FR-033 — une suspension coupe les sessions en cours — devient tenable.

Le jeton d'accès est donc **un renvoi signé vers une session**, pas un porteur de droits.

**Alternatives écartées** :

- *Jeton d'accès opaque vérifié en base* — exigerait une colonne qui n'existe pas.
- *JWT portant les permissions* — révocation différée d'un quart d'heure, pour une économie nulle puisque la portée se vérifie en base de toute façon.
- *Réutiliser le jeton de rafraîchissement comme porteur* — annulerait la rotation (R3) et exposerait le jeton long à chaque requête.

---

## R2 — Transport de la session : deux cookies, deux portées

**Décision** :

| Cookie | Contenu | Attributs | Durée |
|---|---|---|---|
| `epavillon_at` | jeton d'accès signé | `HttpOnly`, `Secure`, `SameSite=Lax`, `Path=/` | 15 min |
| `epavillon_rt` | jeton de rafraîchissement | `HttpOnly`, `Secure`, `SameSite=Strict`, **`Path=/api/auth`** | 12 h, ou 30 j |

**Pourquoi** : le front l'a déjà écrit — « l'API pose un cookie `HttpOnly` que le navigateur renvoie seul », et son client HTTP porte `credentials: 'include'`. Aucun écran ne lit ni ne compose d'en-tête d'autorisation : un jeton porté par en-tête casserait le contrat.

Le chemin restreint du jeton de rafraîchissement est ce qui limite le dégât d'une éventuelle fuite par une autre route : il n'est envoyé qu'aux routes de session.

**Le nom `epavillon_session` n'est pas repris**, bien qu'il soit celui du cookie que tient le front aujourd'hui : c'est un cookie de démonstration, lisible et falsifiable, qui disparaît en B7. Réutiliser son nom ferait cohabiter deux valeurs de forme différente sous une même clé pendant le raccordement.

**Protection contre la requête forgée** : `SameSite` plus **vérification de l'origine annoncée sur toute écriture**. Pas de jeton anti-CSRF en en-tête — aucun écran n'en envoie, l'exiger casserait le front sans arbitrage.

*Précisé à l'implémentation, le 20/08 — la règle ne disait pas ce qu'il advient d'une requête qui n'annonce rien* :

| Ce qu'annonce l'écriture | Traitement |
|---|---|
| `Origin` connue | passe |
| `Origin` inconnue, **`null` compris** | **refusée** — une valeur littérale `null` vient d'une iframe cloisonnée ou d'une redirection inter-schémas ; ce n'est pas une absence |
| Pas d'`Origin`, mais un `Referer` | on retient le schéma et l'autorité du `Referer`, et on applique la même règle |
| Ni l'un ni l'autre | **passe** — les navigateurs posent un `Origin` sur toute écriture, donc l'absence des deux désigne un client qui n'est pas un navigateur, et qui n'est pas le vecteur qu'on ferme ici. Refuser aurait cassé toutes les vérifications par `curl` de `quickstart.md` sans rien protéger |

**Alternatives écartées** :

- *Jeton porté par en-tête d'autorisation* — contredit le front existant, et rend le jeton lisible par tout script de la page.
- *Un seul cookie portant les deux jetons* — perdrait la restriction de chemin, qui est l'essentiel du bénéfice.
- *Jeton anti-CSRF en en-tête* — reporté : il exige une modification du front, donc un arbitrage, donc B7 au plus tôt.

---

## R3 — Rotation du jeton de rafraîchissement et détection de rejeu

**Décision** : chaque renouvellement **révoque la session courante et en ouvre une nouvelle**, avec un jeton neuf. Présenter un jeton dont la session est déjà révoquée pour cause de renouvellement fait **révoquer toutes les sessions vivantes de la personne**.

**Pourquoi** : `identity.sessions` porte `revoked_at` et `revoked_reason` — la rotation s'écrit donc dans le modèle tel quel, en chaînant des lignes, sans colonne supplémentaire. Le motif de révocation distingue les cas : `rotated`, `logout`, `logout_all`, `reuse_detected`, `password_changed`, `status_changed`, plus `anonymization` déjà écrit par `identity.anonymize_person()`. *Corrigé le 20/08 : cette liste disait `suspended` là où le code écrit `status_changed` — une suspension et une exclusion coupent les sessions de la même façon, et un seul motif les couvre — et oubliait `logout_all`. La liste qui fait foi est celle de [data-model.md](data-model.md).*

Un jeton présenté deux fois n'a **aucune explication innocente** : soit il a été volé, soit une copie de la session circule. La seule réponse sûre est de tout couper.

**Alternatives écartées** :

- *Jeton de rafraîchissement réutilisable jusqu'à son échéance* — un jeton volé vaudrait trente jours sans que rien ne le signale.
- *Fenêtre de tolérance au rejeu* (accepter un rejeu dans les quelques secondes, pour les requêtes concurrentes) — ajoute un état à gérer pour un cas que le front ne produit pas : il n'y a qu'un seul appel de renouvellement en vol à la fois.

---

## R4 — Vérification à la compilation et requêtes préparées

**Décision** : macros SQLx à vérification à la compilation, avec `cargo sqlx prepare --workspace` produisant **`backend/.sqlx/`, versionné**.

**Pourquoi** : `.env.example` l'annonce déjà — « lue par SQLx À LA COMPILATION : la base doit tourner avant `cargo build` ». C'est le prix du principe VI, et ce qui fait qu'un nom de colonne inventé échoue au `cargo build` plutôt qu'en production.

Le dossier préparé est versionné pour deux raisons pratiques : l'éditeur d'un poste sans base démarrée cesse de rougir, et une future intégration continue peut construire sans PostgreSQL. Il ne dispense de rien : `cargo test` exige une base réelle (principe X), et `make check-back` tourne après `make check-db`, donc base démarrée.

**Règle d'hygiène** : toute modification d'une requête impose de régénérer `.sqlx/` dans le même commit. Un dossier préparé périmé fait passer une construction que la base refuserait — exactement le contraire de ce qu'on cherche.

**Alternatives écartées** : *ne pas versionner `.sqlx/`* — rend l'édition pénible et interdit toute construction hors machine de développement.

---

## R5 — Égalisation du temps de réponse à la connexion

**Décision** : quand l'adresse est inconnue, ou que la personne n'a pas de compte mot de passe, le service **vérifie quand même** le mot de passe fourni contre une **empreinte factice** calculée une fois au démarrage, avec les mêmes paramètres Argon2id. Le résultat est ignoré ; seul le temps compte.

**Pourquoi** : c'est l'écart n° 1 relevé en écrivant la spécification. Les données simulées tiennent la règle de discrétion sans effort — elles comparent deux chaînes. Argon2id coûte 50 à 100 ms ; abandonner avant de le calculer rend la réponse dix à cent fois plus rapide, et le formulaire de connexion redevient l'annuaire des comptes qu'on voulait fermer. Le message identique ne suffit pas : **le temps parle aussi**.

Mesuré par SC-001 : écart inférieur à 10 % sur cent tentatives de chaque sorte.

**Alternatives écartées** :

- *Retard aléatoire* — masque mal (la moyenne reste distincte) et ajoute de la latence sans rien protéger.
- *Retard constant jusqu'à un plancher* — tenable, mais fixe le coût le plus élevé pour tout le monde et se dérègle silencieusement le jour où les paramètres Argon2id changent.

---

## R6 — Paramètres Argon2id

**Décision** : Argon2id, **mémoire 19 MiB, 2 itérations, parallélisme 1**, sel aléatoire de 16 octets, empreinte encodée au format PHC dans `accounts.password_hash`.

**Pourquoi** : ce sont les paramètres recommandés par l'OWASP pour Argon2id, et ils placent le coût dans la fourchette visée de 50 à 100 ms sur le matériel cible. Le format PHC porte ses propres paramètres : le jour où ils sont durcis, les empreintes existantes restent vérifiables. Le **ré-encodage** d'une ancienne empreinte à la connexion réussie n'est pas livré dans ce jalon, et n'aurait rien à réencoder : aucune route de B1 ne crée ni ne change de mot de passe — l'inscription et la réinitialisation arrivent en phase 6 —, et les paramètres n'ont jamais bougé. Il se posera avec le premier durcissement, au même rang que la rotation des clés de signature.

`COMMENT ON COLUMN accounts.password_hash` le dit déjà : « aucune fonction SQL ne doit vérifier de mot de passe ». Le calcul et la comparaison vivent entièrement dans le service.

**Alternatives écartées** : *bcrypt* — moins résistant au matériel spécialisé, et le prompt impose Argon2id.

---

## R7 — Traduction des erreurs de la base

**Décision** : une table de correspondance dans le noyau, `(SQLSTATE, nom de contrainte) → (code stable, message français, champ fautif)`. Détail dans [`contracts/errors.md`](contracts/errors.md).

**Deux cas, deux traitements** :

1. **Violation de contrainte** (`23505` unicité, `23514` vérification, `23P01` exclusion) — le nom de la contrainte est connu et documenté ; la correspondance donne le code et le message.
   *Précisé le 20/08* : **`23P01` n'est pas couvert dans B1**, et c'est sans conséquence — les trois contraintes `ex_*` du modèle vivent dans `publication` et `negotiation`. Le module qui ouvrira l'une d'elles complète la table de `contracts/errors.md`, faute de quoi son refus sortira en `INTERNAL`.
   *Précisé le 20/08 également* : **un domaine à CHECK lève `23514`, pas `22P02`**, et son refus ne porte ni table ni colonne. La ligne `22P02` de la table était fausse pour `platform.email`.
2. **Exception levée par un trigger du modèle** — `identity.tg_check_role_scope()` lève avec `ERRCODE = 'restrict_violation'` et **un message déjà écrit en français**, portant le rôle, la portée refusée et les portées autorisées. Ce message est **repris tel quel**. Le réécrire côté service produirait deux formulations pour un même refus, et la seconde se périmerait à la première évolution du modèle.

**Pourquoi** : c'est le principe VIII, littéralement. Le cas de la contrainte de forme du type d'événement (`ck_outbox_event_type_format`, trois segments exactement) en est l'exemple d'école : le code ne revalide pas la forme, il traduit l'échec.

**Alternative écartée** : *revalider en Rust avant d'écrire* — double la règle, et la copie diverge dès la première modification du SQL.

---

## R8 — Le chemin du jeton en clair jusqu'au courriel

**Décision** :

1. Le service crée le jeton, en stocke **l'empreinte** dans `identity.one_time_tokens`, et **met en file un travail différé** (`platform.jobs`) dont la charge utile porte le jeton **en clair**, dans la **même transaction**.
2. Il émet **en plus** l'événement de domaine (`platform.emit_event()`), qui **ne porte aucun secret**.
3. Le worker envoie le courriel, puis marque le travail réussi **en vidant sa charge utile**.

**Pourquoi** : la question n'est pas anodine. Le courriel doit contenir le jeton en clair, or la base n'en garde que l'empreinte — le clair doit donc voyager quelque part.

- Il **ne passe pas par l'outbox** : `platform.outbox_events` est une table durable, interrogeable, indexée par agrégat, et destinée à devenir un bus. Y déposer un secret réutilisable serait un défaut permanent.
- Il **passe par `platform.jobs`**, que la constitution désigne explicitement pour « un travail différé qui n'annonce pas un changement d'état — envoi de rappel… ». La charge utile est effacée dès l'envoi : le travail garde sa trace (« un courriel est parti »), pas son contenu.
- **Ce n'est pas un appel inter-module.** Le crate `engagement` n'existe pas avant B6 ; l'envoi appartient donc à `identity`, qui le confie au worker. Aucune arête interdite n'est créée.

`jobs.idempotency_key` porte l'identifiant du jeton : deux demandes du même courriel n'en envoient qu'un.

**Alternatives écartées** :

- *Jeton en clair dans l'outbox* — secret durable dans une table faite pour être relue et rejouée.
- *Envoi synchrone pendant la requête* — couple la réponse HTTP à la disponibilité du serveur de courriel, et fait échouer une inscription parce qu'un relais est lent.
- *Attendre B6 et le module `engagement`* — B1 deviendrait indémontrable : ni vérification d'adresse, ni réinitialisation.

**À reprendre en B6** : `engagement` porte les modèles de message multilingues, le suivi des envois et les rebonds. Ce saut disparaît alors ; la trace laissée ici doit permettre de le retirer sans archéologie.

---

## R9 — Relais d'outbox

**Décision** : un connecteur dédié en écoute sur `platform_outbox` (`LISTEN`/`NOTIFY`), **doublé d'un balayage périodique** toutes les 10 secondes. Réservation par `FOR UPDATE SKIP LOCKED` sur les lignes non publiées et disponibles. Après remise à tous les consommateurs, `published_at` est posée ; en cas d'échec, `attempts` est incrémenté, `last_error` renseigné, et `available_at` repoussée avec un délai croissant.

**Pourquoi la notification ET le balayage** : `platform.emit_event()` appelle déjà `pg_notify` — c'est ce qui évite d'attendre le prochain cycle. Mais une notification est perdue si le relais est arrêté au moment où elle part, et elle n'est pas rejouée. Le balayage est le filet ; la notification est la vitesse.

**Idempotence** : chaque consommateur inscrit `(consommateur, événement)` dans `platform.inbox_events`. Un conflit sur cette clé signifie « déjà traité » et fait passer au suivant sans produire d'effet. C'est le principe IV, et l'écart n° 4 relevé en écrivant la constitution.

**Alternatives écartées** :

- *Balayage seul* — jusqu'à 10 secondes de retard sur un courriel de vérification, ressenti comme une panne.
- *Notification seule* — perd les événements écrits pendant un redémarrage.

---

## R10 — File de travaux

**Décision** : boucle de réservation par `platform.claim_jobs(file, identifiant du worker, lot)`, registre de gestionnaires indexé par le nom de la tâche, `platform.fail_job()` en cas d'échec, et marquage `succeeded` avec `completed_at` en cas de réussite.

Quatre tâches dans ce module :

| Tâche | Déclencheur | Clé d'unicité |
|---|---|---|
| `identity.send_verification_email` | inscription, renvoi de lien | identifiant du jeton |
| `identity.send_password_reset_email` | demande de réinitialisation | identifiant du jeton |
| `identity.send_existing_account_notice` | inscription sur une adresse déjà connue | identifiant de personne + jour |
| `identity.purge_expired_tokens` | récurrente, quotidienne | date du jour |

**Pourquoi la purge** : `ix_one_time_tokens_cleanup` existe en base — un index partiel sur `expires_at WHERE consumed_at IS NULL`. Quelqu'un avait donc prévu de nettoyer, et aucun prompt ne le demandait. C'est aussi le premier usage réel de `platform.jobs` dans ce module : la file est éprouvée sans attendre B6.

**La replanification n'est pas réécrite** : `platform.fail_job()` porte déjà le délai croissant plafonné à une heure et le passage en file morte au-delà de `max_attempts`. Principe VIII.

---

## R11 — Configuration du service : les écarts n° 18 et 19

**Décision** : une structure de configuration typée, chargée de l'environnement au démarrage par `figment`, et **validée à ce moment-là** — une durée mal écrite fait échouer le démarrage, jamais une requête.

Nouvelles clés, à ajouter à `.env.example` :

```
# --- Authentification : réglages d'exploitation (écarts n° 18 et 19) ---------
AUTH_LOCKOUT_THRESHOLD=5
AUTH_LOCKOUT_DURATION=15m
AUTH_ACCESS_TOKEN_TTL=15m
AUTH_SESSION_TTL=12h
AUTH_SESSION_TTL_REMEMBERED=30d
AUTH_TOKEN_TTL_EMAIL_VERIFICATION=24h
AUTH_TOKEN_TTL_PASSWORD_RESET=1h
AUTH_TOKEN_TTL_INVITATION=7d
AUTH_TOKEN_TTL_MAGIC_LINK=15m
AUTH_TOKEN_TTL_SPEAKER_CONFIRMATION=14d
AUTH_SIGNING_KEY=
AUTH_COOKIE_SECURE=false
AUTH_COOKIE_DOMAIN=
APP_PUBLIC_URL=http://localhost:3000
WORKER_ID=
```

**Pourquoi pas en base** : c'est l'argument même des écarts n° 18 et 19. `platform.settings` existe et serait techniquement possible, mais un seuil de verrouillage lu en base à chaque connexion devient un point de panne et une donnée modifiable sans trace de déploiement. Un réglage d'exploitation se change avec un redémarrage, pas avec un `UPDATE`.

**Les durées de jeton sont indexées par finalité**, une entrée par valeur de `identity.token_purpose`. Aucun appelant ne peut poser une expiration lui-même (FR-018) : la fonction qui crée un jeton prend la finalité et lit la durée. C'est exactement ce que l'écart n° 19 demandait — sinon « deux liens de finalités différentes vivront des durées différentes sans que personne l'ait décidé ».

**`APP_PUBLIC_URL`** sert à composer les liens des courriels. C'est l'adresse du **front**, pas de l'API : un lien de vérification mène à un écran, pas à une route.

---

## R12 — Limitation de débit et Valkey : **hors périmètre, et c'est écrit**

**Décision** : aucune limitation de débit dans B1. **Valkey n'est pas utilisé par ce module.**

**Pourquoi c'est dit plutôt que tu** : le verrouillage de compte (écart n° 18) protège **un compte** contre l'essai répété d'un mot de passe. Il ne protège pas contre l'énumération distribuée — un attaquant qui essaie un mot de passe courant sur dix mille adresses ne verrouille rien. Aucune exigence de la spécification ne le demande, et l'ajouter sans arbitrage élargirait le jalon.

Le point est nommé ici pour qu'une session ultérieure ne le redécouvre pas, et ne conclue pas non plus que Valkey a été oublié.

---

## R13 — Envoi de courriel : **l'API n'envoie rien, elle fait envoyer**

**Contrainte d'hébergement, énoncée par le commanditaire le 20/08** : l'API et le site ne vivent pas sur le même serveur, et **seul le serveur du site (Nuxt) a le droit d'émettre du courriel**. Les identifiants SMTP sont dans son environnement à lui.

**Décision** : le worker **compose** le message et le **remet au serveur Nuxt** par un appel HTTP privé, authentifié par un secret partagé. Nuxt ne fait que transporter : il ouvre la connexion SMTP et envoie.

```
identity (service)  ──► platform.jobs          (même transaction, jeton en clair)
worker              ──► POST {MAIL_RELAY_URL}  (sujet + corps déjà composés)
Nuxt (route privée) ──► SMTP                   (Mailpit en local)
```

**Qui compose, et pourquoi ce n'est pas Nuxt** : le message part avec son sujet et son corps déjà écrits, dans la langue de `people.preferred_locale`. Nuxt reçoit un texte, pas un gabarit à remplir. Deux raisons : le texte appartient au module qui déclenche l'envoi, et en B6 la composition passe aux modèles administrables de `engagement.message_templates` — si Nuxt composait, il faudrait alors défaire son travail. **Nuxt est un tuyau, et doit le rester.**

**Ce que la reprise d'essai devient** : rien ne change. Un serveur Nuxt injoignable est un travail différé en échec ; `platform.fail_job()` le replanifie avec son délai croissant et le met en file morte au bout de cinq essais. La file reste au même endroit, et l'exploitation la surveille par `analytics.v_operational_health`.

**Ce que cela coûte, et comment c'est borné** :

| Risque | Réponse |
|---|---|
| Le jeton en clair traverse le réseau | HTTPS **obligatoire** en production. En local, les deux serveurs sont sur la même machine |
| Une route privée exposée sur un serveur public | Secret partagé en en-tête, comparaison à temps constant ; toute requête sans secret valide reçoit 404 — **pas 401** : une route privée ne confirme pas son existence |
| Un double envoi après une reprise sur délai d'attente | Le message porte l'identifiant du travail ; Nuxt retient les identifiants déjà envoyés pendant quelques minutes et ignore un doublon |
| L'API doit joindre le serveur du site | **Sens de connexion confirmé le 20/08** par le commanditaire : c'est l'API qui appelle le site |

**Nouvelles clés de configuration** — côté API : **`MAIL_TRANSPORT`** (`relay` aujourd'hui, `smtp` le jour de l'autorisation), `MAIL_RELAY_URL`, `MAIL_RELAY_TOKEN`. Côté Nuxt : le même secret, plus les identifiants SMTP, **sans le préfixe public** — rien de tout cela n'atteint le navigateur.

`MAIL_TRANSPORT` est la clé annoncée plus bas comme « une clé de configuration » : elle est nommée ici pour qu'on n'ait pas à la deviner. La variante `smtp` **existe déjà** dans le noyau et refuse d'envoyer, avec un message explicite.

Les clés `SMTP_HOST`, `SMTP_PORT` et `SMTP_FROM` de `.env.example` restent, mais **changent de lecteur** : elles étaient annoncées pour l'API, elles sont lues par Nuxt. Le commentaire du fichier doit le dire, sinon la session suivante les câblera au mauvais endroit.

**Ce que cela ajoute au livrable** : une route serveur dans le front, `frontend/server/api/internal/mail.post.ts` — quelques dizaines de lignes, sans interface. C'est le seul morceau de B1 qui vit hors de `backend/`, et il y est parce que la contrainte d'hébergement l'y met, pas par commodité.

### Ce relais sert TOUS les courriels, et il est fait pour disparaître

Deux précisions apportées par le commanditaire le 20/08, et qui décident de la forme du code :

**1. Il ne sert pas qu'à l'inscription.** Tout courriel de la plateforme passe par là — vérification d'adresse, réinitialisation, rappel de compte existant, et demain les invitations, les convocations d'intervenants, les rappels de séance. La petite fonction TypeScript est donc écrite **une fois, générique** : elle reçoit un destinataire, un sujet, un corps, et elle envoie. Elle ne connaît aucun cas particulier et n'en connaîtra jamais.

**2. Elle est temporaire.** Le jour où le serveur de l'API obtient le droit d'émettre, l'envoi est réécrit en Rust et la route Nuxt disparaît.

C'est ce second point qui impose la forme côté API : le noyau expose **un contrat d'envoi**, pas un client HTTP. Deux implémentations sont prévues dès maintenant, choisies par la configuration :

| Implémentation | Quand | Ce qui change ailleurs |
|---|---|---|
| remise au serveur Nuxt | aujourd'hui | — |
| SMTP direct depuis le worker | le jour de l'autorisation | **rien** : une clé de configuration, une implémentation de plus dans le noyau |

Aucun module ne sait par où part son courriel. `identity` met un message en file ; ce qui l'achemine ne le regarde pas. Sans cette séparation, la bascule obligerait à rouvrir chaque module qui envoie un courriel — c'est-à-dire, à terme, presque tous.

**Alternatives écartées** :

- *SMTP direct depuis le worker* — impossible : le serveur de l'API n'a pas le droit d'émettre.
- *Le site vient chercher les messages à intervalle régulier* — écarté par le commanditaire le 20/08. Aurait évité que l'API ait à joindre le site, mais retardait chaque courriel d'une minute et déplaçait la reprise d'essai hors de la file, là où elle n'est pas surveillée.
- *Un service de courriel tiers appelé directement par l'API* — contournerait la contrainte plutôt que de la respecter, et ajouterait un fournisseur non arbitré.

**Mailpit capture tout et n'envoie rien** : en local, Nuxt lui parle, et les parcours se vérifient à l'interface `http://localhost:8025`. La chaîne complète — API, worker, route Nuxt, Mailpit — est donc éprouvable sur une seule machine.

---

## R14 — Contexte de requête et transaction en écriture

**Décision** : le noyau expose **une seule** façon d'ouvrir une transaction en écriture. Elle prend le contexte de requête et pose elle-même, avant de rendre la main :

```
SET LOCAL app.actor_id  = <acteur, ou vide>
SET LOCAL app.request_id = <identifiant de requête>
```

*Précisé à l'implémentation, le 20/08* : **rien dans les types n'interdit d'ouvrir une transaction ailleurs.** Le pool reste accessible — l'écoute `LISTEN/NOTIFY`, le chargement des tables de référence et le harnais de test en exigent un vrai. La porte unique est donc une discipline, pas une clôture ; ce qu'elle garantit, c'est qu'aucune écriture d'une table **auditée** ne se fait sans contexte. Les écritures d'exploitation du worker — réservation de travaux, replanification d'un événement — passent par elle aussi, bien que leurs tables ne portent pas de trigger d'audit : le précédent compte plus que le gain.

**Pourquoi la rendre unique** : le principe VII est une obligation qu'on oublie sans que rien n'échoue — `platform.emit_event()` agrège l'acteur dans les métadonnées et `platform.tg_audit()` dans le journal ; sans contexte, les deux écrivent **anonyme, sans erreur**. C'est l'écart n° 3 relevé en écrivant la constitution : le principe IV ne tient pas sans le VII. Une règle qu'aucun mécanisme ne rappelle finit par ne plus être suivie ; ici, ne pas la suivre demande d'écrire du code exprès.

L'identifiant de requête est lu de l'en-tête `X-Request-Id` s'il est présent, engendré sinon, posé dans le contexte, dans la trace, dans la réponse, et dans la transaction. Il relie donc une page du navigateur, une trace Jaeger et une ligne d'audit.

**La locale** suit le même chemin : négociée depuis `Accept-Language` contre `reference.locales`, avec repli sur le français, elle est passée comme argument à `platform.t()` dans les requêtes qui lisent un texte du modèle. Aucune résolution côté Rust, aucun libellé recopié dans un fichier de traduction.

---

## R15 — Tests d'intégration sur base réelle

**Décision** : au premier test, une **base modèle** est créée une fois et chargée depuis `docs/database/` dans l'ordre de numérotation. Chaque test crée ensuite sa propre base par recopie (`CREATE DATABASE … TEMPLATE …`) et la supprime en sortant.

**Pourquoi** : le principe X interdit tout double en mémoire — « la moitié des invariants de ce projet vit dans la base ». Recharger dix-neuf fichiers avant chaque test coûterait plusieurs secondes chacun ; la recopie d'une base modèle coûte quelques dizaines de millisecondes, pour une base **identique**, triggers et contraintes compris.

**Écarté** : *le harnais de test intégré à SQLx*, qui attend une arborescence de migrations à sa convention. Nos fichiers portent une numérotation qui s'en approche, mais `900_seed.sql` et `910_migration_v1.sql` ne sont pas des migrations de schéma, et faire dépendre les tests d'une coïncidence de nommage est fragile. Le harnais maison tient en peu de lignes et dit exactement ce qu'il fait.

*Deux pièges rencontrés à l'implémentation, le 20/08, et refermés* :

- **Le modèle se périmait en silence.** Créé une fois et jamais rafraîchi, il continuait de servir l'ancien schéma après toute modification de `docs/database/` — le piège que `CLAUDE.md` signale pour le conteneur, reproduit dans le harnais. Le nom du modèle porte désormais **l'empreinte du SQL qu'il contient** (`epavillon_test_template_<empreinte>`) : changer le SQL change le nom, donc reconstruit le modèle, sans consigne à retenir. Les modèles d'une version antérieure sont supprimés au passage. À noter tout de même : `make check` commence par `down -v`, donc la porte de livraison ne pouvait pas valider sur un schéma périmé — le piège ne mordait que sur un `cargo test` lancé seul.
- **Le harnais ne peut pas fermer son pool avant de supprimer sa base.** La suppression doit être terminée quand le test rend la main, donc elle bloque son fil ; or fermer un pool SQLx s'appuie sur le runtime que ce fil vient de bloquer. `DROP DATABASE … WITH (FORCE)` termine les connexions côté serveur et règle le même problème sans rien attendre.
- **Le chargement et la création ne sont pas solidaires.** `CREATE DATABASE` n'est pas transactionnel : un fichier SQL en échec laissait un modèle amputé que plus rien ne reconstruisait. Le modèle est maintenant supprimé si son chargement échoue.

**Écarté aussi** : *une base unique partagée entre tests*, avec transaction annulée en fin de test. Séduisant, mais impossible ici : plusieurs comportements à vérifier — le relais d'outbox, les travaux différés, la notification — franchissent la frontière de transaction par construction.

Les quatre obligations minimales de la constitution (chemin nominal, refus par périmètre sur URL forgée, traduction d'un invariant, écriture dans l'outbox) ont chacune leur test nommé dans [`quickstart.md`](quickstart.md).

---

## R16 — Le garde d'autorisation vit dans le noyau

*Décision prise en découpant les tâches, après la phase 1.*

**Décision** : l'extracteur qui exige une permission, et celui qui borne au périmètre d'administration, vivent dans `kernel`, pas dans `identity`.

**Pourquoi** : l'autorisation se teste par `identity.has_permission()` et `identity.administered_events()`, **deux fonctions SQL**. Tous les modules à venir doivent les appeler — B2 borne ses organisations, B3 ses événements, B4 ses propositions. Or **aucun crate de module n'a le droit de dépendre du crate `identity`** (principe II). Placer le garde dans `identity` créerait donc, dès B2, exactement l'arête interdite.

Ce n'est pas une entorse : `kernel` connaît le schéma `identity` comme il connaît `platform`. Il ne dépend d'aucun crate de module, et le graphe reste sans arête entre modules. Le modèle avait d'ailleurs déjà tranché en posant l'autorisation comme **fonction de base**, appelable par n'importe qui, et non comme un service qu'il faudrait joindre.

**Ce que cela évite** : sans cette décision, B2 aurait eu trois issues, toutes mauvaises — dépendre d'`identity` (interdit), recopier l'appel SQL (duplication qui diverge), ou tester un nom de rôle (contraire au principe V, et déjà à l'origine d'une correction en A8, où le rôle `reviewer` se voyait refuser des dossiers qu'on lui avait confiés).

**Alternatives écartées** :

- *Le garde dans `identity`, exposé par `contracts`* — les contrats portent des **charges utiles d'événements**, pas du code d'accès à la base. Les élargir en ferait un second noyau.
- *Chaque module réécrit son appel* — la même requête dans six crates, et les trois cas du périmètre à ne pas confondre à six endroits. C'est ce genre de duplication qui a produit le contrôle d'accès silencieux corrigé le 16/08.

---

## Ce qui reste ouvert, et pour quand

| Point | Pourquoi il reste ouvert | Échéance |
|---|---|---|
| Limitation de débit sur les points d'entrée publics | aucune exigence de la spécification ; élargirait le jalon sans arbitrage | à poser au commanditaire |
| Rotation des clés de signature | une seule clé suffit au jalon ; le format retenu porte déjà un identifiant de clé, la rotation reste donc possible sans casser le contrat | avant mise en production |
| Reprise de l'envoi de courriel par `engagement` | le crate n'existe pas avant B6 | **B6** |
| Un administrateur qui se retire son dernier rôle | cas limite consigné dans la spécification ; rien en base ne l'empêche | à trancher en implémentant FR-053 |
| Jeton anti-CSRF en en-tête | exigerait une modification du front, donc un arbitrage | **B7** |
