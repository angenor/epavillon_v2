# ePavillon v2 — Document de cadrage

**Plateforme numérique de l'IFDD / Organisation internationale de la Francophonie**
Refonte technique et fonctionnelle · Version 1.0 du document · Août 2026

---

## 1. Résumé exécutif

La plateforme ePavillon est en production et remplit sa fonction : elle a porté plusieurs COP, des cycles de webinaires et un espace de négociation. Elle a été construite dans des conditions contraintes — compétences en cours d'acquisition, ressources serveur limitées, échéances annuelles non négociables. Ces contraintes ont disparu ; la dette qu'elles ont produite, non.

Cette refonte poursuit trois objectifs, dans cet ordre :

1. **Supprimer les défauts structurels** qui coûtent du temps à chaque édition — au premier rang desquels l'impossibilité de fusionner deux organisations créées en double, mais aussi l'arbitrage manuel des créneaux, les migrations SQL requises pour ajouter une simple question à un formulaire, et l'absence de traçabilité des décisions du comité de sélection.
2. **Poser une architecture modulaire** — un monolithe déployé d'un bloc, mais découpé en modules à frontières explicites, chacun extractible en service autonome le jour où la charge ou l'organisation le justifie. Ni microservices prématurés, ni monolithe indémêlable.
3. **Rendre la plateforme extensible sans redéploiement** : taxonomies, formulaires, quotas, journées thématiques, règles de rappel et grilles d'évaluation deviennent des données administrables, pas du code.

La recommandation centrale de ce document porte sur la méthode : **ne pas réécrire d'un bloc**. La v1 doit rester en production, notamment pour l'échéance de novembre 2026, pendant que la v2 est construite et absorbe les modules un par un.

---

## 2. Constat sur la version 1

Cette section n'est pas un procès. Chaque défaut relevé est daté d'une décision rationnelle dans son contexte ; l'objectif est d'identifier ce qui doit changer et pourquoi, afin de ne pas reproduire les mêmes contournements.

### 2.1 Les doublons d'organisations

C'est le défaut cité en premier par le porteur du projet, et il est représentatif.

La v1 disposait pourtant d'une table `organization_aliases`, d'un drapeau `is_duplicate` et d'une colonne `duplicate_of`. Le mécanisme de recherche avant création existait. Ce qui manquait, ce n'était pas une table : c'était **la chaîne complète**.

- La recherche portait sur le nom complet ; le sigle vivait dans une colonne à part, non interrogée de la même manière. Deux utilisateurs cherchant « IFDD » et « Institut de la Francophonie pour le développement durable » obtenaient des résultats différents.
- Rien n'empêchait, au niveau de la base, l'insertion d'un homonyme.
- Aucune fonction de fusion n'existait. Une fois le doublon créé, il était définitif — et les activités, les membres et l'historique restaient répartis entre deux fiches.

**Leçon retenue** : une règle métier qui n'est appliquée que par l'interface n'est pas appliquée. La v2 place les garanties dans le SGBD (index uniques partiels sur le nom normalisé et sur le domaine vérifié), le rapprochement sur toutes les dénominations à la fois, et une fonction de fusion atomique et tracée.

### 2.2 Une table `activities` qui portait deux réalités

`activities` décrivait à la fois le dossier soumis par une organisation et l'activité diffusée au public. Sa colonne `validation_status` mélangeait des états de dossier (`draft`, `submitted`, `under_review`, `approved`, `rejected`) et des états de diffusion (`live`, `completed`). Une machine à états cohérente était impossible à écrire.

Conséquence directe : lorsqu'un cycle de webinaires a eu besoin de plusieurs éditions d'une même activité, la solution a été d'ajouter une colonne `session_edition INTEGER` **dans la table des inscriptions**, et de la propager dans les index uniques et les procédures stockées. Le besoin était légitime ; c'est le modèle qui ne pouvait pas l'accueillir.

### 2.3 Une table d'inscriptions devenue un fourre-tout

`activity_registrations` a accumulé, au fil des besoins :

- six colonnes `guest_*` et deux contraintes `CHECK` croisées pour gérer les inscrits sans compte ;
- `session_edition` pour les activités à éditions multiples ;
- `referral_source` et `referral_source_other`, avec une liste de valeurs figée dans une contrainte `CHECK` ;
- `fallback_payload`, `fallback_error` et `recovered_at` pour rattraper les échecs techniques d'inscription ;
- une table annexe `paco_demographic_data` pour des questions propres à une campagne.

Chacun de ces ajouts a coûté une migration, un déploiement et une adaptation du code. La v2 traite la cause : les questions posées à l'inscription deviennent des **données** (`programme.registration_form_fields`), les réponses un document JSON indexé, et la dualité utilisateur/invité disparaît avec la séparation personne/compte.

### 2.4 Des vocabulaires figés dans des types ENUM

`activity_theme` comptait quinze valeurs, `activity_categories` sept, `thematique_type` onze. Ajouter une thématique pour une nouvelle COP imposait une migration DDL. Renommer était possible, retirer ne l'était pas, traduire et ordonner non plus — les libellés affichés vivaient dans les fichiers de traduction du frontend, désynchronisés des valeurs en base.

La v2 distingue deux natures :

- les **vocabulaires ouverts** (thèmes, catégories, secteurs, types de document, canaux d'acquisition) deviennent des lignes administrables, traduites, ordonnées, colorées et dépréciables ;
- les **machines à états fermées** (statut d'une proposition, statut d'une session) restent des ENUM : leur liste engage le code métier, et une valeur ajoutée à la légère y produirait un comportement indéfini.

### 2.5 Les visuels stockés en colonnes

La table `events` portait six colonnes d'URL de bannière (`banner_high_quality_32_9_url`, `banner_high_quality_16_9_url`, `banner_high_quality_1_1_url` et leurs équivalents basse qualité), `innovations_practices` quatre, `activities` trois. Ajouter un format imposait une migration ; changer de fournisseur de stockage imposait de réécrire toutes les URL en base ; aucun mécanisme ne permettait de savoir quel fichier était encore utilisé, ni de purger les orphelins — alors même que l'espace disque était la contrainte principale.

### 2.6 Ce qui manquait entièrement

- **Traçabilité** : pas de journal d'audit. Qui a rejeté cette proposition, quand, et sur quel motif ? La réponse n'existait pas en base.
- **Justification des décisions** : une note libre sur 20 (`activity_ratings.rating`), sans critères. Impossible d'expliquer un refus à une organisation qui le conteste.
- **Fiabilité des effets de bord** : une validation d'activité devait déclencher une notification, un courriel et une création de réunion Zoom. Ces effets étaient déclenchés par du code applicatif, sans garantie transactionnelle : un échec réseau laissait le système dans un état incohérent, silencieusement.
- **RGPD** : ni consentements, ni procédure d'export, ni anonymisation. Une plateforme portée par une organisation internationale traitant des données de ressortissants européens et africains ne peut pas rester dans cette situation.
- **Gestion des conflits de créneaux** : entièrement manuelle, à l'œil, dans le calendrier.

### 2.7 Le couplage à Supabase

Supabase a permis d'aller vite : authentification, temps réel, stockage et politiques RLS fournis. En contrepartie, la logique métier s'est dispersée entre les politiques RLS, une vingtaine de fonctions edge en TypeScript/Deno et le code Vue. Les fonctions edge portent aujourd'hui des règles métier de première importance (approbation d'activité, création de réunion Zoom, envoi de courriels) sans tests, sans typage partagé avec le client et sans possibilité de rejeu.

Le passage à un VPS et à une API Rust n'est pas un rejet de Supabase : c'est le choix de **rapatrier la logique métier dans un endroit unique, testable et versionné**.

---

## 3. Objectifs et non-objectifs

### 3.1 Objectifs

| # | Objectif | Critère de vérification |
|---|----------|-------------------------|
| O1 | Un référentiel d'organisations sans doublon, avec fusion possible | Zéro homonyme actif dans un même pays ; fusion réalisable en moins de 2 minutes depuis le back-office |
| O2 | Un workflow de sélection traçable et justifiable | Toute décision porte un auteur, une date, un motif et une grille de notation détaillée |
| O3 | Une programmation dont les conflits sont visibles et arbitrables | Tout chevauchement est détecté et affiché avec sa gravité ; la publication est conditionnée à leur résolution |
| O4 | Une plateforme extensible sans redéploiement | Ajouter une thématique, une question d'inscription, une journée thématique ou une règle de rappel se fait depuis le back-office |
| O5 | Des effets de bord fiables | Aucune notification, aucun courriel, aucune réunion visio perdue ou dupliquée, même en cas de panne réseau |
| O6 | Une architecture modulaire réellement extractible | Extraire un module en service autonome ne demande pas de réécrire son schéma |
| O7 | Conformité RGPD | Consentements horodatés, export et anonymisation opérationnels sous 30 jours |
| O8 | Un référencement solide | Rendu serveur natif, URL stables et lisibles, données structurées |

### 3.2 Non-objectifs (explicitement hors périmètre)

- **Refondre l'identité visuelle.** La charte IFDD est conservée.
- **Livrer tous les modules en même temps.** Voir la feuille de route.
- **Atteindre le zéro-couture avec les systèmes de l'OIF.** Aucune interconnexion n'est demandée à ce stade.
- **Microservices dès la phase 1.** Le monolithe modulaire est la cible de la première année.
- **Infolettres.** Le schéma est posé, la fonctionnalité reste hors périmètre immédiat.
- **Livrer l'interface de tous les modules pour la COP31.** Les tables existent, mais l'interface des modules Publications, Négociations, Formations, Outils et Messagerie affiche « En cours de maintenance » jusqu'à leur ouverture, commandée par un drapeau de fonctionnalité.

---

## 4. Périmètre fonctionnel

### 4.1 Module Événements et programmation — le cœur

C'est le module central, celui qui porte la valeur de la plateforme.

**Le cycle métier**, tel que décrit par le porteur du projet :

1. L'OIF/IFDD crée un événement (par exemple la COP climat de l'année) sur la plateforme.
2. Si l'OIF tient un pavillon, un appel à propositions est ouvert.
3. Les organisations, États, ONG soumettent leurs propositions d'activité.
4. Le comité (rôle révisionniste et au-delà) consulte, note et commente chaque proposition depuis le back-office.
5. Les propositions retenues sont sélectionnées, les conflits de créneaux arbitrés.
6. Les organisations sont notifiées et suivent l'état de leur dossier depuis leur espace.
7. Les activités retenues sont programmées et publiées.

**Ce que la v2 ajoute :**

- une **série** distincte de l'**édition** : la COP29, la COP30 et la COP31 sont trois éditions du même rendez-vous, ce qui rend les comparaisons pluriannuelles immédiates ;
- un **appel à propositions** comme entité de plein droit : une édition en ouvre un au plus, ou aucun lorsque l'IFDD ne tient pas de pavillon et se contente d'envoyer un représentant ;
- une **grille d'évaluation**, avec critères pondérés et critères éliminatoires, et une **évaluation en aveugle** optionnelle qui empêche l'effet d'ancrage entre membres du comité ;
- une **séparation proposition / session programmée**, qui permet à une proposition d'engendrer plusieurs sessions (cycles de webinaires) et à l'IFDD de programmer directement une activité sans passer par un appel ;
- la **co-organisation**, absente de la v1 ;
- une **détection des conflits de créneaux**, signalée sans être bloquante ;
- une **règle « un seul direct à la fois »** ;
- des **journées spéciales composées par l'IFDD**, là où la v1 les codait en dur dans le routeur ;
- l'**historique complet des modifications** d'un dossier et des reports de créneau.

**Un appel à propositions par édition, au plus.** Un événement porte zéro appel (COP sans pavillon, où l'IFDD envoie seulement un représentant) ou un seul. Une version antérieure de ce document en autorisait plusieurs, en imaginant qu'une journée thématique puisse ouvrir sa propre fenêtre de soumission ; l'IFDD a tranché : une seule campagne par édition, les journées thématiques étant composées **après** sélection, à partir du vivier commun. C'est plus simple et conforme à la pratique. La règle est garantie par index unique ; si le besoin d'un second appel apparaissait, il suffirait de retirer cet index — rien d'autre dans le schéma ne présuppose l'unicité.

**Plusieurs organisations peuvent co-organiser une activité.** La v1 ne connaissait qu'une seule organisation par activité. Or il est courant qu'une ONG monte une activité avec un ministère et une agence régionale : les co-organisateurs finissaient mentionnés dans le texte de présentation, donc invisibles des filtres, des statistiques et du décompte d'activités par organisation. La v2 distingue le **porteur principal** — celui qui soumet, répond aux demandes de correction et reçoit la décision — des **co-organisateurs, partenaires et soutiens**, chacun avec son rôle et sa confirmation.

**Trois règles d'exploitation, énoncées par l'IFDD, que le modèle traduit fidèlement :**

*Les conflits de créneaux se signalent, ils ne se bloquent pas.* « Les organisations peuvent proposer des créneaux sans se soucier de la date et de l'heure ; c'est à l'admin de réorganiser dans le back-office, visuellement, par glisser-déposer. » Une version antérieure de ce modèle posait une contrainte d'exclusion qui rendait le chevauchement impossible à écrire. C'était une erreur de conception : elle transformait un outil d'arbitrage en mur. Un planificateur travaille par déplacements successifs, et un état transitoire incohérent — deux blocs superposés le temps de recaler le second — fait partie du travail. Une base qui refuse l'écriture pousse à contourner l'outil. Le modèle retenu recense tous les chevauchements avec leur gravité et les rend visibles en permanence ; le seul contrôle bloquant se déplace au moment qui compte, la **publication** du programme.

*Un seul stand, donc jamais deux activités simultanées dans une même édition.* L'IFDD tient un unique pavillon : deux activités d'un même événement ne peuvent pas se tenir en même temps. En revanche, **deux événements distincts peuvent se dérouler en parallèle** — un webinaire et une COP, par exemple — et leurs activités se chevaucher sans difficulté. Une seule contrainte demeure alors : la **diffusion**. Une seule équipe technique, un seul flux : un seul direct à la fois, tous événements confondus. Le canal de diffusion est modélisé comme une ressource, ce qui rend la règle vérifiable et permettra d'en ouvrir un second sans migration.

*Les journées spéciales.* Pendant une COP, l'IFDD construit des journées thématiques — « Journée finance durable », « Journée jeunesse et climat ». Leur composition est une **décision éditoriale** : c'est l'équipe qui choisit les activités qui en font partie, parmi celles qui ont été retenues. Elles ne sont donc pas modélisées comme des jours du calendrier. Trois raisons, chacune observable dès la première édition : une journée spéciale occupe rarement le jour entier — les activités de l'après-midi ne doivent pas y être aspirées par leur seule date ; un même jour peut porter deux fils ; certains fils débordent d'un jour, comme le « Séminaire de Chypre » que la v1 avait dû coder en dur faute de pouvoir l'exprimer. Le rattachement est explicite, N-N, et trace qui a rattaché quoi — il arrive d'avoir à expliquer à une organisation pourquoi elle ne figure pas dans une journée thématique.

**L'historique des modifications.** Toute modification d'un dossier — un intitulé corrigé, une date proposée changée, un intervenant remplacé — est retraçable champ par champ, avec son auteur et sa date. Les reports de créneau font l'objet d'une lecture dédiée : c'est la modification la plus lourde de conséquences pour les inscrits, et celle qu'il faut pouvoir retracer quand une organisation conteste un horaire. La v1 maintenait une table alimentée à la main par le code applicatif, qui ne couvrait que les activités et seulement les écritures passant par le bon chemin ; en v2 l'historique est un sous-produit du journal d'audit, donc exhaustif par construction.

### 4.2 Module Organisations

Référentiel unique, dénominations multiples (nom légal, sigle, traduction, ancien nom, faute fréquente), domaines de courriel vérifiés, adhésions avec historique et référent identifié, détection continue des doublons, fusion atomique et tracée.

Le rattachement automatique par domaine vérifié — un agent qui s'inscrit avec une adresse `@ifdd.francophonie.org` rejoint l'IFDD sans intervention — supprime une part importante des créations en double à la source.

### 4.3 Module Direct

Réunions visio agnostiques du fournisseur (Zoom aujourd'hui, Teams déjà utilisé, autre demain), synchronisation des inscrits avec suivi explicite des échecs et rattrapage, journal des webhooks entrants rejouable, diffusions en direct et replays, messages d'incident à portée variable (événement, journée, session, organisation) avec fenêtre d'affichage.

### 4.4 Module Publications

Les organisations publient des articles. Le point central est le **quota éditorial** : chaque organisation dispose d'un nombre de publications par période, réglable depuis le back-office, plus un quota de stockage. Ces limites sont appliquées par la base de données, donc infranchissables quel que soit le chemin d'écriture.

### 4.5 Module Négociations

Espace réservé aux négociateurs : espaces thématiques (climat, biodiversité, désertification, et tout nouvel espace créé sans migration), réunions unifiées (sessions officielles, concertations francophones, ateliers préparatoires, formations de terrain), documents d'aide (fichier téléversé ou lien externe, jamais les deux), et canaux d'échange en temps réel créés dynamiquement par thématique ou par promotion.

### 4.6 Module Outils

Sondages et assistants IA. Ce module est conçu dès l'origine pour être **déployé séparément** : aucune clé étrangère sortante vers les modules métier, contexte de rattachement libre, URL autonomes. C'est le candidat n°1 à l'extraction.

### 4.7 Module Formations

Formations en ligne, en présentiel ou hybrides. Lorsqu'elles se tiennent sur Zoom, l'enregistrement de la séance rejoint le chapitre correspondant, avec les présentations, exercices et annexes. Chaque chapitre peut se conclure par un quiz, et la formation par une évaluation finale — l'un comme l'autre facultatifs. La réussite conditionne la délivrance d'une attestation.

Ce module fait partie du produit minimum viable : il n'est pas un complément mais une activité à part entière de l'IFDD.

### 4.8 Back-office

Tableau de bord (inscriptions par jour, entonnoir des propositions, taux de participation réel), liste des propositions avec avancement des revues et classement, liste des utilisateurs, **liste des organisations avec le ratio d'acceptation de leurs activités**, gestion des événements, des rôles, des courriels, des messages d'incident et des diffusions.

**Un back-office, plusieurs périmètres.** Le rôle d'administrateur s'attribue globalement ou **sur un seul événement**. C'est la réponse au cas rencontré en v1 : confier un webinaire à un responsable avait imposé de développer une page d'administration séparée, dans l'urgence et en partie codée en dur, uniquement pour qu'il n'ait pas accès au reste de la plateforme. En v2, c'est une attribution de rôle : même back-office, même code, périmètre restreint à son événement et à ce qui en dépend — propositions, sessions, inscriptions, diffusions, incidents. Seul le super-administrateur échappe à toute restriction de portée.

---

## 5. Architecture cible

### 5.1 Principe directeur : *monolith first, microservice ready*

> Un module = un schéma PostgreSQL = un crate Rust = une frontière de service potentielle.

Concrètement :

- **Un seul déploiement** en phase 1 : un binaire d'API, un binaire de workers, une base de données. L'exploitation reste simple, le débogage aussi, les transactions restent atomiques.
- **Des frontières explicites et vérifiées** : chaque module possède son schéma. Toute clé étrangère traversant deux schémas métier doit porter le préfixe `xmod_fk_`. Une vue (`platform.cross_module_fk_report`) contrôle cette règle et **le contrôle est branché dans la vérification locale** — le `make check` décrit au §5.2, où cette vue doit rester vide de toute ligne non conforme. Une frontière violée arrête donc le travail avant le commit qui l'introduit ; elle n'est pas découverte le jour de l'extraction.
- **Une communication par événements dès le premier jour** : les modules ne s'appellent pas directement pour leurs effets de bord. Ils publient des événements de domaine dans un *outbox* transactionnel ; un relais les distribue. Aujourd'hui à des gestionnaires en processus, demain à un bus, sans toucher au code métier.
- **Une extraction outillée** : `platform.generate_module_decoupling_script('tool')` produit les instructions de découplage d'un module. L'extraction devient une opération planifiée, pas un chantier de fouille.

### 5.2 Pile technique

| Couche | Choix | Justification |
|--------|-------|---------------|
| Frontend public | **Nuxt 4** (rendu serveur + génération statique incrémentale) | Le référencement est un objectif de premier rang. La v1 devait pré-rendre les pages avec Puppeteer au moment du build et redéployer à chaque nouvelle activité : ce contournement disparaît. |
| Back-office | **Nuxt 4** en mode application monopage | Pas d'enjeu de référencement, pas de rendu serveur inutile. Même base de code, même système de composants. |
| API | **Rust + Actix Web** | Choix du porteur, confirmé : performance, sûreté mémoire, absence de surprises à l'exécution. Le coût réel est la vitesse de développement initial — voir §5.6. |
| Accès aux données | **SQLx** (requêtes vérifiées à la compilation) | Une requête invalide ne compile pas. Compte tenu de la richesse du schéma, c'est le filet de sécurité déterminant. Pas d'ORM : le SQL reste lisible et optimisable. |
| Base de données | **PostgreSQL 17+** | Contraintes d'exclusion, partitionnement natif, JSONB indexable, recherche plein texte, `pgvector`. La base porte les invariants métier. |
| Stockage objet | **Garage** (API S3), auto-hébergé | Compatible S3 : la migration vers un fournisseur cloud ne changera qu'un point d'accès, puisque la base ne stocke jamais d'URL absolue. |
| Cache et sessions | **Valkey** (Redis) | Présent dans l'environnement local dès le départ, pour ne pas avoir à l'ajouter dans l'urgence. Il n'est mis à contribution que lorsque la mesure le justifie : aucun cache posé par précaution. |
| File de travaux | **PostgreSQL** (`SELECT … FOR UPDATE SKIP LOCKED`) | Aucun courtier à exploiter tant que la charge ne l'exige pas. La table `platform.jobs` est déjà structurée pour migrer vers NATS. |
| Recherche | **PostgreSQL** (`tsvector` + `pg_trgm`) | Un moteur dédié (Meilisearch) n'est justifié qu'au-delà de quelques centaines de milliers de documents. |
| Observabilité | **OpenTelemetry** → Jaeger en local, Grafana / Loki / Tempo en production | Traces distribuées dès le monolithe : le jour de l'extraction, l'instrumentation est déjà en place. |
| Déploiement | **Docker Compose** sur VPS, puis Kubernetes si nécessaire | Ne pas payer la complexité de Kubernetes pour trois conteneurs. |

**Tout se développe en local avant tout déploiement.** PostgreSQL, Valkey, le collecteur OpenTelemetry, Garage et un serveur de courriel de test (Mailpit, qui capture les envois sans rien expédier) tournent dans un `docker compose` local. Les fichiers SQL sont montés en initialisation : repartir d'une base propre est une commande. Le VPS n'entre en jeu qu'une fois le jalon 1 stabilisé.

**Sur l'intégration continue.** Pour un développeur seul et pressé, une chaîne d'intégration complète est du temps pris sur la livraison. Trois vérifications locales suffisent, réunies dans un `Makefile` : le schéma se charge intégralement dans une base neuve (c'est la plus importante — une migration qui ne passe pas sur une base vierge se découvre au déploiement, au pire moment), le front compile et passe le contrôle de types, le back passe `clippy` et ses tests. Le jour où quelqu'un d'autre rejoint le projet, ce `Makefile` devient un fichier de CI en dix lignes.

### 5.3 Structure du code

```
epavillon_v2/
├── backend/                   # workspace Cargo — tout le Rust vit ici
│   ├── Cargo.toml             # définit le workspace
│   ├── crates/
│   │   ├── kernel/            # types partagés, erreurs, i18n, contexte de
│   │   │                      # requête, traits de dépôt, bus d'événements
│   │   ├── contracts/         # schémas des événements de domaine, versionnés
│   │   ├── modules/
│   │   │   ├── identity/      # ┐
│   │   │   ├── organizations/ # │ un crate par module
│   │   │   ├── events/        # │ chacun expose : domaine, dépôts, service,
│   │   │   ├── programme/     # │ routes HTTP, événements publiés/consommés
│   │   │   ├── live/          # │
│   │   │   ├── publications/  # │ un crate ne dépend JAMAIS d'un autre crate
│   │   │   ├── negotiations/  # │ de module : uniquement de `kernel` et des
│   │   │   ├── engagement/    # │ contrats d'événements
│   │   │   ├── media/         # │
│   │   │   ├── training/      # │
│   │   │   └── tools/         # ┘
│   │   ├── api/               # binaire : Actix Web, middlewares, OpenAPI
│   │   └── worker/            # binaire : relais d'outbox, travaux différés
│   └── migrations/            # migrations SQLx, dérivées de docs/database/
├── frontend/                  # application Nuxt
├── docs/                      # cadrage, modèle de données, prompts, progression
└── ops/                       # Docker Compose, sauvegardes, supervision
```

`backend/` et `frontend/` sont **symétriques** : chacun porte son gestionnaire de dépendances, ses commandes et son cycle de vie. Le workspace Cargo vit dans `backend/`, pas à la racine — autrement le dépôt serait « un projet Rust contenant un frontend », ce qui ne reflète pas la réalité d'une application à deux moitiés.

**La règle qui compte** : un crate de module ne dépend jamais d'un autre crate de module. Quand `programme` a besoin de savoir qu'une organisation est vérifiée, il passe par un trait déclaré dans `kernel` et implémenté par `organizations`. Le jour de l'extraction, on substitue une implémentation qui appelle le service distant — le code métier ne change pas.

### 5.4 Communication entre modules

```
  ┌──────────────────────────────────────────────────────────────┐
  │  Une transaction PostgreSQL                                  │
  │                                                              │
  │  UPDATE programme.proposals SET status = 'accepted' …        │
  │  INSERT INTO platform.outbox_events                          │
  │         ('programme.proposal.accepted', …)   ← même COMMIT   │
  └──────────────────────────────────────────────────────────────┘
                              │
                              │  relais (FOR UPDATE SKIP LOCKED + pg_notify)
                              ▼
      ┌───────────────────────┴───────────────────────┐
      ▼                       ▼                       ▼
  engagement              live                    analytics
  notifie                 crée la réunion         met à jour
  l'organisation          Zoom                    l'entonnoir
```

L'état métier et l'annonce de son changement sont écrits ensemble ou pas du tout. Il devient impossible d'avoir un courriel d'acceptation envoyé pour une proposition rejetée, ou une acceptation sans notification. Les consommateurs sont idempotents (`platform.inbox_events`) : un événement rejoué après un redémarrage ne produit pas deux réunions Zoom.

### 5.5 Ce que la base de données garantit elle-même

Les invariants métier vivent dans la base, pas seulement dans l'application. C'est la réponse directe au constat du §2.1 : une règle appliquée uniquement par l'interface n'est pas appliquée.

Une nuance importante, apprise en corrigeant une première version de ce modèle : **tout ce qui est vrai n'a pas vocation à être bloqué**. Deux activités simultanées sur un stand unique sont matériellement impossibles, mais l'interdire à l'écriture empêcherait l'équipe de composer son programme. La base garantit ce qui doit être garanti à tout moment — unicité, cohérence, autorisation — et *signale* ce qui relève d'un arbitrage humain.

| Invariant | Mécanisme |
|-----------|-----------|
| Pas deux organisations actives homonymes dans un même pays | Index unique partiel sur le nom normalisé |
| Un domaine vérifié appartient à une seule organisation | Index unique partiel |
| Un seul appel à propositions par édition | Index unique partiel |
| Un rôle n'est attribuable que sur une portée qu'il autorise | Trigger de contrôle |
| Un seul porteur principal par proposition et par session | Index unique partiel |
| Jamais deux directs simultanés sur un même canal | Index unique partiel au lancement du direct |
| Une journée spéciale ne contient que des activités de son propre événement | Trigger de contrôle |
| Aucune transition d'état non prévue sur une proposition | Table de transitions + trigger |
| Aucune inscription sans les réponses obligatoires | Trigger de validation du formulaire |
| Aucun dépassement de jauge silencieux | Trigger avec bascule en liste d'attente |
| Aucun dépassement de quota de publication | Trigger avec verrou consultatif par organisation |
| Aucun rappel envoyé deux fois | Index unique sur (destinataire, session, décalage) |
| Aucune note supérieure au maximum d'un critère | Trigger de bornes |

### 5.6 Le point de vigilance sur Rust

Actix Web et Rust sont un bon choix pour ce projet : la plateforme a des pics de charge marqués (ouverture des inscriptions, diffusion en direct pendant une COP) et une exigence de fiabilité forte. La sûreté mémoire et l'absence d'exceptions à l'exécution sont des atouts réels.

Il faut cependant énoncer le coût honnêtement : **écrire un CRUD en Rust prend deux à trois fois plus de temps qu'en TypeScript ou en Python**, surtout au début. Trois parades :

1. **Générer ce qui est répétitif.** Le schéma est riche et régulier ; les types de données, les DTO et les clients TypeScript se génèrent (SQLx pour les structures, `utoipa` pour OpenAPI, `openapi-typescript` pour le client Nuxt). Le code écrit à la main doit être le code métier.
2. **Ne pas réécrire ce qui n'apporte rien.** Les tableaux de bord et écrans de consultation peuvent s'appuyer sur des vues SQL exposées telles quelles, plutôt que sur des services complets.
3. **Accepter une exception assumée.** Si un module se révèle trop coûteux à écrire en Rust — typiquement le module Outils, exploratoire par nature — il peut être développé dans une autre technologie **précisément parce qu'il est conçu pour être extractible**. C'est un bénéfice concret de l'architecture, pas un contournement.

### 5.7 Méthode de développement : le front d'abord, sur données simulées

L'ordre de construction n'est pas laissé à l'appréciation de chaque session. Il est celui que le porteur du projet a éprouvé sur ses développements précédents, et il tient en trois temps.

**D'abord le front, sur des données simulées calquées sur le modèle.** Chaque écran est construit contre des jeux de données écrits à la main qui reprennent exactement les noms de tables, de colonnes et de types de `docs/database/`. L'intérêt n'est pas d'économiser l'API : c'est que tout écart entre ce que le modèle prévoit et ce que l'interface réclame — une donnée qui manque, un libellé qui n'existe nulle part, une jointure qu'aucune clé ne permet — apparaît à l'écran immédiatement, et se corrige tant qu'il ne coûte encore rien. L'ordre de correction ne change pas pour autant : si le modèle se révèle insuffisant, on modifie le SQL d'abord, puis les données simulées, puis l'écran. Jamais l'inverse.

**Ensuite l'API, spécifiée à partir de ce que le front consomme.** Le contrat n'est pas inventé en amont : il se déduit de ce qui est réellement affiché. Il est rédigé module par module avec GitHub Spec Kit — constitution, spécification, clarification, plan, tâches, mise en œuvre — dans l'ordre imposé par les dépendances entre modules.

**Enfin le raccordement, écart par écart.** Le client TypeScript est généré depuis la description OpenAPI de l'API, puis comparé aux types du front. Chaque divergence est tranchée explicitement — c'est soit un défaut du front, soit un défaut de l'API — et aucune n'est masquée par une conversion de circonstance. Les données simulées restent en place : elles servent aux tests et au développement hors ligne.

Le détail — l'ordre des écrans, les prompts eux-mêmes et le découpage des modules d'API — vit dans [`PROMPTS_DEVELOPPEMENT.md`](PROMPTS_DEVELOPPEMENT.md).

---

## 6. Décisions d'architecture

Format court : décision, contexte, conséquence.

### ADR-01 — Un schéma PostgreSQL par module

**Décision.** Quinze schémas : `platform`, `reference`, `identity`, `org`, `event`, `programme`, `live`, `publication`, `negotiation`, `engagement`, `media`, `tool`, `training`, `analytics`, et `legacy` — ce dernier temporaire, zone de transit de la reprise des données de la v1 (§9), qui disparaît une fois la bascule réconciliée.
**Contexte.** L'alternative — une base par module dès le départ — interdirait les transactions inter-modules et imposerait une complexité opérationnelle sans contrepartie à ce stade.
**Conséquence.** Frontières lisibles, extraction préparée, transactions atomiques conservées. Impose une discipline de nommage des FK inter-modules, contrôlée automatiquement.

### ADR-02 — UUID v7 comme clés primaires

**Décision.** Toutes les clés primaires sont des UUID v7, générés par `platform.uuid_v7()`.
**Contexte.** Les UUID v4 de la v1 sont aléatoires : chaque insertion touche une page d'index différente, ce qui fragmente les index et dégrade le cache. Les entiers séquentiels exposent les volumes et compliquent la fusion de données entre environnements.
**Conséquence.** Clés opaques et non devinables, mais quasi séquentielles : localité d'index préservée, tri par clé équivalent à un tri chronologique. Sur PostgreSQL 18, la fonction native `uuidv7()` remplace l'implémentation fournie.

### ADR-03 — Textes multilingues en JSONB, pas en colonnes

**Décision.** Un champ traduisible est un objet `{"fr": "…", "en": "…"}` typé par le domaine `platform.i18n_text`, avec français obligatoire.
**Contexte.** La v1 dupliquait les colonnes (`message_fr`, `message_en`). Ajouter une langue imposait une migration sur toutes les tables concernées.
**Conséquence.** Ajouter une locale ne coûte aucune migration. La recherche plein texte s'appuie explicitement sur la version française (langue pivot) ; l'indexation multilingue sera ajoutée si le besoin apparaît.

### ADR-04 — Taxonomies en données, ENUM pour les états

**Décision.** Les vocabulaires ouverts vivent dans `reference.taxonomy_terms` ; les ENUM sont réservés aux machines à états.
**Contexte.** Voir §2.4.
**Conséquence.** Le back-office administre les thématiques, catégories, secteurs, types de document et canaux d'acquisition. Contrepartie assumée : une jointure supplémentaire, et l'intégrité référentielle sur les codes doit être vérifiée par trigger là où le code est stocké en texte.

### ADR-05 — Séparer la personne du compte

**Décision.** `identity.people` (personne physique) et `identity.accounts` (moyens de connexion) sont deux tables distinctes.
**Contexte.** La v1 confondait les deux, d'où les colonnes `guest_*` dans les inscriptions, la duplication des intervenants et l'impossibilité d'anonymiser sans détruire les statistiques.
**Conséquence.** Un invité, un intervenant saisi par un tiers et un utilisateur inscrit sont la même entité. Le rattachement se fait par courriel canonique. L'anonymisation RGPD préserve les agrégats de participation.

### ADR-06 — Séparer la proposition de la session programmée

**Décision.** `programme.proposals` (dossier) et `programme.sessions` (occurrence publiée) sont deux tables distinctes, liées par une relation 1-N.
**Contexte.** Voir §2.2.
**Conséquence.** Les cycles de webinaires sont modélisés nativement. L'IFDD peut programmer une activité sans appel à propositions. Les inscriptions portent sur la session, ce qui rend `session_edition` inutile. Contrepartie : une jointure de plus dans les écrans qui affichent le dossier et sa programmation.

### ADR-07 — Outbox transactionnel plutôt qu'appels directs

**Décision.** Les effets de bord inter-modules passent par `platform.outbox_events`.
**Contexte.** La v1 déclenchait notifications, courriels et création de réunions Zoom depuis le code applicatif, sans garantie transactionnelle.
**Conséquence.** Fiabilité au moins-une-fois, idempotence côté consommateur, rejeu possible, et migration vers un bus sans refonte. Contrepartie : les effets sont asynchrones — l'interface doit refléter un état « en cours » plutôt que de supposer l'effet immédiat.

### ADR-08 — Références de stockage logiques, jamais d'URL en base

**Décision.** `media.assets` stocke un bucket et une clé d'objet ; l'URL publique est composée à la lecture.
**Contexte.** Voir §2.5.
**Conséquence.** Changer de fournisseur de stockage revient à changer un réglage. Les formats sont des lignes (`media.renditions`), la déduplication par empreinte SHA-256 économise l'espace, et les orphelins sont identifiables donc purgeables.

### ADR-09 — RBAC scopé plutôt que rôles globaux

**Décision.** Une attribution de rôle porte une portée : globale, organisation, événement ou espace de négociation. Le code teste une permission, jamais un nom de rôle.
**Contexte.** La v1 posait huit rôles globaux : être révisionniste valait pour toutes les COP, et un référent d'organisation n'existait pas.
**Conséquence.** Un comité de sélection peut être constitué par édition. Ajouter une permission ne casse rien : elle est automatiquement rattachée au rôle super-administrateur par trigger, ce qui évite d'introduire une fonctionnalité que personne ne peut administrer.

### ADR-10 — L'authentification quitte Supabase

**Décision.** Authentification portée par l'API Rust : Argon2id pour les mots de passe, jetons d'accès courts, jetons de rafraîchissement hachés et révocables, TOTP optionnel, fédération OIDC pour les comptes institutionnels.
**Contexte.** Le couplage à `auth.users` de Supabase contraignait le modèle de données et dispersait la logique.
**Conséquence.** Maîtrise complète du cycle de vie des sessions. Contrepartie : une responsabilité de sécurité qui était déléguée revient à l'équipe — d'où l'exigence d'audit de sécurité avant mise en production (§10).

### ADR-11 — Le module Outils est conçu détaché dès l'origine

**Décision.** Aucune clé étrangère sortante depuis `tool` vers les schémas métier ; rattachement par triplet libre et libellé dénormalisé.
**Contexte.** Le porteur du projet annonce l'intention de déployer les outils sous une adresse distincte.
**Conséquence.** L'extraction ne demandera aucune modification de schéma. Contrepartie assumée : pas d'intégrité référentielle sur le contexte, et un travail de nettoyage à prévoir si un contexte est supprimé.

### ADR-12 — Partitionnement mensuel des tables à forte volumétrie

**Décision.** `platform.audit_log`, `engagement.email_messages`, `negotiation.channel_messages` et `analytics.page_views` sont partitionnées par mois.
**Contexte.** Ce sont les tables dont la croissance est linéaire et sans fin.
**Conséquence.** La purge réglementaire devient un `DROP PARTITION`. Contrepartie : la clé primaire doit inclure la colonne de partitionnement, et les clés étrangères entrantes sont contraintes — d'où l'absence de FK vers ces tables.

### ADR-13 — Les conflits de créneaux sont signalés, pas bloqués

**Décision.** Aucune contrainte d'exclusion sur les créneaux. `programme.detect_conflicts()` recense les chevauchements avec leur gravité ; `programme.publication_readiness()` conditionne la publication.
**Contexte.** Une première version posait une contrainte d'exclusion GiST rendant le chevauchement impossible. L'IFDD a corrigé : les organisations proposent leurs créneaux sans se coordonner, et l'arbitrage se fait visuellement dans le back-office, par glisser-déposer.
**Conséquence.** L'outil de planification reste utilisable — un planificateur travaille par déplacements successifs et passe par des états transitoires incohérents. Contrepartie : la cohérence finale dépend d'un contrôle au moment de la publication, et non d'une garantie permanente. C'est le bon compromis ici : le programme n'a de conséquence qu'une fois publié.

### ADR-14 — Un back-office unique, à périmètre variable

**Décision.** Le rôle d'administrateur est attribuable globalement ou sur un seul événement. Toutes les listes du back-office sont filtrées par `identity.administered_events()`.
**Contexte.** En v1, confier un webinaire à un responsable avait imposé une page d'administration séparée, développée dans l'urgence et en partie codée en dur, pour éviter qu'il n'accède au reste.
**Conséquence.** Un seul code, un seul back-office, plus de page parallèle à maintenir. Contrepartie : le filtrage par périmètre devient un invariant de sécurité à vérifier systématiquement — il doit figurer dans la revue de sécurité préalable à la mise en production.


---

## 7. Principes transverses du modèle de données

- **Horodatage** : tout en `timestamptz`, stocké en UTC, converti à l'affichage. Jamais l'inverse.
- **Suppression** : suppression douce (`deleted_at`) réservée aux entités dont l'historique a une valeur (propositions, articles, organisations). Ailleurs, suppression réelle. Les demandes RGPD passent par `identity.privacy_requests`.
- **Audit** : trigger générique `platform.tg_audit()`, attaché explicitement aux entités dont la traçabilité a une valeur métier ou réglementaire — pas partout, pour ne pas doubler le volume d'écriture sans bénéfice.
- **Contexte de requête** : l'application positionne `app.actor_id` et `app.request_id` en début de transaction. L'auteur d'une modification n'a plus à être passé colonne par colonne.
- **Nommage** : `ck_` contraintes de vérification, `ux_` index uniques, `ix_` index, `ex_` contraintes d'exclusion, `xmod_fk_` clés étrangères inter-modules.
- **Idempotence** : toute fonction d'écriture appelée par un worker porte une clé d'unicité métier.

---

## 8. Sécurité et conformité

### 8.1 Sécurité applicative

- Mots de passe hachés en **Argon2id**, jamais vérifiés en SQL.
- Jetons de rafraîchissement stockés **hachés en SHA-256** : un vol de la base ne donne aucun jeton utilisable.
- Secrets des fournisseurs (mot de passe Zoom, URL de démarrage, secret TOTP) **chiffrés côté application** avant écriture, avec séparation stricte entre données publiques et secrets.
- Limitation de débit sur l'authentification, la recherche d'organisations et les inscriptions.
- Validation des téléversements : type MIME réel, analyse antivirus, mise en quarantaine avant publication.
- Journalisation des webhooks entrants avec vérification de signature.

### 8.2 Conformité RGPD

| Exigence | Mise en œuvre |
|----------|---------------|
| Consentement prouvable | `identity.consents` : historique complet horodaté, avec version de la politique et adresse IP |
| Droit d'accès | `identity.privacy_requests` de type `export`, échéance à 30 jours calculée automatiquement |
| Droit à l'effacement | `identity.anonymize_person()` : identité purgée, agrégats de participation conservés |
| Minimisation | Les champs de formulaire marqués `is_sensitive` sont exclus des exports non anonymisés |
| Durée de conservation | Partitionnement mensuel de l'audit et des courriels ; purge par suppression de partition |
| Registre des traitements | À produire lors de la phase 1, alimenté par le catalogue des types de notification et des finalités de consentement |

### 8.3 Exploitation

- Sauvegardes : sauvegarde continue (`pgBackRest`) avec restauration à un instant donné, plus une copie hors site chiffrée quotidienne. **Une restauration doit être testée chaque trimestre** — une sauvegarde jamais restaurée n'est pas une sauvegarde.
- Supervision : disponibilité, latence par point d'entrée, profondeur de l'outbox et de la file de travaux, taux d'échec des synchronisations visio, taux de rebond des courriels.
- Alertes qui comptent réellement : outbox non consommé depuis plus de cinq minutes, travaux en file morte, échec de sauvegarde, certificat expirant.

---

## 9. Stratégie de reprise des données

Détaillée dans [`database/910_migration_v1.sql`](database/910_migration_v1.sql). Quatre temps :

1. **Chargement brut** de la v1 dans un schéma `legacy`, sans transformation ni perte.
2. **Résolution** : dédoublonnage des organisations et des personnes, **décidé et validé par un humain** avant toute écriture. C'est l'unique occasion de nettoyer la dette de la v1 : une fois les données reprises telles quelles, plus personne ne le fera.
3. **Transformation** vers les schémas v2, avec conservation de la correspondance des identifiants (`legacy.id_map`), ce qui permet aux anciennes URL de continuer à résoudre.
4. **Réconciliation** : contrôles de complétude et d'intégrité comparés aux comptages de la v1. Un écart bloque la bascule.

La reprise se fait à **identifiants nouveaux avec table de correspondance**, et non à identifiants constants : c'est ce qui permet de fusionner deux lignes v1 en une seule ligne v2 — le cas des organisations en double et des invités devenus utilisateurs.

Le point d'attention majeur : la fonction `legacy.migrate_organizations()` **refuse de s'exécuter** tant que tous les groupes de doublons proposés n'ont pas été tranchés. C'est délibéré : une fusion erronée mêlerait les activités de deux organisations distinctes, dommage impossible à défaire proprement une fois les activités reprises.

---

## 10. Feuille de route

### Contrainte de calendrier

Les COP se tiennent en novembre, et **l'appel à propositions de la COP31 doit partir**. C'est cette échéance qui commande l'ordre des jalons : tout ce qui n'y concourt pas attend.

Aucune durée n'est indiquée ci-dessous. Le développement se fait avec Claude Code, dont le rendement mesuré sur ce projet rend toute estimation en semaines-homme trompeuse. Ce qui compte est l'**ordre** et les **critères de sortie**, pas le calendrier.

L'échéance de novembre 2026 reste servie par la v1 : rien ne doit dépendre d'une bascule à cette date.

### Jalon 1 — De quoi lancer l'appel à propositions

C'est le seul jalon dont la date compte.

| Domaine | Contenu |
|---------|---------|
| Socle | Environnement local complet, schéma chargé, kernel, relais d'outbox, file de travaux |
| Identité | Inscription, connexion, vérification d'adresse, mot de passe oublié, RBAC scopé (dont l'administrateur d'un seul événement) |
| Organisations | Recherche avant création, rattachement par domaine, membres, fusion des doublons |
| Événements | Création d'une édition, salles, canal de diffusion, journées spéciales, appel à propositions unique, grille de critères, comité |
| Soumission | Formulaire multi-étapes avec brouillon, co-organisateurs, intervenants, documents |
| Espace organisation | Suivi des dossiers, corrections demandées, historique |
| Back-office | Tableau de bord, liste et fiche d'évaluation, planificateur de créneaux, utilisateurs et rôles, messages d'incident |
| Reste | Publications, Négociations, Formations, Outils, Messagerie : « En cours de maintenance », commandé par drapeau de fonctionnalité |

*Critère de sortie* : le parcours complet est jouable sur des données réelles importées — créer un compte, rejoindre une organisation sans créer de doublon, soumettre une proposition à plusieurs organisations, la noter, la retenir, la programmer, publier le programme, s'y inscrire.

### Jalon 2 — Tenir la COP

Module Direct : réunions Zoom et Teams, synchronisation des inscrits avec rattrapage des échecs, diffusions YouTube, replays, messages d'incident et de débordement. Rappels automatiques avant session. Reprise complète des données de la v1.

*Critère de sortie* : un rendez-vous IFDD — un webinaire, à faible enjeu — organisé intégralement sur la v2, du courriel de rappel au replay.

### Jalon 3 — Formations

Formations en ligne, présentielles et hybrides : chapitres, enregistrements de séances, supports, exercices, quiz de fin de chapitre, évaluation finale, attestation. Module attendu au produit minimum viable, livré dès que le jalon 1 est stabilisé.

### Jalon 4 — Ouverture des modules restants

Publications avec quotas, Négociations avec canaux d'échange, tableaux de bord analytiques, puis Outils et assistants IA. Chacun est déjà présent dans le modèle de données ; leur ouverture consiste à livrer l'interface et à lever le drapeau de fonctionnalité, module par module, sans redéploiement de l'ensemble.

### Ce qui doit être fait avant toute mise en production

- Audit de sécurité portant sur l'authentification et le contrôle d'accès (conséquence de l'ADR-10). À défaut d'un audit externe, au minimum une revue dédiée du filtrage par périmètre : un administrateur d'événement ne doit atteindre aucune donnée d'une autre édition, y compris en forgeant une URL.
- Test de restauration de sauvegarde documenté.
- Test de charge sur le scénario « ouverture des inscriptions ».
- Registre des traitements RGPD complété.

---

## 11. Risques et parades

| Risque | Gravité | Parade |
|--------|---------|--------|
| **Réécriture d'un bloc qui n'aboutit pas** | Critique | Fonctionnement en parallèle, bascule par module, v1 maintenue jusqu'à la validation complète. Aucun débranchement de la v1 avant qu'un événement réel n'ait été servi par la v2. |
| **Sous-estimation du coût de développement en Rust** | Élevée | Génération du code répétitif, vues SQL pour les écrans de consultation, possibilité assumée de développer le module Outils dans une autre technologie. |
| Échéance COP manquée | Critique | Le calendrier réserve explicitement novembre 2026 à la v1. |
| Perte ou corruption de données à la reprise | Élevée | Reprise idempotente et rejouable, table de correspondance, contrôles de réconciliation bloquants, v1 conservée en lecture seule six mois. |
| Régression de référencement à la bascule | Moyenne | Slugs conservés, redirections permanentes depuis toutes les anciennes URL, plan de site soumis avant bascule, surveillance des positions pendant huit semaines. |
| Complexité du schéma sous-estimée par l'équipe | Moyenne | Documentation par `COMMENT ON` directement en base, jeu de données de démonstration, guide de contribution par module. |
| Dépendance à un fournisseur visio unique | Moyenne | Abstraction du fournisseur dès la conception ; Teams et Zoom supportés par le même modèle. |
| Facture d'API IA non bornée | Moyenne | `tool.usage_quotas` : quotas par personne et par jour, appliqués en base. |
| Départ de la personne clé du projet | Élevée | Décisions documentées ici et en base ; conventions vérifiées automatiquement plutôt que transmises oralement. |

---

## 12. Indicateurs de succès

À mesurer six mois après la bascule, comparés à la situation actuelle :

| Indicateur | Situation v1 | Cible v2 |
|------------|--------------|----------|
| Organisations en double dans le référentiel | Non mesuré, non corrigeable | Zéro homonyme actif ; toute fusion réalisable |
| Temps d'arbitrage des créneaux d'une COP | Plusieurs jours, à l'œil dans le calendrier | Tout chevauchement détecté et affiché en continu, classé « bloquant » ou « avertissement » ; jamais refusé à la saisie, et aucun chevauchement bloquant restant au contrôle de publication |
| Délai d'ajout d'une question au formulaire d'inscription | Migration + déploiement | Immédiat, depuis le back-office |
| Traçabilité des décisions du comité | Absente | 100 % des décisions avec auteur, date, motif et grille |
| Notifications ou courriels perdus | Non mesuré | Zéro perte ; taux de rebond suivi |
| Délai de traitement d'une demande RGPD | Aucune procédure | Sous 30 jours, outillé |
| Pages indexables sans pré-rendu au build | Zéro | Toutes |

---

## 13. Documents liés

Le présent document fixe le cap et les décisions ; il ne se lit pas seul. Le reste du dossier se répartit ainsi.

- [`../CLAUDE.md`](../CLAUDE.md) — les conventions de code, les huit règles métier et la liste des interdits ; c'est le fichier chargé au démarrage de chaque session de développement.
- [`../README.md`](../README.md) — la présentation du dépôt et les trois premières choses à faire pour démarrer.
- [`README.md`](README.md) — la vue d'ensemble du modèle de données : ce qu'il corrige de la v1, comment il est organisé, ce qu'il garantit lui-même, et l'ordre d'exécution des fichiers SQL.
- [`MODELE_INDEX.md`](MODELE_INDEX.md) — quels fichiers SQL lire pour l'écran ou la tâche du jour, afin de ne jamais avoir à charger le modèle entier.
- [`PROGRESSION.md`](PROGRESSION.md) — l'état d'avancement, les écarts constatés entre le modèle et l'interface, et les décisions prises en cours de route. Il se lit en arrivant et se met à jour en partant : c'est la mémoire du projet entre deux sessions.
- [`PROMPTS_DEVELOPPEMENT.md`](PROMPTS_DEVELOPPEMENT.md) — les prompts de construction, écran par écran puis module d'API par module d'API, dans l'ordre imposé par les dépendances. C'est la mise en œuvre détaillée de la méthode exposée au §5.7.
- [`ENVIRONNEMENT_LOCAL.md`](ENVIRONNEMENT_LOCAL.md) — les services locaux (PostgreSQL, Valkey, Jaeger, Mailpit, Garage), leur configuration, et les vérifications réunies dans le `Makefile`.
- [`CHARTE_GRAPHIQUE.md`](CHARTE_GRAPHIQUE.md) — les couleurs et polices officielles de l'IFDD, dont sont dérivés les jetons de design du frontend.
- [`database/`](database/) — le modèle de données lui-même, un fichier SQL par module. C'est la source de vérité : aucun nom de table, de colonne ou de type ne se devine ailleurs.
- [`historique/note-intention.md`](historique/note-intention.md) — la note d'intention initiale du porteur du projet : les modules attendus, le cycle métier des COP, l'espace d'administration, la nouvelle pile.
- [`historique/retours-cadrage.md`](historique/retours-cadrage.md) — ses retours sur une première version de ce cadrage. Ce sont eux qui ont retourné cinq décisions structurantes — co-organisation, chevauchements non bloqués, appel unique par édition, administrateur à périmètre limité, direct unique — et qui énoncent sa méthode de travail.

**En cas de contradiction, `historique/` fait référence.** Ces deux fichiers portent la parole du commanditaire dans ses mots. Si le présent document, ou n'importe quel autre du dossier, les contredit, c'est le document qui a tort : il doit être corrigé, pas contourné.

---

## Annexe — Questions ouvertes à trancher

Ces points ne bloquent pas le démarrage mais doivent être arbitrés avant les phases concernées.

1. **Messagerie directe et mise en relation** — présentes en v1 (`messages`, `connections`, `appointments`), leur usage réel reste à vérifier. La v2 les accueille dans le schéma `engagement` — `conversations`, `conversation_participants`, `direct_messages`, `connection_requests` et `blocks` — mais l'effort d'interface est conséquent et ne concourt pas à l'échéance de l'appel à propositions. Les rendez-vous de la v1 ne sont en revanche **pas repris** : aucune table ne leur correspond dans le modèle v2, et les reconduire serait une décision à part entière, à prendre au vu de leur usage constaté.
2. **Libellé « QCD »** — le besoin exprimé mentionne des quiz « QCM/QCD ». Le modèle traite les deux cas connus (choix unique et choix multiples) ; le sens exact de QCD reste à confirmer auprès de l'IFDD.
3. **Statut OIF des pays** — le champ `oif_status` doit être renseigné depuis la liste officielle publiée par l'OIF. Cette liste évolue à chaque Sommet et ne peut pas être devinée ; une source de référence doit être désignée.
4. **Politique de conservation** — durées de conservation à fixer par catégorie de données (journal d'audit, courriels, inscriptions, messages de canaux), en lien avec le registre RGPD.
5. **Portée de l'assistant IA** — l'usage « agent qui crée des réunions » suppose des actions à effet de bord déclenchées par un modèle de langage. Le périmètre d'autorisation et les garde-fous doivent être définis avant toute mise en œuvre.
6. **Hébergement du stockage objet** — Garage sur le même VPS que la base, ou sur une machine dédiée ? La réponse dépend du volume prévisionnel de médias, à estimer à partir de l'existant.
