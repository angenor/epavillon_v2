# CLAUDE.md : ePavillon v2

Plateforme de l'IFDD (Institut de la Francophonie pour le développement durable, organe de l'OIF) : COP climat, biodiversité et désertification, webinaires, formations, espace réservé aux négociateurs.

**Pile** : Nuxt 4 · Rust + Actix Web + SQLx · PostgreSQL 17 + pgvector · Garage (S3) · Valkey.
**Architecture** : monolithe modulaire ; un module = un schéma PostgreSQL = un crate Rust = une frontière de service potentielle.

Le dépôt porte aussi **Guide Négo**, l'application mobile des négociatrices et négociateurs francophones (même base, même API, même compte), documentée dans [docs/AppNego/](docs/AppNego/).

---

## À chaque session

Ces étapes remplacent la mémoire entre sessions : le contexte se perd, le dépôt non.

1. Lire [docs/PROGRESSION.md](docs/PROGRESSION.md) (état, fait, à venir). Il se lit en entier à chaque session, donc reste court : « Dernière mise à jour » garde cinq faits d'une ligne, l'ancien part dans [docs/progression/archive/](docs/progression/archive/), le détail vit dans [docs/progression/](docs/progression/), dont on n'ouvre que le fichier utile.
2. Repérer les fichiers SQL de la tâche dans [docs/MODELE_INDEX.md](docs/MODELE_INDEX.md) et les lire.
3. Travailler.
4. **Avant de terminer, mettre à jour la progression** : journal du jour, fichier de l'écran travaillé, ligne de suivi de `docs/PROGRESSION.md` (mode d'emploi en bas de ce fichier). Sinon la session suivante redécouvre tout.

**Session Guide Négo** : lire le point central [docs/AppNego/progress.md](docs/AppNego/progress.md) au lieu de `docs/PROGRESSION.md`, puis le fichier de l'étape dans [docs/AppNego/progression/](docs/AppNego/progression/LISEZMOI.md) et [docs/AppNego/04-roadmap.md](docs/AppNego/04-roadmap.md). En partant, mettre à jour le point central, le fichier de l'étape et le journal du jour (mode d'emploi : `progression/LISEZMOI.md`). **Rien dans `docs/PROGRESSION.md` ni `docs/progression/`**, sauf une modification de `docs/database/`, consignée dans `docs/progression/modele.md` car le modèle est commun (ADR-017).

---

## Règle d'or : `docs/database/*.sql` est la source de vérité du modèle

Aucun nom de table, colonne, type ou énumération ne se devine : on lit le SQL et on en dérive types TypeScript et structures Rust. Le SQL se documente lui-même : tables et colonnes non évidentes commentées en français, en-têtes expliquant les choix de conception et ce qu'ils corrigent de la v1.

Modèle insuffisant : **SQL d'abord**, rechargement de la base, puis code. Jamais l'inverse : un champ côté application absent de la base est une dette immédiate.

---

## Concision

Court mais clair : on coupe le remplissage, jamais une information utile.

**Jamais de tiret cadratin (`—`)** dans un texte qu'on écrit : réponses, code, interface, documentation, commits. Virgule, deux-points, parenthèses ou point à la place. Demande explicite du commanditaire.

**Réponses** : aller au fait (ce qui est fait, où, ce qui reste) ; ni préambule, ni résumé de ce qu'on vient de lire ; mots simples, phrases courtes, terme technique quand il est juste, jamais pour faire savant ; ne pas réexpliquer ligne par ligne le code écrit ; pas d'alternatives sauf si la question porte dessus.

**Fichiers** : très peu de commentaires, un nom juste vaut mieux. On commente le *pourquoi* (décision non évidente, contournement, piège), jamais le *quoi* (pas `// incrémente le compteur`). Une ligne suffit presque toujours ; un paragraphe signale du code à revoir. Exception : le SQL de `docs/database/` et les fichiers de `docs/` portent leur documentation, c'est voulu.

---

## Poser une question au commanditaire

Demande explicite du 20/08, après une question en jargon restée sans réponse : **parler comme à quelqu'un qui n'a pas lu le code.**

- Dire **ce que la personne verra ou ne verra pas**, pas ce qui le porte : « un code à six chiffres en plus du mot de passe », pas « l'issue `mfa_required` du contrat ».
- Aucun nom de fichier, table, colonne ou type dans l'énoncé : ils restent dans la réflexion.
- Une comparaison familière plutôt qu'une définition exacte.
- Pour chaque option, **ce que ça change concrètement** : plus de travail, un écran de plus, quelqu'un de bloqué, une fonctionnalité à moitié faite.
- Recommander une option, et dire pourquoi en une phrase.
- Une question à la fois si possible.

Le reste du dépôt (code, documentation, journaux) garde son vocabulaire technique.

---

## Où trouver quoi

| Question | Fichier |
|----------|---------|
| Tables de l'écran à construire | [docs/MODELE_INDEX.md](docs/MODELE_INDEX.md) |
| État général, suivi des prompts | [docs/PROGRESSION.md](docs/PROGRESSION.md) |
| Écarts et vérifications d'un écran repris | [docs/progression/ecrans/](docs/progression/ecrans/), un fichier par prompt |
| Session d'hier, arbitrages | [docs/progression/journal/](docs/progression/journal/) · [docs/progression/decisions/](docs/progression/decisions/) |
| Prompt de la page ou du module | [docs/PROMPTS_DEVELOPPEMENT.md](docs/PROMPTS_DEVELOPPEMENT.md) |
| Base et services en local | [docs/ENVIRONNEMENT_LOCAL.md](docs/ENVIRONNEMENT_LOCAL.md) |
| Mise en ligne, serveurs | [docs/DEPLOIEMENT.md](docs/DEPLOIEMENT.md), exécuté par `./deploy.sh` |
| Pourquoi ce modèle, décisions | [docs/CADRAGE.md](docs/CADRAGE.md) : §2 constat v1, §5 architecture, §6 les 14 ADR |
| Périmètre du jalon, ce qui attend | [docs/CADRAGE.md](docs/CADRAGE.md) §10 |
| Couleurs, polices, jetons | [docs/CHARTE_GRAPHIQUE.md](docs/CHARTE_GRAPHIQUE.md) puis `frontend/app/assets/css/design-tokens.css` |
| Vue d'ensemble du modèle, invariants, conventions SQL | [docs/README.md](docs/README.md) |
| Guide de style vivant (vrais composants) | `frontend/app/pages/style-guide.vue` |
| **Apparence de l'interface : la référence qui fait autorité** | [docs/guide-de-style-epavillon.html](docs/guide-de-style-epavillon.html), maquette écrite à la main avec ses **quatorze règles d'usage** et ses décisions. **Il tranche** en cas de désaccord avec le Vue, sauf sur les thématiques (voir Direction artistique) |
| Demandes du commanditaire, dans ses mots | [docs/historique/](docs/historique/) |
| Guide Négo : de quoi il s'agit | [docs/AppNego/00-brief.md](docs/AppNego/00-brief.md) |
| Guide Négo : où on en est | [docs/AppNego/progress.md](docs/AppNego/progress.md), puis le fichier de l'étape dans [docs/AppNego/progression/](docs/AppNego/progression/LISEZMOI.md) |
| Guide Négo : étape à construire et son prompt Spec Kit | [docs/AppNego/04-roadmap.md](docs/AppNego/04-roadmap.md) |
| Guide Négo : métier, existant et manques du modèle | [docs/AppNego/02-domaine.md](docs/AppNego/02-domaine.md) |
| Guide Négo : apparence | [docs/AppNego/05-design.md](docs/AppNego/05-design.md) · [design/ecrans/](docs/AppNego/design/ecrans/) (la maquette, **qui fait foi**) · [design/passation/](docs/AppNego/design/passation/) (jetons, thème, pictogrammes, composants, police) |
| Guide Négo : décisions | [docs/AppNego/adr/](docs/AppNego/adr/) |

**Ne charge pas tout** : `CADRAGE.md` fait plusieurs centaines de lignes, le SQL plus de quinze mille. Lire la section ou le fichier utile.

---

## Règles métier

Détaillées dans le cadrage ; on les oublie vite et chacune a déjà coûté une erreur.

1. **Une organisation, plusieurs dénominations** : « IFDD » et « Institut de la Francophonie pour le développement durable » ramènent la même fiche. Défaut n°1 de la v1.
2. **Les chevauchements de créneaux ne sont jamais bloqués** : les organisations proposent librement, l'équipe arbitre par glisser-déposer. On détecte et on affiche, on ne refuse pas. Seule la publication du programme est conditionnée.
3. **Un seul stand** : deux activités d'une même édition ne peuvent matériellement pas avoir lieu en même temps (un seul lieu). Fait du terrain, pas contrainte à coder : le système **signale** un conflit de gravité haute sans jamais l'empêcher (règle 2). Deux **événements** distincts peuvent tourner en parallèle.
4. **Un seul direct à la fois**, tous événements confondus : une équipe technique, un flux.
5. **Un seul appel à propositions par édition**, zéro s'il n'y a pas de pavillon.
6. **Co-organisation** : un porteur principal, des co-organisateurs, des partenaires, des soutiens.
7. **Les journées spéciales sont composées à la main** par l'IFDD parmi les activités retenues. Ce ne sont pas des jours du calendrier : toutes les activités d'un jour n'en font pas partie.
8. **Un administrateur peut n'avoir accès qu'à un événement** : toute liste du back-office est filtrée par le périmètre d'administration, y compris sur URL forgée.

**Guide Négo** en ajoute trois :
- **Trois agendas, jamais confondus** : Sessions de négociation, Réunions de la Francophonie, Pavillon de la Francophonie. « Programme » seul est banni de l'interface et de l'API.
- **La source officielle fait foi** : une donnée importée porte son origine et son heure de lecture ; un signalement validé se pose par-dessus sans jamais la modifier.
- **Rien de produit par une IA n'est publié sans validation humaine.**

---

## Conventions de code

### Front (Nuxt 4)

- TypeScript strict, aucun `any`.
- TailwindCSS v4 : `bg-cyan/50`, pas `bg-opacity-50` ; `cursor-pointer` explicite sur les boutons.
- Aucune chaîne, couleur, date ni libellé en dur : i18n (`fr` par défaut, `en`) et jetons CSS.
- Toute date affichée porte son fuseau (« 14:30 à 16:00, heure de Belém »).
- Aucune page n'importe un mock : tout passe par `composables/useApi.ts` et ses quatre primitives : `call` (un 404 lève), `callOrNull` (un 404 est une réponse), `send` (**jamais rejouée**), `pending` (API pas encore écrite : lit des exemples **et le dit**).
- Une erreur de l'API s'affiche **telle quelle** : son catalogue est déjà français, un second côté site ferait deux textes pour un refus. Le site ne parle que si l'API se tait (injoignable, délai dépassé), sous les clés de `i18n/locales/*/_api.json`.
- **Aucune instance de classe dans l'état d'un store** : le payload du rendu serveur ne sérialise que des objets simples, et une erreur posée dans un `ref` fait rendre 500 à toute la page, seulement en panne d'API, c'est-à-dire au seul moment où ce chemin sert.
- Quatre états par écran : chargement (squelettes), vide, erreur, accès refusé.
- Responsive dès 375 px ; le corps de page ne défile jamais horizontalement.

### Réutiliser avant d'écrire

Redessiner un composant existant crée deux comportements pour un geste, et la copie ne reçoit jamais les corrections. Cas vécu : les éditions d'événement avaient un téléversement d'image complet (fichier, recadrage au rapport exigé, texte alternatif, dépôt) quand la vitrine montrait un bouton désactivé « bientôt ».

Dans l'ordre :
1. **Chercher le composant** : `frontend/app/components/` est rangé par domaine (`ui/`, `media/`, `admin/`, `proposal/`…), montré en fonctionnement par `frontend/app/pages/style-guide.vue`. Un `grep` sur le geste (« upload », « picker », « dialog ») coûte moins qu'une réimplémentation.
2. **Chercher l'écran qui fait déjà pareil** (poser un fichier, une adresse, une plage de dates) : lire sa page, son composant, sa méthode `useApi()`, et refaire pareil.
3. **Étendre par une prop plutôt que recopier** : une variante se corrige une fois, une copie deux fois, et la seconde est oubliée.
4. **Créer seulement si rien n'approche** : dans `ui/` s'il ne connaît aucun métier, sinon dans le dossier du domaine.

Idem pour les méthodes d'API (`composables/api/`), les utilitaires (`utils/`) et les routes Rust : deux fabriques qui déposent un fichier, c'est un contrat de trop.

### Direction artistique (site et back-office de l'ePavillon)

**Ne vaut pas pour Guide Négo**, qui a sa propre identité : direction « Nuit » ([ADR-023](docs/AppNego/adr/023-direction-nuit.md)) : thème unique sombre, accent vert anis, Sora et Manrope embarquées. Référence, au pixel près : [docs/AppNego/design/nuit/](docs/AppNego/design/nuit/LISEZMOI.md). Son système de design vit dans son dossier, borné à `[data-app="guide-nego"]` : il ne redéfinit aucun jeton du site et n'emprunte aucun de ses composants. Le back-office de Guide Négo s'ajoute à celui de l'ePavillon et en garde l'apparence.

Posture : institutionnel et sérieux, mais vivant ; ni tableau de bord SaaS générique, ni site d'ONG militant. La rigueur du site des Nations unies, la lisibilité d'une revue scientifique en ligne, l'énergie d'une billetterie de festival pour la programmation.

Référence détaillée : [docs/guide-de-style-epavillon.html](docs/guide-de-style-epavillon.html) (quatorze règles d'usage numérotées, dessin de chaque composant). Trois règles souvent trahies :
- **Couleurs d'état** : cyan information et action ; vert confirmé ; jaune attention, donc « en cours », qui n'est pas une réussite ; rouge échec, suppression et direct ; **violet report** (arbitré, n'attend plus rien) ; gris clos.
- **Toute cible tactile fait 44 px** (`--target-min`). Les 40 px compacts : barres d'outils sur grand écran seulement, jamais l'action principale d'un écran mobile.
- **Trois pastilles thématiques au plus** par carte, les suivantes en « +N » : au-delà, elles n'informent plus.

**Seule divergence assumée** : le guide fige huit thématiques en jetons CSS. Ne pas suivre : elles vivent dans `reference.taxonomy_terms` avec leur couleur, les figer est le défaut n°1 de la v1. Les exceptions arbitrées (verre, relief, barre, oiseau) suivent.

**À faire** : hiérarchie typographique forte (les titres portent le sens, pas les icônes) ; densité assumée (ce public lit des documents de négociation, un tableau bien composé ne le rebute pas) ; beaucoup de blanc entre les blocs, peu dedans ; la couleur distingue états et thématiques, elle ne décore pas ; coins de 6 à 8 px, ombres très discrètes ou bordures fines.

**À éviter absolument** : néons, halos flous ; illustrations 3D, blobs, formes organiques flottantes ; emoji comme icônes fonctionnelles ; tournures marketing (« Boostez », « Révolutionnez ») et le mot « Bienvenue ».

#### Verre et dégradé : interdits partout, sauf sur un média (arbitré le 19/08)

L'interdiction était totale ; le commanditaire a voulu pour l'accueil le rendu de la plateforme de référence : photo ou vidéo plein cadre, panneaux translucides dessus. L'exception est bornée et outillée :
- **Seulement sur un média** (photo, vidéo). Du verre sur une surface de page est un défaut, pas une variante.
- Matière par **jetons** : `--color-glass`, `-raised`, `-hover`, `-accent`, `--color-glass-border`, `-border-strong`, `--blur-glass`, `--blur-glass-strong`, `--shadow-glass`. Jamais `bg-white/20` dans un composant : la v1 avait treize opacités dans huit fichiers, sans savoir laquelle était la bonne.
- **Le verre sépare, il ne contraste pas** : le contraste vient de `--color-scrim` sous le média. Sans voile, une photo claire rend le panneau illisible malgré le flou.
- **Trois dégradés** : `.scrim-fade-bottom` rattache le rail de vignettes au bas d'une image et rend lisible un texte blanc sur une image de luminosité inconnue ; `.fade-inverse-start` (16/09) fond une photo dans l'aplat institutionnel portant le texte, section d'appel de l'accueil ; `.scrim-fade-top` (05/10) assombrit vers le haut la photo de l'édition sous le titre d'une activité.
- Ces surfaces **ne s'inversent pas** en thème sombre, comme les aplats institutionnels : le fond reste une photo.

Détail : § « Le verre » du guide.

#### L'affiche : abandonnée le 05/10

La direction « affiche de festival » (arbitrée le 30/09 pour la programmation publique) n'a plus d'écran.
- `/programmations` (depuis le 04/10) reprend la « liste éditoriale » des éditions de l'accueil : jetons ordinaires, filets fins, grands chiffres légers en `font-sans`, vignette de couverture, `UiStatusBadge`. Elle s'ouvre sur l'activité en direct s'il y en a une (`EventProgrammeLiveLead`), puis enchaîne les journées ; la semaine est une colonne par jour, séparées d'un filet. Ni thématique, ni format, ni filtre par thématique : la recherche seule.
- La page d'une activité (depuis le 05/10) : bandeau pleine largeur sur la photo de l'édition (voile `bg-scrim/50` et `.scrim-fade-top`), étiquettes d'état et de format, grand titre sur toute la largeur, lieu, date, **logo de l'organisation porteuse en tuile blanche** ; dessous, l'image de l'activité et le billet remontent de 72 px sur le bandeau. En direct, le lecteur remplace l'image et le titre ne bouge pas.
- `--color-poster-*`, `--shadow-poster*`, Archivo et IBM Plex Mono (`assets/css/affiche.css`) restent déclarés sans usage : aucun nouvel écran ne les reprend.

#### Relief moulé : réservé à l'interrupteur (19/08)

L'interrupteur n'est plus une pastille glissant sur un aplat accentué mais un **basculeur mécanique** : piste creusée, curseur bombé portant un voyant, rainures gravées qui s'allument quand le courant passe. Relief par `--color-relief-shade` et `--color-relief-light`, **jamais une ombre noire** : une pièce moulée a sa lumière d'un côté et son ombre de l'autre, un noir translucide ne la sculpte pas. Les couleurs disent l'état, pas la marque : voyant vert allumé (réglage actif = confirmé), rainures cyan, gris sourd éteint.

Ailleurs, la structure passe par les bordures et les ombres restent discrètes. Le dessin vit dans `UiSwitch` seul : **aucun écran ne dessine sa bascule** ; le `ThemeToggle` de la barre garde le sien (ce n'est pas un interrupteur de réglage).

#### Barre de navigation : aplat bleu nuit et liseré (04/10)

- Barre du site public en aplat `--color-surface-inverse` dans les deux thèmes, fermée par un liseré de 4 px aux couleurs du logo (`--color-stripe-1` à `-5`). `--nav-height` : 72 px, liseré compris.
- Tout ce qui s'y pose prend sa variante sur fond sombre : `UiLocaleSwitch` et `UiUserMenu` en `tone="inverse"`, `UiButton` en `inverse`, anneau de focus sur `--color-accent-on-inverse`.
- Au-dessus, un bandeau institutionnel (`--topbar-height`, 44 px), même aplat séparé d'un filet : nom de l'IFDD (jamais traduit) et sigle de l'OIF liés à leurs sites, langue, bouton de thème. Il défile, la barre reste collée.
- Sur grand écran, liens centrés dans une capsule : piste `--color-surface-inverse-raised`, entrée courante en pastille `--color-surface-inverse-selected`. Seule entorse arbitrée au « filet, pas d'aplat » du guide.

#### L'oiseau : réservé à l'accueil (03/10)

- L'illustration du commanditaire (`docs/bird-cursor/`) suit le curseur sur `/` et nulle part ailleurs : `useBirdCursor()` la monte avec la page et la retire en la quittant. Souris seulement, jamais sur tactile ni avec « moins d'animations ».
- Il se pose sur les `data-bird-perch` et parle au survol des `data-bird-say`, texte par i18n (`home.bird.*`). Sa bulle prend les jetons ; ses couleurs propres sont celles d'une illustration.
- `public/bird-cursor.js`, la copie servie, a divergé de la livraison : bulle aux jetons et à son propre niveau d'empilement, sourire (2 s de survol ou un clic : œil en arc, bulle contente, phrases de `happyTexts`). Ajouts marqués « ePavillon ».
- L'oiseau passe sous la barre de navigation, la bulle au-dessus.

### Jetons : marque et rôle, jamais mélangés

- **Marque** : nom de la charte (`--ifdd-cyan`, `--ifdd-vert`…), non négociables, traçables jusqu'au document officiel de l'IFDD. Ils portent les valeurs et ne sont **jamais** redéfinis, même en thème sombre.
- **Sémantiques** : nom de rôle (`--color-surface`, `--color-text`, `--color-border`, `--color-success`…), référencent la marque par `var()`, jamais de valeur hexadécimale.

```css
:root                    { --ifdd-cyan: #00A1E4; }                 /* marque, jamais redéfinie */
:root                    { --color-accent: var(--ifdd-cyan-700); } /* rôle, thème clair */
:root[data-theme="dark"] { --color-accent: var(--ifdd-cyan-300); } /* rôle, thème sombre */
```

Un composant appelle `--color-accent`, jamais `--ifdd-cyan` : le thème sombre redéfinit les rôles sans toucher aux couleurs officielles. Deux conséquences : la marque n'est pas une couleur d'interface (`#00A1E4` sur blanc échoue au contraste AA en texte, d'où `--ifdd-cyan-50` … `--ifdd-cyan-900`) ; le sombre n'est pas un inversement (vert et jaune de la charte, agressifs sur noir, sont désaturés).

### Découpage des fichiers transverses

**Garde-fou : aucun fichier de code de `frontend/` ou `backend/` au-delà de 1000 lignes.** Limite haute, pas cible : au-delà, coûteux à charger et pénible à modifier ; le découpage par écran ou par entité donne naturellement bien moins. Exclus : `docs/database/` (un fichier SQL de module est un tout qui porte sa documentation ; quatre dépassent légitimement mille lignes, les découper les rendrait moins lisibles) et `api.ts` (engendré).

**L'unité de découpage est l'écran, pas le domaine.** Sans cela, traductions, types et mocks grossissent jusqu'à ne plus tenir en contexte. Jamais de fichier unique pour l'un d'eux.

**Traductions** : l'arborescence miroite celle des pages ; le nom de fichier est le chemin de la page aplati par des points.

```
i18n/locales/fr/
├── _common.json     actions, états, formats (partagé)
├── _validation.json messages de validation
├── _nav.json        navigation, pied de page
├── pages/           auth.login.json · auth.register.json · organization.search.json
│                    proposal.form.step-organizations.json · proposal.form.step-speakers.json
│                    admin.proposal.list.json · admin.proposal.review.json · …
└── components/      session-card.json · …
```

**La clé racine est le nom du fichier**, sans `_` ni dossier : `proposal.form.step-speakers.title` vit dans `pages/proposal.form.step-speakers.json` et nulle part ailleurs. Chaque locale a un point d'entrée (`i18n/locales/fr.ts`) qui agrège l'arborescence par `import.meta.glob` : ajouter un fichier ne touche pas `nuxt.config.ts`.

**Types** : un fichier par groupe d'entités, pas par schéma (`programme` compte une vingtaine de tables).

```
types/
├── index.ts     ré-exporte seulement
├── shared.ts    I18nText, alias d'identifiants
├── reference.ts · identity.ts · org.ts
├── event/       series.ts · edition.ts · call.ts · venue.ts
├── programme/   proposal.ts · review.ts · session.ts · registration.ts
└── views.ts
```

**Mocks** : même découpage, plus fin si le volume l'impose (40 propositions écrites à la main ne tiennent pas dans un fichier).

```
mocks/
├── index.ts     ré-exporte seulement
├── ids.ts       identifiants partagés, déclarés UNE SEULE FOIS
├── org.ts · people.ts · event.ts · calls.ts
├── proposals/   drafts.ts · submitted.ts · reviewed.ts · accepted.ts
└── sessions.ts · registrations.ts
```

Pour un écran, on ouvre **ses** traductions, types et mocks, jamais l'ensemble. Plus de trois fichiers de traduction pour une page : découpage à revoir, à signaler dans le journal du jour (`docs/progression/journal/`).

### Textes multilingues : deux sortes à ne pas confondre

Piège de la v1 : libellés de thématiques figés dans le front, désynchronisés de la base.
- **Interface** → fichiers i18n : boutons, titres de sections, messages d'erreur, aides contextuelles. Ils appartiennent au code.
- **Données métier** → colonnes `platform.i18n_text`, résolues à l'affichage par l'utilitaire prévu avec repli sur le français ; jamais `.fr` en direct, jamais recopiées en i18n. Sont concernés : thématiques, catégories, secteurs, types d'organisation, types de document, canaux d'acquisition (tous dans `reference.taxonomy_terms`), titres d'activités, noms de journées spéciales, libellés de salles, intitulés de critères d'évaluation, messages d'incident.

Règle qui tranche : *modifiable par un administrateur depuis le back-office = donnée, pas traduction.*

### Back (Rust)

- Un crate de module ne dépend **jamais** d'un autre crate de module : seulement de `kernel` et des contrats d'événements.
- SQLx vérifié à la compilation. Pas d'ORM.
- Autorisation testée par **permission** (`identity.has_permission`), toujours avec sa portée, jamais par nom de rôle.
- Toute écriture positionne `app.actor_id` et `app.request_id` en début de transaction : c'est ce qui alimente l'audit et l'historique.
- Effets de bord inter-modules par `platform.emit_event()` dans la même transaction, jamais d'appel direct entre modules.
- Ne pas réimplémenter un invariant porté par la base : traduire l'erreur PostgreSQL en message français exploitable.

### SQL

- Clés primaires : `id uuid PRIMARY KEY DEFAULT platform.uuid_v7()`.
- Nommage : `ck_` vérification · `ux_` unique · `ix_` index · `ex_` exclusion · `xmod_fk_` clé étrangère inter-schémas métier (**obligatoire**, vérifié automatiquement).
- Vocabulaires ouverts dans `reference.taxonomy_terms`, jamais un ENUM ; les ENUM sont réservés aux machines à états.
- Commentaires et `COMMENT ON` en français.
- Piège : `timestamptz + interval` est STABLE, donc interdit dans une colonne `GENERATED` ; utiliser un `DEFAULT` ou un trigger.

---

## Sous-agents

**Déléguer dès que la tâche s'y prête, en parallèle.** Le contexte est la ressource rare : déléguer explore large sans le saturer, et deux explorations lancées ensemble coûtent la plus longue, pas la somme (un sous-agent lit le back pendant qu'on lit le front).

- **Oui** : extraire de gros fichiers SQL ce qui concerne la tâche (il rend la conclusion, pas quatorze mille lignes) ; écrans indépendants qui ne partagent que les composants d'interface ; relecture ou audit large (cohérence de la documentation, conformité du modèle, revue de code) ; hypothèse coûteuse (schéma dans une base jetable, recherche dans tout le dépôt) pendant que le travail continue.
- **Non** : travail séquentiel ; écriture demandant une continuité de style ou de vocabulaire (deux agents, deux tons) ; modifications concurrentes des mêmes fichiers (sinon, un périmètre exclusif chacun).
- **En déléguant** : fournir le contexte (il ne lit pas ce fichier), un périmètre de fichiers explicite, le format de réponse. Relire son retour plutôt que le reprendre tel quel : la cohérence d'ensemble nous revient.

---

## Commandes

Environnement local (Postgres avec le schéma, Valkey, Jaeger, Mailpit, Garage) : [docs/ENVIRONNEMENT_LOCAL.md](docs/ENVIRONNEMENT_LOCAL.md). `ops/docker-compose.dev.yml`, `ops/garage.toml`, `Makefile` et `.env.example` existent depuis le 16/08 ; machine neuve : `cp .env.example .env && make up && make garage-init`.

```bash
docker compose -f ops/docker-compose.dev.yml up -d     # services locaux
docker compose -f ops/docker-compose.dev.yml down -v   # + up : base repartie de zéro
make check-safe                                        # avant tout commit, ne détruit rien
make check                                             # DÉTRUIT la base locale : jamais sans accord explicite
make openapi                                           # engendre frontend/app/types/api.ts depuis les routes Rust
cd frontend && npm run dev                             # front
cd backend  && cargo run -p api                        # API
cd backend  && cargo run -p worker                     # travaux différés, relais d'outbox
```

- **Pas de commit sans `make check-safe` vert.**
- **`make check` et `make check-db` effacent la base locale**, qui n'a pas de sauvegarde (ils commencent par `down -v`) : jamais sans l'accord explicite du commanditaire. Le 16/09, un `make check` lancé avant un commit l'a détruite.
- **Le schéma n'est chargé qu'au premier démarrage du conteneur** : après toute modification de `docs/database/`, `down -v`, sinon la base garde l'ancien schéma sans le dire.
- `backend/` et `frontend/` sont symétriques, chacun avec son gestionnaire de dépendances et ses commandes ; le workspace Cargo vit dans `backend/`, pas à la racine.
- Mailpit `http://localhost:8025` (courriels capturés) · Jaeger `http://localhost:16686` (traces).

**Le contrat d'API est engendré, jamais écrit.** `frontend/app/types/api.ts` vient de `make openapi`, qui exporte le document OpenAPI par un binaire (sans base ni serveur) puis le convertit. Il porte chemins, verbes, paramètres et les 65 codes d'erreur stables, **pas** la forme des corps : l'API les désigne par leur **nom TypeScript**, défini dans `frontend/app/types/`. `make check-api-contract`, branché sur `check-front`, refuse un appel vers un chemin absent du contrat, une forme annoncée sans définition, une route laissée en données d'exemple alors que l'API la sert. **Ne jamais modifier `api.ts` à la main.**

### Clés d'API

**OpenRouter** porte tous les appels d'IA de Guide Négo : Gemini pour rédiger, voyage-4 pour vectoriser ([ADR-005](docs/AppNego/adr/005-openrouter-et-embedding-versionne.md)). Clé : `OPENROUTER_API_KEY` dans `.env` ; la production aura la sienne dans `.env.prod`. Aucun code ne la lit encore : le service qui en aura besoin la lira depuis l'environnement, comme un secret, jamais journalisée, jamais renvoyée dans une erreur, jamais exposée au front (aucun préfixe `NUXT_PUBLIC_`).

**Une clé ou un mot de passe ne s'écrit que dans `.env` ou `.env.prod`** : ni code, ni test, document, journal ou message de commit. `.env` est ignoré par Git ; `.env.example` porte le nom de la variable, vide.

---

## Périmètre actuel

- **Site raccordé à l'API depuis le 22/08** : renseigner `NUXT_PUBLIC_API_BASE` suffit. Deux conditions dont l'oubli donne un refus inexpliqué : ouvrir le site sur l'adresse exacte d'`APP_PUBLIC_URL` (seule origine autorisée) et garder le même hôte des deux côtés (la portée d'un cookie ignore le port, pas l'hôte).
- **Depuis le 27/08, aucun écran ne lit de données d'exemple quand l'API est branchée** : les trois derniers (messages d'incident, accueil public et sa vitrine, tableau de bord) sont servis par les crates `content`, `live` et `analytics`. `make check-api-contract` compte zéro route en attente.
- `frontend/app/mocks/` reste, désormais pour les seuls tests et travail hors ligne : sans `NUXT_PUBLIC_API_BASE`, tout le site tourne dessus. `pending()` reste aussi, pour le prochain écran livré avant son API.
- **Jalon en cours** : seulement ce qui permet de lancer l'appel à propositions de la COP31 (authentification, organisations, événements, appel, soumission, espace organisation, back-office).
- Publications, Négociations, Formations et Outils **existent dans le modèle** mais affichent « En cours de maintenance ». La messagerie directe n'est pas un module de ce rang : ni schéma ni crate, ses tables vivent dans `engagement`.
- **Le routage gère l'affichage** commandé par `platform.feature_flags` : le middleware global `feature-flag` sert `pages/maintenance/[module].vue` dès qu'un drapeau `<module>.enabled` est éteint, d'après le registre `frontend/app/utils/feature-modules.ts`. Aucune page ne teste son drapeau : pour fermer un espace, on l'inscrit au registre ; pour l'ouvrir, on bascule le drapeau en base, sans redéploiement.
- Six drapeaux de module semés et éteints : `publications.enabled`, `negotiation.enabled`, `training.enabled`, `messaging.enabled`, `tools.enabled`, `directory.enabled`. Ne pas les confondre avec les drapeaux fins (`negotiation.channels`, `tools.surveys`, `tools.ai_assistant`), qui commandent une fonctionnalité dans un module ouvert et ne remplacent pas un drapeau de module.
- Ne pas développer ces modules sans instruction explicite. **Elle existe pour Guide Négo** : Négociations, puis Formations et Outils, dans l'ordre de [docs/AppNego/04-roadmap.md](docs/AppNego/04-roadmap.md), avec Spec Kit.
