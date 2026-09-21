# Recherche — Guide Négo, coquille et système de design (0a)

Chaque décision : ce qui est retenu, pourquoi, ce qui a été écarté. Les constats viennent de la lecture du dépôt le 21/09/2026.

## Constats de départ

| Constat | Où | Conséquence |
|---|---|---|
| Aucun module PWA, aucun `routeRules`, aucune route en rendu navigateur seul | `frontend/nuxt.config.ts` | Tout est à poser ; rien à défaire |
| Aucun test côté front | `frontend/package.json` | R12 |
| Le magasin des drapeaux du site **ferme tout** quand l'API échoue (`flags = []`) | `frontend/app/stores/features.ts:57` | Contraire de FR-020 bis : Guide Négo ne peut pas s'y fier tel quel — R7 |
| La page de maintenance est au design du site | `pages/maintenance/[module].vue` | Contraire de FR-019 — R7 |
| Le thème du site est un cookie `epavillon_theme`, posé sur `<html data-theme="dark">` | `stores/preferences.ts`, `app.vue` | Guide Négo prend une autre clé, un autre élément, une autre valeur (`sombre`) — R8 |
| i18n en `prefix_except_default` : `/x` et `/en/x` ; `detectBrowserLanguage` écrit le cookie `epavillon_locale` | `nuxt.config.ts:72-90` | Deux adresses par écran, donc deux portées ; et `setLocale` changerait la langue du **site** — R3 |
| Les messages d'une locale partent en **un seul paquet** chargé à la demande | `i18n/locales/fr.ts`, `modules/i18n-messages.ts` | Il entre dans la liste de garde — R5 |
| Le préfixe `/v2/` est un argument de **construction** (`NUXT_APP_BASE_URL`) | `frontend/Dockerfile:21` | Un fichier statique ne peut pas le connaître : chemins relatifs — R4 |
| `theme.css` déclare ses couleurs de charte sous `:root` | `passation/theme.css:7-39` | Sélecteur hors `[data-app="guide-nego"]` : interdit par le principe XIII — R9 |
| `--gn-picto` est une couleur dans `theme.css` et une taille dans `mesures.css` | passation | Collision — R9 |
| Huit rôles sans valeur sombre | `theme.css:189` | FR-023 — R10 |
| Le sprite porte un bloc `<metadata>` C2PA volumineux, et trois `<title>` qui sont des notes d'écart | `pictogrammes.svg` | À nettoyer à la reprise — R11 |
| `GET /platform/feature-flags` rend `ResolvedFeatureFlag[]` (`key`, `is_enabled`), public, résolu par `platform.is_feature_enabled()` | `backend/crates/api/src/routes/platform.rs` | Aucune route nouvelle |
| Node 24 en local | `node -v` | `node --test` lit le TypeScript sans outil — R12 |

---

## R1 — Où vit le code

**Décision** : quatre dossiers, tous nommés `guide-nego`, et un préfixe de composant `Gn`.

| Quoi | Où |
|---|---|
| Pages | `frontend/app/pages/guide-nego/` |
| Mise en page | `frontend/app/layouts/guide-nego.vue` |
| Composants | `frontend/app/components/guide-nego/` — déclaré dans `nuxt.config.ts` avec `prefix: 'Gn'` (`GnBarreOnglets`, `GnBouton`…) |
| Composables | `frontend/app/composables/guide-nego/` — ajouté à `imports.dirs` |
| Logique pure, testable | `frontend/app/utils/guide-nego/` |
| Feuilles de style, police, sprite | `frontend/app/assets/guide-nego/` |
| Fichiers statiques : service worker, manifeste, icônes | `frontend/public/guide-nego/` |
| Traductions | `frontend/i18n/locales/{fr,en}/pages/guide-nego.*.json`, `components/gn-*.json` |

**Pourquoi** : c'est l'arborescence proposée par `01-stack.md`, et un nom de dossier unique rend les trois contrôles du principe XIII écrivables en une ligne de `grep`. Clôt le point ouvert de `progress.md`.

**Écarté** : une couche Nuxt (`layers/`) — isolation plus forte, mais second `nuxt.config`, et ADR-002 demande un sous-arbre, pas une seconde application.

## R2 — Rendu côté navigateur

**Décision** : `routeRules: { '/guide-nego/**': { ssr: false }, '/guide-nego': { ssr: false } }`.

**Pourquoi** : ADR-002. C'est aussi ce qui rend l'application ouvrable hors connexion : le service worker garde **une** page vide, valable pour toutes les adresses.

**Piège** : la page de chargement des routes sans rendu serveur est commune à tout le site. Elle n'existe que pour Guide Négo aujourd'hui ; elle porte le script de thème (R8).

## R3 — Une seule adresse par écran, et le français

**Décision** : les pages de Guide Négo ne sont pas localisées par l'adresse (`defineI18nRoute(false)`) — adresses en français, aucune variante `/en/`. **L'interface s'affiche en français, quel que soit le téléphone** et quel que soit le cookie de langue du site. Guide Négo ne lit ni n'écrit `epavillon_locale`, et n'appelle jamais `setLocale`, qui l'écrit.

**Pourquoi** : le brief le dit — la place de l'application est d'exister « en français » là où les outils officiels sont en anglais ; des menus anglais autour de contenus français n'aident personne. Et `/guide-nego/…` plus `/en/guide-nego/…` feraient deux portées de service worker et deux applications installables. Les fichiers `en` restent fournis (FR-034, règle du dépôt) : un choix de langue rejoindra « Profil et réglages » si le terrain le demande, avec sa propre clé `gn.*`.

**À vérifier en première tâche** : sur une route non localisée, avec `epavillon_locale=en` posé par le site, l'affichage est en `fr`, et le cookie est inchangé après la visite. Si `@nuxtjs/i18n` v10 suit le cookie malgré tout, la mise en page fixe la langue de l'instance (`locale.value = 'fr'`, après chargement des messages `fr`) — écriture en mémoire, sans cookie.

**Vérifié le 21/09 (T001)**, `@nuxtjs/i18n` 10.6, cookie `epavillon_locale=en` posé : la route non localisée s'affiche en `fr`, `/en/guide-nego` rend 404, le nom de route est `guide-nego` sans suffixe de langue. **Mais le module réécrit le cookie en `fr`** : `setLocaleSuspend` appelle `setCookieLocale` sans condition, et aucune option ne l'en empêche par route. Le serveur, lui, n'écrit rien sur ces routes. D'où `frontend/app/plugins/guide-nego-langue.client.ts` : il relève le cookie avant le module (`enforce: 'pre'`) et le rétablit à `page:finish`, sur les seules routes `guide-nego*`. Revérifié : affichage `fr`, cookie resté à `en`. C'est le seul fichier de Guide Négo qui nomme ce cookie, et il vit hors des dossiers `guide-nego`.

Le cookie `epavillon_theme`, lui, est posé à sa valeur par défaut par `app.vue`, racine commune à toutes les pages : ce n'est pas Guide Négo qui l'écrit, et la valeur est celle que le site aurait posée.

**Écarté** : suivre `navigator.languages` ; le repli « deux adresses, deux portées ».

## R4 — Manifeste et service worker : des chemins relatifs

**Décision** : `public/guide-nego/manifest.webmanifest`, statique, avec `start_url: "./"`, `scope: "./"`, `id: "./"` et des icônes en chemins relatifs. Le service worker, `guide-nego/sw.js`, est engendré à la construction (R5) et sa liste est elle aussi relative. Il est enregistré depuis la mise en page, avec `scope` = `<baseURL>guide-nego/`. Les balises `<link rel="manifest">`, `theme-color` et `apple-touch-icon` sont posées par la mise en page, au moyen de `assetUrl()`, qui existe.

**Pourquoi** : une adresse relative se résout contre celle du fichier qui la porte — `/guide-nego/` en local, `/v2/guide-nego/` en ligne, sans rien écrire. C'est la réponse au piège `/v2` de `01-stack.md`, et à « aucun chemin absolu » d'ADR-002. Un service worker servi depuis `guide-nego/` ne peut, par construction, contrôler aucune page du site.

**Écarté** : un manifeste servi par une route Nitro — inutile dès que le relatif suffit.

## R5 — Service worker engendré à la construction, précache complet, cache d'abord

**Le problème qui commande tout** : le site est redéployé souvent, y compris pendant la COP, et chaque déploiement change l'identifiant de construction. Avec une navigation « réseau d'abord », une salle au réseau saturé mais vivant reçoit la nouvelle page vide à temps, puis attend des fichiers `_nuxt/` qu'aucun cache ne porte : écran vide, alors qu'une version complète est dans le téléphone.

**Décision** :

1. **Un module local, `frontend/modules/guide-nego-garde.ts`**, sur le crochet `build:manifest` — comme `modules/i18n-messages.ts`. Il calcule la **fermeture des imports** (statiques et dynamiques, avec leurs `css` et `assets` : police et sprite compris) de l'entrée, de `layouts/guide-nego`, de `pages/guide-nego/**` et du paquet de locale `fr` — `en` n'est pas servi à cette étape, et le garder coûterait des octets sur un réseau saturé ; il rejoindra la liste avec le choix de langue. Il écrit `guide-nego/sw.js` dans la sortie publique, à partir du modèle `frontend/guide-nego/sw.modele.js`, en y inscrivant la version (`buildId`) et la liste. Les adresses y sont **relatives** au service worker (`../_nuxt/…`) : le préfixe `/v2/` reste sans objet (R4).
2. **`install` garde tout, ou rien** : la page vide, la liste, le manifeste, les icônes, dans `gn-<version>`. Un seul fichier manquant fait échouer l'installation ; le navigateur réessaiera, l'ancienne version continue de servir. **Un service worker actif a donc toujours un cache complet.**
3. **Navigation : cache d'abord, toujours.** La page vide servie est celle de la version active. Aucune attente du réseau à l'ouverture.
4. **La nouvelle version se garde sans s'exécuter.** Le fichier `sw.js` garde la même adresse et change d'octets à chaque construction : le navigateur le compare à chaque navigation, et la mise en page appelle `registration.update()` à chaque ouverture. Le nouveau service worker s'installe en arrière-plan, au rythme du réseau. **Pas de `skipWaiting`** : il prend la main à une ouverture suivante, et efface alors les caches plus anciens (FR-016).
5. **Fichiers de construction, police, sprite, icônes** : cache d'abord ; un fichier absent du cache va au réseau et n'est pas gardé — la liste fait foi.
6. **Appels d'API** : jamais gardés ici (R6). **Le drapeau reste lu par l'API** : l'arrêt d'urgence n'attend pas la mise à jour de l'application (R7).

**Abandonné** : le préchauffage par `performance.getEntriesByType('resource')`. Deux angles morts — un tampon limité à 250 entrées, et les imports dynamiques jamais chargés avant la coupure — et il exigeait que la nouvelle version s'exécute pour se garder.

**En développement** : pas de manifeste de construction, donc pas de service worker. Le hors-connexion se vérifie sur une construction (`quickstart.md`).

**Pourquoi pas `@vite-pwa/nuxt`** : il vise l'application entière — portée à la racine ou contrainte partout, et un précache qui prendrait tous les fichiers du site, faute de savoir lesquels sont à Guide Négo. Le module local fait une centaine de lignes et sait exactement cela. **ADR-019 écrit la décision**, cache d'abord compris.

**Coûts assumés** : une correction déployée n'arrive qu'à la **deuxième** ouverture avec réseau — c'est le prix de « s'ouvre toujours », et le drapeau couvre l'urgence. Et la toute première garde prend le temps du réseau : quelqu'un qui installe à l'hôtel et referme au bout de cinq secondes n'a rien de gardé. **On le dit à la personne** : quand `navigator.serviceWorker.ready` se résout pour la première fois sur ce téléphone (clé `gn.garde-annoncee`), un message éphémère dit « Prête hors connexion » ; la page d'installation demande d'ouvrir l'application une fois avec réseau (FR-012 bis).

## R6 — La garde des lectures

**Décision** : IndexedDB, base `guide-nego`, un magasin `lectures` : `cle`, `valeur`, `lu_a` (instant ISO), `empreinte` (nulle à cette étape). Un composable, `useGnLecture(cle, lire)`, rend `{ valeur, luA, source: 'reseau' | 'garde' | 'aucune', etat }` : il tente le réseau, garde en cas de succès, rend la garde en cas d'échec. Aucune bibliothèque : l'enveloppe fait une soixantaine de lignes.

**Pourquoi** : c'est le mécanisme que FR-015 demande de livrer une fois. L'étape 1 y posera ses documents, avec l'empreinte et le « modifié depuis » de `03-api.md`. À cette étape il n'a qu'un client : les drapeaux.

**Préférences** (thème, écran d'ouverture vu) : `localStorage`, parce qu'elles se lisent **avant** le premier affichage, ce qu'IndexedDB ne permet pas.

**Stockage refusé ou vidé** : toute lecture et écriture est sous `try/catch` ; l'application fonctionne alors sans garde, et se regarde à la prochaine ouverture avec réseau.

## R7 — Le drapeau : le registre du site, étendu, et une règle de repli propre

**Décision** : Guide Négo s'inscrit au registre `utils/feature-modules.ts`, comme le demande `CLAUDE.md` — aucune page ne teste son propre drapeau. L'entrée gagne deux champs facultatifs :

- `closedPath` : où mener quand c'est fermé (`/guide-nego/fermee`), au lieu de `/maintenance/<clé>` ;
- `lastKnown: true` : l'état se résout par `useGnDrapeaux()` — lecture gardée (R6) — et non par le magasin du site.

Le middleware global lit ces deux champs ; pour les six modules existants, rien ne change.

**Règle de résolution**, écrite une fois dans `utils/guide-nego/drapeaux.ts`, pure et testée :

| Réponse de l'API | Garde | Verdict |
|---|---|---|
| Réussie, drapeau allumé | — | Ouvert ; gardé |
| Réussie, drapeau éteint | — | **Fermé** ; gardé |
| Réussie, drapeau absent de la réponse | présente | Celui de la garde |
| Échec, délai, réponse illisible | présente | Celui de la garde |
| Échec ou absence | vide | Fermé |

**Sans attendre le réseau** : si la garde porte un état, il vaut **tout de suite** et l'application s'ouvre ; la lecture de l'API part en parallèle et son verdict s'applique à l'arrivée — une réponse « éteint » ferme l'application séance tenante. Seule la toute première ouverture attend la réponse, sous l'arc de chargement. C'est ce qui tient les deux secondes sous réseau lent.

`negotiation.channels` se résout par la même fonction ; il n'est jamais lu si l'application est fermée.

**Pourquoi** : généraliser plutôt que dupliquer. Un second middleware aurait été une seconde définition de « fermé ».

**Écarté** : changer le repli du magasin du site. « Fermé en cas de doute » reste juste pour des modules non construits.

**Données d'exemple** : `guide_nego.enabled` entre dans `mocks/feature-flags.ts`, **allumé**, pour le travail hors ligne ; `negotiation.channels` y reste éteint.

## R8 — Le thème

**Décision** : clé `localStorage` `gn.theme` ∈ `clair | sombre | systeme`, défaut `systeme`. L'attribut `data-theme="sombre"` se pose sur l'élément racine de la mise en page — celui qui porte `data-app="guide-nego"` — et jamais sur `<html>`. « Système » se résout par `matchMedia('(prefers-color-scheme: dark)')`, écouté.

**Sans éclair** : la page de chargement des routes sans rendu serveur porte un script en ligne de cinq lignes qui lit `gn.theme` et pose le fond (`#101704` ou blanc) avant tout le reste. La balise `theme-color` suit le thème.

**Pourquoi** : `theme.css` ne connaît que l'attribut, pas la requête média — le choix de la passation est respecté. Clé, élément et valeur diffèrent de ceux du site : les deux thèmes ne peuvent pas se toucher (FR-024).

## R9 — Reprise de `theme.css` et `mesures.css`

**Décision** : recopiés dans `assets/guide-nego/`, avec quatre corrections et rien d'autre.

1. Le bloc `:root` des `--gn-charte-*` et `--gn-nuance-sombre-*` passe sous `[data-app="guide-nego"]` — **écart 30**.
2. `--gn-picto` (taille) devient `--gn-picto-taille` ; la couleur garde le nom — **écart 31**. Les deux sont inscrits au tableau « Les écarts, tranchés » de `05-design.md`, qui est la référence.
3. Les écarts tranchés : un seul vert d'état sombre `#B5D66A` ; `#2E3F0E` pour le seul squelette sombre ; pas de 16 px — onglet de filtre, titre de ligne et segment à 17 ; onglet de filtre haut de 48.
4. Les valeurs sombres manquantes (R10).

Les feuilles sont importées par la mise en page, pas par `nuxt.config.ts` : elles ne partent pas dans les pages du site. `tokens.json` reste dans `docs/` : c'est la référence des contrastes, pas un fichier d'exécution.

**Styles des composants** : chaque règle s'écrit sous la borne — `[data-app="guide-nego"] .gn-bouton { … }` —, classes préfixées `gn-`, variables `--gn-*`. Pas de `<style scoped>` seul : ses sélecteurs `.x[data-v-…]` sortiraient de la borne que le principe XIII exige, et du contrôle qui la vérifie. `check-guide-nego` lit donc aussi les blocs `<style>` des `Gn*.vue`.

**Conséquence : rien ne se téléporte dans `<body>`.** Les jetons vivent sous `[data-app="guide-nego"]` ; une feuille basse, une boîte de confirmation, un message éphémère ou un voile posés dans `<body>` sortiraient de la borne — sans jetons, sans thème, hors des sélecteurs. La mise en page porte un conteneur `#gn-portail` **dans** son élément racine, et toute téléportation le vise. Le contrôle refuse un `<Teleport to="body">` sous `guide-nego/`.

Aucune classe Tailwind dans Guide Négo : les mesures de la maquette sont en pixels exacts, et les utilitaires du site portent ses jetons.

## R10 — Les valeurs sombres manquantes

**Décision** : huit rôles à définir — `presse`, `focus`, `desactive-fond`, `desactive-texte`, `attention-fond`, `bulle-envoyee` (et son texte), `jauge-fond`, `voile`. Chacun se tire des `--gn-nuance-sombre-*` existantes ; on n'ajoute une nuance que si aucune ne tient le contraste. Seuils : 7:1 texte courant, 4,5:1 texte secondaire, 3:1 filet, anneau de focus et pictogramme porteur de sens. Un script, `frontend/scripts/guide-nego-contrastes.mjs`, calcule les paires déclarées et échoue sous le seuil ; le tableau des valeurs retenues s'écrit dans `05-design.md`, à la suite des écarts.

**Pourquoi un script** : SC-007 demande 100 % des paires ; une mesure à l'œil ne survit pas au premier ajout.

## R11 — Pictogrammes et police

**Pictogrammes** : le sprite est repris sans son bloc `<metadata>` et sans `<title>` — un pictogramme est décoratif (`aria-hidden`), son sens est dans le mot qui l'accompagne (FR-029), et un `<title>` serait une chaîne en dur. Composant `GnPicto` : `nom`, `taille` (16, 18, 20, 24, 26, 40). Le sprite est importé comme fichier et appelé par `<use href>` ; s'il ne se garde pas bien hors connexion sous cette forme, il est inséré une fois dans la mise en page. Les 57 symboles sont repris, `gn-pause`, `gn-chev-up` et `gn-minus` compris.

**Police** : les dix fichiers et `OFL.txt` sous `assets/guide-nego/police/` ; dix `@font-face` avec les deux `unicode-range` de la notice, `font-display: swap`. `font-variant-numeric: tabular-nums` vient de `theme.css`. La déclaration `font-family` vit sous `[data-app="guide-nego"]` ; `@font-face` est une règle sans sélecteur, que le contrôle de bornage admet nommément.

## R12 — Tests et contrôles

**Décision** : pas de cadre de test ajouté.

- **Logique pure** — résolution des drapeaux, état de connexion, choix du thème, largeur des onglets : `node --test` sur `frontend/tests/guide-nego/*.test.ts`. Node 24 lit le TypeScript effaçable ; ces modules n'importent rien de Nuxt.
- **Trois contrôles mécaniques du principe XIII**, dans `frontend/scripts/check-guide-nego.mjs` : aucun import d'un composant du site sous les dossiers `guide-nego` ; aucune déclaration `--ifdd-*` ni `--color-*` dans ses feuilles ; aucun sélecteur hors `[data-app="guide-nego"]` (exceptions : `@font-face`, `@keyframes gn-*`, `@media`). Plus : aucun fichier au-dessus de mille lignes dans ces dossiers, le mot « Programme » seul absent de ses traductions, aucun `setLocale`, aucun `<Teleport to="body">`. Le contrôle du bornage lit les feuilles **et** les blocs `<style>` des composants.
- **Contrastes** : R10.
- Les trois se branchent sur `check-front`, à côté de `check-api-contract`.
- **Le reste est manuel**, et `quickstart.md` l'écrit : téléphone réel, mode avion, comparaison à 360 et 390 px.

**Écarté** : Vitest et Playwright. Utiles, mais c'est une décision pour tout le front, pas pour une étape de Guide Négo.

## R13 — L'icône

**Décision** : un script, `frontend/scripts/guide-nego-icones.mjs`, produit depuis `epavillon-symbole-inverse.svg` : 192 et 512 (`any`), 192 et 512 (`maskable`, symbole dans les 80 % centraux), 180 (`apple-touch-icon`), sur fond `#233400`. Les PNG sont commités ; le script ne tourne qu'à la main. `sharp` entre en dépendance de développement.

**Piège** : le SVG porte un `<text>` « e » en Helvetica ; son rendu dépend des polices de la machine. À vérifier à la première génération ; sinon, convertir la lettre en tracé dans une copie propre à Guide Négo — le fichier du site n'est pas touché.

## R14 — Installation

**Décision** : `useGnInstallation()` capte `beforeinstallprompt`. Bouton « Installer Guide Négo » : s'il y a une proposition captée, il la déclenche ; sinon — iPhone, navigateur sans proposition — il fait défiler jusqu'aux étapes manuelles et y pose le focus. En mode `standalone`, la page d'installation renvoie à l'application. L'adresse montrée en maquette est remplacée par l'adresse réelle, lue de `location`.

## R15 — L'état de connexion

**Décision** : `useGnConnexion()` tient `enLigne` (événements `online`/`offline`, corrigés par le résultat réel des lectures — un Wi-Fi sans issue dit « en ligne » à tort), `luA` (la plus récente des lectures gardées) et `bandeauVu` (par épisode hors connexion, en mémoire). L'en-tête affiche « Synchronisé à HH:MM » ou « Hors connexion — lu à HH:MM » ; sans aucune lecture, « Hors connexion » seul. L'heure est celle du téléphone, **sans fuseau** : elle sert à juger la fraîcheur contre l'horloge du téléphone ; seules les heures d'événement portent le leur. Une lecture qui n'est pas du jour se dit « hier à 23:10 », puis « le 11 nov. à 23:10 » — règle pure dans `utils/guide-nego/connexion.ts`, testée (écart 32).

## R16 — Le modèle

**Décision** : une ligne dans `900_seed.sql` § 2 : `('guide_nego.enabled', '…', false, 0)`. Rien d'autre. Consignée dans `docs/progression/modele.md` (ADR-017). Sur la base locale montée, elle s'insère à la main — `ON CONFLICT DO NOTHING` — ; **jamais `down -v`**. `check-db-safe` n'en dépend pas.

**Ouvrir l'application**, en une requête : `UPDATE platform.feature_flags SET is_enabled = true, rollout_percent = 100 WHERE key = 'guide_nego.enabled'`. Écrit dans `quickstart.md` et, à la mise en ligne, dans `docs/DEPLOIEMENT.md`.
