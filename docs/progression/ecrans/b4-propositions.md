# B4 — Propositions

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de ce module est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md) § PHASE B — B4.

**État** : 🏁 **LIVRÉ** le 21/08 (T001–T160) — **les huit histoires, les trente-sept routes.** `backend/crates/modules/programme` : le dépôt, la machine à états, la liste du comité, la fiche d'évaluation et son voile, l'espace organisation, la correction et le renvoi, les pièces, l'historique et la reprise v1. **37 routes montées sur 37**, frappées sur la vraie application, et **37 opérations documentées** — deux comptes écrits à deux endroits. **Neuf écarts nouveaux le 21/08, n° 102 à 110** ; le n° 102 bloquait la fonctionnalité entière, le n° 104 aurait rendu au déposant la note du comité, le n° 110 fermait la correction d'un dossier retenu. **Trois d'entre eux ont corrigé du code déjà livré**, et aucun n'aurait été vu par relecture — chacun est tombé sur un test. **525 tests** et `make check` **en entier** au vert — volume détruit, schéma rechargé de zéro. **Aucune arête** vers `identity`, `org` ni `event`, **aucun travail différé** — vérifié, pas supposé.

**Une tâche reste ouverte, et c'est la seule** : **T159**. Une session au navigateur a été menée le 21/08 — l'API sert bien ses 37 opérations et ses six codes, les chemins littéraux tiennent sur le serveur monté, et les quatre écrans s'accordent champ pour champ avec ce que le service rend. Mais les parcours du quickstart sont des parcours d'**API**, et le front tourne encore sur les mocks : **T159 se refermera à B7**, quand les deux seront reliés. Le détail de la session est plus bas.

Avant cela, le 20/08 : **spécifié, planifié et découpé** — [`specs/004-propositions/`](../../../specs/004-propositions/spec.md). 8 histoires utilisateur, 17 cas limites, **103 exigences fonctionnelles**, 27 critères de réussite. **Aucune modification du modèle.** Neuf écarts nouveaux, **n° 93 à 101**, dont trois qui auraient cassé à l'exécution. **24 décisions techniques**, **37 routes**, **6 codes d'erreur**, **3 événements émis par le service** (huit le sont par la base), **aucun travail différé**, **une dépendance nouvelle**. Contrôle constitutionnel passé deux fois : **une entorse** et **deux écritures hors schéma**, toutes trois justifiées. **160 tâches en 11 phases, dont 46 de test**, quatre jalons, et **le jalon que le prompt demande à T001–T076**.

---

## Ce que le navigateur a montré (21/08, T159 — partiel)

Session `agent-browser --headed` sur l'API en fonctionnement et sur le front en développement.

**Sur l'API réelle** — pas le harnais de test :

- `GET /api/docs` sert **37 opérations** portant les quatre étiquettes du module, et les **six codes d'erreur** y figurent, engendrés depuis le catalogue du noyau ;
- les six chemins littéraux — `/list`, `/dashboard`, `/committee`, `/transitions`, `/draft`, `/form-context` — répondent **401 et non 404** : l'ordre d'enregistrement tient sur le serveur monté, pas seulement dans le harnais. Le contrôle négatif, `/proposals/list/encore`, rend bien 404 ;
- les cinq écritures — affectation groupée, changement d'état, notation, déport, résolution — répondent **403 `IDENTITY_ORIGIN_REJECTED`, jamais 404** : montées, et gardées par la vérification d'origine avant même le routage.

**Sur les quatre écrans du front** — qui tournent encore sur les **mocks**, le raccordement étant B7. Ce qu'ils prouvent n'est donc pas le comportement de l'API mais **l'accord des formes**, champ pour champ :

- la **liste** (A7) affiche ses facettes dans l'ordre exact que le service rend — statuts et formats dans l'ordre du cycle de vie, les trois signaux transverses ensuite —, avec le rang, les notes et le « 3/3 » ;
- la **fiche d'évaluation** (A8) rend tout ce que la composition sert : historique de participation par organisation, accusés de lecture collectifs, avancement nominatif du comité, revues des pairs, et les trois visibilités de message avec, en toutes lettres, « la demande de correction est toujours partagée avec l'organisation » — l'écart n° 99 est écrit sur l'écran. Un administrateur global y lit « **Lecture seule — vous n'avez pas le droit de noter** » : l'écart n° 50, visible ;
- l'**espace organisation** (A5) et le **dossier du déposant** passent le balayage : ni note, ni rang, ni note pondérée, ni nom de membre du comité, ni « points forts », ni « interne au comité ».

**Une conséquence de l'écart n° 108, rendue concrète par l'écran** : la frise d'un dossier retenu porte aujourd'hui une dernière étape « Programmé », et le dossier du déposant un onglet « Activités et rappels ». L'API les sert **vides** jusqu'à B5 et B6. Au raccordement, la frise perdra donc sa dernière étape et l'onglet affichera zéro — c'est attendu, et il faut que B7 le sache avant de croire à une régression.

**Ce que le navigateur n'a PAS prouvé, et T159 reste donc ouverte** : les parcours du quickstart sont des parcours d'**API** — le voile inspecté sur la charge utile, la consolidation relue en base, le renvoi après clôture, la séance inchangée. Ils sont couverts par les tests d'intégration, pas par une session au navigateur, et le front n'appelle pas encore l'API. La tâche se refermera **à B7**, quand les deux seront reliés.

---

## Phases 9 à 11 — les pièces, l'historique, la reprise v1 et la finition (21/08, T139–T160)

**🏁 B4 est livré : les trente-sept routes du contrat sont montées et frappées sur la vraie application.**

**Le module ne détruit pas ce qu'il n'a pas créé.** Détacher une pièce retire la ligne de rattachement ; l'objet stocké demeure. `media.assets` a son propre cycle de vie — suppression logique, date de purge, worker de purge —, et un même fichier peut être rattaché à deux dossiers : le détruire ici effacerait la pièce d'un autre sans le savoir. Le test relit l'objet **après** le détachement, et c'est pour cette assertion qu'il existe.

**L'objet inconnu est refusé en nommant le champ.** La clé étrangère refuserait aussi, mais son message ne dirait pas lequel — et le contrat d'erreurs annonce `PROPOSAL_UNKNOWN_REFERENCE` « en nommant le champ, comme `ORG_*` et `EVENT_*` le font déjà ».

**La déduction des transitions v1 n'émet rien, et c'est tout le sujet.** Elle écrit dans le journal **sans passer par la mise à jour de l'état**, donc sans réveiller le déclencheur : émettre huit mille événements de dossiers décidés il y a deux ans déclencherait autant de courriels — le pire effet possible d'une reprise. Elle est **rejouable** parce que la condition « journal vide » est *dans* la requête d'insertion, et non un contrôle préalable : deux exécutions simultanées ne peuvent pas semer deux fois le même dossier.

**Une frise qui ne ment pas a demandé une décision que le découpage n'avait pas vue.** Une reprise qui ne recopie pas la date d'origine laisse `created_at` à l'instant de l'import, c'est-à-dire **après** le dépôt et la décision : la frise afficherait « dossier créé » en dernier. L'ouverture est donc **bornée** par les deux autres dates, et ramenée une microseconde avant le dépôt quand elle lui est postérieure. Deux lignes au même instant ne suffisaient pas : `platform.uuid_v7()` n'est **pas** ordonné à l'intérieur d'une milliseconde, et l'ordre serait tombé au hasard — mesuré, pas supposé.

**Elle ne devine ni l'évaluation ni une demande de correction.** Ce qui n'est pas dans les dates du dossier n'est pas déductible, et l'inventer serait pire qu'un trou.

**L'effacement d'un dossier purge ses thématiques à la main** (écart n° 94). `reference.entity_terms` est polymorphe : aucune clé étrangère vers les propositions, aucune cascade, et la fonction de purge annoncée par le modèle n'existe pas. Sans cet appel, les liens d'un dossier effacé restent en base — invisibles, mais comptés par tout ce qui agrège par thématique.

**Le test de montage compte trente-sept, et le contrôle OpenAPI aussi.** Deux comptes écrits à deux endroits : une route ajoutée sans être montée fait échouer le premier, une route annotée sans être montée ferait rendre 404 à la documentation. **Un défaut réel a été attrapé par là** : le scope `/admin/proposals` avait été écrit mais pas monté — le remplacement de texte n'avait pas pris, et la route rendait 404 en silence. C'est exactement ce que ce test existe pour voir.

**Aucun travail différé n'est enregistré, et c'est vérifié plutôt que supposé.** C'est le premier module du jalon dans ce cas : rien ici n'a d'effet à échéance. La déduction v1 est **synchrone** — son résultat doit être lu par celui qui la lance, pas remplacé par un identifiant de tâche.

---

## Phase 8 — US6 : corriger et renvoyer (21/08, T128–T138)

**Écart n° 110 — un dossier RETENU restait fermé à la modification, et rien ne le demandait.**

Les fondations avaient posé `est_modifiable()` sur quatre états — brouillon, déposé, en évaluation, corrections demandées — et fermé la porte dès l'acceptation. **Ni le contrat d'erreurs ni l'arbitrage du commanditaire ne le demandent** : `PROPOSAL_NOT_EDITABLE` y est décrit comme « dossier **rejeté, retiré, annulé**, ou édition terminée », et le commanditaire a tranché le 17/08 — « tant que l'événement n'est pas terminé, il peut modifier ».

Le coût était concret : une organisation retenue qui repère une coquille dans son intitulé trois jours avant sa séance n'aurait eu **aucun moyen** de la corriger. Le découpage l'avait vu sans le dire — T138 demande de vérifier qu'un dossier **retenu** se corrige sans que sa séance bouge, ce qui suppose qu'on puisse le corriger.

**Corriger un dossier retenu ne déplace pas sa séance, et c'est le test le plus cher à ne pas avoir.** Une séance a un créneau **arbitré** par l'IFDD, une salle attribuée, des inscrits prévenus, des rappels programmés. Recopier dessus le créneau *souhaité* d'un dossier corrigé déplacerait une séance à laquelle quarante personnes se sont inscrites, **sans que personne l'ait demandé** (FR-091). Le test change le titre, le créneau, le format et la durée du dossier, puis relit la séance champ par champ. Le commentaire qui l'interdit est à l'endroit exact où la tentation existe — juste avant la validation de la transaction d'écriture.

**Le renvoi et le dépôt sont deux gestes, portés par deux routes.** La fenêtre de l'appel ne borne que le **premier** dépôt : le comité demande une correction à huit jours de la clôture, l'organisation répond trois jours après l'échéance, et lui opposer la clôture serait lui reprocher un délai qu'elle n'a pas choisi. Le déclencheur du modèle le sait déjà. **Le plafond, lui, s'applique aux deux.**

**Le geste n'est pas déduit de l'état, et c'est la correction apportée à US1** : déduire ferait accepter un renvoi par la route de dépôt, et un dossier corrigé franchirait la clôture sans que personne l'ait décidé. Chaque route refuse maintenant l'état qui n'est pas le sien, **après avoir enregistré la saisie** — si le dossier n'est pas dans l'état attendu, ce qui vient d'être tapé est déjà sauvegardé, et c'est ce qui compte pour qui vient de taper.

**L'heure murale fait l'aller-retour, et le test le prouve dans les deux sens** : saisie 14:30 à Belém, relue 14:30, et **17:30 en UTC**. La seconde lecture est celle qui compte — sans elle, une recomposition qui rendrait la chaîne telle quelle passerait.

**Les textes provisoires ne reviennent jamais au formulaire** (écart n° 102). Le test vérifie qu'ils sont **bien en base** — sans eux la ligne n'existerait pas — et **absents de la recomposition**.

**Une édition terminée ferme la modification**, et c'est une règle que la base ne tient nulle part : `proposals` n'a aucune contrainte liée à `events.ends_at`.

---

## Phase 7 — US5 : l'organisation suit son dossier (21/08, T116–T127)

**Le test central ne vérifie pas des champs : il balaie la charge utile entière.** C'est la seule forme qui prouve quelque chose ici — vérifier que la note est nulle laisserait passer celle qui arriverait par un champ ajouté demain, et c'est exactement le défaut que FR-076 et FR-077 existent pour empêcher. Le test cherche les **valeurs** dans la réponse sérialisée : la note, la note pondérée, le texte d'une délibération, celui d'une note personnelle, le nom d'un membre du comité.

**Écart n° 109 — le contrat du front demande deux choses incompatibles dès qu'un membre du comité écrit au déposant.**

`ProposalFile.participants` existe « pour ne pas résoudre les noms un par un », et FR-077 interdit qu'un **nom de membre du comité** atteigne le déposant. Les deux ne peuvent pas être vrais ensemble sur une demande de correction, qui est le cas ordinaire.

**FR-077 l'emporte, et le filtrage est à la source** : seuls les auteurs **membres de l'organisation porteuse** sont nommés. Le message garde son identifiant d'auteur — l'écran l'affiche sous un libellé neutre, et aucune route de cet espace ne permet de résoudre cet identifiant en nom. Relâcher la règle est une ligne à retirer ; l'inverse ne l'est pas, et une fuite de nom ne se reprend pas. **À arbitrer** : le libellé neutre est un choix d'interface que le commanditaire peut vouloir trancher autrement.

**Écart n° 107 — la liste des membres d'une organisation n'a aucune porte.** Le module Organisations n'expose pas de route de liste de membres, et l'espace organisation la demande. La lecture est faite ici, et c'est un **élargissement de la lecture hors schéma n° 6** — « cette personne peut-elle écrire au nom de l'organisation ? » devient « qui le peut ? » —, non une porte nouvelle. À reprendre par B2 si une telle route naît.

**Écart n° 108 — deux blocs que ce jalon ne peut pas remplir, et qui ne mentent pas.** Les séances programmées appartiennent à B5, leurs rappels à B6. Les listes partent **vides**, jamais absentes : un champ absent ferait échouer l'écran, un champ vide dit qu'il n'y a rien. Ce sont des faits de calendrier, pas des oublis.

**L'adhésion active est le seul droit d'entrée.** Un administrateur de l'édition n'entre pas dans l'espace d'une organisation dont il n'est pas membre — il a la fiche du comité pour cela —, et une adhésion **en attente** n'ouvre rien. Les trois refus sont celui d'une ressource inexistante.

**Le déposant pose la résolution, le comité la retire** (écart n° 35). Qui peut le faire n'est écrit nulle part dans le modèle — `resolved_by` est une simple clé étrangère —, et l'arbitrage d'A5 tranche : c'est le déposant qui sait qu'il a corrigé. Le retrait, lui, n'est ouvert qu'au comité : un déposant qui retirerait sa propre résolution ne changerait rien d'utile, mais un déposant qui retirerait celle du comité **effacerait un arbitrage**. Le compteur de demandes ouvertes suit les deux gestes, et c'est pour cela qu'aucun des deux n'émet d'événement — l'état visible est relu à chaque affichage.

---

## Phase 6 — US4 : la fiche d'évaluation, et le voile (21/08, T094–T115)

**Le voile n'est pas un filtre, et le test le prouve sur la charge utile.** Quand il est baissé — appel en aveugle, lecteur affecté, sa revue non déposée —, la requête qui lit les revues des pairs **n'est pas exécutée**. Lire puis vider les champs sensibles laisse la donnée à portée d'un champ oublié dans un type de sortie, d'une trace de débogage, d'un message d'erreur enrichi : c'est le patron qui a produit, en v1, des notes internes visibles dans une réponse JSON que l'écran n'affichait pas. **Ne pas lire supprime la classe entière de défauts** — et c'est ce qui rend le test possible : il cherche les points forts et la note personnelle d'un pair dans la réponse **sérialisée**, pas dans les champs qu'on soupçonne.

**Le décompte, lui, est lu.** Compter n'ancre pas ; lire, si. « Deux revues déposées » s'affiche sans qu'aucune ne sorte.

**Trois cas, trois réponses, et le troisième est celui qui décide de la forme de la règle.** Le voile ne se lève pas sur une revue en **brouillon** — elle ne compte dans aucun agrégat et n'est visible d'aucun pair, elle n'a donc rien à lever. Il se lève **à la seconde où sa propre revue part**. Et il ne descend jamais sur **qui décide sans noter** : l'ancrage vise celui qui va poser une note, et masquer les notes à qui doit trancher rendrait la décision impossible.

**La consolidation est appelée, et c'est le seul contrôle qui dise quelque chose de l'écart n° 98.** `refresh_proposal_score()` existe, son commentaire dit « à appeler après toute saisie de note », et aucun déclencheur ne l'appelle. Elle l'est désormais **dans la transaction** du dépôt d'une revue, et les agrégats rendus sont **relus en base** — le test compare les deux, champ par champ. Sans cet appel, la note du dossier, sa moyenne, son nombre de revues et son élimination resteraient aux valeurs de la ligne : **le classement du comité serait faux sans qu'aucune erreur ne le signale**.

**Écart n° 106 — le refus de plafond de note sortait en 500, et le contrat annonçait un 422.**

`tg_check_score_bounds()` lève un `check_violation` par `RAISE … USING ERRCODE`, **sans nom de contrainte** : une vraie contrainte `CHECK` porte toujours le sien. Le catalogue du noyau ne connaissant que des couples `(code, contrainte)`, le refus tombait dans le repli anonyme et rendait une erreur interne — pour un cas parfaitement ordinaire, une note au-dessus du maximum d'un critère.

Le noyau traduit désormais un `23514` sans contrainte comme il traduit déjà un `23001` : **le message français du modèle est rendu tel quel**. Il est écrit pour être lu — « Note 6.00 supérieure au maximum autorisé (5.00) pour ce critère. » —, et le service y ajoute les bornes de la grille, que le déclencheur ne connaît pas.

**Noter exige une affectation, lire n'en exige pas.** Rien ne lie la permission à l'affectation en base : un membre du comité pourrait sinon noter n'importe quel dossier de son édition. Les deux règles sont **décorrélées**, et le test le montre dans les deux sens — une personne non affectée ouvre la fiche et se voit refuser la notation. **Un déport ferme aussi la notation** : le rouvrir contredirait en silence une déclaration d'impartialité.

**Le déport date l'affectation, il ne l'efface pas.** La ligne demeure avec son motif, et c'est elle qui interdit une réattribution silencieuse. Le motif est **obligatoire** : la colonne existe pour tracer l'impartialité du comité, et un déport sans motif ne se relit pas six mois plus tard, quand une organisation conteste.

**Une pièce en quarantaine est rendue sans adresse.** C'est cette nullité qui commande l'avertissement plutôt que le bouton : le comité doit savoir qu'une pièce manque à son dossier, pas cliquer sur un lien mort.

**Une demande de correction écrite « comité » ressort partagée** (écart n° 99), et **un seul événement part** — sur le message partagé, jamais sur une délibération ni sur une note personnelle. Une réponse du déposant est **toujours** partagée et **jamais** une demande : une organisation ne se demande pas des corrections à elle-même.

**La composition écrit, et c'est assumé.** Ouvrir la fiche pose l'accusé de lecture. B3 composait ses six onglets en transaction lecture seule, et c'était juste — il ne s'y écrivait rien. Ici, une écriture a lieu : la composition passe par la porte d'écriture du noyau, qui pose l'acteur et l'identifiant de requête. **L'état « déjà ouvert » est lu AVANT l'appel qui le pose** — la fonction du modèle insère ou incrémente sans distinguer, et lue après elle dirait toujours « déjà vu ».

**`repo/cross.rs` a été découpé en deux, avant d'atteindre la limite.** La fiche d'évaluation y ajoutait six lectures d'affichage et le fichier franchissait les mille lignes. La ligne de partage est claire et l'espace de noms est le même : `mod.rs` porte ce qui **décide** — ascendance, état de l'appel, adhésion, bornes, grille —, `fiches.rs` ce qui **s'affiche**. Une garde n'a pas besoin de vingt-six colonnes ; un en-tête ne se contente pas de quatre.

---

## Phase 5 — US3 : le comité pilote sa liste (21/08, T077–T093)

**Tout l'écran en une réponse.** Les lignes viennent de `programme.v_proposal_dashboard` **telle quelle** : onze sous-requêtes — avancement des revues, classement, alertes, format, pays du porteur, thématiques, co-organisateurs, membres du comité nommés, retards, prochaine échéance, accusés de lecture. La vue a été étendue le 18/08 précisément pour que cet écran tienne en une requête ; recomposer ses jointures ici réintroduirait le défaut que l'extension a corrigé, et entretiendrait deux définitions du même « 2/3 ».

**Les sept facettes se comptent sur les lignes déjà lues** (R16), et le test **recompte** plutôt que de comparer à des nombres écrits à la main : filtre par filtre, statut par statut, thématique par thématique, il exige l'égalité avec les lignes de la réponse. Le jour où la composition ferait deux lectures, ce test tomberait pour la seule raison qui vaille — les deux mesures auraient été prises à des instants différents.

**Une seule exception, et le modèle l'explique lui-même** : les dossiers non lus viennent de `programme.unread_proposals_for()`, qui prend le lecteur en paramètre. « Non consulté » n'est pas une propriété du dossier mais une **relation** entre un dossier et un lecteur — la même ligne est lue par l'un et pas par l'autre —, et la faire dépendre de `current_setting('app.actor_id')` rendrait le résultat invisible à la relecture.

**Le périmètre est vérifié deux fois, et les deux comptent.** L'extracteur de route refuse un périmètre vide ; le service le refuse aussi, parce que ses tests l'appellent sans passer par une route et qu'une garde qui ne vit que dans la couche HTTP n'en est pas une. Les trois états ne se confondent pas : global, détaché sur une édition, et **vide → refus explicite, jamais une liste vide**. Les confondre afficherait « rien à traiter » à qui n'a aucun droit.

**Six identifiants forgés mènent au même refus qu'un inexistant.** Une administratrice **globale** franchit le contrôle de périmètre quel que soit l'identifiant : c'est donc elle qui éprouve le second contrôle, l'existence de l'édition. Quatre des six désignent des objets bien réels — un dossier, une organisation, une personne, un appel —, et c'est ce qui rend le test discriminant : un service qui répondrait « existe » pour ceux-là aurait laissé fuiter la structure de la base. **C'est la comparaison qui compte**, pas le code pris isolément.

**L'affectation groupée rend compte de chaque dossier** : appliqués **plus** écartés **égale** la sélection. Trois écarts nommés, et **le déporté ne se confond pas avec le déjà confié** — `recused_at` n'est pas une suppression mais la trace d'une déclaration d'impartialité, et réattribuer le dossier la contredirait en silence. Un dossier hors périmètre — ou hors permission — rend **le même écart qu'un inexistant, et sans numéro de dossier**.

**Douze dossiers confiés émettent douze événements.** Les compter est le seul contrôle qui dise quelque chose : vérifier leur présence n'en dirait rien. Un consommateur qui recevrait un lot devrait le déplier lui-même, et son échec porterait alors sur douze effets au lieu d'un — la garde de rejeu est par événement. **Et confier ne change aucun état** : le déclencheur d'état ne s'éveille pas, il n'y a donc rien à dédoublonner ici.

**Le moment annoncé depuis US2 est arrivé.** `GET /proposals/{id}` est monté, et six chemins littéraux entrent désormais en concurrence **réelle** avec lui — même méthode, un seul segment. Le découpage `chemins_litteraux` / `chemins_de_dossier`, posé avant ce moment pour que la règle soit tenue par la structure et non par la vigilance, a tenu ; le test de montage le vérifie sur la vraie application, et **il vérifie aussi que `GET /proposals` survit à `POST /proposals`** — deux ressources du même chemin, mesurées plutôt que supposées.

**La liste des colonnes du dossier est écrite deux fois, et c'est assumé** : `sqlx::query!` exige une chaîne littérale pour vérifier la requête à la compilation. Le prix de la vérification est explicite plutôt que contourné par une composition de chaînes qui la ferait perdre.

**Écart n° 104 — le contrat du front laisse la note du comité atteindre le déposant, et deux routes la lui auraient rendue.**

`Proposal` est décrit comme la ligne de table, **agrégats d'évaluation compris** — note moyenne, note pondérée, élimination. Or `GET /proposals/{id}` et `GET /proposals` sont ouvertes **par l'adhésion active** autant que par le périmètre, et FR-077 interdit qu'une note atteigne le déposant. Servir la ligne telle quelle aurait donné à l'organisation sa propre note, sans qu'aucun écran ne l'ait demandée.

Le masquage est **à la source**, jamais à l'affichage : ce qui n'est pas envoyé ne peut pas fuiter, tandis qu'un filtrage à l'affichage devrait être refait dans chaque écran, chaque courriel et chaque export — et le premier oubli est la fuite. Les deux notes partent nulles et l'élimination part fausse, c'est-à-dire « rien à dire » : l'état exact d'un dossier que personne n'a noté. **`review_count` reste** — un nombre de revues déposées n'est ni une note ni un rang, et l'espace organisation affiche l'avancement de l'instruction. Le rang n'est pas concerné : il est calculé par la vue de pilotage, que cette voie n'ouvre jamais.

**Écart n° 105 — confier un dossier à qui ne siège pas au comité n'a aucun refus au contrat.**

`BulkSkipReason` en porte cinq — déjà confié, déporté, transition impossible, motif exigé, introuvable — et **aucun** ne dit « ne siège pas au comité de cet appel ». La base ne l'interdit pas davantage : `review_assignments.reviewer_id` référence `identity.people`, pas `event.call_reviewers`. Une charge utile forgée peut donc créer une affectation hors comité.

**La conséquence est bornée** — être affecté n'accorde pas `programme.review.write`, la notation reste impossible —, mais la personne apparaît alors dans la colonne « qui évalue » de la liste et dans le décompte des revues attendues. Rien n'a été inventé : la personne visée est seulement vérifiée **existante**, une fois pour toute la sélection et non douze fois, faute de quoi la clé étrangère aurait fait échouer le premier dossier avec un refus qui ne nomme pas le champ fautif. **Ajouter le refus demande une valeur de plus au contrat du front, donc un arbitrage** — la question est posée, elle n'est pas tranchée ici.

---

## Phase 4 — US2 : la machine à états, lue et jamais réécrite (21/08, T063–T076)

**🏁 C'est le jalon que le prompt demandait.** T001–T076 : une organisation dépose, corrige et retire ; la machine à états tient. Le comité peut encore travailler sur la v1 le temps que la suite arrive.

**Les transitions offertes tiennent en une requête, et les deux voies y sont distinctes.** Une règle est retenue quand le lecteur est **porteur** — adhésion active — et que la règle l'y autorise, **ou** quand elle nomme une permission qu'il détient **sur l'édition du dossier**. Les confondre casserait les deux extrémités de la table : le retrait par l'organisation ne nomme **aucune** permission — le tester par la permission le rendrait impossible —, et la mise en évaluation n'est **pas** ouverte au porteur — s'en remettre à l'adhésion la lui offrirait.

**Un même dossier, trois lecteurs, trois menus.** La déposante retire (motif exigé) ; le noteur demande des corrections (motif exigé) ; le décideur retient sans motif et rejette avec. **Et le décideur ne peut pas demander de corrections** : le rôle d'administration ne détient pas `programme.review.write` — écart n° 50, une ligne de la table des droits, modifiable au back-office, pas une fatalité du code. Un administrateur d'une **autre** édition ne se voit offrir aucune transition : la portée est celle de l'édition du dossier.

**Une transition acceptée écrit UNE ligne d'outbox, pas deux.** C'est le contrôle qui justifiait l'avertissement n° 1 du découpage, et le seul qui dise quelque chose d'un doublon — vérifier la présence n'en dit rien.

**Les deux refus du garde se distinguent par le moment, jamais par le texte.** `restrict_violation` rend le message français du déclencheur **repris mot pour mot** ; `not_null_violation` rend « motif exigé », traduction **sûre** parce que la transaction n'écrit que `status` et `decision_reason`, deux colonnes nullables où aucune autre violation de non-nullité n'est possible.

**La colonne de décision est écrasée, le journal garde tout** (écart n° 97). Le test enchaîne une demande de correction motivée, un renvoi **sans** motif — qui **efface** la colonne —, puis un retrait motivé : trois écritures, une seule valeur en colonne, tous les motifs au journal avec leur auteur. C'est exactement ce qui interdit à un écran de lire la colonne.

**L'action groupée évalue dossier par dossier, et écarte avant de toucher.** Une sélection peut traverser deux éditions. Tenter les douze et traduire douze exceptions coûterait douze transactions avortées pour la même réponse. Un dossier hors périmètre rend **le même écart qu'un inexistant**, et **sans numéro de dossier** : l'écart ne dit pas à qui forge une sélection que le dossier existe ailleurs.

**Une correction de commentaire, mesurée plutôt que supposée.** J'avais écrit que l'ordre d'enregistrement des routes provoquait un **405** et qu'un défaut venait d'être rattrapé sur `/proposals/status` et `/proposals/transitions`. **Mesure faite sur la vraie application : Actix ne rend pas 405** — quand la méthode ne correspond à aucune route de la ressource, il poursuit, et le chemin non servi finit sur la route par défaut de l'API, donc en **404**. Le risque de capture n'existe donc **que lorsque les méthodes se recouvrent**, ce qui n'est pas encore le cas.

Il le sera **à US4**, quand `GET /proposals/{id}` arrivera : `/list`, `/dashboard`, `/committee`, `/transitions`, `/form-context` et `/draft` entreront alors tous en concurrence avec lui, sur la même méthode. Le découpage en `chemins_litteraux` / `chemins_de_dossier` est posé **avant** ce moment, pour que la règle soit tenue par la structure et non par la vigilance — et les commentaires disent désormais ce qui a été mesuré, pas ce qui avait été supposé.

**L'accès à un dossier passe par deux voies distinctes**, réunies dans une seule fonction : adhésion active à l'organisation porteuse, **ou** lecture générale dans le périmètre de l'édition. Elles sont testées séparément — un membre d'organisation n'a aucun périmètre, un administrateur détaché n'est membre d'aucune organisation —, et l'échec des deux rend le refus d'un dossier inexistant.

---

## Phase 3 — US1 : une organisation dépose son dossier (21/08, T031–T062)

**Écart n° 102 — les trois textes obligatoires butent sur le même vide que l'adresse d'URL, et la spécification ne l'avait pas vu.**

L'écart n° 95 avait relevé que `slug` est obligatoire et absent du contrat du formulaire. **Il n'avait pas relevé que `title`, `objectives` et `detailed_presentation` sont `NOT NULL` et que `platform.i18n_text` refuse un français vide** — or le formulaire commence par l'étape des organisations, si bien qu'au premier enregistrement automatique ces trois champs n'ont jamais été touchés. Le repli d'adresse ne servait donc à rien : la ligne échouait deux colonnes plus loin, et **la fonctionnalité entière ne démarrait pas**.

La réponse est bornée et réversible : le service pose un **texte provisoire** — « Dossier sans titre », « À compléter » —, remplacé dès la première frappe, **refusé au dépôt** (un dossier ne part pas au comité en s'appelant « Dossier sans titre ») et **effacé à la recomposition**, de sorte que le formulaire ne l'affiche jamais. Deux constantes, une fonction, un point de décision.

**Écart n° 103 — un intervenant connu sans compte reste modifiable, et le plan n'avait prévu que la création.**

Le contrat distingue trois cas : inconnu, **connu sans compte** (« elle reste modifiable »), connu avec compte (identité verrouillée). Ne rien écrire pour celui du milieu aurait produit le pire comportement possible : le déposant corrige « Awa Sow » en « Awa Sow Fall », l'écran accepte, l'enregistrement réussit, **et rien ne change** — sans un mot. Un refus serait défendable ; un succès qui n'écrit pas ne l'est pas. L'écriture est donc étendue à **trois colonnes**, et **seulement en l'absence de compte** ; dès qu'un compte existe, le refus est explicite et **nomme la personne**.

**Le réessai suffixé passe par un point de reprise, et il le fallait.** Une violation d'unicité **avorte la transaction** en PostgreSQL : sans `SAVEPOINT`, le premier homonyme aurait rendu la transaction inutilisable et le réessai aurait échoué sur « current transaction is aborted » — c'est-à-dire dès le deuxième dossier portant le même titre.

**Les trois refus de recevabilité sortent en 200, chacun portant sa valeur**, classés avant l'écriture. Et **une course est reclassée, jamais lue au texte** : si le déclencheur refuse entre notre lecture et notre écriture, on relit les trois conditions plutôt que d'interpréter sa phrase française — trois messages, dont deux interpolent des valeurs, changeraient à la première reformulation du SQL.

**Le brouillon est enregistré avant toute décision de recevabilité.** Si l'appel ferme entre le chargement de la page et le clic, l'organisation ne doit pas perdre en plus ce qu'elle venait de saisir.

**L'heure murale fait l'aller-retour, et le test vérifie les deux sens** : saisie à 14:30 à Belém, relue à 14:30 — et **17:30 en UTC**. C'est la seconde lecture qui prouve que la conversion a eu lieu et qu'on n'a pas stocké la chaîne telle quelle.

**Le porteur ne peut pas être son propre co-organisateur**, et ce n'est pas de la coquetterie : le `ON CONFLICT` du déclencheur de synchronisation ferait basculer la ligne en `lead` au prochain enregistrement, **en silence**, et le dossier perdrait un co-organisateur sans qu'aucune erreur ne le dise.

**Une annonce par organisation ajoutée, et aucune au réenregistrement.** Un brouillon s'écrit toutes les deux secondes ; réannoncer à chaque fois inviterait la même organisation cent fois. Le service distingue l'insertion de la mise à jour par `xmax = 0` — la seule façon de le savoir sans une seconde requête dont le résultat pourrait déjà être périmé.

**Trois choix de garde qui méritent d'être dits.** L'organisation porteuse d'un dossier existant vient de **la base**, jamais du corps : un dossier créé ne change pas de porteur. L'appel d'un dossier existant vient **du dossier** : le corps ne peut pas le déplacer d'une campagne à l'autre. Et le décompte du contexte du formulaire **recoupe les organisations reçues avec les adhésions actives** — sans quoi un client lirait le décompte de dossiers d'une organisation dont il n'est pas membre.

**Le fichier de test a été découpé en deux** — le parcours d'un côté, le contenu du dossier de l'autre : le garde-fou de mille lignes vaut pour les tests comme pour le reste, et il a été franchi à l'écriture.

---

## Phases 1 et 2 — ce qui est posé, et ce que chaque décision a coûté (21/08, T001–T030)

**Le crate ne dépend de personne, et c'est vérifié plutôt que constaté.** `cargo tree -p programme | grep -E '(identity|org|event) v'` ne rend rien. La permission `event.call.manage`, qui vient d'un autre module, est une **chaîne lue en base** et non un symbole importé : le garde vit dans le noyau depuis B1, et l'arête n'existe pas.

**Les six codes d'erreur, et pas un de plus.** Sept refus métier de ce module sont déjà des membres d'union du contrat du front et sortent en 200 avec leur discriminant. **Aucun code n'est ajouté pour la recevabilité** : ses trois refus sont des réponses portant des valeurs, pas des erreurs.

**Les trois contrats d'événements, et surtout ce qui n'y est pas.** `contracts/src/programme.rs` porte les trois charges utiles du service — co-organisation demandée, message partagé, dossier confié. Il ne porte **aucune** charge utile de changement d'état, et son en-tête dit pourquoi à l'endroit exact où l'on serait tenté d'en ajouter une : le déclencheur émet déjà les huit, et un service zélé enverrait tout en double.

**Dix règles pures, éprouvées sans base — 34 tests.** L'adresse d'URL repliée sur titre vide et suffixée sur collision, avec la **marge de trois signes** qui évite qu'un titre de cent soixante caractères échoue à la deuxième tentative seulement ; les huit longueurs alignées sur `TEXT_LIMITS` du front et **comptées en caractères, pas en octets** — les compter en octets reviendrait à refuser le français ; la liste blanche HTML relevée sur la barre d'outils réelle de l'éditeur ; le voile de l'aveugle dans ses **quatre** combinaisons, dont celle de l'administrateur qui décide sans noter et n'est donc pas voilé ; la recevabilité classée **dans l'ordre du déclencheur**, sans quoi le contrôle préalable et le dernier mot diraient deux choses différentes du même dépôt ; les facettes comptées sur les lignes, avec l'ordre de première apparition retenu — une table de hachage rebattrait les filtres à chaque rechargement.

**Une divergence assumée avec B3, et elle est raisonnée** : B3 **ignorait** les codes de thématique inconnus, faute d'un code d'erreur pour le dire. Ici le catalogue en porte un, et la classification est une étape à part entière du formulaire : un code inconnu est **refusé en le nommant**. Accepter en silence une pastille périmée ferait déposer un dossier que le comité ne retrouverait sur aucun filtre.

**Deux pièges du modèle rencontrés à l'écriture, et réglés là où ils se posent.** La liste de thématiques est **dédoublonnée avant l'insertion** : sans cela, deux fois la même pastille rendrait un décompte inférieur à la liste reçue, et le contrôle accuserait un code parfaitement valide. Et `media.assets` ne porte pas de colonne `file_name` mais `original_filename`, **nullable** — relevé par le refus de SQLx à la compilation, pas par une relecture.

**La résolution d'ascendance sur trois niveaux, et le test qui compare deux refus.** Un dossier hors périmètre et un dossier inexistant rendent **le même code, le même message, le même champ** — c'est la comparaison qui prouve quelque chose, pas le code pris isolément. Le message et la revue remontent de deux sauts, chacun par sa requête, chacun avec son test. Un dossier effacé logiquement est traité comme absent.

**Le préfixe `/organizations` est refactorisé — du code livré est modifié.** `org` expose désormais `organization_routes()`, `api` compose le scope une seule fois, sur le patron de `/people`. **Aucune route n'a changé de chemin**, et `crates/api/tests/routes_org.rs` reste vert : c'est lui qui prouve qu'aucune des vingt et une routes de B2 n'est devenue muette.

**La fabrique de test enchaîne cinq créations**, et c'est le premier parcours de bout en bout du jalon : édition avec fuseau de Belém, appel réellement ouvert au sens d'`event.is_call_open()` — statut **et** fenêtre —, grille par défaut semée par la fonction du modèle, organisation vérifiée, adhésion active. Un test vérifie que la fabrique tient sa promesse : sans lui, un terrain silencieusement cassé ferait échouer les tests de la phase 3 pour la mauvaise raison.

**Ce qui n'existe pas, et c'est une décision** : le crate n'expose **pas** de `job_handlers()`. Ce module ne déclare aucun travail différé, et le worker n'est pas modifié.

---

## Ce qui a été livré

**La spécification** : `spec.md` et sa liste de contrôle qualité (`checklists/requirements.md`), passée sans réserve bloquante. Huit histoires priorisées, un tableau des frontières de données, une section d'hypothèses, une section d'écarts et une liste de ce qui est explicitement laissé au plan.

**Aucune modification du modèle.** `070_programme_proposals.sql` a été relu en entier : ses huit sections portent déjà tout ce que le prompt demande — la machine à états en données avec ses quatorze chemins, le dossier et son numéro attribué dès le brouillon, le journal des transitions et ses deux gardes, la co-organisation tenue en cohérence par déclencheur, les intervenants rattachés à des personnes, les pièces, la grille et sa consolidation, les échanges à visibilité explicite, la vue de pilotage du comité et l'historique champ par champ. S'y ajoutent les sections utiles de `060`, `020`, `030`, `050`, `010` et `910`.

**Le contrat du front est repris tel quel**, sans une renégociation de nom de champ : les trois fabriques d'API (`composables/api/proposals.ts`, `proposal-review.ts`, `organization-workspace.ts`) et les six fichiers de contrats (`types/programme/proposal.ts`, `programme/review.ts`, `proposal-form.ts`, `admin-proposals.ts`, `admin-review.ts`, `organization-workspace.ts`, plus `views.ts` pour la vue de pilotage).

**Les huit histoires, dans l'ordre de priorité** : (P1) une organisation dépose son dossier et son brouillon la suit ; la machine à états est lue, jamais réécrite ; le comité pilote sa liste dans son périmètre ; la fiche d'évaluation compose tout, et le voile tient. (P2) l'organisation suit son dossier sans jamais voir sa note ; un dossier se rouvre, se corrige et se renvoie ; les pièces existent et ce qu'elles montrent est borné. (P3) l'historique dit vrai, même pour les dossiers repris de la v1.

**Les douze exigences du prompt sont toutes traitées** — écarts n° 3, 4, 8, 27, 28, 29 (renvoyé au commanditaire), 30, 32, 35, 37, 38, 39 —, et les deux règles qu'il pose en toutes lettres le sont aussi : le fil rendu au soumissionnaire est filtré **à la source**, et la modification d'un dossier retenu ne touche pas sa séance.

---

## Ce que le plan a ajouté (20/08, `/speckit-plan`)

`plan.md`, `research.md` (**24 décisions**), `data-model.md`, `contracts/` (37 routes, 6 codes d'erreur, 3 événements), `quickstart.md`.

**Contrôle constitutionnel passé deux fois.** Une entorse au principe VIII et deux écritures hors schéma, toutes trois justifiées par écrit :

- **Le classement des trois refus de recevabilité avant l'écriture.** Le contrat du front n'attend pas une erreur mais deux réponses **portant des valeurs** — l'échéance d'un appel clos, le plafond atteint —, que le déclencheur n'expose que dans une phrase française. Et le même code d'erreur PostgreSQL sert aux transitions interdites **et** aux trois refus de recevabilité : sans classement, on ne saurait pas laquelle des quatre causes s'applique. Le déclencheur reste le dernier mot.
- **L'écriture des thématiques** dans le référentiel partagé : même dérogation bornée que B3 s'est accordée pour les fils, la table étant polymorphe et sans clé étrangère.
- **La création d'une personne inconnue** : le point que la spécification avait explicitement laissé au plan. **Tranché en faveur de la dérogation bornée**, et le précédent est livré — le module Organisations crée déjà la personne visée par une invitation. La voie du contrat d'événement rendrait la création différée, donc l'intervenant absent de la réponse, donc le doublon indétectable au moment où le déposant est encore devant son écran.

**Cinq décisions ont été tranchées dans la recherche plutôt que laissées à l'implémentation**, parce qu'une décision tacite y aurait produit une faute **à l'exécution** : la double émission d'événements ; la dérivation de l'adresse d'URL ; l'appel à la consolidation des notes ; la distinction des deux codes d'erreur du garde d'état, qui se ferait sinon au **texte** du message ; et la composition du préfixe `/organizations`.

**Un défaut déjà payé, sur le point de se reproduire, et évité** (écart n° 101 et R18) : deux routes du contrat vivent sous `/organizations`, préfixe qu'un `web::scope` unique du module Organisations occupe. Deux scopes du même préfixe **ne se complètent pas** — Actix retient le premier et rend 404 sur les routes du second. Le défaut a coûté trois routes muettes en B2, et le commentaire qui le raconte est dans le code d'`api`. `/organizations` est donc refactorisé sur le patron de `/people` : **du code livré est modifié**, à annoncer en tête du découpage.

**Aucun travail différé, et c'est une décision** : rien dans ce module n'a d'effet à échéance — les rappels et les avis appartiennent à B6 et se déclencheront sur les trois événements du service, la clôture d'un appel échu appartient à B3. Le worker n'est pas modifié, et un test le vérifie plutôt que de le supposer.

**Une dépendance nouvelle, et une seule** : `ammonia`, pour assainir le HTML de la présentation détaillée à l'écriture (écart n° 32). Ni framework, ni runtime, ni couche d'accès — une fonction pure sur une chaîne, fondée sur un analyseur conforme, parce que le filtrage de HTML écrit à la main est le cas d'école du contrôle qu'on croit avoir.

**Le module où la limite de mille lignes se rapproche le plus** : la fiche d'évaluation compose onze tables. D'où un `domain/` de dix fichiers — dix règles que la base ne porte pas —, et trois fichiers de dépôt hors schéma séparés selon leur régime : deux écritures dérogatoires d'un côté, toutes les lectures de l'autre.

---

## Ce que le découpage a ajouté (20/08, `/speckit-tasks`)

`tasks.md` porte **160 tâches en 11 phases, dont 46 tâches de test et 97 parallélisables**, une phase par histoire et **quatre jalons de livraison**. **Le jalon que le prompt demande est T001–T076** : à ce point, l'appel à propositions de la COP31 **reçoit des dossiers**, et le comité peut encore travailler sur la v1 le temps que la suite arrive.

**Quatre avertissements sont écrits en tête**, chacun parce qu'il coûterait cher découvert en chemin : le déclencheur qui **émet déjà** l'événement de domaine ; le préfixe `/organizations` à refactoriser, qui **touche du code livré** ; le premier enregistrement d'un brouillon qui **échoue** sans la dérivation d'adresse d'URL ; et la consolidation des notes que **rien n'appelle**.

**Un encadré de plus, et il est d'une autre nature** : le point resté sans réponse — qui, dans une organisation, peut agir sur un dossier déjà déposé. L'hypothèse de la spécification est tenue (adhésion active), et elle est **isolée dans une seule fonction** plutôt que répandue dans douze gardes : si le commanditaire tranche autrement, une fonction change et rien d'autre.

**Le découpage a fait apparaître trois dépendances que le plan n'avait pas nommées** : (1) la machine à états ne s'éprouve pas sans le dépôt, faire transiter un dossier supposant un dossier ; (2) la fiche d'évaluation dépend d'une tâche de la machine à états — le champ des transitions offertes est alimenté par la même requête —, seule arête entre deux histoires P1, et elle est étroite ; (3) l'espace organisation dépend d'une tâche de la fiche — le fil filtré par visibilité sert les deux côtés, et l'écrire deux fois produirait deux filtres dont le second finirait par diverger, exactement le défaut que le filtrage à la source doit empêcher.

**L'histoire de l'historique et de la reprise v1 est complètement indépendante** : elle peut se faire à tout moment après les fondations, y compris en parallèle des histoires P1.

---

## Écarts relevés en écrivant la spécification de B4 (20/08)

Numérotation à la suite de B3, qui s'arrêtait à 92. Les n° 93 à 100 viennent de la spécification, le n° 101 du plan. Le détail de chacun, avec ce qu'il coûte et la suite donnée, est dans [`specs/004-propositions/spec.md`](../../../specs/004-propositions/spec.md) § « Écarts relevés ».

| N° | Écart | Où | Suite donnée |
|---|---|---|---|
| **93** | **Le déclencheur d'état ÉMET DÉJÀ l'événement de domaine.** `tg_guard_proposal_status()` appelle `platform.emit_event()` à chaque transition acceptée. C'est **l'inverse de B3**, où aucun déclencheur n'émettait rien — le piège n° 1 de B1 et B2 est de retour | `070` § 3 | **Inscrit à la spécification (FR-043)** : le service n'émet rien sur les changements d'état. Un service qui émettrait aussi enverrait tout en double, et le doublon ne se verrait qu'en production |
| **94** | **`platform.purge_term_links()` n'existe pas.** Le commentaire de `reference.entity_terms` annonce que le nettoyage est assuré par cette fonction ; aucune fonction de ce nom n'est définie dans les dix-neuf fichiers | `020` § 4 | **Le service purge lui-même** (FR-102), ce que l'écart n° 3 demandait déjà. Le commentaire du modèle reste inexact : consigné, non corrigé |
| **95** | **L'adresse d'URL d'un dossier est obligatoire, unique par édition, et le formulaire ne la porte pas.** Pire : le premier enregistrement a lieu à la première frappe, quand le titre est vide, et `slugify('')` rend NULL | `070` § 2, `000` § 5.2 | **Traité dans le service** (FR-013) : dérivation, repli, unicité par suffixe. **Aurait cassé au tout premier appel** du formulaire |
| **96** | **Un dossier peut naître dans n'importe quel état.** Le garde est posé `BEFORE UPDATE OF status` ; à l'insertion, l'autre déclencheur ne fait que journaliser | `070` § 3 | **Traité dans le service** (FR-012) : la création pose toujours « brouillon ». Consigné pour la reprise v1, qui insérera dans l'état final et court-circuitera donc la machine — c'est voulu là |
| **97** | **Le motif d'une transition s'écrit dans la colonne de la décision du comité**, et l'écrase — y compris pour un retrait par l'organisation | `070` § 2 et § 3 | **Le motif se lit dans le journal** (FR-042), qui garde chacun avec son auteur. La colonne n'est écrite que parce que le déclencheur l'exige |
| **98** | **Rien n'appelle `refresh_proposal_score()`.** La fonction existe, son commentaire dit « à appeler après toute saisie de note », et aucun déclencheur ne la déclenche | `070` § 5 | **Le service l'appelle dans la même transaction** (FR-065). Sans cela le classement du comité reste figé **sans qu'aucune erreur ne le signale** |
| **99** | **Une demande de correction peut être invisible de celui qu'elle vise, et compter quand même.** `is_change_request` est indépendant de `visibility`, et le compteur des demandes ouvertes ne regarde pas la visibilité | `070` § 6 et § 7 | **Forcée en visibilité partagée** (FR-070). Le compteur redevient honnête par construction. C'est le cas de l'écart n° 38 sous une autre forme |
| **101** | **Le chemin que le prompt propose pour les transitions offertes est déjà celui du journal.** Le contrat du front lit les lignes de `proposal_transitions` sur `GET /proposals/:id/transitions` ; le prompt y voulait les transitions autorisées | `composables/api/proposals.ts` | **Le journal garde son chemin** ; les transitions offertes prennent `available-transitions`, plus le champ `available_transitions` de la fiche — l'alternative que le prompt offrait lui-même. Relevé au plan, pas à la spécification |
| **100** | **La reprise des activités de la v1 est une coquille vide.** `legacy.migrate_activities()` renvoie deux zéros ; son corps est annoncé dans `scripts/migration/03_activities.sql`, **qui n'existe pas** — le dossier `scripts/` non plus | `910` § 6.3 | **La déduction des transitions devient une opération de CE module** (FR-099, FR-100), rejouable et sans doublon, plutôt qu'une ligne d'un script absent |

### Neuf écarts de plus, relevés en implémentant (21/08)

| N° | Écart | Où | Suite donnée |
|---|---|---|---|
| **102** | **Les trois textes obligatoires d'un dossier butent sur le même vide que l'adresse d'URL.** `title`, `objectives` et `detailed_presentation` sont `NOT NULL`, et `platform.i18n_text` refuse un français vide — or le formulaire commence par l'étape des organisations, si bien qu'au premier enregistrement automatique aucun des trois n'a été touché. L'écart n° 95 avait vu le problème pour `slug` seul : **le repli d'adresse ne servait à rien, la ligne échouant deux colonnes plus loin** | `070` § 2, `000` § 5.1 | **Traité dans le service** : un texte provisoire, remplacé dès la première frappe, **refusé au dépôt** et **effacé à la recomposition**. Deux constantes, une fonction, un point de décision — `domain/draft.rs` |
| **103** | **Le contrat déclare qu'un intervenant connu sans compte « reste modifiable », ce que le plan n'avait pas prévu** : R12 n'autorisait que la **création** d'une personne inconnue. Ne rien écrire aurait produit un enregistrement qui réussit et ne change rien, **sans un mot** | `frontend/app/types/proposal-form.ts`, `030` § 2 | **L'écriture est étendue à trois colonnes — prénom, nom, civilité — et seulement en l'absence de compte.** Dès qu'un compte existe, le refus est explicite et nomme la personne. Bornée à `repo/people.rs`, le fichier qui porte déjà la dérogation |

| **104** | **Le contrat du front laisse la note du comité atteindre le déposant.** `Proposal` y est décrit comme la ligne de table, **agrégats d'évaluation compris** — note moyenne, note pondérée, élimination —, et les deux routes qui le rendent sont ouvertes **par l'adhésion** autant que par le périmètre. FR-077 interdit pourtant qu'une note atteigne le déposant | `frontend/app/types/programme/proposal.ts`, routes 18 et 26 | **Masquage à la source sur la voie de l'organisation** : les deux notes partent nulles, l'élimination part fausse — « rien à dire », l'état exact d'un dossier que personne n'a noté. `review_count` reste : un nombre de revues déposées n'est ni une note ni un rang |
| **105** | **Confier un dossier à qui ne siège pas au comité n'a aucun refus au contrat.** `BulkSkipReason` porte cinq valeurs, aucune ne dit « ne siège pas au comité » ; et `review_assignments.reviewer_id` référence `identity.people`, pas `event.call_reviewers` | `frontend/app/types/admin-proposals.ts`, `070` § 5 | **Rien n'est inventé** : seule l'existence de la personne est vérifiée, une fois pour la sélection. La conséquence est bornée — être affecté n'accorde pas le droit de noter. **Ajouter le refus demande une valeur de plus au contrat : à arbitrer** |
| **106** | **Le refus de plafond de note sortait en 500** là où le contrat annonce un 422 « en nommant le critère et sa borne ». `tg_check_score_bounds()` lève un `check_violation` par `RAISE … USING ERRCODE`, **sans nom de contrainte** — une vraie contrainte `CHECK` porte toujours le sien —, et le catalogue du noyau, qui ne connaît que des couples (code, contrainte), le rangeait dans le repli anonyme | `070` § 5, `kernel/src/pg_error.rs` | **Le noyau rend le message français du modèle tel quel**, comme il le fait déjà pour le garde d'état. Le service y ajoute les bornes de la grille, que le déclencheur ne connaît pas |
| **107** | **La liste des membres d'une organisation n'a aucune porte.** Le module Organisations n'expose pas de route de liste de membres, et l'espace organisation la demande | `org/src/lib.rs`, `frontend/app/types/organization-workspace.ts` | **Lue ici, comme élargissement de la lecture hors schéma n° 6** — « qui peut écrire au nom de l'organisation ? » — et non comme porte nouvelle. À reprendre par B2 si une telle route naît |
| **108** | **Deux blocs de l'espace organisation ne peuvent pas être remplis par ce jalon** : les séances programmées appartiennent à B5, leurs rappels à B6 | `075`, `110` | **Les listes partent vides, jamais absentes** : un champ absent ferait échouer l'écran, un champ vide dit qu'il n'y a rien. Fait de calendrier, pas oubli |
| **109** | **Le contrat du front demande deux choses incompatibles dès qu'un membre du comité écrit au déposant** : `ProposalFile.participants` existe « pour ne pas résoudre les noms un par un », et FR-077 interdit qu'un **nom de membre du comité** atteigne le déposant | `frontend/app/types/organization-workspace.ts`, spec § FR-077 | **FR-077 l'emporte, filtrage à la source** : seuls les auteurs membres de l'organisation porteuse sont nommés. Le message garde son identifiant d'auteur, l'écran affichera un libellé neutre. **Ce libellé est un choix d'interface : à arbitrer** |
| **110** | **Un dossier RETENU était fermé à la modification**, ce que ni le contrat d'erreurs — « dossier rejeté, retiré, annulé, ou édition terminée » — ni l'arbitrage du commanditaire du 17/08 ne demandent. Une organisation retenue qui repère une coquille trois jours avant sa séance n'aurait eu aucun moyen de la corriger | `domain/transitions.rs`, `contracts/errors.md` | **`est_modifiable()` corrigé** : rejeté, retiré, annulé — trois états, pas quatre. **Et l'interdiction de propager vers la séance est posée à l'endroit exact où la tentation existe** : le dossier est la demande, la séance est la décision |

**Quatre écarts antérieurs confirmés et traités ici** : le n° 27 (les bornes d'intervenants de l'appel ne sont vérifiées par aucun déclencheur — FR-028), le n° 30 (rien ne remplit le contact du dossier — FR-032, règle par défaut explicite), le n° 35 (qui pose et qui retire la résolution d'une demande de correction n'est écrit nulle part dans le modèle — le déposant pose, le comité retire) et le n° 94 (la fonction de purge des liens de thématique n'existe pas — le service purge lui-même à l'effacement).

**Trois de ces neuf écarts ont corrigé du code déjà livré** : le n° 106 dans le noyau, le n° 110 dans les fondations de ce module, et le refus de confondre dépôt et renvoi dans US1. Aucun n'aurait été vu par relecture — chacun est tombé sur un test.

**Une observation qui n'est pas un écart de B4** : le cache SQLx hors ligne (`backend/.sqlx`) n'a pas été régénéré depuis B3, et B4 y ajoute une centaine de requêtes. Il n'est employé nulle part — `SQLX_OFFLINE` n'est posé ni dans le `Makefile`, ni dans `.env.example`, ni dans les fichiers de composition —, et la compilation passe donc toujours par la base. Le régénérer produirait un très gros diff sans effet aujourd'hui ; ne pas le faire laisse un piège pour le jour où une compilation hors ligne serait tentée. **À trancher hors de ce jalon.**

**Un écart antérieur reste ouvert et n'est pas de ce module** : le n° 29 (le français obligatoire, découvert par une organisation anglophone à l'étape 2). Il appelle un arbitrage, pas une ligne de code.

**Deux questions restent posées au commanditaire**, et la spécification tient l'option la moins engageante en attendant : la n° 8 (le déposant voit-il sa note et son rang ? — option A tenue depuis A5) et celle qu'ouvre l'écart n° 35 (une résolution posée par le déposant vaut-elle clôture, ou déclaration ? — déclaration tenue).

---

## Ce que la spécification laisse au plan

- **La création d'une personne inconnue** depuis le formulaire : dérogation bornée au schéma d'identité, ou contrat d'événement. La seconde voie interdit la création synchrone dont le formulaire a besoin.
- **Le découpage des fichiers** sous le garde-fou de mille lignes : la fiche d'évaluation compose onze tables, c'est le plus gros assemblage du jalon.
- **La forme de l'opération de déduction** des transitions v1 : route d'administration, travail différé, ou les deux.
- **La liste des travaux différés**, dont aucun n'est exigé par le prompt et dont certains appartiennent peut-être à B6.

---

## Ce qui a été vérifié le 20/08 en écrivant la spécification, et comment

Une spécification ne se prouve pas à l'exécution ; ce qui suit est ce qui a été **lu et recoupé** plutôt que supposé.

| Contrôle | Résultat |
|---|---|
| **Le fichier SQL du module a-t-il été lu en entier ?** | Oui — 935 lignes, huit sections. C'est de cette lecture que viennent les écarts n° 93, 95, 96, 97, 98 et 99, dont aucun n'apparaît dans les types du front |
| **Le déclencheur d'état émet-il un événement de domaine ?** | **Oui** — vérifié dans le corps de `tg_guard_proposal_status()`, qui appelle `platform.emit_event()` avec le numéro de dossier, l'édition, l'organisation et les deux états. B3 avait vérifié l'inverse sur son propre module : la conclusion d'un module ne se transporte pas au suivant |
| **Quelque chose appelle-t-il la consolidation des notes ?** | **Non** — aucun déclencheur sur `reviews` hors l'horodatage, aucun sur `review_scores` hors le contrôle du plafond. La fonction n'est appelée nulle part dans les dix-neuf fichiers |
| **La fonction de purge des liens de thématiques existe-t-elle ?** | **Non** — une seule occurrence de `purge_term_links` dans tout `docs/database/`, et c'est le commentaire qui l'annonce |
| **Le corps de la reprise des activités existe-t-il ?** | **Non** — ni `scripts/migration/03_activities.sql`, ni le dossier `scripts/`. La fonction journalise la correspondance des statuts et rend deux zéros |
| **Les permissions du module existent-elles, et qui les détient ?** | Les quatre sont semées. `programme.review.write` n'est détenue **que** par le rôle de révision et par le compte pivot : l'écart n° 50 d'A8 est confirmé, l'administration ne peut pas demander de corrections **par défaut** — et c'est une ligne de table, pas une fatalité |
| **L'affectation d'un membre du comité a-t-elle sa permission ?** | Non, et l'écart n° 48 d'A7 avait déjà tranché : c'est la gestion de l'appel qui garde ce geste. Comme en B3, le garde vit dans le noyau, **aucune arête entre crates de module n'en découle** |
| **Le contrat du front porte-t-il un champ que le modèle ne peut pas remplir ?** | Un seul, et l'inverse : le modèle exige une adresse d'URL que le contrat ne porte pas (écart n° 95). Aucun champ du front n'est sans source en base |
| **Les deux titres de la vue de pilotage** | Vérifiés dans la vue et dans les types du front : `title` porte le document multilingue brut, `title_text` sa résolution. Les deux noms sont repris tels quels au contrat |
| **La liste de contrôle qualité** | Seize points, tous cochés ; cinq réserves écrites en note, aucune bloquante. Zéro marqueur de clarification |
| **Aucune section normative ne nomme le modèle** | Vérifié mécaniquement sur les 103 exigences : aucun nom de table, de colonne, de contrainte ni de fonction. Ils vivent dans le contexte, les entités, les frontières et les écarts |
