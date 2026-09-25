# Feature Specification: Guide Négo — FAQ, parcours « Ma première COP » et lexique (étape 2)

**Feature Branch**: `013-guide-nego-faq-lexique`

**Created**: 2026-09-25

**Status**: Draft — trois questions tranchées par l'orchestrateur le 25/09 (voir *Clarifications*)

**Input**: Étape 2 de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md) : « La FAQ, le parcours « Ma première COP » et le lexique anglais-français de Guide Négo. Maquette : 05-savoir.html et 06-lexique.html ; données d'essai : donnees-savoir.md et donnees-lexique.md. Rien n'existe dans le modèle : tout est à créer. […] Critère : « contact group » trouvé hors connexion ; une entrée porte sa date de vérification. »

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| Les écrans | [05-savoir.html](../../docs/AppNego/design/ecrans/05-savoir.html) — « 01 FAQ », « 02 Entrée de FAQ », « 02b Qu'est-ce qui manque », « 02c Dépassé ou faux », « 03 Question à un expert », « 03b Question envoyée », « 03c Question réservée », « 04 Parcours Ma première COP » ; [06-lexique.html](../../docs/AppNego/design/ecrans/06-lexique.html) — « 01 Ouverture du lexique » à « 06b Proposer un terme » ; [02-socle.html](../../docs/AppNego/design/ecrans/02-socle.html) — « 09 Recherche » ; [04-lecteur.html](../../docs/AppNego/design/ecrans/04-lecteur.html) — « 10 Terme touché » |
| Les données d'essai | [donnees-savoir.md](../../docs/AppNego/design/donnees-savoir.md) — 4 rubriques, 18 questions, 18 étapes du parcours en 4 groupes ; [donnees-lexique.md](../../docs/AppNego/design/donnees-lexique.md) — 19 entrées en 3 familles, dont *contact group*. **Ce sont des données, jamais du code** : elles se chargent en local par un script ; le contenu de production vient du back-office |
| Le système de design | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) et [design/passation/](../../docs/AppNego/design/passation/). Un composant qui manque se crée ici et s'ajoute à la page interne des composants |
| Les mots employés à l'écran | [design/lexique.md](../../docs/AppNego/design/lexique.md) — « Vérifié le 12 novembre 2026 », « Hors connexion — lu à… » sans fuseau (écart 32), « Code d'invitation », vouvoiement |
| Le domaine | [02-domaine.md](../../docs/AppNego/02-domaine.md) — états « Brouillon · Publié · À revoir » (FAQ, lexique), « En attente · Répondue · Ajoutée à la FAQ » (question) ; rôle `expert` ; règle 9 : les rubriques sont des données |
| Les décisions | [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) FAQ et lexique gardés en entier, écritures sans réseau parties au retour · [ADR-011](../../docs/AppNego/adr/011-un-corpus-a-deux-etages.md) une réponse d'expert promue en FAQ entre dans la référence · [ADR-012](../../docs/AppNego/adr/012-toute-source-porte-un-etat.md) « Dépassé ou faux » alimente la file des experts · [ADR-018](../../docs/AppNego/adr/018-direction-typographique-quatre-onglets.md) le lexique n'est pas un onglet, « Aa » l'ouvre de partout |
| Les principes | Constitution : XI « Hors connexion d'abord », XII « Confiance », XIII « Un design propre et borné » |
| Ce qui est déjà livré | La coquille, la garde des lectures, la file d'écritures différées avec son empreinte (0a, 0c) ; le compte, le code d'invitation et le verrou (0b) ; le rôle `expert` et le back-office de Guide Négo (1) ; le lecteur et sa feuille du terme anglais touché, qui dit aujourd'hui que le lexique est à venir (1, 1b) ; le bouton « Aa » et la page du lexique vide (0a) |

**Hors périmètre** : le groupe « Sessions de négociation » de la recherche globale (3a) ; le centre de notifications et toute notification poussée (3b, 6a) ; l'assistant IA et l'indexation du corpus (7) ; les Échanges entre pairs (6) ; les quiz (8) ; la traduction anglaise des contenus métier ; un écran d'administration des rubriques et des familles, ces vocabulaires se sèment ; la remise « à vérifier » de toute la FAQ à l'ouverture d'un nouveau cycle (ADR-012), qui viendra avec le changement de COP.

## Clarifications

### Session 2026-09-25 — tranché le 25/09

- Q : Qui peut répondre « Oui / Non », dire ce qui manque et signaler « Dépassé ou faux » ? → R : **Une personne connectée, quel que soit son accès.** Tout le monde voit les boutons ; sans compte, le geste mène à la connexion, comme le favori d'un document (03-api.md : lecture publique, écriture gardée). Le retour reste anonyme pour l'expert. Le compteur des « plus lues » reste anonyme et ne demande pas de compte.
- Q : Comment la personne apprend-elle qu'un expert a répondu à sa question, ou qu'un terme qu'elle a proposé est publié ? → R : **Par un courriel, et dans « Mes questions »**, par l'envoi de courriel de l'étape 0b. La notification dans l'application viendra avec 3b.
- Q : Que voit-on d'une entrée de FAQ mise « À revoir » ? → R : **Elle reste affichée**, avec sa dernière date de vérification et la ligne « Un expert relit cette réponse », jusqu'à la décision de l'expert (maquette 02c).

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Un mot entendu en salle, trouvé sans réseau (Priority: P1)

En séance, Aïssatou entend « the contact group will reconvene at 3 p.m. ». Le réseau est saturé. Elle touche « Aa » en haut de l'écran : le lexique s'ouvre, clavier sorti, sur « Un mot entendu en salle » et « 19 entrées, sans réseau ». Elle tape « contact grup » : *contact group* — groupe de contact paraît à la première lettre corrigée, avec la première phrase de sa définition. Elle l'ouvre : la famille « Réunions », la traduction, la définition, la phrase « Entendu en salle », les sources et les termes liés. Elle le met en favori.

**Why this priority** : c'est le critère de sortie, et le geste le plus fréquent en salle. Le lexique vit sans la FAQ ni le parcours.

**Independent Test** : charger les données d'essai, ouvrir l'application une fois avec le réseau, passer en mode avion, fermer et rouvrir, toucher « Aa » depuis trois écrans différents, chercher « contact grup », « groupe de contact », « GGA » et « braketed », ouvrir chaque entrée, en mettre deux en favori, parcourir la liste par le rail et filtrer par famille.

**Acceptance Scenarios**

1. **Given** n'importe quel écran de Guide Négo, **When** on touche « Aa », **Then** le lexique s'ouvre, champ actif et clavier sorti, avec le nombre d'entrées et, s'il y en a, les derniers termes consultés ; une croix le referme et ramène à l'écran d'où l'on venait.
2. **Given** le lexique ouvert, avec ou sans réseau, **When** on tape, **Then** les résultats suivent chaque frappe — terme, traduction, première phrase de la définition — et la recherche porte sur le terme anglais, son sigle et sa traduction française, sans tenir compte des accents ni de la casse.
3. **Given** une saisie mal orthographiée (« braketed », « contact grup »), **When** aucune entrée ne correspond exactement, **Then** l'écran propose « Vous cherchiez peut-être : *bracketed text* » et range en tête les entrées proches.
4. **Given** une saisie qui ne ramène rien, **When** on lit l'écran, **Then** il dit « 0 entrée sur 19 », que le terme n'est pas encore dans le lexique, et propose « Proposer « … » » et « Parcourir la liste ».
5. **Given** la liste, **When** on l'ouvre, **Then** les entrées sont rangées par lettre, chaque lettre avec son compte ; un rail A–Z permet de sauter à une lettre en la touchant ou en y glissant le doigt ; une lettre sans entrée est grisée ; des filtres par famille réduisent la liste.
6. **Given** une entrée, **When** on l'ouvre, **Then** elle montre sa famille, le terme en italique, la traduction, la définition, et, quand ils existent, la phrase « Entendu en salle », le sigle développé, les sources et les termes liés — chacun ouvre son entrée ; « Favori » et « Partager » l'accompagnent.
7. **Given** une entrée, **When** on touche « Favori », **Then** elle rejoint « Mes termes favoris », qui se lit sans réseau ; retouchée, elle en sort. Sans compte, les favoris restent sur le téléphone ; avec un compte, ils le suivent sur ses autres appareils.
8. **Given** le lexique jamais ouvert sur ce téléphone et aucun réseau, **When** on touche « Aa », **Then** il se lit quand même en entier dès lors que l'application a été ouverte une fois avec le réseau depuis la publication de cette étape.

---

### User Story 2 — La FAQ se lit en entier sans réseau et dit quand elle a été vérifiée (Priority: P1)

La veille de sa première COP, Moussa ouvre Ressources, puis « FAQ » : « 18 réponses vérifiées par les experts de l'IFDD ». Il parcourt la rubrique « Le processus », ouvre « Quelle différence entre un groupe de contact et des consultations informelles ? » : la réponse, « Vérifié le 12 novembre 2026 », deux sources dont une citation du guide, trois questions liées. En salle, sans réseau, il la retrouve en tapant « contact » dans le champ de la FAQ.

**Why this priority** : c'est la seconde moitié du critère de sortie. Une réponse sans date de vérification ne vaut pas plus qu'une rumeur de couloir.

**Independent Test** : charger les données d'essai, ouvrir la FAQ avec le réseau, passer en mode avion, parcourir chaque rubrique, chercher trois questions en vos mots, ouvrir une entrée complète et vérifier la date, les sources, les questions liées ; suivre une source jusqu'au document.

**Acceptance Scenarios**

1. **Given** la FAQ, **When** on l'ouvre, **Then** elle montre le nombre de réponses publiées, la ligne de synchronisation, le champ « Poser une question en vos mots », les rubriques avec leur compte, la ligne du parcours et sa progression, « Les plus lues » et « Poser une question à un expert ».
2. **Given** une entrée publiée, **When** on l'ouvre, **Then** elle porte sa rubrique, sa question, « Vérifié le » et la date de sa dernière vérification, sa réponse, ses sources, ses questions liées, « Cette réponse vous a-t-elle aidée ? » et « Dépassé ou faux ».
3. **Given** une source qui renvoie à un document de la bibliothèque, **When** on la touche, **Then** le document s'ouvre à la page citée — hors connexion s'il est gardé, sinon l'écran « Ce document n'est pas sur votre téléphone » de l'étape 1 ; une source extérieure affiche son titre et sa référence, et s'ouvre si elle porte une adresse.
4. **Given** le champ de la FAQ, avec ou sans réseau, **When** on tape des mots, **Then** les questions dont la question ou la réponse les contient paraissent à la frappe, tolérantes aux fautes et aux accents.
5. **Given** une entrée publiée puis modifiée ou mise « à revoir » dans le back-office, **When** le téléphone relit la FAQ avec le réseau, **Then** il reçoit la modification sans tout retélécharger ; hors connexion, il affiche la dernière version lue et « Hors connexion — lu à… ».
6. **Given** une entrée dépubliée, **When** le téléphone relit la FAQ, **Then** elle disparaît de la liste, des questions liées et de la recherche.
7. **Given** « Les plus lues », **When** on les regarde, **Then** ce sont les trois entrées les plus ouvertes sur les trente derniers jours ; tant qu'aucune lecture n'est comptée, l'ordre choisi par l'IFDD.

---

### User Story 3 — « Ma première COP » : cocher à son rythme, retrouver ses coches partout (Priority: P2)

Moussa ouvre le parcours : « Parcours en 18 étapes », quatre groupes. Avant de partir, il coche les cinq étapes, dont « Lire : Guide, chapitre 3 », qui ouvre le guide au bon chapitre. Le premier jour, sans réseau, il coche « Retirer mon badge » ; le bandeau dit « Hors connexion — lu à 11:35. Vos coches restent sur le téléphone. » Le soir, sur la tablette de sa délégation, connecté à son compte, il retrouve ses sept coches.

**Why this priority** : c'est le parcours d'accueil des primo-participants, mais il ne conditionne ni le lexique ni la FAQ.

**Independent Test** : cocher trois étapes sans compte, en cocher deux en mode avion avec un compte, rétablir le réseau, ouvrir le même compte sur un autre navigateur et retrouver les coches ; décocher sur l'un, puis sur l'autre, et constater que le dernier geste l'emporte sans perte silencieuse.

**Acceptance Scenarios**

1. **Given** le parcours, **When** on l'ouvre, **Then** il montre le nombre d'étapes cochées sur le total, la prochaine étape non cochée, la barre de progression, et les groupes avec leur compte « n sur N ».
2. **Given** une étape, **When** on touche sa ligne entière, **Then** elle se coche ou se décoche ; cochée, son libellé passe en gris, jamais barré.
3. **Given** une étape qui porte un complément, **When** on le lit, **Then** c'est un détail, une origine (« Réunions de la Francophonie ») ou un lien « Lire : … » qui ouvre un document à sa section ou à sa page, une entrée de FAQ ou une entrée du lexique.
4. **Given** aucune connexion, **When** on coche, **Then** la coche est gardée sur le téléphone et le bandeau le dit ; avec un compte, elle part au retour du réseau.
5. **Given** un compte, **When** on ouvre le parcours sur un autre appareil, **Then** on y retrouve ses coches ; si deux appareils ont changé les coches entre-temps, l'écran le dit et montre l'état retenu, sans perdre une coche en silence.
6. **Given** des coches posées sans compte, **When** on se connecte, **Then** elles rejoignent celles du compte, sans effacer aucune des deux.
7. **Given** le pied du parcours, **When** on le lit, **Then** il dit « Cochez à votre rythme : rien n'est obligatoire. »
8. **Given** une étape ajoutée, retirée ou déplacée par l'IFDD, **When** le téléphone relit le parcours, **Then** la progression se recalcule sur les étapes publiées ; une coche d'une étape retirée ne compte plus.

---

### User Story 4 — La recherche globale ouvre avec cette étape (Priority: P2)

Aïssatou tape « contact » dans « Rechercher » : « 5 résultats, dans tout Guide Négo — hors connexion compris ». En tête, le lexique (*contact group*, *informal informals*) ; puis la FAQ, avec « Vérifié le… » ; puis les passages des documents. Une ligne dit que les sessions de négociation, les réunions de la Francophonie et le Pavillon n'y sont pas encore.

**Why this priority** : un point d'entrée unique, qui profite des trois contenus de cette étape et de l'étape 1. Chacun reste trouvable sans elle.

**Independent Test** : avec le réseau puis en mode avion, chercher « contact », « badge » et un mot du guide ; vérifier les trois groupes, leur ordre, leur compte et la ligne de ce qui n'est pas couvert ; ouvrir un résultat de chaque groupe.

**Acceptance Scenarios**

1. **Given** l'écran « Rechercher », **When** on tape au moins deux caractères, **Then** les résultats paraissent groupés — « Lexique », « FAQ », « Documents » — chaque groupe avec son compte, le mot cherché marqué dans chaque résultat.
2. **Given** les résultats, **When** on lit l'en-tête, **Then** il donne le nombre total et dit si la recherche a porté sur ce qui est sur le téléphone seulement ou aussi sur le serveur.
3. **Given** n'importe quelle recherche, **When** on lit l'écran, **Then** une ligne dit ce que la recherche ne couvre pas encore : les sessions de négociation, les réunions de la Francophonie et le Pavillon.
4. **Given** aucune connexion, **When** on cherche, **Then** le lexique et la FAQ sont couverts en entier, les documents par leur titre, leur résumé et le texte des copies gardées ; l'écran dit que le texte des documents non téléchargés n'a pas été cherché.
5. **Given** un résultat, **When** on le touche, **Then** il ouvre l'entrée du lexique, l'entrée de FAQ, ou le document au passage trouvé.
6. **Given** une recherche sans résultat, **When** on lit l'écran, **Then** il le dit, et propose de soumettre le terme au lexique.

---

### User Story 5 — Dire qu'une réponse ne va pas (Priority: P2)

Dans une entrée de FAQ, Moussa touche « Non » à « Cette réponse vous a-t-elle aidée ? » ; la feuille « Qu'est-ce qui manque ? » lui offre trois choix d'une touche ; il choisit « Trop vague ». Plus tard, dans une autre entrée, il sait que la règle a changé cette session : « Dépassé ou faux », « La règle a changé », une précision, « Envoyer ». Sans réseau, l'envoi part au retour. La réponse reste affichée jusqu'à la décision de l'expert.

**Why this priority** : c'est ce qui garde la FAQ juste pendant la COP (ADR-012). La lecture vit sans lui.

**Independent Test** : sans compte, toucher « Non » et « Dépassé ou faux » et constater qu'ils mènent à la connexion ; connecté, répondre « Oui » à une entrée, « Non » puis « Trop vague » à une autre, « Non » puis « Dépassée ou fausse » à une troisième, signaler « Dépassé ou faux » avec deux motifs et une précision en mode avion ; rétablir le réseau et retrouver les trois retours dans le back-office, le troisième et le quatrième dans la file des experts.

**Acceptance Scenarios**

1. **Given** une entrée, **When** on touche « Oui », **Then** la ligne devient « Merci. » avec une coche verte.
2. **Given** une entrée, **When** on touche « Non », **Then** la feuille « Qu'est-ce qui manque ? » offre « Trop vague », « Ne répond pas à ma question » et « Dépassée ou fausse » — ce dernier annoncé « Part chez les experts de l'IFDD » —, dit « Votre retour est anonyme. » et se ferme au premier choix, ou par « Annuler ».
3. **Given** « Dépassé ou faux », **When** on l'ouvre, **Then** la feuille dit qu'un expert relit la réponse et qu'elle reste affichée jusqu'à sa décision, offre les trois motifs « La règle a changé », « C'est faux », « La source ne dit pas cela » — un ou plusieurs —, un champ facultatif de 600 caractères au plus, « Envoyer » et « Annuler » ; « Envoyer » reste inactif tant qu'aucun motif n'est choisi.
4. **Given** aucune connexion, **When** on envoie un retour ou un signalement, **Then** il est gardé sur le téléphone et part au retour du réseau, une seule fois.
5. **Given** une personne sans compte, **When** elle touche « Oui », « Non » ou « Dépassé ou faux », **Then** l'écran lui propose de se connecter ou de créer un compte, et la ramène à l'entrée ensuite.
6. **Given** un retour ou un signalement, **When** il arrive, **Then** il ne porte ni le nom ni le compte de la personne dans ce que voient les experts, et ne modifie jamais l'entrée.
7. **Given** une même entrée, **When** la même personne répond deux fois à « Cette réponse vous a-t-elle aidée ? », **Then** seule la dernière réponse compte.

---

### User Story 6 — Poser une question à un expert (Priority: P2)

Aïssatou, admise par son code d'invitation, ne trouve pas qui coordonne son groupe sur l'adaptation. « Poser une question à un expert » : elle choisit la thématique « Adaptation », écrit sa question, laisse cochée « Ma question, anonymisée, pourra rejoindre la FAQ », envoie. L'écran dit « Envoyé. Un expert vous répond dès que possible ». Deux jours plus tard, un courriel lui dit qu'un expert a répondu ; elle retrouve la réponse dans « Mes questions ». Moussa, sans code, voit le verrou « Réservé aux négociatrices et négociateurs » et « Saisir mon code d'invitation ».

**Why this priority** : c'est le lien vivant entre les négociatrices et les experts, mais la FAQ publiée vit sans lui.

**Independent Test** : avec un compte admis, poser une question avec consentement et une sans, dont une en mode avion ; répondre depuis le back-office ; retrouver les réponses côté téléphone ; tenter avec un compte non admis et sans compte.

**Acceptance Scenarios**

1. **Given** une personne admise, **When** elle ouvre « Poser une question à un expert », **Then** l'écran porte son badge, le texte d'aide, la thématique à choisir parmi les thématiques de négociation, le champ de 600 caractères au plus, la case de consentement cochée par défaut, et « Envoyer », inactif tant que le champ est vide.
2. **Given** une question envoyée, **When** l'écran s'affiche, **Then** il récapitule la question, la thématique, l'heure d'envoi avec le fuseau de la COP, l'état « Envoyé », et n'annonce aucun délai de réponse.
3. **Given** aucune connexion, **When** elle envoie, **Then** la question part au retour du réseau, une seule fois, et l'écran le dit.
4. **Given** une personne sans accès négociateur — sans compte ou sans code —, **When** elle touche « Poser une question à un expert », **Then** le verrou « Réservé aux négociatrices et négociateurs » s'affiche, avec « Saisir mon code d'invitation » et « Revenir à la FAQ », et rappelle que la FAQ reste ouverte à tous.
5. **Given** ses questions, **When** elle ouvre « Mes questions », **Then** elle voit chacune avec son état — « Envoyé », « Répondue », « Ajoutée à la FAQ » — et, pour une question répondue, la réponse, son auteur et sa date ; cela se lit sans réseau une fois lu.
6. **Given** une question à laquelle un expert répond, **When** la réponse est enregistrée, **Then** un courriel le dit à la personne, et la réponse paraît dans « Mes questions » à la relecture suivante.
7. **Given** une question envoyée sans le consentement, **When** un expert la traite, **Then** il ne peut pas la promouvoir en FAQ.

---

### User Story 7 — Proposer un terme qui manque (Priority: P3)

Aïssatou cherche « placeholder text » : rien. « Proposer « placeholder text » » : le terme est prérempli, elle écrit où elle l'a entendu, et envoie. Un expert le définit et le publie ; un courriel le lui dit.

**Why this priority** : le lexique s'enrichit de la salle, mais le lexique publié vit sans.

**Independent Test** : avec un compte quelconque, proposer un terme depuis « aucun résultat » et depuis la feuille du lecteur, dont un en mode avion ; le retrouver dans le back-office, le publier ; tenter sans compte.

**Acceptance Scenarios**

1. **Given** « aucun résultat », **When** on touche « Proposer « … » », **Then** la feuille « Proposer un terme » s'ouvre, le terme prérempli et modifiable, avec le champ « Où l'avez-vous entendu ? » de 600 caractères au plus, « Proposer ce terme aux experts » et « Annuler ».
2. **Given** une personne connectée à n'importe quel compte, **When** elle envoie, **Then** la proposition part, sans réseau au retour de la connexion, une seule fois ; l'écran dit qu'un expert la définit avant publication.
3. **Given** une personne sans compte, **When** elle veut proposer, **Then** l'écran lui propose de se connecter ou de créer un compte, et garde le terme saisi.
4. **Given** un terme déjà proposé et encore en attente, **When** une autre personne le propose, **Then** sa proposition s'ajoute à la même sans créer de doublon dans la file.
5. **Given** un terme proposé puis publié, **When** l'expert publie l'entrée qui en naît, **Then** chaque personne qui l'a proposé reçoit un courriel qui le dit.

---

### User Story 8 — La feuille du lecteur ouvre le lexique (Priority: P3)

En « Texte agrandi », Aïssatou touche *global goal on adaptation* : la feuille « Lexique » montre la famille, le terme, la traduction, la définition et « Dans ce guide : p. 18, 26, 59 à 61. » ; « Ouvrir dans le lexique » ouvre l'entrée complète.

**Why this priority** : la feuille existe et dit aujourd'hui que le lexique est à venir ; ce récit la raccorde.

**Independent Test** : dans le guide gardé, en mode avion, toucher trois termes anglais présents dans le lexique et un absent.

**Acceptance Scenarios**

1. **Given** un terme touché présent dans le lexique — par son terme, son sigle ou une variante —, **When** la feuille s'ouvre, **Then** elle montre sa famille, le terme, la traduction, la définition, les pages du document où il paraît, « Ouvrir dans le lexique » et « Revenir au texte ».
2. **Given** un terme touché absent du lexique, **When** la feuille s'ouvre, **Then** elle dit qu'il n'y est pas encore, garde les pages du document, et propose de le soumettre.
3. **Given** aucune connexion, **When** on touche un terme, **Then** la feuille se remplit depuis le lexique gardé.

---

### User Story 9 — Le back-office tient la FAQ, le parcours et le lexique (Priority: P1)

Mariam, administratrice, rédige une entrée de FAQ, lui rattache deux sources dont une page du guide et trois questions liées ; le Dr Koffi Mensah, expert, la relit, date sa vérification, et Mariam la publie. Il traite la file : deux signalements « Dépassé ou faux » sur la même entrée — il la met « à revoir », corrige la réponse, redate la vérification ; une question d'Aïssatou — il répond, puis la promeut en FAQ en retirant ce qui pourrait l'identifier ; un terme proposé — il le définit et le publie.

**Why this priority** : sans contenu publié, rien ne se lit ; les trois écrans du téléphone en dépendent. Le chargement des données d'essai sert le poste local seulement.

**Independent Test** : partir d'une base sans contenu, créer une rubrique de FAQ par semis, rédiger et publier une entrée, une étape de parcours et un terme ; relire le téléphone ; traiter un élément de chaque file ; vérifier qu'une personne sans le rôle requis n'accède à rien, adresse forgée comprise.

**Acceptance Scenarios**

1. **Given** une entrée de FAQ, **When** on la rédige, **Then** on saisit sa rubrique, sa question, sa réponse, ses sources — document de la bibliothèque avec section ou pages, ou référence extérieure, avec une citation facultative —, ses questions liées, et on la laisse « Brouillon », la publie, la dépublie ou la met « À revoir ».
2. **Given** une entrée, **When** un expert confirme son contenu, **Then** il date sa vérification — le jour même par défaut — et son nom est gardé ; la date affichée sur le téléphone change à la relecture suivante.
3. **Given** une entrée « À revoir », **When** le téléphone la lit, **Then** elle reste affichée avec sa dernière date de vérification et la ligne « Un expert relit cette réponse », jusqu'à ce que l'expert la revérifie ou la dépublie.
4. **Given** le parcours, **When** on le tient, **Then** on crée, ordonne, publie ou retire ses groupes et ses étapes, avec leur complément facultatif.
5. **Given** un terme du lexique, **When** on le rédige, **Then** on saisit sa famille, le terme anglais, son sigle, sa traduction, sa définition, sa phrase « Entendu en salle », ses sources, ses termes liés et ses variantes d'écriture, et on le laisse « Brouillon », le publie ou le met « À revoir ».
6. **Given** la file des experts, **When** un expert l'ouvre, **Then** il voit, par entrée, les signalements « Dépassé ou faux » avec leurs motifs et précisions, les réponses « Non » avec leur motif, les questions en attente et les termes proposés, les plus anciens d'abord, et les clôt après traitement.
7. **Given** une question avec consentement, **When** l'expert la promeut, **Then** une entrée de FAQ en brouillon naît de la question et de la réponse, que l'expert réécrit avant publication ; rien n'y porte le nom de la personne.
8. **Given** un terme proposé, **When** l'expert le traite, **Then** il crée l'entrée à partir de la proposition, ou rejette la proposition avec un motif.
9. **Given** une personne sans le rôle requis, **When** elle ouvre ces pages ou forge leur adresse, **Then** l'accès est refusé, par l'API.

---

### Edge Cases

- **Le lexique compte quelques centaines d'entrées.** La recherche à la frappe reste immédiate sur le téléphone le plus modeste visé ; le rail et la liste restent fluides.
- **Une saisie d'une seule lettre.** Le lexique montre les entrées commençant par cette lettre ; la recherche globale attend deux caractères.
- **Une saisie en français** (« groupe de contact ») ramène l'entrée anglaise ; l'écran dit « Aussi dans les traductions françaises ».
- **Une saisie avec apostrophe courbe, trait d'union ou accents** : ignorés à la comparaison.
- **Un sigle ambigu** (NDC, CDN) : cherché en anglais comme en français, il ramène la même entrée.
- **Une entrée de lexique dépubliée qui était en favori** : elle quitte les favoris affichés ; republiée, elle y revient.
- **Un terme lié ou une question liée non publiés** : ils n'apparaissent pas.
- **Une source qui renvoie à un document dépublié ou remplacé** : elle affiche son titre et le dit, ou suit « Remplacé par… » de l'étape 1.
- **Une entrée modifiée pendant qu'on la lit** : la version affichée reste jusqu'à la sortie de l'écran.
- **Deux appareils du même compte cochent et décochent la même étape** : le dernier geste reçu l'emporte ; le perdant voit l'état retenu et un message, jamais une perte muette.
- **Un compte se déconnecte** : ses favoris et ses coches synchronisés restent lisibles sur ce téléphone comme ceux d'une personne sans compte ; ses questions quittent le téléphone.
- **Le retrait de l'accès négociateur** : « Mes questions » se ferme sur le verrou ; les questions déjà envoyées restent traitées.
- **Une écriture en attente d'envoi quand la personne se déconnecte** — retour, signalement, question, terme : elle est abandonnée, et l'écran de déconnexion l'a dit avant.
- **Le premier lancement sans jamais avoir eu de réseau** : la FAQ, le parcours et le lexique disent qu'ils se chargeront à la première connexion — la règle de 0a : ce qui n'a jamais été lu ne s'invente pas.
- **Une réponse longue, un terme long, un titre de source long** : tout tient à 360 px sans défilement horizontal.
- **Une personne abuse des retours** : un compte ne compte qu'une fois par entrée pour « Oui / Non », et ses signalements sont limités en nombre par jour.

## Requirements *(mandatory)*

### Lexique

- **FR-001** : Le bouton « Aa » de chaque écran de Guide Négo DOIT ouvrir le lexique, champ actif ; la fermeture DOIT ramener à l'écran d'origine.
- **FR-002** : Le lexique publié DOIT se lire en entier sans réseau — liste, entrées, recherche, favoris — dès que l'application l'a lu une fois.
- **FR-003** : La recherche du lexique DOIT porter sur le terme anglais, son sigle, ses variantes d'écriture et sa traduction française ; suivre chaque frappe ; ignorer casse, accents et ponctuation ; tolérer au moins une faute de frappe par mot ; et proposer « Vous cherchiez peut-être » quand la correspondance n'est qu'approchée.
- **FR-004** : La liste DOIT ranger les entrées par lettre avec leur compte, offrir un rail A–Z (toucher ou glisser, lettres vides grisées) et des filtres par famille.
- **FR-005** : Une entrée DOIT porter sa famille, son terme, sa traduction, sa définition, et, quand elles existent, la phrase « Entendu en salle », le sigle développé, ses sources et ses termes liés ; elle DOIT offrir « Favori » et « Partager ».
- **FR-006** : Chaque entrée DOIT porter une désignation stable et lisible, qui ne change pas quand son texte est corrigé, par laquelle d'autres écrans l'ouvrent — la feuille du lecteur, le parcours, et les types de réunion de l'étape 3a.
- **FR-007** : Le lexique DOIT montrer les derniers termes consultés, gardés sur le téléphone.
- **FR-008** : « Mes termes favoris » DOIT se lire sans réseau ; les favoris DOIVENT rester sur le téléphone sans compte, et suivre le compte sur ses appareils avec un compte.
- **FR-009** : « Aucun résultat » DOIT dire le nombre d'entrées, que le terme n'y est pas encore, et offrir de le proposer et de parcourir la liste.
- **FR-010** : Proposer un terme DOIT être ouvert à toute personne connectée, quel que soit son accès ; la proposition DOIT partir sans réseau au retour de la connexion, une seule fois ; une proposition d'un terme déjà en attente DOIT s'y ajouter ; la publication de l'entrée qui en naît DOIT être annoncée par courriel à chaque personne qui l'a proposé (tranché le 25/09).

### FAQ

- **FR-011** : La FAQ publiée DOIT se lire en entier sans réseau — rubriques, entrées, sources, questions liées, recherche — dès que l'application l'a lue une fois.
- **FR-012** : L'accueil de la FAQ DOIT montrer le nombre de réponses publiées, la ligne de synchronisation, la recherche, les rubriques avec leur compte, le parcours et sa progression, « Les plus lues » et « Poser une question à un expert ».
- **FR-013** : Une entrée publiée DOIT porter « Vérifié le » et la date de sa dernière vérification ; une entrée sans vérification datée NE DOIT pas pouvoir être publiée.
- **FR-014** : Une source DOIT désigner soit un document de la bibliothèque avec sa section ou ses pages, soit une référence extérieure ; elle PEUT porter une citation ; une source vers un document DOIT l'ouvrir à l'endroit cité.
- **FR-015** : La recherche de la FAQ DOIT porter sur la question et la réponse, suivre la frappe et tolérer fautes et accents, sans réseau.
- **FR-016** : « Les plus lues » DOIVENT être les trois entrées les plus ouvertes sur trente jours, comptées sans identifier la personne ; à défaut de lectures comptées, l'ordre choisi par l'IFDD. Une lecture hors connexion n'a pas à être comptée.
- **FR-017** : « Cette réponse vous a-t-elle aidée ? » DOIT recevoir « Oui » ou « Non » ; « Non » DOIT ouvrir « Qu'est-ce qui manque ? » et ses trois choix ; le troisième, « Dépassée ou fausse », DOIT rejoindre la file des experts ; une personne ne compte qu'une fois par entrée.
- **FR-018** : « Dépassé ou faux » DOIT offrir trois motifs, au moins un requis, et une précision facultative de 600 caractères au plus ; il DOIT rejoindre la file des experts et NE DOIT jamais modifier ni masquer l'entrée.
- **FR-019** : Retours et signalements DOIVENT être réservés aux personnes connectées, quel que soit leur accès — sans compte, le geste mène à la connexion ; ils DOIVENT être anonymes pour les experts, partir sans réseau au retour de la connexion une seule fois, et être limités en nombre par personne et par jour (tranché le 25/09).
- **FR-019 bis** : Une entrée « À revoir » DOIT rester affichée, avec sa dernière date de vérification et la ligne « Un expert relit cette réponse », jusqu'à sa revérification ou sa dépublication (tranché le 25/09).

### Questions aux experts

- **FR-020** : Poser une question DOIT être réservé aux personnes disposant de l'accès négociateur, vérifié par l'API ; les autres DOIVENT voir le verrou de l'étape 0b.
- **FR-021** : Une question DOIT porter une thématique de négociation, un texte de 600 caractères au plus et le choix, coché par défaut, qu'elle rejoigne la FAQ anonymisée ; elle DOIT partir sans réseau au retour de la connexion, une seule fois.
- **FR-022** : Aucun écran NE DOIT promettre un délai de réponse.
- **FR-023** : « Mes questions » DOIT montrer les questions de la personne, leur état et, une fois répondues, la réponse, son auteur et sa date ; elles DOIVENT se lire sans réseau une fois lues, et quitter le téléphone à la déconnexion. La réponse d'un expert DOIT être annoncée à la personne par courriel (tranché le 25/09).
- **FR-024** : Une question sans consentement NE DOIT pas pouvoir être promue en FAQ ; une question promue DOIT donner une entrée en brouillon, réécrite par l'expert, qui ne porte ni le nom ni le compte de la personne.

### Parcours « Ma première COP »

- **FR-025** : Le parcours DOIT montrer ses groupes et leurs étapes dans l'ordre fixé par l'IFDD, la progression globale, la prochaine étape non cochée et le compte de chaque groupe.
- **FR-026** : Une étape DOIT se cocher et se décocher en touchant sa ligne entière ; cochée, son libellé passe en gris, jamais barré.
- **FR-027** : Une étape PEUT porter un complément : un détail, une origine, ou un lien de lecture vers un document (section ou page), une entrée de FAQ ou une entrée du lexique.
- **FR-028** : Les coches DOIVENT rester sur le téléphone sans réseau et sans compte ; avec un compte, elles DOIVENT se synchroniser au retour du réseau, un conflit entre appareils étant dit et jamais perdu en silence ; à la connexion, les coches locales DOIVENT rejoindre celles du compte.
- **FR-029** : Le parcours publié DOIT se lire en entier sans réseau.

### Recherche globale

- **FR-030** : L'écran « Rechercher » DOIT chercher à la fois dans le lexique, la FAQ et les documents, et rendre des groupes nommés et comptés, dans cet ordre, le mot cherché marqué.
- **FR-031** : L'écran DOIT dire ce qu'il ne couvre pas encore — les sessions de négociation, les réunions de la Francophonie, le Pavillon — et, sans réseau, que le texte des documents non téléchargés n'a pas été cherché.
- **FR-032** : Sans réseau, la recherche globale DOIT couvrir le lexique et la FAQ en entier, les titres, résumés et éditeurs de la bibliothèque gardée, et le texte des copies gardées ; avec le réseau, elle DOIT couvrir aussi le texte de tous les documents accessibles à la personne.
- **FR-033** : Un résultat DOIT ouvrir l'entrée du lexique, l'entrée de FAQ, ou le document au passage trouvé.

### Lecteur

- **FR-034** : La feuille du terme touché du lecteur DOIT montrer l'entrée du lexique correspondante — par terme, sigle ou variante —, les pages du document où il paraît, « Ouvrir dans le lexique » et « Revenir au texte » ; sans entrée, elle DOIT le dire et offrir de proposer le terme.

### Back-office

- **FR-035** : Le back-office DOIT permettre de rédiger, publier, dépublier et mettre « À revoir » les entrées de FAQ et du lexique, de tenir les groupes et étapes du parcours, et de dater la vérification d'une entrée de FAQ ; la date et le nom de l'expert qui vérifie DOIVENT être gardés.
- **FR-036** : La file des experts DOIT rassembler les signalements « Dépassé ou faux », les réponses « Dépassée ou fausse », les questions en attente et les termes proposés ; chaque élément DOIT se clore avec une issue, gardée avec son auteur et sa date.
- **FR-037** : Dater une vérification, répondre à une question, la promouvoir en FAQ, traiter un signalement et traiter un terme proposé DOIVENT être réservés au rôle `expert` ; rédiger, publier, dépublier et mettre « À revoir » DOIVENT l'être au rôle `expert` et aux administrateurs de Guide Négo ; chaque permission DOIT être vérifiée avec sa portée, par l'API, adresse forgée comprise (plan, R10).
- **FR-038** : Les rubriques de la FAQ et les familles du lexique DOIVENT être des données — vocabulaires semés en base —, jamais des traductions.
- **FR-039** : Le contenu de production DOIT venir du back-office ; les données d'essai de `donnees-savoir.md` et `donnees-lexique.md` DOIVENT se charger sur un poste local par un script rejouable, jamais en production.

### Ce qui vaut pour toute l'étape

- **FR-040** : FAQ, parcours et lexique DOIVENT se relire par des listes « modifié depuis » avec leur empreinte, pour que le téléphone ne retélécharge que ce qui a changé ; toute donnée lue hors connexion DOIT dire l'heure de sa lecture.
- **FR-041** : Les écritures faites sans réseau — coches, favoris, retours, signalements, questions, termes proposés — DOIVENT passer par la file d'écritures différées existante, partir une seule fois et dire leur échec.
- **FR-042** : Les écrans DOIVENT tenir à 360 px sans défilement horizontal, dans les deux thèmes, cibles tactiles comprises, et porter leurs quatre états : chargement, vide, erreur, accès refusé.
- **FR-043** : Les écrans DOIVENT être bâtis sur les composants de Guide Négo ; tout composant nouveau DOIT être ajouté à la page interne des composants. Le back-office garde ceux de l'ePavillon.
- **FR-044** : Les textes d'interface nouveaux DOIVENT vivre dans les fichiers de traduction de leur écran, en français et en anglais ; les contenus de FAQ, de parcours et de lexique sont des données.
- **FR-045** : Les écarts entre la maquette et cette étape DOIVENT être inscrits dans [05-design.md](../../docs/AppNego/05-design.md).

### Key Entities

- **Rubrique de FAQ** : un terme d'un vocabulaire semé, avec son libellé, son pictogramme et son ordre.
- **Entrée de FAQ** : rubrique, question, réponse, état (Brouillon, Publié, À revoir), date de dernière vérification et expert qui l'a faite, ordre des « plus lues » à défaut de lectures, questions liées ; sa question d'origine si elle vient d'une question d'expert.
- **Source** : attachée à une entrée de FAQ ou du lexique ; un document de la bibliothèque avec section ou pages, ou une référence extérieure avec adresse facultative ; citation facultative.
- **Lecture d'une entrée** : un compte anonyme par entrée et par jour, qui sert « Les plus lues ».
- **Retour sur une entrée** : « Oui » ou « Non », motif du « Non », entrée visée, une voix par personne ; anonyme pour les experts.
- **Signalement « Dépassé ou faux »** : entrée visée, un ou plusieurs motifs, précision, état de traitement, issue, expert et date ; anonyme pour les experts.
- **Question à un expert** : auteure, thématique, texte, consentement, état (Envoyée, Répondue, Ajoutée à la FAQ), réponse, expert, dates, entrée de FAQ née d'elle.
- **Groupe et étape du parcours** : libellés, ordre, état de publication ; l'étape porte un complément facultatif — détail, origine, ou lien vers un document, une entrée de FAQ ou une entrée du lexique.
- **Coche du parcours** : une étape cochée par une personne ; sur le téléphone sans compte, en base avec un compte.
- **Famille du lexique** : un terme d'un vocabulaire semé (Réunions, Textes, Thématiques).
- **Entrée du lexique** : désignation stable, famille, terme anglais, sigle, variantes d'écriture, traduction, définition, phrase « Entendu en salle », sources, termes liés, état.
- **Favori** : une entrée du lexique retenue par une personne ; sur le téléphone sans compte, en base avec un compte.
- **Terme proposé** : terme, contexte, personnes qui l'ont proposé, état, issue — entrée créée ou rejet motivé.

### Ce que le modèle change

Le SQL se modifie d'abord, par une migration rejouable qui ne détruit pas la base locale, puis le code s'écrit. Rien n'existe : toutes les entités ci-dessus sont à créer, dans le schéma que le plan désigne. Les deux vocabulaires — rubriques de FAQ et familles du lexique — se sèment dans les vocabulaires de référence existants.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : Après une seule ouverture avec le réseau, en mode avion et application relancée, « contact group » est trouvé dans le lexique en tapant « contact grup », en moins d'une seconde après la dernière frappe.
- **SC-002** : Chaque entrée de FAQ publiée affiche sa date de vérification, avec et sans réseau ; aucune entrée sans date n'est publiée.
- **SC-003** : Sur les 19 entrées d'essai, chaque terme cherché avec une faute d'une lettre, ou par sa traduction française, ramène son entrée dans les trois premiers résultats.
- **SC-004** : Les résultats du lexique suivent la frappe sans décalage perceptible sur un lexique de 500 entrées, sur un téléphone de milieu de gamme.
- **SC-005** : FAQ, parcours et lexique se lisent en entier en mode avion ; une modification publiée depuis le back-office paraît sur le téléphone à la relecture suivante, sans retéléchargement du reste.
- **SC-006** : Une coche posée hors connexion sur un appareil se retrouve sur un second appareil du même compte dans la minute qui suit le retour du réseau des deux ; aucune coche n'est perdue sans que l'écran le dise.
- **SC-007** : Un signalement, un retour, une question ou un terme proposé hors connexion arrive exactement une fois dans le back-office après le retour du réseau.
- **SC-008** : Aucune personne sans accès négociateur ne peut poser une question, et aucune personne sans le rôle requis ne peut lire la file des experts ni publier — les tests d'intégration le prouvent, adresse forgée comprise.
- **SC-009** : Aucun écran de la file des experts ne montre le nom ou le compte de l'auteur d'un retour, d'un signalement, ou d'une question promue.
- **SC-010** : La recherche globale rend les trois groupes pour « contact » avec et sans réseau, et dit dans les deux cas ce qu'elle ne couvre pas.
- **SC-011** : Depuis n'importe quel écran de Guide Négo, le lexique s'ouvre en un toucher.
- **SC-012** : Les écrans sont fidèles à la maquette à 360 px, en thème clair et sombre, aux écarts inscrits près.

## Assumptions

- **Le contenu est global à Guide Négo**, pas propre à une COP : la même FAQ, le même parcours, le même lexique servent chaque session ; une entrée qui ne vaut que pour une COP le dit dans son texte.
- **Les contenus sont rédigés en français**, dans les colonnes multilingues du modèle ; l'interface anglaise les affiche en français à défaut de traduction.
- **Les favoris et les coches sans compte restent sur le téléphone** ; ADR-003 et le domaine (« Visiteur, compte : garde ses favoris ») le permettent.
- **« Partager »** passe par le partage du téléphone, avec le lien de l'entrée et son titre ; sans partage disponible, il copie le lien.
- **« Mes termes favoris »** s'ouvre depuis le lexique ; la maquette n'en dessine pas le chemin.
- **« Les plus lues »** ne compte que les ouvertures faites avec le réseau : les compter hors connexion ajouterait une écriture de plus pour peu de précision.
- **La recherche globale ne cherche pas le texte des documents non téléchargés sans réseau** : elle ne l'a pas, et le dit.
- **Les données d'essai** sont incomplètes (une seule entrée de FAQ a sa réponse ; cinq termes sont « rédigés » sans source) : elles se chargent telles quelles, les manques restent visibles, et rien ne s'invente pour les combler.
- **La branche** `013-guide-nego-faq-lexique` part de `main`, qui porte les étapes 0 à 1b. L'étape 3a se construit en parallèle et ouvrira une entrée du lexique par sa désignation stable (FR-006).
