# Feature Specification: Guide Négo — la bibliothèque de documents et le lecteur (étape 1)

**Feature Branch**: `011-guide-nego-documents`

**Created**: 2026-09-22

**Status**: Draft — relue par le commanditaire le 22/09 : une question tranchée, trois corrections, un arbitrage

**Input**: Étape 1 de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md). Toute personne, même sans compte, cherche et filtre les documents publics par type, thématique et COP ; un document concerne zéro ou plusieurs thématiques ; il est un fichier ou un lien externe, jamais les deux. La fiche montre résumé, version, date, éditeur, et le bandeau « Remplacé par… » qui mène au document à jour. Toute personne, même sans compte, télécharge un document public pour le lire sans réseau avec sa progression ; avec un compte, le favori ; « Mes documents » et la place occupée, tout retirer en un geste *(corrigé le 22/09 : la feuille de route réservait le téléchargement au compte)*. Les documents réservés demandent l'accès négociateur et s'effacent du téléphone à la déconnexion. Le lecteur : page en lecture, barre repliée et dépliée, sommaire, recherche dans le document, progression et reprise, taille du texte, thème ; un terme anglais touché ouvre une feuille basse, vide tant que le lexique n'existe pas ; une page peut porter la note de correction d'un expert ; l'état « non téléchargé et pas de réseau » le dit. Back-office : publier un document en une journée, poser et retirer une note de correction.

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| Les écrans de cette étape | [03-documents.html](../../docs/AppNego/design/ecrans/03-documents.html) : « 01 Bibliothèque » à « 04 Hors connexion », les six fiches « 05a » à « 05f », « 06a Mes documents » et « 06b » sa confirmation · [04-lecteur.html](../../docs/AppNego/design/ecrans/04-lecteur.html) : « 01 » à « 12 », de la barre repliée à l'état « non téléchargé et pas de réseau » |
| Les données d'essai | [donnees-lecteur.md](../../docs/AppNego/design/donnees-lecteur.md) — le sommaire réel du *Guide des négociations*, la page 59, le terme *global goal on adaptation* — et les cinq documents de la bibliothèque dessinés par la maquette. Les extraits des pages 18, 26 et 68, la définition du terme et le texte de la note de correction sont inventés et ne se publient pas |
| Le système de design et les composants | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) — sections « 4 ter » Documents et « 4 quater » Lecteur — et [design/passation/](../../docs/AppNego/design/passation/). Un composant qui manque se crée ici et s'ajoute à la page interne des composants |
| Les mots employés à l'écran | [design/lexique.md](../../docs/AppNego/design/lexique.md) : états d'un document « À jour · Remplacé par… · Dépassé · Téléchargé » ; « Réservé aux négociatrices et négociateurs » ; « Expert » pour qui valide le fond ; « Thématique », jamais « thème » ; « Hors connexion — lu à… », sans fuseau ; un terme anglais reste en anglais et en italique |
| Les écarts entre le système et les pages | Tranchés dans [05-design.md](../../docs/AppNego/05-design.md) — dont l'écart 20 (le triangle rouge dit une erreur, une lecture impossible ou un passage dépassé — élargi le 22/09), 32 (heures de lecture sans fuseau), 33 (aplat rouge assombri des boutons « Tout retirer » et « Retirer ») et 39 (le verrou sans compte a trois sorties) |
| Les décisions | [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) tout ce qui se lit se lit hors connexion — documents gardés à la demande, place visible et libérable d'un geste, réservés effacés à la déconnexion · [ADR-019](../../docs/AppNego/adr/019-service-worker-engendre-cache-d-abord.md) les documents vivent hors des caches de la coquille et survivent aux déploiements · [ADR-011](../../docs/AppNego/adr/011-un-corpus-a-deux-etages.md) et [ADR-012](../../docs/AppNego/adr/012-toute-source-porte-un-etat.md) le marqueur pour l'assistant, la note de correction sur une page · [ADR-000](../../docs/AppNego/adr/000-guide-nego-est-la-face-mobile-de-l-epavillon.md) un document publié une fois sert le site et l'application |
| Ce qui est reporté ici | De l'étape 0c : le geste « Libérer » de « Mes téléchargements », son scénario et FR-024 — **il ne libère que les documents téléchargés, jamais les lectures**, qu'on range là où leur effacement se mesure aussitôt |
| Le modèle | `docs/database/100_negotiations.sql` (documents, favoris), `020_reference.sql` (types de document, thématiques de négociation), `050_media.sql` (fichiers stockés), `060_events.sql` (éditions). **Ce qui manque s'ajoute au SQL d'abord** — voir *Ce que le modèle ne porte pas encore* |
| Les arbitrages du 22/09 | (1) **Le texte du lecteur est tiré du PDF publié**, et de lui seul : l'administratrice dépose le PDF, l'application en tire le texte, les pages et le sommaire, et montre **chaque page** à l'aperçu, son rendu à côté du texte recomposé ; elle peut choisir « ouvrir tel quel » pour tout document — le repli du scan. **C'est le risque de l'étape** : le plan commence par un essai sur le vrai guide de la CdP30. (2) **Un document public se télécharge sans compte** : la feuille de route réservait le téléchargement au compte, à tort — une déléguée sans compte lirait le guide avec du réseau, jamais en salle, contre ADR-003. Le favori reste au compte ; un document réservé exige toujours l'accès. (3) **« Nouveau » dit ce que la personne n'a pas encore ouvert** : publié depuis moins de sept jours **et** jamais ouvert sur ce téléphone. (4) **Le triangle rouge de la note reste** : l'écart 20 devient « triangle rouge = erreur, lecture impossible, ou passage dépassé ». (5) **Les thématiques d'un document passent par `reference.entity_terms`**, la doctrine du modèle. (6) **Un rôle `expert`** naît ici, de portée globale, avec une permission par geste |
| Les principes | Constitution : XI « Hors connexion d'abord », XII « Confiance », XIII « Un design propre et borné » ; règle n° 8 du site — une liste du back-office est filtrée par le périmètre d'administration |
| Ce qui est déjà livré | 0a : la coquille, la barre d'onglets, l'en-tête, le bandeau hors connexion, le thème, la garde des lectures et leur heure. 0b : le compte, l'admission, le verrou `GnVerrou`, la déconnexion. 0c : « Ma journée » et son bloc « Documents récents » vide, « Mes téléchargements » et sa jauge `GnJauge`, la place mesurée. **Rien de cela ne se réécrit ici** |

**Hors périmètre** : l'indexation des documents par l'assistant, et le correctif des vecteurs qui la précède (étape 7) ; les quiz ; la proposition de documents par les membres ; **le lexique lui-même** — ses entrées, sa recherche, l'écran « Ouvrir dans le lexique » (étape 2) ; la recherche globale, qui rassemblera documents, FAQ et lexique (étape 2) ; la notification « Nouveau document » et son réglage (étape 3b) ; les documents liés à une session de négociation (étape 3a) ; **le bouton « Marquer » de la barre du lecteur**, dont la maquette ne dessine ni le résultat ni la liste — voir *Assumptions* ; l'état « Dépassé » d'une source entière et son circuit de validation (étape 7) ; les notes de correction sur un intervalle de vidéo (étape 8) ; les écrans de la bibliothèque côté site de l'ePavillon.

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Publier le guide en une journée (Priority: P1)

Le 2 novembre, l'IFDD reçoit la version finale du *Guide des négociations — CdP31*. Mariam, administratrice de Guide Négo, ouvre le back-office, crée un document, téléverse le fichier, choisit le type « Guide », laisse les thématiques vides — le guide les traverse toutes —, rattache la COP31, écrit le résumé, indique la version et l'éditeur, désigne le guide de la CdP30 comme document remplacé, le laisse public, et coche « utilisable par l'assistant ». Elle feuillette l'aperçu — chaque page rendue à côté de son texte recomposé, le sommaire repéré — en quelques minutes, puis publie. Dans l'heure, le guide est dans la bibliothèque de chaque téléphone, et l'ancien porte le bandeau « Remplacé par… ».

**Why this priority** : c'est la moitié du critère de sortie. Sans document publié, rien d'autre ne se teste sur de vraies données.

**Independent Test** : partir d'un fichier du guide et d'un lien externe, publier les deux depuis le back-office en chronométrant, puis les retrouver dans l'application — le guide avec ses pages et son sommaire, le lien avec son adresse —, et constater que l'ancien guide renvoie au nouveau.

**Acceptance Scenarios**

1. **Given** une personne disposant de la permission de publier des documents dans Guide Négo, **When** elle crée un document, **Then** le formulaire lui demande le titre, le résumé, le type, les thématiques (aucune, une ou plusieurs), la COP (aucune ou une), la version, la date du document, l'éditeur, la langue, la source — un fichier téléversé **ou** un lien externe —, le document remplacé le cas échéant, l'accès — public ou réservé —, et le marqueur « utilisable par l'assistant ».
2. **Given** le formulaire, **When** elle a fourni un fichier et saisit aussi un lien, **Then** l'enregistrement est refusé avec un message qui dit qu'un document est un fichier ou un lien, jamais les deux ; de même s'il n'a ni l'un ni l'autre.
3. **Given** un fichier téléversé, **When** son traitement se termine, **Then** le back-office montre un aperçu : nombre de pages, taille, sommaire repéré, et **chaque page** rendue à côté de son texte recomposé, qui se feuillette page à page en quelques minutes — l'ordre de lecture, les en-têtes et pieds de page écartés, les notes, les tableaux, les césures et les italiques s'y vérifient avant de publier.
4. **Given** l'aperçu d'un document dont le texte se recompose mal — ou bien, mais que l'administratrice préfère montrer à l'identique —, **When** elle choisit « ouvrir tel quel », **Then** le document entier se publie en pages d'origine, sans taille de texte, termes touchables ni recherche dans le texte, et sa fiche le dit ; elle peut revenir sur ce choix sans republier le fichier.
5. **Given** un document enregistré mais pas publié, **When** on regarde la bibliothèque de l'application, **Then** il n'y paraît pas.
6. **Given** un document désigné comme remplaçant un autre, **When** il est publié, **Then** l'ancien reste dans la bibliothèque, marqué « Remplacé », et sa fiche porte le bandeau qui mène au nouveau.
7. **Given** un document déjà remplacé par un autre, **When** on tente de le désigner comme remplacé par un troisième, **Then** le back-office le refuse et nomme le remplaçant existant.
8. **Given** le marqueur « utilisable par l'assistant », **When** on crée un document, **Then** il est décoché par défaut, et le formulaire dit qu'il ne prendra effet qu'avec l'assistant.
9. **Given** une administratrice dont le périmètre ne couvre pas Guide Négo, **When** elle ouvre la liste des documents ou forge l'adresse d'un document, **Then** elle ne voit rien et ne modifie rien.
10. **Given** un document publié, **When** l'administratrice le dépublie, **Then** il quitte la bibliothèque, et les copies gardées sur les téléphones s'effacent à leur prochaine lecture de la bibliothèque.
11. **Given** un document publié, **When** on veut changer son fichier, **Then** le back-office propose de publier une nouvelle version qui le remplace, plutôt que de modifier en silence un fichier que des téléphones ont déjà gardé.

---

### User Story 2 — Trouver un document et lire sa fiche, même sans compte (Priority: P1)

À la veille de la COP, un conseiller d'un ministère, sans compte, ouvre l'onglet Ressources puis « Documents de négociation ». Il tape « adaptation », filtre sur la thématique Adaptation et la COP30, trouve la note technique sur le bilan de la CdP30. La fiche lui montre le résumé, la version 1.2, la date, l'éditeur — et un bandeau : ce document est remplacé par le guide de la CdP31, qu'il ouvre d'un toucher. Il voit aussi, dans la liste, un résumé pour les décideurs marqué « Réservé » : sa fiche lui dit ce qu'il faut pour l'ouvrir.

**Why this priority** : l'accès sans compte est la promesse publique de la bibliothèque, et le bandeau « Remplacé par… » empêche de négocier sur un texte périmé.

**Independent Test** : sans compte, rechercher un mot, combiner deux filtres, obtenir une liste vide puis la vider des filtres, ouvrir une fiche remplacée et suivre le bandeau, ouvrir la fiche d'un document réservé.

**Acceptance Scenarios**

1. **Given** une personne sans compte, **When** elle ouvre la bibliothèque, **Then** l'écran est titré « Documents de négociation », sous-titré du nombre de documents, et porte la recherche « Rechercher un titre, un mot du texte » et trois filtres : « Type », « Thématique », « COP ».
2. **Given** la liste, **When** on lit une ligne, **Then** elle porte le type, le titre, le nombre de pages, la taille, l'éditeur et l'année — ou « Page web » pour un lien —, et ses marques : « Téléchargé », « À jour », « Nouveau », « Réservé », « Lien externe », « Remplacé par… ».
3. **Given** un filtre ouvert, **When** on le lit, **Then** il s'ouvre en feuille basse, permet plusieurs choix, affiche le nombre de documents par valeur, dit « Aucun choix : tous les types », et son bouton annonce le nombre de résultats avant de l'appliquer — « Afficher 1 document ».
4. **Given** les valeurs des filtres « Type » et « Thématique », **When** on les regarde, **Then** elles viennent des vocabulaires de la base, dans la langue de la personne avec repli sur le français ; aucune ne figure dans un fichier de traduction ni dans le code. Les valeurs du filtre « COP » sont les éditions auxquelles un document au moins est rattaché.
5. **Given** deux filtres qui ne laissent aucun document, **When** la liste se vide, **Then** l'écran nomme ce qui a été cherché — « Aucun bulletin sur le genre » —, rappelle combien de documents existent en tout et offre « Retirer les filtres ».
6. **Given** une recherche, **When** elle porte sur un mot du texte d'un document, **Then** le document est trouvé même si le mot n'est ni dans son titre ni dans son résumé.
7. **Given** un document public qui est un fichier, **When** on ouvre sa fiche, **Then** elle porte le type et l'éditeur, les marques, le résumé, les thématiques — ou « Aucune : ce document traverse toutes les négociations » —, les détails — version, date, éditeur, langue, COP le cas échéant —, et les actions.
8. **Given** un document remplacé, **When** on ouvre sa fiche, **Then** le bandeau « Remplacé par » nomme le document à jour, sa date de publication, son nombre de pages et s'il est déjà téléchargé ; l'action principale devient « Ouvrir le document à jour », et « Télécharger quand même cette version » reste possible.
9. **Given** un document qui est un lien externe, **When** on ouvre sa fiche, **Then** elle dit « S'ouvre dans le navigateur — réseau nécessaire, rien n'est gardé sur le téléphone », porte l'adresse du site et l'heure d'ajout, et son action est « Ouvrir dans le navigateur » ; il ne se télécharge pas.
10. **Given** un document réservé et une personne sans accès négociateur, **When** elle ouvre sa fiche, **Then** le titre, le type, les marques, le nombre de pages et la taille restent visibles ; le résumé, les thématiques et le téléchargement sont cachés ; l'écran dit « Réservé aux négociatrices et négociateurs », combien d'autres documents restent ouverts à tous, et offre les sorties du verrou livré en 0b — trois sans compte (écart 39), « Saisir mon code d'invitation » et « Continuer en visiteur » avec un compte.
11. **Given** une personne sans compte, **When** elle ouvre un document public qui est un fichier, **Then** elle peut le lire avec le réseau et le télécharger pour le lire sans réseau, comme avec un compte ; seul « Favori » l'invite à créer un compte ou à se connecter (arbitré le 22/09).
12. **Given** un document publié depuis moins de sept jours et jamais ouvert sur ce téléphone, **When** on regarde la liste, **Then** il porte la marque jaune « Nouveau » et compte dans « n nouveaux » ; **When** la personne l'ouvre une première fois, **Then** il perd la marque sur ce téléphone. Un document publié depuis plus de sept jours ne la porte jamais.
13. **Given** le bouton « Partager », **When** on le touche, **Then** le partage du téléphone s'ouvre avec le titre et l'adresse de la fiche ; une personne qui reçoit l'adresse d'un document réservé tombe sur le verrou, jamais sur son contenu.

---

### User Story 3 — Télécharger et lire en salle, sans réseau (Priority: P1)

Aïssatou, négociatrice admise, télécharge le guide la veille au soir, dans sa chambre d'hôtel. Le lendemain, en salle de négociation, le réseau est saturé. Elle ouvre Guide Négo : la bibliothèque dit « Hors connexion — lu à 11:35 » et « 1 lisible maintenant ». Elle ouvre le guide : « Reprise à la page 59 — lue hier ». Elle lit ; le pied de page dit « Page 59 sur 92 · 3.6 Adaptation ». Elle veut ouvrir un autre guide, qu'elle n'a pas téléchargé : l'écran le lui dit sans détour, et lui propose de le télécharger au retour du réseau.

**Why this priority** : c'est l'autre moitié du critère de sortie — lu en salle sans réseau —, et elle vaut pour toute personne : une déléguée sans compte doit pouvoir lire le guide en salle comme une négociatrice admise.

**Independent Test** : sans compte, télécharger le guide, passer en mode avion, fermer puis rouvrir l'application, lire jusqu'à une page, fermer, rouvrir et constater la reprise ; tenter un document non téléchargé et lire l'état qui le dit.

**Acceptance Scenarios**

1. **Given** une personne, avec ou sans compte, et un document public qui est un fichier — ou réservé, si elle a l'accès négociateur —, **When** elle touche « Télécharger pour lire sans réseau », **Then** la fiche montre la progression — « Téléchargement — 35 % », « 3,3 Mo sur 9,4 Mo » — et « Annuler le téléchargement ».
2. **Given** un téléchargement terminé, **When** la fiche se met à jour, **Then** elle dit « Téléchargé. Lisible sans connexion. », l'action principale devient « Lire », et « Retirer du téléphone — 9,4 Mo » est offert.
3. **Given** un document téléchargé et le réseau coupé, **When** elle ouvre la bibliothèque, **Then** le bandeau dit « Hors connexion — lu à … », le sous-titre dit combien de documents sont lisibles maintenant, et chaque document non téléchargé porte « Non téléchargé — disponible au retour du réseau » — ou « Lien externe — réseau nécessaire ».
4. **Given** un document téléchargé, **When** elle l'ouvre sans réseau, **Then** le lecteur l'affiche en entier, sommaire, recherche et notes de correction compris.
5. **Given** un document déjà lu, **When** elle le rouvre, **Then** le lecteur reprend à la dernière page lue et le dit — « Reprise à la page 59 — lue hier » — avec « Début » pour repartir de la première page.
6. **Given** le lecteur ouvert, **When** elle lit, **Then** la barre est repliée : une ligne d'en-tête — retour, titre, « Aa » — et un pied qui porte la page, le nombre de pages, la section en cours et une jauge de progression.
7. **Given** la barre repliée, **When** elle touche le centre de la page, **Then** la barre se déplie et offre « Sommaire », « Rechercher » et « Réglages » ; un second toucher la replie.
8. **Given** un document non téléchargé et le réseau coupé, **When** elle cherche à l'ouvrir, **Then** l'écran dit « Ce document n'est pas sur votre téléphone », nomme le document, son nombre de pages et sa taille, propose « Télécharger au retour du réseau », et nomme un document lisible maintenant s'il en existe un.
9. **Given** « Télécharger au retour du réseau » demandé, **When** le réseau revient, **Then** le téléchargement part sans nouveau geste, une seule fois, même si l'application a été fermée entre-temps.
10. **Given** un document gardé sur le téléphone, **When** une nouvelle version du même document est publiée, **Then** la copie gardée reste lisible telle quelle et porte le bandeau « Remplacé par… » dès la prochaine lecture de la bibliothèque ; rien n'est remplacé ni effacé sans que la personne le demande.
11. **Given** une note de correction posée après le téléchargement, **When** la bibliothèque est relue avec le réseau, **Then** la note paraît dans la copie gardée sans qu'il faille retélécharger le document.
12. **Given** une personne sans compte qui a téléchargé des documents publics, **When** elle crée un compte ou se connecte, **Then** ses copies restent sur le téléphone, lisibles, avec leur progression ; le téléchargement n'a rien créé sur le serveur, et rien n'y est rattaché après coup.

---

### User Story 4 — Lire confortablement : sommaire, recherche, taille, thème, termes anglais (Priority: P2)

En séance de nuit, Aïssatou passe le lecteur en sombre et agrandit le texte. Elle cherche « progrès collectifs » : cinq passages dans le guide, celui où elle se trouve marqué « vous êtes ici ». Elle passe d'une occurrence à l'autre. Elle touche *global goal on adaptation*, en italique : une feuille s'ouvre en bas de l'écran.

**Why this priority** : ce sont les gestes qui font d'un fichier un outil de salle. Ils s'appuient sur le récit 3 sans le conditionner.

**Independent Test** : sur le guide téléchargé et sans réseau, ouvrir le sommaire et sauter à une section, chercher une expression et parcourir ses occurrences, changer la taille et le thème, toucher un terme anglais.

**Acceptance Scenarios**

1. **Given** la barre dépliée, **When** elle ouvre « Sommaire », **Then** il porte le titre et le nombre de pages, les chapitres repliés avec leur nombre de sous-parties et leur page, le chapitre en cours déplié et la section en cours marquée ; toucher une entrée ouvre sa page.
2. **Given** « Rechercher », **When** elle tape une expression, **Then** l'écran dit combien de passages et sur combien de pages — « 5 passages dans 92 pages — sans réseau » —, liste chaque passage avec sa page, sa section et un extrait où l'expression est marquée, et signale « vous êtes ici » sur celui de la page en cours.
3. **Given** un passage choisi, **When** le lecteur y va, **Then** il dit « Occurrence 3 sur 5 », marque l'occurrence courante d'un surlignage plein et les autres d'un surlignage clair, et offre « précédente », « suivante » et la fermeture.
4. **Given** « Réglages », **When** la feuille s'ouvre, **Then** elle offre la taille du texte — Normale, Grande, Très grande — et le thème — Clair, Sombre, Système —, et dit que le réglage s'applique à tous les documents et se retrouve dans le profil.
5. **Given** une taille plus grande, **When** elle s'applique, **Then** le texte se recompose sans défilement horizontal, la page en cours est conservée, et la numérotation des pages reste celle du document d'origine.
6. **Given** le thème changé dans le lecteur, **When** elle revient au reste de l'application, **Then** le même thème s'y applique : c'est le réglage livré en 0a, pas un second.
7. **Given** un terme anglais marqué dans le texte, **When** elle le touche, **Then** une feuille basse s'ouvre, titrée du terme ; **tant que le lexique n'existe pas**, elle dit que la traduction viendra avec le lexique et n'offre que « Revenir au texte » — jamais une définition inventée.
8. **Given** la recherche dans le document, **When** le réseau est coupé, **Then** elle fonctionne à l'identique sur un document téléchargé.

---

### User Story 5 — « Mes documents », la place, et ce qui s'efface (Priority: P2)

Avant de rentrer, Aïssatou ouvre « Mes documents ». Elle voit ce qui est sur son téléphone — 7 Mo —, ce qui reste libre, et ses deux favoris. Elle touche « Tout retirer du téléphone » ; la confirmation lui dit ce qui ne se lira plus sans réseau, et ce qui reste. Sur la tablette de sa délégation, elle s'était connectée pour lire le résumé réservé : en se déconnectant, il s'efface de la tablette.

**Why this priority** : c'est ce qui rend la place maîtrisable et les documents réservés sûrs sur un appareil partagé. Il reprend le geste reporté de l'étape 0c.

**Independent Test** : télécharger deux documents dont un réservé, mettre deux documents en favori, ouvrir « Mes documents », tout retirer et constater la baisse de la place ; retélécharger le réservé, se déconnecter et constater qu'il a disparu du téléphone, pas le public.

**Acceptance Scenarios**

1. **Given** une personne connectée, **When** elle touche « Favori » sur une fiche, **Then** le document entre dans ses favoris ; le bouton se remplit, et un second toucher l'en retire.
2. **Given** un favori posé sur un premier appareil, **When** elle se connecte sur un second, **Then** elle retrouve le même favori.
3. **Given** un favori posé sans réseau, **When** le réseau revient, **Then** il est enregistré une seule fois.
4. **Given** « Mes documents », ouvert avec ou sans compte, **When** l'écran s'ouvre, **Then** il dit le nombre de documents téléchargés et de favoris ; la section « Sur le téléphone » porte la place occupée et, pour chaque document, la date de son téléchargement ; la jauge dit la place utilisée par Guide Négo et la place libre sur le téléphone ; la section « Favoris » rappelle « Un favori se lit sans réseau seulement s'il est aussi téléchargé » — sans compte, cette section invite à créer un compte ou à se connecter.
5. **Given** des documents téléchargés, **When** elle touche « Tout retirer du téléphone », **Then** une confirmation dit combien de documents, lesquels et quelle place, qu'ils ne se liront plus sans réseau, et qu'ils restent dans la bibliothèque et dans ses favoris ; elle choisit « Revenir » ou « Retirer ».
6. **Given** « Retirer » confirmé, **When** l'effacement se termine, **Then** la place affichée diminue aussitôt, sans rien effacer des lectures qui font marcher l'application sans réseau — accès, thématiques, bibliothèque connue.
7. **Given** « Mes téléchargements » dans le profil, livré en 0c, **When** on l'ouvre, **Then** il mène à « Mes documents », avec le nombre de documents gardés et leur place.
8. **Given** un document réservé téléchargé, **When** la personne se déconnecte, **Then** il s'efface du téléphone avant que la déconnexion se termine, et le message de déconnexion le dit ; les documents publics téléchargés restent.
9. **Given** un document réservé téléchargé, **When** l'accès négociateur de la personne est retiré, **Then** le document s'efface du téléphone à la prochaine lecture de son accès.
10. **Given** un document public téléchargé, **When** il devient réservé, **Then** il s'efface des téléphones des personnes sans accès à la prochaine lecture de la bibliothèque.
11. **Given** « Ma journée », **When** la personne a lu ou téléchargé des documents, **Then** le bloc « Documents récents » livré vide en 0c porte ses derniers documents ouverts, avec leur dernière page lue.

---

### User Story 6 — La note de correction d'un expert (Priority: P3)

Le Dr Koffi Mensah, expert de l'IFDD, relit le guide pendant la COP. À la page 59, un passage parle des indicateurs de l'objectif mondial d'adaptation comme d'un sujet ouvert ; ils ont été adoptés depuis. Il pose une note de correction sur ce passage. Dans le lecteur, un filet rouge borde le paragraphe et une ligne dit « Note de correction — passage dépassé » ; la dépliée donne le texte, son nom et la date.

**Why this priority** : c'est ce qui garde le guide juste pendant la COP sans republier 92 pages. Le lecteur et la publication vivent sans lui.

**Independent Test** : poser une note sur une page du guide depuis le back-office, la lire dans l'application en ligne puis sur la copie téléchargée sans réseau, la retirer et constater qu'elle disparaît.

**Acceptance Scenarios**

1. **Given** une personne disposant de la permission de corriger un document, **When** elle choisit une page du document et, si elle le veut, le passage visé, **Then** elle écrit le texte de la note et l'enregistre ; la note porte son nom, sa qualité et la date.
2. **Given** une note posée, **When** on lit la page dans l'application, **Then** le passage visé — ou le haut de la page si aucun passage n'est visé — porte un filet rouge et la ligne repliée « Note de correction — passage dépassé ».
3. **Given** la ligne repliée, **When** on la touche, **Then** la note se déplie : le texte, puis la signature — nom, « expert IFDD », date.
4. **Given** une note retirée depuis le back-office, **When** l'application relit le document, **Then** la note disparaît, y compris des copies téléchargées, et le back-office garde trace de qui l'a posée, qui l'a retirée et quand.
5. **Given** plusieurs notes sur un même document, **When** on ouvre le sommaire, **Then** rien n'y change à cette étape ; chaque note se lit sur sa page.
6. **Given** une note de correction, **When** on la lit, **Then** elle ne modifie jamais le texte du document : elle se pose par-dessus.

---

### Edge Cases

- **Le fichier téléversé n'a pas de sommaire repérable.** Le document se publie quand même ; le lecteur n'offre pas « Sommaire », et l'aperçu du back-office l'a dit avant la publication.
- **Le texte du fichier ne se recompose pas** — un scan sans texte, un fichier protégé. L'aperçu le dit ; le document se publie comme fichier à ouvrir tel quel, la fiche le dit, et la recherche dans le texte ne le couvre pas.
- **Le texte se tire, mais se recompose mal** — colonnes lues dans le désordre, en-têtes et pieds de page mêlés au texte, notes et tableaux éclatés, mots coupés par des césures, italiques perdues. L'aperçu page par page le montre ; l'administratrice choisit « ouvrir tel quel » et publie le jour même. Découvert le jour de la publication, le problème ne doit coûter aucun délai.
- **Le téléphone n'a plus de place.** Le téléchargement échoue proprement, la fiche dit que la place manque et combien il en faut, et renvoie vers « Mes documents ».
- **Le réseau tombe au milieu d'un téléchargement.** La fiche le dit ; le téléchargement reprend au retour du réseau ou se relance d'un geste, sans laisser une copie à moitié lisible.
- **Le stockage du navigateur ne mesure pas la place.** « Mes documents » dit la place des documents qu'il connaît et tait la place libre plutôt que d'afficher zéro — comme 0c l'a posé.
- **Un document réservé est ouvert quand la session expire.** Le lecteur se ferme sur le verrou à la prochaine lecture de l'accès ; la copie réservée ne s'efface qu'à une déconnexion ou un retrait d'accès, pas sur une simple panne de réseau — une API muette n'est pas une déconnexion (0c, reprise 1).
- **Deux documents se désignent comme remplaçants l'un de l'autre, ou forment une boucle.** Le back-office le refuse.
- **Un document remplacé l'a été à son tour.** Le bandeau mène au document à jour de la chaîne, pas à l'intermédiaire.
- **Le document remplaçant est réservé, et la personne n'a pas l'accès.** Le bandeau dit qu'une version à jour existe, réservée, et mène à sa fiche verrouillée.
- **Un lien externe ne répond plus.** L'application ne le vérifie pas ; le navigateur le dit. Le back-office le signale à la publication s'il ne répond pas.
- **La page de reprise n'existe plus** — le document a été remplacé par une version plus courte. La reprise ne vaut que pour la même version ; une autre version s'ouvre au début.
- **Une personne sans compte met un document en favori.** Le geste l'invite à créer un compte ou à se connecter, et ne pose rien localement.
- **Deux appareils téléchargent le même document.** Chacun garde sa copie ; « Tout retirer » n'efface que celles du téléphone où il est touché.
- **Une recherche dans le document ne trouve rien.** L'écran le dit, avec l'expression cherchée, sans quitter la page en cours.
- **Une thématique ou un type de document est désactivé.** Il cesse d'être proposé dans les filtres ; les documents qui le portent restent trouvables par les autres filtres.

## Requirements *(mandatory)*

### Publier (back-office)

- **FR-001** : Un document DOIT être soit un fichier téléversé, soit un lien externe, jamais les deux ni aucun ; la règle DOIT être tenue par la base et le refus dit en clair.
- **FR-002** : Le formulaire DOIT porter : titre et résumé (français, anglais facultatif), type, thématiques (zéro ou plusieurs), COP (zéro ou une), version, date du document, éditeur, langue, source, document remplacé, accès public ou réservé, marqueur « utilisable par l'assistant ».
- **FR-003** : Le type et les thématiques DOIVENT être pris dans les vocabulaires de la base ; les thématiques sont celles de négociation, semées à l'étape 0c.
- **FR-004** : Un document DOIT pouvoir être enregistré sans être publié ; seul un document publié paraît dans l'application.
- **FR-005** : À la fin du traitement d'un fichier, le back-office DOIT montrer un aperçu — nombre de pages, taille, sommaire repéré, et **chaque page** rendue à côté de son texte recomposé, à feuilleter page à page — avant la publication. Un document de cent pages DOIT se parcourir en entier en quelques minutes.
- **FR-005 bis** : L'administratrice DOIT pouvoir choisir « ouvrir tel quel » pour le document entier, que le texte se recompose ou non ; le document se lit alors en pages d'origine, et le choix se change sans republier le fichier.
- **FR-006** : Un document NE DOIT avoir qu'un seul remplaçant, et les remplacements NE DOIVENT former aucune boucle.
- **FR-007** : Le fichier d'un document publié NE DOIT pas être changé en place ; un nouveau fichier se publie comme une nouvelle version qui remplace la précédente.
- **FR-008** : Le marqueur « utilisable par l'assistant » DOIT être décoché par défaut et dire qu'il ne prend effet qu'avec l'assistant ; aucune indexation n'est faite à cette étape.
- **FR-009** : Une administratrice DOIT pouvoir dépublier un document ; il quitte la bibliothèque et les copies gardées s'effacent à la prochaine lecture de la bibliothèque.
- **FR-010** : Les écrans du back-office DOIVENT être filtrés par le périmètre d'administration, y compris quand l'adresse est forgée, et l'autorisation DOIT se tester par permission avec sa portée.
- **FR-011** : Toute écriture du back-office DOIT être tracée — qui, quand, quoi.
- **FR-012** : Une personne entraînée DOIT pouvoir publier un document complet en moins de quinze minutes, fichier téléversé compris.

### Trouver (bibliothèque et fiche)

- **FR-013** : La bibliothèque et la fiche des documents publics DOIVENT être accessibles sans compte.
- **FR-014** : La recherche DOIT porter sur le titre, le résumé, l'éditeur et le texte des documents dont le texte se recompose.
- **FR-015** : La bibliothèque DOIT offrir trois filtres à choix multiples — Type, Thématique, COP — combinables entre eux et avec la recherche, chacun annonçant le nombre de documents par valeur et le nombre de résultats avant application.
- **FR-016** : Les libellés des types, des thématiques et des COP DOIVENT venir de la base, dans la langue de la personne avec repli sur le français, et ne figurer dans aucun fichier de traduction ni dans le code.
- **FR-017** : Chaque ligne de la liste DOIT porter type, titre, pages, taille, éditeur et année — ou « Page web » — et les marques qui s'appliquent : Téléchargé, À jour, Nouveau, Réservé, Lien externe, Remplacé par…
- **FR-018** : La marque « Nouveau », jaune — le signe du non-lu —, DOIT désigner un document publié depuis moins de sept jours **et** jamais ouvert sur ce téléphone ; elle disparaît à la première ouverture. Elle se calcule sur l'appareil et ne demande pas de compte.
- **FR-019** : La fiche DOIT porter le résumé, les thématiques ou la mention qu'il n'y en a aucune, la version, la date, l'éditeur, la langue et la COP le cas échéant.
- **FR-020** : La fiche d'un document remplacé DOIT porter le bandeau « Remplacé par » menant au document à jour de la chaîne, avec sa date, ses pages et s'il est déjà téléchargé ; la version remplacée DOIT rester lisible et téléchargeable.
- **FR-021** : La fiche d'un lien externe DOIT dire qu'il s'ouvre dans le navigateur et que rien n'est gardé sur le téléphone ; un lien NE DOIT pas se télécharger.
- **FR-022** : Un document réservé DOIT rester visible dans la liste pour tous — titre, type, marques, pages, taille —, mais son résumé, ses thématiques, son contenu et son téléchargement NE DOIVENT être servis qu'à une personne disposant de l'accès négociateur ; le refus DOIT être tenu par l'API, pas seulement par l'écran.
- **FR-023** : Le verrou d'un document réservé DOIT être celui livré en 0b, avec ses sorties selon qu'il y a un compte ou non.
- **FR-024** : « Partager » DOIT ouvrir le partage du téléphone avec le titre et l'adresse de la fiche, sans jamais transmettre le contenu d'un document réservé.

### Garder (téléchargement, favoris, « Mes documents »)

- **FR-025** : Le téléchargement d'un document public pour le lire sans réseau NE DOIT PAS demander de compte : il ne crée rien sur le serveur (arbitré le 22/09). Un document réservé DOIT exiger l'accès négociateur. Le favori, qui suit le compte, DOIT demander un compte.
- **FR-025 bis** : L'écran « 02 Ouverture » livré en 0a range les téléchargements sous « Avec un compte » ; il DOIT les en sortir — écart 41.
- **FR-026** : Le téléchargement DOIT montrer sa progression, pouvoir s'annuler, et ne laisser aucune copie partielle lisible.
- **FR-027** : Une personne DOIT pouvoir demander un téléchargement sans réseau ; il part au retour du réseau, une seule fois, même après fermeture de l'application.
- **FR-028** : Les documents téléchargés DOIVENT être rangés à part des caches de la coquille, survivre aux déploiements, et leur effacement DOIT se voir aussitôt dans la place mesurée.
- **FR-029** : Les favoris DOIVENT suivre le compte d'un appareil à l'autre ; un favori posé sans réseau part au retour du réseau, une seule fois.
- **FR-030** : « Mes documents », accessible avec ou sans compte, DOIT montrer les documents téléchargés sur ce téléphone avec leur date et leur place, la place utilisée et libre, et les favoris du compte.
- **FR-031** : « Tout retirer du téléphone » DOIT passer par une confirmation qui dit quels documents, quelle place, ce qui ne se lira plus sans réseau et ce qui reste ; un document DOIT aussi pouvoir se retirer seul depuis sa fiche.
- **FR-032** : Retirer NE DOIT effacer que des documents téléchargés, jamais les lectures qui font marcher l'application sans réseau.
- **FR-033** : Les documents réservés DOIVENT s'effacer du téléphone à la déconnexion, avant qu'elle se termine, et à la perte de l'accès négociateur ; une panne de réseau NE DOIT pas les effacer. Le message de déconnexion livré en 0c DOIT distinguer les documents publics, qui restent, des réservés, qui s'effacent.
- **FR-034** : Une copie gardée d'un document dépublié, ou devenu réservé pour une personne sans accès, DOIT s'effacer à la prochaine lecture de la bibliothèque.
- **FR-035** : Chaque téléchargement DOIT être compté, pour la mesure d'usage des documents, sans rien retenir de la personne.
- **FR-036** : Le bloc « Documents récents » de « Ma journée » DOIT porter les derniers documents ouverts sur l'appareil, avec leur dernière page lue.

### Lire (le lecteur)

- **FR-037** : Le lecteur DOIT afficher le texte du document recomposé à la largeur de l'écran, avec la numérotation des pages du document d'origine. Ce texte, ses pages et son sommaire DOIVENT être tirés automatiquement du fichier PDF publié — l'administratrice ne fournit rien d'autre (arbitré le 22/09). Un PDF dont le texte ne se tire pas, ou pour lequel l'administratrice a choisi « ouvrir tel quel », s'ouvre en pages d'origine, sans taille de texte, termes touchables ni recherche dans le texte, et sa fiche le dit.
- **FR-038** : La barre DOIT être repliée à la lecture — en-tête d'une ligne, pied portant page, nombre de pages, section en cours et jauge — et se déplier d'un toucher au centre sur « Sommaire », « Rechercher », « Réglages ».
- **FR-039** : Le lecteur DOIT retenir la dernière page lue de chaque document sur l'appareil et reprendre à cette page en le disant, avec un retour au début ; la reprise ne vaut que pour la même version.
- **FR-040** : Le sommaire DOIT porter les chapitres repliables, leur nombre de sous-parties et leur page, la section en cours, et mener à la page choisie.
- **FR-041** : La recherche dans le document DOIT fonctionner sans réseau sur une copie téléchargée, dire le nombre de passages et de pages, lister chaque passage avec page, section et extrait, signaler le passage de la page en cours, et permettre d'aller d'occurrence en occurrence en distinguant la courante des autres.
- **FR-042** : Le lecteur DOIT offrir trois tailles de texte — Normale, Grande, Très grande — valant pour tous les documents et gardées sur l'appareil, et le réglage de thème de 0a.
- **FR-043** : Un terme anglais marqué dans le texte DOIT ouvrir une feuille basse titrée du terme ; tant que le lexique n'existe pas, elle NE DOIT porter aucune traduction ni définition, et dit qu'elles viennent avec le lexique.
- **FR-044** : L'ouverture d'un document non téléchargé sans réseau DOIT dire qu'il n'est pas sur le téléphone, avec ses pages et sa taille, proposer de le télécharger au retour du réseau, et nommer un document lisible maintenant s'il en existe un.
- **FR-045** : Une personne sans compte DOIT pouvoir lire un document public avec le réseau, et le télécharger pour le lire sans réseau.

### Corriger (notes d'expert)

- **FR-046** : Une personne disposant de la permission de corriger DOIT pouvoir poser une note sur une page d'un document, et, si elle le veut, sur un passage de cette page ; la note porte son texte, son auteur, sa qualité et sa date.
- **FR-047** : Le lecteur DOIT signaler la note par un filet et une ligne repliée « Note de correction — passage dépassé », et la déplier sur son texte et sa signature.
- **FR-048** : Une note NE DOIT jamais modifier le texte du document ; elle se pose par-dessus.
- **FR-049** : Une note DOIT pouvoir être retirée ; le retrait DOIT atteindre les copies téléchargées à la prochaine lecture, et l'historique — pose et retrait, auteurs, dates — DOIT être gardé.
- **FR-050** : Les notes d'un document téléchargé DOIVENT se mettre à jour sans retélécharger le document.

### Ce qui vaut pour tous les écrans de l'étape

- **FR-051** : La bibliothèque, les fiches et « Mes documents » DOIVENT s'afficher sans réseau à partir de ce qui a été lu, avec l'heure de cette lecture ; la bibliothèque DOIT se relire par ce qui a changé depuis la dernière lecture.
- **FR-052** : Chaque écran DOIT porter ses quatre états : chargement, vide, erreur, accès refusé ; l'état d'erreur n'est employé que pour une vraie panne.
- **FR-053** : Chaque écran DOIT tenir à 360 px sans défilement horizontal, dans les deux thèmes, cibles tactiles comprises.
- **FR-054** : Les écrans de l'application DOIVENT être bâtis sur les composants de Guide Négo, sans composant ni jeton du site ; ceux du back-office, sur les composants du back-office de l'ePavillon. Tout composant nouveau de Guide Négo DOIT être ajouté à la page interne des composants.
- **FR-055** : Les textes d'interface DOIVENT vivre dans des fichiers de traduction découpés par écran, en français et en anglais ; les libellés des vocabulaires n'y figurent jamais.
- **FR-056** : Un document publié une fois DOIT pouvoir servir le site comme l'application ; rien de propre à l'application n'est dupliqué dans le document.

### Key Entities

- **Document** : un titre et un résumé multilingues, un type, zéro ou plusieurs thématiques de négociation, zéro ou une COP, une version, une date, un éditeur, une langue, un accès public ou réservé, un marqueur pour l'assistant, un état publié ou non. Sa source est un fichier ou un lien, jamais les deux.
- **Fichier d'un document** : le PDF téléversé, sa taille, son nombre de pages, son sommaire repéré, sa forme lisible — le texte recomposé, page par page, avec ses termes anglais marqués — et son mode de lecture : recomposé ou « tel quel ». Figé une fois publié, sauf le mode de lecture.
- **Remplacement** : le lien d'une version à la suivante. Un seul remplaçant, pas de boucle ; le document à jour est le bout de la chaîne.
- **Favori** : le lien entre une personne et un document, porté par le compte.
- **Copie gardée** : un document téléchargé sur un appareil, sa date et sa place. Ce qu'elle contient — le texte recomposé, le PDF, ou les deux — se décide au plan sur l'essai du vrai guide. Propre à l'appareil, sans compte possible ; s'efface à la demande, à la déconnexion s'il est réservé, ou quand il cesse d'être servi.
- **Progression de lecture** : la dernière page lue d'une version d'un document, et quand. Propre à l'appareil.
- **Réglage de lecture** : la taille du texte. Propre à l'appareil, pour tous les documents.
- **Note de correction** : un texte posé par un expert sur une page, et le cas échéant sur un passage, d'un document ; son auteur, sa date, son retrait et l'auteur du retrait.

### Ce que le modèle ne porte pas encore

Le SQL se modifie d'abord, puis la base se recharge — migration rejouable, sans détruire la base locale —, puis le code s'écrit.

| Manque | Ce qu'il faut | Où |
|---|---|---|
| Les thématiques d'un document | Le lien document ↔ thématique de négociation, zéro ou plusieurs, **par `reference.entity_terms`** (arbitré le 22/09) — la doctrine du modèle, que suivent événements, propositions, publications et formations ; la table propre de 0c se justifiait par une préférence avec son historique. Le vocabulaire se vérifie à l'écriture, comme pour les thématiques des propositions ; un brouillon supprimé efface ses liens dans la même transaction, le ménage promis par `platform.purge_term_links()` n'existant pas ; la lecture écarte les termes désactivés | Aucune table nouvelle — `020_reference.sql` porte déjà `entity_terms` |
| La COP d'un document | Un lien facultatif vers l'édition, sur le modèle de `negotiation.meetings.event_id` | `100_negotiations.sql` |
| « Remplacé par » | Le lien existant va du neuf vers l'ancien (`supersedes_id`) ; il faut garantir un seul remplaçant et aucune boucle, et lire la chaîne dans les deux sens | `100_negotiations.sql` |
| La date du document | Seule la date de mise en ligne existe ; il faut la date du document lui-même, distincte | `100_negotiations.sql` |
| L'éditeur d'un fichier | `external_publisher` est nommé pour le lien ; il doit valoir pour toute source — à renommer ou à redéfinir au plan | `100_negotiations.sql` |
| Les types de la maquette | Le vocabulaire des types porte guide, note technique, document utile, présentation, rapport, autre ; la maquette emploie aussi « Résumé » et « Bulletin » | `020_reference.sql` |
| Le nombre de pages, le sommaire, la forme lisible, le mode de lecture | Rien. À porter avec le fichier du document, avec le choix « ouvrir tel quel » | `100_negotiations.sql` ou `050_media.sql` — au plan, après l'essai |
| Les notes de correction | Rien. Page, passage facultatif, texte, auteur, date, retrait et son auteur. [ADR-012](../../docs/AppNego/adr/012-toute-source-porte-un-etat.md) les annonce pour toute source ; elles naissent ici sur les documents, et l'étape 8 les étendra à la vidéo | `100_negotiations.sql` |
| Le marqueur pour l'assistant | Existe, mais **vrai par défaut** — contraire à [ADR-011](../../docs/AppNego/adr/011-un-corpus-a-deux-etages.md). Le défaut passe à faux | `100_negotiations.sql` |
| L'expert | **Aucun rôle n'existe.** Un rôle `expert`, de portée globale, attribué depuis l'écran des utilisateurs du back-office (A12), avec **une permission par geste** — ici, poser et retirer une note. Nommé pour les étapes qui y ajouteront les leurs : 2 (questions à l'expert), 7 (état des sources), 8 (notes sur la vidéo) | `030_identity.sql` |

Ne s'ajoutent **pas** au modèle : la progression de lecture, la taille du texte et les copies gardées, propres à l'appareil ; le compteur de téléchargements et sa fonction existent déjà.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : Le *Guide des négociations* est publié de bout en bout — fichier téléversé, fiche complète, remplacement de la version précédente — en une journée de travail, et en moins de quinze minutes par une administratrice qui connaît l'écran.
- **SC-001 bis** : L'aperçu d'un document de cent pages se parcourt en entier, rendu et texte recomposé côte à côte, en moins de dix minutes.
- **SC-002** : Téléchargé la veille, **avec ou sans compte**, le guide s'ouvre et se lit en entier en mode avion, application fermée puis rouverte, sommaire, recherche et notes de correction compris.
- **SC-003** : Rouvert, un document reprend à la dernière page lue dans 100 % des cas pour une même version.
- **SC-004** : Une personne sans compte trouve un document par une recherche ou deux filtres en moins de dix secondes.
- **SC-005** : Après « Tout retirer du téléphone », la place affichée baisse d'au moins la taille des documents retirés, sans délai, et l'application s'ouvre toujours sans réseau.
- **SC-006** : Après une déconnexion, aucun document réservé ne reste lisible sur le téléphone ; les documents publics téléchargés, si.
- **SC-007** : Aucune API ne sert le résumé, le contenu ou le fichier d'un document réservé à une personne sans accès — les tests d'intégration le prouvent, adresse forgée comprise.
- **SC-008** : Une note de correction posée depuis le back-office paraît dans le lecteur, sur la copie téléchargée comprise, à la première lecture de la bibliothèque qui suit.
- **SC-009** : La recherche dans un document de cent pages, sans réseau, rend ses passages en moins d'une seconde.
- **SC-010** : Les écrans sont fidèles à la maquette à 360 px, en thème clair et sombre, dans les trois tailles de texte, sans défilement horizontal — aux écarts inscrits près.
- **SC-011** : Aucun libellé de type, de thématique ni de COP n'apparaît dans un fichier de traduction ni dans le code — la recherche le prouve.
- **SC-012** : Aucune feuille de terme anglais ne porte une traduction ou une définition qui ne vienne pas du lexique.

## Assumptions

- **Le texte tiré du PDF est le risque de l'étape.** Savoir qu'il se recompose ne suffit pas : un PDF mis en page — colonnes, en-têtes et pieds, notes, tableaux, césures — peut donner un texte qui se recompose mal, et une seule page d'aperçu ne le montre pas. D'où l'aperçu de chaque page et le repli « ouvrir tel quel » offert à tout document. Le plan commence par un essai sur le vrai guide de la CdP30 : il vérifie l'ordre de lecture, les en-têtes et pieds, les notes et tableaux, les césures et les italiques, choisit l'outil d'extraction sur ce résultat — licence comprise —, et dit ce que pèse la copie gardée : quelques centaines de ko de texte recomposé contre 9 Mo de PDF. Ce qu'on garde sur le téléphone se décide là, avec ce que le texte perd — tableaux, figures.
- **Le lecteur recompose le texte.** La maquette fait varier la taille du texte de 17 à 24 px, marque les termes anglais et cherche dans le texte sans réseau : ce n'est pas l'affichage d'une page figée. La numérotation reste celle du document d'origine — « Page 59 sur 92 » —, pour qu'une page citée en salle soit la même pour tous. **Il est tiré du PDF publié, et de lui seul** (arbitré par le commanditaire le 22/09) : c'est le seul choix qui tient « publié en une journée ». Une version Word fournie à part aurait demandé un fichier de plus à chaque publication ; afficher les pages comme des images aurait perdu la taille du texte et les termes touchables. Les fichiers acceptés sont donc des PDF.
- **Les termes anglais touchables** sont ceux que le texte marque comme tels — l'italique du document d'origine, où le guide écrit ses termes anglais. À l'étape 2, le lexique pourra reconnaître ses propres entrées.
- **Une COP au plus par document.** « Bilan de la CdP30 » concerne la COP30 ; un document transversal n'en a pas. Plusieurs COP par document n'est dessiné nulle part.
- **La progression et la taille du texte restent sur l'appareil**, comme la copie qu'elles accompagnent ; les favoris suivent le compte, comme le modèle et [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) les portent — un favori posé sans réseau part au retour du réseau.
- **Écart avec le texte de « À propos » (0c)** : son paragraphe de confidentialité dit que les favoris restent sur le téléphone. Ils suivent le compte ; les téléchargements, eux, restent sur le téléphone, avec ou sans compte. Le paragraphe est corrigé à cette étape.
- **Lire et télécharger sans compte** (arbitré le 22/09) : un document public se lit et se télécharge par toute personne ; le favori seul demande un compte. Le prompt de l'étape dans [04-roadmap.md](../../docs/AppNego/04-roadmap.md) est corrigé, et l'écran « 02 Ouverture », qui rangeait les téléchargements sous « Avec un compte », fait l'objet de l'écart 41.
- **« Nouveau » = publié depuis moins de sept jours et jamais ouvert sur ce téléphone** (arbitré le 22/09). La marque est jaune — le même signe que le non-lu, le jaune disant ce qui concerne la personne à l'instant : un document déjà lu ne l'est plus.
- **« Marquer » n'est pas livré** — écart 42, accepté le 22/09. La barre dépliée de la maquette le montre, mais rien ne dit ce qu'il produit ni où on retrouve ses marques ; un bouton sans effet trompe (constitution, XII), comme les interrupteurs de 0c. Il paraîtra quand son résultat sera dessiné.
- **« Dépassé » ne s'emploie ici que dans la note de correction** d'un passage. L'état d'une source entière vient avec l'étape 7.
- **Le triangle rouge de la note reste** (arbitré le 22/09) : un passage dépassé ou faux est une erreur du texte ; l'écart 20 l'opposait à la croix d'« annulé », pas à la note. Il devient « triangle rouge = erreur, lecture impossible, ou passage dépassé ».
- **Les contradictions de la maquette se tranchent ainsi** : « Remplacé » au masculin partout (lexique) ; le Bulletin, lien externe, porte « Lien externe — réseau nécessaire » et jamais « Non téléchargé » ; « Tout se lit sans connexion » de l'écran Ressources de 02-socle devient faux et se corrige — seuls les documents téléchargés se lisent sans réseau ; le chiffre de la note d'essai (100 ou 59 indicateurs) est une donnée inventée, qui ne se publie pas.
- **L'heure d'ajout d'un lien** est une heure d'événement, donc avec son fuseau, comme le dessine la fiche « 05e » ; les heures de lecture restent sans fuseau (écart 32).
- **Le lexique n'existe pas encore** : la feuille basse du terme touché dit ce qui viendra, sans « Ouvrir dans le lexique » tant qu'il n'y a pas d'entrée à ouvrir.
- **Les quatre autres documents de la maquette** servent de jeu d'essai ; seul le guide, dont l'IFDD détient les droits, se publie réellement pour la recette.
- **La branche Git** n'est pas créée par cette commande ; elle suivra la convention `011-guide-nego-documents` au plan.
