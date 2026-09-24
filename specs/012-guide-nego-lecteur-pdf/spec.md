# Feature Specification: Guide Négo — le lecteur montre le PDF d'origine (étape 1b)

**Feature Branch**: `012-guide-nego-lecteur-pdf`, partie de `011-guide-nego-documents`

**Created**: 2026-09-24

**Status**: Draft — relue par le commanditaire le 24/09 : trois hypothèses tranchées (voir *Les arbitrages du 24/09*)

**Input**: Étape 1b de [docs/AppNego/04-roadmap.md](../../docs/AppNego/04-roadmap.md), décidée par le commanditaire le 24/09 après avoir lu le texte recomposé de l'étape 1 : on lit le document tel que l'IFDD l'a mis en page — pages, colonnes, tableaux, figures —, et non un texte recomposé. Le document s'ouvre sur ses pages d'origine, nettes à tout grossissement, qu'on fait défiler, agrandit d'un pincement ou d'un double toucher, et dont le texte se sélectionne ; en ligne, la première page paraît sans attendre le fichier entier. Un second mode, « Texte agrandi », garde le lecteur de l'étape 1 pour qui lit mal une page A4 réduite à la largeur d'un téléphone. Le texte extrait sert à la recherche dans le document — le passage repéré sur la page —, à la recherche de la bibliothèque et au sommaire. La copie gardée devient le PDF et le texte extrait. Les notes de correction se signalent en marge de la page. Le mode « tel quel » disparaît. Critère : le guide téléchargé se lit en mode avion sur un Android de milieu de gamme et sur un iPhone, pages nettes à tout grossissement, sans saccade sur ses 90 pages ; une recherche mène au passage sur sa page.

---

## Ce qui fait foi

| Sujet | Référence |
|---|---|
| La spec que ce cycle modifie | [specs/011-guide-nego-documents/spec.md](../011-guide-nego-documents/spec.md). **Ce cycle remplace ses récits 3, 4, 5 et 6 pour ce qui touche la lecture** ; la publication (récit 1), la bibliothèque et la fiche (récit 2), les favoris, « Mes documents » et l'effacement (récit 5, hors contenu de la copie) ne changent pas. Le détail est dans *Ce que ce cycle change dans la spec 011* |
| Les écrans | [04-lecteur.html](../../docs/AppNego/design/ecrans/04-lecteur.html) pour la barre de lecture repliée et dépliée, le sommaire, la recherche, les réglages (« 07 Réglages »), la note et la reprise. **Sa page de texte recomposé devient le mode « Texte agrandi » ; la page du PDF est l'affichage par défaut**, qu'aucune maquette ne dessine ; le choix du mode prend dans « 02 Barre dépliée » l'emplacement de « Marquer » — écarts 43 et 44 de [05-design.md](../../docs/AppNego/05-design.md) |
| Les arbitrages du 24/09 | (1) **« Texte agrandi » remplace « Ouvrir tel quel »** : par défaut il suit le verdict de l'extraction donné à l'aperçu — proposé si le texte s'est bien recomposé, retiré sinon —, et l'administratrice change ce choix sans republier ; la recherche et le sommaire restent dans tous les cas. (2) **Le choix du mode se voit** : « Pages · Texte » occupe dans la barre dépliée la place laissée libre par « Marquer », et reste aussi dans « Réglages » ; « Aa » n'ouvre que le lexique (ADR-018), donc le bouton des réglages s'appelle « Réglages » et son pictogramme ne ressemble pas à « Aa » ; au premier document ouvert sur un écran étroit, une ligne dit une seule fois que « Texte agrandi » existe. (3) **La copie gardée porte un numéro de format** : une copie d'un format ancien s'efface et le document redevient « Non téléchargé » — pour les copies de l'étape 1, et pour tout changement de copie à venir |
| Le système de design | [01-systeme.html](../../docs/AppNego/design/ecrans/01-systeme.html) — « 4 quater » Lecteur — et [design/passation/](../../docs/AppNego/design/passation/). Un composant qui manque se crée ici et s'ajoute à la page interne des composants |
| Les mots employés à l'écran | [design/lexique.md](../../docs/AppNego/design/lexique.md) — dont « Hors connexion — lu à… », sans fuseau (écart 32) |
| Les décisions | [ADR-003](../../docs/AppNego/adr/003-tout-ce-qui-se-lit-se-lit-hors-connexion.md) tout ce qui se lit se lit hors connexion · [ADR-019](../../docs/AppNego/adr/019-service-worker-engendre-cache-d-abord.md) les documents vivent hors des caches de la coquille · [ADR-021](../../docs/AppNego/adr/021-pdfium-dans-le-worker.md), dont la règle « le PDF ne va jamais sur le téléphone » **tombe** : un ADR nouveau la remplace au plan · [ADR-012](../../docs/AppNego/adr/012-toute-source-porte-un-etat.md) la note de correction sur une page |
| L'essai de l'étape 1 | [essai-extraction.md](../011-guide-nego-documents/essai-extraction.md) : le vrai guide de la CdP30 fait **90 pages et 2,9 Mo** ; son texte extrait pèse 297 Ko ; les images de ses 19 pages à tableau ou figure pesaient 3,5 Mo à elles seules |
| Les principes | Constitution : XI « Hors connexion d'abord », XII « Confiance », XIII « Un design propre et borné » |
| Ce qui est déjà livré | Étape 1 entière : la publication, l'extraction du texte, des pages et du sommaire, l'aperçu page par page du back-office, la bibliothèque, la fiche, le téléchargement, « Mes documents », la place, l'effacement des réservés, les notes de correction, la barre du lecteur, le sommaire, la recherche, les réglages, la feuille du terme anglais. **Rien de cela ne se réécrit sans nécessité** : ce cycle change ce que le lecteur affiche et ce que la copie contient |

**Hors périmètre** : tout ce que l'étape 1 a exclu et qui le reste — l'assistant et l'indexation, le lexique, la recherche globale, « Marquer » (écart 42), les notes sur la vidéo ; l'annotation personnelle d'un PDF (surligner, écrire dans la marge) ; l'impression ; l'ouverture du PDF dans une autre application du téléphone — le visionneur du téléphone est écarté (voir *Assumptions*) ; la lecture des PDF sur le site de l'ePavillon.

---

## Ce que ce cycle change dans la spec 011

| Dans 011 | Devient |
|---|---|
| Récit 1, scénarios 3 et 4 ; FR-005, FR-005 bis — l'aperçu montre le texte recomposé comme ce qu'on lira ; « ouvrir tel quel » | L'aperçu montre, page par page, la page d'origine et le texte qui sert à la recherche, au sommaire et au mode « Texte agrandi ». **« Ouvrir tel quel » disparaît** ; il est remplacé par le choix, plus étroit, de proposer ou retirer « Texte agrandi », posé par défaut d'après le verdict de l'extraction (FR-037) |
| Récit 3 ; FR-037 — le lecteur affiche le texte recomposé | Le lecteur affiche les pages du PDF ; le texte recomposé devient le mode « Texte agrandi » |
| Récit 4 ; FR-041, FR-042, FR-043 — recherche, taille du texte, termes anglais | La recherche mène au passage sur la page du PDF ; la taille du texte et les termes anglais n'existent qu'en « Texte agrandi » |
| FR-028 et l'entité *Copie gardée* — texte recomposé et images des pages à tableau ou figure | Le PDF et le texte extrait, sans image de page |
| Récit 6 ; FR-047 — la note borde un paragraphe du texte recomposé | La note se signale en marge de la page du PDF, à la hauteur du passage quand il y est retrouvé ; en « Texte agrandi », comme à l'étape 1 |
| Edge cases « Le texte ne se recompose pas » et « se recompose mal » | Le document s'ouvre sur ses pages comme tout autre ; seuls « Texte agrandi », la recherche et le sommaire en dépendent |

---

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Lire le guide sur ses pages, en salle, sans réseau (Priority: P1)

Aïssatou a téléchargé le guide la veille. En salle, le réseau est saturé. Elle ouvre le document : « Reprise à la page 59 — lue hier ». La page 59 est celle qu'elle a imprimée chez elle — mêmes colonnes, même tableau, même figure. Elle tient toute la largeur du téléphone ; Aïssatou pince pour agrandir le tableau, qui reste net, le parcourt du doigt, revient à la largeur d'un double toucher, puis fait défiler jusqu'à la page 61. Elle cite un paragraphe à sa délégation : elle le sélectionne, le copie et le colle dans sa messagerie. Le pied de page dit « Page 61 sur 92 · 3.6 Adaptation ».

**Why this priority** : c'est la décision du commanditaire et le critère de sortie. Une page citée en salle doit être la même pour tous, avec ses tableaux et ses figures.

**Independent Test** : télécharger le guide, passer en mode avion, fermer puis rouvrir l'application, ouvrir le guide, parcourir ses 90 pages en faisant défiler, agrandir un tableau au plus fort grossissement et le lire, revenir à la largeur, sélectionner et copier un paragraphe, fermer, rouvrir et constater la reprise. Le mener sur un Android de milieu de gamme et sur un iPhone.

**Acceptance Scenarios**

1. **Given** un document qui est un fichier, téléchargé ou lu avec le réseau, **When** on l'ouvre, **Then** il s'affiche sur ses pages d'origine — mise en page, colonnes, tableaux, figures, couleurs —, la page tenant d'abord toute la largeur de l'écran.
2. **Given** le lecteur ouvert, **When** on fait défiler, **Then** les pages se suivent de haut en bas, sans rupture ni retour en arrière, du début à la fin du document, et le pied de page suit la page en cours.
3. **Given** une page affichée, **When** on pince pour l'agrandir ou qu'on la touche deux fois, **Then** elle s'agrandit autour du point touché ; elle reste nette à chaque grossissement, jusqu'au plus fort ; on la parcourt du doigt dans tous les sens ; un nouveau double toucher la ramène à la largeur de l'écran.
4. **Given** le lecteur, **When** on lit le pied de page, **Then** il porte le numéro imprimé sur la page du document et le nombre de pages — « Page 59 sur 92 » —, la section en cours quand le sommaire existe, et la jauge de progression.
5. **Given** un document déjà lu, **When** on le rouvre, **Then** le lecteur reprend à la dernière page lue et le dit — « Reprise à la page 59 — lue hier » —, avec « Début » pour repartir de la première page ; la reprise ne vaut que pour la même version.
6. **Given** un document dont le texte s'est extrait, **When** on appuie longuement sur un mot de la page, **Then** le texte se sélectionne, la sélection s'étend à la main, et « Copier » colle ailleurs les mots de la page dans leur ordre de lecture.
7. **Given** la barre repliée, **When** on touche la page une fois, **Then** la barre se déplie sur « Sommaire », « Rechercher » et « Réglages », comme à l'étape 1 ; un second toucher la replie. Un double toucher agrandit et ne déplie pas la barre.
8. **Given** un document non téléchargé et le réseau présent, **When** on l'ouvre, **Then** la première page paraît avant que le fichier entier soit arrivé, et les suivantes au fil de la lecture ; on peut sauter à une page lointaine sans attendre celles d'avant.
9. **Given** le thème sombre, **When** on lit, **Then** la barre, le pied, les feuilles et le fond autour des pages suivent le thème ; la page reste celle du document, avec ses couleurs.
10. **Given** un document non téléchargé et le réseau coupé, **When** on cherche à l'ouvrir, **Then** l'écran « Ce document n'est pas sur votre téléphone » livré à l'étape 1 s'affiche, inchangé.
11. **Given** un document dont le texte ne s'est pas extrait — un scan —, **When** on l'ouvre, **Then** il s'affiche sur ses pages comme tout autre, et la barre n'offre ni « Sommaire », ni « Rechercher », ni « Texte agrandi ».

---

### User Story 2 — Trouver un passage et y aller (Priority: P1)

Pendant une suspension de séance, Aïssatou cherche « progrès collectifs », sans réseau. « 5 passages dans 92 pages — sans réseau » ; chaque passage donne sa page, sa section et un extrait. Elle touche le troisième : le lecteur ouvre la page 61, et l'expression y est surlignée, à sa place dans la colonne de droite. Plus tard, elle ouvre le sommaire et saute au chapitre 4.

**Why this priority** : c'est la seconde moitié du critère de sortie. Sans la recherche, un guide de 90 pages en PDF ne se consulte pas pendant une séance.

**Independent Test** : sur le guide téléchargé et sans réseau, chercher dix expressions tirées du texte, ouvrir chaque passage et constater qu'il est marqué sur sa page ; ouvrir le sommaire et sauter à trois sections.

**Acceptance Scenarios**

1. **Given** « Rechercher » dans un document dont le texte s'est extrait, **When** on tape une expression, **Then** l'écran dit combien de passages et sur combien de pages, liste chaque passage avec sa page, sa section et un extrait où l'expression est marquée, et signale « vous êtes ici » sur celui de la page en cours — comme à l'étape 1, avec ou sans réseau.
2. **Given** un passage choisi, **When** le lecteur y va, **Then** il ouvre la page du PDF, amène le passage à l'écran, le marque d'un surlignage plein et les autres passages de la page d'un surlignage clair, et dit « Occurrence 3 sur 5 » avec « précédente », « suivante » et la fermeture.
3. **Given** une occurrence que le lecteur ne retrouve pas à sa place sur la page du PDF, **When** on la choisit, **Then** le lecteur ouvre sa page et dit que l'expression s'y trouve sans pouvoir la marquer ; il ne marque jamais un autre endroit.
4. **Given** la recherche, **When** on la ferme, **Then** les surlignages disparaissent et la page en cours reste celle où l'on était allé.
5. **Given** « Sommaire », **When** on l'ouvre, **Then** il est celui de l'étape 1 — chapitres repliables, nombre de sous-parties, page, section en cours — et toucher une entrée ouvre sa page du PDF, en haut de la section.
6. **Given** la bibliothèque, **When** on cherche « un mot du texte », **Then** elle trouve les documents dont le texte extrait porte ce mot, comme à l'étape 1 ; un document dont le texte ne s'est pas extrait n'est trouvé que par son titre, son résumé et son éditeur.

---

### User Story 3 — « Texte agrandi », pour qui lit mal une page réduite (Priority: P2)

Moussa, négociateur, lit mal une page A4 réduite à la largeur de son téléphone, et agrandir puis parcourir chaque ligne du doigt le fatigue. À son premier document, une ligne lui a dit que « Texte agrandi » existe. Page 59, il touche la page, puis « Texte » dans la barre : la même page, recomposée à la largeur de l'écran, en Très grande taille. Un tableau y est remplacé par un renvoi ; il le touche et retrouve le tableau sur la page d'origine. Le lendemain, Guide Négo s'ouvre de lui-même en « Texte agrandi ».

**Why this priority** : c'est ce qui garde l'acquis de l'étape 1 pour les personnes qui en ont besoin. Le lecteur des pages vit sans lui.

**Independent Test** : sur le guide téléchargé et sans réseau, passer en « Texte agrandi » à une page donnée, changer la taille, toucher un terme anglais, suivre le renvoi d'un tableau, revenir aux pages ; fermer, rouvrir et constater que le mode est gardé ; ouvrir un scan et constater que le mode n'est pas offert.

**Acceptance Scenarios**

1. **Given** un document où « Texte agrandi » est offert, **When** on déplie la barre, **Then** elle porte ses quatre emplacements — « Sommaire », « Rechercher », le choix « Pages · Texte » à la place de « Marquer », « Réglages » —, le mode en cours marqué.
2. **Given** le même document, **When** on ouvre « Réglages », **Then** la feuille offre aussi le choix du mode, le thème, et, en « Texte agrandi » seulement, les trois tailles de l'étape 1, de 17 à 24 px.
3. **Given** un téléphone où aucun document n'a encore été ouvert, **When** on ouvre un premier document où « Texte agrandi » est offert, sur un écran étroit, **Then** une ligne d'information dit que « Texte agrandi » existe et où le trouver ; elle ne revient plus, sur aucun document, une fois vue.
4. **Given** la page 59 en cours, **When** on passe d'un mode à l'autre, **Then** l'autre mode s'ouvre sur la page 59, et le pied de page garde le même numéro.
5. **Given** « Texte agrandi », **When** on lit, **Then** le texte est celui du lecteur de l'étape 1 : recomposé à la largeur, sans défilement horizontal, termes anglais touchables ouvrant la feuille du terme, thème suivi.
6. **Given** un tableau ou une figure dans une page, **When** on la lit en « Texte agrandi », **Then** un renvoi nommé « Tableau » ou « Figure » tient sa place, et le toucher ouvre la page du PDF à cet endroit ; aucune image de page n'est gardée pour cela.
7. **Given** un mode choisi, **When** on ouvre un autre document ou qu'on relance l'application, **Then** le même mode s'applique ; le choix vaut pour tous les documents et reste sur le téléphone.
8. **Given** « Texte agrandi » choisi et un document où il n'est pas offert, **When** on ouvre ce document, **Then** il s'ouvre sur ses pages et le dit une fois, sans changer le choix gardé pour les autres documents.
9. **Given** un document dont le texte ne s'est pas extrait, ou pour lequel « Texte agrandi » est retiré, **When** on déplie la barre ou qu'on ouvre « Réglages », **Then** le choix du mode n'y paraît pas.
10. **Given** n'importe quel écran du lecteur, **When** on cherche les réglages, **Then** le bouton s'appelle « Réglages » et son pictogramme ne ressemble pas à « Aa », qui n'ouvre que le lexique.

---

### User Story 4 — La copie gardée : le PDF et son texte (Priority: P2)

Aïssatou télécharge le guide : la fiche annonce 3,2 Mo, ce que la copie occupe vraiment. Plus tard, dans « Mes documents », elle voit cette place, la libère d'un geste, et la jauge baisse d'autant. Sur la tablette de sa délégation, elle avait lu en ligne, sans le télécharger, le résumé réservé ; en se déconnectant, rien n'en reste lisible.

**Why this priority** : la copie change de contenu ; la place annoncée, l'intégrité de la copie et la sûreté des réservés doivent rester vraies. Les gestes, eux, sont ceux de l'étape 1.

**Independent Test** : télécharger le guide et un document réservé, couper le téléchargement au milieu puis le reprendre, comparer la place annoncée et la place mesurée, lire en ligne un autre réservé sans le télécharger, se déconnecter, passer en mode avion et constater qu'aucun réservé ne s'ouvre ; tout retirer et constater la baisse de la place.

**Acceptance Scenarios**

1. **Given** un document téléchargé, **When** on regarde ce que garde le téléphone, **Then** c'est le PDF et le texte extrait — le texte seulement s'il s'est extrait —, et rien d'autre : aucune image de page.
2. **Given** la fiche d'un document, **When** on lit sa taille, **Then** elle est celle de la copie gardée, PDF et texte compris, et « Mes documents » compte la même.
3. **Given** un téléchargement interrompu — réseau perdu, place manquante, annulation —, **When** on regarde le téléphone, **Then** la copie est entière ou absente : jamais un PDF sans son texte, ni un texte sans son PDF, ni un fichier tronqué.
4. **Given** la place, « Libérer », « Tout retirer du téléphone », l'effacement des réservés à la déconnexion et au retrait d'accès, « Remplacé par… » et la dépublication, **When** on les emploie, **Then** ils se comportent comme à l'étape 1 sur la nouvelle copie, et la place baisse d'au moins la taille des copies retirées.
5. **Given** un document réservé lu avec le réseau sans être téléchargé, **When** la personne se déconnecte ou perd l'accès, **Then** aucune de ses pages ne reste lisible sur le téléphone, même en mode avion.
6. **Given** un document public lu avec le réseau sans être téléchargé, **When** le réseau tombe, **Then** il ne se présente pas comme lisible hors connexion ; seule une copie téléchargée l'est.
7. **Given** une copie d'un format ancien — celles de l'étape 1, texte et images de pages, sur les téléphones d'essai —, **When** l'application s'ouvre, **Then** la copie est effacée, sa place rendue, et le document redevient « Non téléchargé » ; il se retélécharge d'un geste.

---

### User Story 5 — La note de correction sur la page (Priority: P3)

Le Dr Koffi Mensah a posé une note sur un passage de la page 59. Aïssatou, sur sa copie gardée, voit en marge de la page un filet rouge à la hauteur du paragraphe visé, et une ligne « Note de correction — passage dépassé ». Elle la touche : la note se déplie, avec le texte, le nom de l'expert et la date, et la page reste visible, le passage à l'écran.

**Why this priority** : c'est ce qui garde le guide juste pendant la COP sans republier ; l'étape 1 l'a livré, il faut le reporter sur la page d'origine. La lecture vit sans lui.

**Independent Test** : poser une note sur un passage de la page 59 et une note sans passage sur la page 60 depuis le back-office, relire la bibliothèque, passer en mode avion, lire les deux notes sur les pages puis en « Texte agrandi », retirer une note et constater qu'elle disparaît au retour du réseau.

**Acceptance Scenarios**

1. **Given** une note posée sur un passage cité, **When** on lit la page du PDF, **Then** un filet rouge et le triangle signalent la note dans la marge, à la hauteur du passage quand le lecteur le retrouve sur la page, sinon en tête de page.
2. **Given** une note posée sans passage, **When** on lit sa page, **Then** elle se signale en tête de page.
3. **Given** la note signalée, **When** on la touche, **Then** elle se déplie — texte, puis signature « Nom, expert IFDD — date » — sans cacher la page : le passage visé reste à l'écran, et un toucher la replie.
4. **Given** une note sur une copie gardée, **When** on lit en « Pages » puis en « Texte agrandi », **Then** elle paraît dans les deux modes ; en « Texte agrandi », elle borde le bloc du passage, comme à l'étape 1.
5. **Given** une note posée ou retirée après le téléchargement, **When** la bibliothèque est relue avec le réseau, **Then** la copie gardée la montre ou la perd sans être retéléchargée.
6. **Given** une note, **When** on la lit, **Then** elle ne modifie jamais la page ni le texte du document : elle se pose à côté.

---

### User Story 6 — Le back-office vérifie ce que sert le texte (Priority: P3)

Mariam publie le guide de la CdP31. L'aperçu lui montre chaque page d'origine et, à côté, le texte qui servira à chercher, au sommaire et au « Texte agrandi ». Sur un bulletin mis en page sur trois colonnes, l'extraction a jugé que le texte se recompose bien, mais il est lu dans le désordre : elle retire « Texte agrandi » pour ce document, garde la recherche et le sommaire, et publie le jour même.

**Why this priority** : la publication de l'étape 1 fonctionne ; ce récit ajuste l'aperçu et remplace « ouvrir tel quel ». Sans lui, un texte mal recomposé serait offert aux personnes qui lisent le moins bien.

**Independent Test** : publier le guide et un PDF à colonnes, retirer « Texte agrandi » au second sans republier le fichier, vérifier sur le téléphone que le mode n'y est plus offert et que la recherche fonctionne ; tenter de publier un PDF protégé par un mot de passe.

**Acceptance Scenarios**

1. **Given** un fichier traité, **When** l'administratrice ouvre l'aperçu, **Then** chaque page d'origine paraît à côté du texte extrait de cette page, qu'elle feuillette page à page ; l'aperçu dit que ce texte sert à la recherche, au sommaire et au mode « Texte agrandi », et que le lecteur montre la page d'origine.
2. **Given** l'aperçu, **When** elle le parcourt, **Then** « Ouvrir tel quel » n'y figure plus.
3. **Given** un fichier traité, **When** l'aperçu s'ouvre, **Then** il donne le verdict de l'extraction, et « Texte agrandi » est proposé si le texte s'est bien recomposé, retiré sinon.
4. **Given** ce choix par défaut, **When** l'administratrice juge autrement, **Then** elle propose ou retire « Texte agrandi » pour ce document, avant ou après la publication, sans republier le fichier ; la recherche et le sommaire restent dans tous les cas, et le téléphone suit le choix à la prochaine lecture de la bibliothèque.
5. **Given** un fichier dont le texte ne s'extrait pas, **When** elle le publie, **Then** l'aperçu l'a dit avant, et le document se publie sans « Texte agrandi », recherche dans le texte ni sommaire.
6. **Given** un fichier que le lecteur ne pourrait pas afficher — protégé par un mot de passe, endommagé —, **When** elle veut le publier, **Then** le back-office le refuse et dit pourquoi.
7. **Given** une note de correction à poser, **When** l'expert choisit la page, **Then** il la voit telle que le lecteur la montre et, s'il le veut, cite le passage visé, comme à l'étape 1.

---

### Edge Cases

- **Une page de format inhabituel** — paysage, A3, affiche. Elle tient la largeur de l'écran comme les autres ; le défilement ne saute pas.
- **Le téléphone tourné en paysage.** La page tient la nouvelle largeur et la page en cours est gardée.
- **Un grand écran** — tablette. La page tient la largeur de la colonne de lecture, sans s'étirer au-delà de ce qui reste lisible.
- **Le plus fort grossissement atteint.** Le pincement s'arrête ; la page reste nette. En parcourant une page agrandie, seul le contenu de la page se déplace : l'interface du lecteur, elle, ne défile jamais de côté.
- **Un toucher, un double toucher, un appui long.** Le toucher déplie la barre, le double toucher agrandit, l'appui long sélectionne : aucun ne déclenche les deux autres.
- **Le réseau tombe pendant la lecture en ligne d'un document non téléchargé.** Les pages déjà affichées restent à l'écran ; les autres disent qu'il faut le réseau et proposent « Télécharger au retour du réseau ».
- **Un document réservé est lu en ligne quand la session expire.** Le lecteur se ferme sur le verrou à la prochaine lecture de l'accès, comme à l'étape 1 ; les pages en ligne ne s'obtiennent plus.
- **Une recherche qui trouve une expression à cheval sur deux colonnes ou deux pages.** Le lecteur marque ce qu'il retrouve sur la page de l'occurrence ; s'il ne retrouve rien, il le dit (récit 2, scénario 3).
- **Un passage de note qui figure deux fois sur la page.** La note se signale à la hauteur de la première occurrence.
- **Le numéro imprimé diffère de la position** — couverture et pages liminaires non numérotées, chiffres romains. Le pied porte le numéro imprimé ; une page sans numéro imprimé porte sa position.
- **Un document au texte partiellement extrait** — quelques pages scannées au milieu. Il s'ouvre sur ses pages ; la recherche ne couvre que les pages dont le texte s'est extrait, et le dit ; « Texte agrandi » suit le verdict de l'extraction et le choix de l'administratrice.
- **Le téléphone manque de mémoire** — un Android d'entrée de gamme. Le lecteur ne garde prêtes que les pages proches de celle qu'on lit ; il ne se ferme pas et ne recharge pas l'application au milieu du document.
- **Le téléphone n'a plus de place pour une copie.** Comme à l'étape 1 : le téléchargement échoue proprement, dit la place qui manque et renvoie à « Mes documents ».
- **Le texte copié depuis un document réservé.** La copie se fait comme pour un public : la personne a l'accès, et ce qu'elle en fait relève d'elle.

## Requirements *(mandatory)*

### Lire les pages

- **FR-001** : Le lecteur DOIT afficher tout document qui est un fichier sur ses pages d'origine — mise en page, colonnes, tableaux, figures, couleurs —, que son texte se soit extrait ou non.
- **FR-002** : Les pages DOIVENT se suivre en défilement continu, la page tenant d'abord la largeur de l'écran ou de la colonne de lecture.
- **FR-003** : Le lecteur DOIT permettre d'agrandir d'un pincement ou d'un double toucher, autour du point touché, jusqu'à au moins quatre fois la largeur de l'écran ; un double toucher DOIT ramener à la largeur.
- **FR-004** : Une page DOIT rester nette à tout grossissement : elle se redessine au grossissement atteint, et non par agrandissement d'une image.
- **FR-005** : Le pied de page DOIT porter le numéro imprimé de la page — sa position s'il n'en a pas —, le nombre de pages du document, la section en cours quand un sommaire existe, et la jauge de progression.
- **FR-006** : Le lecteur DOIT reprendre à la dernière page lue de la même version, dans l'un ou l'autre mode, et le dire, avec « Début » ; la progression reste sur l'appareil.
- **FR-007** : Dans un document dont le texte s'est extrait, le texte d'une page DOIT se sélectionner et se copier dans son ordre de lecture.
- **FR-008** : Un toucher DOIT déplier ou replier la barre de l'étape 1 ; le double toucher et l'appui long NE DOIVENT pas la déplier.
- **FR-009** : Avec le réseau, un document non téléchargé DOIT afficher sa première page sans attendre le fichier entier, puis les pages au fil de la lecture, y compris une page lointaine atteinte par le sommaire ou la recherche.
- **FR-010** : Le lecteur NE DOIT garder prêtes que les pages proches de celle qu'on lit, pour tenir la mémoire d'un téléphone de milieu de gamme sur un document de cent pages, sans fermeture ni rechargement.
- **FR-011** : L'interface du lecteur — barre, pied, feuilles, fond autour des pages — DOIT suivre le thème de l'application ; la page du document NE DOIT jamais être recolorée.
- **FR-012** : Le lecteur DOIT tourner dans l'application, sans renvoyer au visionneur du téléphone ni à une autre application.

### Chercher et naviguer

- **FR-013** : La recherche dans le document DOIT reposer sur le texte extrait, fonctionner sans réseau sur une copie gardée et avec le réseau sur un document non téléchargé, et rendre ce qu'elle rendait à l'étape 1 — nombre de passages et de pages, page, section et extrait de chaque passage, « vous êtes ici ».
- **FR-014** : Un passage choisi DOIT ouvrir sa page du PDF, amener le passage à l'écran et le marquer d'un surlignage plein, les autres passages de la page d'un surlignage clair, avec « Occurrence n sur N », « précédente » et « suivante ».
- **FR-015** : Un passage que le lecteur ne retrouve pas sur la page du PDF DOIT ouvrir sa page en disant qu'il ne peut pas le marquer ; le lecteur NE DOIT jamais marquer un autre endroit.
- **FR-016** : Le sommaire de l'étape 1 DOIT mener à la page du PDF de l'entrée choisie.
- **FR-017** : La recherche « un mot du texte » de la bibliothèque DOIT continuer de porter sur le texte extrait.
- **FR-018** : Un document dont le texte ne s'est pas extrait NE DOIT offrir ni « Rechercher », ni « Sommaire », ni « Texte agrandi » ; sa fiche DOIT dire qu'on le lit sur ses pages, sans recherche dans le texte.

### « Texte agrandi »

- **FR-019** : Le lecteur DOIT offrir un second mode, « Texte agrandi », qui est le lecteur de l'étape 1 : texte recomposé à la largeur de l'écran, trois tailles de 17 à 24 px, termes anglais touchables ouvrant la feuille du terme, sans défilement horizontal, thème suivi.
- **FR-020** : Le choix « Pages · Texte » DOIT occuper, dans la barre dépliée, l'emplacement laissé libre par « Marquer », et figurer aussi dans « Réglages » ; le passage d'un mode à l'autre DOIT garder la page en cours.
- **FR-020 bis** : Au premier document ouvert où « Texte agrandi » est offert, sur un écran étroit, une ligne d'information DOIT dire une seule fois par téléphone que ce mode existe et où le trouver.
- **FR-020 ter** : Le bouton des réglages DOIT s'appeler « Réglages », et son pictogramme NE DOIT pas ressembler à « Aa » : « Aa » n'ouvre que le lexique, sur tous les écrans (ADR-018).
- **FR-021** : Le mode choisi DOIT valoir pour tous les documents et rester sur le téléphone ; un document où « Texte agrandi » n'est pas offert DOIT s'ouvrir sur ses pages, le dire, et ne pas changer ce choix.
- **FR-022** : « Texte agrandi » NE DOIT être offert que si le choix du document le propose. Ce choix suit par défaut le verdict de l'extraction — proposé si le texte s'est bien recomposé, retiré sinon — et l'administratrice le change ; un document dont le texte ne s'est pas extrait ne peut pas le proposer.
- **FR-023** : En « Texte agrandi », un tableau ou une figure DOIT être remplacé par un renvoi nommé qui ouvre la page du PDF à cet endroit.
- **FR-024** : La taille du texte et les termes touchables NE DOIVENT exister qu'en « Texte agrandi » ; en « Pages », la feuille « Réglages » offre le mode — quand il est offert — et le thème.

### Garder

- **FR-025** : La copie gardée DOIT contenir le PDF et le texte extrait — le PDF seul si le texte ne s'est pas extrait —, et aucune image de page.
- **FR-026** : La copie gardée DOIT être entière ou absente ; une interruption NE DOIT laisser ni fichier tronqué ni moitié de copie lisible.
- **FR-027** : La taille annoncée sur la fiche et comptée dans « Mes documents » DOIT être celle de la copie gardée.
- **FR-028** : La place, « Libérer », « Tout retirer du téléphone », l'effacement des réservés à la déconnexion et au retrait d'accès, « Remplacé par… », la dépublication et le téléchargement au retour du réseau DOIVENT se comporter comme à l'étape 1.
- **FR-029** : Les pages d'un document réservé lu en ligne sans téléchargement NE DOIVENT pas rester lisibles sur le téléphone après une déconnexion ou un retrait d'accès ; un document lu en ligne sans téléchargement NE DOIT pas se présenter comme lisible hors connexion.
- **FR-030** : Le fichier d'un document réservé NE DOIT être servi, même par morceaux, qu'à une personne disposant de l'accès négociateur, vérifié à chaque demande ; le refus DOIT être tenu par l'API.
- **FR-031** : La copie gardée DOIT porter un numéro de format. Une copie d'un format que l'application ne lit plus DOIT s'effacer, sa place rendue, et le document redevenir « Non téléchargé » — pour les copies de l'étape 1 comme pour tout changement de copie à venir.

### Corriger

- **FR-032** : Une note de correction DOIT se signaler dans la marge de la page du PDF — filet rouge et triangle —, à la hauteur de son passage quand le lecteur le retrouve sur la page, en tête de page sinon ou quand aucun passage n'est cité.
- **FR-033** : La note dépliée DOIT porter son texte et sa signature, et laisser la page visible, le passage visé à l'écran.
- **FR-034** : Les notes DOIVENT paraître dans les deux modes, sur une copie gardée comme en ligne, et se mettre à jour sans retélécharger le document — FR-049 et FR-050 de l'étape 1 restent.
- **FR-035** : Une note NE DOIT jamais modifier la page ni le texte du document.

### Publier (back-office)

- **FR-036** : L'aperçu DOIT montrer chaque page d'origine à côté du texte extrait de cette page, et dire que ce texte sert à la recherche, au sommaire et à « Texte agrandi ».
- **FR-037** : « Ouvrir tel quel » DOIT disparaître, remplacé par le choix de proposer ou retirer « Texte agrandi » : l'aperçu donne le verdict de l'extraction et pose le choix par défaut qui en découle, et l'administratrice le change, avant ou après la publication, sans republier le fichier. La recherche et le sommaire NE DOIVENT pas en dépendre.
- **FR-038** : Un fichier que le lecteur ne peut pas afficher — protégé par un mot de passe, endommagé — NE DOIT pas pouvoir être publié ; le refus dit pourquoi.
- **FR-039** : Le traitement d'un fichier NE DOIT plus produire d'images de page pour le téléphone.

### Ce qui vaut pour tout le cycle

- **FR-040** : Les écrans DOIVENT tenir à 360 px sans défilement horizontal de l'interface, dans les deux thèmes, cibles tactiles comprises, et porter leurs quatre états : chargement, vide, erreur, accès refusé.
- **FR-041** : Les écrans DOIVENT être bâtis sur les composants de Guide Négo ; tout composant nouveau DOIT être ajouté à la page interne des composants. Le back-office garde ceux de l'ePavillon.
- **FR-042** : Les textes d'interface nouveaux DOIVENT vivre dans les fichiers de traduction de leur écran, en français et en anglais.
- **FR-043** : Tout ce qu'il faut pour lire un PDF sans réseau DOIT être présent dans l'application installée, et se prouver en mode avion.
- **FR-044** : Les écarts entre la maquette et ce cycle — la page du PDF par défaut, le choix du mode à la place de « Marquer », le pictogramme de « Réglages » — DOIVENT être inscrits dans [05-design.md](../../docs/AppNego/05-design.md) (écarts 43 et 44) ; la règle d'ADR-021 qui tient le PDF hors du téléphone DOIT être remplacée par un ADR.

### Key Entities

- **Fichier d'un document** : le PDF publié, sa taille, son nombre de pages, les numéros imprimés de ses pages, son sommaire, son texte extrait page par page, le verdict de l'extraction et **le choix d'offrir ou non « Texte agrandi »**. Il ne porte plus d'images de page ni de mode « tel quel ».
- **Copie gardée** : le PDF et le texte extrait d'un document, sur un appareil, avec son numéro de format, sa date et sa place. Entière ou absente. Propre à l'appareil.
- **Progression de lecture** : la dernière page lue d'une version, commune aux deux modes. Propre à l'appareil.
- **Réglage de lecture** : le mode — « Pages » ou « Texte agrandi » —, la taille du texte, et si la ligne qui annonce « Texte agrandi » a été vue. Propre à l'appareil, pour tous les documents.
- **Note de correction** : inchangée — page, passage cité facultatif, texte, auteur, date, retrait.

### Ce que le modèle change

Le SQL se modifie d'abord, par une migration rejouable qui ne détruit pas la base locale, puis le code s'écrit.

| Changement | Ce qu'il faut | Où |
|---|---|---|
| Le choix « ouvrir tel quel » | Devient le choix de proposer « Texte agrandi », posé par défaut d'après le verdict de l'extraction — nom, sens et défaut changent | `100_negotiations.sql` |
| Les images de page | Ne se produisent plus ; ce qui les portait tombe, et les images déjà déposées se purgent | `100_negotiations.sql` et le bucket privé |
| Le poids de la copie gardée | Se recalcule : PDF et texte extrait | `100_negotiations.sql` |
| La position du texte sur la page | Pour marquer un passage trouvé ou cité, le lecteur doit le retrouver sur la page : par le texte que le rendu de la page porte déjà, ou par des positions tirées à l'extraction. **Le plan tranche**, et dit ce qui manque au modèle | Au plan |
| Les numéros imprimés des pages | Existent depuis l'étape 1 | Rien |

Ne s'ajoutent **pas** au modèle : le mode de lecture choisi, propre à l'appareil.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001** : Téléchargé la veille, le guide de 90 pages se lit en entier en mode avion, application fermée puis rouverte, sur un Android de milieu de gamme et sur un iPhone : parcouru de la première à la dernière page, l'application ne se ferme pas, ne se recharge pas, et le défilement suit le doigt sans arrêt perceptible.
- **SC-002** : Arrêtée à l'écran, une page s'affiche en entier en moins d'une seconde ; agrandie au plus fort, elle est nette — caractères sans escalier ni flou — en moins d'une seconde après la fin du geste.
- **SC-003** : Avec le réseau, la première page d'un document non téléchargé est lisible en moins de trois secondes sur un réseau ordinaire et en moins de huit sur un réseau bridé, avant que le fichier entier soit arrivé.
- **SC-004** : Sur dix expressions tirées du guide, cherchées sans réseau, les passages sont rendus en moins d'une seconde, et chaque passage choisi est marqué à sa place sur sa page — ou le lecteur dit qu'il ne peut pas le marquer, jamais ailleurs.
- **SC-005** : Rouvert, un document reprend à la dernière page lue dans 100 % des cas pour une même version ; un changement de mode garde la page dans 100 % des cas.
- **SC-006** : Un paragraphe copié depuis une page du guide se colle avec ses mots dans l'ordre de lecture, sans en-tête ni pied de page mêlés.
- **SC-007** : La taille annoncée d'une copie gardée est à 5 % près la place qu'elle occupe ; après « Tout retirer du téléphone », la place affichée baisse d'au moins cette taille, sans délai, et aucune image de page ne reste sur le téléphone.
- **SC-008** : Après une déconnexion, aucune page d'un document réservé — téléchargé ou lu en ligne — ne s'affiche en mode avion.
- **SC-009** : Aucune API ne sert le fichier d'un document réservé, entier ou par morceaux, à une personne sans accès — les tests d'intégration le prouvent, adresse forgée comprise.
- **SC-010** : Une note posée depuis le back-office paraît sur la copie gardée, dans les deux modes, à la première lecture de la bibliothèque qui suit ; posée sur un passage du guide, elle se signale à sa hauteur.
- **SC-011** : Les écrans sont fidèles à la maquette à 360 px, en thème clair et sombre, aux écarts inscrits près ; en thème sombre, la page du document garde ses couleurs.
- **SC-012** : « Texte agrandi » n'est offert sur aucun document dont le texte ne s'est pas extrait ou pour lequel il est retiré ; là où il est offert, une personne qui ne le connaît pas le trouve sans ouvrir « Réglages ».

## Assumptions

- **Les pages se dessinent dans l'application, pas dans le visionneur du téléphone.** Hors de l'application, une application installée sur iPhone n'en revient pas proprement, et ni la recherche marquée sur la page ni les notes n'y paraîtraient. Le moyen de dessiner les pages — et ce qu'il ajoute à l'application installée — se choisit au plan, par un ADR qui remplace « le PDF ne va jamais sur le téléphone » d'ADR-021.
- **« Net à tout grossissement »** se tient jusqu'à quatre fois la largeur de l'écran au moins : au-delà, un tableau du guide se lit déjà caractère par caractère. Le plus fort grossissement se fixe au plan sur le vrai guide.
- **« Texte agrandi » remplace « ouvrir tel quel »** (arbitré le 24/09). Un texte lu dans le désordre, offert aux personnes qui lisent le moins bien, serait pire que pas de texte (constitution, XII) : le verdict de l'extraction pose le choix par défaut, l'administratrice le corrige. Le choix ne touche ni la recherche ni le sommaire, qui tolèrent un ordre imparfait.
- **Le mode se choisit là où on le voit** (arbitré le 24/09) : ceux qui ont besoin de « Texte agrandi » sont ceux qui peinent à lire la page, et ne doivent pas fouiller une feuille de réglages. La barre dépliée garde ainsi les quatre emplacements de « 02 Barre dépliée ». Le pictogramme actuel des réglages dessine deux « A » : il change, et le nouveau entre dans la famille des pictogrammes.
- **« Écran étroit »** désigne un écran de téléphone tenu en portrait ; le seuil se fixe au plan. Sur une tablette, la page du PDF se lit déjà à une taille confortable et la ligne ne paraît pas.
- **Le mode par défaut est « Pages »**, à la première ouverture, pour toute personne.
- **Les tailles de « Texte agrandi »** sont les trois de l'étape 1 — Normale, Grande, Très grande —, de 17 à 24 px.
- **Les exemples reprennent la maquette** : « Page 59 sur 92 » et « 5 passages dans 92 pages » ; le vrai guide de la CdP30 en fait 90, et c'est sur lui que se mesure le critère.
- **La copie gardée pèse environ 3,2 Mo pour le guide** — 2,9 Mo de PDF et 297 Ko de texte —, contre 3,75 Mo à l'étape 1 : la décision allège le téléphone.
- **Rien à migrer en production** : l'étape 1 n'est pas fusionnée dans `main` avant ce cycle, et le lecteur recomposé n'y part jamais. Seules existent les copies des postes et téléphones d'essai, que le numéro de format efface (FR-031, arbitré le 24/09).
- **Lire en ligne ne télécharge pas.** Un document lu avec le réseau sans « Télécharger » n'est pas une copie gardée : il ne compte pas dans la place et ne se lit pas hors connexion.
- **Sélectionner et copier** valent pour tout document dont le texte s'est extrait, réservés compris : la personne a l'accès, et le texte copié n'est pas plus sensible que la page affichée. Un scan ne se sélectionne pas — aucune reconnaissance de caractères n'est faite.
- **Le lecteur d'écran** lit la page par son texte extrait, comme il lisait le texte recomposé ; « Texte agrandi » reste le mode le plus accessible.
- **« Marquer » reste absent** (écart 42) : ce cycle ne dessine pas son résultat.
- **La branche** `012-guide-nego-lecteur-pdf` part de `011-guide-nego-documents`, dont les phases sont toutes commitées, et reçoit `main`.
