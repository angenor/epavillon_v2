# A3 — Page publique de l'événement

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 17/08. 1 page + redirection d'accueil, 10 composants sous `app/components/event/`, 1 fichier de contrats (`types/event-programme.ts`), 1 composable (`useCountdown`), 1 utilitaire (`utils/call.ts`, qui rejoue `is_call_open()` et `effective_deadline()`), 4 fichiers de traduction (2 × 2 locales). **Dépendance nouvelle : `vue-cal` 4.10**, la bibliothèque de calendrier que le planificateur A9 réutilisera — ici en lecture seule. **Le modèle a été corrigé d'abord** : écarts n° 14 et 15 soldés dans `v_public_schedule`, plus `reference.term_badges()`. Quatre éditions ajoutées aux données simulées, sans lesquelles quatre états de l'écran n'auraient eu aucune donnée. **Repris le 17/08 après retours** : la programmation est sortie de cet écran pour devenir la page `/programmations` (voir le journal), la frise des jalons est remontée au-dessus de l'encart d'appel, et `UiStatusTimeline` a été corrigée — ses traits de liaison ne touchaient ni la pastille dont ils partaient, ni celle qu'ils rejoignaient

---

## Écarts relevés en écrivant la page publique de l'événement (A3, 17/08)

Les deux écarts que le tableau des points bloqués demandait de trancher **avant cet écran** l'ont été, dans le SQL, base rechargée. Deux nouveaux points apparaissent, tous deux des obligations d'API inscrites au prompt **B3**.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **14** | **RÉGLÉ le 17/08 — `v_public_schedule` ne joignait pas le PAYS de l'organisation** | `075` § 6 | Sur une COP, le pays situe une organisation aussi sûrement que son nom : deux « Réseau ouest-africain… » ne se distinguent que par lui. La carte le recevait en propriété séparée, à charge pour chaque écran de charger `organizations` ET `countries` | **Corrigé dans le SQL** : `organization_country_code` et `organization_country`. `UiSessionCard` lit la ligne, la propriété de contournement ne sert plus qu'à FORCER une valeur en démonstration |
| **15** | **RÉGLÉ le 17/08 — la vue n'exposait que les CODES des thématiques** alors qu'elle agrégeait complètement les journées spéciales | `075` § 6, `020` § 4 | Asymétrie sans raison : les pastilles ont besoin d'un libellé et d'une couleur, tous deux en base. Chaque écran devait charger la taxonomie et refaire la correspondance — c'est ainsi que la v1 a fini avec ses libellés figés dans le frontend | **Corrigé dans le SQL** : colonne `themes`, alimentée par la fonction nouvelle `reference.term_badges()`. `theme_codes` reste, pour filtrer |
| **25** | **`event.events` ne porte pas son visuel** : le rattachement média est polymorphe, et rien dans la réponse d'une édition ne dit où est sa bannière | `050` § 8, `060` § 2 | Un aller-retour de plus par page (`GET /events/:id/banner`), là où la couverture d'une séance est résolue EN BASE par `v_public_schedule`. Le rôle `banner` existe pourtant depuis l'origine dans `media.attachable_roles` — il n'avait simplement jamais été consommé | **Reporté au prompt B3** : `GET /events/:slug` embarque sa bannière résolue par `media.attached_image('event','events',id,'banner')`, et `useApi().events.banner()` disparaît. Ne PAS ajouter une colonne à `event.events` : l'image n'appartient pas à la table, elle est rattachée |
| **26** | **Aucune ressource ne rend « les éditions publiques »** — la page en a besoin pour son sélecteur d'année, et le critère (« ni brouillon ni annulée ») est une règle, pas un filtre d'écran | `060` § 2 | Écrit côté front, ce critère se recopierait dans chaque écran qui liste des éditions, et divergerait au premier statut ajouté à l'ENUM | **Reporté au prompt B3** : `GET /events/public` porte la règle. Le front la tient provisoirement dans `useApi().events.publicList()`, à un seul endroit |

**Un état de l'écran reste sans données, et c'est assumé** : l'encart d'appel « À VENIR » (un appel dont `opens_at` est dans le futur). Le composant le rend, mais aucune édition simulée n'est dans ce cas — il faudrait inventer une COP32 avec une ville hôte que personne ne connaît, et le jeu de données porte déjà une réserve sur le lieu de la COP31. Les deux autres états — ouvert, clos — sont éprouvés sur données réelles.

**Deux états temporels ne peuvent pas se démontrer sur des données figées** : « en cours » et « en direct » supposent que l'instant présent tombe dans un créneau. La légende les rend tous les six en permanence, et le guide de style les montre côte à côte ; c'est le seul endroit où ils coexistent.

---

## Ce qui a été vérifié le 17/08 sur la page publique de l'événement, et comment

Une page qui bascule entre deux vues, recharge un programme et partage des filtres ne se prouve pas au rendu statique. Tout ce qui suit a été exercé **dans un navigateur réel**.

| Contrôle | Résultat |
|---|---|
| **La correction du modèle tient-elle sur une base vierge ?** | `make check-db` (`down -v` puis rechargement complet) **vert**. La vue rend bien ses trois colonnes nouvelles (`information_schema` : `organization_country_code` en `character`, `organization_country` et `themes` en `jsonb`) |
| **`reference.term_badges()` fait-elle ce qu'elle annonce ?** | Éprouvée en base : `'[]'` — et non `NULL` — sur une entité sans terme ; sur deux termes rattachés, rend `code`, `label` (fr + en) et `color`, **dans l'ordre de `entity_terms.sort_order`**. `terms_of()` sur la même entité rend les mêmes codes : les deux fonctions ne divergent pas |
| Les quatre questions du prompt, dans l'ordre | En-tête (titre, dates avec fuseau, lieu, mode, visuel), encart d'appel, frise des jalons, journées spéciales, programmation, critères. L'appel est **au-dessus** de la programmation : c'est la réponse qu'une organisation vient chercher |
| **Les trois états de l'encart d'appel** | **Ouvert** (COP31) : bandeau accent, rebours « 44 jours », bouton de dépôt, lien vers les critères, et la mention « Prolongé ; échéance annoncée à l'origine : 31 août 2026 ». **Clos** (COP30, COP29) : ton gris, plus de bouton de dépôt. **À venir** : rendu par le composant, aucune donnée pour l'exercer — voir les écarts |
| Le rebours est-il juste ? | 44 jours entre le 17/08/2026 et l'échéance effective du 30/09/2026 — c'est `extended_until`, pas `closes_at`. Il se rafraîchit à la minute au-delà d'une heure restante, à la seconde en deçà |
| **La frise des jalons ne devance pas les événements** | Les quatre jalons sont datés d'avance : la déduction « dernière étape datée » de `UiStatusTimeline` aurait annoncé la conférence terminée. Les états sont donc calculés — ouverture *franchie*, clôture *courante*, résultats et tenue *à venir* |
| **Une date sans heure n'affiche pas d'heure** | `results_expected_at` est une colonne `date` : elle rendait « 15 novembre 2026 **à 09:00** », heure produite par la conversion vers Belém et jamais saisie. Corrigé dans l'encart et dans la frise (`TimelineStep.dateOnly`) |
| Journées spéciales | Les deux fils publiés de la COP31, avec leur couleur en liseré, leur portée annoncée « dates indicatives », et le nombre d'activités rattachées. La journée genre et climat de la COP30 apparaît quand on consulte cette édition : **une archive garde ses journées spéciales** |
| **Les deux vues portent les mêmes données** | Filtre « Finance climatique » posé en vue liste : « 4 activités sur 20 », jeton de filtre affiché. Bascule en calendrier : **4 blocs seulement**, et le calendrier s'ouvre sur le premier jour qui en porte un (10 novembre). Retour en liste : le filtre est toujours là |
| Le jour du calendrier EST le filtre de jour | Naviguer d'un jour à l'autre dans le calendrier met à jour le filtre partagé ; revenir en liste montre ce jour-là. Sans cela, la navigation du calendrier serait perdue au changement de vue |
| **Le sélecteur d'année** | `CONFÉRENCES` : COP31 · 2027, COP30 · 2025, COP29 · 2024. `AUTRES ACTIVITÉS` : PACO27, PACO26. Le classement vient de `event_series.kind`, pas d'une liste de slugs |
| Changer d'année ne quitte pas la page | COP30 sélectionnée → bandeau « Vous consultez la programmation de COP30 », bouton « Revenir à COP31 », 5 activités, légende réduite à la journée genre et climat, **et l'en-tête, l'appel et les critères restent ceux de la COP31**. L'URL porte `?programme=cop30-belem-2025` : le lien se partage |
| **L'état « terminée » existe enfin** | Les trente séances de la COP31 se tiennent en novembre 2027 : toutes « à venir ». Les archives rendent `temporal_state = 'past'` — pastille grise, carte en retrait |
| **Le fuseau suit l'ÉDITION, pas la plateforme** | Les webinaires PACO affichent « heure de Toronto », et **le décalage change en cours d'année** : `UTC−5` le 12 février, `UTC−4` le 16 avril. Le calendrier place ses blocs à l'heure de l'édition, pas à celle de la machine (`wallClockInZone()`) |
| Programme non publié | PACO 2027 : la section reste présente et annonce « Programme à venir — la programmation sera publiée après la sélection des activités » |
| Édition sans appel | `/evenements/paco-2026` : ni encart d'appel, ni frise des jalons ; les critères affichent leur état vide. Sans pavillon, pas d'appel — règle métier n° 5 |
| Édition inconnue | `/evenements/inconnu` répond **200** avec « Édition introuvable », et non une erreur technique : l'adresse est fautive, rien n'est en panne |
| **Le repère qui ne repose pas sur la couleur** | Légende visible en permanence, six états nommés. Dans le calendrier, chaque bloc porte **l'heure, le NOM de l'état écrit, et le titre** ; la couleur de la journée spéciale est un liseré, distinct de l'état |
| Accessibilité du calendrier | Le contenu de chaque bloc est un vrai `<button>`, donc atteignable au clavier — les blocs de vue-cal ne le sont pas. La vue liste reste l'équivalent complet, et c'est elle par défaut |
| **La page est-elle atteignable ?** | `/` **redirige** vers l'édition en cours (302 vers `/evenements/cop31-belem-2027`), `/en` vers `/en/events/…`, fragment conservé. Les entrées « Programme » et « Appel à propositions » de la barre publique pointaient vers trois adresses **sans page** : elles visent désormais les ancres de cette page |
| 375 px, thème sombre, thème clair, anglais | `scrollWidth == clientWidth == 375`, fond `rgb(35,31,32)` — le noir de charte. Le calendrier suit les jetons dans les deux thèmes (sa feuille de style native est écrite pour un thème clair). Anglais complet, **zéro clé brute** |
| Non-régression du guide de style | `/style-guide` répond **200** dans les deux langues, thématiques et pays toujours rendus sur les cartes — désormais lus sur la ligne de la vue et non recomposés |
| `make check-front` | Vert — `nuxt typecheck` à 0 erreur, construction complète |
| Aucun fichier de code applicatif > 1000 lignes | Le plus long des fichiers créés est `app/components/event/Programme.vue` (502 lignes) |

**Deux défauts de composants trouvés en chemin, corrigés là où ils étaient** — aucun n'appartenait à cet écran :

| Défaut | Où | Correction |
|---|---|---|
| **`UiSelect` affichait « facultatif » sur un filtre**, et son `placeholder` étant `disabled`, **la valeur « tous » n'était pas re-sélectionnable** : on pouvait filtrer, pas défiltrer | `components/ui/Select.vue` | Propriété `hideOptional` (même motif que la correction d'A2 sur `UiSearchInput`), et les filtres déclarent une option explicite de valeur vide plutôt qu'un `placeholder`. Le commentaire de la propriété dit désormais pourquoi une liste de filtre ne passe pas par elle |
| **`UiStatusTimeline` affichait une heure sur une étape datée au jour** | `components/ui/StatusTimeline.vue`, `types/ui.ts` | `TimelineStep.dateOnly`. La distinction vient du modèle : `results_expected_at` est une `date`, pas un `timestamptz` |

---

## Refonte du 19/08 — le dépôt monte dans le bandeau, la page passe à deux colonnes

**La demande** : « un design plus moderne et beau, le bouton de soumission pourrait être dans le hero parce qu'il est important ; pour ce qui y est déjà, regarde si c'est pertinent de laisser dedans ou de faire sortir ».

### Ce qui a bougé dans le bandeau

| | Verdict | Pourquoi |
|---|---|---|
| Série, état, titre | **reste** | C'est ce qui IDENTIFIE l'édition. Rien ne peut le remplacer plus haut |
| Dates, lieu, mode, pavillon | **reste**, redessiné en tuiles | Quatre faits posés à même la photographie flottaient sans attache et se lisaient comme un paragraphe éclaté. Chacun tient désormais dans sa tuile de verre : le contraste ne dépend plus de ce que montre l'image |
| **Rebours, échéance, bouton de dépôt** | **entre** | C'est la raison de la visite, et il fallait défiler pour la trouver. À 375 px, le premier écran ne montrait que le titre de la conférence |
| Description de l'appel, conditions, consignes, prolongation | **reste hors du bandeau** | Ce qu'on vient VÉRIFIER, pas ce qu'on vient chercher. Dans la colonne latérale (`EventCallDetails`) |

### Ce que la page est devenue

1. **Bandeau** — photographie plein cadre, voile en deux temps, tuiles de verre, **panneau d'action flottant** (`EventHeroCall`).
2. **Deux colonnes** — à droite, collantes : les échéances (frise passée à la **verticale**) et le détail de l'appel ; à gauche : présentation, journées spéciales, programmation, critères.

**Une capsule d'ancres collante avait été posée entre les deux, et le commanditaire l'a fait retirer le même jour.** Elle a existé le temps d'un aller-retour ; ce qui suit garde la trace de ce qu'elle a appris, parce que le piège se représentera au premier sommaire de la plateforme.

### Fichiers

| Fichier | Nature |
|---|---|
| `components/event/HeroCall.vue` | **nouveau** — le panneau d'action du bandeau, trois phases |
| `components/event/CallDetails.vue` | **remplace** `CallBanner.vue` — allégé de l'action, garde ce qu'on vérifie |
| `components/event/Hero.vue` | réécrit — grille deux colonnes, créneau `action` qui expose sa MATIÈRE (`glass` / `surface`) |
| `components/event/HeroFacts.vue` | réécrit — tuiles, `tone: glass \| surface` |
| `components/event/Milestones.vue`, `Presentation.vue`, `SpecialDays.vue`, `ProgrammeLink.vue`, `Criteria.vue` | redessinés |
| `components/ui/Button.vue`, `types/ui.ts` | variante **`glass`**, bornée au média |
| `assets/css/main.css` | défilement animé sous `prefers-reduced-motion: no-preference` |
| `i18n/…/pages/event.public.json` (fr, en) | `sections.*`, `programmeLink.countUnit` ; `programmeLink.count` retiré |

### Ce qui a été vérifié dans un navigateur réel

| Contrôle | Résultat |
|---|---|
| **Le dépôt est-il au premier écran ?** | 1440 × 900 : panneau visible entier à droite du titre. **375 × 812 : le bouton « Déposer une proposition » est à 620 px du haut**, donc dans le premier écran d'un iPhone SE |
| Les trois phases de l'appel | **Ouvert** (COP31) : « 42 jours », échéance, dépôt, critères. **Clos** (COP29) : pastille grise, date de clôture, annonce des résultats, plus de bouton de dépôt. **À venir** : rendu par le composant, toujours sans donnée pour l'exercer |
| **Le piège de la barre d'ancres, avant son retrait** | Son repérage marquait éternellement « L'appel » comme courant : **une section collante intersecte la fenêtre en permanence**, et un `IntersectionObserver` ne sait pas l'en distinguer. À retenir pour le prochain sommaire de la plateforme : un sommaire ne liste pas ce qui est déjà à l'écran, et le repérage se fait sur la dernière section franchie, pas sur ce qui intersecte |
| L'ancre tombe-t-elle au bon endroit ? | Clic sur « Voir les critères d'évaluation » : `#criteres` sous la barre de navigation du site (`scroll-mt-24`) |
| Édition **sans affiche** (COP29) | En-tête sobre : même composition, aucune matière de verre, aucun voile — le panneau d'action passe en `tone="surface"` |
| 375 px | `scrollWidth == clientWidth == 375` |
| Thème sombre | Le verre **ne s'inverse pas** — le fond est une photographie dans les deux thèmes. Les cartes suivent les jetons |
| `nuxt typecheck` + construction | Vert |

### Retrait de la barre d'ancres, le 19/08

Le commanditaire : « enlève la barre de navigation superposée en bas du hero ». Elle est supprimée, et **rien n'en
reste en dette** : `EventSectionNav` et `utils/edition-poster.ts` sont effacés, le type `SectionNavItem` retiré de
`types/navigation.ts`, les libellés `sections.*` retirés des deux locales, et les deux ancres qui n'avaient été créées
que pour elle (`#presentation`, `#journees-speciales`, `#echeances`) rendues à leur absence. Le décalage d'ancre
redescend de `scroll-mt-40` à `scroll-mt-24` — il ne dégage plus que la barre de navigation du site — et la colonne
collante de `lg:top-36` à `lg:top-24`.

**Ce que le retrait coûte, et qui est assumé** : passé le bandeau, plus rien ne rappelle le dépôt. La colonne collante
garde l'échéance et les conditions sous les yeux ; le bouton, lui, demande de remonter.

Le défilement animé de `main.css` **reste** : « Voir les critères d'évaluation », depuis le bandeau, traverse toujours
deux écrans d'un coup, et c'est ce que la règle sert à montrer.
