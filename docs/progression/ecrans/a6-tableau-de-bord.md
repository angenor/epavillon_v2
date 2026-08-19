# A6 — Tableau de bord back-office

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de cet écran est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md).

**État** : ✅

---

## Ce qui a été livré

Fait le 17/08. **Le modèle a été corrigé d'abord, sur deux points** : `analytics.mv_daily_registrations` (les inscriptions aux activités par jour, par édition — `mv_daily_signups` compte des comptes de plateforme et ne se ventile pas) et `live.active_incidents_for_event()` (symétrique descendante de `active_incidents(session)`). 1 page, 5 composants sous `app/components/admin/`, 2 fichiers de contrats (`types/analytics.ts`, `types/admin-dashboard.ts`), 3 fichiers de mocks (`incidents.ts`, `analytics.ts`, `admin-dashboard.ts`), 2 fichiers de traduction. Le sélecteur d'événement passe de la navigation latérale à la tête de page et vit dans l'URL. Trois défauts hérités corrigés : redirection de connexion en 404, `hide-label` non transmis par trois champs, inscriptions simulées datées dans le futur **Révisé le 18/08 sur demande du commanditaire** : ApexCharts (`vue3-apexcharts`) remplace les cinq tracés dessinés à la main, six indicateurs de tête ouvrent la zone des chiffres, et `moyenne_mobile_7j` — présente dans le modèle depuis le premier jour, affichée nulle part — devient la seconde série des deux courbes

---

## Écarts relevés en écrivant le tableau de bord du back-office (A6, 17/08)

Deux défauts du modèle, **corrigés dans le SQL avant d'écrire une ligne d'interface** (voir « Modifications du modèle »). Quatre autres points sont des lectures ou des règles que le modèle ne porte pas et qui appartiennent à l'API.

| N° | Écart | Où | Ce qu'il coûte à l'interface | Suite donnée |
|---|---|---|---|---|
| **40** | **RÉGLÉ le 17/08 — « inscriptions » recouvrait deux faits sans rapport.** `mv_daily_signups` compte des créations de compte sur toute la plateforme ; l'écran a besoin des inscriptions AUX ACTIVITÉS d'une édition. Et la projection existante **ne porte pas `event_id`** : elle n'est ventilable par aucune COP | `130` § 2 | Un administrateur détaché sur une seule édition lisait une courbe qui ne parlait pas de son édition — ou pas de courbe du tout | **`mv_daily_registrations` ajoutée au modèle** (§ 4 bis), série continue par (jour, événement) |
| **41** | **RÉGLÉ le 17/08 — les incidents actifs d'une ÉDITION ne se lisaient nulle part.** `active_incidents(session)` remonte la hiérarchie depuis une séance ; personne ne savait la descendre depuis une édition | `080` § 5 | Le bloc d'actions aurait recomposé le balayage à la main, et l'écran A13 une seconde fois — avec deux résultats possiblement différents sur la portée `organization` | **`live.active_incidents_for_event()` ajoutée au modèle**, symétrique descendante, cible résolue |
| **42** | **Aucune vue ne répond à « ce qui demande une action ».** Ce n'est pas une donnée mais un JUGEMENT : cinq familles réunies par un critère métier — quelqu'un de l'équipe peut le régler aujourd'hui — et un seuil de proximité d'échéance | `070`, `075`, `040`, `080`, `130` | Composée écran par écran, la zone 1 deviendrait un N+1 et divergerait au premier écran qui la referait | **Obligation d'API (prompt B7)** : `GET /admin/dashboard` rend la composition entière. Rejouée aujourd'hui par `mocks/admin-dashboard.ts` |
| **43** | **Le délai qui rend un dossier « urgent » n'existe pas dans le modèle.** L'écran alerte sur un dossier sans revue quand son échéance applicable — affectation de revue, à défaut clôture de l'appel — tombe dans les 21 jours, ou quand AUCUN révisionniste n'est affecté | `060` § 5, `070` § 5 | Vingt et un jours est écrit dans le frontend : le jour où l'IFDD voudra en changer, il faudra reprendre le code | À porter par `calls_for_proposals` (une colonne de plus) ou par `platform.settings`. **Non tranché** — inscrit au prompt B4 |
| **44** | **`v_platform_overview` n'est pas filtrable par édition.** Elle compte la plateforme entière, ce qui la rend inutilisable telle quelle sur un écran dont le sujet est UNE édition | `130` § 10 | L'écran n'en affiche que ce qui n'appartient à aucune édition — personnes, organisations, doublons — et tire tout le reste des projections par événement | Assumé : la vue est juste, c'est son usage qui devait être borné. Noté dans `types/analytics.ts` |
| **45** | **`v_operational_health.libelle` est un texte français figé dans le SQL.** Aucun administrateur ne peut le modifier depuis le back-office : ce n'est donc pas une donnée métier, c'est une traduction | `130` § 11 | Le back-office en anglais affichait « Courriels en rebond ou signalés (7 jours) » | L'écran traduit par le **code** de l'indicateur, stable, et retombe sur `libelle` pour un indicateur ajouté en base avant de l'être en i18n |

**Ce que la zone 1 ne montre pas, et pourquoi** : une ligne par FAMILLE, jamais une par élément. Quarante dossiers non évalués donneraient quarante lignes, et le bloc censé se lire d'un coup d'œil deviendrait la liste des propositions — qui existe déjà, avec ses filtres (A7). Décompte, trois exemples nommés, et un lien vers l'écran **déjà réglé** sur le problème (`?filtre=non-evaluees`).

**Deux familles ne se filtrent pas par édition, et c'est voulu** : les doublons d'organisation (une organisation n'appartient à aucune COP) et les incidents de portée globale (ils couvrent toute la plateforme). Aucune des deux ne révèle l'existence d'une autre édition.

**Le premier bloc vide a été vérifié, pas supposé** : la composition a été temporairement neutralisée pour rendre la page. Il rend un encart en retrait, coche verte, « Rien n'attend l'équipe » — ni bordure rouge, ni glyphe d'alerte, ni zone laissée béante.

**Des bâtons et non une ligne** sur les deux courbes quotidiennes : relier « 3 dépôts mardi » à « 0 mercredi » par un segment dessine une pente qui n'a jamais existé, et sur une série creuse le tracé devient un peigne illisible — mesuré à l'écran avant de changer. L'axe des abscisses est un **axe de temps** que les repères étendent : c'est ce qui permet de marquer une échéance encore à venir sans la coller au bord du cadre, donc sans la faire passer pour aujourd'hui.

**Trois défauts hérités, trouvés en chemin et corrigés** : `middleware/auth.ts` redirigeait vers `localePath('/auth/login')` — un chemin de fichier, quand la page déclare ses adresses par `defineI18nRoute` : le 404 a été mesuré, la correction est le NOM de route (`auth-login`), et `organization/join.vue` portait la même faute ; `UiSelect`, `UiInput` et `UiTextarea` ne transmettaient pas `hide-label` à `UiFormField`, qui le gère pourtant — le sélecteur d'événement affichait son libellé deux fois ; les soixante-sept inscriptions simulées de la COP31 étaient datées de septembre-octobre 2026, **entièrement dans le futur**, et la courbe des inscriptions était vide alors que le jeu était plein — antidatées de trente jours, jusqu'au 3 août 2026, jour de publication du programme.

---

## Ce qui a été vérifié le 17/08 sur le tableau de bord du back-office, et comment

Un écran de pilotage se prouve sur ce qu'il AFFICHE, pas sur ce qu'il compile. Tout ce qui suit a été mesuré sur le rendu réel — HTML servi par le serveur de développement, captures d'écran pilotées par Chrome sans interface — et non lu dans le code.

| Contrôle | Résultat |
|---|---|
| **Les deux ajouts au modèle tiennent-ils sur une base vierge ?** | `make check` (`down -v` puis rechargement complet des 18 fichiers) **vert**. `mv_daily_registrations` présente avec son index unique, `live.active_incidents_for_event()` créée, `analytics.refresh_all(false)` rend **huit vues, toutes en succès**, rapport de frontières de modules **vide** |
| **La nouvelle projection dit-elle vrai ?** | Éprouvée en transaction annulée sur un jeu synthétique : quatre jours de série **continue** pour deux inscriptions et une annulation, cumul exact à 1 sur les quatre jours, **annulation comptée à part et jamais soustraite**, moyenne mobile décroissante conforme |
| **La nouvelle fonction remonte-t-elle les quatre portées ?** | Quatre incidents créés (globale, édition, journée, séance) : **les quatre remontent**, triés par gravité décroissante, avec leur cible résolue (« Séance d'essai », « Journée eau », « Édition d'essai »). Un cinquième, publié mais dont la fenêtre est close, **est écarté** |
| **RÈGLE MÉTIER N° 8 — un administrateur d'une seule édition** | Rendu avec le compte de Perret (`admin` sur la seule COP31) : **aucun `<select>` dans la page**, aucune option, aucun compteur, aucune mention d'un périmètre. Le même rendu avec un administrateur global porte **cinq options**. La clé `nav.admin.eventScope.restricted` — « votre compte n'administre qu'un seul événement » — a été **supprimée du fichier de traduction** : c'était l'aveu que le prompt interdit |
| Le tableau de bord est-il réellement filtré par édition ? | COP31 : cinq lignes d'action, entonnoir à 40 dossiers, deux courbes peuplées. COP29 (`?evenement=…`) : **deux lignes seulement** — l'incident global et les doublons, les deux qui n'appartiennent à aucune édition —, entonnoir à zéro, trois blocs en état vide |
| **Le premier bloc reste-t-il lisible VIDE ?** | Vérifié en neutralisant temporairement la composition, puis restauré : encart **en retrait**, coche verte, « Rien n'attend l'équipe » et la phrase qui énumère ce qui a été regardé. Ni bordure rouge, ni glyphe d'alerte, ni zone béante |
| Les cinq familles d'alerte se déclenchent-elles ? | Toutes les cinq sur la COP31 : 6 dossiers déposés sans évaluation, 2 évaluations en retard, 6 chevauchements de créneaux, 2 messages d'incident publiés, 1 doublon d'organisation. Chacune porte ses trois exemples nommés et son lien |
| Les chiffres de l'entonnoir s'accordent-ils avec les dossiers ? | 40 ouverts, 35 déposés, 14 en évaluation, 19 décidés, 16 retenus ; sorties 5 brouillons, 1 retiré, 3 écartés. **16 + 3 = 19 décidés**, 5 + 35 = 40 ouverts. Taux d'acceptation 84 % (16/19), rendement 46 % (16/35) : les deux taux du modèle, distincts et cohérents |
| **L'échéance est-elle marquée sur la courbe ?** | Les deux repères sont placés — « Ouverture » au 1er juin, « Échéance » au 30 septembre, en rouge. **L'échéance est DEVANT** : l'axe est un axe de temps que les repères étendent, la série des dépôts s'arrête au jour présent et l'écart entre les deux se lit |
| Graphiques sobres | Aucun dégradé, aucune ombre, aucun aplat sous la courbe, aucune boîte de légende : **une couleur par série** (cyan pour les dépôts, violet pour les inscriptions) et le nom de la série écrit **au bout de la série**. Les couleurs des thématiques viennent de `taxonomy_terms.color_hex`, jamais d'un jeton |
| **Aucun défilement horizontal à 375 px** | Mesuré : la capture pleine page fait **375 px de large**. Elle en faisait 570 avant correction — une pastille d'exemple portant le titre complet d'une édition poussait toute la page. Les deux parts de la pastille se tronquent désormais, la précision bornée à 40 % |
| Thème sombre | Rendu identique, entièrement porté par les jetons : **aucun composant de cet écran ne teste le thème** |
| Anglais | `/en/admin` : cinq lignes d'action traduites, **aucune clé brute**. Les libellés d'indicateur de santé, français dans la vue SQL, sont traduits par leur **code** avec repli sur le libellé de la vue |
| Les états de l'écran | Chargement (squelettes), erreur avec reprise, vide (aucune édition), **accès refusé** (aucun droit d'administration) — distinct du vide : l'un dit « vous n'avez pas ce droit », l'autre « il n'y a rien à voir » |
| Aucun fichier de code applicatif > 1000 lignes | Le plus long des fichiers créés est `mocks/analytics.ts` (630 lignes). **`useApi.ts` atteint 822 lignes** : à découper avant A8 |

**Repris dans un navigateur réel (`agent-browser --headed`), après ce premier passage** — connexion par le formulaire, pas par un cocher forgé :

| Contrôle | Résultat |
|---|---|
| Connexion réelle, administratrice globale (Bakayoko) | Le sélecteur porte **cinq éditions**. Changer d'édition **recharge les données et écrit l'URL** (`?evenement=…`) : COP30 tombe à deux lignes d'action, entonnoir à un dossier — le filtrage se voit en direct |
| **Connexion réelle, périmètre d'une seule édition** (Nko Diop, `programmer` sur la COP31) | **`document.querySelectorAll('select').length === 0`** sur toute la page. La barre affiche « COP31 — … » et « heure de Belém », rien d'autre |
| **URL forgée vers une édition hors périmètre** | `?evenement=<COP30>` avec ce compte : la page reste sur la COP31 (40 dossiers), et **l'URL est ramenée à l'édition réellement montrée** — un lien recopié ne promet pas ce qu'il n'ouvre pas |
| Arborescence d'accessibilité | Les cinq lignes d'action sont des **liens au nom complet** (décompte, détail, exemples, échéance) ; les trois zones sont des `region` nommées ; le sélecteur est un `combobox` étiqueté |
| Résumé des graphiques pour les lecteurs d'écran | « Graphique : Dépôts, du 9 juin 2026 au 17 août 2026. 35 au total. Journée la plus forte : 2 le 17 juin 2026. » Un graphique est une image : sans ce texte, il ne dit rien |
| Cibles tactiles | `min-height` mesurée à **44 px** sur les lignes d'action (`--target-min`) |
| Destinations | `/admin/evaluations?filtre=en-retard`, `/admin/propositions?filtre=non-evaluees`, `/admin/programmation?filtre=conflits`, `/admin/incidents`, `/admin/organisations/doublons` — chaque ligne mène à l'écran **déjà réglé** sur le problème. Les cinq écrans n'existent pas encore (A7 à A13) : le routeur le signale en console, c'est attendu |
| Dépliage des indicateurs au vert | Les onze s'affichent avec leur valeur et leur seuil (`26 / 120`, `0 / 1`…) |
| Défilement horizontal à 375 px | `scrollWidth === clientWidth === 375`. Mesuré dans le navigateur, pas déduit d'une capture |
| Thème sombre | Bascule à chaud (`data-theme="dark"`), rendu intact |

**Deux défauts corrigés pendant cette passe** : le fuseau s'affichait « heure de **Belem** », sans accent — `timeZoneCityLabel()` déduit la ville de l'identifiant IANA, qui n'en porte pas ; la barre passe désormais `event.events.city`, comme le bandeau d'A3. Et une URL forgée hors périmètre laissait l'adresse annoncer une édition que la page ne montrait pas : elle est réécrite.

**Un compte de démonstration ne peut pas se connecter par l'interface** : Claire Perret, l'administratrice détachée sur la seule COP31, porte `mfa_enabled_at` et le second facteur n'est pas implémenté (emplacement réservé d'A1). Le contrôle du périmètre restreint a donc été fait avec Fatou Nko Diop, `programmer` sur la seule COP31 — `administered_events()` retient ce rôle au même titre qu'`admin`.

---

## Ce qui a été vérifié le 18/08 sur les graphiques du tableau de bord, et comment

Connexion en administratrice globale (`a.bakayoko@ifdd.francophonie.org`), édition COP31, dans un vrai navigateur.

| Ce qui a été vérifié | Comment | Résultat |
|---|---|---|
| **Les sept graphiques se dessinent** | `document.querySelectorAll('.apexcharts-canvas').length` sur `/admin` | 7 — deux étincelles de carte, deux courbes quotidiennes, l'entonnoir, deux répartitions |
| **Les six indicateurs disent ce que le modèle dit** | Comparaison carte à carte avec l'entonnoir affiché juste en dessous | 35 dépôts, 44 jours avant la clôture du 30 septembre, 31 revues rendues sur 39, 84 % d'acceptation sur 19 dossiers tranchés, 18 séances, 20 inscriptions |
| **La bascule de thème SANS RECHARGEMENT** | Clic sur « Sombre » puis « Clair », sans rechargement, capture à chaque étape | Palette, encre des libellés, couleur de la moyenne mobile et fond des infobulles suivent. C'est ce test qui a révélé les défauts 2 et 3 du journal — les deux passaient inaperçus après un rechargement |
| **Les graduations verticales sont entières** | Lecture de l'axe des deux courbes | 0, 1, 2, 3 — et non « 0.00, 1.00 » ni deux graduations portant le même nombre |
| **Les libellés d'axe ne sont plus coupés** | Position et texte de chaque `tspan` d'axe, mesurés dans le DOM | Tous commencent à l'intérieur du cadre ; les plus longs sont tronqués **par la fin** (« Agriculture et alime… »), le libellé entier restant dans l'infobulle |
| **Les couleurs viennent bien de la base** | Répartition par thématique, thème clair et thème sombre | Chaque barre porte le `color_hex` de son terme ; le chiffre écrit dessus change d'encre selon la teinte, par mesure de contraste |
| **L'entonnoir ne prend pas une forme qui mentirait** | Série de l'appel COP31 : 40, 35, 14, 19, 16 | Elle n'est pas décroissante (14 puis 19) : le tracé reste en barres. La forme en trapèzes s'élargirait au milieu, affirmant le contraire des chiffres qu'elle porte |
| **Aucun graphique n'est chargé là où il n'y en a pas** | Session de navigateur neuve sur `/evenements/cop31-belem-2027`, requêtes filtrées sur `apexcharts` | Seul le greffon (quelques lignes) part ; ni `vue3-apexcharts` ni `apexcharts` ne sont demandés |
| **375 px** | Fenêtre à 375 px, `scrollWidth` comparé à `clientWidth` | 375 = 375, aucun défilement horizontal ; les six cartes s'empilent, les axes des graphiques se resserrent sans se chevaucher |
| **Types et construction** | `npm run typecheck`, `npm run build` | Zéro erreur. Le paquet d'entrée client passe de 994 à 913 ko ; ApexCharts vit dans un fragment de 496 ko chargé à la demande |
