# Feature Specification: Socle technique et Identité (B1)

**Feature Branch**: `001-socle-identite`

**Created**: 2026-08-20

**Status**: Draft

**Input**: User description: "Module SOCLE + IDENTITÉ de l'API ePavillon v2. Kernel (contexte de requête, erreurs, i18n, bus d'événements), relais d'outbox, file de travaux. Authentification Argon2id, jetons d'accès courts, jetons de rafraîchissement hachés et révocables, vérification d'adresse, réinitialisation. RBAC scopé et filtrage par périmètre d'administration. Traiter les écarts n° 18, 19 et 20 relevés en écrivant l'écran A1, et l'ordre imposé des contrôles de connexion. Livrable : `backend/crates/modules/identity`, plus `kernel`, `contracts`, `api` et `worker`, qui n'existent pas encore."

---

## Contexte

Le modèle de données est complet et fait autorité — `docs/database/030_identity.sql` (comptes, sessions, jetons, RBAC scopé, RGPD) et `docs/database/010_platform.sql` (audit, outbox, file de travaux, drapeaux, registre des modules). **Aucune modification du SQL n'est proposée ici** : l'écran A1 avait déjà constaté que `030_identity.sql` prévoyait tout.

Le front existe depuis le 17/08 et consomme des données simulées. Ses contrats — `frontend/app/types/auth.ts`, `frontend/app/types/identity.ts`, `frontend/app/types/admin-users.ts` — et les chemins déclarés dans `frontend/app/composables/useApi.ts` **sont le contrat de cette API**. Ils ne se renégocient pas.

Cette spécification est la première du cycle B : elle crée l'arborescence `backend/` que les cinq modules suivants réutiliseront.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Se connecter sans rien apprendre à un inconnu (Priority: P1)

Une personne saisit son adresse et son mot de passe. Si les deux sont justes, elle entre. Sinon, elle reçoit **exactement le même refus**, que l'adresse existe ou non : le formulaire de connexion ne doit jamais servir d'annuaire des comptes.

Ce n'est qu'**après** un mot de passe correct que la plateforme accepte d'en dire plus : compte verrouillé, personne suspendue, adresse non vérifiée. À ce moment-là, elle parle à quelqu'un qui vient de prouver son identité, et ne lui apprend donc rien qu'il ne sache déjà.

**Why this priority**: c'est la porte d'entrée de toute la plateforme, et la seule règle du module dont une erreur se paie en fuite d'information plutôt qu'en gêne d'usage. L'écran existe déjà et attend ces six issues.

**Independent Test**: exercer les six issues de connexion sur un jeu de comptes couvrant chaque cas, et vérifier que le refus « identifiants invalides » est identique au caractère près pour une adresse inconnue et pour un mot de passe faux.

**Acceptance Scenarios**:

1. **Given** une adresse inconnue de la plateforme, **When** on tente de se connecter, **Then** la réponse est « identifiants invalides » et rien d'autre — même code, même message, même durée apparente que pour un mot de passe faux.
2. **Given** un compte existant et un mot de passe faux, **When** on tente de se connecter, **Then** la réponse est « identifiants invalides », et le compteur d'échecs du compte est incrémenté.
3. **Given** un compte dont le verrou court encore, **When** on se connecte avec le **bon** mot de passe, **Then** la réponse est « verrouillé » et **porte la date de fin du verrou**, que l'écran affiche.
4. **Given** une personne suspendue et son bon mot de passe, **When** elle se connecte, **Then** la réponse est « suspendu », accompagnée de la date de fin de suspension quand elle existe.
5. **Given** un compte dont l'adresse n'a jamais été vérifiée et son bon mot de passe, **When** il se connecte, **Then** la réponse est « adresse non vérifiée » et porte l'adresse visée, **et aucune session n'est ouverte**.
6. **Given** un compte valide, vérifié, non verrouillé, d'une personne active, **When** il se connecte, **Then** une session s'ouvre et la fiche de la personne est rendue.

---

### User Story 2 — Rester connecté, et pouvoir couper une session (Priority: P1)

Une personne connectée navigue sans se ré-authentifier à chaque écran. Sa session se prolonge d'elle-même tant qu'elle est vivante, et s'interrompt quand elle se déconnecte — sur cet appareil, ou sur tous.

**Why this priority**: sans session, aucun autre écran du back-office n'est atteignable. C'est la dépendance de tous les modules suivants.

**Independent Test**: se connecter, appeler une route protégée, laisser le jeton d'accès expirer, constater que la session se renouvelle seule ; se déconnecter, constater que le même jeton de rafraîchissement ne vaut plus rien.

**Acceptance Scenarios**:

1. **Given** une connexion réussie, **When** le client appelle la route « qui suis-je », **Then** la fiche de la personne connectée est rendue sans qu'aucun identifiant ne circule dans l'URL ni dans le corps.
2. **Given** un jeton d'accès expiré et un jeton de rafraîchissement valide, **When** le client rejoue son appel, **Then** la session se renouvelle et l'appel aboutit.
3. **Given** une session, **When** la personne se déconnecte, **Then** la session est révoquée, motif à l'appui, et toute tentative de renouvellement échoue.
4. **Given** un jeton de rafraîchissement déjà utilisé, **When** il est rejoué, **Then** le renouvellement est refusé **et toutes les sessions de la personne sont révoquées** — un jeton rejoué est le signe d'un vol.
5. **Given** une personne qui coche « rester connecté », **When** elle se connecte, **Then** sa session vit trente jours ; sans la case, douze heures.
6. **Given** une base contenant les empreintes de session, **When** on l'inspecte, **Then** **aucun jeton utilisable n'y figure** — seules des empreintes, inexploitables telles quelles.

---

### User Story 3 — Ne voir que le périmètre qu'on m'a confié (Priority: P1)

Un administrateur détaché sur une seule édition ouvre le back-office. Il y voit ses propositions, ses organisations, ses incidents, ses utilisateurs — et rien d'une autre COP, **y compris s'il forge l'adresse d'une page**.

Une personne sans aucun droit d'administration reçoit un **refus explicite**, jamais une liste vide.

**Why this priority**: c'est la règle métier n° 8, celle que la v1 n'a pas su tenir — elle avait dû construire une seconde page d'administration codée en dur pour le responsable d'un webinaire. Neuf écrans du back-office l'attendent.

**Independent Test**: avec un compte détaché sur une édition, appeler chaque liste du back-office, puis forger l'identifiant d'une autre édition dans l'URL et constater le refus.

**Acceptance Scenarios**:

1. **Given** une personne dont le périmètre est global, **When** elle demande une liste du back-office, **Then** elle reçoit toutes les éditions.
2. **Given** une personne détachée sur l'édition A, **When** elle demande une liste, **Then** elle ne reçoit que l'édition A.
3. **Given** cette même personne, **When** elle demande explicitement l'édition B par son identifiant, **Then** elle reçoit un **refus d'accès**, indiscernable de la réponse rendue pour un identifiant inexistant.
4. **Given** une personne sans aucun droit d'administration, **When** elle demande une liste du back-office, **Then** elle reçoit un **refus d'accès**, et non une liste vide.
5. **Given** une autorisation à vérifier, **When** le service la teste, **Then** il teste une **permission avec sa portée**, jamais un nom de rôle.

---

### User Story 4 — Créer un compte et prouver son adresse (Priority: P2)

Une personne s'inscrit avec le strict minimum : prénom, nom, adresse, pays, mot de passe. Elle reçoit un courriel, suit le lien, son adresse est vérifiée, elle peut se connecter.

Si l'adresse était déjà connue, **la réponse à l'écran est exactement la même** : c'est le courriel envoyé qui diffère — un rappel « vous avez déjà un compte » au lieu d'un lien de vérification.

**Why this priority**: l'appel à propositions de la COP31 s'ouvre à des organisations qui n'ont pas encore de compte. Elle vient après la connexion parce qu'un jeu de comptes existe déjà en base.

**Independent Test**: s'inscrire avec une adresse libre, puis avec une adresse déjà prise, et comparer les deux réponses ; suivre le lien reçu et constater la vérification.

**Acceptance Scenarios**:

1. **Given** une adresse libre, **When** on s'inscrit, **Then** la réponse annonce l'envoi d'un lien de vérification, la personne est créée sans compte utilisable et **aucune session n'est ouverte**.
2. **Given** une adresse déjà connue, **When** on s'inscrit, **Then** la réponse est **identique** à la précédente, et aucun compte n'est modifié.
3. **Given** un lien de vérification valide, **When** on le suit, **Then** l'adresse est marquée vérifiée, le jeton est consommé, et la réponse porte l'adresse.
4. **Given** un lien déjà utilisé, **When** on le suit, **Then** la réponse dit « déjà utilisé » — et non « expiré », même si le jeton a périmé depuis : le travail est fait, il n'y a rien à redemander.
5. **Given** un lien périmé, **When** on le suit, **Then** la réponse dit « expiré », et l'écran peut proposer un renvoi.
6. **Given** un jeton inventé de toutes pièces, **When** on le soumet, **Then** la réponse dit « invalide ».
7. **Given** une demande de renvoi de lien, **When** elle est faite pour une adresse quelconque, **Then** la réponse est invariable, et un nouveau lien n'est envoyé que si l'adresse existe et n'est pas vérifiée.

---

### User Story 5 — Retrouver l'accès à un compte (Priority: P2)

Une personne a oublié son mot de passe. Elle demande un lien, le reçoit, en choisit un nouveau. La demande elle-même ne dit jamais si l'adresse est connue.

**Why this priority**: sans elle, tout compte perdu devient une demande d'assistance. Elle réutilise l'essentiel de la mécanique de jetons de l'histoire précédente.

**Independent Test**: demander une réinitialisation pour une adresse inconnue puis pour une adresse connue, comparer les réponses ; mener un cycle complet et vérifier que l'ancien mot de passe ne fonctionne plus.

**Acceptance Scenarios**:

1. **Given** une adresse inconnue, **When** on demande une réinitialisation, **Then** la réponse est la même que pour une adresse connue.
2. **Given** un jeton de réinitialisation valide, **When** on le contrôle avant d'afficher le formulaire, **Then** la réponse le déclare valide et porte l'adresse concernée.
3. **Given** un jeton valide et un nouveau mot de passe conforme, **When** on l'enregistre, **Then** le mot de passe est remplacé, le jeton consommé, **le compteur d'échecs et le verrou remis à zéro**, et **toutes les sessions de la personne révoquées**.
4. **Given** un formulaire ouvert la veille au soir, **When** il est validé le lendemain matin avec un jeton entre-temps périmé, **Then** l'enregistrement est refusé pour péremption — le jeton est revérifié à l'envoi, pas seulement à l'affichage.
5. **Given** un mot de passe trop court ou sans majuscule, **When** on l'enregistre, **Then** l'erreur **désigne le champ fautif** et porte un message en français.

---

### User Story 6 — Confier un rôle, le retirer, sur la bonne portée (Priority: P2)

Un administrateur attribue un rôle à une personne, sur une portée : la plateforme entière, une organisation, une édition, un espace de négociation. Il peut aussi le retirer, en disant pourquoi.

Un administrateur détaché sur une édition **ne peut attribuer que là**, et ne peut retirer que ce qu'il aurait pu accorder.

**Why this priority**: l'écran A12 existe et l'attend. C'est aussi ce qui permet de peupler les périmètres que l'histoire n° 3 fait respecter.

**Independent Test**: avec un compte détaché sur une édition, attribuer un rôle sur cette édition (accepté), puis sur une autre et globalement (refusés) ; retirer un rôle global (refusé).

**Acceptance Scenarios**:

1. **Given** un administrateur global, **When** il attribue un rôle sur n'importe quelle portée autorisée pour ce rôle, **Then** l'attribution est enregistrée avec son auteur, sa date et son motif d'octroi.
2. **Given** un administrateur détaché sur l'édition A, **When** il attribue un rôle sur l'édition B ou globalement, **Then** l'écriture est refusée.
3. **Given** un rôle qui n'admet pas la portée visée, **When** on tente l'attribution, **Then** l'erreur rendue est **la traduction du refus de la base**, pas une règle recopiée dans le service.
4. **Given** une attribution en cours, **When** un administrateur la retire, **Then** l'attribution est marquée révoquée avec **son auteur et son motif**, et n'est jamais supprimée.
5. **Given** une écriture de rôle, **When** le client déclare lui-même de quels droits il dispose, **Then** cette déclaration est **ignorée** : l'API lit sa propre session.
6. **Given** une personne, **When** on demande ses permissions effectives, **Then** chaque permission est rendue **avec sa portée**, et non comme une simple liste de codes.

---

### User Story 7 — Les effets différés partent une fois, et une seule (Priority: P2)

Un changement d'état de la plateforme annonce ce qu'il vient de faire, et cette annonce part avec le changement — pas avant, pas à la place. Un relais la reprend et la remet à qui doit agir. Un relais qui redémarre ne fait pas partir deux fois le même courriel.

**Why this priority**: c'est la mécanique par laquelle les modules suivants communiqueront sans jamais s'appeler entre eux. Sa mise à l'épreuve la plus simple est ici : le courriel de vérification d'adresse.

**Independent Test**: provoquer une inscription, constater l'événement écrit dans la même transaction, arrêter et relancer le relais, vérifier qu'un seul courriel est parti.

**Acceptance Scenarios**:

1. **Given** un changement d'état qui doit s'annoncer, **When** la transaction est validée, **Then** l'événement est présent, avec l'acteur et l'identifiant de requête ; **When** elle est annulée, **Then** aucun événement ne subsiste.
2. **Given** un relais arrêté puis relancé sur des événements déjà traités, **When** il reprend, **Then** aucun effet n'est produit une seconde fois.
3. **Given** un travail différé qui échoue, **When** il est repris, **Then** il est replanifié avec un délai croissant, puis mis de côté au-delà du nombre d'essais autorisé.
4. **Given** deux demandes du même travail, **When** elles portent la même clé d'unicité, **Then** une seule est exécutée.
5. **Given** toute écriture de l'API, **When** elle a lieu, **Then** l'auteur et l'identifiant de la requête sont posés **avant** la première modification — un audit anonyme est un échec, même si rien ne casse.

---

### User Story 8 — Honorer une demande RGPD (Priority: P3)

Une personne demande l'export, la rectification ou l'effacement de ses données. Un administrateur de la plateforme voit la file des demandes et leur échéance, et les traite. L'effacement purge l'identité et conserve les agrégats de participation.

**Why this priority**: obligation réglementaire réelle, mais qui ne bloque pas l'ouverture de l'appel à propositions. L'écran existe déjà.

**Independent Test**: déposer une demande de chaque type, constater l'échéance à trente jours, exécuter un effacement et vérifier que les compteurs de participation ne bougent pas.

**Acceptance Scenarios**:

1. **Given** une demande déposée, **When** elle est enregistrée, **Then** son échéance réglementaire est posée à trente jours.
2. **Given** la file des demandes, **When** un administrateur **détaché sur une édition** la consulte, **Then** l'accès est refusé : cette file **exige la portée globale** et ne se borne pas par édition.
3. **Given** une demande d'**effacement**, **When** elle est exécutée, **Then** l'identité est purgée, les comptes et adresses supprimés, les sessions révoquées, et un événement d'anonymisation est émis.
4. **Given** une demande d'**export** ou de **rectification**, **When** on tente d'y appliquer l'effacement, **Then** l'opération est refusée — anonymiser qui ne demandait qu'une copie détruirait son identité.
5. **Given** une personne anonymisée, **When** on consulte les statistiques de participation d'une édition passée, **Then** les compteurs sont inchangés.

---

### Edge Cases

- **Une adresse inconnue coûte-t-elle moins de temps qu'un mot de passe faux ?** Si le service abandonne avant de calculer une empreinte, la différence de durée devient un oracle d'existence de comptes. Le coût de vérification doit être payé dans les deux cas.
- **Un compte verrouillé dont le verrou vient d'expirer.** Le compteur d'échecs doit repartir de zéro, sans quoi le sixième échec de la semaine reverrouille aussitôt.
- **Une personne sans compte mot de passe** — invité inscrit à un webinaire, intervenant saisi par un tiers. Elle existe dans la plateforme mais ne peut pas se connecter : la réponse est « identifiants invalides », jamais « ce compte n'a pas de mot de passe ».
- **Une personne `blocked`.** Le contrat du front ne prévoit que `suspended` ; l'exclusion durable emprunte donc la même issue, sans date de fin.
- **Un jeton à la fois consommé et périmé.** « Déjà utilisé » l'emporte sur « expiré » : le premier dit que le travail est fait, le second envoie redemander un courriel inutile.
- **Deux liens de vérification demandés coup sur coup.** Le plus récent doit valoir ; les précédents, pour la même finalité et la même personne, sont invalidés.
- **Une adresse déjà vérifiée qui demande un renvoi de lien.** La réponse reste invariable, mais aucun courriel ne part.
- **Un rôle attribué avec une date de fin déjà passée**, ou une portée dont la cible a été supprimée dans un autre module — l'attribution devient orpheline sans qu'aucune clé étrangère ne le signale (c'est un choix du modèle : la portée n'en porte pas).
- **Un administrateur qui se retire à lui-même son dernier rôle.** Rien ne l'en empêche en base ; il faut décider si l'API le laisse faire.
- **Le dernier super-administrateur.** Son rôle est système et non supprimable, mais son **attribution** l'est.
- **La personne connectée est suspendue pendant sa session.** Une session ouverte ne doit pas survivre à la suspension.
- **Un module marqué désactivé dans le registre en base.** Ses routes ne sont pas montées au démarrage ; une requête vers elles répond « inconnu », pas « interdit ».
- **La base est absente à la compilation.** Les requêtes étant vérifiées à la compilation, la chaîne d'intégration doit démarrer une base avant de construire.

---

## Requirements *(mandatory)*

### Le socle partagé

- **FR-001**: Le dépôt DOIT porter une arborescence `backend/` avec cinq emplacements : le noyau technique, les contrats d'événements, le binaire HTTP, le binaire des travaux différés, et un dossier de modules métier. Ces emplacements sont ceux qu'impose la constitution et ne se réinventent pas.
- **FR-002**: Un crate de module NE DOIT dépendre que du noyau et des contrats — jamais d'un autre crate de module. Le graphe de dépendances DOIT être vérifiable mécaniquement.
- **FR-003**: Le noyau DOIT porter un **contexte de requête** transportant au minimum l'identifiant de requête, l'acteur authentifié quand il existe, et la locale demandée.
- **FR-004**: Toute transaction en écriture DOIT positionner l'acteur et l'identifiant de requête **avant** sa première modification, de sorte que l'audit et l'historique par champ soient exploitables.
- **FR-005**: Le noyau DOIT porter un **type d'erreur unique** rendant, pour chaque échec : un code stable (identifiant machine, jamais traduit), un message en français destiné à l'affichage, et, pour une erreur de validation, le champ fautif.
- **FR-006**: Les erreurs de la base — vérification, unicité, exclusion, refus de trigger — DOIVENT être **traduites** en erreurs d'API à partir de leur code SQL et du nom de la contrainte, jamais anticipées par une revalidation applicative.
- **FR-007**: Le noyau DOIT résoudre les textes multilingues de la base selon la langue demandée par la requête, avec repli sur le français. Les libellés de données ne DOIVENT jamais être recopiés dans des fichiers de traduction.
- **FR-008**: Les effets de bord inter-modules DOIVENT passer par l'émission d'un événement **dans la même transaction** que le changement d'état. L'insertion directe dans la table d'événements est interdite ; le type d'événement respecte la forme à trois segments imposée par la base.
- **FR-009**: Un **relais** DOIT reprendre les événements non publiés, les remettre à leurs consommateurs, et se réveiller sur notification sans attendre son prochain balayage. Il DOIT réserver les lignes sans se bloquer entre instances.
- **FR-010**: Tout consommateur DOIT se garder du rejeu en traçant le couple (consommateur, événement) déjà traité.
- **FR-011**: Une **file de travaux** DOIT permettre de différer un travail qui n'annonce pas un changement d'état, avec réservation atomique par lot, replanification à délai croissant en cas d'échec, mise de côté au-delà du nombre d'essais, et clé d'unicité dès qu'un doublon serait visible par un utilisateur.
- **FR-012**: Au démarrage, l'API DOIT lire le **registre des modules** en base : un module désactivé n'est pas monté.
- **FR-013**: L'API DOIT exposer une route de santé décrivant l'état d'exploitation du système, et une route de vivacité utilisable par l'orchestrateur.

### Réglages d'exploitation — écarts n° 18 et 19

- **FR-014**: Le **seuil de verrouillage** et la **durée du verrou** DOIVENT être déclarés dans la configuration du service, **et non en base** — ce sont des réglages d'exploitation, pas des invariants de données. Valeurs par défaut : **5 échecs consécutifs**, **15 minutes**.
- **FR-015**: Le compteur d'échecs DOIT être remis à zéro par : une connexion réussie, l'expiration du verrou, et une réinitialisation de mot de passe menée à terme.
- **FR-016**: La réponse « verrouillé » DOIT porter **la date de fin du verrou**, que l'écran de connexion sait déjà rendre.
- **FR-017**: La **durée de validité d'un jeton à usage unique** DOIT être déclarée **par finalité**, au même endroit, dans la configuration du service. Valeurs par défaut : vérification d'adresse **24 heures**, réinitialisation de mot de passe **1 heure**, invitation **7 jours**, lien magique **15 minutes**, confirmation d'intervenant **14 jours**.
- **FR-018**: Aucun appelant NE DOIT poser lui-même une date d'expiration : elle se dérive de la finalité.

### Connexion — l'ordre des contrôles est la règle

- **FR-019**: Le mot de passe DOIT être vérifié **en premier**. Tant qu'il n'est pas juste, la seule réponse possible est « identifiants invalides », adresse inconnue ou non.
- **FR-020**: Le coût de vérification DOIT être payé même quand l'adresse est inconnue, de sorte que la durée de la réponse ne trahisse pas l'existence du compte.
- **FR-021**: Verrouillage, suspension, adresse non vérifiée et second facteur NE DOIVENT être signalés qu'**après** un mot de passe correct, et dans cet ordre.
- **FR-022**: Le mot de passe DOIT être haché en Argon2id. Aucune fonction de la base ne vérifie de mot de passe.
- **FR-023**: Un échec de mot de passe DOIT incrémenter le compteur du compte ; au seuil, le verrou est posé pour la durée configurée.
- **FR-024**: **Écart n° 20 — une adresse non vérifiée interdit la connexion.** Mot de passe correct et adresse jamais vérifiée : la connexion est refusée, la réponse porte l'adresse visée, aucune session n'est ouverte. **Aucun statut de personne n'est ajouté pour cela** : l'état est déjà porté par la date de vérification, et deux sources pour un même fait divergent toujours.
- **FR-025**: Une personne suspendue ou exclue DOIT recevoir la réponse « suspendu », avec sa date de fin quand elle existe.
- **FR-026**: Un compte portant un second facteur activé DOIT recevoir la réponse « second facteur requis ». **Aucune route d'enrôlement ni de validation du code n'est livrée dans ce jalon** : l'emplacement est réservé, comme dans l'écran A1.
- **FR-027**: Une personne connue de la plateforme mais dépourvue de compte mot de passe DOIT recevoir « identifiants invalides ».

### Sessions

- **FR-028**: Une connexion réussie DOIT ouvrir une session portant le poste utilisé et l'adresse d'origine, et délivrer un **jeton d'accès court** et un **jeton de rafraîchissement**.
- **FR-029**: Le jeton de rafraîchissement NE DOIT être conservé qu'en **empreinte** : un vol de la base ne donne aucun jeton utilisable.
- **FR-030**: La durée de la session DOIT suivre le choix de la personne : **trente jours** avec « rester connecté », **douze heures** sans — les valeurs que tient déjà le front.
- **FR-031**: Le renouvellement DOIT faire **tourner** le jeton de rafraîchissement. Le rejeu d'un jeton déjà consommé DOIT être refusé **et révoquer toutes les sessions de la personne**.
- **FR-032**: La déconnexion DOIT révoquer la session avec son motif. Une révocation en masse DOIT exister pour le cas de compromission.
- **FR-033**: Une session DOIT cesser de valoir dès que la personne est suspendue, exclue ou anonymisée, sans attendre son échéance.
- **FR-034**: La route « qui suis-je » DOIT rendre la personne connectée **d'après la session portée par la requête**, sans qu'aucun identifiant ne soit fourni par le client.

### Discrétion des réponses

- **FR-035**: La création de compte DOIT rendre **toujours** la même réponse, adresse libre ou déjà prise. C'est le courriel qui diffère : lien de vérification dans un cas, rappel de compte existant dans l'autre.
- **FR-036**: La demande de réinitialisation et le renvoi de lien de vérification DOIVENT rendre **toujours** la même réponse, adresse connue ou non.
- **FR-037**: Aucune erreur NE DOIT laisser déduire l'existence d'une donnée hors du périmètre de l'appelant : un identifiant inaccessible se refuse comme un identifiant inexistant.

### Jetons à usage unique

- **FR-038**: Un jeton NE DOIT être conservé qu'en empreinte ; sa valeur en clair n'existe que dans le courriel.
- **FR-039**: Le refus d'un jeton DOIT distinguer trois motifs — invalide, expiré, déjà utilisé — et **« déjà utilisé » l'emporte sur « expiré »**.
- **FR-040**: L'émission d'un jeton DOIT invalider les jetons non consommés de la **même finalité** pour la même personne.
- **FR-041**: La consommation d'un jeton DOIT être **atomique** : deux clics simultanés sur le même lien n'aboutissent qu'une fois.
- **FR-042**: Un jeton de réinitialisation DOIT être **revérifié à l'enregistrement** du nouveau mot de passe, et pas seulement au contrôle préalable.
- **FR-043**: Un changement de mot de passe abouti DOIT remettre à zéro le compteur d'échecs et le verrou, et révoquer toutes les sessions de la personne.
- **FR-044**: Les jetons périmés et consommés DOIVENT être purgés par un travail différé récurrent.

### Autorisation, portée et périmètre d'administration

- **FR-045**: L'autorisation DOIT se tester par **permission**, jamais par nom de rôle, et **toujours avec sa portée**.
- **FR-046**: Une permission de mutation DOIT se vérifier sur la portée **visée par l'écriture**, non sur celle de l'appelant.
- **FR-047**: Toute liste du back-office DOIT être bornée au périmètre d'administration de l'appelant, y compris quand l'identifiant vient de l'URL.
- **FR-048**: Les **trois cas** du périmètre DOIVENT rester distincts : global, éditions listées, **aucun droit → refus explicite** et jamais une liste vide.
- **FR-049**: L'API DOIT exposer les permissions effectives d'une personne, **chacune avec sa portée**, pour que le front affiche ou masque une action. Le refus, lui, reste à l'API.
- **FR-050**: L'API DOIT exposer le périmètre d'administration d'une personne sous la forme que le front consomme déjà — un indicateur global et une liste d'éditions, jamais nuls.

### Back-office de l'identité

- **FR-051**: L'API DOIT servir la liste des utilisateurs du back-office avec ses facettes et ses compteurs, bornée au périmètre d'administration.
- **FR-052**: L'API DOIT servir la fiche d'un utilisateur, ses attributions de rôle avec leur portée résolue et leur état, et ses permissions effectives.
- **FR-053**: L'attribution d'un rôle DOIT exiger la permission d'attribution **sur la portée visée**. Le retrait DOIT exiger **la même permission sur la même portée**.
- **FR-054**: Le retrait DOIT enregistrer **son auteur et son motif**, distincts du motif d'octroi. Une attribution révoquée n'est jamais supprimée.
- **FR-055**: Les écritures de rôle NE DOIVENT accepter **aucun paramètre par lequel le client déclarerait ses propres droits** : l'API lit sa propre session.
- **FR-056**: Le changement de statut d'une personne — suspension, exclusion, réactivation — DOIT exiger la permission de gestion des utilisateurs, enregistrer son motif et son auteur, et respecter l'obligation de date de fin sur une suspension.
- **FR-057**: L'API DOIT servir les options d'attribution — rôles assignables, portées atteignables — **restreintes à ce que l'appelant peut réellement accorder**.

### RGPD

- **FR-058**: L'API DOIT permettre d'enregistrer un consentement et de lire l'état courant de chaque finalité, l'historique restant conservé comme preuve.
- **FR-059**: La file des demandes RGPD DOIT exiger la **portée globale** et **ne se borne pas par édition**.
- **FR-060**: L'effacement NE DOIT s'appliquer qu'à une demande d'**effacement**. Une demande d'export ou de rectification le refuse.
- **FR-061**: L'effacement DOIT purger l'identité, supprimer comptes et adresses, révoquer les sessions, **et conserver les agrégats de participation**.

### Contrat et documentation

- **FR-062**: Les noms de champs et les chemins DOIVENT être ceux que le front consomme déjà. Ils ne se renégocient pas.
- **FR-063**: L'API DOIT produire une **documentation OpenAPI générée**, jamais écrite à la main, couvrant chaque route, chaque forme de réponse et **chaque code d'erreur stable**.
- **FR-064**: L'API DOIT résoudre les textes multilingues selon l'en-tête de langue de la requête, que le front pose déjà sur chaque appel.

---

### Key Entities *(include if feature involves data)*

Toutes existent déjà dans `docs/database/`. Aucune n'est créée, aucune n'est modifiée.

- **Personne** (`identity.people`) — une personne physique, avec ou sans compte, identifiée par son adresse canonique. Porte la **date de vérification d'adresse**, dont la nullité interdit la connexion (FR-024), le statut, la langue et le fuseau. Toute la plateforme la référence.
- **Compte** (`identity.accounts`) — un moyen de se connecter rattaché à une personne. Porte l'empreinte du mot de passe, le second facteur, le **compteur d'échecs** et le **verrou** (écart n° 18). Un seul compte mot de passe par personne.
- **Adresse secondaire** (`identity.person_emails`) — sert au rapprochement d'un invité avec son futur compte.
- **Session** (`identity.sessions`) — un jeton de rafraîchissement, en empreinte seule, révocable individuellement ou en masse.
- **Jeton à usage unique** (`identity.one_time_tokens`) — cinq finalités, chacune avec **sa propre durée de validité** (écart n° 19). Empreinte seule, consommation unique.
- **Permission, rôle, attribution** (`identity.permissions`, `roles`, `role_permissions`, `role_assignments`) — une attribution porte **toujours** une portée : plateforme, organisation, édition, espace de négociation. La portée n'a pas de clé étrangère, sa cible pouvant devenir un service distant.
- **Consentement et demande RGPD** (`identity.consents`, `privacy_requests`) — preuve du consentement, échéance réglementaire à trente jours.
- **Événement de domaine** (`platform.outbox_events`) — annonce d'un changement d'état, écrite dans la même transaction que lui.
- **Garde d'idempotence** (`platform.inbox_events`) — couple consommateur/événement déjà traité.
- **Travail différé** (`platform.jobs`) — file de travaux avec clé d'unicité, replanification et file morte.
- **Journal d'audit** (`platform.audit_log`) — alimenté par trigger, source de l'historique champ par champ.
- **Registre des modules** (`platform.modules`) — lu au démarrage pour décider quelles routes monter.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Les six issues de connexion prévues par le front sont toutes atteignables, et une **adresse inconnue est indiscernable d'un mot de passe faux** — même code, même message, et écart de temps de réponse inférieur à 10 % sur cent tentatives de chaque sorte.
- **SC-002**: Une personne qui n'existe pas ne peut être découverte par **aucun** des quatre points d'entrée publics — connexion, inscription, demande de réinitialisation, renvoi de lien.
- **SC-003**: Une personne s'inscrit, reçoit son lien et vérifie son adresse **en moins de deux minutes**, sans assistance.
- **SC-004**: Un administrateur détaché sur une édition ne peut atteindre **aucune** donnée d'une autre édition sur l'ensemble des listes du back-office, y compris en forgeant l'identifiant dans l'URL — vérifié route par route.
- **SC-005**: Une personne sans droit d'administration reçoit **un refus** et jamais une liste vide, sur l'ensemble de ces mêmes routes.
- **SC-006**: Cent demandes simultanées du même travail différé portant la même clé d'unicité produisent **une seule** exécution ; un relais arrêté et relancé sur mille événements déjà traités n'en rejoue **aucun**.
- **SC-007**: Chaque changement d'état qui doit s'annoncer laisse **exactement un** événement, portant son acteur et son identifiant de requête ; une transaction annulée n'en laisse **aucun**.
- **SC-008**: Toute écriture est retrouvable dans l'historique de son entité, **avec le nom de son auteur** — aucune trace anonyme.
- **SC-009**: La base ne contient **aucun secret utilisable** : ni mot de passe, ni jeton de session, ni jeton de lien, seulement des empreintes — vérifié par inspection après un cycle complet des parcours.
- **SC-010**: Les chemins et les noms de champs servis correspondent **exactement** à ce que le front consomme : le basculement des données simulées vers l'API ne demande aucune modification d'écran.
- **SC-011**: La documentation OpenAPI générée couvre **100 %** des routes livrées, et **chaque code d'erreur stable** y figure.
- **SC-012**: La porte de qualité passe : schéma rechargé de zéro, rapport de frontières de modules vide, mise en forme et analyse statique sans avertissement, tests d'intégration au vert sur base réelle.
- **SC-013**: Aucun fichier de `backend/` ne dépasse **1000 lignes**.

---

## Assumptions

Décisions prises faute de précision dans la demande, et à confirmer au besoin.

- **Le second facteur reste un emplacement réservé** — arbitré en session le 20/08. L'API rend l'issue « second facteur requis », mais ne livre ni enrôlement ni validation du code : aucun compte ne peut l'activer, l'issue reste donc inatteignable en pratique. C'est ce qu'a livré l'écran A1.
- **La connexion fédérée est hors périmètre.** Retirée des écrans le 17/08 sur décision du commanditaire ; la base garde sa capacité, l'API ne l'expose pas.
- **Valeurs par défaut du verrouillage : 5 échecs, 15 minutes.** Les données simulées retenaient 12 minutes, valeur explicitement inventée. Ces deux nombres sont des réglages, changeables sans redéploiement du code.
- **Durées de jeton : 24 h pour la vérification d'adresse, 1 h pour la réinitialisation** — les deux que l'écart n° 19 tranche explicitement. Les trois autres finalités (invitation, lien magique, confirmation d'intervenant) reçoivent une durée par défaut faute d'écran qui les consomme dans ce jalon.
- **Durées de session : 12 heures, ou 30 jours avec « rester connecté »** — reprises telles quelles du store d'authentification du front.
- **La session voyage dans un cookie que le navigateur ne peut pas lire**, comme l'annonce déjà le front. Les écritures se protègent par vérification de l'origine plutôt que par un en-tête supplémentaire, qu'aucun écran n'envoie aujourd'hui.
- **Les courriels transactionnels de l'identité sont envoyés par le binaire des travaux différés**, à partir des événements émis. La composition multilingue riche et le suivi des envois appartiennent au module Engagement (B6), qui n'existe pas encore : ce jalon se contente d'un envoi simple, capté en local par Mailpit.
- **Les règles de robustesse du mot de passe sont celles qu'applique déjà le front** — huit signes au moins, une majuscule, une minuscule. L'API les revérifie ; elle ne les durcit pas sans arbitrage.
- **Le back-office de l'identité entre dans ce jalon** : liste des utilisateurs, fiche, attribution et retrait de rôle, changement de statut, file RGPD. L'écran A12 les attend, et les cinq obligations consignées dans `docs/progression/api.md` pour B1 en relèvent.
- **La file RGPD exige la portée globale** et ne se borne pas par édition — obligation n° 4 relevée en écrivant A12.
- **Dépendance d'environnement** : les requêtes étant vérifiées à la compilation, la base doit être démarrée et le schéma chargé pour construire. La chaîne d'intégration en tient compte.

---

## Vérifications faites en écrivant cette spécification

- **Le filtre de permission de `identity.administered_events` est le bon** — question laissée ouverte par B0 (`docs/progression/ecrans/b0-constitution.md` § écart 6). La fonction ne retient que les attributions portant `programme.proposal.read_all`. Relecture du semis de `030_identity.sql` § 6 : `admin`, `reviewer` et `programmer` la détiennent tous les trois, et `super_admin` détient tout par trigger. **Les quatre rôles attribuables sur une édition la portent donc** ; aucun rôle d'administration d'édition ne tombe en `(false, '{}')`. Le filtre n'est pas trop étroit, et aucune modification n'est requise. `900_seed.sql` § 5 ne sème qu'un super-administrateur global, ce qui ne contredit rien.
- **`030_identity.sql` n'a besoin d'aucune modification** pour ce module — reconduction du constat de l'écran A1 après relecture intégrale du fichier.
- **Aucun code Rust n'existe dans le dépôt** : `backend/` est absent, et la cible de vérification correspondante est inerte. Cette spécification crée donc l'arborescence, elle n'en complète aucune.
