# Feature Specification: Guide Négo — les sessions de négociation : l'agenda et son import (étape 3a)

**Feature Branch**: `014-guide-nego-sessions-agenda`

**Created**: 2026-09-25

**Status**: Draft — quatre questions tranchées le 25/09 (voir *Clarifications*)

**Input**: Étape 3a de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md) : les sessions de négociation de Guide Négo — les réunions officielles de la CCNUCC, et elles seules : ni les réunions de la Francophonie, ni le Pavillon (ADR-008). Liste d'un jour, fiche, « Mon agenda », import de la source officielle qui n'écrit que les écarts et se coupe seul passé un seuil de lectures manquées, bloc des sessions de « Ma journée ». Critère : l'import tourne sur les données archivées et se coupe seul quand la source manque.

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| Les écrans | [07-sessions.html](../../docs/AppNego/design/ecrans/07-sessions.html) — liste d'un jour, ses quatre variantes (nominal, hors connexion, lecture impossible, vide) ; [08-detail-de-session.html](../../docs/AppNego/design/ecrans/08-detail-de-session.html) — la fiche, sauf l'encart de signalement ; la partie « Mon agenda » de [09-signaler-et-mon-agenda.html](../../docs/AppNego/design/ecrans/09-signaler-et-mon-agenda.html) ; le bloc « Votre prochaine session de négociation » de « Ma journée » dans [02-socle.html](../../docs/AppNego/design/ecrans/02-socle.html) |
| Le système de design | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) — « 4 septies » — et [design/passation/](../../docs/AppNego/design/passation/) ; les états d'une session à la ligne 38 de [05-design.md](../../docs/AppNego/05-design.md) : un état = un pictogramme + un mot + une couleur |
| Les mots employés à l'écran | [design/lexique.md](../../docs/AppNego/design/lexique.md) — « Ouverte · Accès limité », « Source officielle · lu à… », « Traduction automatique », « Lecture impossible » ; « lu à » et « Synchronisé à » sans fuseau (écart 32), les heures de session avec le leur |
| Les décisions | [ADR-008](../../docs/AppNego/adr/008-trois-agendas-jamais-confondus.md) trois agendas jamais confondus · [ADR-009](../../docs/AppNego/adr/009-la-source-officielle-fait-foi.md) la source officielle fait foi · [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) tout ce qui se lit se lit hors connexion · [ADR-005](../../docs/AppNego/adr/005-openrouter-et-embedding-versionne.md) le fournisseur d'IA |
| La décision du commanditaire du 25/09 sur les titres | Le titre anglais reste affiché et fait foi ; une traduction française s'affiche avec lui, marquée « Traduction automatique », **sans relecture humaine** — exception bornée à ces titres, une COP en compte des centaines par jour. Écart au principe XII, à inscrire |
| Les principes | Constitution : XI « Hors connexion d'abord », XII « Confiance », XIII « Un design propre et borné » |
| Ce qui est déjà livré | 0a la coquille, l'onglet « Négociations » (vide) et la garde des lectures ; 0b le compte et l'accès négociateur ; 0c « Mes thématiques » et « Ma journée » avec ses blocs vides ; 1 et 1b les documents et la feuille du terme anglais du lecteur |

## Clarifications

### Session 2026-09-25 — tranché le 25/09 par l'orchestrateur, pour le commanditaire

- Q : Comment l'application sait-elle quel est le groupe de la personne ? → R : **Les groupes de négociation sont un vocabulaire en base**, comme les thématiques, semé des groupes réels (Groupe africain, PMA, G77 et Chine, AOSIS, Groupe arabe, LMDC, AILAC, UE, GIE, BASIC…). Ils se cochent sur l'écran « Mes thématiques » et suivent le compte. Aucun groupe coché : toutes les coordinations passent le filtre. Un titre de la source se rattache à un groupe par les dénominations du terme, jamais par une liste écrite dans le code.
- Q : Qui attribue une thématique à une session ? → R : **L'IFDD rattache une fois par COP chaque point de l'ordre du jour à une thématique**, au back-office ; les sessions de ce point en héritent. Une session sans thématique passe le filtre, marquée « Thématique non précisée ».
- Q : La fiche montre-t-elle les documents de Guide Négo ? → R : **Non en 3a**. Seuls ceux de la source officielle étaient admis — et elle n'en donne aucun (FR-029, constaté après la réponse).
- Q : Que fait « Me rappeler 15 minutes avant » sans notification ? → R : **Accepté tel que proposé** (FR-034) : le bandeau n'existe que dans l'application ouverte, et la ligne sous l'interrupteur le dit.
- Retouche d'une décision prise seul : une session n'est déclarée « Annulée — retirée du programme officiel » qu'**absente de deux lectures réussies de suite** ; une liste partielle ne l'annule pas d'un coup.
- Le type de réunion se relie au lexique **par le texte anglais du terme** (« contact group »), comme le lecteur : le lexique le résout vers son entrée ; aucun identifiant de l'étape 2 n'est requis.

**Hors périmètre** : les signalements et leur encart, l'état « Non annoncée », « Signaler une réunion non annoncée » (3b) ; toute notification d'un changement ; les notifications poussées ; l'export vers le calendrier du téléphone ; « Partager ma journée » ; les blocs « Changements du jour » et « Aujourd'hui, vos trois agendas » de « Ma journée » ; les réunions de la Francophonie (4) et le Pavillon (5) ; la construction du lexique (étape 2, menée en parallèle).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Une négociatrice lit les sessions du jour, avec leur état et leur heure (Priority: P1)

Aïssatou ouvre l'onglet « Négociations » le jeudi de la deuxième semaine. Elle voit la bande des jours de la COP, le jour courant choisi, et la liste des sessions de ce jour, rangée par heure de début. Chaque ligne donne les heures de début et de fin, la salle, le type de réunion, le titre français marqué « Traduction automatique » sous le titre anglais d'origine, l'accès ouvert ou limité, la thématique et l'état. Par défaut, la liste ne garde que ses thématiques ; « Toutes » les montre toutes. En tête, « Source officielle — lu à 11:35 ».

**Why this priority**: c'est la raison d'être de l'étape — savoir où aller, à quelle heure, et si cela a changé.

**Independent Test**: l'import allumé sur les données archivées, ouvrir l'onglet, changer de jour, basculer le filtre, retrouver les six états sur un jour d'essai.

**Acceptance Scenarios**:

1. **Given** l'import allumé et lu, **When** Aïssatou ouvre l'onglet, **Then** la bande ne montre que les jours qui ont au moins une session, le jour courant est choisi s'il en a, sinon le premier jour à venir, et chaque heure porte le fuseau du lieu de la COP (« 15:00 — 16:30, heure d'Antalya »).
2. **Given** Aïssatou suit « Adaptation » et « Genre », **When** le filtre est sur « Mes thématiques », **Then** seules restent les sessions de ces thématiques et les coordinations de son groupe, et le compteur dit combien de sessions sont affichées.
3. **Given** une session dont l'heure a changé à la source, **When** la liste s'affiche, **Then** la ligne est rangée à la nouvelle heure, porte « Déplacée » et montre l'ancienne heure barrée ; une salle changée paraît dans la marque d'état (« était à 11:30, salle 3 »).
4. **Given** une session annulée à la source, **When** la liste s'affiche, **Then** elle garde sa place, son titre est barré et elle porte « Annulée à 09:30 ».
5. **Given** une session dont l'heure de début est passée et l'heure de fin non, **Then** elle porte « En cours » ; passée l'heure de fin, « Terminée ».
6. **Given** aucune session de ses thématiques ce jour, **When** le filtre est sur « Mes thématiques », **Then** l'écran le dit, annonce le prochain créneau de ses thématiques s'il existe, le nombre de sessions du jour sur d'autres thématiques, et propose « Voir toutes les sessions » et « Modifier mes thématiques ».

---

### User Story 2 — L'import lit la source officielle et se coupe seul quand elle manque (Priority: P1)

L'IFDD allume l'import d'un interrupteur du back-office, pour une COP. Un travail récurrent lit la source officielle, compare, et n'écrit que ce qui a changé. Chaque session porte son origine et l'heure de sa dernière lecture. L'administratrice lit l'état de l'import : dernière lecture réussie, dernière tentative, lectures manquées d'affilée, nombre d'écarts de la dernière lecture. Quand la source ne répond plus au-delà du seuil réglé, l'application cesse d'afficher les sessions et renvoie au programme officiel de la CCNUCC. C'est aussi ce qu'elle montre tant que l'import n'est pas allumé.

**Why this priority**: sans lui, rien ne s'affiche ; et c'est la garde qui empêche de montrer une liste périmée comme fraîche (principe XII). Le critère de l'étape en dépend.

**Independent Test**: sur les données archivées, allumer l'import, vérifier les sessions écrites ; relire la même archive et vérifier qu'aucune écriture n'a lieu ; lire une archive modifiée et vérifier que seuls les écarts s'écrivent ; rendre la source injoignable jusqu'au seuil et vérifier que l'application affiche « Lecture impossible » et ne sert plus aucune session.

**Acceptance Scenarios**:

1. **Given** l'import éteint, **When** une personne ouvre l'onglet, **Then** aucune session ne s'affiche et l'écran renvoie au programme officiel de la CCNUCC.
2. **Given** l'import allumé sur l'archive d'une COP passée, **When** la première lecture réussit, **Then** toutes ses sessions officielles sont écrites, chacune avec son origine, son lien vers l'original et l'heure de lecture.
3. **Given** une seconde lecture de la même archive, **Then** aucune session n'est réécrite, seule l'heure de dernière lecture avance, et l'état de l'import compte zéro écart.
4. **Given** une archive où une session change d'heure, une autre de salle, une troisième disparaît et une quatrième apparaît, **When** elle est lue, **Then** exactement ces quatre sessions sont touchées — l'ancienne heure et l'ancienne salle gardées pour l'affichage « avant → après », la nouvelle écrite, la disparue comptée absente —, et l'état de l'import compte quatre écarts (un écart = une session touchée).
5. **Given** la source injoignable ou illisible, **When** les lectures manquées d'affilée atteignent le seuil, **Then** l'application affiche « Lecture impossible — le programme officiel de la CCNUCC n'a pas répondu depuis 06:40 », ne montre plus aucune session officielle, même passée, et renvoie au programme officiel ; **When** une lecture réussit de nouveau, **Then** l'affichage revient seul.
6. **Given** l'administratrice change le seuil, **Then** il vaut dès la lecture suivante, sans redéploiement.

---

### User Story 3 — La fiche d'une session dit ce qui a changé et d'où cela vient (Priority: P2)

Aïssatou touche la ligne « Genre et changements climatiques ». La fiche donne le titre français et l'anglais, l'état, puis, sous « Source officielle · lu à 11:35 », l'heure (ancienne barrée → nouvelle, avec le jour et le fuseau), la salle (de même), le type de réunion avec son terme anglais, le point de l'ordre du jour, l'accès et la thématique ; le lien « Voir l'original sur le programme officiel de la CCNUCC ».

**Why this priority**: la liste suffit pour se rendre en salle ; la fiche sert à comprendre un changement et à préparer.

**Independent Test**: ouvrir la fiche d'une session déplacée, d'une annulée, d'une en cours et d'une coordination de groupe, et vérifier chaque ligne contre la maquette.

**Acceptance Scenarios**:

1. **Given** une session déplacée, **Then** les lignes « Heure » et « Salle » montrent l'ancienne valeur barrée et la nouvelle ; une ligne inchangée montre sa seule valeur.
2. **Given** un type de réunion, **When** Aïssatou touche son terme anglais, **Then** le lexique s'ouvre sur ce terme.
3. **Given** une session hors de l'ordre du jour officiel, **Then** la ligne dit « Hors ordre du jour officiel ».
4. **Given** une session annulée, **Then** « Ajouter à mon agenda » est désactivé.

---

### User Story 4 — Mon agenda : suivre des sessions, voir les chevauchements, être rappelé (Priority: P2)

Aïssatou ajoute trois sessions à son agenda depuis leur fiche. « Mon agenda » les range par jour et par heure. Deux d'entre elles se chevauchent : chacune le dit (« Chevauche 11:00–12:30 — Fonds pour l'adaptation »), rien n'est empêché. Une session annulée reste, barrée, et ne chevauche plus rien. Sur la fiche, elle arme « Me rappeler 15 minutes avant ». Le bloc « Votre prochaine session de négociation » de « Ma journée » montre la prochaine.

**Why this priority**: c'est l'usage quotidien en salle, mais il suppose la liste et la fiche.

**Independent Test**: avec un compte, ajouter deux sessions qui se chevauchent et une annulée, ouvrir « Mon agenda », vérifier les marques ; refaire l'ajout réseau coupé puis rétabli, vérifier qu'il arrive une seule fois.

**Acceptance Scenarios**:

1. **Given** Aïssatou connectée, **When** elle touche « Ajouter à mon agenda », **Then** le bouton devient « Dans mon agenda » avec une coche, et la session paraît dans « Mon agenda » ; toucher de nouveau l'en retire.
2. **Given** sans réseau, **When** elle ajoute une session, **Then** l'ajout se voit tout de suite, part au retour du réseau, et n'arrive qu'une fois.
3. **Given** deux sessions de son agenda qui se chevauchent, **Then** chacune porte la marque de chevauchement qui nomme l'autre, avec ses heures ; **Given** l'une est annulée, **Then** aucune ne la porte.
4. **Given** une personne sans compte, **When** elle touche « Ajouter à mon agenda », **Then** l'application lui propose de se connecter, sans perdre la fiche.
5. **Given** « Me rappeler 15 minutes avant » armé, **Then** la ligne de « Mon agenda » porte « Rappel 15 minutes avant », et la personne voit ce que le rappel fera réellement — voir FR-034.
6. **Given** des sessions dans son agenda, **When** elle ouvre « Ma journée », **Then** le bloc « Votre prochaine session de négociation » montre la prochaine, avec son heure, son titre et sa salle.

---

### User Story 5 — Tout ce qui a été lu se relit sans réseau (Priority: P2)

Dans la salle plénière, le réseau tombe. Aïssatou rouvre l'onglet : la liste des jours déjà lus s'affiche, avec « Hors connexion — lu à 11:35 » et, à la première ouverture hors ligne, le bandeau « Les changements survenus depuis ne sont pas ici ». Les fiches déjà ouvertes s'ouvrent ; « Mon agenda » aussi.

**Why this priority**: principe XI ; c'est la condition d'usage en salle.

**Independent Test**: lire la liste et deux fiches en ligne, couper le réseau, recharger l'application, tout rouvrir.

**Acceptance Scenarios**:

1. **Given** la liste lue une fois en ligne, **When** le réseau est coupé, **Then** tous les jours de la COP se relisent, chacun avec l'heure de la dernière lecture.
2. **Given** une fiche jamais ouverte et dont rien n'a été gardé, **When** hors connexion, **Then** l'écran dit qu'elle n'a pas encore été lue, sans erreur.
3. **Given** la dernière lecture faite alors que l'affichage était coupé, **When** hors connexion, **Then** l'écran montre « Lecture impossible », pas une liste plus ancienne.

---

### Edge Cases

- La source marque une session reportée (« POSTPONED ») sans nouvelle heure : elle passe « Annulée », avec la mention « reportée par la source ».
- Une coordination d'observateurs (organisations de jeunesse, d'entreprises…) ou un événement parallèle : jamais importés.
- Une session disparaît de la source sans être annulée : elle n'est pas effacée ; absente de deux lectures réussies de suite, elle passe « Annulée » avec la mention qu'elle a été retirée du programme officiel, et l'heure de ce constat.
- Une coordination dont le titre ne se rattache à aucun groupe de négociation connu : elle ne se distingue pas d'une coordination d'observateurs, et **n'est pas importée**. Un groupe réel manquant s'ajoute au vocabulaire, en base.
- Une session déplacée deux fois : la marque montre la valeur précédente ; tout l'historique des écarts est gardé.
- Une session sans heure de fin à la source : la ligne ne montre que le début ; elle passe « Terminée » à la fin du jour.
- Une session qui franchit minuit : elle appartient au jour de son début, dans le fuseau de la COP.
- La traduction d'un titre échoue ou la clé manque : le titre anglais s'affiche seul, sans la mention « Traduction automatique » ; la traduction est retentée à la lecture suivante.
- Un titre anglais change à la source : sa traduction est refaite ; l'ancienne n'est plus affichée.
- La source répond mais son contenu est illisible (format changé) : c'est une lecture manquée, pas une liste vide ; jamais « toutes les sessions ont disparu ».
- La source répond avec une liste vide alors que la COP est en cours : lecture manquée, pour la même raison.
- L'import est éteint pendant la COP : l'affichage se coupe aussitôt, comme au seuil.
- Une personne sans thématique choisie : « Mes thématiques » propose de les choisir, et « Toutes » s'affiche.
- Deux sessions de l'agenda identiques en heures : elles se chevauchent.
- Un rappel armé sur une session déplacée suit la nouvelle heure ; sur une session annulée, il se désarme.
- Le téléphone est dans un autre fuseau que la COP : toute heure de session reste celle du lieu, avec son fuseau.

## Requirements *(mandatory)*

### La liste d'un jour

- **FR-001** : L'onglet « Négociations » DOIT n'afficher que les sessions de négociation officielles de la CCNUCC — ni réunion de la Francophonie, ni activité du Pavillon. Le mot « Programme » seul n'apparaît nulle part.
- **FR-002** : La bande des jours DOIT ne compter que les jours qui ont au moins une session, défiler horizontalement, marquer le jour choisi et le jour courant, et griser les jours passés.
- **FR-003** : À l'ouverture, le jour choisi DOIT être le jour courant s'il a des sessions, sinon le prochain jour qui en a, sinon le dernier.
- **FR-004** : Chaque ligne DOIT porter l'heure de début et de fin, la salle, le type de réunion, le titre anglais d'origine précédé de « EN » et, quand elle existe, sa traduction française marquée « Traduction automatique », l'accès (« Ouverte » ou « Accès limité »), la thématique et l'état ; toucher la ligne ouvre la fiche.
- **FR-005** : Les lignes DOIVENT être rangées par heure de début actuelle ; une session annulée garde sa place.
- **FR-006** : Le filtre « Mes thématiques / Toutes » DOIT être offert ; « Mes thématiques » retient les sessions des thématiques que la personne suit les sessions sans thématique, marquées « Thématique non précisée », et les coordinations des groupes que la personne a cochés, portées « Mon groupe » — toutes les coordinations quand elle n'en a coché aucun. Un compteur dit le nombre de sessions affichées.
- **FR-006a** : Les groupes de négociation DOIVENT être un vocabulaire en base, semé des groupes réels ; la personne coche les siens sur l'écran « Mes thématiques », et ce choix suit le compte et passe par la file hors connexion. Une coordination se rattache à son groupe par les dénominations du terme, jamais par une liste du code.
- **FR-007** : La thématique d'une session DOIT venir du point de l'ordre du jour auquel elle appartient, que l'IFDD rattache à une thématique du vocabulaire, une fois par COP, au back-office. Une session sans point rattaché n'a pas de thématique.
- **FR-008** : Les états DOIVENT être Prévue, En cours, Déplacée (ancienne valeur barrée), Annulée, Terminée, chacun avec son pictogramme, son mot et sa couleur. « En cours » et « Terminée » se déduisent de l'heure, dans le fuseau de la COP ; « Annulée » et « Déplacée » viennent de la source.
- **FR-009** : Toute heure de session DOIT porter le fuseau du lieu de la COP ; « lu à » et « Synchronisé à » portent l'heure du téléphone sans fuseau (écart 32).
- **FR-010** : La liste DOIT porter en tête « Source officielle — lu à <heure> » et en pied le lien « Programme officiel de la CCNUCC ».
- **FR-011** : L'état vide de « Mes thématiques » DOIT annoncer le prochain créneau des thématiques suivies s'il existe, le nombre de sessions du jour sur d'autres thématiques, et proposer « Voir toutes les sessions » et « Modifier mes thématiques ».

### L'import

- **FR-012** : Un travail récurrent DOIT lire la source officielle d'une COP, comparer à ce qui est en base, et n'écrire que les écarts : session apparue, disparue, changée (heure, salle, titre, type, accès, point de l'ordre du jour, état).
- **FR-012a** : L'import NE DOIT retenir que les réunions de négociation — négociations, plénières, événements mandatés, consultations de la présidence, coordinations d'un groupe de négociation de Parties — et écarter les événements parallèles, conférences de presse, autres événements de la présidence et coordinations d'observateurs. Le tri se fait par les données des vocabulaires, jamais par une liste du code.
- **FR-013** : Chaque session importée DOIT porter son origine (la source officielle), son identifiant à la source, son lien vers l'original, l'heure de sa première et de sa dernière lecture.
- **FR-014** : Chaque changement d'heure ou de salle DOIT garder la valeur précédente, datée, pour l'affichage « avant → après » ; l'historique des écarts d'une session est gardé.
- **FR-015** : Une session qui disparaît de la source NE DOIT PAS être effacée : absente de **deux lectures réussies de suite**, elle passe « Annulée », avec la mention qu'elle a été retirée du programme officiel et l'heure du constat ; absente d'une seule, elle ne change pas.
- **FR-016** : Une lecture qui échoue — source injoignable, délai dépassé, contenu illisible, liste vide pendant la COP — DOIT compter comme une lecture manquée et n'écrire aucune session.
- **FR-017** : Passé un seuil réglable de lectures manquées d'affilée, l'API NE DOIT PLUS servir aucune session officielle de cette COP et DOIT dire que la lecture est impossible, depuis quand, et renvoyer au programme officiel. Une lecture réussie rétablit l'affichage.
- **FR-018** : L'import DOIT être éteint par défaut ; éteint, l'affichage est coupé comme au seuil. Il s'allume et s'éteint d'un interrupteur du back-office, sans redéploiement.
- **FR-019** : L'état de l'import DOIT se lire au back-office : allumé ou non, COP visée, source lue, dernière lecture réussie, dernière tentative et son erreur, lectures manquées d'affilée, seuil, nombre d'écarts de la dernière lecture, affichage servi ou coupé ; et un journal des dernières lectures.
- **FR-020** : La lecture de la source DOIT passer par un lecteur interchangeable : un lecteur de données archivées — un vrai programme d'une COP passée, rangé dans le dépôt —, et un lecteur de la source réelle, à configurer quand l'accord du secrétariat sera obtenu. Le mécanisme s'éprouve entier sur le premier.
- **FR-021** : L'écriture de l'import NE DOIT déclencher aucune notification (hors périmètre) ni aucun effet réservé aux réunions organisées par l'IFDD.

### La traduction des titres

- **FR-022** : Le titre anglais d'origine DOIT être gardé tel que la source le donne, toujours affiché, et faire foi.
- **FR-023** : Chaque titre anglais DOIT recevoir une seule traduction française automatique, gardée avec le modèle qui l'a produite et sa date, et refaite seulement si le titre anglais change. Elle s'affiche marquée « Traduction automatique », sans relecture humaine (décision du 25/09, écart au principe XII inscrit).
- **FR-024** : Sans traduction — service indisponible, clé absente, échec —, le titre anglais DOIT s'afficher seul, sans mention de traduction ; l'import ne s'arrête pas pour autant.

### La fiche

- **FR-026** : La fiche DOIT montrer le titre français et l'anglais, l'état, puis sous « Source officielle · lu à <heure> » : l'heure avec le jour et le fuseau, la salle, le type de réunion avec son terme anglais, le point de l'ordre du jour ou « Hors ordre du jour officiel », l'accès, la thématique ; puis « Voir l'original sur le programme officiel de la CCNUCC ».
- **FR-027** : Une ligne changée DOIT montrer l'ancienne valeur barrée et la nouvelle ; une ligne inchangée sa seule valeur.
- **FR-028** : Le terme anglais du type de réunion DOIT ouvrir le lexique à l'adresse `/guide-nego/lexique?terme=<texte anglais>` (« contact group ») ; le lexique de l'étape 2 le résout sur le téléphone, sans réseau. Tant que l'étape 2 n'est pas fusionnée, l'adresse mène à un lexique vide — admis. Le terme anglais d'un type est une donnée du vocabulaire.
- **FR-029** : **La source officielle ne lie aucun document à une session** — constaté le 25/09 sur les données réelles de la COP29 et de la COP30. La fiche n'a donc pas de section « Documents liés » à cette étape ; elle viendra avec une source qui les porte. Aucun document de Guide Négo n'est rattaché à une session (Q3).

### Mon agenda et le rappel

- **FR-030** : Une personne connectée DOIT pouvoir ajouter une session à son agenda et l'en retirer depuis la fiche ; l'agenda suit le compte, sur tous ses appareils. Sans compte, le geste propose de se connecter.
- **FR-031** : L'ajout et le retrait faits sans réseau DOIVENT se voir aussitôt, partir au retour du réseau et n'arriver qu'une fois.
- **FR-032** : « Mon agenda » DOIT ranger les sessions suivies par jour et par heure, avec la même bande des jours, le titre français, la salle, la thématique, et l'état quand il n'est pas « Prévue ».
- **FR-033** : Deux sessions suivies dont les créneaux se recouvrent DOIVENT chacune porter « Chevauche <heures> — <titre de l'autre> » ; aucun ajout n'est jamais refusé pour un chevauchement ; une session annulée ne chevauche rien.
- **FR-034** : « Me rappeler 15 minutes avant » DOIT s'armer depuis la fiche d'une session de l'agenda, suivre le compte et passer par la file hors connexion. Ce que la personne voit : la ligne de « Mon agenda » porte « Rappel 15 minutes avant » ; quand l'application est ouverte dans les quinze minutes qui précèdent la session, un bandeau en tête d'écran nomme la session, son heure et sa salle ; sous l'interrupteur, une ligne dit que le rappel paraît dans l'application ouverte, sans sonnerie. Aucun texte ne promet une alerte téléphone fermé.
- **FR-035** : Un rappel DOIT suivre la nouvelle heure d'une session déplacée et se désarmer quand elle est annulée ; retirer la session de l'agenda le désarme.
- **FR-036** : Le bloc « Votre prochaine session de négociation » de « Ma journée » DOIT montrer la prochaine session de « Mon agenda » qui n'est ni terminée ni annulée ; sans agenda, la prochaine session des thématiques suivies ; sinon sa ligne vide actuelle. Il affiche l'heure avec le fuseau, le titre français, l'anglais et la salle. Les autres blocs ne changent pas.

### Hors connexion

- **FR-037** : La liste de tous les jours de la COP, lue une fois en ligne, DOIT se relire sans réseau, avec « Hors connexion — lu à <heure> » ; à la première ouverture hors ligne, un bandeau dit que les changements survenus depuis n'y sont pas.
- **FR-038** : Les fiches déjà ouvertes et « Mon agenda » DOIVENT se relire sans réseau ; une fiche jamais lue le dit, sans erreur.
- **FR-039** : Une copie gardée alors que l'affichage était coupé DOIT se relire coupée ; jamais une copie plus ancienne ne remplace la coupure.

### Back-office

- **FR-040** : Le back-office de Guide Négo DOIT porter l'écran de l'import de la COP que sert l'application : l'interrupteur, le choix de la source, le seuil de lectures manquées, l'état et le journal (FR-019) ; et la liste des points de l'ordre du jour de la COP, chacun rattaché ou non à une thématique, le nombre de points sans thématique étant affiché. Il est réservé aux administrateurs de toute la plateforme, comme les autres écrans de Guide Négo.
- **FR-041** : Le back-office NE DOIT PAS permettre de modifier une session officielle : la source fait foi ; les corrections viendront par les signalements (3b).

### Ce qui vaut pour toute l'étape

- **FR-042** : La lecture des sessions et de l'état de l'import est publique ; l'agenda et le rappel demandent un compte ; aucune lecture ne demande l'accès négociateur.
- **FR-043** : Les types de réunion (plénière, groupe de contact, consultations informelles, aparté, coordination de groupe…) et les salles sont des données, jamais des traductions.

### Key Entities

- **Session de négociation** : une réunion officielle d'une COP, venue de la source ; titre anglais d'origine et sa traduction, début et fin, salle, type, accès, point de l'ordre du jour, thématique, état, origine, identifiant et lien à la source, heures de première et de dernière lecture.
- **Changement** : un écart constaté par l'import sur une session — champ, ancienne et nouvelle valeur, heure du constat.
- **Import d'une COP** : l'interrupteur, la source et son lecteur, le seuil, l'état courant, les lectures manquées d'affilée.
- **Lecture** : une tentative de l'import — heure, issue, erreur, nombre d'écarts.
- **Traduction d'un titre** : le texte français, le modèle et la date, pour un titre anglais donné.
- **Type de réunion** : un terme du vocabulaire, avec son terme anglais, qui ouvre le lexique.
- **Point de l'ordre du jour** : son numéro et son intitulé à la source, et la thématique que l'IFDD lui rattache pour la COP.
- **Groupe de négociation** : un terme du vocabulaire, avec ses dénominations ; une personne en suit zéro, un ou plusieurs ; une coordination en désigne un.
- **Suivi d'une session** : une personne, une session, le rappel armé ou non.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : Sur les données archivées, l'import écrit toutes les sessions de l'archive ; une seconde lecture identique n'écrit aucune session.
- **SC-002** : Une archive portant quatre changements produit exactement quatre écarts (sessions touchées) ; l'heure et la salle changées s'affichent « avant → après ».
- **SC-003** : Source injoignable, l'affichage se coupe à la lecture qui atteint le seuil, et plus aucune session officielle n'est servie ; il revient à la première lecture réussie.
- **SC-004** : Import éteint, aucune session n'est servie et l'écran renvoie au programme officiel.
- **SC-005** : Une négociatrice trouve la salle et l'heure de sa prochaine session en moins de dix secondes depuis l'ouverture de l'application.
- **SC-006** : Les six états d'essai (Prévue, En cours, Déplacée, Annulée, Terminée, et la déplacée de salle) se reconnaissent à l'écran sans la couleur seule.
- **SC-007** : Réseau coupé après une lecture, la liste de chaque jour de la COP, les fiches ouvertes et « Mon agenda » se relisent, chacun avec son heure de lecture.
- **SC-008** : Un ajout à l'agenda fait sans réseau arrive une seule fois au retour du réseau.
- **SC-009** : Deux sessions suivies qui se chevauchent portent chacune la marque ; une annulée n'en porte ni n'en cause.
- **SC-010** : Aucune traduction n'est demandée deux fois pour un même titre anglais, et aucun test n'appelle le fournisseur d'IA.
- **SC-011** : Toute heure de session affichée porte le fuseau de la COP, quel que soit le fuseau du téléphone.

## Assumptions

- **Une COP à la fois** : l'import vise une édition, qui donne le lieu et le fuseau ; les sessions s'y rattachent.
- **La source réelle**, non documentée (ADR-009), est le calendrier de conférence du site de la CCNUCC : une réponse JSON par COP, sans champ d'état ni document, le point de l'ordre du jour et la nature de la réunion dans le titre, derrière une protection anti-robot qui refuse un client sans navigateur ([research R2](research.md)). Son lecteur est écrit sur ce format et se branchera quand l'accord du secrétariat ouvrira un accès. Le mécanisme s'éprouve sur l'archive réelle de la COP30, rangée dans le dépôt.
- **Réglages par défaut** : une lecture toutes les cinq minutes pendant la COP, seuil de trois lectures manquées d'affilée — réglable au back-office.
- **« Déplacée »** désigne une session dont l'heure ou la salle a changé depuis sa première lecture ; elle le reste jusqu'à sa fin.
- **Le lexique** est construit en parallèle (étape 2) ; le type de réunion lui passe son texte anglais, qu'il résout. D'ici là, la feuille s'ouvre comme dans le lecteur.
- **Le rappel** n'a pas de canal hors de l'application à cette étape : ni notification poussée, ni courriel. Les notifications viendront en 3b sur le même choix.
- **La lecture est publique** : les sessions officielles sont une information publique ; « Accès limité » dit qui peut entrer dans la salle, pas qui peut lire la ligne.
- **La traduction** passe par le fournisseur d'IA d'ADR-005, par l'API, avec la clé lue dans l'environnement ; jamais depuis le téléphone.
