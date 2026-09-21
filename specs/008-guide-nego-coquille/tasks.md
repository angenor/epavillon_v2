---

description: "Task list — Guide Négo, coquille et système de design (0a)"
---

# Tasks: Guide Négo — coquille et système de design (0a)

**Input**: Design documents from `/specs/008-guide-nego-coquille/`

**Prerequisites**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: `node --test` sur la logique pure seulement (research R12) ; trois scripts de contrôle dans `check-front` ; le reste se vérifie à la main selon [quickstart.md](quickstart.md). Aucun cadre de test n'est ajouté.

**Organization**: par récit de [spec.md](spec.md) — US1 installer et ouvrir, US2 mode avion, US3 drapeau, US4 thème, US5 page des composants.

## Format: `[ID] [P?] [Story] Description`

- **[P]** : parallélisable — fichier différent, aucune dépendance en cours
- Chemins relatifs à la racine du dépôt. Branche : `008-guide-nego-coquille`.

---

## ⚠️ Six choses à lire avant de commencer

1. **Jamais `make check`, `make check-db` ni `down -v`.** La ligne de semis s'insère à la main (T003). La porte est `make check-safe`.
2. **Tout sélecteur CSS vit sous `[data-app="guide-nego"]`** — sauf `@font-face`, `@keyframes gn-*` et `@media`. Cela vaut pour les blocs `<style>` des composants : chaque règle s'écrit `[data-app="guide-nego"] .gn-… { }`, jamais en `<style scoped>` seul. **Rien ne se téléporte dans `<body>`** : toute téléportation vise `#gn-portail`, dans l'élément racine de la mise en page — hors de lui, ni jetons ni thème. Aucun jeton `--ifdd-*` ni `--color-*`, aucun composant du site, aucune classe Tailwind. T010 le vérifie dès la phase 2 : il doit rester vert à chaque tâche.
3. **Guide Négo s'affiche en français et ne touche pas au cookie `epavillon_locale`** : jamais de `setLocale`. Les fichiers `en` sont fournis, non servis.
4. **Aucune chaîne en dur, aucune couleur en dur.** Un fichier de traduction par écran, clé racine = nom du fichier.
5. **Le service worker n'existe pas en développement** : il est engendré à la construction. Tout ce qui touche au hors-connexion se vérifie sur `npm run build`, avec et sans `NUXT_APP_BASE_URL=/v2/`.
6. **Les mesures viennent de la maquette, pas de l'œil** : `docs/AppNego/design/ecrans/01-systeme.html`, `passation/composants.md`, et les écarts tranchés de `docs/AppNego/05-design.md` (1 à 32) — le code suit ces décisions, pas les pages qui les contredisent.

---

## Phase 1: Setup

- [X] T001 Vérifier R3 avant tout : créer `frontend/app/pages/guide-nego/index.vue` nue (`defineI18nRoute(false)`, un `$t()` d'essai) et `routeRules` `'/guide-nego'` et `'/guide-nego/**'` en `ssr: false` dans `frontend/nuxt.config.ts` ; poser à la main le cookie `epavillon_locale=en`, ouvrir `/guide-nego` : l'affichage doit être en `fr`, `/en/guide-nego` ne doit pas exister, et le cookie doit être inchangé. Si l'affichage suit le cookie, fixer la langue de l'instance en mémoire (`locale.value = 'fr'` après chargement des messages) — sans `setLocale`. Consigner le résultat dans `specs/008-guide-nego-coquille/research.md` § R3
- [X] T002 Ajouter `('guide_nego.enabled', 'Guide Négo : l''application mobile entière.', false, 0)` à l'`INSERT` de `platform.feature_flags` dans `docs/database/900_seed.sql` § 2, et consigner le changement dans `docs/progression/modele.md`
- [X] T003 Insérer la même ligne à la main sur la base locale montée (`ON CONFLICT (key) DO NOTHING`, requête de `specs/008-guide-nego-coquille/quickstart.md`) et vérifier que `GET /platform/feature-flags` la rend éteinte
- [X] T004 [P] Ajouter `guide_nego.enabled` **allumé** à `frontend/app/mocks/feature-flags.ts` ; `negotiation.channels` y reste éteint
- [X] T005 Déclarer dans `frontend/nuxt.config.ts` le dossier `~/components/guide-nego` avec `prefix: 'Gn'` (avant `~/components`), et `imports.dirs: ['composables/guide-nego']` ; créer les dossiers `frontend/app/{components,composables,utils,assets}/guide-nego/`, `frontend/public/guide-nego/`, `frontend/tests/guide-nego/`
- [X] T006 [P] Écrire `docs/AppNego/adr/019-service-worker-engendre-cache-d-abord.md` : module local sur `build:manifest` plutôt que `@vite-pwa/nuxt` ; précache complet, navigation cache d'abord, mise à jour en arrière-plan sans `skipWaiting` ; le drapeau reste lu par l'API ; coût assumé (une correction arrive à la deuxième ouverture) — d'après research R5

---

## Phase 2: Foundational — fondations du système de design et garde des lectures

**⚠️ Bloque tous les récits.**

- [X] T007 [P] Reprendre `docs/AppNego/design/passation/theme.css` dans `frontend/app/assets/guide-nego/theme.css` : bloc `:root` rentré sous `[data-app="guide-nego"]` (écart 30) ; un seul vert d'état sombre `#B5D66A` (écart 3) ; `#2E3F0E` pour le seul squelette sombre (écart 1) ; règles « réduire les animations » gardées
- [X] T008 [P] Reprendre `passation/mesures.css` dans `frontend/app/assets/guide-nego/mesures.css` : `--gn-picto` (taille) renommé `--gn-picto-taille` (écart 31) ; pas de 16 px — onglet de filtre, titre de ligne et segment à 17, onglet de filtre haut de 48 (écarts 6, 15) ; 48 px ligne de liste, 56 px ligne à réglage (écarts 13, 14)
- [X] T009 [P] Copier les dix `.woff2` et `OFL.txt` de `docs/AppNego/design/passation/police/` vers `frontend/app/assets/guide-nego/police/` ; écrire `frontend/app/assets/guide-nego/police.css` : dix `@font-face` (400, 600, 700, italiques 400 et 600 ; latin et latin étendu avec les deux `unicode-range` de `LISEZMOI.md`), `font-display: swap`
- [X] T010 Écrire `frontend/scripts/check-guide-nego.mjs` : (a) aucun import de `~/components/` hors `guide-nego` sous les dossiers `guide-nego` ; (b) aucune déclaration `--ifdd-*` ni `--color-*` dans `assets/guide-nego/` et les `<style>` des composants `Gn*` ; (c) aucun sélecteur hors `[data-app="guide-nego"]` dans `assets/guide-nego/*.css` **ni dans les blocs `<style>` des `.vue` sous les dossiers `guide-nego`**, exceptions `@font-face`, `@keyframes gn-*`, `@media` ; (d) aucun fichier de plus de 1000 lignes sous ces dossiers ; (e) le mot « Programme » seul absent de `frontend/i18n/locales/*/pages/guide-nego.*.json` et `components/gn-*.json` ; (f) aucun `setLocale` ni `epavillon_` sous ces dossiers ; (g) aucun `<Teleport to="body">` sous ces dossiers. Le brancher sur `check-front` dans `Makefile` et comme script `check:guide-nego` de `frontend/package.json`
- [X] T011 Définir dans `frontend/app/assets/guide-nego/theme.css` les huit rôles sombres manquants — `presse`, `focus`, `desactive-fond`, `desactive-texte`, `attention-fond`, `bulle-envoyee` et son texte, `jauge-fond`, `voile` — à partir des `--gn-nuance-sombre-*` (écarts 4, 27, research R10)
- [X] T012 Écrire `frontend/scripts/guide-nego-contrastes.mjs` : lit `theme.css`, résout les `var()`, calcule le contraste des paires déclarées dans le script (texte courant 7:1, secondaire 4,5:1, filet, focus et pictogramme porteur de sens 3:1), en clair et en sombre, échoue sous le seuil ; le brancher sur `check-front` dans `Makefile` ; inscrire le tableau des huit valeurs retenues et de leurs contrastes dans `docs/AppNego/05-design.md`, à la suite des écarts
- [X] T013 [P] Reprendre `passation/pictogrammes.svg` dans `frontend/app/assets/guide-nego/pictogrammes.svg` : 57 symboles `gn-*`, sans le bloc `<metadata>` ni aucun `<title>`
- [X] T014 Créer `frontend/app/components/guide-nego/GnPicto.vue` : props `nom`, `taille` (16, 18, 20, 24, 26, 40), `aria-hidden`, `<use href>` sur le sprite importé ; si `<use>` externe ne rend pas sous `ssr: false` avec préfixe, insérer le sprite une fois dans la mise en page (research R11)
- [X] T015 Écrire `frontend/app/assets/guide-nego/base.css` : sous `[data-app="guide-nego"]`, fond, couleur, police, `tabular-nums`, corps 17 / 1,5, `min-height: 100dvh`, colonne centrée de 360 px au plus large utile, zones sûres (`env(safe-area-inset-*)`), anneau de focus, `@keyframes gn-spin` et `gn-pulse`
- [X] T016 Créer `frontend/app/layouts/guide-nego.vue` : élément racine `data-app="guide-nego"`, import des quatre feuilles, emplacement de page, et un conteneur `#gn-portail` **dans** l'élément racine, cible de toute téléportation ; aucune barre d'onglets ni en-tête ici (les pages les composent) ; `lang="fr"` garanti selon T001
- [X] T017 [P] Écrire l'enveloppe IndexedDB dans `frontend/app/utils/guide-nego/garde.ts` (base `guide-nego`, magasin `lectures` : `cle`, `valeur`, `lu_a`, `empreinte`) — toute opération sous `try/catch`, rend `null` si le stockage est refusé (data-model.md)
- [X] T018 Créer `frontend/app/composables/guide-nego/useGnLecture.ts` : `useGnLecture(cle, lire)` → `{ valeur, luA, source: 'reseau' | 'garde' | 'aucune', etat }`, délai de 5 s, garde en cas de succès, rend la garde en cas d'échec, **rend la garde aussitôt** puis rafraîchit en parallèle ; objets simples seulement (contracts/hors-connexion.md)

**Checkpoint** : `make check-safe` vert ; une page nue sous la mise en page s'affiche en Atkinson, en clair.

---

## Phase 3: User Story 3 — L'IFDD ouvre et ferme l'application (P1)

Placée avant US1 : toutes les pages naissent derrière le drapeau, et l'étape se construit sur un site en ligne.

**Goal** : le drapeau `guide_nego.enabled` ouvre et ferme toutes les adresses de Guide Négo, sans redéploiement ; une panne d'API ne ferme jamais.

**Independent Test** : quickstart § 1, sans session.

- [X] T019 [P] [US3] Écrire `frontend/app/utils/guide-nego/drapeaux.ts` : `resoudreDrapeau(reponse, garde, cle)` selon la table de research R7 — seule une réponse réussie qui dit « éteint » ferme ; absent, échec, délai ou illisible ⇒ la garde ; garde vide ⇒ fermé. Aucun import de Nuxt
- [X] T020 [P] [US3] Écrire `frontend/tests/guide-nego/drapeaux.test.ts` (`node --test`) : les cinq lignes de la table R7, pour `guide_nego.enabled` et `negotiation.channels` ; ajouter le script `test:guide-nego` à `frontend/package.json` et le brancher sur `check-front` dans `Makefile`
- [X] T021 [US3] Créer `frontend/app/composables/guide-nego/useGnDrapeaux.ts` : lit `useApi().platform.featureFlags()` par `useGnLecture('drapeaux', …)`, ne garde que les deux clés, expose `ouverte`, `echangesOuverts`, `pret` ; verdict immédiat d'après la garde, réévalué à l'arrivée de la réponse ; `echangesOuverts` est faux tant que `ouverte` l'est
- [X] T022 [US3] Étendre `ClosedModule` dans `frontend/app/utils/feature-modules.ts` de deux champs facultatifs `closedPath` et `lastKnown`, et inscrire l'entrée `guide-nego` (`flag: 'guide_nego.enabled'`, `routeNames: ['guide-nego']`, `entryPath: '/guide-nego'`, `closedPath: '/guide-nego/fermee'`, `lastKnown: true`) ; les six entrées existantes ne changent pas
- [X] T023 [US3] Adapter `frontend/app/middleware/feature-flag.global.ts` : pour une entrée `lastKnown`, résoudre par `useGnDrapeaux()` au lieu du magasin du site ; mener à `closedPath` sans `localePath` ; ne jamais rediriger la route `closedPath` sur elle-même ; ramener de `closedPath` à `entryPath` quand c'est rouvert. Comportement inchangé pour les autres modules
- [X] T024 [US3] Créer `frontend/app/pages/guide-nego/fermee.vue` (mise en page `guide-nego`, `defineI18nRoute(false)`, `noindex`) : titre, une phrase, au design de Guide Négo ; textes dans `frontend/i18n/locales/{fr,en}/pages/guide-nego.fermee.json`
- [X] T025 [US3] Appliquer le verdict arrivé après coup, **dans les deux sens** : dans `frontend/app/layouts/guide-nego.vue`, observer `ouverte` — réponse « éteint » ⇒ naviguer vers `fermee` (spec US2 scénario 10) ; réponse « allumé » alors que la garde disait fermé ⇒ de `fermee` vers l'accueil, sans attendre une navigation

**Checkpoint** : quickstart § 1, lignes 1 à 6.

---

## Phase 4: User Story 1 — Installer et ouvrir sans compte (P1) 🎯 MVP

**Goal** : écran d'ouverture, barre d'onglets, en-tête avec « Aa », un état vide par onglet, « Profil et réglages » dans Ressources, page d'installation.

**Independent Test** : sur Android, ouvrir l'adresse, installer, lancer depuis l'icône, continuer en visiteuse, toucher chaque onglet.

- [X] T026 [P] [US1] Écrire `frontend/app/utils/guide-nego/onglets.ts` : liste des onglets (clé, route, pictogramme `gn-home` · `gn-nego` · `gn-franco` · `gn-chat` · `gn-res`), « Échanges » en quatrième si ouverts ; et son test `frontend/tests/guide-nego/onglets.test.ts` (quatre, cinq, ordre)
- [X] T027 [P] [US1] Créer `frontend/app/components/guide-nego/GnBarreOnglets.vue` : 68 px, filet 2 px dessus, pictogramme 26 au-dessus du libellé 13, largeur = libellé + 3 px de chaque côté, 48 au moins, reste réparti ; actif = filet 3 px + 700 + `accent` + `aria-current="page"` ; pressé, focus intérieur ; compteur carré 22 px en option ; zone sûre basse ; quand la place manque (police agrandie, moins de 360 px) la barre seule défile horizontalement, sans libellé tronqué ni sur deux lignes, et amène l'onglet actif dans la vue (`scrollIntoView`) — à 320 px, quatre onglets ne défilent pas ; textes dans `frontend/i18n/locales/{fr,en}/components/gn-barre-onglets.json`
- [X] T028 [P] [US1] Créer `frontend/app/components/guide-nego/GnEntete.vue` : titre 28/700, sous-titre 17/600, filet 3 px, bouton « Aa » 48 px bord 2 px (repos, ouvert) menant au lexique, retour 48 px en variante secondaire, emplacement de ligne de connexion au-dessus du titre ; **ni notifications ni avatar** ; textes dans `components/gn-entete.json`
- [X] T029 [P] [US1] Créer `frontend/app/components/guide-nego/GnEtatVide.vue` : pictogramme 24, titre 20/700, texte, sortie en lien facultative
- [X] T030 [P] [US1] Créer `frontend/app/components/guide-nego/GnBouton.vue` : principal, secondaire, discret, dangereux ; repos, pressé, focus, désactivé, actif ; 48 px, rayon 4, pleine largeur ou demi ; rend un lien ou un bouton
- [X] T031 [P] [US1] Créer `frontend/app/components/guide-nego/GnEnteteGroupe.vue` (capitales 15/700, compteur facultatif, filet 3 px) et `GnLigneReglage.vue` (pictogramme 24, libellé 17/600, valeur 15, chevron ou emplacement ; 56 px ; filet 1 px sauf dernière ; pressée)
- [X] T032 [US1] Créer `frontend/app/components/guide-nego/GnEcran.vue` : compose en-tête, contenu défilant, barre d'onglets (prop `onglets`), d'après `.gn-ecran` et `.gn-ecran--sans-onglets` de `mesures.css` ; lit `useGnDrapeaux().echangesOuverts`
- [X] T033 [P] [US1] Remplacer la page nue de T001, `frontend/app/pages/guide-nego/index.vue`, et créer les autres pages d'onglet `negociations.vue`, `francophonie.vue`, `echanges.vue` : `definePageMeta({ layout: 'guide-nego' })`, `defineI18nRoute(false)`, `GnEcran` + `GnEtatVide` ; titres entiers « Sessions de négociation », « Réunions de la Francophonie », « Pavillon de la Francophonie » ; `echanges` renvoie à l'accueil si les Échanges sont fermés ; textes dans `pages/guide-nego.accueil.json`, `.negociations.json`, `.francophonie.json`, `.echanges.json`
- [X] T034 [P] [US1] Créer `frontend/app/pages/guide-nego/ressources/index.vue` : état vide **et** la seule ligne « Profil et réglages » (`gn-user`, chevron, sans filet) menant à `ressources/reglages` ; textes dans `pages/guide-nego.ressources.json`
- [X] T035 [P] [US1] Créer `frontend/app/pages/guide-nego/lexique.vue` : écran secondaire (retour), état vide, « Aa » à l'état ouvert ; textes dans `pages/guide-nego.lexique.json`
- [X] T036 [US1] Créer `frontend/app/pages/guide-nego/ouverture.vue` d'après `02-socle.html` « 02 Ouverture » : « Bienvenue », sous-titre, trois groupes et leurs lignes, **seul** « Continuer en visiteur » (pose `gn.ouverture-vue`, mène à l'accueil), lien vers `installer` hors mode installé ; rediriger `guide-nego/` vers `ouverture` tant que la clé est absente ; textes dans `pages/guide-nego.ouverture.json`
- [X] T037 [P] [US1] Créer `frontend/app/composables/guide-nego/useGnInstallation.ts` : capte `beforeinstallprompt`, expose `proposable`, `installee` (`display-mode: standalone`, `navigator.standalone`), `installer()`
- [X] T038 [US1] Créer `frontend/app/pages/guide-nego/installer.vue` d'après « 01 Installation » : adresse réelle lue de `location`, étapes Android numérotées, étapes iPhone, « Installer Guide Négo » (déclenche la proposition, sinon défile jusqu'aux étapes et y pose le focus), « Continuer dans le navigateur » ; une phrase dit d'ouvrir l'application une fois avec réseau avant de compter sur elle en salle ; renvoie à l'accueil en mode installé, s'affiche toujours dans un navigateur ; textes dans `pages/guide-nego.installer.json`
- [X] T039 [P] [US1] Écrire `frontend/scripts/guide-nego-icones.mjs` (`sharp` en dépendance de développement de `frontend/package.json`) : depuis `frontend/public/logos/svg/epavillon-symbole-inverse.svg`, fond `#233400`, produire `frontend/public/guide-nego/icones/192.png`, `512.png`, `192-masque.png`, `512-masque.png` (symbole dans les 80 % centraux), `180.png` ; vérifier le rendu de la lettre « e » et, s'il dépend de la police, travailler sur une copie vectorisée sous `frontend/public/guide-nego/icones/` sans toucher au fichier du site ; commiter les PNG
- [X] T040 [US1] Écrire `frontend/public/guide-nego/manifest.webmanifest` (contracts/hors-connexion.md : `./` partout, icônes relatives) et poser dans `frontend/app/layouts/guide-nego.vue`, par `useHead` et `assetUrl()`, `rel="manifest"`, `apple-touch-icon`, `theme-color`, `apple-mobile-web-app-capable`, le titre « Guide Négo »

**Checkpoint** : à 360 px, quatre puis cinq onglets (drapeau `negotiation.channels` à 100), libellés entiers ; chaque onglet se recharge sur lui-même.

---

## Phase 5: User Story 2 — L'application s'ouvre en mode avion (P1)

**Goal** : ouverture sans réseau et sous réseau lent sur la version gardée ; bandeau une fois, rappel dans l'en-tête, « Synchronisé à ».

**Independent Test** : quickstart § 2, mode avion puis « réseau lent + nouvelle construction ».

- [X] T041 [P] [US2] Écrire `frontend/app/utils/guide-nego/connexion.ts` — transitions en ligne ⇄ hors connexion, `bandeauVu` remis à faux à chaque épisode, libellé à afficher selon `luA` connu ou non ; et `formaterLecture(luA, maintenant)` : « à 14:05 » le jour même, « hier à 23:10 », puis « le 11 nov. à 23:10 », heure du téléphone sans fuseau (écart 32) — et son test, qui couvre minuit, la veille, l'avant-veille et le changement d'année, `frontend/tests/guide-nego/connexion.test.ts`
- [X] T042 [US2] Créer `frontend/app/composables/guide-nego/useGnConnexion.ts` : `enLigne` (événements `online`/`offline`, corrigé par le résultat réel des lectures de `useGnLecture`), `luA` (la plus récente lecture gardée), `bandeauVu` ; état partagé par `useState`, objets simples
- [X] T043 [P] [US2] Créer `frontend/app/components/guide-nego/GnBandeauConnexion.vue` : pleine largeur, `attention-aplat`, texte noir 15/700, `gn-wifi-off` 18, 48 px au moins, `role="status"` ; « Hors connexion — lu à HH:MM. Tout ce qui est ici se lit sans réseau. », sans heure si inconnue ; textes dans `components/gn-connexion.json`
- [X] T044 [US2] Brancher la connexion dans `GnEntete.vue` et `GnEcran.vue` : ligne « Synchronisé à HH:MM » (15/600 `succes`, `gn-sync` 16) ou « Hors connexion — lu à HH:MM » (15/600 gris) ; bandeau sous l'en-tête une fois par épisode ; heure rendue par `formaterLecture`, sans fuseau
- [X] T045 [US2] Écrire le modèle `frontend/guide-nego/sw.modele.js` (contracts/hors-connexion.md) : `VERSION` et `LISTE` à inscrire ; `install` garde toute la liste dans `gn-<VERSION>` ou échoue ; pas de `skipWaiting` ; `activate` efface les autres `gn-*` et prend les pages ; navigation dans la portée ⇒ page vide du cache, **cache d'abord** ; adresse de la liste ⇒ cache d'abord ; API, autre origine, méthode autre que `GET` ⇒ laissés passer
- [X] T046 [US2] Écrire le module `frontend/modules/guide-nego-garde.ts` sur le crochet `build:manifest` : fermeture des imports statiques et dynamiques, avec `css` et `assets`, de l'entrée, de `layouts/guide-nego`, de `pages/guide-nego/**` et du paquet de locale `fr` — pas `en`, non servi à cette étape ; ajoute `./`, le manifeste et les icônes ; adresses relatives au service worker ; écrit `guide-nego/sw.js` dans la sortie publique ; inactif en développement ; échoue la construction si la liste est vide
- [X] T047 [US2] Enregistrer le service worker dans `frontend/app/layouts/guide-nego.vue` : hors développement, `register(assetUrl('guide-nego/sw.js'), { scope: assetUrl('guide-nego/') })`, puis `registration.update()` à chaque ouverture ; jamais depuis une page du site
- [X] T048 [P] [US2] Créer `frontend/app/components/guide-nego/GnMessageEphemere.vue` : fond `titre`, texte blanc 17, action `vif-jaune` facultative, 6 s, `role="status"`, téléporté dans `#gn-portail`, durées par `--gn-duree-*`
- [X] T049 [US2] Dire « Prête hors connexion » (FR-012 bis) : dans `frontend/app/layouts/guide-nego.vue`, quand `navigator.serviceWorker.ready` se résout et que `gn.garde-annoncee` est absente, afficher `GnMessageEphemere` et poser la clé ; libellé dans `frontend/i18n/locales/{fr,en}/components/gn-connexion.json`
- [X] T050 [US2] Vérifier sur construction, sans préfixe puis avec `NUXT_APP_BASE_URL=/v2/` : la liste couvre police, sprite, paquet de locale `fr` et toutes les pages ; mode avion après une seule ouverture, « Prête hors connexion » vu, sans avoir visité les onglets ; aucune requête vers une autre origine ; aucun service worker sur les pages du site ; corriger `T045`–`T047` au besoin
- [X] T051 [US2] Dérouler l'essai « Réseau lent + nouvelle construction » de `specs/008-guide-nego-coquille/quickstart.md` : ouverture en moins de deux secondes sur la version gardée, deux caches pendant la mise à jour, installation interrompue sans dégât, drapeau éteint appliqué sans attendre

**Checkpoint** : quickstart § 2, points 1 à 4, 7, et le tableau du réseau lent.

---

## Phase 6: User Story 4 — Thème clair, sombre ou système (P2)

**Goal** : « Profil et réglages » n'ouvre que « Affichage → Thème » ; le choix tient, sans éclair, sans toucher au site.

**Independent Test** : forcer « Sombre », fermer, rouvrir en mode avion ; le site garde son thème.

- [ ] T052 [P] [US4] Écrire `frontend/app/utils/guide-nego/theme.ts` (`clair | sombre | systeme` + préférence du téléphone ⇒ thème affiché ; valeur invalide ⇒ `systeme`) et son test `frontend/tests/guide-nego/theme.test.ts`
- [ ] T053 [US4] Créer `frontend/app/composables/guide-nego/useGnTheme.ts` : clé `localStorage` `gn.theme`, écoute de `matchMedia('(prefers-color-scheme: dark)')`, expose `choix`, `affiche`, `choisir()` ; brancher dans `frontend/app/layouts/guide-nego.vue` : `data-theme="sombre"` sur l'élément racine — jamais sur `<html>` —, `theme-color` selon le thème ; ne lit ni n'écrit `epavillon_theme`
- [ ] T054 [US4] Créer `frontend/app/spa-loading-template.html` : script en ligne qui lit `gn.theme` et la préférence du téléphone et pose le fond (`#101704` ou `#FFFFFF`) avant tout affichage ; vérifier qu'aucune page du site ne l'emploie (aucune autre route en `ssr: false`)
- [ ] T055 [P] [US4] Créer `frontend/app/components/guide-nego/GnSegmente.vue` : choix unique, 48 px, bord 2 px, rayon 4, segments égaux séparés de 2 px, texte 17/600, pictogramme 20 facultatif, actif = `titre` plein et texte blanc ; `role="radiogroup"`, flèches du clavier, `aria-checked`
- [ ] T056 [US4] Créer `frontend/app/pages/guide-nego/ressources/reglages.vue` : écran secondaire, groupe « Affichage », libellé « Thème », `GnSegmente` « Clair » (`gn-sun`) · « Sombre » (`gn-moon`) · « Système » ; rien d'autre ; textes dans `pages/guide-nego.reglages.json`
- [ ] T057 [US4] Passer en sombre tous les écrans livrés (ouverture, installation, fermée, cinq onglets, lexique, réglages) et corriger toute valeur claire restée par oubli dans les composants `frontend/app/components/guide-nego/`

**Checkpoint** : quickstart § 2, points 5 et 8.

---

## Phase 7: User Story 5 — La page des composants (P2)

**Goal** : les fondations en entier et les composants de FR-028, dans leurs états et les deux thèmes, fidèles à `01-systeme.html` à 360 px.

**Independent Test** : quickstart § 3.

- [ ] T058 [P] [US5] Créer `frontend/app/components/guide-nego/GnEtatErreur.vue` (titre 20/700 `danger` + `gn-warn`, ce qui a échoué, ce qui reste vrai, deux sorties) et `GnChargement.vue` (arc 24 trait 2,5 `aria-label` ; squelette h. 16 ; fixes sous « réduire les animations »)
- [ ] T059 [P] [US5] Créer `GnChampRecherche.vue`, `GnChamp.vue`, `GnZoneTexte.vue` dans `frontend/app/components/guide-nego/` : repos, focus, erreur, désactivé ; loupe 24 et croix ; libellé au-dessus ; zone 120 px et compteur « n / 600 »
- [ ] T060 [P] [US5] Créer `GnCase.vue`, `GnInterrupteur.vue` (52 × 32, coche dans le curseur), `GnCercle.vue` dans `frontend/app/components/guide-nego/` : la ligne de 56 px est la cible ; `aria-checked`, focus visible
- [ ] T061 [P] [US5] Créer `GnOngletsFiltre.vue` (17/600, actif 700 + filet 3, 48 px de haut) et `GnPilule.vue` (40 px dans une zone de 48, rayon 24, décochable, variante à chevron 20, `aria-pressed`) dans `frontend/app/components/guide-nego/`
- [ ] T062 [P] [US5] Créer `GnMarqueEtat.vue` (pictogramme 20 + mot 15/700 + couleur du rôle ; les six états de session et les rôles sémantiques ; croix rouge = annulé, triangle rouge = erreur — écart 20), `GnEtiquette.vue` et `GnLigneInformation.vue` dans `frontend/app/components/guide-nego/`
- [ ] T063 [P] [US5] Créer `GnFeuilleBasse.vue` (voile 60 %, filet 3 px, poignée 40 × 4, options de 56 px, « Annuler », piège à focus, Échap, retour du focus) et `GnConfirmation.vue` (bord 2 px, action principale à droite, variante dangereuse) dans `frontend/app/components/guide-nego/`, tous deux téléportés dans `#gn-portail` ; `GnMessageEphemere` existe depuis US2 ; durées et courbes par `--gn-duree-*`, `--gn-courbe-*`
- [ ] T064 [US5] Créer `frontend/app/pages/guide-nego/composants.vue` (`noindex`, aucun lien n'y mène) : bascule clair · sombre · côte à côte, sept sections chargées depuis `frontend/app/components/guide-nego/planche/` ; textes dans `pages/guide-nego.composants.json`
- [ ] T065 [P] [US5] Écrire les sections de fondations `PlancheCouleurs.vue`, `PlancheTypographie.vue` (dont « œ », « Œ », capitales accentuées, colonne d'horaires), `PlancheMesures.vue`, `PlanchePictogrammes.vue` (57, avec leur nom), `PlancheMouvement.vue` dans `frontend/app/components/guide-nego/planche/`
- [ ] T066 [US5] Écrire `PlancheComposants.vue` et `PlancheEtats.vue` dans `frontend/app/components/guide-nego/planche/` : chaque composant de contracts/systeme-de-design.md dans toutes ses variantes et tous ses états — barre à 4 et 5 onglets, avec compteur — ; rien qui ne soit livré ; découper si un fichier approche mille lignes
- [ ] T067 [US5] Comparer la page à `docs/AppNego/design/ecrans/01-systeme.html`, sections 1 à 7, à 360 px en clair puis en sombre, puis la barre à cinq onglets à 390 px (écart 29), à 320 px, et à 360 px avec la police à 130 % — la barre seule défile, l'onglet actif est dans la vue, rien n'est tronqué ; à 320 px avec quatre onglets, rien ne défile ; corriger les composants de `frontend/app/components/guide-nego/`
- [ ] T068 [US5] Parcourir la page au clavier et au lecteur d'écran (écart 28) — ordre de focus, anneau visible, nom accessible, annonce des états, cercle et pilules décochables — puis avec « réduire les animations » ; corriger les composants de `frontend/app/components/guide-nego/`

**Checkpoint** : quickstart § 3.

---

## Phase 8: Polish & recette

- [ ] T069 [P] Relire `frontend/i18n/locales/en/pages/guide-nego.*.json` et `components/gn-*.json` : toutes les clés de `fr` y sont, aucune chaîne en dur dans les gabarits de `frontend/app/{pages,components}/guide-nego/`
- [ ] T070 [P] Vérifier que le site n'a pas bougé (quickstart § 5) : accueil, une page du back-office, `/negociations`, en clair et en sombre ; les six modules fermés se ferment comme avant ; aucun service worker hors de `guide-nego/`
- [ ] T071 Dérouler `specs/008-guide-nego-coquille/quickstart.md` en entier sur un téléphone Android réel, puis l'installation sur iPhone ; noter les écarts et les corriger
- [ ] T072 `make check-safe` au vert (jamais `make check`)
- [ ] T073 Écrire la procédure d'ouverture — `is_enabled = true` **et** `rollout_percent = 100`, pas de déploiement progressif — dans `docs/DEPLOIEMENT.md`, et mettre à jour `docs/AppNego/progress.md` : ligne 0a, journal, point ouvert R3 levé, ADR-019

---

## Dependencies & Execution Order

- **Phase 1 → Phase 2 → US3 → US1 → US2**, dans cet ordre. US3 passe avant US1 pour que toute page naisse fermée. US2 a besoin des pages d'US1 : la liste de garde se calcule sur elles.
- **US4** dépend de la phase 2 et, pour sa page, de `GnEcran` et `GnEnteteGroupe` (US1). **US5** dépend d'US1, d'US2 (bandeau) et d'US4 (`GnSegmente`) pour T066 ; T058 à T063 et T065 peuvent partir dès la fin de la phase 2.
- T001 conditionne T016 et toutes les pages. T010 et T012 s'écrivent tôt et restent verts ensuite. T046 suit T045. T050 et T051 ferment US2.

## Parallel Opportunities

- Phase 2 : T007, T008, T009, T013, T017 ensemble — cinq fichiers disjoints.
- US1 : T026 à T031, puis T033 à T035, T037, T039.
- US5 : T058 à T063 et T065 — sept lots, un sous-agent chacun, périmètre = les fichiers nommés dans la tâche ; donner à chacun `contracts/systeme-de-design.md`, la section de `composants.md`, les écarts tranchés et les six avertissements ci-dessus.
- Séquentiel par nature : US2 (T045 → T051), et tout ce qui touche `layouts/guide-nego.vue` (T016, T025, T040, T047, T049, T053).

## Implementation Strategy

1. **MVP = phases 1 à 5** : l'application fermée par son drapeau, ouvrable sans compte, et qui tient en mode avion — les deux premiers critères de recette.
2. **Puis US4**, court, qui donne sa seule destination à « Profil et réglages ».
3. **Puis US5**, le plus gros volume et le plus parallélisable — troisième critère de recette.
4. `make check-safe` à chaque fin de phase ; un commit par phase.
