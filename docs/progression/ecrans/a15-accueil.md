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
