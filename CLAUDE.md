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

Trois familles de fichiers grossissent avec le projet et deviendraient vite intenables à charger en contexte. Elles sont **découpées par domaine**, jamais monolithiques :

| Famille | Emplacement | Règle |
|---------|-------------|-------|
| Traductions | `i18n/locales/<locale>/<domaine>.json` | Le **premier segment de la clé est le nom du fichier** : `proposal.form.speakers.title` vit dans `proposal.json`, et nulle part ailleurs |
| Types | `types/<schéma>.ts` | Un fichier par schéma PostgreSQL — `org.ts`, `event.ts`, `programme.ts`… `index.ts` ne fait que ré-exporter |
| Données simulées | `mocks/<domaine>.ts` | Même découpage que les types ; les identifiants partagés vivent dans `mocks/ids.ts`, déclarés une seule fois |

Conséquence directe sur la façon de travailler : **n'ouvre que le fichier du domaine concerné**, plus `common.json` pour les traductions génériques. Ne charge jamais l'ensemble des traductions, des types ou des mocks pour modifier un écran.

Quand un fichier de traduction dépasse environ 200 lignes, le scinder en sous-dossier (`proposal/form.json`, `proposal/review.json`) et adapter les clés en conséquence.

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
cargo run -p api                                       # API
```

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
