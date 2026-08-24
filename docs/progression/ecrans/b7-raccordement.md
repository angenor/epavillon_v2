# B7 — Raccordement du front à l'API

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de ce module est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md) § PHASE B — B7.

**État** : ✅ **LIVRÉ le 22/08. LE SITE PARLE À L'API.** Le client TypeScript est **engendré** depuis les routes Rust et vérifié à chaque `make check` ; **152 écarts ont été tranchés** un par un, chacun avec sa preuve lue des deux côtés ; les quatre routes qui manquaient au socle sont écrites ; la session voyage, se renouvelle et se perd proprement ; et **une API injoignable ne casse plus une seule page**.

---

## Ce qui a été livré

### Le contrat, engendré et non écrit

`frontend/app/types/api.ts` vient de `make openapi`, qui exporte le document OpenAPI par un **binaire** — `cargo run -p api --bin openapi` — et non par la route `/api/docs`. La distinction n'est pas cosmétique : la route ne décrit que les modules **réellement montés**, si bien qu'engendrer le client depuis une base de développement en retirerait les chemins d'un module éteint ce jour-là. Le binaire ne touche ni base ni réseau.

Il porte **156 chemins, 175 opérations, le catalogue des 65 codes d'erreur stables** et la forme du corps d'erreur. Il ne porte **pas** les corps de requête et de réponse : l'API les désigne par leur **nom TypeScript** dans la description de chaque opération, et leur définition reste dans `frontend/app/types/`. C'est une décision de l'API — décrire deux fois la même forme produit deux vérités qui divergent au premier ajustement — et ce lien, purement déclaratif, est désormais **vérifié** :

`make check-api-contract`, branché sur `check-front`, refuse trois choses. Un chemin appelé par le site et absent du contrat. Une forme annoncée par l'API et introuvable dans `app/types/`. Une route laissée en données d'exemple alors que l'API la sert. Il rend aujourd'hui : **128 appels sur 156 chemins, 123 formes annoncées — toutes définies**, et **19 routes en attente d'API**, nommées.

Un garde-fou de plus, côté Rust cette fois : le binaire d'export **refuse un `operation_id` porté deux fois**. Le défaut existait — `fiche`, sur la fiche d'une personne et sur celle d'un utilisateur du back-office — et il produisait un fichier TypeScript qui ne compilait pas, avec un message ne nommant ni la route ni le module.

### Le noyau de la bascule

- **`utils/api-error.ts`** — deux façons d'échouer, et pas une de plus. `ApiRequestError` : l'API a répondu et refuse ; elle porte son code stable, son **message français déjà composé par l'API**, le champ fautif et la référence d'incident. `ApiUnreachableError` : l'API n'a pas répondu. La distinction commande tout le reste — la première se raconte à la personne, la seconde allume le mode dégradé. **Un statut sans corps d'API n'est pas un refus d'API** : c'est un 502 de passerelle ou une page d'erreur en HTML, et le traiter comme un refus ferait afficher « La ressource demandée est introuvable » alors que c'est l'API entière qui manque.
- **`composables/api/http.ts`** — le client. Cookies transmis au rendu serveur (`credentials: 'include'` ne vaut que dans un navigateur), rotation du jeton **une fois et jamais deux en parallèle**, erreurs normalisées, mode dégradé tenu. Un `FORBIDDEN` générique devient `ForbiddenError`, que l'écran traduit en « accès refusé » ; les 403 qui **nomment** leur raison — « seul un référent peut faire cela », « ce dossier ne vous est pas confié » — restent des messages, qu'un écran plein écran ferait disparaître.
- **`composables/useApi.ts`** — quatre primitives au lieu de deux. `call` (un 404 lève), `callOrNull` (un 404 est une réponse), `send` (**jamais rejouée** : `retry: 0`), et `pending`.
- **`useApiStatus` / `UiApiOfflineBanner`** — une API injoignable n'est pas l'erreur d'un écran mais un état de la plateforme. Sans état partagé, une page qui charge six blocs afficherait six messages identiques, et la personne en conclurait que six choses sont cassées.
- **`useMockData` / `UiMockDataBanner`** — les trois écrans que l'API ne sert pas encore lisent des exemples **et le disent**, en nommant les routes attendues.

### Ce qui a été ajouté à l'API

Quatre routes manquaient au **socle transverse**, et trois écrans du jalon s'arrêtaient net sans elles — inscription, rattachement à une organisation, dépôt d'un dossier : `GET /reference/countries`, `/reference/locales`, `/reference/taxonomies/{code}/terms` et `/platform/feature-flags`. Elles vivent dans le crate `api` et non dans un module, parce que `reference` et `platform` sont des schémas **transverses** : les loger dans un crate de module obligerait les cinq autres à en dépendre. Aucune n'exige de session — un formulaire d'inscription a besoin de la liste des pays avant qu'un compte existe.

**Les drapeaux sont rendus RÉSOLUS**, pas bruts : `platform.is_feature_enabled(clé, personne)` tranche en base. Rendre la table obligeait le site à réimplémenter le déploiement progressif — donc à en donner une seconde version — et publiait au passage les identifiants des personnes visées.

Sept autres corrections, toutes dans des modules déjà livrés :

| Ce qui n'allait pas | Ce qui a été fait |
|---|---|
| La grille d'évaluation d'un appel n'avait aucune route, et la page publique d'une édition avec appel échouait en entier | `GET /events/{id}/call` porte sa grille (`PublicCall.criteria`) — une vague d'appels en moins pour la page qui l'affiche |
| L'espace organisation rendait **404** quand on n'est pas membre : les trois écrans affichaient « une erreur est survenue » | `200` avec `null`. L'indiscernabilité voulue est intacte ; elle ne demandait pas un statut d'erreur |
| La fiche d'une organisation se cassait quand la projection matérialisée ne la connaissait pas encore | Fiche de performance **toujours pleine**, compteurs à zéro — ce que fait déjà la liste |
| La relecture d'un dossier émettait **deux fois** les clés `co_organizations` et `speakers` | `skip_serializing` sur les deux champs du `flatten`. Cela marchait par accident |
| La santé d'exploitation exigeait la portée globale | Permission sur n'importe quelle portée : ce qu'elle révèle touche d'abord les rappels d'un administrateur détaché |
| Le formulaire d'édition proposait des pays retirés du référentiel | `WHERE is_active` |
| Cinq noms de contrat désignaient autre chose que ce que l'API rend | `email_unverified`, `RoleAssignmentView[]`, `PublicEditionRow[]`, `SessionRegisterPayload`, `SaveDraftPayload` |

### Ce qui a été corrigé côté site

Neuf périmètres, traités par autant d'agents sur des fichiers exclusifs, chacun relu par un second. Les corrections les plus lourdes : le brouillon repris qui ne contenait rien, les intervenants qui partageaient tous la même clé de liste, un troisième refus de dépôt que l'écran n'affichait pas, la recherche de doublons qui interrogeait la recherche du grand public, les écritures sans branche d'erreur, et **quatorze formes** écrites d'après les structs Rust et le SQL.

---

## Les écarts, et comment ils ont été tranchés

**152 écarts**, groupés en douze familles, chacun avec son verdict — défaut du site, défaut de l'API, conforme, hors périmètre — et sa preuve. Les verdicts « bloquant » ou « défaut de l'API » sont passés par une **contre-expertise** chargée de les réfuter ; elle en a **renversé quatorze et ajouté trois**, dont trois défauts d'API qui n'en étaient pas et auraient coûté du travail inutile côté Rust.

### La ligne de partage

Ce qui manque à un module **livré**, on le corrige. Ce qui exige un module **non livré**, on le déclare et on le consigne. C'est ce qui distingue un raccordement d'un développement de module, et c'est ce qui explique les 19 routes en attente.

### Écart n° 138 — l'OpenAPI ne décrit pas ses corps

**Structurel, et assumé.** Les corps sortent en `Record<string, never>` : l'API les nomme, elle ne les décrit pas. Verdict : **conforme à la décision de l'API**, mais le lien devait cesser d'être déclaratif — d'où `check-api-contract`.

### Écart n° 139 — trois écrans du jalon n'ont aucune API

Les messages d'incident (A13), l'accueil public et sa vitrine (A15), le tableau de bord (A6). Leurs données vivent en base — schémas `live`, `content`, `analytics` — mais **aucun crate Rust ne les sert**. Verdict : **défaut de l'API, hors périmètre du raccordement**. Les faire appeler leur route produirait un 404 : un écran livré et vérifié se mettrait à afficher une panne le jour où l'adresse est posée. Ils passent donc par `pending()`, lisent les exemples **API configurée ou non**, et le **disent**. Le faux-semblant aurait été de servir ces données sans le signaler ; l'écran cassé aurait été de les réclamer à une API qui ne les a pas.

### Écart n° 140 — une instance d'erreur ne traverse pas le rendu serveur

**Trouvé en éprouvant le mode dégradé, et invisible autrement.** Les stores retenaient l'erreur elle-même. Le payload de Nuxt est composé par `devalue`, qui refuse tout ce qui n'est pas un objet simple : une `ApiUnreachableError` posée dans un store faisait échouer la sérialisation de la page **entière**, et le visiteur recevait un **500** au lieu de l'écran dégradé écrit pour lui. Le défaut est d'autant plus vicieux qu'il n'apparaît **que lorsque l'API est en panne** — c'est-à-dire au seul moment où ce chemin sert. Les stores retiennent désormais un `LoadFailure` : message, code, référence d'incident.

### Écart n° 141 — le rendu serveur ne peut pas renouveler un jeton

Le cookie de rafraîchissement est limité au chemin `/api/auth` : il n'atteint **jamais** le serveur Nuxt, qui vit sur un autre port. Ce n'est pas un manque — c'est ce qui borne le dégât d'une fuite. Conséquence : la rotation est un geste du **navigateur**, exclusivement, et un témoin de session non secret dit s'il vaut la peine de la tenter. Sans lui, chaque page vue par un visiteur anonyme paierait un appel à `/auth/refresh`.

---

## Ce qui a été vérifié, et comment

**Contre l'API réelle, pas contre une supposition.**

- **Le parcours de session, de bout en bout** : inscription → `verification_sent` ; connexion avant vérification → `email_unverified` ; connexion après → `authenticated`, **deux cookies aux bonnes portées** (`epavillon_at` sur `/`, `SameSite=Lax`, 15 min ; `epavillon_rt` sur `/api/auth`, `SameSite=Strict`, 12 h) ; `/auth/me` → la personne ; **`/auth/refresh` → `renewed`**, et la session continue.
- **`/auth/me` ne rend jamais 401** — vérifié sans session : `200 null`.
- **Une écriture depuis une origine inconnue est refusée** : `403 IDENTITY_ORIGIN_REJECTED`, message français, référence d'incident. Depuis l'origine autorisée, les en-têtes CORS sont complets.
- **Le site lit bien la base et non les mocks** : la page d'inscription rend **250 options de pays** portant les identifiants de `reference.countries` — les données simulées en comptent une trentaine, avec d'autres identifiants.
- **Mode dégradé, API arrêtée** : `/inscription` et `/connexion` rendent **200** avec le bandeau « Liaison interrompue » et le message français. Avant la correction de l'écart n° 140, la même page rendait **500**.
- **Reprise** : l'API relancée, le bandeau disparaît et les 249 pays reviennent, sans rechargement forcé.
- **`make check` en entier — base détruite et rechargée de zéro, 857 tests au vert, `clippy -D warnings` sans un avertissement.** **Un test intermittent de B6 a été corrigé pour cela** : `le_travail_se_replanifie_sans_doublon` relisait `now()` à chaque armement alors que la clé d'unicité du travail porte l'horodatage **à la seconde**. Une fois sur dix, les deux appels tombaient de part et d'autre d'une seconde, posaient deux clés différentes, et le test échouait en mesurant le contraire de ce qu'il affirme. L'instant est désormais un paramètre : le second armement rejoue le **même** créneau, comme le fait un worker qu'on redémarre.

### Ce qui n'a pas pu être vérifié

Le parcours métier complet — déposer un dossier, le noter, le programmer — **n'a pas été joué sur l'API réelle** : la base de développement ne porte que son semis, et aucune édition publique n'y existe. Les routes concernées sont couvertes par les tests d'intégration du backend, qui montent leurs propres données. Le point de contrôle manuel reste à faire sur une base semée.

---

## Ce qui reste dû

**À l'API** — hors périmètre du raccordement, chacun exigeant un module non livré :

1. Le crate `live` — sept routes des messages d'incident (A13).
2. Le crate `content` — dix routes de la vitrine, plus `GET /home` (A15).
3. La composition `GET /admin/dashboard` et les projections `analytics` (A6), plus le rafraîchissement récurrent des vues matérialisées par le worker.
4. Ajouter ou retirer une **dénomination** et un **domaine** d'organisation : aucune route ne l'expose, et c'est la règle métier n° 1 qui en dépend.
5. `review_note` sur `org.duplicate_candidates` — la justification saisie pour écarter une paire est envoyée puis perdue. **Le SQL d'abord**, selon la règle du projet.
6. `location_note` au corps de `PUT /sessions/{id}/schedule` — l'une des deux façons de lever le point bloqué « séance sans lieu ».

**Au site** — le dépôt de fichiers reste à brancher : la photo d'un intervenant et les pièces d'un dossier sont saisies, montrées, puis perdues. `POST /media/assets` existe depuis B6 ; il faut lui envoyer un corps composite, ce que la couche d'accès ne sait pas encore faire.

**Au commanditaire** — une décision remontée en chemin : une fois l'édition publiée, **republier ne publie plus rien**. Les séances arbitrées après la première publication ne deviennent donc jamais publiques.

---

## Complément du 24/08 — l'accueil dit vrai sur trois blocs

**L'accueil ne retombait pas sur les exemples par choix, mais par tout ou rien** : il demandait ses
cinq blocs en un seul appel — `GET /home` —, et cette route appartient au module de la vitrine, qui
n'existe pas. Un bloc manquant faisait donc basculer l'écran entier.

Trois de ces cinq blocs étaient pourtant déjà servis. Ils le sont désormais :

| Bloc | Source |
|------|--------|
| Les éditions passées et à venir | `GET /events/public` |
| Les prochaines séances | `GET /schedule`, **sans édition** |
| Les chiffres de chaque édition | **aucun appel** — chaque ligne d'édition les porte déjà |
| Le bandeau qui défile | en attente du module de la vitrine |
| Les épingles « À venir » | en attente du module de la vitrine |

**`GET /schedule` accepte une lecture sans édition** (écart n° 141). L'accueil compose une liste
toutes COP confondues : lui faire nommer une édition le ferait passer à côté des autres. La lecture
est alors bornée dans les deux sens — séances passées écartées, plafond de 50 lignes, 200 au maximum.
Sans ces bornes, la page d'accueil rendrait la programmation entière de toutes les éditions de
l'histoire de la plateforme à chaque affichage. **`par_adresse` ne prend aucun plafond** : elle cherche
une séance précise, et la borner ferait rendre « introuvable » pour une séance qui existe mais arrive
tard dans le programme.

**Les chiffres du programme ne coûtent aucun appel** (écart n° 142). Le Rust les joignait déjà à
chaque ligne de `GET /events/public`, par la gauche ; c'est `PublicEditionRow` côté site qui les
ignorait. Cinq colonnes ajoutées au type, et l'accueil compose ses statistiques à partir de la liste
qu'il tient en main plutôt que de les redemander.

### Un défaut réel, qu'aucune vérification mécanique n'aurait trouvé

**Le bandeau des données d'exemple ne s'affichait jamais** — et le marquage, lui, avait bien lieu.
Le middleware qui vide la liste à chaque navigation **se rejoue à l'hydratation**, sur la route que le
serveur vient de rendre : il effaçait le marquage une fraction de seconde avant que le bandeau ne se
rende. Il compare maintenant `to` et `from`, et ne vide que sur un vrai changement de page.

Le défaut ne se voyait ni au `typecheck`, ni au contrat, ni dans le rendu serveur — seulement en
regardant l'écran. C'est l'argument pour la vérification au navigateur, et il a resservi ici.

### Ce qui a été vérifié

- Au navigateur : `GET /api/events/public` et `GET /api/schedule` partent réellement.
- L'accueil affiche « Aucun appel à propositions ouvert » — **ce qui est vrai** : la base ne porte
  aucune édition.
- Le bandeau nomme la seule route qui manque, et son texte ne prétend plus que tout l'écran est un
  exemple.
- 7 tests de la programmation publique au vert, dont un nouveau sur la lecture sans édition et son
  plafond.

### Ce que cela découvre, et qui n'est pas tranché

**La base de développement ne porte aucune donnée métier** : 249 pays, 74 termes, 13 drapeaux,
4 séries, 1 organisation — mais **zéro édition, zéro séance, zéro proposition**. Les écrans branchés
affichent donc leurs états vides. C'est exact, et c'est inutilisable pour montrer quoi que ce soit.

Deux voies, à trancher avec le commanditaire : créer la COP31 depuis le back-office — le parcours que
le jalon vise —, ou semer un jeu de démonstration en base. Le jeu riche des maquettes (13
organisations, 40 propositions, 30 séances) n'existe que dans `frontend/app/mocks/`.

---

## Complément du 24/08 — deux blocages levés, et une règle métier posée dans le modèle

### Le back-office refusait un administrateur global

`canAdminister` testait `events.length > 0` — la **liste des événements** — au lieu du **périmètre**.
Sur une plateforme en service les deux se confondent ; sur une plateforme **neuve**, ils divergent, et
il fallait une édition pour entrer dans l'écran où l'on crée la première édition. Le blocage était
circulaire et ne se voyait qu'une fois : à la mise en service.

Le contrat déclarait pourtant les trois cas depuis le premier jour, dans l'en-tête de `useApi.ts` :

    { is_global: true,  event_ids: [] }      administrateur de la plateforme
    { is_global: false, event_ids: [id…] }   administrateur des éditions listées
    { is_global: false, event_ids: [] }      aucun droit → accès refusé

Un `computed` corrigé, **dix écrans débloqués** — tous ceux du back-office s'appuient dessus.

### Le compte d'amorçage ne peut pas se connecter

Le semis crée `admin@epavillonclimatique.francophonie.org` avec le rôle `super_admin`, **sans mot de
passe** — c'est délibéré, et le fichier l'explique : « aucun mot de passe par défaut n'est écrit en
base, c'est une faille classique des amorçages ». Mais il ne pose **aucun jeton d'activation** non
plus. Conséquence : sur un poste neuf, personne ne peut entrer dans le back-office.

Le rôle a donc été accordé à la main au compte du commanditaire, une fois, avec son motif inscrit dans
la colonne `note` de l'attribution. **C'est le seul geste de cette session qui contourne
l'application.** La question reste ouverte : le semis doit-il poser un jeton d'activation, ou la mise
en service passe-t-elle par une commande dédiée ?

### Une adhésion active porte toujours sa fonction

Demande du commanditaire, posée dans le **modèle** (`ck_memberships_job_title`) et non dans les écrans :
trois chemins mènent à une adhésion active, et trois validations auraient divergé.

| Chemin | Fonction |
|--------|----------|
| Demande de rattachement | **exigée** — la personne parle d'elle-même |
| Création d'une organisation | **exigée** — le créateur devient référent actif d'emblée |
| Invitation par un référent | facultative — l'adhésion reste en attente |
| **Acceptation d'une invitation** | **exigée** — c'est là que l'invitée la déclare |

L'écran d'acceptation acceptait **automatiquement au chargement** ; il demande désormais la fonction
avant. Elle n'y est pas pré-remplie même si le référent en a proposé une : le jeton est à usage unique,
et le lire pour afficher son contenu le consommerait.

### Ce qui a été vérifié

- Chargement du schéma **de zéro sur une base jetable** : 17 schémas, 159 tables, contrainte comprise.
  La base de développement, elle, a reçu la contrainte par `ALTER TABLE` — un rechargement aurait
  détruit le seul compte capable de se connecter.
- **100 tests du module Organisations au vert**, `cargo fmt` et `clippy -D warnings` sans avertissement.
- Au navigateur : `/admin` et `/admin/evenements` s'ouvrent, avec « Nouvelle édition » et un état vide
  exact — « Aucune édition ne relève de votre périmètre d'administration ».
- L'API rend `is_global: true` **sur la même session**, sans reconnexion, dès le rôle accordé.

### Ce qui reste

La base ne porte **aucune donnée métier** : 0 édition, 0 séance, 0 proposition. Le parcours de mise en
service — créer la COP31, son calendrier, son appel à propositions — n'a **jamais été éprouvé bout en
bout**. C'est le prochain pas, et c'est lui qui dira si la chaîne tient.
