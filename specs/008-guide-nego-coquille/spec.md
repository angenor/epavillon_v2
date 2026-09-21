# Feature Specification: Guide Négo — coquille et système de design (étape 0a)

**Feature Branch**: `008-guide-nego-coquille`

**Created**: 2026-09-21

**Status**: Draft

**Input**: Étape 0a de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md) — la coquille de Guide Négo, l'application mobile des négociatrices et négociateurs francophones ([brief](../../docs/AppNego/00-brief.md)). Une personne l'installe depuis son navigateur et l'ouvre sans compte ; elle voit la barre d'onglets, le bouton « Aa » et un état vide par onglet ; l'application s'ouvre hors connexion et le dit ; un drapeau de module l'ouvre ou la ferme sans redéploiement ; l'onglet Ressources porte la ligne « Profil et réglages », qui n'ouvre que le choix du thème ; les fondations du système de design sont reprises en entier, avec les seuls composants de la coquille et ceux que toute étape emploie ; une page interne montre ce qui est livré, dans les deux thèmes. Prompt corrigé le 21/09 dans la feuille de route.

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| Le système de design : couleurs, tailles, mesures, pictogrammes, composants, états | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) |
| Les écrans de cette étape | [02-socle.html](../../docs/AppNego/design/ecrans/02-socle.html) : « 01 Installation », « 02 Ouverture », « 08 Ma journée hors connexion » (pour le bandeau seulement), « 14c Ressources » (pour la barre d'onglets et la ligne « Profil et réglages »), « 11 Profil » (pour le sélecteur segmenté du thème seulement) |
| Ce qui se reprend tel quel | [design/passation/](../../docs/AppNego/design/passation/) : thème, mesures, pictogrammes, composants, mouvement, police |
| Les écarts entre le système et les pages | Tranchés dans [05-design.md](../../docs/AppNego/05-design.md) § « Les écarts, tranchés » — **le code suit ces décisions, pas les pages qui les contredisent** |
| Les décisions | [ADR-002](../../docs/AppNego/adr/002-web-installable-d-abord-capacitor-avant-les-echanges.md) installable depuis le navigateur · [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) hors connexion · [ADR-016](../../docs/AppNego/adr/016-vert-et-jaune-fonces-police-hors-charte.md) et [ADR-018](../../docs/AppNego/adr/018-direction-typographique-quatre-onglets.md) design |
| Les principes | Constitution, XI « Hors connexion d'abord » et XIII « Un design propre et borné » |

**Hors périmètre** : le compte et la connexion, le code d'invitation, le verrou de module réservé en fonctionnement, l'accueil « Ma journée », les thématiques, le profil hors le choix du thème, les notifications, le lexique lui-même, les composants propres à une étape ultérieure, et **tout contenu**. Ils viennent aux étapes 0b, 0c et suivantes.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Installer l'application et l'ouvrir sans compte (Priority: P1)

Une négociatrice reçoit l'adresse de Guide Négo. Elle l'ouvre dans le navigateur de son téléphone : une page lui explique comment l'ajouter à son écran d'accueil, sur Android et sur iPhone, et lui laisse le choix de continuer dans le navigateur. Une fois installée, l'application s'ouvre depuis son icône, plein écran, comme une application. Sans créer de compte, elle voit l'écran d'ouverture, qui dit ce qu'on trouve sans compte, avec un compte et avec un code d'invitation ; elle continue en visiteuse et arrive dans l'application : un en-tête avec le bouton « Aa », une barre d'onglets en bas, et pour chaque onglet un état vide qui dit ce qui viendra là. L'onglet Ressources porte en plus la ligne « Profil et réglages ».

**Why this priority** : c'est le squelette sur lequel toutes les étapes suivantes posent leurs écrans. Sans lui, rien d'autre ne se livre.

**Independent Test** : sur un téléphone Android, ouvrir l'adresse, installer, lancer depuis l'icône, continuer en visiteuse, toucher chaque onglet.

**Acceptance Scenarios**

1. **Given** une personne qui ouvre l'adresse dans le navigateur de son téléphone, **When** la page d'installation s'affiche, **Then** elle lit les étapes pour Android et pour iPhone, et dispose de deux sorties : installer, ou continuer dans le navigateur.
2. **Given** un navigateur capable de proposer l'installation, **When** elle touche « Installer Guide Négo », **Then** le navigateur propose l'ajout à l'écran d'accueil ; **Given** un navigateur qui n'en est pas capable, **Then** les étapes manuelles restent la marche à suivre et aucun bouton ne reste sans effet.
3. **Given** l'application installée, **When** elle la lance depuis son icône, **Then** elle s'ouvre plein écran, sans barre d'adresse, sous le nom « Guide Négo » et avec l'icône provisoire — et ne repasse pas par la page d'installation.
4. **Given** une première ouverture sans compte, **When** l'écran d'ouverture s'affiche, **Then** il présente les trois niveaux d'accès de la maquette et la sortie « Continuer en visiteur », qui mène à l'application.
5. **Given** les Échanges fermés, **When** la barre d'onglets s'affiche, **Then** elle porte **quatre** onglets — Accueil · Négociations · Francophonie · Ressources —, chacun avec son pictogramme et son libellé entier, à largeur de libellé.
6. **Given** les Échanges ouverts — le drapeau des canaux d'échange allumé —, **When** la barre s'affiche, **Then** elle en porte **cinq**, « Échanges » en quatrième position, et tient dans 360 px sans tronquer ni abréger aucun libellé.
7. **Given** n'importe quel onglet, **When** elle le touche, **Then** l'onglet devient actif (filet, graisse, couleur), l'écran porte son titre et le bouton « Aa » en haut à droite — sans bouton de notifications ni avatar, qui viennent aux étapes 3b et 0b —, et montre un **état vide** : un pictogramme, un titre, ce qu'il n'y a pas encore.
8. **Given** le bouton « Aa », **When** elle le touche, **Then** il répond au geste et mène à un état vide du lexique — le lexique lui-même vient à l'étape 2.
9. **Given** l'onglet Ressources, **When** il s'affiche, **Then** il montre son état vide **et** la ligne « Profil et réglages », seule ligne de l'écran.
10. **Given** une personne déjà passée par l'écran d'ouverture, **When** elle rouvre l'application, **Then** elle arrive directement sur l'onglet Accueil.

---

### User Story 2 — L'application s'ouvre en mode avion (Priority: P1)

Dans une salle où le réseau est saturé, ou en mode avion, la négociatrice touche l'icône. L'application s'ouvre quand même, avec sa police, ses pictogrammes et ses onglets. Un bandeau jaune lui dit qu'elle est hors connexion et de quand date ce qu'elle lit ; ensuite, un rappel discret reste dans l'en-tête. Au retour du réseau, l'en-tête dit « Synchronisé à… ».

**Why this priority** : c'est le premier critère de recette de l'étape, et la promesse centrale de l'application (ADR-003, principe XI). Une coquille qui attend le réseau ne sert pas en salle.

**Independent Test** : ouvrir l'application une fois avec réseau, la fermer, passer en mode avion, la relancer depuis l'icône, parcourir tous les onglets.

**Acceptance Scenarios**

1. **Given** l'application déjà ouverte une fois avec réseau, **When** elle est relancée en mode avion, **Then** elle s'ouvre complètement : mise en page, police, pictogrammes, tous les onglets et leurs états vides, l'écran d'ouverture, la page fermée — aucun écran blanc, aucune erreur du navigateur.
2. **Given** une ouverture hors connexion, **When** le premier écran s'affiche, **Then** un bandeau pleine largeur, sous l'en-tête, dit « Hors connexion — lu à HH:MM » avec l'heure de la dernière lecture réussie, et ce qui reste lisible.
3. **Given** le bandeau déjà vu, **When** elle change d'onglet, **Then** le bandeau ne revient pas ; un rappel discret « Hors connexion — lu à HH:MM » reste dans l'en-tête de chaque écran.
4. **Given** le réseau qui revient pendant l'utilisation, **When** une lecture réussit, **Then** le rappel laisse place à « Synchronisé à HH:MM », sans rechargement demandé à la personne.
5. **Given** le réseau qui tombe pendant l'utilisation, **When** l'application le constate, **Then** le bandeau apparaît une fois, et rien de ce qui était affiché ne disparaît.
6. **Given** une personne qui n'a jamais ouvert l'application, **When** elle tente de l'ouvrir sans réseau, **Then** le comportement est celui du navigateur : rien n'a pu être gardé, et ce cas n'est pas à couvrir.
7. **Given** une première ouverture avec réseau, **When** l'application est gardée en entier, **Then** un message éphémère dit « Prête hors connexion » — une seule fois par téléphone ; tant qu'il n'a pas paru, rien ne promet l'ouverture sans réseau.
8. **Given** une nouvelle version de l'application mise en ligne, **When** la personne rouvre l'application avec réseau, **Then** elle reçoit la nouvelle version sans désinstaller, et l'ancienne continue de s'ouvrir hors connexion tant que la nouvelle n'est pas arrivée en entier.
9. **Given** une nouvelle version en ligne et un réseau trop lent pour la charger, **When** elle ouvre l'application, **Then** elle s'ouvre sur la version gardée, en moins de deux secondes ; la nouvelle se prépare en arrière-plan et sert à une ouverture suivante.
10. **Given** la même situation, **When** l'IFDD éteint le drapeau, **Then** l'application se ferme dès que la réponse arrive : l'arrêt d'urgence n'attend pas la mise à jour.

---

### User Story 3 — L'IFDD ouvre et ferme l'application sans redéployer (Priority: P1)

L'équipe de l'IFDD tient un interrupteur : tant qu'il est éteint, personne n'entre dans Guide Négo, et l'adresse montre une page qui dit que l'application n'est pas encore ouverte. Le jour venu, l'équipe l'allume et l'application s'ouvre à tous — sans qu'un développeur ait à remettre quoi que ce soit en ligne. En cas de problème, l'équipe l'éteint de la même façon.

**Why this priority** : deuxième critère de recette. L'application se construit par étapes sur le site en ligne ; sans cet interrupteur, chaque étape inachevée serait visible du public.

**Independent Test** : drapeau éteint, ouvrir l'adresse **sans être connecté à aucun compte** : page fermée. Allumer le drapeau en base — `is_enabled = true` **et** `rollout_percent = 100` —, recharger : l'application. L'éteindre : page fermée. Aucun redéploiement entre les trois. Puis, application ouverte, arrêter l'API et relancer l'application : elle reste ouverte.

**Acceptance Scenarios**

1. **Given** le drapeau éteint, **When** une personne ouvre n'importe quelle adresse de Guide Négo, y compris en la saisissant à la main, **Then** elle voit la page « fermée » et aucun écran de l'application.
2. **Given** le drapeau éteint, **When** la page fermée s'affiche, **Then** elle est aux couleurs de Guide Négo — et non à celles du site — et dit en une phrase que l'application n'est pas encore ouverte.
3. **Given** le drapeau qu'on allume en base, **When** la personne rouvre ou recharge l'application, **Then** elle entre, sans redéploiement ni réinstallation.
4. **Given** l'application installée et le drapeau qu'on éteint, **When** la personne la rouvre **avec réseau**, **Then** elle voit la page fermée.
5. **Given** l'application installée, **When** elle est ouverte **sans réseau**, **Then** c'est le dernier état connu du drapeau qui vaut : ouverte si elle l'était, fermée si elle l'était.
6. **Given** le drapeau de Guide Négo, **When** on le bascule, **Then** aucun espace du site ne change d'état, et basculer un drapeau du site — y compris celui de l'espace Négociations — ne change rien à Guide Négo.
7. **Given** l'application ouverte lors de la dernière lecture, **When** l'API est injoignable, trop lente ou répond par une erreur, **Then** l'application reste ouverte : seule une réponse qui dit « éteint » la ferme.
8. **Given** le drapeau allumé mais déployé à moins de 100 %, **When** une personne sans compte ouvre l'application, **Then** elle la voit fermée — c'est pourquoi ce drapeau ne connaît pas de déploiement progressif.
9. **Given** la page interne des composants, **When** le drapeau est éteint, **Then** elle reste fermée au public comme le reste.

---

### User Story 4 — Choisir son thème : clair, sombre ou système (Priority: P2)

En plein soleil entre deux bâtiments, le thème clair ; tard le soir en salle, le thème sombre. Par défaut, l'application suit le réglage du téléphone. La personne peut forcer l'un ou l'autre, et son choix tient d'une ouverture à l'autre.

**Why this priority** : le thème sombre est une condition d'usage (séances de nuit), mais la coquille se démontre déjà en clair.

**Independent Test** : basculer le téléphone en sombre : l'application suit. Ouvrir Ressources → « Profil et réglages » → « Affichage », forcer « Clair », la fermer, la rouvrir : elle est claire.

**Acceptance Scenarios**

1. **Given** aucun choix fait, **When** l'application s'ouvre, **Then** elle suit le thème du téléphone, et change avec lui sans rechargement.
2. **Given** l'écran « Profil et réglages », **When** il s'affiche, **Then** il ne porte qu'un groupe « Affichage » et sa ligne « Thème », avec le sélecteur segmenté de l'écran « 11 Profil » ; ni « Mon accès », ni déconnexion, ni aucun autre réglage.
3. **Given** le réglage du thème, **When** la personne choisit « Clair », « Sombre » ou « Système », **Then** l'application change aussitôt et garde ce choix aux ouvertures suivantes, y compris hors connexion.
4. **Given** un choix « Sombre » enregistré, **When** l'application s'ouvre, **Then** aucun éclair clair ne précède le thème sombre.
5. **Given** le thème sombre, **When** n'importe quel écran ou composant s'affiche, **Then** il porte ses valeurs sombres propres — fond `#101704`, vert et jaune désaturés — et non une inversion ; aucun composant ne garde une valeur claire par oubli.
6. **Given** le choix du thème dans Guide Négo, **When** la personne ouvre le site de l'ePavillon sur le même téléphone, **Then** le thème du site n'en est pas affecté, et l'inverse non plus.

---

### User Story 5 — La page des composants, témoin du système de design (Priority: P2)

L'équipe de conception et les développeurs des étapes suivantes ouvrent une page interne qui montre les fondations du système en entier — couleurs, échelle de texte, mesures, pictogrammes, mouvement — et les composants livrés à cette étape, dans leurs états, en clair et en sombre côte à côte ou par bascule. Elle se compare à `01-systeme.html` pour tout ce qu'elle porte : c'est la preuve que le système est repris fidèlement, et l'endroit où l'on vient chercher un composant avant d'en écrire un. Chaque étape suivante y ajoute les siens.

**Why this priority** : troisième critère de recette. Elle ne sert pas la négociatrice directement, mais chaque écran à venir en dépend.

**Independent Test** : ouvrir la page à 360 px de large, en clair puis en sombre, à côté de `01-systeme.html`, et comparer section par section.

**Acceptance Scenarios**

1. **Given** la page des composants à 360 px, **When** on la compare à `01-systeme.html`, **Then** couleurs, tailles de texte, mesures et pictogrammes correspondent en entier, et chaque composant que la page porte correspond au sien, aux écarts tranchés près.
2. **Given** chaque composant livré à cette étape (FR-028), **When** il est montré, **Then** il l'est dans ses états décrits — repos, pressé, focus, désactivé, actif le cas échéant — et dans les deux thèmes.
3. **Given** la famille de pictogrammes, **When** la planche s'affiche, **Then** elle montre tous ceux de `pictogrammes.svg`, `pause`, `chevUp` et `minus` compris, chacun avec son nom.
4. **Given** un texte contenant « œ », « Œ », des capitales accentuées et des horaires, **When** il s'affiche, **Then** tous les glyphes viennent de la police de l'application — aucun repli visible — et les chiffres des horaires s'alignent en colonne.
5. **Given** la page ouverte à 390 px, **When** la barre à cinq onglets s'affiche, **Then** elle occupe la largeur sans trou ni débordement (écart 29).
6. **Given** un clavier physique ou un lecteur d'écran, **When** on parcourt les composants interactifs, **Then** l'ordre de focus suit l'ordre de lecture, le focus est visible, et chaque changement d'état est annoncé (écart 28).
7. **Given** le réglage « réduire les animations » du téléphone, **When** un composant animé s'affiche, **Then** il apparaît en place, l'arc de chargement et le squelette sont fixes, et aucune information n'est perdue.
8. **Given** un composant de `composants.md` qui n'est pas livré à cette étape, **When** on ouvre la page, **Then** il n'y figure pas — ni ébauche, ni case vide.
9. **Given** la page des composants, **When** une personne du public parcourt l'application, **Then** aucun lien n'y mène : on n'y arrive que par son adresse.

---

### Edge Cases

- **Adresse d'onglet ouverte directement dans le navigateur, sans installation** : l'application s'affiche normalement ; l'installation est proposée, jamais imposée.
- **iPhone** : pas de proposition d'installation par le navigateur ; la page donne les étapes manuelles, et le bouton « Installer » n'y reste pas inerte.
- **Page d'installation ouverte depuis l'application installée** : elle renvoie à l'accueil. Rouverte dans un navigateur, elle s'affiche normalement — un navigateur ne sait pas dire si l'application est déjà installée.
- **Téléphone réglé en anglais, ou site de l'ePavillon consulté en anglais sur le même téléphone** : l'interface de Guide Négo reste en français — c'est sa raison d'être. Elle ne lit ni ne change la langue choisie sur le site. Aucune chaîne n'est en dur pour autant.
- **Taille de texte agrandie, ou écran plus étroit que 360 px** : la barre d'onglets seule défile horizontalement, jamais la page ; aucun libellé n'est tronqué, abrégé ni mis sur deux lignes ; l'onglet actif est toujours amené dans la vue. À 320 px, quatre onglets tiennent sans défiler.
- **Écran plus large que 360 px, tablette, ordinateur** : la colonne de l'application reste lisible et centrée ; aucune mise en page de bureau n'est attendue.
- **Encoche, barre de gestes du téléphone** : la barre d'onglets et l'en-tête ne passent pas dessous.
- **Drapeau illisible et jamais lu auparavant** — pas de réseau, ou API en panne à la toute première ouverture : l'application reste fermée — on n'ouvre pas ce qu'on n'a pas le droit d'ouvrir.
- **Panne d'API en pleine COP** : l'application déjà ouverte le reste. Une erreur, un délai dépassé, une réponse illisible ou un drapeau absent de la réponse ne valent jamais « éteint ».
- **Canaux d'échange allumés alors que l'application est fermée** : l'application reste fermée ; le cinquième onglet ne se voit qu'une fois le drapeau de l'application allumé.
- **Heure de dernière lecture inconnue** (première ouverture déjà hors connexion après installation) : le bandeau dit « Hors connexion » sans inventer d'heure.
- **Heure de lecture et fuseau** : « lu à » et « Synchronisé à » disent l'heure du téléphone, **sans fuseau** — elles servent à juger la fraîcheur contre l'horloge affichée au-dessus. Seules les heures d'événement portent leur fuseau.
- **Lecture qui n'est pas du jour** : l'heure seule tromperait — « lu à 23:10 » paraît frais le lendemain soir. Alors « lu hier à 23:10 », et au-delà « lu le 11 nov. à 23:10 ». « Synchronisé… » suit la même règle.
- **Place de stockage refusée ou vidée par le téléphone** : l'application se rouvre avec réseau et se garde à nouveau ; elle ne reste pas cassée.

---

## Requirements *(mandatory)*

### Functional Requirements

**Installation et ouverture**

- **FR-001** : L'application DOIT être installable depuis le navigateur d'un téléphone Android et d'un iPhone, sans magasin d'applications, et s'ouvrir ensuite plein écran depuis son icône sous le nom « Guide Négo ».
- **FR-002** : Une page d'installation DOIT reprendre l'écran « 01 Installation » : étapes pour Android, étapes pour iPhone, « Installer Guide Négo », « Continuer dans le navigateur ».
- **FR-003** : L'icône DOIT être le symbole inversé de l'ePavillon (`frontend/public/logos/svg/epavillon-symbole-inverse.svg`), à titre provisoire, décliné à toutes les tailles qu'exigent l'écran d'accueil et l'écran de lancement, y compris la forme adaptative d'Android, sans que le symbole soit rogné.
- **FR-004** : À la première ouverture, l'écran « 02 Ouverture » DOIT présenter les trois niveaux d'accès ; seule la sortie « Continuer en visiteur » est active à cette étape. L'application retient que l'écran a été vu.
- **FR-005** : L'application DOIT fonctionner sous le préfixe d'adresse du site en ligne comme sans préfixe : installation, relance depuis l'icône et ouverture hors connexion valent dans les deux cas.

**Navigation**

- **FR-006** : Une barre d'onglets basse DOIT porter Accueil · Négociations · Francophonie · Ressources, et « Échanges » en quatrième position quand le drapeau existant des canaux d'échange, `negotiation.channels`, est allumé. Aucun drapeau n'est créé pour l'onglet : un seul interrupteur commande la fonctionnalité et son onglet. Son état se lit, se garde et se rend hors connexion comme celui du drapeau de l'application (FR-020).
- **FR-007** : Les onglets sont à largeur de libellé, 48 px au moins, pictogramme au-dessus du libellé ; un libellé n'est jamais tronqué, abrégé, mis sur deux lignes, ni sous 13 px. À 360 px, les cinq onglets tiennent exactement. Quand la place manque — police agrandie, écran plus étroit —, la barre seule défile horizontalement et amène l'onglet actif dans la vue ; la page ne défile jamais de côté.
- **FR-008** : Chaque écran DOIT porter l'en-tête du système — titre, sous-titre éventuel, filet — et le bouton « Aa » en haut à droite, 48 px. À cette étape l'en-tête NE porte NI le bouton de notifications et son compteur (étape 3b), NI l'avatar « Mon compte » (étape 0b).
- **FR-009** : Chaque onglet, et la destination du bouton « Aa », DOIVENT montrer un état vide conforme au composant « État vide » : pictogramme, titre, ce qui viendra. Aucun contenu, réel ou d'exemple, n'y figure. L'onglet Ressources porte en plus, et seulement, la ligne « Profil et réglages » de l'écran « 14c Ressources ».
- **FR-010** : L'onglet actif, le retour arrière du téléphone et le rechargement DOIVENT rester cohérents : chaque onglet a son adresse, et la recharger y ramène.
- **FR-011** : Le mot « Programme » seul NE DOIT apparaître nulle part ; les trois agendas gardent leurs noms entiers, y compris dans les états vides.

**Hors connexion**

- **FR-012** : Après une première ouverture avec réseau, l'application entière de cette étape — tous les écrans, la police, les pictogrammes, l'icône — DOIT s'ouvrir sans réseau.
- **FR-012 bis** : Quand l'application est gardée en entier pour la première fois sur ce téléphone, elle DOIT le dire par un message éphémère, « Prête hors connexion ». La page d'installation DOIT dire d'ouvrir l'application une fois avec réseau avant de compter sur elle en salle.
- **FR-013** : Hors connexion, un bandeau conforme au composant « Bandeau de connexion » DOIT s'afficher une fois par épisode hors connexion, puis laisser un rappel dans l'en-tête ; les deux portent l'heure de la dernière lecture réussie.
- **FR-014** : En ligne, l'en-tête DOIT dire « Synchronisé à HH:MM », l'heure de la dernière lecture — distincte de l'instant présent (écart 25). Cette heure et celle de « lu à » sont celles du téléphone, sans fuseau ; une lecture de la veille se dit « hier à HH:MM », une plus ancienne « le 11 nov. à HH:MM » (écart 32).
- **FR-015** : L'application DOIT garder sur le téléphone ce qu'elle a lu, avec l'heure de lecture de chaque chose, et le rendre sans réseau. À cette étape, cela couvre l'état du drapeau et les préférences ; le mécanisme DOIT être celui que les étapes suivantes emploieront pour leurs contenus, sans le réécrire.
- **FR-016** : Une nouvelle version DOIT remplacer l'ancienne sans geste de la personne ni perte de ce qui est gardé, et l'ancienne DOIT rester utilisable hors connexion jusque-là.
- **FR-016 bis** : Une nouvelle version en ligne NE DOIT ni ralentir ni casser l'ouverture : dès qu'une version complète est gardée, l'application s'ouvre dessus sans attendre le réseau ; la nouvelle se garde en arrière-plan, n'est servie qu'une fois complète. Elle prend la main **au chargement d'une page**, jamais en cours d'usage : une version complète déjà gardée sert donc au chargement suivant, et non à une fermeture de l'application, qui sur un téléphone peut ne jamais venir. Une version qui finit de se garder pendant l'usage attend le chargement d'après : aucune saisie ne se perd. L'état du drapeau, lui, se relit à chaque ouverture et s'applique dès sa réponse.
- **FR-017** : Rien de ce que garde Guide Négo NE DOIT modifier le comportement du site de l'ePavillon : ni ses pages, ni ses appels, ni son cache.

**Ouverture et fermeture**

- **FR-018** : Un drapeau de module propre à Guide Négo DOIT ouvrir ou fermer l'application entière. Il est semé éteint. Il est distinct de tous les drapeaux existants, dont celui de l'espace Négociations du site.
- **FR-018 bis** : « Allumé » veut dire `is_enabled = true` **et** `rollout_percent = 100`. L'application s'ouvre sans compte, et sans session la plateforme n'ouvre un drapeau qu'à 100 % : ce drapeau NE connaît PAS de déploiement progressif, et la procédure d'ouverture l'écrit.
- **FR-019** : Drapeau éteint, toute adresse de Guide Négo — page d'installation et page des composants comprises — DOIT montrer la page fermée, au design de Guide Négo.
- **FR-020** : Un changement du drapeau DOIT prendre effet à la prochaine ouverture ou au prochain rechargement avec réseau, sans redéploiement ni réinstallation. Sans réseau, le dernier état lu vaut ; jamais lu, l'application est fermée.
- **FR-020 bis** : Seule une réponse réussie de l'API qui dit « éteint » ferme l'application. Une API injoignable, trop lente, en erreur, ou une réponse illisible VALENT « sans réseau » : le dernier état lu tient.

**Thème**

- **FR-021** : Trois choix — Clair, Sombre, Système — ; « Système » par défaut. Le choix est gardé sur le téléphone, vaut hors connexion, et s'applique avant le premier affichage.
- **FR-022** : La ligne « Profil et réglages » de l'onglet Ressources DOIT ouvrir un écran qui ne porte, à cette étape, que « Affichage → Thème », avec le sélecteur segmenté de l'écran « 11 Profil ». C'est sa place définitive, rien n'y est provisoire : 0b y ajoute « Mon accès » et la déconnexion, 0c le reste.
- **FR-023** : Tous les rôles de couleur DOIVENT avoir une valeur sombre, y compris ceux que la maquette ne montre pas en sombre — pressé, focus, désactivé, bandeau « Remplacé par… », bulle envoyée, voile (écarts 4 et 27). Chaque valeur ajoutée a son contraste mesuré : 7:1 pour le texte courant, 4,5:1 pour le texte secondaire, 3:1 pour un filet ou un pictogramme porteur de sens.
- **FR-024** : Le thème de Guide Négo et celui du site sont indépendants.

**Système de design**

- **FR-025** : Les **fondations** DOIVENT être reprises **en entier** — couleurs et rôles des deux thèmes (`theme.css`), mesures (`mesures.css`), famille complète de pictogrammes (`pictogrammes.svg`), police, mouvement (`mouvement.md`) —, corrigées des écarts tranchés : `#2E3F0E` pour le squelette sombre seulement ; un seul vert d'état en sombre, `#B5D66A` ; pas de 16 px — onglets de filtre et titres de ligne à 17, onglet de filtre haut de 48 ; 13 px réservé aux libellés d'onglets, marque de rôle, compteur et jour de la bande ; 48 px pour une ligne de liste, 56 px pour une ligne à réglage, à cocher ou à cercle ; croix rouge pour « annulé », triangle rouge pour « erreur » ; badges de maquette et rayon de 6 px absents.
- **FR-026** : Le système DOIT rester borné à Guide Négo : il ne redéfinit aucun jeton du site, n'emprunte aucun composant du site, et aucune de ses règles ne s'applique hors de l'application. Inversement, aucune règle du site ne doit altérer l'application.
- **FR-027** : La police Atkinson Hyperlegible Next DOIT être embarquée — 400, 600, 700, italiques fournis, latin et latin étendu — et gardée pour le hors-connexion ; aucune police ne se télécharge d'un service tiers. Les horaires emploient des chiffres tabulaires.
- **FR-028** : Des composants de `composants.md`, seuls DOIVENT exister à cette étape ceux de la coquille et ceux que toute étape emploie : barre d'onglets (compteur compris), en-tête d'écran avec bouton « Aa », bandeau de connexion, états vide et erreur, chargement (arc et squelette), boutons, champ de recherche, champ de formulaire et zone de texte, case à cocher, interrupteur, cercle de choix, filtres et sélecteur segmenté, en-tête de groupe, ligne de réglage, marque d'état, étiquettes, feuille basse, boîte de confirmation, message éphémère, ligne d'information bordée. Tous les autres — lignes de session, de document, de canal, bulles, tour d'assistant, quiz, restitutions, verrou de module réservé… — arrivent avec l'étape qui les emploie et s'ajoutent alors à la page des composants. Aucun n'est ébauché d'avance.
- **FR-029** : Un état DOIT toujours se dire par un pictogramme, un mot et une couleur — jamais la couleur seule. Le jaune ne signale que ce qui concerne la personne à l'instant.
- **FR-030** : Toute cible tactile fait 48 px au moins.
- **FR-031** : Le mouvement suit `mouvement.md` : durées et courbes nommées ; rien ne porte une information par l'animation seule ; « réduire les animations » ramène toutes les durées à zéro et fige l'arc et le squelette.
- **FR-032** : Une page interne DOIT montrer les fondations en entier — couleurs, échelle de texte, mesures, planche des pictogrammes, mouvement — et chaque composant de FR-028, dans ses états et dans les deux thèmes. Elle est faite pour grandir : chaque étape y ajoute ses composants sans la réorganiser. Aucun lien de l'application n'y mène.
- **FR-033** : Les composants interactifs livrés DOIVENT être utilisables au clavier et au lecteur d'écran : ordre de focus, focus visible, nom accessible, annonce des changements d'état — y compris le cercle de choix et les filtres décochables.

**Langue**

- **FR-034** : Aucune chaîne en dur : français par défaut, anglais fourni. L'application s'affiche en français à cette étape, quels que soient le téléphone et la langue du site ; elle ne lit ni n'écrit le réglage de langue du site. Les textes de cette étape sont des textes d'interface ; aucun ne vient de la base.

### Key Entities

- **Drapeau de module de Guide Négo** : l'interrupteur unique de l'application. Allumé ou éteint, semé éteint, basculé en base. Allumé = activé et déployé à 100 %. Distinct des drapeaux des modules du site.
- **Drapeau des canaux d'échange** (`negotiation.channels`) : il existe déjà, semé éteint. Allumé, la barre passe à cinq onglets. À cette étape, seul son effet sur la barre existe ; aucun autre drapeau ne commande l'onglet.
- **Préférences sur le téléphone** : le thème choisi, l'écran d'ouverture déjà vu. Elles ne quittent pas l'appareil — il n'y a pas de compte à cette étape.
- **Dernière lecture** : ce que l'application a lu du réseau et quand. C'est l'heure que disent « Hors connexion — lu à… » et « Synchronisé à… ».
- **Système de design** : jetons de thème clair et sombre, mesures, famille de pictogrammes, police, composants, mouvement — borné à l'application.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : Sur un téléphone Android de milieu de gamme, une personne installe l'application et atteint l'onglet Accueil en moins de deux minutes, sans aide.
- **SC-002** : Installée puis ouverte une fois avec réseau, l'application s'ouvre en mode avion dans 100 % des essais, tous écrans de l'étape compris, en moins de deux secondes — du toucher de l'icône à l'en-tête lisible, sur le téléphone Android de milieu de gamme de SC-001 — et de même sous un réseau bridé, une nouvelle version étant en ligne.
- **SC-003** : Hors connexion, « Hors connexion — lu à HH:MM » est visible sur 100 % des écrans, et l'heure affichée est celle de la dernière lecture réussie.
- **SC-004** : Le drapeau éteint ferme l'application, et allumé la rouvre, au prochain rechargement avec réseau — zéro redéploiement, zéro réinstallation. API arrêtée, l'application ouverte le reste dans 100 % des essais.
- **SC-005** : À 360 px, la comparaison de la page des composants avec `01-systeme.html`, section par section, en clair puis en sombre, ne laisse, pour tout ce que la page porte, aucun écart autre que ceux tranchés dans `05-design.md`. Les fondations y sont entières ; les composants sont ceux de FR-028.
- **SC-006** : À 360 px et à 390 px, les cinq libellés d'onglets sont entiers et la barre ne déborde pas ; la page ne défile jamais horizontalement.
- **SC-007** : 100 % des paires texte-fond des deux thèmes atteignent 7:1 pour le texte courant et 4,5:1 pour le texte secondaire ; 100 % des cibles tactiles font 48 px au moins.
- **SC-008** : Une fois l'application gardée sur le téléphone, aucune requête ne part vers un service tiers, police comprise.
- **SC-009** : Après cette étape, le site de l'ePavillon est inchangé : aucune de ses pages ne diffère à l'affichage, en clair comme en sombre, et ses vérifications existantes passent.
- **SC-010** : Aucune règle de style de Guide Négo ne s'applique hors de l'application, aucun jeton du site n'y est redéfini, aucun composant du site n'y est importé — trois contrôles automatiques, à zéro.
- **SC-011** : Les trois critères de recette de la feuille de route sont constatés sur un vrai téléphone : ouverture en mode avion, fermeture par le drapeau, fidélité de la page des composants.

---

## Assumptions

Confirmées ou corrigées par le commanditaire le 21/09/2026.

- **Le drapeau de l'application** s'appelle `guide_nego.enabled` — nom retenu. L'ajouter au jeu de données initial est une modification de `docs/database/`, à consigner dans `docs/progression/modele.md` (ADR-017) ; sur une base déjà montée, il s'insère à la main, sans `down -v`.
- **Le cinquième onglet** suit `negotiation.channels`, déjà semé pour les canaux d'échange, sur les mêmes tables. Pas de second drapeau : deux interrupteurs pour une seule chose donneraient un onglet ouvert sur une fonctionnalité fermée. L'onglet « Échanges », quand il s'affiche à cette étape, montre un état vide.
- **Écran d'ouverture** : les boutons « Créer mon compte » et « Me connecter » de la maquette arrivent à l'étape 0b. À cette étape, ils ne s'affichent pas — un bouton inerte tromperait.
- **Le mot « Bienvenue »** de l'écran d'ouverture est gardé : il est banni du site, pas de Guide Négo, dont la maquette fait foi.
- **Le thème** se règle dans « Profil et réglages », sa place définitive ; l'écran ne porte que ce réglage à cette étape.
- **L'en-tête** ne porte ni notifications ni avatar à cette étape ; le compteur existe comme partie de la barre d'onglets, dans la page des composants.
- **Ce qui est « déjà lu »** se réduit, à cette étape, à la coquille, aux deux drapeaux et aux préférences. Le mécanisme de garde des données est livré et démontré, pour que l'étape 1 y pose les documents.
- **Aucun rendu côté serveur** pour ces pages, aucun chemin absolu (ADR-002) : l'application doit pouvoir être emballée pour les magasins plus tard sans réécriture.
- **Cibles** : Chrome sur Android et Safari sur iPhone, versions courantes. L'installation assistée vaut là où le navigateur la propose ; ailleurs, les étapes manuelles.
- **Adresse** : l'application vit sous une adresse du site de l'ePavillon ; le sous-domaine montré en maquette (`guide-nego.francophonie.org`) est une illustration, et se décide au déploiement.
- **Aucune route d'API nouvelle** : `GET /platform/feature-flags`, que l'API sert déjà, suffit aux deux drapeaux.
- **Le suivi** de cette étape s'écrit dans `docs/AppNego/progress.md`, jamais dans la progression de l'ePavillon.
