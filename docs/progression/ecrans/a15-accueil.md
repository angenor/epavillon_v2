# A15 — Accueil public et vitrine administrable

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 19/08. **Le modèle a été écrit d'abord** : nouveau schéma `content` (`115_content.sql`), les vues `event.v_public_editions` et `programme.v_edition_stats`, et le rôle `video` ajouté à `media.attachment_role`. 3 pages (`index.vue` — qui **remplace la redirection** du 17/08 —, `admin/vitrine/{index,nouveau,[id]}.vue`), 11 composants `app/components/home/`, 6 composants `app/components/admin/showcase/`, 4 utilitaires purs (`showcase.ts`, `edition-history.ts`, `showcase-form.ts`, plus les fabriques de mocks), 3 fichiers de contrats (`types/content.ts`, `types/home.ts`, `types/admin-showcase.ts`), 3 fichiers de mocks, 2 fabriques d'API (`api/home.ts`, `api/admin-showcase.ts`), 6 fichiers de traduction (3 × 2 locales) et 18 espaces réservés d'image. L'aperçu du back-office **réutilise le composant du bandeau public** — pas une seconde mise en page. Deux compléments hors périmètre du prompt : `content.highlight.manage` ajoutée à `mocks/permissions.ts` et le module `content` à `mocks/platform.ts`, sans quoi l'écran des permissions effectives (A12) affichait un code technique. **L'historique des éditions a été refondu le 19/08** : le rail-affiche `min-h-[calc(100svh-var(--nav-height))]`, `--radius-xl` sorti de sa réserve pour les affiches, et les groupes par millésime aplatis à l'affichage — l'ordre reste celui de `groupEditionsByYear()`.

---

## Refonte du panneau « À venir » — 24/08

**L'écart constaté par le commanditaire** : le panneau latéral n'affichait, en pratique, que les encarts composés dans `/admin/vitrine?emplacement=panneau`. Il devait montrer les **événements à venir** puis la **frise des activités retenues**. Les deux blocs existaient déjà dans le code — prochaines séances, prochains rendez-vous — mais ils venaient APRÈS les épingles, et sur une base sans édition ni séance ils ne s'affichaient pas du tout : le panneau ne montrait plus que les annonces.

**Ce qui a été fait** — maquette validée d'abord ([canevas Claude Design](https://claude.ai/code/artifact/48025aba-7c16-4339-812e-8773385a2283)), puis appliquée :

| Fichier | Nature |
|---------|--------|
| `utils/aside-programme.ts` | **neuf** — groupement par journée, écart en journées civiles, édition commune, prochaine séance d'une édition, durée d'une édition |
| `utils/datetime.ts`, `composables/useDateTime.ts` | `formatDayLong` / `dayLong` — « mercredi 17 novembre », l'en-tête de journée d'une frise |
| `components/home/AsideTimeline.vue` | **neuf** — la frise : rail vertical, une pastille par jour, les séances dessous |
| `components/home/AsideThemeTags.vue` | **neuf** — pastilles thématiques sur fond sombre, plafonnées à deux |
| `components/home/AsideEdition.vue` | refondu — carte pleine pour le prochain rendez-vous, ligne compacte pour les suivants |
| `components/home/AsideSession.vue` | complété — lieu, thématiques, sigle d'édition, créneau sans la date (portée par l'en-tête du jour) |
| `components/home/AsidePanel.vue` | recomposé — ordre inversé, `stats` et `now` reçus |
| `components/home/EditionHistory.vue` | `id="editions"` — le panneau y renvoie |
| `pages/index.vue` | passe `stats` et `generated_at` au panneau |
| `i18n/locales/{fr,en}/pages/home.json` | clés `aside.editions.*` et `aside.programme.*` |

**Quatre décisions.**

1. **Les épingles restent, mais en dernier.** Les retirer aurait laissé l'emplacement « panneau » du back-office sans aucune surface d'affichage — un administrateur composerait un contenu invisible. Elles suivent donc la frise, où elles jouent leur vrai rôle : des rappels datés qui accompagnent le calendrier.
2. **« Maintenant » vient de `generated_at`**, jamais de `Date.now()` : deux valeurs différentes au rendu serveur et à l'hydratation feraient rejouer l'écran pour une seconde d'écart.
3. **Le jour est celui de la séance.** Le panneau mêle les éditions ; grouper sur l'horloge du visiteur ferait basculer une séance du 17 au 18 novembre selon l'endroit d'où on regarde la page.
4. **La durée ne se dit que d'un pavillon.** « Douze jours » décrit une COP ; le même calcul appliqué à un cycle de webinaires étalé sur l'année annonçait « 302 jours ». La mention s'efface hors pavillon.

**Vérifié** : `npm run typecheck`, `npm run build`, `make check-api-contract` (130 appels, 123 formes, 19 routes en attente). **Au navigateur**, sur données d'exemple, en 1440 × 900 et en 390 × 844 : les trois blocs dans l'ordre, la frise groupée par jour avec ses fuseaux, le sigle d'édition présent parce que la frise mêle PACO et COP31, les épingles reléguées en fin de panneau. Trois défauts corrigés à la capture — « heure de Belem » sans accent (la ville de l'édition n'était pas transmise), la durée absurde d'un cycle, et l'intitulé complet qui répétait le libellé qu'on venait de lire.

**Reste ouvert, à trancher avec le commanditaire** : la frise doit-elle suivre UNE édition — celle en cours — plutôt que toutes ? Aujourd'hui elle mêle les éditions, comme le faisait le bloc des prochaines séances, et nomme l'édition sur chaque carte quand elles diffèrent.


---

## `home_aside` retiré du modèle — 24/08

**Arbitrage du commanditaire, le même jour** : la colonne « À venir » ne se compose plus. Elle affiche les événements à venir puis la frise des activités retenues, sans rien d'éditorial. Le bandeau d'ouverture, lui, garde la vitrine.

Le modèle écrit qu'« un emplacement sans rendu n'existe pas » (`115_content.sql` § 1). `home_aside` en perdait un : il est retiré partout.

| Fichier | Nature |
|---------|--------|
| `docs/database/115_content.sql` | `content.highlight_placement` ne porte plus que `home_hero` |
| `types/content.ts`, `types/home.ts` | `HighlightPlacement` réduit ; `HomeScreen.aside` supprimé |
| `composables/api/home.ts` | ne lit plus que le bandeau |
| `components/home/AsidePin.vue` | **supprimé** |
| `components/home/AsidePanel.vue` | deux blocs, plus de troisième |
| `pages/admin/vitrine/index.vue` | onglets d'emplacement retirés, `?emplacement=` disparaît |
| `pages/admin/vitrine/nouveau.vue`, `components/admin/showcase/Form.vue` | plus de choix d'emplacement |
| `components/admin/showcase/Preview.vue` | un seul rendu, plus d'aiguillage |
| `mocks/{content,ids,covers,admin-showcase,home}.ts` | quatre épingles, leurs identifiants et leurs rattachements retirés |
| `i18n/locales/{fr,en}/pages/{home,admin.showcase.list,admin.showcase.form}.json` | clés d'épingles et d'emplacement retirées |

**La base a été alignée à chaud**, et c'est délibéré : `make check` commence par un `down -v` qui aurait détruit le seul compte capable de se connecter (voir le journal du 24/08). `DROP VIEW`, recréation du type, `ALTER COLUMN … USING`, vue recréée à l'identique depuis le fichier. `content.highlights` étant vide, aucune ligne ne pouvait être perdue. **Le chargement de zéro a été revérifié sur une base jetable** : 178 tables, aucune erreur.

**Vérifié** : `make check-db-safe` (conforme), `npm run typecheck`, `npm run build`, `make check-api-contract`, l'accueil et `/admin/vitrine` au navigateur.

## 16/09 — la section d'appel devient une affiche

Demande du commanditaire : rendre la section « Appel ouvert » plus belle, avec une image à droite fondue de gauche à droite.

- `components/home/CallSection.vue` : aplat institutionnel, couverture de l'édition (`cover`, à défaut `banner`) sur les trois cinquièmes droits, rebours dans un panneau de verre posé sur l'image. Sous 1024 px l'image passe derrière tout le bloc, voilée. Sans image : libellé de l'édition en filigrane. Appel clos : image en niveaux de gris. Le bouton secondaire devient un lien blanc — le contour accent disparaît sur l'aplat foncé.
- `assets/css/main.css` : **second dégradé de la charte**, `.fade-inverse-start`, arbitré ce jour. Il part de `--color-surface-inverse` ; le texte ne se pose jamais sur sa partie transparente. Guide de style et `CLAUDE.md` amendés.

**Vérifié** : `nuxi typecheck`, l'accueil au navigateur à 1440 px (thèmes clair et sombre) et à 375 px, sans défilement horizontal.

**Écart de données relevé** : le libellé d'édition de la COP31 est tronqué en base (« 31e conférence des Nations Unies sur le »), et le pays s'affiche « Turquie, Türkiye ». Le titre de la section les reprend tels quels.

## 03/10 — frise du panneau « À venir » allégée

Demande du commanditaire : la carte d'une activité dans la frise ne montre plus ni la salle (ni, à défaut, le mode de participation) ni les thématiques. Restent le créneau avec son fuseau, le titre, l'organisation — et le sigle de l'édition quand la frise en mêle plusieurs. `HomeAsideThemeTags` et les clés `home.aside.programme.format` / `moreThemes`, devenus sans usage, sont supprimés.

Même jour, seconde demande : **la carte d'un événement quitte « Prochains événements » dès que sa programmation est publiée** — la frise la nomme déjà (« Au programme — CdP31 »). Condition double dans `AsidePanel` : `programme_published_at` posé **et** au moins une séance de l'édition dans la frise ; sans la seconde, une édition publiée dont les séances ne tiendraient pas dans la frise bornée par l'API disparaîtrait du panneau. Les éditions suivantes remontent, la première prend la carte pleine.

**Vérifié** : `nuxi typecheck`. Non vu au navigateur (API locale arrêtée).

## 03/10 — le témoignage du bandeau passe en bas de cadre

Demande du commanditaire : le panneau de verre de la citation masquait le milieu de la photographie. Trois maquettes comparées sur une planche (bas de cadre, colonne de droite, carte repliée) ; **le bas de cadre est retenu**.

- `HomeShowcaseSlide` : plus d'encart. Le texte s'aligne en bas, au-dessus du rail : pastille de nature et titre sur une ligne, citation avec un grand guillemet cyan suspendu, puis, à droite derrière un filet vertical sur grand écran (dessous sur téléphone), l'auteur, le rattachement et le lien. Même rendu dans l'aperçu du back-office (`compact`, toujours empilé).
- Contraste : fondu `.scrim-fade-bottom` sur les quatre cinquièmes bas ; voile général à 20 % sur grand écran, 45 % sous `lg`, où le texte couvre la moitié de l'image.
- Nouveau jeton `--color-accent-on-inverse` (cyan-300) pour le guillemet ; libellé du guillemet en i18n (`home.showcase.quoteMark`, « et “). `--color-glass-accent` et `--blur-glass-strong` n'ont plus d'usage, gardés et signalés comme tels au guide de style, dont le § « Le verre » est amendé.

**Vérifié** : `nuxi typecheck` ; l'accueil au navigateur sur les données d'exemple à 1440 px et 390 px, sans défilement horizontal. Non vu : l'aperçu du back-office.

**Écart de données relevé** : l'exemple de Biligua Koivogui affiche « Délégation de Guinée · Guinée » — l'organisation et le pays se répètent.

## 03/10 — « Les éditions » : une vue liste en plus des affiches

Demande du commanditaire : garder le rail d'affiches tel quel et offrir une bascule vers la direction « liste éditoriale » (maquette C parmi trois comparées sur une planche).

- `HomeEditionHistory` : deux boutons-icônes `UiButton` (`pressed`, groupe `role="group"`) à côté des onglets — **liste par défaut** (demande du commanditaire, même jour), affiches au choix. Le choix est gardé dans le cookie `ep-editions-vue`, pour que le rendu serveur serve d'emblée la bonne vue. La hauteur d'un écran et les flèches du rail ne valent que pour les affiches. Le rail est désormais suivi par un `watch` et non équipé au seul montage : il naît et meurt avec la vue comme avec l'état vide.
- Vue liste : regroupée par année (`history.groups`, déjà fourni par `buildEditionHistory`), l'année en grand chiffre, grisée quand toutes ses éditions sont closes. Chaque ligne (`HomeEditionRow`) : vignette 16:9 (noir et blanc si l'édition est terminée, millésime en filigrane sans image), série, titre en lien couvrant, nombre d'activités, dates avec fuseau et lieu, état, flèche. Sous `lg`, dates et état passent sous le titre.
- `useEditionSummary` : lien, image, dates, fuseau, lieu et millésime d'une édition, extraits de `HomeEditionCard` et partagés avec la ligne.

**Vérifié** : `nuxi typecheck` ; au navigateur sur les données d'exemple, thèmes clair et sombre, 1440 et 390 px, sans défilement horizontal ; retour aux affiches avec les flèches du rail.

## 03/10 — l'oiseau qui suit le curseur

Demande du commanditaire : appliquer `docs/bird-cursor/` à l'accueil. Arbitrage consigné dans les [décisions du jour](../decisions/2026-10-03.md) et dans `CLAUDE.md`.

- `public/bird-cursor.js` : copie du moteur livré, sans le certificat C2PA embarqué dans le SVG (8 Ko), et bulle aux jetons (`--color-surface-raised`, `--color-text`, `--color-border-strong`, `--radius-lg`, `--font-sans`, `--shadow-md`) — elle suit donc le thème sombre. Le moteur n'est pas modifié.
- `useBirdCursor()` (+ `types/bird-cursor.d.ts`) : charge le script au montage de `/`, le monte avec `scale: 0.38`, le retire en quittant la page ; un retour sur l'accueil le remonte sans recharger le script. Rien sur un écran tactile (`hover: hover` et `pointer: fine`) ; « moins d'animations » est respecté par le script lui-même.
- Perchoirs : cartes du panneau « À venir », carte d'appel, affiches et lignes d'édition. **Un perchoir doit avoir un bord haut visible** : l'en-tête des éditions, retiré le même jour, faisait poser l'oiseau dans le vide au-dessus des onglets (son bord haut est celui du titre, à gauche). La bascule affiches/liste reçoit un cadre fin pour qu'il s'y pose quand il en parle. En vue liste, la liste n'a plus de marge sous l'en-tête : la première ligne s'appuie sur son trait, sans quoi l'oiseau s'y posait 24 px dans le vide. Répliques (`home.bird.*`, fr/en) : titre « À venir », « Tout le programme », dépôt d'une proposition (appel ouvert seulement), lien vers l'édition, bascule affiches/liste.

**Vérifié** : `nuxi typecheck`, `test:site` (16/16) ; au navigateur sur les données d'exemple : oiseau monté, en vol puis posé sur une carte du panneau, bulle d'une réplique, retrait en quittant l'accueil et retour sans doublon.

- Empilement : l'oiseau à 25, entre le contenu de l'accueil (20 au plus) et la barre de navigation (`z-30`), sous laquelle il passe au défilement. La bulle a son propre niveau (9999), comme une infobulle : option `bubbleZIndex` ajoutée à la copie servie, seule retouche du moteur avec le style de la bulle.

- **Sourire** (demande du même jour) : après 2 s de survol de l'oiseau, ou au clic, la paupière se ferme et un arc « ^ » se dessine sur l'œil, avec une joue rosée et un sautillement ; la bulle dit une phrase tirée au hasard (`home.bird.happy`, fr/en), prioritaire sur les répliques. Seul le groupe `#bird` capte la souris (curseur main) ; le clic ne traverse pas vers le lien dessous ; le survol de l'oiseau l'empêche de changer de perchoir. Le SVG est `aria-hidden`. Le composable remonte l'oiseau si la langue change, pour ses phrases.

**Limite relevée** : sur une réplique collée au haut de l'écran (le titre « À venir »), la bulle, bornée à la fenêtre, recouvre l'oiseau. Comportement du moteur, laissé tel quel.

### 05/10 — rail de vignettes centré, barre de défilement

Vignettes centrées sous la citation, commandes de lecture à droite ; la vignette courante porte une barre qui se remplit au rythme du défilement à la place de la coche (signal de forme, pas de teinte). Typecheck vert, vu à 1440 px — [journal](../journal/2026-10-05.md).
