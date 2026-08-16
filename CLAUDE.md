# CLAUDE.md — ePavillon v2

Plateforme numérique de l'IFDD (Institut de la Francophonie pour le développement durable, organe de l'OIF). Elle sert les COP climat, biodiversité et désertification, ainsi que des webinaires, des formations et un espace réservé aux négociateurs.

**Pile** : Nuxt 4 (front) · Rust + Actix Web + SQLx (API) · PostgreSQL 17 + pgvector · Garage (S3) · Valkey
**Architecture** : monolithe modulaire — un module = un schéma PostgreSQL = un crate Rust = une frontière de service potentielle.

---

## Avant toute chose, à chaque session

1. Lire **[docs/PROGRESSION.md](docs/PROGRESSION.md)** — où en est le projet, ce qui est fait, ce qui vient.
2. Identifier les fichiers SQL concernés par la tâche dans **[docs/MODELE_INDEX.md](docs/MODELE_INDEX.md)**, et les lire.
3. Travailler.
4. **Mettre à jour `docs/PROGRESSION.md` avant de terminer.** Une session qui ne le fait pas oblige la suivante à tout redécouvrir.

Ces quatre points remplacent la mémoire entre sessions. Le contexte se perd, le dépôt non.

---

## Règle d'or

> **`docs/database/*.sql` est la source de vérité du modèle de données.**

Aucun nom de table, de colonne, de type ou d'énumération ne se devine ni ne s'invente. On lit le fichier SQL concerné, on en dérive les types TypeScript et les structures Rust. Le SQL porte sa propre documentation : chaque table et chaque colonne non évidente est commentée en français, et les en-têtes de fichier expliquent les décisions de conception et ce qu'elles corrigent de la v1.

Si le modèle paraît insuffisant pour une fonctionnalité, **on modifie le SQL d'abord**, on recharge la base, puis on écrit le code. Jamais l'inverse. Un champ ajouté côté application sans exister en base est une dette immédiate.

---

## Où trouver quoi

| Question | Fichier |
|----------|---------|
| Quelles tables pour l'écran que je construis ? | [docs/MODELE_INDEX.md](docs/MODELE_INDEX.md) |
| Où en est le projet ? | [docs/PROGRESSION.md](docs/PROGRESSION.md) |
| Le prompt de la page ou du module à construire | [docs/PROMPTS_DEVELOPPEMENT.md](docs/PROMPTS_DEVELOPPEMENT.md) |
| Comment monter la base et les services en local | [docs/ENVIRONNEMENT_LOCAL.md](docs/ENVIRONNEMENT_LOCAL.md) |
| Pourquoi le modèle est fait ainsi ? Quelles décisions ? | [docs/CADRAGE.md](docs/CADRAGE.md) — §2 constat v1, §5 architecture, §6 les 14 ADR |
| Le périmètre du jalon en cours, ce qui attend | [docs/CADRAGE.md](docs/CADRAGE.md) §10 |
| Couleurs, polices, jetons de design | [docs/CHARTE_GRAPHIQUE.md](docs/CHARTE_GRAPHIQUE.md) puis `frontend/app/assets/css/design-tokens.css` |
| Vue d'ensemble du modèle, invariants, conventions SQL | [docs/README.md](docs/README.md) |
| Le guide de style vivant | `frontend/app/pages/style-guide.vue` |
| Ce que demandait le commanditaire, dans ses mots | [docs/historique/](docs/historique/) |

**Ne charge pas tout.** `CADRAGE.md` fait plusieurs centaines de lignes et les 18 fichiers SQL en font plus de onze mille : lis la section ou le fichier dont tu as besoin, pas l'ensemble.

---

## Les huit règles métier qui reviennent partout

Elles sont détaillées dans le cadrage, mais on les oublie vite et chacune a déjà coûté une erreur :

1. **Une organisation, plusieurs dénominations.** Chercher « IFDD » ou « Institut de la Francophonie pour le développement durable » doit ramener la même fiche. C'est le défaut n°1 de la v1.
2. **Les chevauchements de créneaux ne sont jamais bloqués.** Les organisations proposent librement, l'équipe arbitre par glisser-déposer. On détecte et on affiche ; on ne refuse pas. Seule la publication du programme est conditionnée.
3. **Un seul stand.** Deux activités d'une même édition ne peuvent pas se tenir en même temps. En revanche deux **événements** distincts peuvent tourner en parallèle.
4. **Un seul direct à la fois**, tous événements confondus : une seule équipe technique, un seul flux.
5. **Un seul appel à propositions par édition** — zéro s'il n'y a pas de pavillon.
6. **Plusieurs organisations peuvent co-organiser** une activité : un porteur principal, des co-organisateurs, des partenaires, des soutiens.
7. **Les journées spéciales sont composées à la main** par l'IFDD, parmi les activités retenues. Ce ne sont pas des jours du calendrier : toutes les activités d'un jour n'en font pas partie.
8. **Un administrateur peut n'avoir accès qu'à un seul événement.** Toute liste du back-office est filtrée par le périmètre d'administration — y compris quand l'utilisateur forge une URL.

---

## Conventions de code

### Front (Nuxt 4)

- TypeScript strict, aucun `any`.
- TailwindCSS v4 : `bg-cyan/50` et non `bg-opacity-50` ; les boutons ont besoin de `cursor-pointer` explicite.
- Aucune chaîne en dur dans un template : tout passe par i18n (`fr` par défaut, `en`).
- Aucune couleur en dur : tout passe par les jetons CSS.
- Toute date affichée porte son fuseau (« 14:30 — 16:00, heure de Belém »).
- Aucune page n'importe un mock : tout passe par `composables/useApi.ts`.
- Quatre états obligatoires par écran : chargement (squelettes), vide, erreur, accès refusé.
- Responsive à partir de 375 px ; le corps de page ne défile jamais horizontalement.

### Découpage des fichiers transverses

Traductions, types et données simulées grossissent avec le projet et deviendraient vite impossibles à charger en contexte.

> **Garde-fou : aucun fichier du dépôt ne dépasse 1000 lignes.**
> Ce n'est pas une cible mais une limite haute — au-delà, le fichier devient coûteux à charger et pénible à modifier. L'organisation reste le découpage par écran ou par entité, qui produit naturellement des fichiers bien plus courts.

**L'unité de découpage est l'ÉCRAN, pas le domaine.**

#### Traductions

L'arborescence miroite celle des pages. Le nom de fichier est le chemin de la page, aplati par des points :

```
i18n/locales/fr/
├── _common.json                          actions, états, formats — partagé
├── _validation.json                      messages de validation
├── _nav.json                             navigation, pied de page
├── pages/
│   ├── auth.login.json
│   ├── auth.register.json
│   ├── organization.search.json
│   ├── proposal.form.step-organizations.json
│   ├── proposal.form.step-speakers.json
│   ├── admin.proposal.list.json
│   ├── admin.proposal.review.json
│   └── …
└── components/
    ├── session-card.json
    └── …
```

**La clé racine est le nom du fichier**, sans le préfixe `_` ni le dossier : `proposal.form.step-speakers.title` vit dans `pages/proposal.form.step-speakers.json`, et nulle part ailleurs. On sait ouvrir un seul fichier, sans chercher.

Pour éviter d'énumérer quarante fichiers dans `nuxt.config.ts`, chaque locale a un point d'entrée unique (`i18n/locales/fr.ts`) qui agrège l'arborescence par `import.meta.glob`. Ajouter un fichier ne demande alors aucune modification de configuration.

#### Types

Un fichier par **groupe d'entités**, pas par schéma : `programme` compte une vingtaine de tables, bien trop pour un seul fichier lisible.

```
types/
├── index.ts              ré-exporte seulement
├── shared.ts             I18nText, alias d'identifiants
├── reference.ts · identity.ts · org.ts
├── event/                series.ts · edition.ts · call.ts · venue.ts
├── programme/            proposal.ts · review.ts · session.ts · registration.ts
└── views.ts
```

#### Données simulées

Même découpage, plus fin encore quand le volume l'impose — 40 propositions écrites à la main ne tiennent pas dans un fichier.

```
mocks/
├── index.ts              ré-exporte seulement
├── ids.ts                identifiants partagés, déclarés UNE SEULE FOIS
├── org.ts · people.ts · event.ts · calls.ts
├── proposals/            drafts.ts · submitted.ts · reviewed.ts · accepted.ts
└── sessions.ts · registrations.ts
```

#### Ce que cela change concrètement

Pour modifier un écran, on ouvre **son** fichier de traduction, **ses** types, **ses** mocks. Jamais l'ensemble. Si tu te surprends à charger plus de trois fichiers de traduction pour une seule page, le découpage est à revoir — signale-le dans `docs/PROGRESSION.md`.

---

## Sous-agents

Tu peux lancer autant de sous-agents que tu le juges utile. Le contexte d'une session est la ressource rare de ce projet : déléguer permet d'explorer large sans le saturer.

**Cas où c'est le bon réflexe :**
- Lire plusieurs fichiers SQL volumineux pour en extraire ce qui concerne la tâche — le sous-agent rend la conclusion, pas les onze mille lignes.
- Construire des écrans indépendants qui ne partagent que les composants d'interface.
- Relire ou auditer un large périmètre : cohérence de la documentation, conformité du modèle, revue de code.
- Vérifier une hypothèse coûteuse (charger le schéma dans une base jetable, chercher une occurrence dans tout le dépôt) pendant que le travail principal continue.

**Cas où il vaut mieux s'en passer :**
- Travail séquentiel dont chaque étape dépend de la précédente.
- Écriture qui demande une continuité de style ou de vocabulaire — deux sous-agents produiront deux tons.
- Modifications concurrentes sur les mêmes fichiers : donne à chacun un périmètre exclusif, sinon leurs écritures se marchent dessus.

**Quand tu délègues :** donne au sous-agent le contexte dont il a besoin (il ne lit pas ce fichier automatiquement), un périmètre de fichiers explicite, et le format de réponse attendu. Relis ce qu'il renvoie plutôt que de le reprendre tel quel — c'est toi qui réponds de la cohérence de l'ensemble.

### Deux sortes de textes multilingues — ne pas les confondre

C'est un piège dans lequel la v1 est tombée, avec des libellés de thématiques figés dans les fichiers du frontend et désynchronisés de la base.

**Textes d'interface** → fichiers i18n. Libellés de boutons, titres de sections, messages d'erreur, aides contextuelles. Ils appartiennent au code.

**Données métier multilingues** → colonnes `platform.i18n_text` en base, résolues à l'affichage par l'utilitaire prévu, avec repli sur le français. Jamais `.fr` en direct, jamais recopiées dans un fichier de traduction.

Relèvent de la base, et **jamais** des fichiers i18n : les thématiques, catégories, secteurs, types d'organisation, types de document, canaux d'acquisition (tous dans `reference.taxonomy_terms`), ainsi que les titres d'activités, noms de journées spéciales, libellés de salles, intitulés de critères d'évaluation et messages d'incident.

La règle qui tranche : *si un administrateur peut le modifier depuis le back-office, ce n'est pas une traduction — c'est une donnée.*

### Back (Rust)

- Un crate de module ne dépend **jamais** d'un autre crate de module : uniquement de `kernel` et des contrats d'événements.
- SQLx avec vérification à la compilation. Pas d'ORM.
- L'autorisation se teste par **permission** (`identity.has_permission`), jamais par nom de rôle, et toujours avec sa portée.
- Toute écriture positionne `app.actor_id` et `app.request_id` en début de transaction — c'est ce qui alimente l'audit et l'historique.
- Les effets de bord inter-modules passent par `platform.emit_event()` dans la même transaction. Jamais d'appel direct entre modules.
- Le code ne réimplémente pas un invariant déjà porté par la base : il traduit l'erreur PostgreSQL en message français exploitable.

### SQL

- Clés primaires : `id uuid PRIMARY KEY DEFAULT platform.uuid_v7()`.
- Nommage : `ck_` vérification · `ux_` unique · `ix_` index · `ex_` exclusion · `xmod_fk_` clé étrangère inter-schémas métier (**obligatoire**, vérifié automatiquement).
- Vocabulaires ouverts dans `reference.taxonomy_terms`, jamais un ENUM. Les ENUM sont réservés aux machines à états.
- Commentaires et `COMMENT ON` en français.
- Piège connu : `timestamptz + interval` est STABLE, donc interdit dans une colonne `GENERATED`. Utiliser un `DEFAULT` ou un trigger.

---

## Commandes

L'environnement local (Postgres avec le schéma chargé, Valkey, Jaeger, Mailpit, Garage) est décrit dans [docs/ENVIRONNEMENT_LOCAL.md](docs/ENVIRONNEMENT_LOCAL.md) — les fichiers `ops/docker-compose.dev.yml` et `Makefile` sont à créer au moment d'initialiser le projet.

```bash
docker compose -f ops/docker-compose.dev.yml up -d     # services locaux
docker compose -f ops/docker-compose.dev.yml down -v   # + up : base repartie de zéro
make check                                             # avant tout commit important
cd frontend && npm run dev                             # front
cd backend  && cargo run -p api                        # API
cd backend  && cargo run -p worker                     # travaux différés, relais d'outbox
```

`backend/` et `frontend/` sont symétriques : chacun porte son gestionnaire de dépendances et ses commandes. Le workspace Cargo vit dans `backend/`, pas à la racine.

Interfaces locales : Mailpit `http://localhost:8025` (courriels capturés) · Jaeger `http://localhost:16686` (traces).

**Attention** : le schéma n'est chargé qu'au premier démarrage du conteneur. Après toute modification d'un fichier de `docs/database/`, détruire le volume (`down -v`) — sinon la base garde l'ancien schéma sans le dire.

---

## Ce qu'il ne faut pas faire

- Inventer un nom de champ sans avoir lu le fichier SQL correspondant.
- Ajouter une colonne côté application sans l'ajouter d'abord au modèle.
- Créer un fichier unique de traductions, de types ou de mocks — ils se découpent par domaine.
- Recopier dans un fichier i18n un libellé qui vient de la base (thématique, catégorie, type d'organisation…).
- Bloquer un chevauchement de créneaux.
- Tester un rôle par son nom plutôt qu'une permission.
- Oublier le filtrage par périmètre d'administration sur une liste du back-office.
- Écrire une couleur, une date ou un libellé en dur.
- Terminer une session sans mettre à jour `docs/PROGRESSION.md`.
- Committer sans que `make check` passe.

---

## Périmètre actuel

Le jalon en cours ne contient que ce qui permet de **lancer l'appel à propositions de la COP31** : authentification, organisations, événements, appel, soumission, espace organisation, back-office.

Les modules Publications, Négociations, Formations, Outils et Messagerie **existent dans le modèle de données** mais leur interface affiche « En cours de maintenance », commandée par un drapeau dans `platform.feature_flags`. Ne pas les développer sans instruction explicite.
