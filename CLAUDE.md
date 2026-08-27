# CLAUDE.md — ePavillon v2

Plateforme numérique de l'IFDD (Institut de la Francophonie pour le développement durable, organe de l'OIF). Elle sert les COP climat, biodiversité et désertification, ainsi que des webinaires, des formations et un espace réservé aux négociateurs.

**Pile** : Nuxt 4 (front) · Rust + Actix Web + SQLx (API) · PostgreSQL 17 + pgvector · Garage (S3) · Valkey
**Architecture** : monolithe modulaire — un module = un schéma PostgreSQL = un crate Rust = une frontière de service potentielle.

---

## Avant toute chose, à chaque session

1. Lire **[docs/PROGRESSION.md](docs/PROGRESSION.md)** — où en est le projet, ce qui est fait, ce qui vient. Il ne porte que l'essentiel ; le détail vit dans **[docs/progression/](docs/progression/)**, où l'on n'ouvre que le fichier utile à la tâche du jour.
2. Identifier les fichiers SQL concernés par la tâche dans **[docs/MODELE_INDEX.md](docs/MODELE_INDEX.md)**, et les lire.
3. Travailler.
4. **Mettre à jour la progression avant de terminer** — le journal du jour, le fichier de l'écran travaillé, et la ligne de suivi dans `docs/PROGRESSION.md`. Le mode d'emploi est en bas de ce fichier-là. Une session qui ne le fait pas oblige la suivante à tout redécouvrir.

Ces quatre points remplacent la mémoire entre sessions. Le contexte se perd, le dépôt non.

---

## Règle d'or

> **`docs/database/*.sql` est la source de vérité du modèle de données.**

Aucun nom de table, de colonne, de type ou d'énumération ne se devine ni ne s'invente. On lit le fichier SQL concerné, on en dérive les types TypeScript et les structures Rust. Le SQL porte sa propre documentation : chaque table et chaque colonne non évidente est commentée en français, et les en-têtes de fichier expliquent les décisions de conception et ce qu'elles corrigent de la v1.

Si le modèle paraît insuffisant pour une fonctionnalité, **on modifie le SQL d'abord**, on recharge la base, puis on écrit le code. Jamais l'inverse. Un champ ajouté côté application sans exister en base est une dette immédiate.

---

## Concision

Court, mais clair. La brièveté ne doit jamais coûter une information utile — on coupe le remplissage, pas le sens.

**Dans les réponses**
- Aller au fait : ce qui a été fait, où, et ce qui reste. Pas de préambule, pas de résumé de ce qu'on vient de lire.
- Des mots simples et des phrases courtes. Le terme technique quand il est juste, jamais pour faire savant.
- Ne pas réexpliquer le code qu'on vient d'écrire ligne par ligne : le lecteur l'ouvre s'il veut le détail.
- Pas de liste de ce qu'on aurait pu faire autrement, sauf si la question porte dessus.

**Dans les fichiers**
- **Très peu de commentaires.** Un nom juste vaut mieux qu'un commentaire ; du code lisible n'en demande pas.
- On commente le *pourquoi*, jamais le *quoi* : une décision non évidente, un contournement, un piège. Pas `// incrémente le compteur`.
- Une ligne suffit presque toujours. Un paragraphe de commentaire signale que le code est à revoir, pas à documenter.

Cette règle ne touche pas le SQL de `docs/database/` ni les fichiers de `docs/` : le modèle porte sa documentation, c'est voulu.

---

## Quand tu me poses une question

**Pose-la en mots simples, comme à quelqu'un qui n'a pas lu le code.** C'est une demande explicite du commanditaire, faite le 20/08 après une question posée en jargon qui n'a pas pu recevoir de réponse.

- Dis **ce que la personne verra ou ne verra pas**, pas quelle colonne ou quel type porte la chose. « Un code à six chiffres en plus du mot de passe » se comprend ; « l'issue `mfa_required` du contrat » ne se comprend pas.
- Aucun nom de fichier, de table, de colonne ou de type dans l'énoncé. Ils appartiennent à ta réflexion, pas à la question.
- Une comparaison familière vaut mieux qu'une définition exacte.
- Pour chaque réponse possible, dis **ce que ça change concrètement** : plus de travail, un écran de plus, quelqu'un de bloqué, une fonctionnalité à moitié faite.
- Recommande une option, et dis pourquoi en une phrase.
- Une seule question à la fois quand c'est possible.

Cette règle vaut pour les questions posées au commanditaire. Le reste du dépôt — code, documentation, journaux — garde son vocabulaire technique.

---

## Où trouver quoi

| Question | Fichier |
|----------|---------|
| Quelles tables pour l'écran que je construis ? | [docs/MODELE_INDEX.md](docs/MODELE_INDEX.md) |
| Où en est le projet ? | [docs/PROGRESSION.md](docs/PROGRESSION.md) — l'état général et le suivi des prompts |
| Les écarts et les vérifications de l'écran que je reprends | [docs/progression/ecrans/](docs/progression/ecrans/) — un fichier par prompt |
| Ce qu'a fait la session d'hier, ce qui a été tranché | [docs/progression/journal/](docs/progression/journal/) et [docs/progression/decisions/](docs/progression/decisions/) |
| Le prompt de la page ou du module à construire | [docs/PROMPTS_DEVELOPPEMENT.md](docs/PROMPTS_DEVELOPPEMENT.md) |
| Comment monter la base et les services en local | [docs/ENVIRONNEMENT_LOCAL.md](docs/ENVIRONNEMENT_LOCAL.md) |
| Pourquoi le modèle est fait ainsi ? Quelles décisions ? | [docs/CADRAGE.md](docs/CADRAGE.md) — §2 constat v1, §5 architecture, §6 les 14 ADR |
| Le périmètre du jalon en cours, ce qui attend | [docs/CADRAGE.md](docs/CADRAGE.md) §10 |
| Couleurs, polices, jetons de design | [docs/CHARTE_GRAPHIQUE.md](docs/CHARTE_GRAPHIQUE.md) puis `frontend/app/assets/css/design-tokens.css` |
| Vue d'ensemble du modèle, invariants, conventions SQL | [docs/README.md](docs/README.md) |
| Le guide de style vivant, rendu par les vrais composants | `frontend/app/pages/style-guide.vue` |
| **À quoi l'interface doit ressembler — la référence qui fait autorité** | [docs/guide-de-style-epavillon.html](docs/guide-de-style-epavillon.html) : maquette complète écrite à la main, avec ses **quatorze règles d'usage** et ses décisions de conception. En cas de désaccord avec l'implémentation Vue, **c'est lui qui tranche** — sauf sur les thématiques, voir ci-dessous |
| Ce que demandait le commanditaire, dans ses mots | [docs/historique/](docs/historique/) |

**Ne charge pas tout.** `CADRAGE.md` fait plusieurs centaines de lignes et les fichiers SQL en totalisent plus de quinze mille : lis la section ou le fichier dont tu as besoin, pas l'ensemble.

---

## Les huit règles métier qui reviennent partout

Elles sont détaillées dans le cadrage, mais on les oublie vite et chacune a déjà coûté une erreur :

1. **Une organisation, plusieurs dénominations.** Chercher « IFDD » ou « Institut de la Francophonie pour le développement durable » doit ramener la même fiche. C'est le défaut n°1 de la v1.
2. **Les chevauchements de créneaux ne sont jamais bloqués.** Les organisations proposent librement, l'équipe arbitre par glisser-déposer. On détecte et on affiche ; on ne refuse pas. Seule la publication du programme est conditionnée.
3. **Un seul stand.** Deux activités d'une même édition ne peuvent **matériellement** pas se tenir en même temps : il n'y a qu'un lieu. C'est un fait du terrain, pas une contrainte à coder — le système **signale** un tel chevauchement comme conflit de gravité haute, sans jamais l'empêcher (voir la règle n°2). En revanche deux **événements** distincts peuvent tourner en parallèle.
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
- Aucune page n'importe un mock : tout passe par `composables/useApi.ts`, qui offre quatre primitives —
  `call` (un 404 lève), `callOrNull` (un 404 est une réponse), `send` (**jamais rejouée**) et `pending`
  (l'écran dont l'API n'existe pas encore : il lit des exemples **et le dit**).
- Un message d'erreur venant de l'API s'affiche **tel quel** : son catalogue est déjà français, et en
  écrire un second côté site donnerait deux textes pour un même refus. Le site ne parle que lorsque
  l'API s'est tue — injoignable, délai dépassé —, sous les clés de `i18n/locales/*/_api.json`.
- **Aucune instance de classe dans l'état d'un store** : le payload du rendu serveur ne sérialise que
  des objets simples, et une erreur posée dans un `ref` fait rendre 500 à la page entière — seulement
  en cas de panne d'API, c'est-à-dire au seul moment où ce chemin sert.
- Quatre états obligatoires par écran : chargement (squelettes), vide, erreur, accès refusé.
- Responsive à partir de 375 px ; le corps de page ne défile jamais horizontalement.

### Direction artistique

Institutionnel et sérieux, mais vivant. Ni tableau de bord SaaS générique, ni site d'ONG militant. Références de posture : le site des Nations unies pour la rigueur, une revue scientifique en ligne pour la lisibilité, une billetterie de festival pour l'énergie de la programmation.

**La référence détaillée est [docs/guide-de-style-epavillon.html](docs/guide-de-style-epavillon.html)** — elle porte quatorze règles d'usage numérotées et le dessin de chaque composant. Trois d'entre elles se trahissent facilement et coûtent cher :

- **La couleur d'un état n'est pas celle qu'on croit.** Cyan pour l'information et l'action ; vert pour ce qui est confirmé ; jaune pour ce qui demande attention — donc pour « en cours », qui n'est pas une réussite ; rouge pour l'échec, la suppression et le direct ; **violet pour le report**, déjà arbitré et qui n'attend plus rien ; gris pour ce qui est clos.
- **Toute cible tactile fait 44 px** (`--target-min`). Les 40 px compacts sont réservés aux barres d'outils sur écran large, jamais à l'action principale d'un écran mobile.
- **Trois pastilles thématiques au plus** sur une carte ; les suivantes se replient en « +N ». Au-delà, elles cessent d'informer.

**Une seule divergence assumée avec ce guide** : il fige huit thématiques dans des jetons CSS. Ne pas le suivre là-dessus — les thématiques vivent dans `reference.taxonomy_terms` avec leur couleur, et les figer dans la feuille de style est exactement le défaut n° 1 de la v1.

**Et une exception, arbitrée le 19/08** : le verre dépoli du bandeau d'accueil, décrite ci-dessous et inscrite au guide.

**À faire**
- Hiérarchie typographique forte : les titres portent le sens, pas les icônes.
- Densité informationnelle assumée. Ces gens lisent des documents de négociation ; un tableau bien composé ne les rebute pas.
- Beaucoup de blanc entre les blocs, peu à l'intérieur des blocs.
- La couleur distingue des états et des thématiques ; elle ne décore pas.
- Coins légèrement arrondis (6 à 8 px), ombres très discrètes ou bordures fines.

**À éviter absolument**
- Néons, halos flous.
- Illustrations 3D, blobs, formes organiques flottantes.
- Emoji en guise d'icônes fonctionnelles.
- Les tournures marketing (« Boostez », « Révolutionnez ») et le mot « Bienvenue ».

**Le verre et le dégradé : interdits partout, sauf sur un média.**

Cette ligne interdisait le verre dépoli et les dégradés sans exception. **Le commanditaire a tranché autrement le 19/08** : la page d'accueil reprend le rendu de la plateforme de référence — fond photographique ou vidéo plein cadre, panneaux translucides flottant dessus. L'exception est donc **arbitrée, bornée, et outillée** :

- Elle ne vaut **que sur un média** — une photographie, une vidéo. Un panneau de verre sur une surface de page est un défaut, pas une variante.
- La matière vient de **jetons** : `--color-glass`, `-raised`, `-hover`, `-accent`, `--color-glass-border`, `-border-strong`, `--blur-glass`, `--blur-glass-strong`, `--shadow-glass`. Jamais un `bg-white/20` écrit dans un composant — la v1 avait treize opacités différentes réparties dans huit fichiers, et personne ne savait laquelle était la bonne.
- **Le verre sépare, il ne contraste pas.** Le contraste reste porté par `--color-scrim` sous le média. Sans voile, une photographie claire rend le panneau illisible malgré son flou.
- **Un seul dégradé** existe : `.scrim-fade-bottom`, le fondu qui rattache le rail de vignettes au bas d'une image. Il ne décore rien — il rend lisible un texte blanc sur une image dont on ignore la luminosité.
- Ces surfaces **ne s'inversent pas** en thème sombre, comme les aplats institutionnels : le fond est une photographie dans les deux thèmes.

Le détail est au § « Le verre » de [docs/guide-de-style-epavillon.html](docs/guide-de-style-epavillon.html).

**Le relief moulé : réservé à l'interrupteur, arbitré le 19/08.**

Même forme d'exception, autre matière. L'interrupteur n'est plus une pastille glissant sur un aplat accentué mais un **basculeur mécanique** — piste creusée, curseur bombé portant un voyant, rainures gravées qui s'allument quand le courant passe. Son relief vient de deux jetons, `--color-relief-shade` et `--color-relief-light`, **jamais d'une ombre noire** : une pièce moulée reçoit sa lumière d'un côté et son ombre de l'autre, et un seul noir translucide ne la sculpte pas. Les couleurs y disent l'état, pas la marque : vert pour le voyant allumé (un réglage actif est *confirmé*), cyan pour les rainures, gris sourd éteint.

L'exception s'arrête à cette commande. Partout ailleurs, la structure passe par les bordures et les ombres restent discrètes. Le dessin vit dans `UiSwitch` et nulle part ailleurs : **aucun écran ne dessine sa propre bascule**, et le `ThemeToggle` de la barre de navigation garde le sien, qui n'est pas un interrupteur de réglage.

### Deux niveaux de jetons de design, à ne jamais mélanger

Les couleurs de **marque** gardent le nom de la charte (`--ifdd-cyan`, `--ifdd-vert`…) : elles sont non négociables et doivent rester traçables jusqu'au document officiel de l'IFDD. Elles portent les valeurs et ne sont **jamais** redéfinies, pas même en thème sombre.

Les jetons **sémantiques** portent un nom de rôle (`--color-surface`, `--color-text`, `--color-border`, `--color-success`…) et référencent la marque par `var()`, sans jamais redéclarer une valeur hexadécimale :

```css
:root                    { --ifdd-cyan: #00A1E4; }              /* marque, jamais redéfinie */
:root                    { --color-accent: var(--ifdd-cyan-700); }  /* rôle, thème clair */
:root[data-theme="dark"] { --color-accent: var(--ifdd-cyan-300); }  /* rôle, thème sombre */
```

Un composant appelle `--color-accent`, jamais `--ifdd-cyan`. C'est ce qui permet au thème sombre de redéfinir les rôles sans toucher aux couleurs officielles.

Deux conséquences pratiques : les couleurs de marque ne sont pas des couleurs d'interface — le cyan `#00A1E4` sur fond blanc **ne passe pas le contraste AA en texte**, d'où les nuances dérivées `--ifdd-cyan-50` … `--ifdd-cyan-900` ; et le thème sombre n'est pas un inversement — le vert et le jaune de la charte deviennent agressifs sur fond noir, il faut les désaturer.

### Découpage des fichiers transverses

Traductions, types et données simulées grossissent avec le projet et deviendraient vite impossibles à charger en contexte.

> **Garde-fou : aucun fichier de code applicatif — `frontend/` et `backend/` — ne dépasse 1000 lignes.**
> Ce n'est pas une cible mais une limite haute — au-delà, le fichier devient coûteux à charger et pénible à modifier. L'organisation reste le découpage par écran ou par entité, qui produit naturellement des fichiers bien plus courts.
>
> **`docs/database/` en est exclu.** Un fichier SQL de module est un tout cohérent qui porte sa propre documentation ; quatre d'entre eux dépassent légitimement les mille lignes. Les découper les rendrait moins lisibles, pas plus.

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

Pour modifier un écran, on ouvre **son** fichier de traduction, **ses** types, **ses** mocks. Jamais l'ensemble. Si tu te surprends à charger plus de trois fichiers de traduction pour une seule page, le découpage est à revoir — signale-le dans le journal du jour, `docs/progression/journal/`.

---

## Sous-agents

Tu peux lancer autant de sous-agents que tu le juges utile. Le contexte d'une session est la ressource rare de ce projet : déléguer permet d'explorer large sans le saturer.

**Cas où c'est le bon réflexe :**
- Lire plusieurs fichiers SQL volumineux pour en extraire ce qui concerne la tâche — le sous-agent rend la conclusion, pas les quatorze mille lignes.
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

L'environnement local (Postgres avec le schéma chargé, Valkey, Jaeger, Mailpit, Garage) est décrit dans [docs/ENVIRONNEMENT_LOCAL.md](docs/ENVIRONNEMENT_LOCAL.md). Les fichiers `ops/docker-compose.dev.yml`, `ops/garage.toml`, `Makefile` et `.env.example` existent depuis le 16/08 ; sur une machine neuve, `cp .env.example .env && make up && make garage-init` suffit.

```bash
docker compose -f ops/docker-compose.dev.yml up -d     # services locaux
docker compose -f ops/docker-compose.dev.yml down -v   # + up : base repartie de zéro
make check                                             # avant tout commit important
make openapi                                           # engendre frontend/app/types/api.ts depuis les routes Rust
cd frontend && npm run dev                             # front
cd backend  && cargo run -p api                        # API
cd backend  && cargo run -p worker                     # travaux différés, relais d'outbox
```

**Le contrat d'API est ENGENDRÉ, jamais écrit.** `frontend/app/types/api.ts` vient de `make openapi`,
qui exporte le document OpenAPI par un binaire — sans base, sans serveur — et le convertit. Il porte les
chemins, les verbes, les paramètres et les 65 codes d'erreur stables ; il ne porte **pas** la forme des
corps, que l'API désigne par leur **nom TypeScript** et dont la définition reste dans
`frontend/app/types/`. Ce lien est vérifié par `make check-api-contract`, branché sur `check-front` :
il refuse un appel vers un chemin absent du contrat, une forme annoncée sans définition, et une route
laissée en données d'exemple alors que l'API la sert. **Ne pas modifier `api.ts` à la main** ; il est
exclu du garde-fou des mille lignes.

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
- Noyer une réponse ou un fichier sous les commentaires et les explications.
- Terminer une session sans mettre à jour la progression — journal du jour, fichier de l'écran, ligne de suivi.
- Regrossir `docs/PROGRESSION.md` : il se lit en entier à chaque session, le détail va dans `docs/progression/`.
- Committer sans que `make check` passe.

---

## Périmètre actuel

**Le site est raccordé à l'API depuis le 22/08.** Renseigner `NUXT_PUBLIC_API_BASE` suffit ; vide, tout
tourne sur `frontend/app/mocks/`, qui reste en place pour les tests et le travail hors ligne. Deux
conditions, dont l'oubli produit un refus que rien n'explique : **ouvrir le site sur l'adresse exacte
d'`APP_PUBLIC_URL`** (seule origine autorisée) et **garder le même hôte des deux côtés** — la portée
d'un cookie ignore le port, pas l'hôte.

**Plus aucun écran ne lit de données d'exemple quand l'API est branchée**, depuis le 27/08 : les trois
derniers — les messages d'incident, l'accueil public avec sa vitrine, et le tableau de bord — sont
servis par les crates `content`, `live` et `analytics`. `make check-api-contract` compte **zéro route
en attente**.

**Les jeux d'exemple de `frontend/app/mocks/` restent**, et c'est désormais leur seule raison d'être :
les tests, et le travail hors ligne. Sans `NUXT_PUBLIC_API_BASE`, tout le site fonctionne encore sur
eux. La primitive `pending()` reste elle aussi, pour le prochain écran livré avant son API.

Le jalon en cours ne contient que ce qui permet de **lancer l'appel à propositions de la COP31** : authentification, organisations, événements, appel, soumission, espace organisation, back-office.

Les modules Publications, Négociations, Formations et Outils **existent dans le modèle de données** mais leur interface affiche « En cours de maintenance ». La messagerie directe n'est pas un module de ce rang : ni schéma, ni crate — ses tables vivent dans `engagement`.

L'affichage est commandé par `platform.feature_flags`, et **le routage s'en charge tout seul** : le middleware global `feature-flag` sert `pages/maintenance/[module].vue` dès qu'un drapeau `<module>.enabled` est éteint, d'après le registre `frontend/app/utils/feature-modules.ts`. Aucune page ne teste son propre drapeau — pour fermer un espace, on l'inscrit au registre ; pour l'ouvrir, on bascule le drapeau en base, sans redéploiement.

Les six drapeaux de module sont semés et éteints : `publications.enabled`, `negotiation.enabled`, `training.enabled`, `messaging.enabled`, `tools.enabled`, `directory.enabled`. **Ne pas les confondre avec les drapeaux fins** (`negotiation.channels`, `tools.surveys`, `tools.ai_assistant`), qui commandent une fonctionnalité à l'intérieur d'un module déjà ouvert et ne peuvent pas tenir lieu de drapeau de module. Ne pas développer ces modules sans instruction explicite.
