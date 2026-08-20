# B1 — Socle + Identité

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de ce module est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md) § PHASE B — B1.

**État** : 🟨 spécifiée, planifiée, découpée, **et les phases 1 à 4 sont implémentées** le 20/08 — `backend/` existe et l'on s'y connecte, 58 tâches sur 126 (T001–T058)

---

## Ce qui a été livré

`/speckit-specify` a produit **`specs/001-socle-identite/`** :

| Fichier | Contenu |
|---------|---------|
| `spec.md` | 8 histoires utilisateur priorisées, 13 cas limites, **64 exigences fonctionnelles** en 9 groupes, 12 entités du modèle, 13 critères de réussite mesurables, 11 hypothèses, et les vérifications faites en chemin |
| `checklists/requirements.md` | Contrôle qualité de la spécification — tous les points passent, avec deux écarts assumés et motivés |

`.specify/feature.json` pointe désormais sur `specs/001-socle-identite` (fichier ignoré par git, propre à chaque poste).

**Aucun code n'était écrit à ce stade** : `backend/` n'existait pas encore le matin du 20/08. La spécification décrit ce que le module doit faire, pas comment. *(Les sections suivantes, datées elles aussi, racontent l'implémentation.)*

### Les quatre exigences du prompt, et où elles atterrissent

| Exigence | Traitement |
|---|---|
| **Écart n° 18** — verrouillage sans seuil | FR-014 à FR-016. Seuil et durée **dans la configuration du service**, jamais en base. Défauts : **5 échecs, 15 minutes** (les mocks retenaient 12 minutes, valeur explicitement inventée). La réponse `locked` porte la date de fin, que l'écran sait déjà rendre. FR-015 tranche la question laissée ouverte — **qui remet le compteur à zéro** : une connexion réussie, l'expiration du verrou, une réinitialisation menée à terme |
| **Écart n° 19** — durée de validité d'un jeton | FR-017 et FR-018. **Une durée par finalité**, déclarées au même endroit. Vérification d'adresse **24 h**, réinitialisation **1 h** — les deux que l'écart tranche explicitement. Les trois autres finalités reçoivent un défaut faute d'écran qui les consomme dans ce jalon. FR-018 interdit à un appelant de poser lui-même une expiration |
| **Écart n° 20** — adresse non vérifiée | FR-024. La règle est confirmée et devient une exigence de l'API. **Aucun statut de personne n'est ajouté** — l'état reste porté par `people.email_verified_at`, comme l'écart le demandait |
| **Ordre des contrôles de connexion** | FR-019 à FR-021, plus FR-020 qui va au-delà de l'énoncé : le coût de vérification se paie **même sur une adresse inconnue**, sinon la durée de réponse redevient un oracle d'existence de comptes. SC-001 le mesure — écart de temps inférieur à 10 % sur cent tentatives de chaque sorte |

### Puis `/speckit-plan`, le même jour

| Fichier | Contenu |
|---------|---------|
| `plan.md` | contexte technique, **contrôle des dix principes de la constitution avant et après conception**, arborescence des cinq crates, ce que le plan ne tranche pas |
| `research.md` | **16 décisions techniques**, chacune avec ses alternatives écartées |
| `data-model.md` | ce que le code fait du modèle existant, les quatre machines à états, la correspondance Rust ↔ base, et d'où vient chaque forme attendue par le front |
| `contracts/routes.md` | ~30 routes, leur autorisation, et **la politique de statut HTTP** |
| `contracts/errors.md` | catalogue de codes stables + **traduction des contraintes PostgreSQL** |
| `contracts/events.md` | 9 événements de domaine, 4 travaux différés, la remise du courriel au site |
| `quickstart.md` | lancer, éprouver à la main, et les 16 tests nommés |

**Verdict du contrôle constitutionnel : aucune violation, `Complexity Tracking` vide.**

### Puis `/speckit-tasks`, toujours le même jour

`tasks.md` — **126 tâches en 11 phases, dont 23 tests**. Une phase par histoire utilisateur, chacune
éprouvable seule et fermée par un point de contrôle. Les tests ne sont pas optionnels : le principe X
impose des tests d'intégration sur base réelle, et ses **quatre obligations minimales** ont chacune leur
tâche nommée — chemin nominal (T046, T061), refus sur URL forgée (T066), invariant de la base traduit
(T098), écriture dans l'outbox (T103).

**Trois jalons de livraison**, et le deuxième est celui qui compte pour la suite du projet :

| Jalon | Tâches | Ce qu'on peut faire |
|---|---|---|
| Le plus petit incrément qui vaille | T001–T047 | se connecter, sans que le formulaire dise quels comptes existent |
| **Le socle qui débloque B2 à B6** | **T001–T067** | un appelant identifié, une permission testable, un périmètre borné — les cinq modules suivants ont tout ce qu'il leur faut |
| Le module complet | T001–T115 | l'appel à propositions de la COP31 peut s'ouvrir |

Les trois décisions qui structurent le reste :

1. **Le jeton d'accès est signé, pas stocké — et c'est le MODÈLE qui l'a décidé.** `identity.sessions` ne porte qu'une empreinte de jeton de **rafraîchissement** : aucune colonne n'existe où loger un jeton d'accès opaque. Ce qui n'est pas stocké doit être auto-porteur. Corollaire : le jeton **ne transporte aucune permission** — un jeton portant ses droits les figerait un quart d'heure, alors que la portée se vérifie en base de toute façon.
2. **Le rejeu d'un jeton de rafraîchissement révoque toutes les sessions.** Un jeton présenté deux fois n'a aucune explication innocente.
3. **L'API n'enverra aucun courriel elle-même** — voir l'écart n° 8 ci-dessous.

---

## Écarts constatés en spécifiant, puis en planifiant

Les sept premiers sont apparus en écrivant `spec.md`, les quatre suivants en écrivant `plan.md`, le dernier en découpant `tasks.md`.

1. **LA DISCRÉTION SE TRAHIT PAR LE TEMPS, PAS SEULEMENT PAR LE MESSAGE.** Le prompt, l'écran A1 et
   les mocks énoncent tous la règle en termes de *réponse* : même message pour une adresse inconnue et
   pour un mot de passe faux. Le mock la tient trivialement — il compare deux chaînes. Une vraie
   implémentation Argon2id ne la tient **pas** : abandonner avant de calculer l'empreinte quand
   l'adresse est inconnue rend la réponse dix à cent fois plus rapide, et le formulaire redevient
   l'annuaire des comptes qu'on voulait fermer. **Ajouté comme exigence (FR-020) et comme critère
   mesurable (SC-001)**, alors qu'aucune des trois sources ne le mentionne.

2. **LE REJEU D'UN JETON DE RAFRAÎCHISSEMENT N'ÉTAIT SPÉCIFIÉ NULLE PART.** Le prompt demande des
   jetons « hachés et révocables », ce qui couvre le vol de la base et la déconnexion — pas le vol du
   jeton lui-même. `identity.sessions` porte `revoked_at` et `revoked_reason` mais ne dit rien de la
   rotation. **FR-031 tranche** : le renouvellement fait tourner le jeton, et le rejeu d'un jeton déjà
   consommé révoque **toutes** les sessions de la personne. Sans cela, un jeton volé vaut aussi
   longtemps que sa session, et rien ne le signale.

3. **UNE SESSION OUVERTE SURVIVAIT À LA SUSPENSION.** Le contrôle de suspension a lieu **à la
   connexion** (mocks `authenticate()`, écran A1). Une personne suspendue pendant qu'elle navigue
   garde donc sa session jusqu'à son échéance — douze heures, ou trente jours si elle avait coché
   « rester connecté ». Ni le modèle ni l'écran ne s'en occupent : la suspension n'est un contrôle
   d'accès que si elle coupe l'existant. **FR-033**.

4. **`identity.people` PORTE `blocked`, LE CONTRAT DU FRONT NON.** `person_status` connaît quatre
   états ; `LoginResult` n'en expose que `suspended`. L'exclusion durable emprunte donc la même issue,
   sans date de fin — ce que le mock fait déjà (`person.status === 'suspended' || 'blocked'`), sans
   que ce soit écrit ailleurs que dans le code. **Consigné en cas limite**, et non traité comme un
   défaut : ajouter une issue au contrat obligerait à reprendre l'écran A1 pour un état qu'aucun
   administrateur ne prononce aujourd'hui.

5. **LE COURRIEL TRANSACTIONNEL N'A PAS DE PROPRIÉTAIRE DANS CE JALON.** Les modèles de message, le
   suivi des envois et les rebonds vivent dans `110_engagement.sql`, dont le crate arrive en **B6**.
   Or B1 ne fonctionne pas sans envoyer : ni vérification d'adresse, ni réinitialisation. Un crate de
   module ne pouvant dépendre d'un autre, **l'envoi est porté par le binaire des travaux différés**,
   à partir des événements émis par `identity` — le seul emplacement qui ne crée pas d'arête interdite.
   Composition riche et suivi restent à B6. **Hypothèse confirmée pour la file, corrigée pour le
   dernier maillon** : le worker met bien le travail en file, mais il n'envoie pas lui-même — voir
   l'écart n° 8.

6. **LE JALON N'AVAIT PAS DE PURGE DES JETONS PÉRIMÉS.** `ix_one_time_tokens_cleanup` existe en base —
   un index partiel sur `expires_at WHERE consumed_at IS NULL` — donc quelqu'un, un jour, a prévu de
   les nettoyer. Aucun prompt ne le demande. **FR-044** en fait un travail différé récurrent : c'est le
   premier usage réel de `platform.jobs` dans ce module, et il éprouve la file sans attendre B6.

7. **DEUX CLICS SIMULTANÉS SUR LE MÊME LIEN.** `one_time_tokens.consumed_at` est une simple colonne :
   rien n'empêche deux requêtes concurrentes de la lire nulle et de la poser toutes les deux. Sur une
   vérification d'adresse c'est sans conséquence ; sur une réinitialisation, cela ouvre deux
   changements de mot de passe. **FR-041** exige une consommation atomique.

8. **L'API N'A PAS LE DROIT D'ÉMETTRE DU COURRIEL — contrainte d'hébergement, énoncée le 20/08 en cours
   de plan.** L'API et le site vivent sur deux serveurs distincts, et **seul celui du site dispose du
   SMTP** ; ses identifiants sont dans l'environnement de Nuxt. Le plan avait d'abord retenu un envoi
   direct depuis le worker (`lettre`) — impossible. La conception est reprise : le worker **compose** le
   message et le **remet au site** par un appel HTTP privé, authentifié par un secret partagé ; le site
   ouvre la connexion SMTP. La reprise d'essai ne bouge pas : un site injoignable est un travail en
   échec, replanifié par `platform.fail_job()`.

   Trois conséquences qui ne se devinent pas :
   - **`.env.example` change de lecteur pour trois clés.** `SMTP_HOST`, `SMTP_PORT` et `SMTP_FROM` y
     sont annoncées pour l'API ; elles sont désormais lues par le site. Sans correction du commentaire,
     la session suivante les câblera au mauvais endroit.
   - **B1 livre un fichier hors de `backend/`** : `frontend/server/api/internal/mail.post.ts`. C'est le
     seul, et il y est parce que la contrainte l'y met.
   - **Le noyau expose un CONTRAT d'envoi, pas un client HTTP.** Le commanditaire a précisé le même
     jour que la fonction du site est temporaire et sera réécrite en Rust le jour de l'autorisation.
     Sans cette séparation, la bascule obligerait à rouvrir chaque module qui envoie un courriel —
     c'est-à-dire, à terme, presque tous.

9. **AUCUN CONSOMMATEUR MÉTIER N'EXISTE DANS CE JALON, et il fallait le dire.** `identity` est le seul
   module : l'outbox n'a personne à qui annoncer quoi que ce soit. La garde d'idempotence de
   `platform.inbox_events` serait donc restée non éprouvée jusqu'à B2. Le plan retient un consommateur
   de **télémétrie**, utile pour de vrai — il rend visible dans Jaeger ce qui traverse l'outbox — et qui
   exerce la garde de bout en bout : arrêter le worker, le relancer sur des événements déjà traités,
   vérifier qu'aucun n'est rejoué.

10. **`identity.anonymize_person()` ÉMET DÉJÀ SON ÉVÉNEMENT.** La fonction appelle `platform.emit_event()`
    avant de rendre la main. Un service qui émettrait « personne anonymisée » après l'avoir appelée en
    écrirait deux, **sans qu'aucune erreur ne le signale**. C'est le piège le plus discret du module ;
    il est inscrit dans `contracts/events.md`.

11. **UNE LECTURE FRANCHIT UNE FRONTIÈRE DE MODULE, et le modèle l'avait prévu.** La liste et la fiche
    du back-office affichent le nom de l'organisation, qui vit dans `org.organizations`. Ce n'est ni une
    dépendance de crate ni un appel de module à module — c'est une jointure en lecture seule —, et
    `identity.people.primary_organization_id` porte déjà le commentaire « Cross-module assumé ». Le
    point est noté **pour B2**, où il se décidera au lieu de se découvrir.

12. **LE GARDE D'AUTORISATION NE POUVAIT PAS VIVRE DANS `identity`, et cela ne s'est vu qu'au découpage.**
    `has_permission()` et `administered_events()` sont des **fonctions SQL** que tous les modules à venir
    doivent appeler — B2 borne ses organisations, B3 ses événements, B4 ses propositions. Or aucun crate
    de module n'a le droit de dépendre du crate `identity` : placer le garde là aurait créé, **dès B2**,
    exactement l'arête que le principe II interdit. Il va donc dans `kernel`, qui connaît le schéma
    `identity` comme il connaît `platform` — aucune dépendance de crate, graphe intact.

    Sans cette correction, B2 aurait eu trois issues, toutes mauvaises : dépendre d'`identity`
    (interdit), recopier l'appel SQL dans chaque module (duplication qui diverge, et les trois cas du
    périmètre à ne pas confondre à six endroits), ou tester un nom de rôle — ce qui a déjà coûté une
    correction au prompt A8. Consignée en **R16** de `research.md`, et signalée en tête de `tasks.md`.

---

## Ce qui a été vérifié

| Contrôle | Résultat |
|---|---|
| **Le filtre de permission de `identity.administered_events`** — question laissée ouverte par B0 (`b0-constitution.md` § écart 6) | **Le filtre est le bon, aucune modification requise.** La fonction ne retient que `programme.proposal.read_all` ; relecture du semis de `030_identity.sql` § 6 : `admin`, `reviewer` et `programmer` la détiennent tous les trois, et `super_admin` détient tout par trigger. **Les quatre rôles attribuables sur une édition la portent donc** — aucun rôle d'administration d'édition ne tombe en `(false, '{}')`. `900_seed.sql` § 5 ne sème qu'un super-administrateur global, ce qui ne contredit rien |
| `030_identity.sql` lu **intégralement** (644 lignes), `010_platform.sql` intégralement (524 lignes) | Aucune modification du modèle n'est nécessaire — le constat de l'écran A1 se reconduit. Chaque nom cité dans `spec.md` vient de ces deux fichiers, pas de mémoire |
| Le contrat du front, relu à la source | `types/auth.ts`, `types/identity.ts`, `types/admin-users.ts`, `mocks/auth.ts`, `stores/auth.ts`, et les chemins déclarés dans `composables/useApi.ts`. **Aucun nom de champ n'est renégocié** ; FR-062 renvoie à ces fichiers plutôt que d'en tenir une copie qui divergerait |
| Les durées de session viennent du front, pas d'une invention | `stores/auth.ts` : `SESSION_MAX_AGE` 12 h, `REMEMBERED_MAX_AGE` 30 jours. Reprises telles quelles en FR-030 |
| Les cinq obligations d'API relevées en écrivant A12 | Toutes portées : portée visée pour l'attribution (FR-053), même droit pour retirer (FR-053), disparition du paramètre `granted` (FR-055), file RGPD en portée globale (FR-059), effacement réservé aux demandes d'effacement (FR-060) |
| État du dépôt avant écriture | `backend/` absent, `specs/` absent, `.specify/feature.json` absent. **Rien n'a été réécrit** — tout est créé |
| Aucun marqueur `[NEEDS CLARIFICATION]` résiduel | La seule question de périmètre — le second facteur — a été posée et tranchée en séance |

| Le contrôle des dix principes de la constitution | Passé **deux fois** — avant et après conception. Aucune violation, `Complexity Tracking` vide. Deux points ont été tranchés dans `research.md` plutôt que laissés à l'implémentation, parce qu'une décision tacite y aurait produit une entorse : la forme du jeton d'accès et le chemin du jeton en clair vers le courriel |
| Les valeurs reprises du dépôt, et non inventées | Préfixe `/api` (imposé par `NUXT_PUBLIC_API_BASE` de `.env.example`) · durées de session 12 h / 30 j (`stores/auth.ts`) · chemins et formes de réponse (`composables/useApi.ts`, `composables/api/admin-users.ts`, `types/admin-users.ts`) · services locaux et ports (`ops/docker-compose.dev.yml`) · cibles de vérification (`Makefile`) |

**Non vérifié, et qui ne pouvait pas l'être à ce stade** : rien de ce que la spécification exige n'était exercé — il n'y avait pas une ligne de Rust dans le dépôt le matin du 20/08. La première mise à l'épreuve a été `/speckit-implement`, plus bas.

---

# Implémentation — phases 1 et 2 (`/speckit-implement`, 20/08)

**T001 à T035 sont faites.** Le socle technique est debout et éprouvé ; aucune histoire utilisateur
n'est commencée — la phase 3 (US1, la connexion) ouvre le prochain incrément.

## Ce qui a été livré

`backend/` — workspace Cargo symétrique de `frontend/`, cinq crates, **2 837 lignes de Rust**, aucun
fichier au-dessus de **332 lignes**.

| Emplacement | Contenu |
|---|---|
| `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `.gitignore` | Workspace, versions communes, chaîne d'outils épinglée |
| `crates/kernel/src/config.rs` | Configuration typée, **validée au démarrage** — une durée nulle, une clé de signature vide ou un relais sans adresse refusent le lancement. Les durées de jeton sont indexées **par finalité** |
| `crates/kernel/src/context.rs` | Identifiant de requête, acteur, locale. Une **portée de tâche** rend l'identifiant visible depuis la construction d'une réponse d'erreur |
| `crates/kernel/src/error.rs` | Les **20 codes d'API** de `contracts/errors.md`, chacun avec son statut et son message français. Le vingt-et-unième, `MAIL_RELAY_UNREACHABLE`, en est délibérément absent : lui donner une variante lui donnerait un statut HTTP, donc un chemin vers une réponse |
| `crates/kernel/src/pg_error.rs` | `(SQLSTATE, contrainte) → erreur`, plus le passage **tel quel** du message français d'un trigger, et la détection d'une collision d'aléa sur une empreinte de jeton |
| `crates/kernel/src/db.rs` | Pool, et **l'unique** ouverture de transaction en écriture : elle pose `app.actor_id` et `app.request_id` avant de rendre la main |
| `crates/kernel/src/i18n.rs` | Négociation contre `reference.locales`, avec les facteurs de qualité, repli sur le français |
| `crates/kernel/src/crypto.rs` | Argon2id 19 MiB / 2 itérations, **empreinte factice calculée au démarrage**, jetons aléatoires, empreintes SHA-256, comparaison à temps constant |
| `crates/kernel/src/telemetry.rs` | `tracing` + export OTLP vers Jaeger, vidange des traces à l'arrêt |
| `crates/kernel/src/events.rs` | `platform.emit_event()` — jamais d'INSERT direct —, registre de consommateurs, garde `platform.inbox_events` |
| `crates/kernel/src/jobs.rs` | Mise en file, `claim_jobs()`, `fail_job()`, et la réussite **qui vide la charge utile** |
| `crates/kernel/src/mail.rs` | Contrat d'envoi, remise HTTP au serveur du site, variante SMTP **présente et non branchée** |
| `crates/kernel/src/auth.rs` | `has_permission()`, `administered_events()` avec ses trois cas distincts, et les extracteurs de route |
| `crates/kernel/src/testing.rs` | Base modèle chargée une fois depuis `docs/database/`, recopie par test, suppression en sortant |
| `crates/contracts/src/identity.rs` | Les neuf charges utiles d'événements, et leurs constantes de type |
| `crates/api/` | Démarrage, état partagé, montage conditionnel d'après `platform.modules`, intergiciels de contexte et d'origine |
| `crates/worker/` | Relais d'outbox (notification **et** balayage), boucle de travaux, consommateur de télémétrie |
| `.sqlx/` | 17 requêtes préparées, **versionnées** |
| `.env.example` | Les 21 clés d'authentification et de relais, et le commentaire SMTP corrigé — **ces trois clés sont lues par le site, pas par l'API** |

## Les écarts relevés en implémentant

1. **La chaîne d'outils du poste ne suffisait pas.** `rustc` rendait 1.93.0, SQLx 0.9 exige 1.94 : la
   compilation échouait avant la première ligne écrite. `rust-toolchain.toml` épingle **1.97.1**, déjà
   présente sur la machine.

2. **SQLx passe de 0.8 à 0.9**, contre ce qu'annonçait `plan.md`. La CLI installée est en 0.9 et
   `cargo sqlx prepare` refuse une version de crate différente ; réinstaller la CLI en 0.8 aurait touché
   les autres projets du poste. Rien de structurel ne change — les macros vérifiées à la compilation,
   seule chose qu'exige le principe VI, sont identiques. **Une conséquence à connaître** : la 0.9 refuse
   toute requête composée dynamiquement ; le harnais de test l'assume par `AssertSqlSafe`, et c'est le
   seul endroit du dépôt qui le fait.

3. **La règle d'origine ne disait rien du cas de l'en-tête absent** (`research.md` § R2). Le contrôle
   porte sur l'**origine annoncée** : l'en-tête `Origin`, ou à défaut le schéma et l'autorité du
   `Referer`. Une écriture sans aucun des deux passe — refuser aurait cassé toutes les vérifications
   manuelles de `quickstart.md`, qui appellent l'API par `curl`, **sans rien protéger** : les
   navigateurs posent un `Origin` sur toute écriture, donc l'absence des deux désigne un client qui
   n'est pas un navigateur. Une origine annoncée et inconnue est refusée, **`null` compris** — une
   valeur littérale `null` vient d'une iframe cloisonnée ou d'une redirection inter-schémas, ce n'est
   pas une absence. *La première rédaction de cet écart ne parlait que de l'en-tête absent, et laissait
   croire qu'une écriture sans `Origin` passe toujours : elle est refusée si un `Referer` d'un autre
   site l'accompagne.*

4. **La clé qui bascule l'envoi de courriel n'était pas nommée.** Elle s'appelle `MAIL_TRANSPORT`,
   vaut `relay` aujourd'hui et `smtp` le jour de l'autorisation. La variante SMTP existe déjà et refuse
   d'envoyer avec un message explicite.

5. **Le contrôle de frontières de `quickstart.md` compte une ligne de trop.**
   `cargo tree -p identity | grep -c "modules/"` rend **1**, pas 0 : la ligne racine porte elle-même le
   chemin `crates/modules/identity`. Le contrôle juste est
   `cargo tree -p identity | tail -n +2 | grep -c "crates/modules/"`.

6. **`Requires<P>` n'exige qu'une permission de portée globale — et c'est un arbitrage, pas une
   impossibilité.** *La première rédaction de cet écart le justifiait par « Actix ne passe aucun
   argument à un extracteur », ce qui est faux* : un extracteur reçoit la requête entière, `match_info()`
   compris, renseigné avant l'extraction. Le vrai motif est ailleurs : un extracteur de portée ciblée
   devrait déclarer le nom du paramètre **et** son type de portée, puis rendre lui-même le refus quand
   le paramètre manque ou n'est pas un UUID — c'est-à-dire décider de la forme d'une erreur loin du
   gestionnaire qui la rend. Une portée ciblée se vérifie donc dans le gestionnaire, par
   `require_permission(…, Scope::Event(id))`. Le point de décision reste unique.

7. **Le harnais de test s'interbloquait en fermant son pool.** Mesuré, pas supposé : la suppression de
   la base doit être terminée quand le test rend la main, donc elle bloque son fil ; or fermer un pool
   SQLx s'appuie sur le runtime que ce fil vient de bloquer. `DROP DATABASE … WITH (FORCE)` termine les
   connexions côté serveur et règle le même problème sans rien attendre.

## Ce qui a été vérifié

| Contrôle | Résultat |
|---|---|
| `make check-back` | **Vert.** La cible exécute désormais `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` et `cargo test --workspace --all-features` — sans les deux options, `clippy` ne voyait ni les tests d'intégration ni `kernel/src/testing.rs`, le seul fichier du dépôt qui compose du SQL dynamiquement |
| Aucune arête entre deux crates de module | `cargo tree -p identity \| tail -n +2 \| grep -c "crates/modules/"` = **0**. `identity` ne dépend que de `kernel` et de `contracts` |
| Aucun fichier au-dessus de 1000 lignes | Le plus long est `kernel/src/config.rs`, **332 lignes** |
| Le harnais de test lui-même | `harness.rs` : la base jetable porte les **16 schémas**, `identity.has_permission()` y répond, et la base **a disparu** quand le test rend la main |
| Le contexte d'écriture atteint réellement l'audit | `contexte_ecriture.rs` : une insertion dans `identity.people` par la porte du noyau laisse dans `platform.audit_log` **l'acteur et l'identifiant de requête attendus** ; et hors transaction, `platform.current_request_id()` ne porte plus rien — `set_config(…, true)` est bien local |
| L'API démarre et répond | `GET /api/ready` → 200, sur `127.0.0.1:8080`. **Pas `/health`** : le contrat réserve ce nom à la route protégée qui rendra les chiffres d'exploitation |
| `X-Request-Id` fait le tour complet | Engendré s'il manque, **repris tel quel** s'il est fourni, rendu dans l'en-tête de réponse **et** dans le corps d'erreur |
| L'intergiciel d'origine | `POST` avec `Origin: https://attaquant.example` → **403 `IDENTITY_ORIGIN_REJECTED`**, corps portant son `request_id` ; avec l'origine du site, la requête passe au routeur |
| Le SQLSTATE du domaine d'adresse | Mesuré sur la base : `SELECT 'pasunemail'::platform.email` rend **`23514 / email_check`**, jamais `22P02` — le contrat disait le contraire, et le code le suivait |
| Un module non monté rend 404 | `/api/negotiation/...` → 404, pas 403 |
| Le relais d'outbox, de bout en bout | Un événement émis par `platform.emit_event()` est relayé **en moins d'une seconde** (`LISTEN/NOTIFY`), `published_at` posée, `correlation_id` conservé depuis le contexte de la transaction |
| La garde d'idempotence | `published_at` remise à `NULL`, l'événement est **repris sans être retraité** : `platform.inbox_events` garde une seule ligne, le consommateur n'écrit pas une seconde trace |
| Le worker s'arrête vraiment sur `Ctrl-C` | **Mesuré : arrêt en 1 s.** Il ne s'arrêtait pas — `db.close()` attendait une connexion que le relais d'outbox garde pour la vie du processus |
| Un travail réservé par un worker tué revient à la file | Ligne posée à `running` avec un verrou de 45 min : reprise au tour suivant, réservée, puis replanifiée par `platform.fail_job()` faute de gestionnaire |
| Jaeger reçoit les traces de l'API | Service `epavillon-api` visible, span `http` portant `methode`, `chemin` et `request_id`. **Il ne recevait rien** : aucun span n'était ouvert, et l'exportateur mourait au premier envoi faute de réacteur sur son fil |
| Un chemin inconnu rend le corps du catalogue | `GET /api/nimporte-quoi` → 404 `{"code":"NOT_FOUND",…,"request_id":…}`, et non la réponse vide d'Actix |
| `Origin: null` sur une écriture | **403 `IDENTITY_ORIGIN_REJECTED`** — c'est une origine annoncée, pas une absence |
| `Referer` d'un autre site sans `Origin` | **403** — le repli sur le référent applique la même règle |
| `.sqlx/` n'est pas ignoré par git | `git check-ignore backend/.sqlx/` ne rend rien — le dossier est versionnable, comme R4 l'exige |

**Non vérifié, et qui ne peut pas l'être avant la phase 3** : aucune route métier n'existe encore, donc
ni connexion, ni inscription, ni périmètre d'administration exercé sur une vraie liste. Le premier
courriel ne partira qu'avec la phase 5 — la route de relais du site (`frontend/server/api/internal/mail.post.ts`)
reste à écrire.


---

# Audit du code livré (20/08, après les phases 1 et 2)

Un audit croisé — sept lecteurs sur le code et les documents, chacun suivi d'un sceptique chargé de
réfuter ses constats — a rendu **53 constats confirmés sur 80** : 3 hauts, 19 moyens, 31 bas.
**Tous sont traités.** Ce qui suit est ce qui n'apparaît pas dans les sept écarts du jour même.

## Ce qui était CASSÉ et ne l'est plus

| Défaut | Ce qu'il produisait | Correction |
|---|---|---|
| Le catalogue rangeait le refus du domaine `platform.email` sous `22P02` | Une adresse mal écrite sortait en **500**, pas en 422 — sur un chemin **nominal**, puisque le principe VIII fait du domaine le validateur prévu | `23514 / email_check` dans `pg_error.rs` et dans `errors.md`, mesuré sur la base |
| Aucun span n'était ouvert nulle part | **Jaeger ne recevait rien**, malgré un exportateur configuré : la couche OpenTelemetry ignore tout événement hors span | Span par requête dans l'intergiciel, spans dans les deux boucles du worker |
| L'exportateur OTLP utilisait le client HTTP **asynchrone** | Son fil de traitement n'a pas de réacteur Tokio : il mourait au premier export, et plus rien ne partait ensuite | Client **bloquant** (`reqwest-blocking-client`) |
| Le worker n'obéissait pas à `Ctrl-C` | `db.close()` attendait la connexion d'écoute du relais, gardée pour la vie du processus | Les deux tâches sont abandonnées avant la fermeture du pool. **Arrêt mesuré : 1 s** |
| Une erreur de base au moment de clore un travail tuait la boucle | Et avec elle le relais d'outbox, qui partage le processus — alors que la même erreur à la réservation était simplement réessayée | Journalisée et réessayée, comme sa voisine |
| Un travail réservé par un worker tué restait `running` pour toujours | Jusqu'à dix courriels de vérification perdus par `Ctrl-C` en cours de lot | Reprise au bout d'un bail de 30 min, écrite dans le worker — le modèle sait voir ce cas, pas le réparer |
| `AUTH_COOKIE_SECURE` absente valait `false` | Une variable oubliée en production aurait posé le cookie de session **sans `Secure`**, sans que rien ne le dise | Le défaut vaut `true` : il ferme au lieu d'ouvrir |
| Un chemin inconnu rendait la réponse vide d'Actix | 404 sans code stable, sans message français, sans `request_id` | Corps du catalogue, comme toute autre erreur |
| `Origin: null` passait le contrôle d'écriture | Une iframe cloisonnée écrivait avec les cookies de la personne | Refusé : c'est une origine annoncée, pas une absence |
| Le modèle de test ne se rafraîchissait jamais | Après modification d'un fichier de `docs/database/`, `cargo test` recopiait l'ancien schéma **sans le dire** | Le nom du modèle porte l'empreinte du SQL |
| Un chargement de modèle interrompu laissait une base amputée | Que plus rien ne reconstruisait, même après correction du SQL | Le modèle est supprimé si son chargement échoue |
| `make check-back` n'analysait ni les tests ni `testing.rs` | Le seul fichier composant du SQL dynamiquement échappait au portail — et le compte rendu de la veille citait la commande que j'avais lancée à la main, pas celle de la cible | `--all-targets --all-features` dans le `Makefile` |

## Ce qui était FAUX dans le code ou les documents

- **Le noyau prétendait une garantie qu'il n'avait pas.** « Aucune écriture ne passe par là » au-dessus
  de `Db::pool()`, alors que le noyau y écrivait quatre fois. La porte unique est une **discipline** —
  le pool reste accessible, `LISTEN/NOTIFY` et le harnais en exigent un vrai. Les écritures du worker
  passent maintenant par `write()` quand même, et la phrase dit ce qu'elle garantit.
- **`administered_events` repliait en silence le NULL que le SQL désigne comme son défaut historique.**
  `unwrap_or(false)` redoublait un invariant que la base porte déjà par `COALESCE` — exactement le
  contrôle d'accès qui échoue sans erreur que le commentaire du modèle décrit. Les colonnes sont
  déclarées `!` : un NULL fait rougir, il ne se replie pas.
- **`MAIL_RELAY_UNREACHABLE` avait un statut HTTP**, alors que le catalogue écrit qu'il n'est jamais
  rendu à un client. Il quitte l'énuméré des codes d'API.
- **`negotiate` honorait `q=0` comme une préférence**, là où la RFC 7231 en fait un refus ; un facteur
  illisible était promu à la priorité maximale.
- **`RUST_LOG=info,epavillon=debug` visait une caisse qui n'existe pas** : le filtre ne faisait rien,
  silencieusement.
- **La justification de l'écart n° 6 était fausse** — voir l'écart réécrit plus haut.
- **Dix documents portaient des décomptes périmés** : 18 fichiers SQL au lieu de 19, 15 schémas au lieu
  de 16, 15 928 lignes au lieu de 16 006, un `backend/migrations/` que le projet a écarté, un `ops/`
  « à créer » versionné depuis le 16/08, et `NUXT_PUBLIC_API_BASE` renseignée dans un extrait alors que
  `.env.example` exige qu'elle reste vide. Là où le chiffre ne servait qu'à dire « c'est trop pour tout
  charger », il est remplacé par une formule qui ne se périme pas ; **un seul endroit garde un décompte
  exact et daté**, `docs/README.md`, et les autres y renvoient.

## Deux dettes datées, tranchées mais non refermées

1. **Un travail passé en file morte garde sa charge utile** — donc son jeton en clair. `succeed()` la
   vide, `platform.fail_job()` jamais. Rien ne fuit aujourd'hui : `identity::job_handlers()` est vide.
   **À trancher avant le premier envoi de courriel** : vider au passage en `dead`, ou restreindre aux
   tâches qui portent un secret — la charge utile d'un travail mort est, pour `media` et `analytics`,
   la seule matière de diagnostic. Écrit dans `contracts/events.md` § 3.
2. **L'API ne pose aucun en-tête CORS.** Le site appellera avec `credentials: 'include'` depuis un autre
   port : sans `Access-Control-Allow-Origin` ni `-Credentials`, aucun appel de navigateur n'aboutira.
   Rien n'est cassé tant que `NUXT_PUBLIC_API_BASE` reste vide. **À livrer avec B7.** Écrit dans
   `contracts/routes.md`.

---

# Phases 3 et 4 — la connexion, et la session (T036–T058)

**23 tâches, 58 sur 126.** `make check-back` au vert : mise en forme, `clippy -D warnings` sans un
avertissement, **29 tests** dont 26 nouveaux. 4 952 lignes de Rust hors tests, aucun fichier au-dessus
de 351.

## Ce qui a été livré

| Emplacement | Ce qu'il porte |
|---|---|
| `domain/ids.rs` | identifiants typés par agrégat — un `Uuid` nu se passe partout sans que rien ne proteste |
| `domain/login.rs` | les **six issues** de connexion, `person_status`, `auth_provider` |
| `domain/password.rs` | les trois exigences opposables, **les mêmes que le site applique déjà** |
| `domain/person.rs` | la personne telle que l'API la rend, champ pour champ le type du site |
| `domain/access_token.rs` | jeton signé Ed25519, **sans aucune permission dedans** |
| `repo/people.rs` · `accounts.rs` · `sessions.rs` | les lectures et écritures des trois agrégats |
| `service/auth.rs` | l'ordre des contrôles, et la discrétion |
| `service/session.rs` | ouverture, rotation, détection de rejeu, révocation |
| `routes/auth.rs` · `cookies.rs` | `POST /auth/login`, `/logout`, `/refresh`, `GET /auth/me`, et les deux cookies |
| `api/middleware/auth.rs` | l'intergiciel qui résout la session et remplit l'acteur |

### Les six issues, dans l'ordre imposé

Mot de passe **d'abord** ; verrou, suspension, adresse non vérifiée et second facteur **ensuite**, et
dans cet ordre. Chacun des quatre derniers suppose l'identité prouvée : les rendre plus tôt
renseignerait qui ne connaît pas le mot de passe. **Les six sortent en 200** — un refus prévu par le
contrat du site n'est pas une erreur HTTP.

### La session

Le jeton d'accès dure quinze minutes et n'est **qu'un renvoi signé vers une session** : les permissions
se relisent en base à chaque requête, ce qui rend la révocation immédiate. Le jeton de rafraîchissement
ne vit en base qu'en empreinte, et le cookie qui le porte n'est envoyé qu'à `/api/auth`. Le
renouvellement **révoque la session courante et en ouvre une nouvelle** ; un jeton dont la session porte
déjà le motif `rotated` n'a aucune explication innocente, et fait tomber **toutes** les sessions de la
personne.

## Le test qui a échoué au premier essai, et qui avait raison

**22,8 % d'écart de temps** entre une adresse inconnue (12,9 ms) et un mot de passe faux (16,8 ms), là
où SC-001 en tolère dix.

Le hachage n'était pas en cause : mesuré des deux côtés, 11,11 ms contre 11,13 ms — l'empreinte factice
du noyau faisait exactement son travail. La différence venait d'ailleurs : **une adresse connue écrit
son compteur d'échecs, une adresse inconnue n'écrit rien.** Une milliseconde et demie qu'aucun journal
ne montre, et qui suffit à redire quels comptes existent.

Correction : le hachage part sur un fil dédié, et l'écriture du compteur est lancée **avant** de
l'attendre. La base répond en une milliseconde là où le calcul en prend une dizaine — l'écriture
disparaît entièrement derrière lui. **Écart mesuré après correction : 1,5 %** (12,27 ms contre
12,46 ms).

L'incrément devient **optimiste**, puisque le verdict n'est pas encore connu. Il se défait tout seul
quand le mot de passe est juste, ce que FR-015 demandait déjà. Le verrou, lui, n'est jamais posé de
façon optimiste : il attend, et prend alors la transaction qu'il mérite, son événement dedans.

C'est exactement ce que `research.md` § R5 annonçait — « le temps parle aussi » — mais l'oracle n'était
pas là où on l'attendait. Sans le test, personne ne l'aurait vu.

## Écarts relevés en implémentant

| # | Écart | Décision |
|---|---|---|
| 21 | **La clé de signature ne peut pas être un bloc PEM** : `AUTH_SIGNING_KEY` vit sur une ligne d'un `.env` | 32 octets, en hexadécimal ou en base64. Le jeton reste **exactement un JWT `EdDSA`**, composé sur place et signé par `ed25519-dalek` : c'est le format de la **clé** qui est choisi, pas celui du jeton. `.env.example` porte la commande qui l'engendre |
| 22 | **Soixante-quatre signes hexadécimaux forment aussi du base64 valide**, de quarante-huit octets — le premier décodage qui aboutit n'est pas le bon. Trouvé au démarrage de l'API, qui refusait la clé du `.env` local | C'est la **longueur obtenue** qui départage les deux écritures, jamais l'ordre des essais. Deux tests unitaires le tiennent |
| 23 | **Le renouvellement ne doit pas repousser l'échéance** : un jeton d'accès dure un quart d'heure, donc le renouvellement tombe toutes les quinze minutes — une durée relancée à chaque tour n'expire jamais | La session neuve hérite de l'`expires_at` de celle qu'elle remplace. « Douze heures » veut dire douze heures |
| 24 | **Argon2id non optimisé met près d'une seconde** par vérification : le test de discrétion, deux cents hachages, devenait interminable | `opt-level = 3` sur les seules **dépendances**, en développement. Le code du projet garde ses assertions et sa compilation rapide |
| 25 | **`platform.email` est un domaine sur `citext`, et PostgreSQL rabat `citext` sur `text`** par sa conversion implicite : `primary_email = $1` aurait été **sensible à la casse**, silencieusement | `$1::text::citext` dans la lecture de connexion, et un test qui se connecte avec `Awa.DIALLO@Example.ORG` |
| 26 | Une adresse annoncée dans `X-Forwarded-For` part vers une colonne `inet` | Elle est **validée avant** : une valeur forgée y produirait une erreur de conversion, donc un 422 sur une connexion par ailleurs correcte |

## Vérifications

### Les tests

| Ce qui est vérifié | Test |
|---|---|
| Les six issues, une par une, plus l'ordre des contrôles | `connexion_issues` — 9 cas |
| Adresse inconnue et mot de passe faux : même réponse, **même temps** | `discretion_temps_de_reponse` — 100 tentatives de chaque sorte, médianes, écart sous 10 % |
| Adresse non vérifiée → refus, **sans session** (FR-024) | `connexion_refusee_si_adresse_non_verifiee` |
| Un verrou échu libère le compte et remet le compteur à zéro (FR-015) | `un_verrou_echu_libere_le_compte` |
| Renouvellement, rotation, déconnexion, déconnexion répétée | `session_rotation` — 5 cas |
| Jeton rejoué → **toutes** les sessions révoquées (FR-031) | `rejeu_du_jeton_revoque_tout` |
| Suspension, changement de mot de passe et **anonymisation** coupent les sessions (FR-033) | `suspension_coupe_les_sessions` — 3 cas |
| Signature, altération, péremption, clé étrangère, format de clé | 6 tests unitaires du codec |

### Éprouvé à la main

- Deux `{"status":"invalid_credentials"}` **au caractère près**, adresse inconnue et mot de passe faux.
- Une connexion réussie pose ses deux cookies aux bonnes portées : `epavillon_at` en `SameSite=Lax` sur
  `/` pour 900 s, `epavillon_rt` en `SameSite=Strict` sur `/api/auth` pour la durée de la session.
- `GET /auth/me` rend `null` en **200** sans session — pas 401 : le store du site appelle cette route à
  chaque navigation, y compris déconnecté.
- `POST /auth/refresh` rend `renewed` et repose les deux cookies ; après déconnexion, `expired`.
- `POST /auth/logout` réussit **deux fois de suite**, et sans session du tout.
- Une écriture d'origine inconnue rend toujours `IDENTITY_ORIGIN_REJECTED`, un chemin inconnu le corps
  du catalogue.

### Les portes

```
cargo tree -p identity | tail -n +2 | grep -c "crates/modules/"   →  0
find backend/crates -name '*.rs' | xargs wc -l | sort -rn | head  →  351 au plus
make check-back                                                   →  vert, 29 tests
```

## Ce qui reste

Les phases 5 à 11 — T059 à T126. La suivante, **US3 (T059–T067)**, ferme le jalon **T001–T067**, celui
qui débloque B2 à B6 : un appelant identifié, une permission testable, un périmètre borné.

---

# Audit des écarts (20/08, après les phases 3 et 4)

Second audit croisé — huit lecteurs, chacun suivi d'un sceptique chargé de faire tomber ses constats,
puis un dernier chargé de dire ce que personne n'avait regardé. **45 constats confirmés sur 59, plus 9
trouvés par le critique de complétude. Tous traités.** Les huit angles : contrat de routes ↔ code ·
formes de réponse ↔ types du site · exigences fonctionnelles ↔ code · les seize décisions techniques ↔
code · décomptes et noms périmés ↔ réalité mesurée · défauts du code neuf · les dix principes ↔ code
neuf · configuration clé par clé.

## Ce qui était CASSÉ et ne l'est plus

| Défaut | Ce qu'il produisait | Correction |
|---|---|---|
| **Un verrou échu ne rendait qu'UN essai au compte, pas tous** (gravité haute, trois lecteurs sur huit l'ont trouvé) | Le compteur d'échecs continuait de grimper pendant le verrou et n'était purgé que sur le chemin du **mot de passe correct**. À l'échéance, il valait donc seuil+N : la première faute de frappe suivante reposait un verrou d'un quart d'heure, **indéfiniment**. Quelqu'un qui s'était verrouillé en cherchant son mot de passe n'avait plus droit qu'à un essai par quart d'heure, à vie | La purge appartient au **chemin commun** : elle se fait dans l'incrément lui-même, avant que le seuil ne soit testé, sur les deux branches. Un test l'exerce là où il fallait — verrou antidaté **puis mot de passe faux** |
| Deux renouvellements simultanés portant le même jeton ouvraient **deux sessions** | Le booléen rendu par la révocation était jeté : le second appel révoquait zéro ligne et ouvrait quand même une session. Un double-clic laissait une session orpheline vivante, née d'un jeton déjà consommé ; un voleur qui rejouait en parallèle passait sans éveiller la détection | **La révocation EST le verrou** : si elle ne touche aucune ligne, c'est un rejeu, et le chemin de coupure générale s'applique. Un test lance les deux renouvellements ensemble |
| Deux échecs simultanés au franchissement du seuil posaient **deux verrous et deux événements** | La décision de verrouiller reposait sur une lecture antérieure à l'incrément | C'est la base qui décide : l'écriture ne touche que les lignes non verrouillées, et l'événement ne part que si une ligne a changé |
| Un corps de requête mal formé sortait en **400 avec le texte anglais de serde** | Sans code stable, sans message français, sans `request_id` dans le corps — et le texte nommait les champs de la charge utile. `errors.md` annonçait pourtant ce point livré « avec la phase 3 », qui l'était | 422 `VALIDATION_FAILED` portant le **nom du champ**, seule chose empruntée à serde et filtrée avant de sortir ; 413 au-delà d'un mégaoctet. Un test HTTP vérifie que « missing field » ne franchit jamais la réponse |
| Le rejeu détecté **laissait les deux cookies** dans le navigateur | Chaque appel suivant redéclenchait la détection et écrivait une alerte sans objet | Le 401 efface les deux cookies avec la même portée que la pose |
| Un `X-Forwarded-For` **présent mais illisible effaçait** l'adresse de la session | Le repli sur l'adresse du pair ne jouait que si l'en-tête était absent | Il joue aussi quand l'en-tête est invalide : c'est l'adresse du pair qui reste |
| Une **panne de base** pendant la résolution de session se lisait « déconnecté » | Toute erreur SQLx était avalée et rendue comme une absence de session : `GET /auth/me` annonçait « déconnecté » à quelqu'un qui ne l'était pas, et une route protégée le renvoyait se reconnecter en vain | La résolution distingue « aucune session valide » d'un échec technique. Le refus ne tombe que sur les requêtes **qui présentaient un cookie** — un visiteur anonyme n'a rien à quoi la panne s'applique |
| **Aucun test ne traversait HTTP** | Statuts, corps d'union, préfixe `/api`, attributs des deux cookies, corps d'erreur : rien de ce qui ne vit que dans la couche route n'était couvert. Une inversion de `SameSite` ou un 401 rendu à la place d'un 200 serait resté vert jusqu'au raccordement du site — et `plan.md` déclarait l'obligation n° 1 du principe X tenue | `crates/api` devient bibliothèque **et** binaire : `api::build_app` monte l'application une fois, le binaire la lance, et `tests/routes_auth.rs` la dresse telle quelle — **avec ses trois intergiciels** — sur une base jetable. Neuf cas |
| Un `UPDATE` de `last_seen_at` portait sur la ligne **qu'on venait d'insérer** | Écriture morte : `now()` vaut l'instant d'ouverture de la transaction, l'`INSERT` avait déjà posé la même valeur | L'appel disparaît ; `data-model.md` dit ce que la colonne porte vraiment — la rotation crée une ligne neuve, donc la date suit l'activité sans écriture supplémentaire |
| `people::status_of` était du **code mort au commentaire faux** | Elle se disait employée par l'intergiciel de session, qui ne l'appelle pas : le contrôle vit dans le `AND p.status = 'active'` de la lecture jointe. Un prochain lecteur aurait cru à deux allers-retours, ou ajouté un second contrôle redondant | Retirée |

## Ce qui était FAUX dans les documents

| Document | Ce qu'il affirmait | Ce qui est vrai |
|---|---|---|
| `plan.md` § Primary Dependencies | `jsonwebtoken` (Ed25519) | `ed25519-dalek` : le JWT `EdDSA` est composé dans le module pour que la clé tienne sur une ligne de `.env` (écart n° 21) |
| `plan.md` § principe VII et `contracts/events.md` | « aucune écriture du jalon ne passe à côté » de la porte unique | Trois écritures du compteur d'échecs empruntent le pool, et c'est **assumé** — voir l'écart n° 27 |
| `research.md` § R3 | motifs `rotated, logout, reuse_detected, password_changed, suspended, anonymization` | `suspended` n'existe pas — c'est `status_changed`, qui couvre suspension **et** exclusion — et `logout_all` manquait. La liste qui fait foi est celle de `data-model.md` |
| `research.md` § R6 | les empreintes « se ré-encodent à la prochaine connexion réussie » | Rien à ré-encoder : aucune route de B1 ne crée ni ne change de mot de passe, et les paramètres n'ont jamais bougé. Le point se posera au premier durcissement |
| `contracts/errors.md` | `IDENTITY_SESSION_EXPIRED` et `_REVOKED` rendus sur le renouvellement | Le renouvellement rend 200 avec son union : la règle du contrat l'emporte. Les deux codes attendent une route protégée, et le catalogue le dit maintenant |
| `data-model.md` § correspondance Rust ↔ base | `inet` → `std::net::IpAddr`, « sans exception » | `identity.sessions.ip_address` traverse en chaîne validée à la frontière : la faire passer en `IpAddr` déplacerait le refus d'une adresse forgée vers la base, donc un 500 là où il faut un 422 |
| `quickstart.md` | un test `issues_de_connexion`, un `/api/docs` « à partir de la phase 3 », et trois noms de tests futurs que `tasks.md` nomme autrement | Le fichier s'appelle `connexion_issues`, `/api/docs` est livré par T117 en **phase 11**, et la colonne nomme désormais des **fichiers**, une convention à la fois |
| `quickstart.md` § vérifications à la main | des `curl` avec `a.sowfall@roac-afrique.org` | `900_seed.sql` ne sème **aucun mot de passe**, et ces adresses n'existent que dans les données simulées du site. Le document dit maintenant que ces vérifications sont tenues par les tests jusqu'à la phase 6 |
| `docs/README.md` et `README.md` | 15 · 142 · 14 · 7 · 153 · 167 | **16 · 143 · 17 · 8 · 160 · 173**, recomptés le 20/08. La requête de recomptage oubliait le schéma `content` : elle reproduisait l'erreur à chaque usage |
| `docs/ENVIRONNEMENT_LOCAL.md` | un extrait de `.env` sans les dix-sept clés du 20/08, une fin de fichier qui n'est plus la bonne, « le front n'a besoin que de `NUXT_PUBLIC_API_BASE` » | Les blocs manquants sont reproduits ou renvoyés, les **deux clés livrées vides qui font échouer le démarrage** sont nommées, et le site lit aussi `NUXT_PUBLIC_SITE_URL` |
| `.specify/memory/constitution.md` | `check-back` = `cargo fmt --check`, `clippy -- -D warnings`, `cargo test` | La cible porte `--all-targets --all-features`, sans quoi ni les tests ni le seul fichier qui compose du SQL dynamiquement ne sont analysés. **Amendée en 1.0.1** — correctif, aucune règle ne change |
| `docs/PROMPTS_DEVELOPPEMENT.md` | la commande qui vérifie la limite de 1000 lignes | Elle ne mesurait plus que `node_modules` et `target` : bornée à `frontend/app` et `backend/crates` |
| `ops/docker-compose.dev.yml` | « valkey — cache et sessions » | Aucune ligne de `backend/` ne parle à Valkey, et les sessions vivent en base |
| `frontend/app/types/auth.ts` | « Sept issues sont prévues », le second facteur branché « prompt B1 » | Six, et le second facteur est arbitré **hors** de B1 |

## Trois écarts de plus — deux refermés, un laissé

27. **Les écritures du compteur d'échecs empruntaient le pool, hors de la porte unique.** ✅ **Refermé.**
    Elles ont d'abord été assumées : `identity.accounts` ne porte pas de trigger d'audit — seulement
    `tg_accounts_updated_at` —, et une transaction du noyau coûte quatre allers-retours au lieu d'un,
    écart qui se mesure et qui distinguerait une adresse connue d'une adresse inconnue (SC-001).
    **C'était renoncer à la garantie pour une raison qui n'en était pas une** : l'écriture est
    désormais lancée **avant** l'attente du hachage, et sa transaction se replie entièrement derrière
    lui. Mesuré après coup : **écart de temps 1,4 %**, contre 10 % tolérés. Toute écriture du jalon
    passe de nouveau par `Db::write()`, et l'acteur y est même posé dès que le mot de passe est prouvé
    — ce que la version « rapide » ne faisait pas. Les deux documents qui l'affirmaient disent de
    nouveau vrai.

28. **L'adresse d'origine d'une session était déclarative : `X-Forwarded-For` était cru sur parole.**
    ✅ **Refermé.** `TRUSTED_PROXIES` déclare les mandataires dont on accepte l'en-tête — adresses ou
    préfixes, IPv4 et IPv6 —, **et le défaut ferme** : sans eux, c'est l'adresse du pair qui fait foi.
    Quand un mandataire est déclaré, la chaîne annoncée se remonte **de droite à gauche** — chaque saut
    ajoute son prédécesseur à droite, donc l'adresse la plus à droite est la seule que notre propre
    frontal ait pu vérifier —, en sautant les mandataires connus jusqu'au premier maillon qui n'en est
    pas. Un segment illisible arrête la remontée : au-delà, plus rien n'est vérifiable. Le tout vit
    dans `kernel::net`, pour que les cinq modules suivants n'aient pas à le refaire, avec neuf tests
    unitaires et un test HTTP. Au passage, la colonne `inet` cesse de traverser en texte : la
    caractéristique `ipnetwork` de SQLx rend la vérification à la compilation, et une adresse mal
    formée est refusée par le parseur de Rust plutôt que par PostgreSQL.

29. **Un fichier du front dépasse la limite de 1000 lignes** : `frontend/app/composables/useApi.ts`,
    1 008 lignes. C'est le seul du dépôt, il est antérieur au socle Rust, et son découpage est un
    chantier de front qui n'appartient pas à ce jalon. Signalé, non corrigé. La commande qui le
    constate est de nouveau utilisable (voir plus haut).

## Ce qui a été vérifié après correction

| Contrôle | Résultat |
|---|---|
| `make check-back` | **Vert.** 52 tests, dont 35 sur base réelle et jetable — **10 traversent HTTP** — et 17 unitaires |
| Aucune écriture hors de la porte du noyau | `grep` sur `service/` et `repo/` : les seuls appels à `Db::pool()` restants sont des **lectures** |
| L'en-tête d'adresse d'un pair inconnu | Ignoré : c'est l'adresse du pair qui est enregistrée. Neuf tests unitaires couvrent la remontée de chaîne, les préfixes, les deux familles d'adresses et les adresses IPv4 projetées |
| La discrétion, avec la transaction rétablie | **Écart de 1,4 %** — l'écriture se replie derrière le hachage |
| Le chemin nominal de chaque route livrée | `routes_auth` : `/api/ready`, `/api/auth/login` (succès et refus), `/api/auth/me` (avec et sans session), `/api/auth/refresh`, `/api/auth/logout` — statuts, corps, cookies |
| Les deux cookies, attribut par attribut | `epavillon_at` : `HttpOnly`, `SameSite=Lax`, `Path=/`, durée positive. `epavillon_rt` : `HttpOnly`, `SameSite=Strict`, **`Path=/api/auth`** |
| Un corps incomplet | 422 `VALIDATION_FAILED`, champ `password`, `request_id` présent, **et le texte de serde absent** |
| Un verrou échu | Compteur remis à **1** par la tentative en cours, aucun verrou reposé, et la connexion passe |
| Deux renouvellements simultanés | **Une seule** session ouverte, l'autre appel rend `IDENTITY_REFRESH_REUSED` |
| La discrétion, après les corrections | Écart de temps toujours sous 10 % — le test l'exige à chaque exécution |
| Le front compile toujours | `npm run typecheck` : 0 erreur. *(Le résolveur de `@vue/language-core` écrit deux avertissements `ERR_PACKAGE_PATH_NOT_EXPORTED` sur un greffon de `vue-router` ; ils sont antérieurs à ce jalon et n'échouent pas.)* |

---

# Phase 5 — US3 : le périmètre d'administration (T059–T067)

## Ce qui a été livré

**LE BACK-OFFICE SE PARTAGE SANS SE DUPLIQUER.** La règle métier n° 8 est tenue par l'API, et non
plus seulement par l'écran : un administrateur détaché sur une édition ne voit que ce qu'on lui a
confié, y compris quand il forge un identifiant dans l'URL.

Huit routes de lecture, toutes gardées :

| Route | Ce qui la garde | Ce qu'elle rend |
|---|---|---|
| `GET /people` | `identity.person.read`, **quelle que soit la portée** | les personnes |
| `GET /people/{id}` | soi-même, **décidé par la session**, ou la permission | la personne, `null` si elle n'existe pas |
| `GET /people/{id}/roles` | idem | les attributions **en cours**, portées résolues |
| `GET /people/{id}/permissions` | idem | `effective_permissions()`, telle quelle |
| `GET /people/{id}/administered-events` | idem | le périmètre, **jamais nul** |
| `GET /admin/users` | la permission **et** un périmètre non vide | la liste bornée, ses facettes, ses compteurs |
| `GET /admin/users/{id}` | idem | la fiche complète, `in_scope` disant si elle est modifiable |
| `GET /admin/users/{id}/effective-permissions` | idem | les permissions **enrichies de leur origine** |

Le garde du noyau gagne deux pièces que les cinq modules suivants réutiliseront : la permission
testée **sur n'importe quelle portée** — `has_permission()` ne répond que pour une portée précise, ce
qui refuserait un référent d'organisation à qui le contrat ouvre la lecture — et son extracteur de
route.

### L'origine d'une permission n'existe nulle part en base

`effective_permissions()` rend `(permission, portée)`. C'est **assez pour autoriser et pas pour
expliquer** — or l'écran demandé est un écran d'explication : « pourquoi cette personne peut-elle
décider d'un dossier ? ». La réponse est un rôle et une portée, et il peut y en avoir plusieurs pour
la même permission. Elle se recompose depuis les attributions et le catalogue, jamais depuis la
fonction.

### L'historique ne passe pas par le journal d'audit

`data-model.md` renvoyait à `platform.audit_log` pour l'historique des attributions. Il n'en a plus
besoin : depuis que `revoked_by` et `revoked_reason` ont été ajoutées au prompt A12, **une ligne de
`role_assignments` porte ses deux événements** — l'octroi avec son auteur et sa `note`, le retrait
avec les siens. Une attribution n'étant jamais supprimée, l'historique se lit sur la table, sans
dépendre du partitionnement ni de la rétention du journal.

## Les écarts relevés en implémentant

30. **`GET /people` n'est PAS bornée par le périmètre d'administration, et c'est le contrat qui le
    dit.** `contracts/routes.md` ouvre cette route à `identity.person.read` « quelle que soit la
    portée », sans mention de filtrage, là où `/admin/users` porte explicitement la borne. Les deux
    lectures ne servent pas la même chose : l'une nomme des personnes, l'autre compose l'écran
    d'administration avec ses comptes, ses verrous et ses demandes RGPD. **Le contrat a été suivi à la
    lettre**, et l'écart est noté ici plutôt que tranché seul — si la lecture doit se borner, c'est le
    contrat qui doit changer d'abord.

31. **Le rattachement d'une personne à une édition n'a qu'une source dans ce jalon.** Une personne
    n'appartient à aucune édition ; le lien se lit par l'autre bout — ce qu'elle y fait. Tant que
    `programme` et `event` n'ont pas de crate, **la seule trace de ce lien est l'attribution de rôle
    portant sur l'édition**. C'est donc le filtre retenu, et il est écrit dans le SQL de la liste, pas
    dans le code appelant : les autres rattachements (propositions, sessions, inscriptions) s'y
    **ajouteront** en B4 et B5, sans le remplacer. Le noter maintenant évite qu'on le redécouvre comme
    un oubli.

32. **`has_permission()` ne sait pas répondre « sur n'importe quelle portée ».** La fonction du modèle
    prend une portée en argument, et la portée globale ne couvre que les attributions globales. La
    question que pose le contrat — « détient-elle ce droit quelque part ? » — se lit sur
    `effective_permissions()`. Ajouté au **noyau**, comme le reste du garde : B2 en aura besoin dès sa
    première liste.

33. **La dénomination d'une organisation est un texte, pas un texte multilingue.**
    `org.organizations.legal_name` est du `text` ; le contrat du site attend un `I18nText` pour le nom
    d'une portée. La conversion est faite au transport (`{"fr": …}`) — c'est une mise en forme, pas
    une traduction inventée. Le jour où les dénominations deviendront multilingues, seul ce point
    changera.

34. **Une édition de test se sème « en ligne ».** `ck_events_physical_location` exige un pays et une
    ville dès que le mode de participation n'est pas `online` — deux colonnes qui ne changent rien au
    périmètre. Noté parce que le prochain qui sèmera une édition tombera dessus.

## Ce qui a été vérifié

| Contrôle | Résultat |
|---|---|
| `make check-back` | **Vert.** `cargo fmt --check`, `clippy --all-targets --all-features -D warnings` sans un avertissement, **59 tests** |
| La liste ne montre que le périmètre confié | Deux éditions, deux administrateurs détachés : chacun se voit, aucun ne voit l'autre, et l'écran **dit** qu'il a restreint |
| Une fiche hors périmètre | Rendue, avec `in_scope: false` — la taire ferait croire à une disparition, la rendre modifiable ouvrirait l'édition voisine |
| Un identifiant forgé sans la permission | **403.** « Soi-même » se décide par la session ; sur soi, la même lecture passe |
| Les permissions d'un administrateur détaché | Toutes de portée `event`, **aucune globale** — c'est ce qui bornera ses écritures en phase 8 |
| Les trois cas du périmètre | Global (couvre les éditions à venir), édition listée (et rien d'autre), **aucun droit → refus**, jamais une liste vide |
| Un rôle sans le droit qui compte | `editor` détient des permissions et **aucun périmètre** : le filtre porte sur `programme.proposal.read_all`, pas sur le nom du rôle |
| Le montage HTTP | `GET /api/people` et `GET /api/admin/users` sans session → **401** ; `/api/ready` → 200 |
| La limite de 1000 lignes | Le plus gros fichier de `backend/` fait **422 lignes** |

---

# Phase 6 — US4 : l'inscription et la vérification d'adresse (T068–T080)

## Ce qui a été livré

**LE COURRIEL ENTRE DANS LE JALON, ET IL SORT VRAIMENT.** Le parcours complet a été mené à la
main : inscription, courriel reçu dans Mailpit, lien suivi, adresse vérifiée, connexion réussie —
et le même lien rejoué rend « déjà utilisé ».

Trois routes, toutes en 200 : `POST /auth/register`, `/auth/verify-email`,
`/auth/verify-email/resend`. Et une quatrième, hors de `backend/` : la route privée d'envoi du site,
`frontend/server/api/internal/mail.post.ts`, seul morceau de B1 que la contrainte d'hébergement place
là.

### La réponse invariable tient par le message ET par le temps

`FR-035` demande la même réponse, adresse libre ou déjà prise. Le message identique ne suffit pas :
créer un compte coûte un hachage Argon2id de cinquante à cent millisecondes, ne rien créer coûte une
lecture d'une milliseconde. **Le mot de passe est donc haché dans les deux cas**, y compris quand
rien ne sera écrit — sans quoi le formulaire d'inscription redirait à la montre ce qu'on vient de
taire. C'est la même leçon que la phase 3, appliquée avant de se faire prendre.

### Le chemin du jeton en clair, et les deux endroits où il n'est pas

Le courriel doit porter le jeton en clair ; la base n'en garde que l'empreinte SHA-256. Le clair
voyage donc dans la charge utile du travail différé, **créée dans la transaction du changement
d'état** — un `ROLLBACK` n'en laisse rien. Il n'est **ni dans `identity.one_time_tokens`, ni dans
l'outbox**, et un test le vérifie en cherchant la chaîne elle-même dans les deux tables.

## Le trou de la file, refermé avant le premier envoi

`contracts/events.md` en avertissait : « **un travail mort garde sa charge utile** — à trancher avant
la phase des courriels ». `succeed()` la vide, `platform.fail_job()` ne la vide jamais : un envoi
passé en file morte aurait conservé son jeton en clair indéfiniment.

Il ne suffisait pas de vider toutes les charges utiles : pour `media`, `live` et `analytics`, celle
d'un travail mort est **la seule matière de diagnostic**. La tâche déclare donc ce qu'elle
transporte — `JobHandler::carries_secret()`, faux par défaut —, et le worker efface la charge utile
en même temps qu'il constate la mort. Le diagnostic est perdu là où il est dangereux, gardé partout
ailleurs.

Même examen pour `last_error`, que l'avertissement nommait aussi : le message d'échec d'un envoi
**ne recopie ni l'adresse du destinataire ni le corps renvoyé par le relais**. Il porte le code
stable et le statut HTTP, ce qui suffit à l'exploitation.

## Les écarts relevés en implémentant

35. **`localhost` n'est pas `127.0.0.1`, et un envoi sur deux échouait sans raison visible.**
    `npm run dev` fait écouter le site sur `127.0.0.1` ; sur macOS, « localhost » se résout d'abord
    en `::1`, où rien ne répond. `curl` replie sur IPv4 tout seul, le client HTTP du worker non — et
    l'erreur ne dit que « relais injoignable ». `MAIL_RELAY_URL` est passée **en chiffres** dans
    `.env.example` et `.env`, avec le commentaire qui explique pourquoi.

36. **Le premier appel au site en développement dépasse le délai d'attente de quinze secondes.**
    Nitro compile la route à la volée ; le courriel part, la réponse arrive trop tard, et le travail
    est replanifié. **Rien n'est cassé** — c'est exactement le cas que la mémoire courte des
    identifiants absorbe, et elle l'absorbe : `duplicate_ignored`. Le délai n'a pas été allongé : en
    production la route est compilée, et quinze secondes sont déjà généreuses pour une remise locale.

37. **L'invalidation d'un jeton se fait par son expiration, faute de colonne.** Le modèle porte
    `consumed_at` et `expires_at`, rien qui dise « remplacé ». Poser `consumed_at` dirait « c'est
    fait », ce qui est faux ; l'expiration est donc ramenée à maintenant, et qui clique un ancien
    lien lit « périmé » — le message juste, puisqu'un nouveau vient d'arriver.

38. **Une adresse connue *sans compte* suit le chemin du rappel, pas celui de la création.**
    Quelqu'un saisi comme intervenant existe sans s'être jamais connecté. Lui créer un compte à la
    volée reviendrait à laisser un tiers s'approprier une identité déjà présente dans la base ; le
    rappel « vous avez déjà un compte » est ce qu'il reçoit, et la réponse à l'écran ne change pas.

39. **Deux inscriptions simultanées sur la même adresse rendent toutes les deux la réponse unique.**
    C'est l'unicité de la base qui tranche, et la perdante traduit son conflit en… rien du tout :
    rendre `IDENTITY_EMAIL_ALREADY_USED` serait précisément la divulgation que FR-035 interdit. Le
    code d'erreur reste au catalogue pour les écritures d'administration, où il est légitime.

## Ce qui a été vérifié

| Contrôle | Résultat |
|---|---|
| `make check-back` | **Vert.** `cargo fmt --check`, `clippy --all-targets --all-features -D warnings` sans un avertissement, **75 tests** |
| Le parcours complet, à la main | Inscription → courriel dans **Mailpit** → lien suivi → `verified` → **connexion réussie** |
| Le même lien rejoué | `already_used`, et l'adresse reste vérifiée |
| Deux réponses d'inscription | **Identiques champ pour champ**, adresse libre ou déjà prise |
| Le courriel, lui, diffère | Lien de vérification d'un côté ; rappel **sans aucun lien** de l'autre |
| Le jeton en clair | Absent de `one_time_tokens` et de `outbox_events` ; présent dans la seule charge utile du travail, **vidée à la réussite** (`payload = {}` constaté) |
| Les trois refus et leur ordre | `invalid`, `expired`, `already_used` — et « consommé puis périmé » rend bien **`already_used`** |
| Deux consommations simultanées | Une seule aboutit, **un seul événement émis** |
| Un lien plus récent | Invalide le précédent (FR-040) |
| Le renvoi de lien | Même réponse sur adresse inconnue, en attente et déjà vérifiée — **un seul envoi** produit |
| La route privée sans secret | **404**, jamais 401 |
| Le dédoublonnage du site | Second appel du même identifiant : `duplicate_ignored`, aucun second courriel |
| Le front compile | `npm run typecheck` : 0 erreur |
| La limite de 1000 lignes | Plus gros fichier de `backend/` : **422 lignes** |

---

# Phase 7 — US5 : retrouver l'accès à un compte (T081–T088)

## Ce qui a été livré

**UN COMPTE PERDU SE RÉCUPÈRE SANS PASSER PAR L'ASSISTANCE.** Le cycle complet a été mené à la
main : demande, courriel reçu dans Mailpit, lien suivi, mot de passe choisi — et **l'ancien mot de
passe ne vaut plus rien**, la session ouverte avec lui rend `null`.

Trois routes : `POST /auth/password-reset` (réponse invariable), `GET /auth/password-reset/check`
(le jeton en **paramètre de requête**, c'est un `GET`), `POST /auth/password-reset/confirm`. Une
quatrième tâche différée, `identity.send_password_reset_email`, qui **déclare porter un secret**.

### Deux statuts, et ils ne disent pas la même chose

Un jeton refusé sort en **200** avec son discriminant — l'écran propose de redemander un lien. Un
mot de passe refusé sort en **422** sur le champ `password` — le formulaire se corrige sur place,
sans repasser par la boîte aux lettres. Un service qui rendrait les deux de la même façon laisserait
l'écran sans moyen de les distinguer.

### Le jeton est revérifié à l'envoi, et le contrôle ne vaut aucune garantie

FR-042. Le contrôle préalable sert à ne pas faire composer un mot de passe pour rien ; entre
l'affichage du formulaire et sa validation, le jeton a pu périmer. Un onglet ouvert la veille au
soir et rempli le lendemain matin est un cas ordinaire, pas une bizarrerie — et le test le joue.

### Ce qu'un changement de mot de passe emporte avec lui

FR-043, en trois effets dans la même transaction : le compteur d'échecs retombe à zéro, **le verrou
est levé** — sans quoi quelqu'un qui s'est verrouillé en cherchant son mot de passe resterait bloqué
un quart d'heure avec celui qu'il vient de choisir —, et **toutes les sessions tombent** avec le
motif `password_changed`.

## Les écarts relevés en implémentant

40. **Les liens des courriels menaient à des pages qui n'existent pas — depuis la phase 6.**
    Le site **traduit ses chemins** (`prefix_except_default`, français par défaut) : la vérification
    d'adresse se sert sur `/verification-adresse` et `/en/verify-email`. Les courriels pointaient
    vers `/auth/verify-email`, le nom du **fichier de page**, introuvable dans les deux langues — et
    un courriel dont le seul contenu utile mène à une 404 ne sert à rien. Découvert en suivant le
    lien de réinitialisation dans Mailpit. `mail.rs` porte désormais les quatre écrans cités et leur
    chemin dans chaque langue, avec un test qui les fige.

41. **L'acteur d'une réinitialisation ne se connaît qu'après la consommation du jeton.**
    La personne n'a pas de session : la transaction s'ouvre sans savoir qui parle, et l'identifiant
    sort du jeton consommé — donc de l'intérieur de la transaction. `kernel::db::set_actor()` repose
    `app.actor_id` en cours de route, sinon l'événement `identity.account.password_changed`
    porterait un acteur nul pour un changement qu'elle a bel et bien fait. Constaté rempli en base.

42. **La réponse invariable ne se comble pas ici comme à l'inscription.**
    À l'inscription, l'écart tenait au hachage Argon2id, payé des deux côtés. Ici il tient à trois
    écritures brèves contre une lecture : écrire un jeton factice pour une adresse inconnue mettrait
    **en file un courriel vers un destinataire qui n'existe pas**, et le remède serait pire. Ce qui
    est fait à la place : la transaction d'écriture est ouverte **avant** de savoir si l'adresse est
    connue, de sorte que les deux chemins paient le même aller-retour. L'écart restant est de
    quelques millisecondes sur un réseau public — les cinquante d'Argon2id, elles, se voyaient.

43. **Un mot de passe refusé ne brûle pas le lien.** Le mot de passe est jugé **avant** que le jeton
    ne soit consommé : la personne corrige et renvoie, sans repasser par sa boîte aux lettres. Le
    test l'exige explicitement — trois saisies refusées, puis la bonne, sur le même lien.

44. **La réinitialisation crée le compte mot de passe s'il n'existe pas.** `INSERT … ON CONFLICT`
    plutôt qu'`UPDATE` : quelqu'un dont le compte est né d'une invitation n'a pas encore de ligne
    `password`, et un lien de réinitialisation qui échouerait sur ce cas ne lui laisserait aucune
    issue. L'inférence porte sur l'index **partiel** `ux_accounts_password_per_person`, d'où la
    clause `WHERE` recopiée.

45. **Mailpit trie par date de réception, pas par date de demande.** Un travail replanifié livre son
    courriel après un plus récent : le premier essai à la main a été mené avec un jeton déjà
    invalidé par une demande postérieure, et l'API a eu raison de dire « périmé ». Pour la prochaine
    vérification manuelle : croiser l'heure du message avec `identity.one_time_tokens.created_at`,
    pas se fier à l'ordre de la boîte.

## Ce qui a été vérifié

| Contrôle | Résultat |
|---|---|
| `make check-back` | **Vert.** `cargo fmt --check`, `clippy --all-targets --all-features -D warnings` sans un avertissement, **88 tests** |
| Le cycle complet, à la main | Demande → courriel dans **Mailpit** → lien suivi → `reset` → **connexion avec le nouveau mot de passe** |
| L'ancien mot de passe | `invalid_credentials` |
| La session ouverte avant le changement | `GET /auth/me` rend **`null`** ; trois sessions révoquées, motif `password_changed` |
| Le même lien, deux fois | `already_used`, **en 200** |
| Un mot de passe refusé | **422**, `field: "password"`, message français — et le lien **n'est pas consommé** |
| Adresse connue / inconnue | `{"status":"sent"}` dans les deux cas ; **aucun jeton, aucun travail** créé pour l'inconnue |
| Un lien plus récent | Invalide le précédent, qui rend **`expired`** et non « déjà utilisé » (FR-040) |
| Le jeton périmé entre l'affichage et l'envoi | Contrôle `valid`, puis enregistrement **`expired`** — et le mot de passe reste inchangé (FR-042) |
| Un jeton de vérification présenté à la réinitialisation | **`invalid`**, jamais « déjà utilisé » — la finalité entre dans le filtre |
| Le verrou et le compteur | `failed_attempts = 0`, `locked_until = NULL` après un changement abouti (FR-043) |
| L'événement | **Un seul**, canal `reset`, **acteur renseigné** |
| La charge utile du travail | `payload = {}` après l'envoi ; effacée aussi à la mort du travail (`carries_secret`) |
| Le lien du courriel | `http://localhost:3000/nouveau-mot-de-passe?token=…` — la page répond **200** |
| La limite de 1000 lignes | Plus gros fichier de `backend/` : **502 lignes** (`tests/routes_auth.rs`) |

---

# Phase 8 — US6 : confier un rôle, le retirer, sur la bonne portée (T089–T099)

## Ce qui a été livré

**UN WEBINAIRE SE CONFIE À SON RESPONSABLE SANS LUI OUVRIR LE RESTE DE LA PLATEFORME.** Le scénario
de l'histoire a été mené à la main : une administratrice détachée sur une édition attribue là
(accepté), sur une autre et globalement (refusés), et ne peut pas retirer un rôle global qu'elle
n'aurait jamais pu accorder.

Quatre routes : `POST /admin/users/{id}/roles`, `DELETE /admin/users/roles/{id}`,
`PUT /admin/users/{id}/status`, `GET /admin/users/role-options`.

### La portée est le sujet de l'autorisation, pas un détail de la ligne

`identity.role.assign` sur la COP31 et la même permission globalement sont **deux droits
différents**. L'attribution se vérifie sur la portée **visée** ; le retrait, sur la portée de
**l'attribution visée** — et c'est cette seconde moitié qu'on oublie, celle sans laquelle une
administratrice d'édition défait un rôle global.

### Le refus du trigger ressort avec son message français

*(Obligation n° 3 du principe X.)* Le modèle sait déjà écrire « Le rôle « org_manager » ne peut pas
être attribué sur la portée « global » (portées autorisées : organization). » Le service le rend
**mot pour mot** : le reformuler produirait un second libellé qui se périmerait à la première
évolution du SQL.

### Le retrait pose trois colonnes ; il ne supprime jamais

`revoked_at`, `revoked_by`, `revoked_reason` — et `note`, le motif de **l'octroi**, reste intacte.
C'est ce qui répond, six mois plus tard, à « pourquoi cette personne n'est-elle plus au comité ? ».

## Les écarts relevés en implémentant

46. **`RoleWriteResult` gagne un champ `message`, et le type du site avec lui.**
    `contracts/errors.md` exige que `scope_not_allowed` porte le message du trigger « tel quel », et
    le type consommé par le site n'avait **aucun champ où le loger** : l'écran n'aurait rien pu
    afficher d'autre qu'« impossible ». Ajouter un champ est un changement **mineur** — le front
    l'ignore tant qu'il ne le lit pas. Ajouté au type TypeScript et aux données simulées pour que
    les deux contrats disent la même chose.

47. **Une portée ciblée sans cible se refuse avant la base — entorse assumée au principe VIII.**
    `ck_role_assignment_scope` dirait la même chose, mais **l'autorisation passe avant l'écriture**
    et sans cible il n'y a rien sur quoi tester le droit. Le refus porte donc le **même code** que
    la base rendrait, `IDENTITY_ROLE_SCOPE_MISMATCH` sur `scope_id` : l'appelant ne distingue pas
    les deux chemins.

48. **Un refus de la base avorte la transaction, et la réponse se compose après.**
    PostgreSQL n'accepte plus rien après une erreur : `duplicate`, `scope_not_allowed` et
    `missing_deadline` ne peuvent pas être composés dedans. Le service abandonne la transaction et
    relit sur le pool. Vérifier d'abord serait une course — et le principe VIII dit de laisser la
    base trancher.

49. **`forbidden_scope` divulguait les rôles de la personne visée.** *(Trouvé en éprouvant à la
    main, pas par un test.)* La route n'exigeait que la session : n'importe quel compte connecté
    lisait les attributions de n'importe qui en tentant une écriture vouée à l'échec, puisque le
    refus rendait la liste « pour que l'écran se recale ». Deux corrections : `identity.role.assign`
    **sur au moins une portée** ouvre désormais la route — sans elle, **403**, pas un discriminant
    —, et le refus de portée ne rend plus aucune liste. `forbidden_scope` répond à un administrateur
    qui vise la mauvaise portée, pas à un inconnu qui sonde.

50. **Une valeur hors liste se faisait passer pour un champ fautif.** *(Même origine.)*
    `status: "anonymized"` rendait `field: "anonymized"` : l'écran aurait souligné une case qui
    n'existe pas. Le filtre du noyau prenait le premier terme entre accents graves, or **seuls deux
    messages de serde nomment un champ** — `missing field` et `unknown field`. Les autres nomment
    une valeur. Sans préfixe reconnu, le refus sort sans champ : générique, mais jamais trompeur.

51. **Le changement de statut exige la portée globale, et le contrat ne le disait pas.**
    Il le précise pour la file RGPD, pas ici. Une suspension vaut sur **toute la plateforme** : il
    n'existe aucune édition à laquelle la rapporter, et un administrateur détaché sur une COP ne
    peut pas fermer un compte qui sert ailleurs.

52. **La réactivation efface la fin de suspension.** `suspended_until` est écrite quel que soit le
    statut : la laisser ferait afficher « suspendu jusqu'au… » sur un compte actif, à toute lecture
    qui ne teste pas le statut d'abord.

53. **Un doublon peut nommer une attribution expirée.** `ux_role_assignments_active` ne filtre que
    sur `revoked_at IS NULL` : une attribution arrivée à son terme **bloque encore**. Le conflit se
    cherche donc dans toutes les attributions, pas dans les actives — sinon le refus nommerait une
    ligne absente de la liste que l'écran affiche.

54. **Le mock du site range la fenêtre invalide sous `scope_not_allowed`.** Le contrat en fait une
    erreur 422 `IDENTITY_ROLE_WINDOW_INVALID`, et c'est ce que l'API rend. Écart laissé ouvert —
    c'est un chantier de front —, à refermer au raccordement (B7).

## Ce qui a été vérifié

| Contrôle | Résultat |
|---|---|
| `make check-back` | **Vert.** `cargo fmt --check`, `clippy --all-targets --all-features -D warnings` sans un avertissement, **110 tests** |
| Le scénario de l'histoire, à la main | Détachée sur la COP31 : attribution **acceptée** là, **refusée** sur l'autre édition et globalement |
| La symétrie du retrait | La même administratrice ne peut pas retirer un rôle **global** : `forbidden_scope` |
| Le refus du trigger | `scope_not_allowed`, message repris **mot pour mot** : « Le rôle « org_manager » ne peut pas être attribué sur la portée « global » (portées autorisées : organization). » |
| Le retrait | La ligne **reste**, avec `revoked_by`, `revoked_reason` — et la note d'octroi intacte |
| Le doublon | `duplicate`, avec l'identifiant de l'attribution en conflit |
| Les droits déclarés par le client | Sans effet : `person_id`, `actor_id` et `granted` envoyés dans le corps sont ignorés, le rôle se pose sur la personne de **l'URL** |
| Un compte sans droit d'attribution | **403**, et aucune attribution ne franchit la réponse |
| Les options d'attribution | Détachée : `can_assign_global` faux, une seule édition accordable, l'autre **visible et désactivée**, trois rôles offerts sur onze |
| La suspension sans terme | `missing_deadline` en **200**, et le statut reste `active` |
| La suspension avec terme | `saved`, motif et auteur enregistrés, session de la personne coupée (`/auth/me` → `null`) |
| La réactivation | `suspended_until` **effacée** |
| Le statut d'effacement | **422**, et **aucun champ désigné** — « anonymized » est une valeur, pas une case |
| Les événements | `identity.role.granted`, `identity.role.revoked`, `identity.person.status_changed` — un par changement d'état |
| La limite de 1000 lignes | Plus gros fichier de `backend/` : **588 lignes** (`service/admin_users.rs`) |

---

# Phase 9 — US7 : les effets différés partent une fois, et une seule (T100–T107)

## Ce qui a été livré

**La purge des jetons devient une tâche récurrente.** `repo/tokens.rs::purge()` existait depuis la
phase 6 et n'était appelée par personne. `jobs/purge.rs` en fait un gestionnaire de travail qui
**se replanifie lui-même** : rien dans le noyau ne porte de récurrence, et une boucle de plus dans le
worker aurait été un second ordonnanceur à surveiller. La clé d'unicité porte **le jour visé** — au
plus une purge par jour, quel que soit le nombre de redémarrages.

**Le démarrage du worker réarme la chaîne**, il ne la lance pas. La différence compte : une purge qui
meurt après ses cinq essais n'a pas replanifié la suivante, et sa clé occupe déjà l'index. Sans ce
réarmage, la chaîne se romprait définitivement, en silence. C'est pourquoi la planification de départ
est dans `crates/worker/src/main.rs` et pas dans le gestionnaire.

**La route de santé existe.** `GET /api/health` rend les quatorze indicateurs de
`analytics.v_operational_health` avec leurs seuils et la pire gravité rencontrée. **Les seuils ne
sont pas recalculés en Rust** : le modèle porte déjà la décision de ce qui mérite attention, et la
redoubler ferait deux vérités. `GET /api/ready`, elle, **sollicite réellement le pool** — un
processus qui répond pendant que sa base est injoignable n'est pas prêt, et le dire en 200 ferait
router du trafic vers un serveur qui ne peut rien servir.

**Cinq tests, dont trois dans le noyau.** `outbox_transactionnel` (obligation n° 4 du principe X),
`idempotence_consommateur`, `unicite_des_travaux`, `reprise_dessai`, `aucun_secret_en_base`.

## Les écarts relevés en implémentant

55. **`job_handlers()` prend désormais la base.** La purge écrit ; les trois envois de courriel, non
    — ils sont des fonctions pures de leur charge utile, et c'est ce qui leur permet de ne jamais
    dépendre d'une lecture qui aurait changé depuis la mise en file. Le premier travail du module qui
    touche la base fait entrer `Db` dans la signature, pour tout le monde.

56. **`analytics.v_operational_health` porte quatorze indicateurs, pas quatre.** Le contrat en nomme
    quatre — outbox en retard, travaux en échec, courriels en rebond, partitions manquantes — et les
    quatre y sont, sous les noms du modèle : `outbox_non_publie`, `travaux_file_morte`,
    `emails_rebond_7j`, `partitions_manquantes`. La route rend **tout** ce que la vue rend :
    filtrer sur quatre codes écrits en dur périmerait la route à la première ligne ajoutée à la vue.

57. **La permission `analytics.dashboard.read` est déclarée dans le crate `api`.** Elle appartient au
    module `analytics`, qui n'a pas de crate dans ce jalon. La déclarer dans `identity` laisserait
    croire qu'elle en relève ; elle vit donc auprès de la seule route qui la teste.

58. **Un travail replanifié n'est pas réservable avant son heure**, et c'est ce qui empêche une file
    en panne de tourner à vide. Éprouvé dans `reprise_dessai` : le tour qui suit un échec ne réserve
    rien du tout.

## Ce qui a été vérifié

| Contrôle | Résultat |
|---|---|
| **Le point de contrôle de la phase**, à la main | Inscription provoquée, **worker arrêté** : rien ne part, le travail reste `queued`. Worker relancé : **un seul courriel** dans Mailpit, charge utile vidée |
| La purge récurrente | Au démarrage : « purge des jetons planifiée pour aujourd'hui », exécutée, **et la suivante posée à minuit UTC**. Worker relancé le même jour : aucune seconde purge — la clé du jour tient |
| `GET /api/ready` | `{"status":"ok"}`, sans session |
| `GET /api/health` | **401** sans session, **403** avec une session sans le droit, 200 et quatorze indicateurs avec `admin` |
| Un site injoignable | Le travail échoue, se replanifie avec un délai croissant, et meurt au bout de **cinq** essais — puis sa charge utile est effacée parce qu'elle portait un jeton |
| Un worker tué en cours de lot | Le travail reste réservé ; la reprise ne le rend **pas** avant l'expiration du bail, et le rend ensuite **sans lui rendre son essai** |
| SC-009, par fouille du catalogue | Après le cycle complet, **aucune** colonne textuelle de `identity` ni de `platform` ne contient le mot de passe, les deux jetons de session, ni les deux jetons de lien |

---

# Phase 10 — US8 : honorer une demande RGPD (T108–T115)

## Ce qui a été livré

**La file, et elle ne se borne pas.** `GET /admin/privacy-requests` exige
`identity.person.manage` **en portée globale**. Un administrateur détaché sur une édition reçoit
**403**, jamais une file filtrée : filtrer donnerait l'illusion d'un traitement complet à qui ne voit
qu'un morceau, alors que le délai réglementaire court sur le reste. Le service ne prend **aucun**
argument de périmètre — il ne peut structurellement pas filtrer.

**L'effacement ne répond qu'à une demande d'effacement** (FR-060). Le contrôle vient avant
l'écriture : un export anonymisé détruirait une identité que personne n'a demandé d'effacer, et rien
ne la rendrait. Le refus sort en **200** avec `wrong_type`, et la demande n'avance pas.

**Le service n'émet pas `identity.person.anonymized`**, et il dit pourquoi à l'endroit où l'on serait
tenté d'ajouter la ligne : `identity.anonymize_person()` appelle elle-même `platform.emit_event()`.

**Les lectures RGPD quittent `repo/admin_users.rs`** pour `repo/privacy.rs`, avec la file complète, le
verrouillage d'une demande, son marquage et son dépôt.

## Les écarts relevés en implémentant

59. **La route de dépôt n'existe pas dans ce jalon.** Le contrat ne porte que la file et son
    traitement ; l'écran de profil d'où une personne déposerait sa demande arrive plus tard.
    `service::privacy::submit()` existe quand même, et **c'est le seul endroit d'où
    `identity.privacy_request.received` doit partir** : l'écrire ailleurs le jour venu produirait
    deux dépôts qui ne se ressemblent pas.

60. **`deadline_days` est une constante Rust, et la table en est la source.** `due_at` vient d'un
    `DEFAULT` — rien ne relie les deux, et elles dériveraient sans que rien ne le dise. `effacement.rs`
    les rapproche : il dépose une demande et compare `due_at - created_at` à la constante. C'est le
    seul garde-fou possible pour une valeur qui vit dans un `DEFAULT`.

61. **Les quatre issues portent la file, y compris `not_found`.** C'est l'inverse de l'arbitrage du
    refus de rôle, et pour une raison précise : là-bas, `forbidden_scope` répondait à quelqu'un **qui
    n'avait pas le droit de lire** ; ici, les quatre issues répondent à un appelant qui lit déjà cette
    file par la même permission. Lui rendre ne divulgue rien et lui évite un aller-retour.

62. **`completed_at` suit l'état, il n'est pas un argument.** Laisser l'appelant le choisir
    permettrait une demande « en cours » datée de sa clôture.

63. **L'anonymisation efface l'identité et laisse les compteurs.** Vérifié par une inscription à une
    session : la ligne de `programme.registrations` survit avec ses minutes de présence, parce que la
    personne n'est **pas supprimée** — son identifiant technique reste, seule son identité part.

## Ce qui a été vérifié

| Contrôle | Résultat |
|---|---|
| Un administrateur d'édition | **403**, périmètre non vide et pourtant refusé — un périmètre ne vaut pas la portée globale |
| Un administrateur global | La file entière, y compris la demande d'une personne rattachée à **aucune** édition |
| L'anonymisation sur un export | `wrong_type` en **200**, statut de la personne inchangé, rien d'effacé |
| L'anonymisation sur un effacement | `anonymized` : identité purgée, comptes supprimés, sessions révoquées, jeton d'accès qui ne résout plus personne |
| **Un seul événement** | `identity.person.anonymized` : **une** ligne, celle de la fonction SQL |
| Les compteurs de participation | **Inchangés** — inscription et minutes de présence intactes après effacement |
| Les trois actes administratifs | `start`, `complete`, `reject` font avancer le dossier et **ne touchent pas** à l'identité |
| L'échéance | 30 jours, portés par le `DEFAULT` de la table, et annoncés tels quels par l'écran |

---

# Phase 11 — Finition et points transverses (T116–T126)

## Ce qui a été livré

**La documentation OpenAPI est engendrée, jamais écrite.** Vingt-six chemins — les vingt-quatre du
module et les deux d'exploitation —, chacun annoté auprès du gestionnaire qu'il décrit. Une route
ajoutée sans son annotation ne se documente pas ; une route documentée qui disparaîtrait ne
compilerait plus. C'est le seul couplage qui empêche une documentation de mentir.

**Le catalogue d'erreurs est engendré depuis `ErrorCode::ALL`** : les vingt codes stables, avec leur
statut HTTP et leur message français, injectés dans le schéma `ApiError` au montage. Un code ajouté
au noyau apparaît dans la documentation au prochain démarrage, et un code oublié n'existe pas. Deux
tests le tiennent.

**`GET /api/docs` sert le document**, ouvert par défaut et fermé par `API_DOCS_ENABLED=false` en
production, où il décrirait la totalité de la surface d'appel à qui sonde le port.

## Les écarts relevés en implémentant

64. **Les formes de réponse sont désignées par leur nom TypeScript, et déclarées `object`.** Leur
    source unique est `frontend/app/types/` — le contrat de routes le dit —, et en dériver un second
    jeu de schémas en Rust produirait deux vérités dont la seconde se périmerait à la première
    évolution du site. Le nom mène au fichier qui fait foi. **Le catalogue d'erreurs, lui, vit
    entièrement en Rust** : il est donc engendré, code par code. À reconsidérer au raccordement (B7),
    quand les deux côtés se parleront pour de vrai.

65. **L'interface web de la documentation n'est pas embarquée.** `utoipa-swagger-ui` télécharge ses
    fichiers **à la compilation** : une dépendance réseau dans le portail de vérification, pour un
    confort que n'importe quel lecteur d'OpenAPI apporte déjà. Le document est le livrable.

66. **Le schéma d'erreur est reposé APRÈS la fusion des documents.** `merge` garde le schéma déjà
    présent : la déclaration plate que l'API porte pour ses propres routes l'emportait, et la
    documentation n'énumérait aucun code. Trouvé par le test, pas par la lecture.

67. **`platform.audit_log.actor_label` n'était renseignée par personne** — alors que la colonne existe
    pour « rester lisible après anonymisation RGPD ». Sans elle, le nom de l'auteur se lit par
    jointure, et une personne qui exerce son droit à l'effacement fait devenir « Utilisateur
    anonymisé » toutes ses décisions passées. **Le modèle est corrigé** : `platform.tg_audit()` lit
    `display_name` à l'écriture, seul instant où le nom existe encore.

68. **Le relais du site envoyait trois fois le même courriel.** Il retenait l'identifiant du message
    **après** l'envoi ; or le doublon réel est concurrent — le client de l'API abandonne au bout de
    quinze secondes et réessaie deux secondes plus tard, pendant que le premier envoi est encore en
    cours. La garde ne protégeait que d'un doublon séquentiel, qui n'arrive jamais. L'identifiant est
    désormais **réservé avant** l'envoi, et rendu si l'envoi échoue : perdre un message vaut pire que
    le tenter deux fois. **Trouvé en éprouvant à la main, pas par un test.**

69. **Une seule trace d'audit est légitimement anonyme** : l'inscription de soi-même. La personne n'a
    pas encore d'identifiant quand sa ligne est écrite, et poser « acteur = la personne créée » serait
    un mensonge — elle n'était pas authentifiée. `toute_ecriture_laisse_son_auteur.rs` la borne à
    **une seule ligne, sur sa propre création**.

## Ce qui a été vérifié

| Contrôle | Résultat |
|---|---|
| **`make check` en entier, depuis la racine** | **Vert, code de sortie 0, zéro avertissement.** Base détruite et rechargée de zéro, seize schémas, rapport de frontières vide, projections analytiques rafraîchies, site compilé, `clippy -D warnings` muet, **141 tests** |
| Requêtes préparées | **143**, engendrées avec `--all-targets --all-features` |
| Taille des fichiers | Plus gros fichier de `backend/` : **588 lignes** (`service/admin_users.rs`) — la limite est à 1000 |
| Frontières de crates | `cargo tree -p identity \| tail -n +2 \| grep -c "crates/modules/"` → **0** |
| `GET /api/docs` | **26 chemins**, **20 codes d'erreur** — le catalogue entier |
| Les six issues de connexion, à la main | `authenticated`, `mfa_required`, `invalid_credentials`, `locked` (avec sa date de fin), `suspended` (avec sa date de fin), `email_unverified` (**sans cookie**) |
| La discrétion, à la main | Adresse inconnue : médiane **22,2 ms** · mot de passe faux : **22,7 ms** — réponse identique au caractère près |
| Le cycle d'inscription, à la main | Réponse invariable sur adresse libre et prise, courriel de vérification puis rappel de compte existant dans Mailpit, lien suivi → vérifié, rejoué → **`already_used`** |
| Le verrouillage, à la main | Cinq échecs, puis `locked` **avec sa date de fin** au sixième essai fait avec le **bon** mot de passe |
| Le périmètre, à la main | Administrateur d'édition : une ligne dans `/admin/users`, `scoped_to_events` vrai, **403** sur la file RGPD. Compte sans droit : **403** sur `/admin/users` |
| La coupure de session | Suspension posée en base → la session ouverte de la personne rend **401** à la requête suivante |

---

# Hors lot — les en-têtes CORS, avancés de B7 (20/08)

## Pourquoi maintenant

La question posée était : *peut-on éprouver les huit histoires depuis un vrai navigateur ?* La réponse
était non, et pour trois raisons empilées :

1. le site tourne sur les données simulées — `NUXT_PUBLIC_API_BASE` est vide, et le renseigner ferait
   afficher l'état d'erreur sur tous les écrans de B2 à B6, dont les routes n'existent pas ;
2. **l'API ne posait aucun en-tête CORS**, donc aucun appel de navigateur ne pouvait aboutir ;
3. contourner le site en pilotant l'API depuis sa propre origine ne marchait pas non plus : l'
   intergiciel d'origine refuse toute écriture dont l'`Origin` annoncée n'est pas `APP_PUBLIC_URL`, et
   un navigateur en pose une sur chaque écriture. **Ce n'est pas un défaut** — c'est la protection
   anti-requête-forgée qui fait son travail.

Seul le point 2 était une dette. Une fois levée, un navigateur ouvert sur **n'importe quelle page du
site** peut appeler l'API sans que `NUXT_PUBLIC_API_BASE` bouge : l'origine est la bonne, et aucun
écran ne casse. C'est ce qui a permis d'éprouver les huit histoires pour de vrai.

## Ce qui a été livré

`crates/api/src/middleware/cors.rs` — le pendant, côté réponse, d'`OriginCheck`, dont il **partage la
liste d'origines et la fonction de normalisation**. Quatre décisions valent d'être écrites :

- **Jamais `*`.** Le navigateur refuse une origine générique dès que `Allow-Credentials` est vrai.
  L'origine est renvoyée telle qu'annoncée, après contrôle.
- **Le préalable `OPTIONS` ne franchit pas l'intergiciel.** Le chemin visé n'accepte souvent que
  `POST` : le laisser aller au routeur rendrait 404 ou 405 là où le navigateur attend une permission.
- **`X-Request-Id` est exposé.** Le contrat annonce qu'il voyage sur toute réponse ; sans cette ligne,
  le navigateur le cache au code du site, et personne ne peut le citer dans un signalement.
- **Les réponses d'erreur portent les en-têtes.** D'où la place de l'intergiciel — **le plus à
  l'extérieur**, pour envelopper aussi les refus des trois autres. Sans cela, un 401 ou un 403 est
  masqué, et l'écran affiche une panne réseau à la place du message que l'API a composé.

Les en-têtes demandés au préalable sont **renvoyés tels quels** plutôt que comparés à une liste
fermée : l'origine est déjà contrôlée, et une liste écrite dans le code échouerait en silence le jour
où le site ajoute un en-tête.

## Ce qui a été vérifié

Cinq tests (`entetes_cors`), et **les huit histoires pilotées depuis un vrai navigateur** — ce que
`curl` ne prouve pas :

| Histoire | Ce que le navigateur a montré |
|---|---|
| **US1** connexion | `email_unverified` avant vérification puis `authenticated` ; mot de passe faux et adresse inconnue rendent **le même corps, au caractère près** |
| **US2** sessions | `/auth/me` → `null` hors session ; connexion, `renewed`, `signed_out`, `null` de nouveau. **Les cookies de session sont invisibles à `document.cookie`** — `HttpOnly` tient — et pourtant envoyés. **`epavillon_rt` n'apparaît même pas dans le bocal à la racine** : son `Path=/api/auth` le confine aux routes d'authentification, et la rotation a pourtant réussi |
| **US3** périmètre | Administratrice globale : 3 lignes, `scoped_to_events` faux. Administrateur d'édition : **1 ligne**, `scoped_to_events` vrai |
| **US4** inscription | Deux inscriptions sur la même adresse : **corps identiques**, aucun cookie posé. Mailpit montre **un lien de vérification et un rappel de compte existant** — le second sans aucun lien |
| **US5** réinitialisation | Contrôle `valid`, mot de passe trop court en **422** sur le champ `password` **sans brûler le lien**, changement `reset`, session coupée (`null`), ancien mot de passe refusé, nouveau accepté, lien rejoué `already_used` |
| **US6** rôles | Sur sa COP : `granted`. En portée globale : `forbidden_scope`, **et rien ne fuite avec le refus**. Deux fois le même : `duplicate`. `super_admin` réclamé avec `granted: true` dans le corps : **sans effet**. Et depuis la portée globale, le message du trigger **mot pour mot**. Le retrait d'un rôle global par un administrateur d'édition : `forbidden_scope` |
| **US7** effets différés | Un travail, un courriel. Rien ne part worker arrêté ; un seul courriel au redémarrage |
| **US8** RGPD | Administrateur d'édition : **403**, jamais une file filtrée. Administratrice globale : la file, échéance 30 jours, `start` puis `anonymize` — et la personne devient « Utilisateur anonymisé … » |

Une confirmation involontaire, et bonne à prendre : le premier essai a été lancé depuis l'onglet
**Mailpit**, dont l'origine n'est pas autorisée. Le navigateur a bloqué l'appel. La liste d'origines
fait donc bien son travail, observée en direct.

**Le site n'a rien perdu** : `NUXT_PUBLIC_API_BASE` reste vide, et l'écran de connexion se rend comme
avant, sur les données simulées.
