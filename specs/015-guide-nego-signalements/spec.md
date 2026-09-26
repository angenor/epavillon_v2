# Feature Specification: Guide Négo — signalements et notifications (étape 3b)

**Feature Branch**: `015-guide-nego-signalements`

**Created**: 2026-09-25

**Status**: Draft — trois questions tranchées le 25/09 (voir *Clarifications*)

**Input**: Étape 3b de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md) : les signalements sur les sessions de négociation (ADR-010), par-dessus l'agenda livré à l'étape 3a — signaler depuis une fiche ou une réunion non annoncée depuis la liste, en trois gestes, hors connexion, réservé aux négociatrices ; validation en un geste par un administrateur depuis son téléphone, avec « Annuler » six secondes, ou refus motivé ; encart sans nom d'auteur par-dessus la donnée officielle, retiré quand la source rattrape ; « Mes signalements » ; notifications d'un changement sur une session suivie, dans l'application et par courriel ; centre de notifications et réglage par thématique. Critère : un signalement envoyé hors connexion part au retour du réseau ; validé, il s'affiche par-dessus la donnée officielle, et la personne qui suit la session est prévenue.

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| Les écrans | [09-signaler-et-mon-agenda.html](../../docs/AppNego/design/ecrans/09-signaler-et-mon-agenda.html) — feuille « Signaler un changement » (1a), précision (1b), envoi (1c), réunion non annoncée (1d), « Mes signalements » (2a), notifications (4b) ; l'encart de [08-detail-de-session.html](../../docs/AppNego/design/ecrans/08-detail-de-session.html) (1a, 1e) ; la ligne « Non annoncée » de [07-sessions.html](../../docs/AppNego/design/ecrans/07-sessions.html) ; l'écran 1 de [11-validation.html](../../docs/AppNego/design/ecrans/11-validation.html) (file et feuille « Ne pas retenir ») ; la cloche et « 10 Notifications » de [02-socle.html](../../docs/AppNego/design/ecrans/02-socle.html), la section « Notifications par thématique » du profil, l'interrupteur « Notifications » d'« À propos » |
| Le système de design | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) et [design/passation/](../../docs/AppNego/design/passation/) ; violet `#732F85` pour « Non annoncée » et l'encart (05-design l. 38) ; le jaune ne dit que « non lu » (l. 102) |
| Les mots | [design/lexique.md](../../docs/AppNego/design/lexique.md) — « Signaler », « Valider », « Envoyé · Validé · Non retenu », « Non annoncée (signalée par le réseau) » |
| Les décisions | [ADR-010](../../docs/AppNego/adr/010-un-signalement-se-pose-par-dessus.md) un signalement se pose par-dessus · [ADR-009](../../docs/AppNego/adr/009-la-source-officielle-fait-foi.md) · [ADR-008](../../docs/AppNego/adr/008-trois-agendas-jamais-confondus.md) · [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) · [ADR-014](../../docs/AppNego/adr/014-whatsapp-est-remplace-a-terme.md) (pas de notification poussée avant Capacitor) |
| Les principes | Constitution : XI, XII (un signalement ne modifie jamais la donnée officielle), XIII |
| Ce qui est déjà livré | 0b l'accès négociateur et les courriels de décision ; 0c « Mes thématiques », la file d'écritures, le profil (sans notifications, écart 40) ; 3a l'import, la liste, la fiche, « Mon agenda », les changements constatés par l'import |

## Clarifications

### Session 2026-09-25 — tranché le 25/09 par l'orchestrateur, pour le commanditaire

- Q : Qui décide qu'un courriel part ? → R : **L'interrupteur « Notifications » d'« À propos »** : allumé par défaut, enregistré comme consentement avec la version du texte servi ; éteint, plus aucun courriel, l'application prévient toujours. **L'écart 40 se referme pour lui seul** ; « Mesures d'usage » et « Annuaire du réseau » restent non livrés.
- Q : Que règle une ligne de thématique du profil ? → R : **Elle élargit, elle ne coupe jamais** : allumée, elle ajoute les changements de toutes les sessions de la thématique hors de mon agenda et les réunions non annoncées validées de la thématique ; **éteinte par défaut**. Les sessions de mon agenda préviennent toujours.
- Q : Comment une réunion non annoncée cesse-t-elle de s'afficher ? → R : **Retrait d'un geste par l'administration, et d'office à la fin de son jour** ; aucun rapprochement automatique avec la source.
- Retouches des décisions prises seul : « Autre chose » se retire aussi d'office quand la session est terminée ; dans la liste et « Mon agenda », le repère n'est jamais la couleur seule — losange violet, mot court « Signalé », libellé lisible par les lecteurs d'écran.

**Hors périmètre** : les notifications poussées (écran verrouillé, 4a) ; les files de validation autres que les signalements (11-validation 1b à 1e, 1g) ; la preuve jointe (photo) ; la modération des échanges ; les consentements « Mesures d'usage » et « Annuaire du réseau » (écart 40 : sans effet à cette étape).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Une négociatrice signale un changement, même sans réseau (Priority: P1)

Devant la salle 3, Aïssatou voit que l'écran annonce la salle 9. Depuis la fiche, elle touche « Signaler un changement », choisit « La salle a changé », écrit « Salle 9, bâtiment B » (facultatif), puis « Envoyer ». Trois gestes. Le réseau est tombé : le signalement part au retour du réseau, une seule fois. La fiche garde « Votre signalement — envoyé à 11:20, en vérification ».

**Why this priority**: c'est l'entrée de toute l'étape ; sans signalement, rien à valider ni à notifier.

**Independent Test**: avec un compte admis, signaler réseau coupé, rétablir le réseau, vérifier une seule ligne en file de validation.

**Acceptance Scenarios**:

1. **Given** une personne admise (accès négociateur), **When** elle touche « Signaler un changement » sur une fiche, **Then** la feuille propose « La session est annulée », « L'heure a changé », « La salle a changé », « Autre chose » ; chaque choix ouvre la précision facultative, et « Envoyer » est actif sans rien remplir.
2. **Given** sans réseau, **When** elle envoie, **Then** « Signalement envoyé. Vérification en cours. » s'affiche, le signalement paraît dans « Mes signalements » comme envoyé, et part au retour du réseau ; renvoyé deux fois par la file, il n'existe qu'une fois.
3. **Given** une personne connectée sans accès négociateur, ou sans compte, **When** elle touche « Signaler un changement », **Then** l'application dit que le signalement est réservé aux négociatrices et négociateurs et mène à l'accès, sans perdre la fiche.
4. **Given** depuis le bas de la liste d'un jour, **When** elle touche « Signaler une réunion non annoncée », **Then** elle remplit « Quoi » (requis), « Où », « Quand », une thématique, et envoie.
5. **Given** un signalement déjà envoyé et non tranché par la même personne sur la même session, **When** elle en envoie un second, **Then** l'application le refuse en le disant.

---

### User Story 2 — Un administrateur valide ou refuse en un geste, depuis son téléphone (Priority: P1)

Dans Guide Négo, l'administratrice ouvre « Signalements » : chaque carte donne l'autrice (nom, pays), le motif, la précision, la session, et ce que dit la source officielle **à cet instant**. Elle touche « Valider » : « Validé. Affiché à toutes et tous. » avec « Annuler » pendant six secondes. Ou « Ne pas retenir » : trois motifs (« La source officielle la maintient », « Déjà pris en compte », « Pas assez précis ») et une précision facultative, que l'autrice verra.

**Why this priority**: rien ne s'affiche sans validation (ADR-010) ; c'est la garde de la confiance.

**Independent Test**: valider un signalement, vérifier l'encart ; valider puis annuler dans les six secondes, vérifier qu'aucun encart ne reste et qu'aucune notification n'est partie ; refuser, vérifier le motif chez l'autrice.

**Acceptance Scenarios**:

1. **Given** la permission de valider sur la portée globale, **When** l'administratrice ouvre Guide Négo, **Then** une entrée « Signalements » avec le nombre à traiter est visible ; sans elle, rien de la validation n'apparaît.
2. **Given** un signalement à traiter, **Then** la carte montre la valeur officielle de la session lue à la dernière lecture de l'import, avec son heure.
3. **Given** « Valider » puis « Annuler » dans les six secondes, **Then** le signalement redevient à traiter, aucun encart n'a été vu comme durable et **aucune notification n'est partie**.
4. **Given** « Valider » sans annulation, **Then** l'encart s'affiche et les personnes concernées sont prévenues.
5. **Given** « Ne pas retenir » avec un motif, **Then** l'autrice voit « Non retenu à HH:MM » et « Motif : … » dans « Mes signalements ».

---

### User Story 3 — Le signalement validé se pose par-dessus la donnée officielle (Priority: P1)

Sur la fiche, sous l'état officiel, un encart violet dit « Signalé par le réseau — validé par l'IFDD à 11:12 » et ce qui change ; la section « Source officielle » reste entière dessous. Une session que la source dit « Prévue » et qu'un signalement validé dit « Annulée » montre les deux, datées. Une réunion non annoncée validée paraît dans la liste, « Non annoncée — signalée par le réseau, validée à 11:12 ». Quand la source rattrape le changement, l'encart se retire seul.

**Why this priority**: c'est la raison d'être du signalement ; et la règle qui ne doit jamais céder : la donnée officielle n'est jamais modifiée.

**Independent Test**: valider un changement de salle ; lire une archive où la source donne la même salle ; vérifier que l'encart disparaît et que la ligne officielle n'a jamais été touchée par le signalement.

**Acceptance Scenarios**:

1. **Given** un changement validé, **Then** la fiche montre l'encart sans nom d'auteur, et la liste comme « Mon agenda » marquent la ligne d'un signe « signalé par le réseau ».
2. **Given** l'import lit une valeur égale à celle du signalement validé (même salle, même heure, session annulée), **Then** l'encart se retire, et le signalement reste « Validé » chez son autrice.
3. **Given** l'affichage des sessions officielles coupé (lecture impossible), **Then** les réunions non annoncées validées restent affichées, avec la phrase de la maquette ; les encarts sur des sessions officielles disparaissent avec elles.
4. **Given** hors connexion, **Then** l'encart et les réunions non annoncées lus avant la coupure restent lisibles, avec l'heure de lecture.
5. **Given** la session officielle terminée, **Then** l'encart n'est plus affiché.

---

### User Story 4 — Être prévenue d'un changement sur une session suivie (Priority: P2)

Aïssatou suit « Groupe de contact sur le plan d'action genre ». L'import constate son déplacement ; ou un signalement validé l'annonce. La cloche de l'en-tête porte un compteur jaune ; le centre « Notifications » liste « Groupe de contact … — déplacé à 15:00, salle 9 », origine et heure, non lue en gras avec son carré jaune ; un toucher ouvre la fiche. Un courriel part aussi, un seul pour plusieurs changements rapprochés sur la même session.

**Why this priority**: le critère de l'étape ; mais il repose sur US1 à US3 et sur 3a.

**Independent Test**: suivre une session, faire lire à l'import une archive qui la déplace, vérifier la notification et le courriel (Mailpit) ; deux changements rapprochés → un courriel.

**Acceptance Scenarios**:

1. **Given** une session dans « Mon agenda », **When** l'import y constate un changement d'heure, de salle ou une annulation, **Then** une notification paraît dans le centre, et un courriel part selon FR-030.
2. **Given** un signalement validé sur une session suivie, **Then** la même notification part, marquée « signalé par le réseau ».
3. **Given** deux changements sur la même session à quelques minutes, **Then** un seul courriel, qui dit l'état final.
4. **Given** « Tout marquer comme lu », **Then** le compteur tombe à zéro.
5. **Given** hors connexion, **Then** le centre montre ce qui a été lu, avec l'heure de lecture.

---

### User Story 5 — « Mes signalements » et le réglage des notifications (Priority: P3)

Aïssatou retrouve ses signalements, du plus récent au plus ancien, chacun « Envoyé », « Validé » ou « Non retenu » avec son motif. Dans son profil, « Notifications par thématique » règle ce qui la prévient ; dans « À propos », l'interrupteur « Notifications » prend enfin effet.

**Independent Test**: trois signalements dans les trois états ; changer un réglage et vérifier son effet sur la notification suivante.

**Acceptance Scenarios**:

1. **Given** des signalements, **Then** « Mes signalements » les liste avec leur état, leur heure et, s'ils sont non retenus, le motif ; on y accède depuis le message d'envoi et depuis le profil.
2. **Given** l'interrupteur « Notifications » éteint, **Then** plus aucun courriel, l'application prévient toujours ; **Given** la ligne « Adaptation » allumée, **Then** un changement d'une session d'Adaptation hors de l'agenda prévient aussi.

---

### Edge Cases

- Une session officielle disparaît (annulée par l'import) pendant qu'un signalement l'attend : la carte de validation montre l'état officiel du moment.
- Deux négociatrices signalent la même chose : deux signalements ; valider l'un n'efface pas l'autre, que l'administratrice classe « Déjà pris en compte ».
- Un signalement validé contredit un changement importé plus tard : la source prime dans « Source officielle », l'encart reste jusqu'au rattrapage ou à la fin de la session.
- Une réunion non annoncée sans heure : elle paraît sans heure en fin de journée, marquée « Non annoncée ».
- Une réunion non annoncée que la source finit par publier : voir FR-021.
- L'administratrice valide sans réseau : le geste est refusé, avec un message clair (la validation n'est pas mise en file).
- Le compte de l'autrice perd l'accès négociateur : ses signalements déjà validés restent affichés, sans son nom de toute façon.
- La file d'écritures rejoue un signalement déjà reçu : aucune seconde ligne, réponse de succès.
- Une personne se retire de l'agenda : elle n'est plus prévenue des changements de cette session.

## Requirements *(mandatory)*

### Signaler

- **FR-001** : « Signaler un changement » DOIT être offert sur toute fiche de session officielle, y compris annulée par la source, et « Signaler une réunion non annoncée » en bas de la liste d'un jour.
- **FR-002** : Signaler DOIT être réservé aux personnes qui ont l'accès négociateur (0b) ; les autres voient le geste et sont menées à l'accès.
- **FR-003** : Un signalement de changement DOIT porter un motif parmi « La session est annulée », « L'heure a changé », « La salle a changé », « Autre chose », une valeur proposée facultative (nouvelle heure, nouvelle salle) et une précision facultative de 600 caractères au plus.
- **FR-004** : Une réunion non annoncée DOIT porter « Quoi » (requis), « Où », « Quand » (heure de début, jour choisi dans la liste) et une thématique facultative.
- **FR-005** : Envoyer DOIT tenir en trois gestes au plus, sans boîte de confirmation ; la phrase « Un administrateur vérifie avant d'afficher. Votre nom n'apparaît pas aux autres. » tient lieu de confirmation.
- **FR-006** : Un signalement fait sans réseau DOIT se voir aussitôt comme envoyé et partir au retour du réseau ; une référence posée par le téléphone garantit qu'un envoi rejoué ne crée jamais deux signalements.
- **FR-007** : Un second signalement d'une même personne sur une même session, tant que le premier n'est pas tranché, DOIT être refusé avec un message qui le dit.
- **FR-008** : La fiche DOIT montrer « Votre signalement — envoyé à HH:MM, en vérification » tant qu'il n'est pas tranché.

### Valider

- **FR-009** : Valider et refuser DOIVENT être réservés à une permission propre, sur la portée globale ; l'écran vit dans Guide Négo, avec son design, et n'apparaît qu'à qui la porte.
- **FR-010** : La file DOIT montrer, pour chaque signalement à traiter : l'autrice (nom, pays), le motif, la valeur proposée, la précision, l'heure, la session et ce que dit la source officielle à cet instant avec son heure de lecture ; les plus anciens d'abord ; puis les traités du jour.
- **FR-011** : « Valider » DOIT agir en un geste et offrir « Annuler » six secondes ; annulé, le signalement redevient à traiter et **aucune notification liée ne part, jamais**. La validation ne devient publique qu'à sa publication, une trentaine de secondes plus tard ; le message le dit (« Validé. Affiché dans une minute au plus. »).
- **FR-012** : « Ne pas retenir » DOIT demander un motif parmi trois et une précision facultative ; l'autrice voit l'un et l'autre.
- **FR-013** : La validation NE DOIT PAS se mettre en file hors connexion : sans réseau, le geste est refusé et le dit.
- **FR-014** : L'historique DOIT dire qui a signalé, qui a validé ou refusé, et quand (ADR-010) ; il n'est lu qu'au back-office et par l'administration.

### Afficher par-dessus

- **FR-015** : Un signalement NE DOIT JAMAIS modifier une session importée ni ses changements constatés (principe XII) ; il est un objet à part, lié à la session.
- **FR-016** : Un changement validé DOIT s'afficher sur la fiche dans un encart violet, sous l'état officiel et au-dessus de « Source officielle » : « Signalé par le réseau — validé par l'IFDD à HH:MM », le motif et la valeur proposée, la précision — **sans nom d'auteur**.
- **FR-017** : La liste et « Mon agenda » DOIVENT marquer la ligne d'une session qui porte un encart d'un repère **pictogramme + mot + couleur** — losange violet et « Signalé », avec un libellé complet pour les lecteurs d'écran (« Signalé par le réseau, validé par l'IFDD à HH:MM ») ; aucun texte d'encart dans la liste.
- **FR-018** : Une réunion non annoncée validée DOIT paraître dans la liste de son jour, à son heure, marquée « Non annoncée — signalée par le réseau, validée à HH:MM », avec sa fiche ; elle n'est jamais présentée comme une session officielle, et peut s'ajouter à « Mon agenda ».
- **FR-019** : L'encart DOIT se retirer seul quand l'import lit une valeur qui le rattrape — session annulée pour « annulée », même heure de début pour « l'heure a changé », même salle pour « la salle a changé » ; « Autre chose » ne se rattrape pas automatiquement.
- **FR-020** : Tout encart — « Autre chose » compris — DOIT cesser de s'afficher d'office quand la session est terminée ; l'administration PEUT retirer un encart à tout moment.
- **FR-021** : Une réunion non annoncée DOIT cesser de s'afficher quand l'administration la retire d'un geste, ou d'office à la fin de son jour dans le fuseau de la COP ; aucun rapprochement automatique avec la source.
- **FR-022** : Coupure : les encarts sur les sessions officielles disparaissent avec elles ; les réunions non annoncées validées restent, avec « Le signalement du réseau ne vient pas du programme officiel : il reste affiché. »
- **FR-023** : Hors connexion, encarts et réunions non annoncées lus DOIVENT rester lisibles avec l'heure de lecture ; ce qui a été validé après ne s'affiche pas.

### Notifier

- **FR-024** : Un changement d'heure, de salle ou une annulation constaté par l'import sur une session DOIT produire une notification pour chaque personne qui la suit (dans « Mon agenda »).
- **FR-025** : Un signalement validé DOIT produire la même notification, marquée « signalé par le réseau », pour les personnes qui suivent la session ; une réunion non annoncée validée, pour les personnes dont la ligne de sa thématique est allumée (FR-031).
- **FR-026** : L'autrice DOIT être prévenue dans l'application de la décision sur son signalement (validé, non retenu).
- **FR-027** : Le centre de notifications DOIT s'ouvrir par la cloche de l'en-tête (compteur jaune des non lues) : liste par jour, texte, origine et heure, non lue en gras avec un carré jaune, un toucher ouvre la fiche et marque lu, « Tout marquer comme lu », état vide, hors connexion avec l'heure de lecture.
- **FR-028** : Une notification DOIT commencer par l'état (« Déplacée — », « Annulée — ») et finir par l'agenda d'origine (« Sessions de négociation »).
- **FR-029** : Plusieurs changements rapprochés sur une même session NE DOIVENT produire qu'un courriel, qui dit l'état final.
- **FR-030** : Le courriel DOIT partir pour les notifications de changement (FR-024, FR-025) **si et seulement si** l'interrupteur « Notifications » d'« À propos » est allumé — allumé par défaut, enregistré comme consentement avec la version du texte servi ; son effet est dit sous l'interrupteur. La décision sur un signalement (FR-026) ne part pas par courriel.
- **FR-031** : Le profil DOIT porter « Notifications par thématique » : une ligne par thématique suivie, **éteinte par défaut** ; allumée, elle ajoute les changements de toutes les sessions de cette thématique et les réunions non annoncées validées de cette thématique. Elle ne coupe jamais les sessions de « Mon agenda ».
- **FR-032** : Aucune notification poussée ; aucun texte ne promet une alerte téléphone fermé.

### Ce qui vaut pour toute l'étape

- **FR-033** : Les motifs de signalement et de refus sont des listes fermées du code, libellées par l'interface ; les thématiques, types et salles viennent de la base.
- **FR-034** : Toute heure d'événement porte le fuseau de la COP ; « lu à » l'heure du téléphone sans fuseau (écart 32).
- **FR-035** : Aucune écriture n'est perdue : décision, notification et courriel naissent dans la même transaction que ce qui les cause, ou d'un travail qui relit l'état au moment d'agir.

### Key Entities

- **Signalement** : autrice, session (ou réunion non annoncée), motif, valeur proposée, précision, référence client, état (envoyé, validé, non retenu), décideur, motif de refus, heures ; instantané de la source à la décision ; heure de publication ; retrait (rattrapé, retiré) et son heure — la fin de la session se calcule.
- **Réunion non annoncée** : quoi, où, quand, jour, thématique, née d'un signalement validé, d'origine « réseau », distincte des sessions officielles.
- **Notification** : destinataire, type, texte, lien vers la fiche, lue ou non, clé de regroupement par session.
- **Réglage de notification** : par personne et thématique, et l'accord aux notifications.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : Un signalement envoyé hors connexion arrive une seule fois au retour du réseau, même rejoué.
- **SC-002** : Signaler tient en trois gestes au plus.
- **SC-003** : Après validation, la ligne officielle et ses changements constatés sont identiques, octet pour octet, à ce qu'ils étaient avant.
- **SC-004** : Valider puis annuler dans les six secondes n'envoie aucune notification ni courriel.
- **SC-005** : Une personne qui suit une session reçoit la notification d'un changement importé ou validé dans la minute qui suit.
- **SC-006** : Deux changements sur une même session dans la même tranche de dix minutes produisent un seul courriel par personne.
- **SC-007** : L'encart d'un changement se retire à la première lecture de l'import qui le rattrape.
- **SC-008** : Aucun nom d'autrice n'est lisible par une personne sans la permission de valider.
- **SC-009** : Le centre de notifications, « Mes signalements », les encarts et les réunions non annoncées se relisent hors connexion avec leur heure de lecture.

## Assumptions

- « Suivre une session » = l'avoir dans « Mon agenda » (3a).
- « Validé par l'IFDD » partout — la maquette dit parfois « un expert » ; c'est l'administration de l'IFDD qui valide (lexique : l'expert valide le fond, l'administrateur gère).
- Les courriels suivent le patron de 0b : composés dans le module, envoyés par la file de travaux, gardés par la liste de suppression.
- La validation se fait sur la portée globale, comme toute l'administration de Guide Négo (tranché le 21/09).
- Les motifs de signalement et de refus sont fermés à cette étape ; le texte libre les complète.
