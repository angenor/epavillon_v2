# Feature Specification: Guide Négo — compte et admission (étape 0b)

**Feature Branch**: `009-guide-nego-compte-admission`

**Created**: 2026-09-21

**Status**: Draft

**Input**: Étape 0b de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md) — l'entrée dans Guide Négo. Une personne crée un compte ou se connecte avec celui de l'ePavillon : c'est le même compte ([ADR-001](../../docs/AppNego/adr/001-un-seul-compte-la-session-distingue-le-mobile.md)) ; la session note qu'elle vient de l'application, et de quel appareil. Elle saisit un code d'invitation reçu sur WhatsApp : code juste, inconnu, révoqué, épuisé, trop d'essais. Selon le réglage d'admission — code, approbation, ou les deux ([ADR-006](../../docs/AppNego/adr/006-admission-par-code-approbation-en-reglage.md)) —, elle entre aussitôt ou attend qu'un administrateur l'admette. Le code du réseau des négociatrices lui donne en plus l'appartenance au réseau ([ADR-007](../../docs/AppNego/adr/007-le-reseau-distingue-pas-le-genre.md)) ; aucun champ « genre » ne commande un droit. Un module réservé, vu sans code, montre son verrou et invite à saisir le code. Dans le profil : « mon accès » et la déconnexion. Côté back-office de l'ePavillon, avec ses composants : créer et révoquer des codes, voir qui est entré avec lequel et retirer ces accès, choisir le mode d'admission, traiter les demandes en attente.

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| Les écrans de cette étape | [02-socle.html](../../docs/AppNego/design/ecrans/02-socle.html) : « 03a Compte », « 03b Connexion », « 04a Code juste », « 04b Code inconnu », « 04c Code révoqué », « 05 Demande en attente », « 13 Réservé », et la ligne « Mon accès » et la déconnexion dans « 11 Profil » |
| Le système de design et les composants | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) et [design/passation/](../../docs/AppNego/design/passation/), déjà repris à l'étape 0a. Un composant qui manque se crée ici et s'ajoute à la page interne des composants |
| Les mots employés à l'écran | [design/lexique.md](../../docs/AppNego/design/lexique.md) § « Les personnes et l'accès » : « Code d'invitation » — jamais « clé », « jeton » ni « mot de passe » ; « Réservé aux négociatrices et négociateurs » ; « Demande en attente » ; « Administrateur » pour qui gère, « Expert » pour qui valide le fond |
| Les écarts entre le système et les pages | Tranchés dans [05-design.md](../../docs/AppNego/05-design.md) § « Les écarts, tranchés » — le code suit ces décisions, pas les pages qui les contredisent |
| Les décisions | [ADR-001](../../docs/AppNego/adr/001-un-seul-compte-la-session-distingue-le-mobile.md) un seul compte, la session distingue · [ADR-006](../../docs/AppNego/adr/006-admission-par-code-approbation-en-reglage.md) admission par code, approbation en réglage · [ADR-007](../../docs/AppNego/adr/007-le-reseau-distingue-pas-le-genre.md) le réseau distingue, pas le genre · [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) hors connexion |
| Les états de l'admission | [02-domaine.md](../../docs/AppNego/02-domaine.md) § États : Par code · En attente d'approbation · Admise · Refusée · Révoquée |
| Le modèle | `docs/database/030_identity.sql` (comptes, sessions, rôles, portées, `identity.has_permission`), `100_negotiations.sql` (espaces, membres, permissions `negotiation.space.access` et `negotiation.space.manage`), `010_platform.sql` (`platform.settings`), `900_seed.sql`. **Ce qui manque s'ajoute au SQL d'abord** — voir *Ce que le modèle ne porte pas encore* |
| Les principes | Constitution : XII « Confiance », XIII « Un design propre et borné » ; règle de l'ePavillon n° 8 — l'autorisation se teste par permission et par portée, jamais par nom de rôle |

**Hors périmètre** : les thématiques et leur choix à la première entrée, l'accueil « Ma journée », les réglages du profil autres que l'accès et la déconnexion (le thème est déjà livré en 0a), le centre de notifications et le réglage des notifications (étape 3b), le contenu des modules réservés eux-mêmes — documents réservés, canaux, questions aux experts, annuaire —, et la double authentification. Ils viennent aux étapes 0c, 1 et suivantes.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Entrer dans Guide Négo avec son compte (Priority: P1)

Une négociatrice a installé l'application et l'a parcourue en visiteuse. Elle décide de créer son compte : prénom et nom, adresse électronique, pays, mot de passe. L'écran lui dit qu'un seul compte sert à Guide Négo et à l'ePavillon, et que son adresse d'ePavillon convient si elle en a déjà un. Elle valide, confirme son adresse, et se retrouve dans l'application, connectée. Si elle avait déjà un compte, elle prend « Me connecter » et, au besoin, « Mot de passe oublié » — un lien lui arrive, valable une heure. Plus tard, depuis « Profil et réglages », elle se déconnecte.

**Why this priority** : sans compte, ni code ni demande d'accès n'ont de sens. C'est la première marche, et la seule qui se livre seule.

**Independent Test** : sur un téléphone, créer un compte depuis l'application, se déconnecter, se reconnecter avec la même adresse ; puis, avec un compte créé sur le site de l'ePavillon, se connecter dans l'application sans se réinscrire.

**Acceptance Scenarios**

1. **Given** une visiteuse sans compte, **When** elle ouvre « Créer mon compte » depuis l'écran d'ouverture ou depuis un module réservé, **Then** elle voit un formulaire en trois étapes dont elle est à la première — prénom et nom, adresse, pays, mot de passe —, l'aide « Celle de votre compte ePavillon, si vous en avez un », et la sortie « J'ai déjà un compte — Me connecter ».
2. **Given** une personne déjà inscrite sur le site de l'ePavillon, **When** elle se connecte dans l'application avec la même adresse et le même mot de passe, **Then** elle entre sans créer de second compte, et retrouve ses informations.
3. **Given** une personne connectée dans l'application, **When** on regarde ses sessions ouvertes, **Then** celle-ci porte qu'elle vient de l'application — et non du site — et l'appareil d'où elle a été ouverte.
4. **Given** une personne qui a oublié son mot de passe, **When** elle demande un lien, **Then** la réponse à l'écran est la même que l'adresse existe ou non, et le lien reçu reste valable une heure.
5. **Given** une personne connectée, **When** elle se déconnecte depuis le profil, **Then** sa session de cet appareil est fermée, l'application revient à l'état visiteur, et ses autres appareils restent connectés.
6. **Given** une personne connectée dans l'application et sur le site, **When** elle se déconnecte de l'application, **Then** sa session du site n'est pas fermée.
7. **Given** une personne qui se connecte depuis l'application, **When** on regarde la durée de sa session, **Then** elle couvre plusieurs mois sans qu'aucune case n'ait été cochée, et elle repart de sa pleine durée à chaque renouvellement.
8. **Given** une personne qui a créé son compte depuis l'application, **When** elle ouvre le lien reçu par courriel, **Then** elle arrive sur une page de Guide Négo qui confirme son adresse et la ramène à la suite de son parcours — et jamais sur un écran du site sans retour.
9. **Given** un téléphone où l'application installée ne partage pas le stockage du navigateur, **When** la personne confirme son adresse puis revient à l'application, **Then** l'application a relu l'état de son compte d'elle-même et affiche l'étape suivante.

---

### User Story 2 — Ouvrir les modules réservés avec le code reçu sur WhatsApp (Priority: P1)

Le code circule dans le groupe WhatsApp du réseau, ou est remis à l'atelier préparatoire. La négociatrice le saisit à la deuxième étape de son entrée, ou plus tard depuis un module verrouillé. Le code est reconnu : l'application le lui dit par son nom — « Code reconnu. Bienvenue dans le réseau des négociatrices et négociateurs francophones » — et les modules réservés s'ouvrent aussitôt. S'il ne l'est pas, l'écran dit pourquoi, et ce qu'elle peut faire : vérifier les caractères, demander le nouveau code à son groupe, ou faire une demande à l'IFDD.

**Why this priority** : c'est le critère de sortie de l'étape, et la porte de tout ce que les étapes suivantes réservent.

**Independent Test** : avec un code actif en base et le mode d'admission « code », saisir successivement un code juste, un code inconnu, un code révoqué, un code épuisé, puis répéter les essais faux jusqu'au blocage.

**Acceptance Scenarios**

1. **Given** le mode d'admission « code » et un code actif, **When** la personne connectée le saisit, **Then** l'accès aux modules réservés lui est accordé aussitôt, elle en est avertie à l'écran, et l'usage est rattaché à ce code.
2. **Given** un code du réseau des négociatrices, **When** il est accepté, **Then** la personne reçoit, en plus de l'accès, l'appartenance au réseau ; un code général donne le même accès, sans l'appartenance.
3. **Given** un code qui n'existe pas, **When** il est saisi, **Then** l'écran dit « Ce code n'est pas reconnu », rappelle le nombre de caractères et propose « Demander l'accès à l'IFDD ».
4. **Given** un code révoqué, **When** il est saisi, **Then** l'écran dit qu'il a été révoqué, avec la date, indique qu'un nouveau code a été diffusé dans le groupe, et propose « Demander l'accès à l'IFDD » ou « Plus tard ».
5. **Given** un code dont le nombre d'usages est atteint ou dont la validité est passée, **When** il est saisi, **Then** l'écran le dit dans ses propres mots — épuisé, ou terminé — et propose la demande d'accès.
6. **Given** plusieurs essais faux d'affilée, **When** le seuil est franchi, **Then** l'application refuse les essais suivants pendant un temps annoncé à l'écran, sans jamais laisser deviner quel code serait juste, et la demande d'accès reste ouverte.
7. **Given** une personne déjà admise, **When** elle saisit le code du réseau, **Then** elle gagne l'appartenance au réseau sans recevoir un second accès ni perdre celui qu'elle a.
8. **Given** une personne sans réseau, **When** elle veut saisir un code, **Then** l'application le lui dit clairement et garde sa saisie pour le retour du réseau, sans jamais annoncer un accès qu'elle n'a pas obtenu.
9. **Given** un code saisi en minuscules, avec des espaces autour ou une casse mêlée, **When** il est envoyé, **Then** il est reconnu comme le code écrit dans le groupe.

---

### User Story 3 — L'IFDD tient ses codes, et sait qui est entré avec lequel (Priority: P2)

Depuis le back-office de l'ePavillon, un administrateur crée un code pour le réseau des négociatrices et un autre pour les négociateurs en général : un libellé, une validité, un nombre d'usages, et s'il donne ou non l'appartenance au réseau. Il voit la liste de ses codes, leur état et leur nombre d'usages. Quand un code fuite hors du groupe, il le révoque : plus personne n'entre avec lui. Il ouvre alors la liste des personnes entrées avec ce code, et retire l'accès de celles qui n'auraient pas dû l'avoir.

**Why this priority** : sans ce tableau, le code est un secret qu'on ne peut ni changer ni rattraper. C'est la seconde moitié du critère de sortie.

**Independent Test** : créer un code, l'utiliser depuis l'application, le voir apparaître dans les usages, le révoquer, vérifier qu'il n'ouvre plus, puis retirer l'accès d'une personne et vérifier qu'elle ne voit plus les modules réservés.

**Acceptance Scenarios**

1. **Given** un administrateur du périmètre, **When** il crée un code, **Then** il en choisit le libellé, la validité, le nombre d'usages — ou l'absence de limite —, sa portée — un espace de négociation précis, ou Guide Négo en entier — et s'il porte l'appartenance au réseau ; le code engendré lui est montré pour qu'il le diffuse.
2. **Given** une liste de codes, **When** il l'ouvre, **Then** chaque ligne porte son état — actif, révoqué, épuisé, terminé —, ses usages sur son maximum, sa validité et son auteur.
3. **Given** un code actif, **When** il le révoque, **Then** le code cesse d'ouvrir dès la tentative suivante, la date et l'auteur de la révocation sont conservés, et **les accès déjà accordés ne sont pas retirés** : c'est un second geste, explicite.
4. **Given** un code utilisé, **When** il ouvre ses usages, **Then** il voit qui est entré avec lui et quand, et peut retirer l'accès d'une personne, ou de toutes celles du code en un geste, en confirmant.
5. **Given** un accès retiré, **When** la personne rouvre l'application, **Then** elle ne voit plus les modules réservés et « Mon accès » dit que son accès a été retiré.
6. **Given** un administrateur dont le périmètre ne couvre qu'un événement, **When** il ouvre la liste des codes — y compris en forgeant l'adresse d'un code hors de son périmètre —, **Then** il ne voit et ne modifie que ceux de son périmètre.

---

### User Story 4 — Durcir l'admission sans redéployer (Priority: P2)

Des codes circulent hors du groupe. L'administrateur bascule le mode d'admission sur « approbation » depuis le back-office. Dès l'ouverture suivante, l'application ne propose plus l'entrée immédiate : la personne fait une demande, voit un écran qui la rassure — nom, pays, heure d'envoi, état « En attente » — et continue de lire tout ce qui est ouvert. L'administrateur trouve la demande dans une file, l'examine et admet ou refuse. En mode « les deux », le code juste ne suffit plus : il ouvre une demande, que l'administrateur tranche.

**Why this priority** : c'est le filet de sécurité d'ADR-006, et l'autre moitié du critère de sortie. Il se livre après le chemin du code, mais avant la finition des écrans d'accès.

**Independent Test** : basculer le réglage sur « approbation » sans aucune mise en ligne, faire une demande depuis l'application, la traiter dans le back-office, et vérifier que la personne voit son accès changer ; recommencer en mode « les deux » avec un code juste.

**Acceptance Scenarios**

1. **Given** le mode « code », **When** l'administrateur le bascule sur « approbation », **Then** le changement prend effet sans mise en ligne ni redémarrage, et la personne qui ouvre l'application ensuite se voit proposer la demande d'accès et non la saisie d'un code.
2. **Given** le mode « approbation », **When** une personne connectée demande l'accès, **Then** sa demande est enregistrée une seule fois, et l'écran « Demande en attente » lui montre son nom, son pays, l'heure d'envoi avec son fuseau, l'état « En attente », et lui rappelle que les documents, la FAQ, le lexique et les sessions de négociation restent lisibles.
3. **Given** le mode « les deux » et un code juste, **When** la personne le saisit, **Then** elle n'entre pas aussitôt : une demande est ouverte, qui porte le code reconnu, et l'administrateur la tranche.
4. **Given** une demande en attente, **When** l'administrateur l'admet, **Then** la personne reçoit l'accès — et l'appartenance au réseau si la demande portait un code du réseau —, un courriel le lui annonce, et elle le voit à sa prochaine ouverture.
5. **Given** une demande en attente, **When** l'administrateur la refuse, **Then** un courriel le lui annonce avec, s'il en a laissé un, le motif, qu'elle retrouve aussi dans « Mon accès » ; elle peut faire une nouvelle demande, ou saisir un code si le mode l'accepte.
6. **Given** une personne dont la demande est en attente, **When** elle reçoit un code par son groupe et que le mode redevient « code », **Then** l'écran d'attente lui propose « J'ai reçu un code — Le saisir », et le code accepté remplace la demande.
7. **Given** une file de demandes, **When** un administrateur l'ouvre, **Then** il ne voit que celles de son périmètre, avec le nom, le pays, l'heure et le code éventuel, et peut les traiter une par une.

---

### User Story 5 — Comprendre ce qu'ouvre le code, et où l'on en est (Priority: P3)

Une personne connectée touche « Échanges », un document réservé ou l'annuaire sans avoir d'accès. L'écran ne se contente pas de refuser : il dit ce que le module contient, d'où vient le code, et rappelle que tout le reste de Guide Négo demeure ouvert. Deux sorties : saisir le code, ou continuer en visiteuse. Plus tard, dans « Profil et réglages », la ligne « Mon accès » lui dit où elle en est : visiteuse, en attente, admise — avec sa date et, le cas échéant, son appartenance au réseau — ou accès retiré.

**Why this priority** : c'est la finition qui rend les trois récits précédents compréhensibles sans explication. Elle se livre en dernier sans rien bloquer.

**Independent Test** : sans accès, ouvrir chaque module réservé et vérifier le verrou et ses deux sorties ; puis, avec chacun des états d'accès, ouvrir « Mon accès » et lire ce qu'il dit.

**Acceptance Scenarios**

1. **Given** une personne sans accès, **When** elle ouvre un module réservé, **Then** elle voit le titre du module, la mention « Réservé aux négociatrices et négociateurs », la liste de ce qui s'y trouve, la phrase qui rappelle ce qui reste ouvert, et les sorties « Saisir mon code d'invitation » et « Continuer en visiteur ».
2. **Given** une personne sans compte, **When** elle ouvre un module réservé, **Then** le verrou la mène d'abord à la création de compte ou à la connexion, puis à la saisie du code.
3. **Given** une personne admise, **When** elle ouvre « Mon accès », **Then** elle lit son état, la date de son admission, **ce que son accès ouvre — une COP nommée, ou tout Guide Négo** — et, si elle l'a, son appartenance au réseau ; aucune de ces lignes ne mentionne ni ne demande son genre.
4. **Given** une personne dont la demande est en attente, **When** elle ouvre « Mon accès », **Then** elle retrouve l'écran de sa demande et son heure d'envoi.
5. **Given** une personne hors connexion, **When** elle ouvre « Mon accès », **Then** elle lit l'état tel qu'il a été lu, avec l'heure de cette lecture.

---

### Edge Cases

- **Une adresse déjà inscrite crée un compte depuis l'application** : la réponse à l'écran ne dit pas que l'adresse existe — c'est la règle de l'ePavillon — et la personne est invitée à se connecter ou à demander un nouveau mot de passe.
- **Compte suspendu** : la connexion est refusée avec le message de l'API ; aucun accès n'est accordé, aucune demande n'est ouverte.
- **Adresse non vérifiée** : la personne reste sur le chemin de vérification ; elle ne peut ni saisir un code ni demander l'accès tant que son adresse n'est pas confirmée.
- **Le code atteint son quota entre l'affichage et l'envoi** : le dernier usage possible est accordé à une seule personne ; les suivants reçoivent « épuisé ».
- **Deux appareils saisissent le même code pour la même personne** : un seul accès est accordé, sans doublon.
- **Le mode d'admission bascule pendant qu'une demande est en attente** : la demande reste valable et traitable ; elle n'est jamais perdue par un changement de réglage.
- **Un code est révoqué alors que des personnes sont entrées avec** : elles gardent leur accès jusqu'à un retrait explicite, et l'écran des usages le montre.
- **Une personne refusée redemande** : la nouvelle demande est acceptée ; l'historique des décisions précédentes reste visible pour l'administrateur.
- **Une seule demande à la fois** : tant qu'une demande est en attente, une seconde n'est pas créée ; la personne est ramenée à l'écran d'attente.
- **L'administrateur perd son périmètre entre l'affichage et la décision** : la décision est refusée et la file se recharge.
- **La personne se déconnecte alors qu'elle a une demande en attente** : elle la retrouve en se reconnectant sur n'importe quel appareil.
- **Une personne admise pour une COP ouvre un module réservé d'une autre COP** : elle voit le verrou de cette COP-là, et l'invitation à en saisir le code ; son accès existant n'est ni perdu ni élargi.
- **Un code d'une COP terminée** : il n'ouvre plus, et le dit par son propre message — terminé —, sans laisser croire à une erreur de saisie.
- **Le courriel de décision ne part pas** : la décision reste enregistrée et visible dans « Mon accès » ; l'envoi se rejoue, et l'échec n'annule jamais l'admission.
- **Saisie du code, demande d'accès ou déconnexion sans réseau** : l'application le dit et ne promet rien ; la déconnexion sans réseau ferme l'application localement et la session se ferme au retour du réseau.

---

## Requirements *(mandatory)*

### Functional Requirements

#### Le compte et la session

- **FR-001** : l'application DOIT permettre de créer un compte avec prénom et nom, adresse électronique, pays et mot de passe, et DOIT dire à l'écran qu'un seul compte sert à Guide Négo et à l'ePavillon.
- **FR-002** : la création de compte depuis l'application DOIT suivre exactement les règles de l'ePavillon — vérification de l'adresse, exigences de mot de passe, réponse invariable qui ne révèle pas si une adresse est déjà inscrite — sans second chemin d'inscription ni second catalogue de messages.
- **FR-003** : une personne inscrite sur le site DOIT pouvoir se connecter dans l'application sans se réinscrire, et l'inverse ; le système NE DOIT créer aucun second compte pour la même adresse.
- **FR-004** : l'application DOIT offrir la connexion et la demande d'un nouveau mot de passe, dont le lien reste valable une heure.
- **FR-004 bis** : un courriel déclenché **depuis l'application** — vérification d'adresse, nouveau mot de passe — DOIT porter un lien qui ramène **dans Guide Négo**, à son apparence, et non sur un écran du site. Le même courriel déclenché depuis le site reste inchangé. C'est le module qui envoie le courriel qui choisit le lien, d'après le client de la demande.
- **FR-004 ter** : sur un téléphone où l'application installée ne partage pas le stockage du navigateur, la page de confirmation DOIT dire que l'adresse est confirmée et renvoyer à l'application ; **l'application DOIT relire l'état du compte d'elle-même à son retour au premier plan**, sans que la personne ait à toucher quoi que ce soit. L'écran d'attente DOIT porter « J'ai confirmé mon adresse » et « Renvoyer le courriel ».
- **FR-005** : toute session ouverte DOIT porter son type de client — site ou application — et l'appareil d'où elle a été ouverte, de sorte qu'on puisse dénombrer les personnes qui utilisent l'application sans jamais dédoubler une personne.
- **FR-006** : la déconnexion DOIT fermer la seule session de cet appareil, laisser les autres ouvertes, et ramener l'application à l'état visiteur.
- **FR-006 bis** : une session ouverte depuis l'application DOIT durer **longtemps et d'office** — sans case à cocher —, assez pour couvrir la préparation d'une COP **et** la COP elle-même, et sa durée DOIT repartir à chaque renouvellement. Douze heures, la durée du site, laisserait une personne déconnectée en salle de négociation, là où aucun réseau ne lui permet de se reconnecter.
- **FR-006 ter** : quand la session a expiré et qu'il n'y a pas de réseau, l'application DOIT **garder ce qui a été lu** et l'afficher avec l'heure de sa lecture ; elle NE DOIT réclamer la reconnexion qu'au retour du réseau, sans jamais vider l'écran ni renvoyer à la connexion dans le vide.
- **FR-007** : l'écran d'entrée DOIT montrer où l'on en est dans le parcours — compte, code, thématiques — même si la troisième étape n'est pas livrée à cette étape.
- **FR-008** : le système NE DOIT ni demander, ni afficher, ni stocker le genre d'une personne pour décider d'un droit ; la civilité existante du compte NE DOIT commander aucun accès.

#### Le code d'invitation

- **FR-009** : une personne connectée DOIT pouvoir saisir un code d'invitation, depuis le parcours d'entrée ou depuis un module verrouillé.
- **FR-010** : le système DOIT reconnaître un code **quelle que soit la casse et quels que soient les séparateurs** — espaces autour, espaces internes, tirets : `nego-024`, `NEGO 024` et `Nego024` désignent le même code. Il circule recopié à la main depuis un message.
- **FR-011** : un code juste, en mode « code », DOIT accorder l'accès aux modules réservés dans la même opération que l'enregistrement de son usage, et l'écran DOIT le confirmer en nommant le réseau.
- **FR-012** : l'accès accordé DOIT être une permission avec sa portée, jamais un nom de rôle testé à l'affichage. La portée est celle du code : un administrateur choisit, **à la création de chaque code**, s'il ouvre **un espace de négociation précis** — une COP, et elle seule — ou **Guide Négo en entier**, sans limite de durée.
- **FR-012 bis** : un accès lié à un espace NE DOIT ouvrir que les modules réservés de cet espace ; un module réservé d'un autre espace DOIT montrer son verrou et inviter à saisir le code de cet espace. Un accès sans limite de durée NE DOIT cesser que par un retrait explicite.
- **FR-013** : un code marqué comme code du réseau des négociatrices DOIT accorder, en plus de l'accès, l'appartenance au réseau ; un code général DOIT accorder le même accès sans l'appartenance.
- **FR-014** : l'appartenance au réseau NE DOIT ouvrir aucun droit supplémentaire à cette étape ; elle est une appartenance, qui servira au canal réservé et aux chiffres demandés par les bailleurs.
- **FR-015** : chaque issue d'un code DOIT donner un message distinct qui dit ce qui se passe et ce que la personne peut faire ensuite ; aucune NE DOIT laisser l'écran **sans au moins une action possible** à l'écran. Les neuf issues sont : accepté, **demande ouverte** (mode « les deux »), **déjà admise**, inconnu, révoqué, épuisé, terminé, **pas encore ouvert**, trop d'essais.
- **FR-016** : un code révoqué DOIT dire la date de sa révocation et renvoyer au nouveau code diffusé dans le groupe.
- **FR-017** : le système DOIT limiter les essais de code **par personne**, tous appareils confondus, annoncer à l'écran le temps d'attente une fois le seuil franchi, et NE DOIT jamais laisser deviner ce qui distingue un code inconnu d'un code révoqué ou épuisé quand la limite est atteinte. L'appareil est enregistré comme information, **jamais comme borne du compteur** : il est déclaré par le client, donc changer d'identifiant ne DOIT pas remettre le compte à zéro.
- **FR-018** : une personne qui a déjà l'accès DOIT pouvoir saisir un autre code pour gagner l'appartenance au réseau, sans recevoir un second accès ni perdre le sien.
- **FR-019** : hors connexion, l'application DOIT dire que la saisie d'un code demande le réseau, et NE DOIT jamais annoncer un accès qui n'a pas été obtenu.
- **FR-020** : les messages de refus d'un code DOIVENT venir du catalogue d'erreurs de l'API, affichés tels quels ; le client NE DOIT pas en écrire un second.

#### Le mode d'admission et les demandes

- **FR-021** : le mode d'admission DOIT être un réglage à trois valeurs — code seul, approbation seule, les deux —, modifiable depuis le back-office et pris en compte **sans mise en ligne ni redémarrage**.
- **FR-022** : en mode « approbation seule », l'application NE DOIT pas proposer la saisie d'un code et DOIT proposer la demande d'accès.
- **FR-023** : en mode « les deux », un code juste NE DOIT pas accorder l'accès : il DOIT ouvrir une demande qui porte le code reconnu, à trancher par un administrateur.
- **FR-024** : une personne connectée et vérifiée DOIT pouvoir demander l'accès ; le système NE DOIT retenir qu'une seule demande en attente par personne et par portée.
- **FR-025** : l'écran d'attente DOIT montrer le nom, le pays, l'heure d'envoi avec son fuseau, l'état « En attente », annoncer que la réponse arrivera **par courriel**, et rappeler ce qui reste lisible sans accès ; il DOIT offrir « Aller à ma journée » et « J'ai reçu un code — Le saisir ». La première sortie mène à **l'accueil déjà livré à l'étape 0a**, avec son état vide : le contenu de « Ma journée » reste hors périmètre, la destination existe.
- **FR-026** : une demande DOIT porter un état parmi : **en attente, admise, refusée, annulée** ; le passage d'un état à l'autre DOIT conserver qui a décidé, quand, et le motif éventuel. « Annulée » est le fait de la personne, qui est entrée par un code entre-temps ; **« révoquée » ne qualifie jamais une demande** — c'est un accès qu'on retire.
- **FR-027** : l'admission d'une demande DOIT accorder l'accès — et l'appartenance au réseau si la demande portait un code du réseau — dans la même opération que le changement d'état.
- **FR-028** : la personne DOIT recevoir un courriel à l'admission comme au refus de sa demande, qui dit la décision et, s'il y en a un, le motif ; l'état DOIT de toute façon être à jour dans « Mon accès » à sa prochaine ouverture. Le courriel suit le chemin d'envoi déjà en place et NE DOIT partir qu'une fois la décision enregistrée.
- **FR-029** : un changement de mode d'admission NE DOIT ni annuler ni rendre intraitable une demande déjà en attente.

#### Le verrou et « Mon accès »

- **FR-030** : un module réservé ouvert sans accès DOIT afficher son titre, la mention « Réservé aux négociatrices et négociateurs », **la liste de ce qu'il contiendra, telle que la maquette l'écrit**, le rappel de ce qui reste ouvert, et deux sorties : saisir le code, ou continuer en visiteur.
- **FR-031** : le verrou vu sans compte DOIT mener d'abord à la création de compte ou à la connexion, puis à la saisie du code.
- **FR-032** : le verrou DOIT s'appuyer sur la permission effective de la personne, jamais sur un nom de rôle ni sur un état retenu côté client sans vérification par l'API.
- **FR-033** : « Mon accès », dans le profil, DOIT dire l'état de la personne — visiteuse, en attente, admise avec sa date, refusée, accès retiré —, ce que son accès ouvre, et son appartenance au réseau le cas échéant.
- **FR-034** : hors connexion, « Mon accès » DOIT afficher l'état tel qu'il a été lu, avec l'heure de cette lecture, selon la règle de l'étape 0a.
- **FR-035** : le profil DOIT porter la déconnexion, avec la mention de ce qu'elle laisse sur le téléphone.

#### Le back-office de l'ePavillon

- **FR-036** : un administrateur DOIT pouvoir créer un code d'invitation en choisissant son libellé, sa validité, son nombre d'usages maximal — ou l'absence de limite —, **sa portée : un espace de négociation précis, ou Guide Négo en entier** —, et s'il porte l'appartenance au réseau des négociatrices.
- **FR-037** : le code engendré DOIT être montré à sa création pour être diffusé, et DOIT rester lisible ensuite dans la liste : c'est un code partagé, pas un secret nominatif.
- **FR-038** : la liste des codes DOIT montrer, pour chacun, son état — actif, révoqué, épuisé, terminé —, ses usages sur son maximum, sa validité, son auteur et sa portée.
- **FR-038 bis** : un code limité à N usages NE DOIT en accorder que N, **même si deux personnes entrent à la même seconde** sur le dernier. Le dépassement DOIT être impossible, pas seulement improbable.
- **FR-039** : la révocation d'un code DOIT prendre effet dès la tentative suivante, conserver sa date et son auteur, et **NE DOIT PAS** retirer les accès déjà accordés.
- **FR-040** : l'administrateur DOIT pouvoir voir qui est entré avec un code et quand, retirer l'accès d'une personne, et retirer en un geste ceux de toutes les personnes d'un code, après confirmation.
- **FR-041** : un accès retiré DOIT cesser d'ouvrir les modules réservés dès l'ouverture suivante de l'application, et DOIT être dit dans « Mon accès ».
- **FR-042** : l'administrateur DOIT pouvoir lire et changer le mode d'admission, et voir quel effet chaque valeur produit pour la personne qui entre.
- **FR-043** : la file des demandes en attente DOIT montrer le nom, le pays, l'heure d'envoi et le code éventuel, et permettre d'admettre ou de refuser avec un motif facultatif.
- **FR-044** : le back-office de l'admission — codes, usages, demandes, mode — DOIT être réservé aux administrateurs **de la plateforme entière**, **y compris quand l'adresse est forgée**. *Tranché le 21/09* : pour Guide Négo, « son périmètre » veut dire **global**. La garde est `negotiation.space.manage` sur la portée globale, et non « sur n'importe quelle portée » — le rôle `admin` porte cette permission et s'attribue aussi sur un événement, alors qu'aucun espace de négociation n'est rattaché à une édition. Un administrateur limité à un événement NE DOIT rien voir de ce back-office : ni liste, ni fiche, ni entrée de menu. Aucune fonction de périmètre nouvelle n'est créée pour cela.
- **FR-045** : ces écrans DOIVENT être construits avec les composants existants du back-office de l'ePavillon, et NE DOIVENT emprunter ni jeton ni composant à Guide Négo.
- **FR-046** : la création d'un code, sa révocation, un retrait d'accès, un changement de mode et chaque décision sur une demande DOIVENT laisser une trace qui nomme leur auteur.

### Ce que le modèle ne porte pas encore

Ces manques ont été relevés dans `docs/database/` ; **ils s'ajoutent au SQL avant tout code** :

- le type de client et l'appareil sur les sessions ;
- les codes d'invitation partagés, avec leur validité, leur quota, leur portée, leur révocation et leur auteur ;
- le rattachement d'un usage à son code — qui est entré avec lequel ;
- les demandes d'accès et leurs états ;
- l'appartenance au réseau des négociatrices, tenue **hors de l'identité de la personne** ;
- la limitation des essais de code ;
- la clé du mode d'admission dans les réglages de plateforme, avec sa valeur de départ.

La permission `negotiation.space.access`, le rôle `negotiator` et la portée `negotiation_space` existent déjà : rien n'est à créer de ce côté.

### Key Entities

- **Compte** : une personne, une adresse vérifiée, un pays, un mot de passe. Le même pour l'ePavillon et Guide Négo. Ne porte aucun genre commandant un droit.
- **Session** : une connexion ouverte sur un appareil. Porte désormais d'où elle vient — site ou application — et quel appareil.
- **Code d'invitation** : une courte suite de caractères diffusée à un groupe. Porte un libellé, une portée — un espace de négociation précis, ou Guide Négo en entier —, une validité, un nombre d'usages maximal, l'indication qu'il ouvre ou non l'appartenance au réseau, son auteur, et son éventuelle révocation datée.
- **Usage d'un code** : le lien entre une personne et le code par lequel elle est entrée, avec sa date. C'est lui qui permet de retirer les accès d'un code compromis.
- **Demande d'accès** : une personne, un code éventuel, une heure d'envoi, un état — en attente, admise, refusée —, le décideur, sa date et son motif.
- **Accès** : la permission d'entrer dans les modules réservés, avec sa portée, accordée par un code ou par une admission, et retirable.
- **Appartenance au réseau des négociatrices** : une appartenance portée par le code utilisé, jamais par une case sur l'identité. Elle n'ouvre aucun droit de plus à cette étape.
- **Mode d'admission** : un réglage à trois valeurs, modifiable depuis le back-office, lu à chaque tentative d'entrée.

---

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : une négociatrice qui reçoit le code sur WhatsApp crée son compte, saisit le code et voit les modules réservés s'ouvrir en moins de trois minutes, sans aide extérieure.
- **SC-002** : un administrateur révoque un code et bascule le mode d'admission sur « approbation » **sans aucune mise en ligne** ; la personne qui ouvre l'application ensuite se voit proposer la demande, et le code révoqué n'ouvre plus.
- **SC-003** : les **neuf** issues d'un code produisent chacune un message distinct qui dit quoi faire ensuite, et chacune laisse **au moins une action possible** à l'écran.
- **SC-004** : une personne inscrite sur le site entre dans l'application avec la même adresse sans créer de second compte, dans 100 % des cas.
- **SC-005** : après un retrait d'accès, la personne ne voit plus aucun module réservé dès son ouverture suivante, et « Mon accès » l'explique.
- **SC-006** : aucun écran ni aucune donnée de ce périmètre ne demande, n'affiche ni n'utilise le genre pour accorder un droit ; l'appartenance au réseau se déduit du seul code utilisé.
- **SC-007** : l'IFDD peut dire à tout moment combien de personnes sont entrées avec chaque code, et combien appartiennent au réseau des négociatrices.
- **SC-008** : un administrateur dont le périmètre ne couvre qu'un événement ne voit **rien** du back-office de Guide Négo — aucun code, aucun usage, aucune demande, aucune entrée de menu —, et l'adresse forgée d'un code lui rend le **même refus qu'un identifiant inexistant**.
- **SC-009** : hors connexion, l'application ouvre « Mon accès » et affiche l'état lu avec son heure, et ne promet jamais un accès non obtenu.
- **SC-010** : les écrans de cette étape sont fidèles à la maquette à 360 px, en thème clair et sombre, sans défilement horizontal.
- **SC-011** : toute décision sur une demande — admission comme refus — part en courriel à la personne concernée, et la décision reste acquise même si l'envoi échoue.
- **SC-012** : une personne connectée depuis l'application le reste pendant toute une COP, préparation comprise, sans avoir rien coché ; son renouvellement ne raccourcit jamais sa durée.
- **SC-013** : une personne qui crée son compte depuis l'application et confirme son adresse depuis son courriel se retrouve dans l'application, sur la suite de son parcours, **sur les deux systèmes de téléphone** — sans avoir à chercher comment y revenir.
- **SC-014** : cinq essais de code faux, puis un sixième depuis un appareil déclaré différent : le sixième est refusé comme les précédents.

---

## Assumptions

- **L'authentification existante est reprise telle quelle.** Inscription, vérification d'adresse, connexion, réinitialisation, rotation et révocation de sessions sont déjà livrées pour l'ePavillon ; cette étape ne les réécrit pas, elle les ouvre à l'application et ajoute à la session le type de client et l'appareil.
- **Le format du code** : une courte suite de lettres, de chiffres et de tirets, de **huit caractères, tirets compris** — par exemple `NEGO-024` —, engendrée par le système et non choisie par l'administrateur, sans caractères prêtant à confusion (`0/O`, `1/I/L` écartés). **La maquette se contredit sur ce point** : elle écrit « vérifiez les huit caractères » sous un exemple, `NEGO-24`, qui en porte sept. Huit fait foi, et l'exemple de la maquette se corrige.
- **La limite d'essais** : cinq essais faux d'affilée, puis quinze minutes d'attente, **par personne**. Le seuil et le délai sont des réglages, non des valeurs écrites dans le code.
- **Le mode d'admission de départ est « code seul »** : c'est le fonctionnement décrit par ADR-006 en temps normal.
- **La portée d'un code se choisit à chaque création**, et le choix proposé par défaut est **l'espace de négociation de l'édition en cours** : le code d'une COP se périme avec elle, un code sans limite de durée reste un geste délibéré.
- **Le courriel est le canal de réponse aux demandes**, à l'admission comme au refus. C'est un écart assumé avec la maquette, dont l'écran d'attente annonce une notification : il dira « un courriel » tant que le centre de notifications de l'étape 3b n'existe pas. À inscrire aux écarts de [05-design.md](../../docs/AppNego/05-design.md).
- **Un code ouvre l'accès, jamais un rôle d'administration** : aucun code ne peut accorder un droit de gestion.
- **L'appartenance au réseau ne change rien à ce qui est visible à cette étape** : les canaux, l'annuaire et les questions aux experts arrivent plus tard ; l'appartenance est posée maintenant pour que les chiffres remontent dès le départ.
- **Le verrou est livré ici, et se pose là où un écran existe.** À cette étape, un seul : l'onglet Échanges. Les questions aux experts, l'annuaire et les documents réservés n'ont pas encore de page — **chaque étape qui en ouvre une y pose le verrou**, avec le composant livré ici. Le nommer maintenant sur des écrans qui n'existent pas donnerait une exigence que rien ne pourrait vérifier.
- **La déconnexion ne promet que ce qui existe** : elle dit que les téléchargements restent sur le téléphone ; l'effacement des documents réservés à la déconnexion relève de l'étape 1, qui les introduit.
- **Le courriel est le seul canal sortant disponible** : le centre de notifications et les notifications poussées arrivent à l'étape 3b.
- **L'interface reste en français** quel que soit le réglage du téléphone, comme à l'étape 0a ; les traductions anglaises suivent le même découpage par écran.
- **La durée d'une session d'application est un réglage**, posé à 90 jours glissants. Le site garde ses douze heures, et ses trente jours avec « se souvenir de moi ».
- **La base ne se détruit pas.** `identity.sessions` est une table en service : le changement de schéma se livre par un script de migration rejouable, éprouvé sur une copie, et contrôlé en comparant le schéma obtenu à celui que produisent les fichiers du modèle.
- **Le drapeau `guide_nego.enabled` continue de commander l'ouverture de l'application** ; cette étape n'ajoute aucun drapeau de module.
