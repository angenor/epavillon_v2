# Feature Specification: Guide Négo — thématiques, « Ma journée » et profil (étape 0c)

**Feature Branch**: `010-guide-nego-accueil-profil`

**Created**: 2026-09-22

**Status**: Draft — deux questions tranchées par le commanditaire le 22/09

**Input**: Étape 0c de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md) — ce qui accueille une personne entrée dans Guide Négo. À sa première entrée, elle choisit une ou plusieurs thématiques de négociation, modifiables ensuite ; ce sont des données de la base, jamais des traductions. L'accueil « Ma journée » rassemble ce qui la concerne aujourd'hui ; chaque ligne porte son origine ; tant que les modules suivants n'existent pas, leurs blocs affichent leur état vide, et chaque étape suivante remplit le sien. Profil et réglages : thématiques, téléchargements et place occupée, thème, à propos, confidentialité et consentements. Ses choix la suivent d'un appareil à l'autre et restent lisibles hors connexion.

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| Les écrans de cette étape | [02-socle.html](../../docs/AppNego/design/ecrans/02-socle.html) : « 06 Thématiques », « 07 Ma journée », « 08 Ma journée hors connexion », « 11 Profil », « 12 À propos » |
| Le système de design et les composants | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) et [design/passation/](../../docs/AppNego/design/passation/), repris à l'étape 0a. Un composant qui manque se crée ici et s'ajoute à la page interne des composants |
| Les mots employés à l'écran | [design/lexique.md](../../docs/AppNego/design/lexique.md) : « Thématique » — jamais « thème », « catégorie » ni « track » ; « Thème » ne désigne que l'apparence claire ou sombre ; « Sessions de négociation », « Réunions de la Francophonie », « Pavillon de la Francophonie » en toutes lettres, le mot « Programme » seul banni ; « négociatrices et négociateurs », jamais « utilisateurs » ; « Hors connexion — lu à… », « Synchronisé à… » |
| Les écarts entre le système et les pages | Tranchés dans [05-design.md](../../docs/AppNego/05-design.md) — dont l'écart 32, qui nomme cette étape : la ligne « Dernière synchronisation » du profil ne porte pas de fuseau ; et l'écart 40, posé ici : les trois interrupteurs de consentement de l'écran « 12 À propos » ne sont pas livrés |
| Les décisions | [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) tout ce qui se lit se lit hors connexion · [ADR-008](../../docs/AppNego/adr/008-trois-agendas-jamais-confondus.md) trois agendas jamais confondus · [ADR-007](../../docs/AppNego/adr/007-le-reseau-distingue-pas-le-genre.md) le réseau distingue, pas le genre · [ADR-016](../../docs/AppNego/adr/016-vert-et-jaune-fonces-police-hors-charte.md) et [ADR-018](../../docs/AppNego/adr/018-direction-typographique-quatre-onglets.md) l'identité propre de l'application |
| Les deux arbitrages du 22/09 | **Les thématiques de négociation sont un vocabulaire à elles** — `negotiation_theme` dans `reference.taxonomy_terms`, semé des dix thématiques de la maquette, comme `activity_theme` l'est pour les activités : une donnée, jamais une liste dans le code. Les sessions (3a) et les alertes (3b) s'y rattacheront ; le Pavillon garde `activity_theme` et n'est **jamais** filtré par « mes thématiques ». **Les textes qui engagent ont une source unique** — un fichier Markdown `fr`/`en` par texte, portant sa version, embarqué dans l'API et servi par une route publique ; le site et l'application servent le même texte, et la version servie remplace le réglage `PRIVACY_POLICY_VERSION` |
| Le modèle | `docs/database/020_reference.sql` (vocabulaires et termes), `030_identity.sql` (personnes, profils de négociateur, consentements), `100_negotiations.sql` (espaces, accès, appartenance au réseau), `010_platform.sql` (réglages). **Ce qui manque s'ajoute au SQL d'abord** — voir *Ce que le modèle ne porte pas encore* |
| Les principes | Constitution : XI « Hors connexion d'abord », XII « Confiance », XIII « Un design propre et borné » ; contrainte « Trois agendas, jamais confondus » ; règle des deux sortes de textes multilingues — *si un administrateur peut le modifier, ce n'est pas une traduction* |
| Ce qui est déjà livré | Étape 0a : la coquille, la mise en page, la barre d'onglets, l'en-tête, le bandeau hors connexion, le réglage du thème, la garde des données lues et son heure de lecture, la page interne des composants. Étape 0b : le compte, l'admission, la ligne « Mon accès » et la déconnexion du profil. **Rien de cela ne se réécrit ici** |

**Hors périmètre** : la recherche globale (étape 2) ; le centre de notifications, la cloche de l'en-tête et le réglage des notifications par thématique (étape 3b) ; les documents et « Mes documents » (étape 1) ; les trois agendas eux-mêmes — sessions de négociation (3a), réunions de la Francophonie (4), Pavillon (5) ; les échanges et l'annuaire (étape 6) ; **les trois interrupteurs de consentement** de la maquette, qui ne paraîtront qu'avec l'étape donnant un effet à chacun ; **les pages `/confidentialite` et `/conditions-utilisation` du site**, aujourd'hui liées mais absentes — elles se construiront côté ePavillon sur la même route publique ; **un écran d'administration des textes**, qui pourra venir plus tard derrière cette même route sans rien défaire ; **l'administration des vocabulaires** — le site n'en a pour aucun, un terme s'ajoute par le SQL.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Choisir ses thématiques, et les retrouver partout (Priority: P1)

Aïssatou vient d'entrer dans Guide Négo avec le code reçu sur WhatsApp. Dernière étape de son entrée, l'application lui demande ce qu'elle suit : adaptation, finance, genre, article 6… Elle en coche deux, voit le récapitulatif « 2 thématiques suivies — Adaptation, Genre », et valide par « Suivre ces thématiques ». L'écran lui a dit qu'elle pourra les changer dans son profil. Le lendemain, sur la tablette prêtée par sa délégation, elle se connecte : ses deux thématiques sont là, sans qu'elle ait rien refait.

**Why this priority** : c'est le critère de sortie de l'étape, et ce qui commandera le filtre des sessions de négociation (3a) et les alertes (3b). Sans lui, ces étapes n'ont rien à filtrer.

**Independent Test** : sur un téléphone, entrer dans l'application, cocher deux thématiques, valider ; se connecter sur un second appareil avec le même compte et constater les deux mêmes thématiques ; en changer une depuis le profil et la voir changer sur le premier appareil à sa prochaine lecture.

**Acceptance Scenarios**

1. **Given** une personne connectée qui n'a encore choisi aucune thématique, **When** elle termine son entrée, **Then** l'écran « Mes thématiques » lui est proposé, sous-titré « Une ou plusieurs », avec l'indicateur « Étape 3 sur 3 — Thématiques » et l'aide qui annonce que ses sessions de négociation et ses alertes suivront ces thématiques, et qu'elle pourra les changer dans son profil.
2. **Given** l'écran des thématiques, **When** elle touche n'importe où sur une ligne, **Then** la thématique se coche ou se décoche, et le récapitulatif du pied dit combien elle en suit et lesquelles.
3. **Given** aucune thématique cochée, **When** elle cherche à valider, **Then** la validation n'est pas possible et l'écran dit qu'il en faut au moins une.
4. **Given** deux thématiques cochées, **When** elle valide, **Then** son choix est enregistré sur son compte et l'application ouvre « Ma journée ».
5. **Given** une personne qui a choisi ses thématiques sur un premier appareil, **When** elle se connecte sur un second, **Then** elle retrouve exactement les mêmes, sans avoir à les ressaisir.
6. **Given** une personne qui suit deux thématiques, **When** elle ouvre « Mes thématiques » depuis son profil, **Then** l'écran s'ouvre avec ses thématiques déjà cochées, et non sur une liste vierge ; la modification prend effet aussitôt validée.
7. **Given** la liste des thématiques, **When** on regarde les libellés affichés, **Then** ils viennent de la base — du vocabulaire propre aux filières de négociation, distinct de celui qui classe les activités du Pavillon —, dans la langue de la personne avec repli sur le français, et aucun ne figure dans un fichier de traduction ni dans le code.
8. **Given** une thématique retirée du vocabulaire par l'IFDD, **When** la personne rouvre l'écran, **Then** cette thématique ne lui est plus proposée, sans que son choix sur les autres soit touché ni qu'une erreur paraisse.
9. **Given** les thématiques suivies, **When** on regarde ce qu'elles commandent à cette étape, **Then** elles ne filtrent encore rien : elles sont enregistrées et affichées, et les étapes 3a et 3b s'en serviront.

---

### User Story 2 — « Ma journée » s'ouvre, même quand il n'y a encore rien (Priority: P1)

L'onglet Accueil s'appelle « Ma journée ». Il rassemble ce qui concerne la personne aujourd'hui : sa prochaine session de négociation, les changements du jour, ses trois agendas, ses documents récents, et l'accès au lexique. À cette étape, aucun de ces modules n'existe encore : chaque bloc montre son état vide — ce qu'il n'y a pas, et quand cela reviendra —, et rien ne ressemble à une panne. En salle, sans réseau, le même écran s'ouvre, coiffé du bandeau « Hors connexion — lu à 11:35 ».

**Why this priority** : c'est la porte d'entrée quotidienne de l'application, et le cadre que les étapes 1, 3a, 4 et 5 rempliront chacune à leur tour. Un cadre mal posé se paye cinq fois.

**Independent Test** : ouvrir l'onglet Accueil sur un compte neuf, en ligne puis en mode avion, et vérifier que les cinq blocs paraissent dans l'ordre, chacun avec son état vide, sans message d'erreur.

**Acceptance Scenarios**

1. **Given** une personne connectée, **When** elle ouvre l'onglet Accueil, **Then** l'écran est titré « Ma journée » et sous-titré par le jour — « Jeudi 12 novembre » — ; le fuseau, nommé d'après le lieu de l'édition, vient avec les heures de sessions à l'étape 3a.
2. **Given** « Ma journée » sans aucune donnée, **When** l'écran s'affiche, **Then** les blocs paraissent dans cet ordre — prochaine session de négociation, changements du jour, aujourd'hui vos trois agendas, documents récents, accès au lexique — et chacun des quatre premiers porte son état vide propre : un titre, ce qui manque, quand cela reviendra, et une sortie quand il y en a une.
3. **Given** un bloc vide, **When** on le lit, **Then** rien n'y évoque une erreur, un échec ou une indisponibilité du service.
4. **Given** « Ma journée », **When** on regarde l'en-tête, **Then** l'avatar de la personne y figure à gauche et ouvre « Profil et réglages », le bouton « Aa » ouvre le lexique, et aucune cloche de notifications n'y paraît — elle vient à l'étape 3b.
5. **Given** le réseau coupé, **When** la personne ouvre « Ma journée », **Then** l'écran s'affiche entier, le bandeau dit « Hors connexion — lu à … », et le bloc des changements dit ses lignes « connues à … » plutôt que « mises à jour à … ».
6. **Given** le réseau revenu, **When** la lecture aboutit, **Then** le bandeau cède la place à « Synchronisé à … » sans que la personne ait à recharger l'écran.
7. **Given** une personne sans compte, **When** elle ouvre l'onglet Accueil, **Then** l'écran s'affiche sans erreur et l'invite à créer un compte ou à se connecter, sans avatar ni thématiques.
8. **Given** les blocs de « Ma journée », **When** ils porteront des lignes aux étapes suivantes, **Then** chaque ligne dira son agenda d'origine — Sessions de négociation, Réunions de la Francophonie, Pavillon de la Francophonie — et le mot « Programme » seul n'y paraîtra jamais.
9. **Given** les activités du Pavillon, **When** elles rejoindront « Ma journée » à l'étape 5, **Then** elles y paraîtront toutes, sans être filtrées par les thématiques suivies.
10. **Given** l'accès direct au lexique en bas de l'écran, **When** la personne le touche, **Then** le lexique s'ouvre, avec l'état vide qu'il porte jusqu'à l'étape 2.

---

### User Story 3 — Profil et réglages, tout ce qui la concerne au même endroit (Priority: P2)

Depuis son avatar ou depuis l'onglet Ressources, Aïssatou ouvre « Profil et réglages ». Elle y voit son nom, son pays, ce qu'elle suit — thématiques, téléchargements et place occupée, accès —, le réglage de l'apparence, l'entrée vers « À propos », l'heure de sa dernière synchronisation, et le bouton pour se déconnecter, qui lui rappelle que ses téléchargements restent sur son téléphone.

**Why this priority** : c'est le seul chemin pour revenir sur ses thématiques, et le point d'entrée de tout réglage ajouté par les étapes suivantes. Il s'appuie sur l'écran livré à l'étape 0b, qu'il complète sans le refaire.

**Independent Test** : ouvrir le profil depuis les deux chemins, changer ses thématiques, basculer l'apparence, lire la place occupée, se déconnecter et constater que rien n'est perdu.

**Acceptance Scenarios**

1. **Given** une personne connectée, **When** elle touche son avatar depuis « Ma journée » ou la ligne « Profil et réglages » de l'onglet Ressources, **Then** le même écran s'ouvre, titré de son nom et sous-titré de son pays.
2. **Given** le profil, **When** on le parcourt, **Then** il porte le groupe « Mon suivi » — « Mes thématiques » avec les thématiques suivies en valeur, « Mes téléchargements » avec le nombre de documents gardés et la place occupée, « Mon accès » livré à l'étape 0b —, le groupe « Affichage » avec le thème, le groupe « Application » avec l'entrée « À propos » et la ligne « Dernière synchronisation », puis la déconnexion.
3. **Given** « Mes thématiques », **When** la personne l'ouvre, **Then** elle retrouve l'écran de choix, ses thématiques cochées, et revient au profil une fois sa modification validée.
4. **Given** aucun document gardé — ce qui est le cas à cette étape —, **When** elle ouvre « Mes téléchargements », **Then** l'écran dit qu'aucun document n'est gardé sur ce téléphone et à quelle étape ils arriveront, et montre malgré tout la place occupée par l'application sur l'appareil.
5. ~~**Given** de la place occupée, **When** elle demande de la libérer, **Then** une confirmation dit ce qui sera effacé et ce qui restera lisible hors connexion, et la place affichée diminue une fois le geste confirmé.~~ **Passé à l'étape 1** (22/09, écart inscrit ci-dessous) : le geste ne libère que les documents téléchargés, et il n'y en a pas encore.
6. **Given** le réglage du thème livré à l'étape 0a, **When** il paraît dans le groupe « Affichage », **Then** ses trois valeurs — Clair, Sombre, Système — fonctionnent comme avant, et ce réglage reste propre à cet appareil.
7. **Given** la ligne « Dernière synchronisation », **When** on la lit, **Then** elle porte l'heure de la dernière lecture réussie sans fuseau, selon l'écart 32, et n'est pas une action.
8. **Given** le profil d'une personne admise, **When** on lit la ligne « Mon accès », **Then** elle porte l'état de son accès, jamais un rôle genré — comme tranché à l'étape 0b.
9. **Given** le réseau coupé, **When** elle ouvre son profil, **Then** l'écran s'affiche avec ce qui a déjà été lu et l'heure de cette lecture.
10. **Given** le groupe « Notifications par thématique » que dessine la maquette, **When** on ouvre le profil à cette étape, **Then** il n'y figure pas : il vient avec le centre de notifications, à l'étape 3b.

---

### User Story 4 — À propos, et les textes qui engagent (Priority: P3)

Depuis le profil, Aïssatou ouvre « À propos ». Elle y lit qui édite l'application, d'où viennent les sessions de négociation, et ce qui reste sur son téléphone. Trois lignes ouvrent la politique de confidentialité, les conditions d'utilisation et les licences — les deux premières dans le dessin de Guide Négo, avec leur version, et lisibles ensuite sans réseau.

**Why this priority** : ces textes sont aujourd'hui promis par le site sans exister — l'inscription y renvoie et enregistre un accord sous une version qui vient d'un réglage, pas d'un texte. Une source unique corrige les deux applications d'un coup.

**Independent Test** : ouvrir l'écran et chacun des textes, en ligne puis sans réseau ; vérifier que le texte servi et sa version sont les mêmes pour le site et pour l'application ; modifier un texte sans changer sa version et constater que le contrôle refuse.

**Acceptance Scenarios**

1. **Given** le profil, **When** la personne ouvre « À propos », **Then** l'écran est titré ainsi, sous-titré du nom de l'application, de sa version et de l'édition en cours, et porte le paragraphe qui nomme l'IFDD et l'OIF.
2. **Given** l'écran, **When** on le parcourt, **Then** il porte l'étiquette qui dit d'où viennent les sessions de négociation, le paragraphe de confidentialité, et le groupe des textes — et aucun interrupteur d'accord.
3. **Given** le groupe des textes, **When** la personne ouvre la politique de confidentialité ou les conditions d'utilisation, **Then** le texte s'affiche en entier dans le dessin de Guide Négo, avec sa version.
4. **Given** un texte déjà lu, **When** la personne le rouvre sans réseau, **Then** il s'affiche en entier avec l'heure de sa lecture.
5. **Given** le même texte demandé par le site et par l'application, **When** on compare, **Then** c'est le même texte et la même version — une seule source les sert.
6. **Given** un texte modifié sans que sa version change, **When** le contrôle s'exécute, **Then** il refuse.
7. **Given** une personne qui accorde un consentement, **When** on regarde ce qui est consigné, **Then** la version enregistrée est celle que l'API a servie, et non une valeur de configuration.
8. **Given** les licences, **When** la personne les ouvre, **Then** elles disent la police et les bibliothèques embarquées, et n'appellent aucun accord.
9. **Given** une personne sans compte, **When** elle ouvre « À propos », **Then** elle lit l'écran et ses textes.
10. **Given** le paragraphe de confidentialité, **When** on le lit, **Then** il distingue ce qui reste sur le téléphone — favoris, téléchargements — de ce qui suit le compte — thématiques et accords.

---

### Edge Cases

- **Aucune thématique n'existe en base.** L'écran de choix ne montre pas une liste vide sans explication : il dit qu'aucune thématique n'est proposée pour l'instant et laisse continuer sans bloquer l'entrée.
- **La personne quitte l'application au milieu du choix.** À sa prochaine ouverture, elle retrouve l'étape des thématiques, sans avoir à refaire les précédentes.
- **Elle décoche tout depuis le profil.** La modification n'est pas enregistrable : au moins une thématique est exigée, et l'écran le dit avant qu'elle ne valide.
- **Deux appareils modifient les thématiques en même temps, tous deux en ligne.** Le dernier choix enregistré fait foi, et l'autre appareil l'adopte à sa prochaine lecture, sans message d'échec.
- **Un choix pris hors connexion arrive après un autre, fait ailleurs entre-temps.** Le choix parti en retard **n'écrase pas** le plus récent : il est abandonné, l'application relit l'état vrai et le dit — « Vos thématiques ont changé sur un autre appareil ». Rien ne se perd en silence, et rien ne revient en arrière.
- **Le choix est fait en mode avion.** Il s'affiche aussitôt comme choisi, part au retour du réseau, et n'est enregistré qu'une fois même si l'application a été fermée entre-temps.
- **Le stockage du téléphone est refusé ou plein.** L'application s'ouvre malgré tout, la place occupée dit ce qu'elle peut, et rien ne lève d'erreur à l'écran.
- **La date du jour change pendant que l'écran est ouvert** — minuit, ou changement de fuseau en voyage. « Ma journée » se remet à la bonne date à sa prochaine lecture, sans laisser une date fausse en titre.
- **La lecture échoue alors que le réseau est là.** L'écran garde ce qui avait été lu et le dit ; l'état d'erreur n'est employé que là, jamais pour un module simplement absent.
- **Un texte est demandé dans une langue où il n'existe pas.** Le français est servi, et l'écran le dit plutôt que de rendre une page vide.
- **La version d'un texte change après qu'une personne l'a accepté.** Son accord reste consigné avec la version qu'elle a lue ; rien n'est réputé accordé pour la nouvelle.
- **Une thématique suivie est désactivée puis réactivée.** Le suivi n'est pas perdu entre-temps ; il reparaît avec elle.

## Requirements *(mandatory)*

### Les thématiques

- **FR-001** : Les thématiques de négociation DOIVENT former un vocabulaire à elles, distinct de celui qui classe les activités du Pavillon, semé des dix thématiques de la maquette et modifiable sans toucher au code. Aucun libellé, aucune couleur et aucune liste NE DOIT être écrit dans un fichier de traduction ni dans le code.
- **FR-002** : Le système DOIT résoudre le libellé d'une thématique dans la langue de la personne, avec repli sur le français.
- **FR-003** : Le système DOIT proposer le choix des thématiques en dernière étape du parcours d'entrée, après l'admission, avec l'indicateur d'étapes et le sous-titre « Une ou plusieurs ».
- **FR-004** : Une personne DOIT pouvoir cocher et décocher plusieurs thématiques, toute la ligne servant de cible, et DOIT voir en permanence combien elle en suit et lesquelles.
- **FR-005** : Le système DOIT exiger au moins une thématique pour enregistrer un choix, empêcher la validation à zéro, et dire pourquoi.
- **FR-006** : Les thématiques suivies DOIVENT être attachées au compte de la personne, et non à l'appareil : elles se retrouvent à l'identique sur tout appareil où elle se connecte.
- **FR-007** : Une personne DOIT pouvoir modifier ses thématiques depuis « Profil et réglages », sur le même écran que le premier choix, ses thématiques déjà cochées.
- **FR-008** : Le système NE DOIT proposer que les thématiques actives ; une thématique désactivée cesse d'être proposée sans effacer les choix portant sur les autres.
- **FR-009** : Le choix des thématiques DOIT être possible sans réseau : l'écran s'affiche à partir de la dernière liste lue, avec son heure de lecture, et l'enregistrement part au retour du réseau, une seule fois.
- **FR-009 bis** : Un choix parti en retard NE DOIT JAMAIS écraser un choix plus récent fait sur un autre appareil. Le système DOIT le détecter, abandonner le choix en retard, relire l'état vrai et l'annoncer à la personne. Une écriture en attente NE DOIT jamais partir sous un autre compte que celui qui l'a prise, et la file DOIT se vider à la déconnexion.
- **FR-010** : Le système NE DOIT proposer le choix des thématiques qu'à une personne disposant d'un compte ; une visiteuse sans compte ne le voit pas.
- **FR-011** : Les thématiques suivies NE DOIVENT filtrer aucune liste à cette étape ; les activités du Pavillon NE DOIVENT jamais être filtrées par elles, à aucune étape.

### « Ma journée »

- **FR-012** : L'onglet Accueil DOIT porter l'écran « Ma journée », titré ainsi et sous-titré par le jour, recalculé au retour au premier plan. Le fuseau n'y est pas nommé à cette étape : celui de l'appareil trompe, et celui de l'édition vient en 3a ([R11](research.md)).
- **FR-013** : L'écran DOIT présenter ses blocs dans un ordre fixe : prochaine session de négociation, changements du jour, aujourd'hui vos trois agendas, documents récents, accès au lexique.
- **FR-014** : Chaque bloc dont le module n'est pas encore livré DOIT afficher son propre état vide — ce qui manque, quand cela reviendra, et une sortie quand il y en a une — et NE DOIT en aucun cas afficher une erreur, disparaître, ni rester en chargement.
- **FR-015** : Chaque ligne de « Ma journée » DOIT porter son agenda d'origine ; le mot « Programme » employé seul NE DOIT paraître ni à l'écran ni dans les noms exposés par l'API.
- **FR-016** : L'en-tête de « Ma journée » DOIT porter l'avatar de la personne, qui ouvre « Profil et réglages », et le bouton « Aa », qui ouvre le lexique. La cloche des notifications n'est pas livrée à cette étape.
- **FR-017** : « Ma journée » DOIT s'afficher en entier sans réseau, à partir de ce qui a déjà été lu, coiffée du bandeau « Hors connexion — lu à … » ; le bloc des changements DOIT alors dire ses lignes « connues à … ».
- **FR-018** : Au retour du réseau, l'écran DOIT passer à « Synchronisé à … » sans rechargement ni geste.
- **FR-019** : « Ma journée » DOIT rester accessible sans compte, et inviter alors à créer un compte ou à se connecter.

### Profil et réglages

- **FR-020** : « Profil et réglages » DOIT être atteignable depuis l'avatar de « Ma journée » et depuis la ligne dédiée de l'onglet Ressources, et mener au même écran.
- **FR-021** : L'écran DOIT porter le nom et le pays de la personne, puis les groupes « Mon suivi », « Affichage » et « Application », et la déconnexion en pied, sans défaire ce que l'étape 0b y a posé.
- **FR-022** : La ligne « Mes thématiques » DOIT montrer les thématiques suivies en valeur et ouvrir leur modification.
- **FR-023** : La ligne « Mes téléchargements » DOIT montrer le nombre de documents gardés et la place occupée sur l'appareil ; tant que les documents n'existent pas, elle DOIT dire son état vide et montrer malgré tout la place occupée par l'application.
- **FR-024** : ~~Une personne DOIT pouvoir libérer la place occupée, après une confirmation qui dit ce qui sera effacé et ce qui reste lisible sans réseau.~~ **Passé à l'étape 1** : libérer ne vise que les documents téléchargés, jamais les lectures ; à cette étape il n'y a rien à libérer, donc **aucun bouton**.
- **FR-025** : Le réglage du thème DOIT rester celui de l'étape 0a — Clair, Sombre, Système — et rester propre à l'appareil.
- **FR-026** : La ligne « Dernière synchronisation » DOIT porter l'heure de la dernière lecture réussie sans fuseau, et n'être qu'une information.
- **FR-027** : Le profil DOIT s'afficher sans réseau, avec l'heure de sa dernière lecture.

### À propos et les textes qui engagent

- **FR-028** : L'écran « À propos » DOIT porter le nom de l'application, sa version, l'édition en cours, l'éditeur, l'origine des sessions de négociation, le paragraphe de confidentialité et le groupe des textes.
- **FR-029** : Le paragraphe de confidentialité DOIT distinguer ce qui reste sur le téléphone — favoris et téléchargements — de ce qui suit le compte — thématiques et accords donnés.
- **FR-030** : La politique de confidentialité et les conditions d'utilisation DOIVENT avoir une source unique, en français et en anglais, portant sa version, servie par une route publique de l'API — le même texte et la même version pour le site et pour l'application.
- **FR-031** : La version servie par cette route DOIT être celle consignée avec un consentement, en remplacement du réglage de configuration employé aujourd'hui.
- **FR-032** : Un texte modifié sans que sa version change DOIT faire échouer le contrôle.
- **FR-033** : Les deux textes DOIVENT s'afficher en entier dans le dessin de Guide Négo, porter leur version, et rester lisibles hors connexion une fois lus, avec l'heure de leur lecture.
- **FR-034** : Les licences DOIVENT dire la police et les bibliothèques embarquées ; elles ne sont pas un texte d'engagement, n'appellent aucun accord, et se tiennent avec la construction.
- **FR-035** : Aucun interrupteur d'accord NE DOIT paraître à cette étape : les trois que dessine la maquette n'ont encore aucun effet — pas de mesure d'usage, pas de notifications, pas d'annuaire —, et un interrupteur sans effet trompe. Chacun paraîtra avec l'étape qui lui donne un effet, et l'écart est inscrit.
- **FR-036** : L'enregistrement d'un accord et sa relecture, avec leur version, **sont déjà portés par l'existant** — cette étape n'a rien à construire pour eux, et n'ajoute aucune route d'accord. Elle change seulement d'où vient la version (FR-031).
- **FR-037** : Sans compte, l'écran « À propos » et ses textes DOIVENT rester lisibles.

### Ce qui vaut pour tous les écrans de l'étape

- **FR-038** : Tout écran de lecture de cette étape DOIT s'afficher sans réseau à partir de ce qui a déjà été lu, et porter l'heure de cette lecture.
- **FR-039** : Toute écriture faite sans réseau DOIT partir au retour du réseau et n'arriver qu'une fois.
- **FR-040** : Chaque écran DOIT porter ses quatre états : chargement, vide, erreur, accès refusé.
- **FR-041** : Chaque écran DOIT tenir à 360 px sans défilement horizontal, dans les deux thèmes, cibles tactiles comprises.
- **FR-042** : Les écrans DOIVENT être bâtis sur les composants de Guide Négo ; aucun composant ni jeton du site NE DOIT être employé, et tout composant nouveau DOIT être ajouté à la page interne des composants.
- **FR-043** : Les textes d'interface DOIVENT vivre dans des fichiers de traduction découpés par écran, en français et en anglais ; les libellés de thématiques n'y figurent jamais.
- **FR-044** : Les écrans de cette étape DOIVENT employer les mots du lexique, « Thème » ne désignant jamais une thématique.

### Key Entities

- **Thématique de négociation** : une filière suivie par une personne — un code, un libellé multilingue, un ordre d'affichage, un état actif ou non. Vocabulaire propre aux négociations, distinct de celui des activités du Pavillon, modifiable sans redéploiement.
- **Suivi de thématique** : le lien entre une personne et une thématique. Porté par le compte, jamais par l'appareil. **Une seule vérité** : soit les spécialisations déjà portées par le profil de négociateur, soit une table propre — tranché au plan, jamais les deux.
- **Réglage d'apparence** : le thème choisi — clair, sombre, système. Propre à l'appareil.
- **Texte qui engage** : politique de confidentialité, conditions d'utilisation — un fichier par langue, portant sa version, servi par l'API. Source unique du site et de l'application.
- **Accord** : ce qu'une personne accorde ou retire, pour un objet nommé, à une date, sous la version de texte servie par l'API. L'historique n'est jamais écrasé. Aucun n'est offert à l'écran à cette étape.
- **Ligne de « Ma journée »** : ce qui concerne la personne aujourd'hui, portant toujours son agenda d'origine, son heure et son état. Aucune n'existe à cette étape ; les étapes 1, 3a, 4 et 5 les apportent.
- **Place occupée** : ce que l'application garde sur l'appareil, exprimé en clair et libérable en un geste.

### Ce que le modèle ne porte pas encore

Le SQL se modifie d'abord, puis la base se recharge, puis le code s'écrit — jamais l'inverse.

| Manque | Ce qu'il faut | Où |
|---|---|---|
| Le vocabulaire des thématiques de négociation | `negotiation_theme`, déclaré à choix multiple, et ses dix termes semés | `020_reference.sql` — à côté d'`activity_theme`, qui reste celui du Pavillon |
| Le suivi d'une thématique par une personne | Le lien personne ↔ thématique. **À trancher au plan, une vérité et pas deux** : `identity.negotiator_profiles.specializations` existe déjà — un tableau de codes de termes, sans intégrité référentielle, attaché au profil de négociateur — ou une table propre. Le choix se justifie dans le plan | `030_identity.sql` |
| Les textes qui engagent | Aucune table : un fichier Markdown `fr`/`en` par texte, portant sa version, embarqué dans l'API, servi par une route publique, et un contrôle qui refuse un texte modifié sans nouvelle version | Hors base ; la version servie remplace le réglage `PRIVACY_POLICY_VERSION` |
| Les objets d'accord propres à Guide Négo | Rien à semer tant qu'aucun accord n'est offert. Les consentements et leur vue d'état courant existent déjà | `030_identity.sql` |

Rien n'est à ajouter pour « Ma journée » à cette étape : ses blocs sont vides, et les agendas viennent avec les étapes 3a, 4 et 5.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : Une négociatrice choisit ses thématiques et arrive sur « Ma journée » en moins d'une minute, sans aide.
- **SC-002** : Les thématiques choisies sur un premier appareil se retrouvent à l'identique sur un second, dès la première ouverture après connexion, sans aucun geste.
- **SC-003** : « Ma journée » s'ouvre avec ses cinq blocs, chacun dans son état vide, sans qu'aucun message d'erreur ne paraisse — en ligne comme en mode avion.
- **SC-004** : Réseau coupé, chacun des écrans de l'étape s'affiche et dit l'heure de sa dernière lecture.
- **SC-005** : Un choix de thématiques fait en mode avion est enregistré au retour du réseau, une seule fois, y compris si l'application a été fermée entre-temps.
- **SC-005 bis** : Un choix fait hors connexion sur un état devenu périmé est abandonné, jamais appliqué : l'état le plus récent survit, et la personne apprend pourquoi son choix n'a pas pris.
- **SC-006** : Les écrans nouveaux sont fidèles à la maquette à 360 px, en thème clair et en thème sombre, sans défilement horizontal — aux écarts inscrits près.
- **SC-007** : Aucun libellé de thématique n'apparaît dans un fichier de traduction ni dans le code — la recherche le prouve.
- **SC-008** : Le site et l'application reçoivent le même texte et la même version pour la politique de confidentialité et les conditions d'utilisation ; un texte modifié sans nouvelle version fait échouer le contrôle.
- **SC-009** : Tout accord consigné porte la version servie par l'API, et non une valeur de configuration.
- **SC-010** : La place occupée affichée correspond à ce que l'appareil rapporte, à l'unité affichée près. *Sa diminution après une libération passe à l'étape 1, avec les documents.*
- **SC-011** : Chaque bloc vide de « Ma journée » dit ce qui manque et à quel moment cela viendra ; aucun ne reste en chargement.
- **SC-012** : Aucun composant ni jeton du site n'est employé sous le dossier de Guide Négo, et chaque composant nouveau figure sur la page interne des composants.

## Assumptions

- **Le thème reste propre à l'appareil.** « Ses choix la suivent d'un appareil à l'autre » vise les thématiques et les accords, qui sont des données du compte ; l'apparence claire ou sombre s'applique avant toute connexion et dépend de l'appareil et de l'heure de la journée — elle reste où l'étape 0a l'a mise.
- **Au moins une thématique.** La maquette écrit « Une ou plusieurs » sans dessiner le cas de zéro : la validation à zéro est refusée, à la première entrée comme à la modification.
- **Divergence assumée avec la maquette, écran « 12 À propos ».** Son paragraphe de confidentialité écrit « Vos thématiques, favoris et téléchargements restent sur votre téléphone », alors que le critère de sortie exige que les thématiques suivent la personne d'un appareil à l'autre. Le texte est corrigé : les thématiques et les accords suivent le compte, les favoris et les téléchargements restent sur le téléphone.
- **Les trois interrupteurs d'accord ne sont pas livrés** — écart 40, arbitré le 22/09. Aucun n'a d'effet à cette étape : la mesure d'usage n'existe pas, les notifications viennent à 3b, l'annuaire après le MVP. Chacun paraîtra avec l'étape qui lui donne un effet ; le moyen de consigner et de relire un accord, lui, est livré ici.
- **Un éditeur de textes pourra venir plus tard** derrière la même route publique, sans rien défaire de ce qui est livré.
- **La cloche des notifications n'est pas livrée.** La maquette la dessine dans l'en-tête de « Ma journée » avec son compteur ; elle arrive avec le centre de notifications, à l'étape 3b, de même que le groupe « Notifications par thématique » du profil.
- **Le sous-titre de « Ma journée » ne porte que le jour** (révisé le 22/09, [R11](research.md)) : le fuseau de l'appareil, nommé, trompe ; celui du lieu de l'édition vient en 3a. Les heures de lecture — « lu à 11:35 » — n'en portent jamais (écart 32).
- **« Mes téléchargements » s'ouvre sur son état vide** : les documents arrivent à l'étape 1. La place occupée, elle, est réelle dès maintenant — la coquille et les données lues en occupent déjà.
- **Pas de bouton « Libérer » à cette étape — écart inscrit le 22/09, arbitré par le commanditaire.** Le geste ne videra que les documents téléchargés (étape 1), **jamais les lectures** : elles pèsent quelques dizaines de kilo-octets, et ce sont elles — l'accès, les thématiques, le vocabulaire, demain l'agenda du jour — qui font marcher l'application sans réseau. À cette étape il n'y a donc rien à libérer, et un geste sans effet trompe (constitution, XII), comme les interrupteurs d'accord. Le scénario 5 du récit 3 et FR-024 passent à l'étape 1, qui rangera les documents là où leur suppression se mesure.
- **Les deux textes naissent « en attente du texte de l'IFDD »** (arbitré le 22/09). Aucun texte n'est écrit par un outil : il serait servi publiquement, et chaque inscription enregistrerait un accord à un texte que personne n'a lu — rien de produit par une IA ne se publie sans validation humaine. La route le dit, l'écran affiche « Texte en préparation par l'IFDD », et la version reste `2026-01` ; le texte fourni prend sa date d'entrée en vigueur pour version.
- **« À propos » n'affirme que ce qui est vrai aujourd'hui** (arbitré le 22/09) : pas « repris avec l'accord du secrétariat » — la demande est en cours, et l'étiquette de source ne parlera d'accord qu'une fois l'accord obtenu ; pas d'hébergeur nommé — le commanditaire dira lequel avant qu'on l'écrive ; pas de restitutions, qui n'existent pas. « L'édition en cours » est la COP climat qui se tient, sinon la prochaine, lue par la route publique des éditions ; la version est l'identifiant de la construction servie.
- **L'écran des thématiques sert deux fois** : première entrée et modification. C'est le même écran, avec ou sans l'indicateur d'étapes.
- **L'avatar** est celui du compte de l'ePavillon quand il existe, et les initiales de la personne sinon.
- **Les pages `/confidentialite` et `/conditions-utilisation` du site** restent à construire côté ePavillon, sur la route livrée ici. Cette étape ne les fait pas, mais ne laisse plus le texte manquant.
