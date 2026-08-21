# Phase 0 — Recherche et décisions techniques : Propositions (B4)

**Date** : 2026-08-20 · **Spec** : [spec.md](spec.md) · **Plan** : [plan.md](plan.md)

Vingt-trois décisions. Aucune n'est un goût : chacune vient du modèle, du contrat du front, ou d'un défaut déjà payé en B1, B2 ou B3. Les alternatives écartées sont écrites, parce qu'une décision sans son alternative se redéfait à la première relecture.

---

## R1 — Le crate s'appelle `programme` et il n'en livre que la moitié

**Décision** : `backend/crates/modules/programme`, un seul crate pour tout le schéma `programme`. B4 y pose la partie « propositions » (fichier `070`) ; **B5 y ajoutera la partie « séances »** (fichier `075`) sans créer de second crate.

**Pourquoi** : le principe II énonce l'équation *un module = un schéma PostgreSQL = un crate*, et le registre `platform.modules` porte une seule entrée `programme`, de schéma `programme`. Deux crates pour un schéma casseraient l'équation et rendraient ambigu le montage conditionnel — quel crate un `disabled` éteindrait-il ?

**Alternatives écartées** : *deux crates `proposal` et `session`* — deux modules pour un schéma, et surtout deux crates qui devraient se lire l'un l'autre (une séance naît d'une proposition acceptée), c'est-à-dire l'arête interdite. *Un crate `proposal` maintenant, renommé en B5* — un renommage de crate en cours de jalon coûte plus que la retenue de le nommer juste tout de suite.

**Conséquence pour B5** : `lib.rs` expose déjà `routes()`, et B5 y ajoutera ses configurateurs. Les dossiers `domain/`, `repo/`, `service/` et `routes/` sont donc nommés par **agrégat** et non par « proposition », pour que B5 y ajoute les siens sans réorganiser.

---

## R2 — Le service n'émet aucun événement de changement d'état, parce que la base les émet déjà

**Décision** : les huit changements d'état d'un dossier sont annoncés par `tg_guard_proposal_status()`, qui appelle `platform.emit_event()` dans la transaction. **Le service n'en émet aucun.** Il émet trois événements que la base n'émet pas :

| Événement | Quand | Pourquoi il existe |
|---|---|---|
| `programme.coorganization.requested` | une organisation est ajoutée au dossier avec un rôle autre que porteur | une co-organisation annoncée **engage un tiers** : le front dit « sera invitée à confirmer sa participation ». B6 enverra la demande |
| `programme.comment.shared` | un message est écrit en visibilité partagée avec le déposant | c'est le seul écrit du comité qui sorte du comité. Sans annonce, l'organisation découvre une demande de correction en revenant sur son espace |
| `programme.review.assigned` | un dossier est confié à un membre du comité, avec son échéance | c'est ce qui déclenche le rappel de revue de B6, et l'action groupée en confie douze d'un coup |

**Pourquoi** : c'est **l'inverse de B3**, où aucun déclencheur du module n'émettait rien (écart n° 87). Le piège de B1 (`anonymize_person()`) et de B2 (`merge_organizations()`) est de retour. Émettre à son tour produirait **deux** événements par transition, donc deux courriels et deux notifications, et le doublon ne se verrait qu'en production.

**Alternative écartée** : *émettre depuis le service et ignorer celui du déclencheur côté consommateur* — la garde de rejeu du principe IV protège d'un même événement rejoué, pas de deux événements distincts décrivant le même fait.

**Ce qui n'émet rien, et pourquoi** : l'enregistrement d'un brouillon (aucun autre module n'en dépend), la notation et la note personnelle (internes au comité), l'accusé de lecture (compteur), la résolution d'une demande (l'état visible est le compteur, relu à chaque affichage), la modification d'un dossier retenu (**surtout pas** — c'est précisément ce qui ne doit rien propager vers la séance).

---

## R3 — La fiche d'évaluation se compose en une transaction, et cette transaction **écrit**

**Décision** : la composition des onze lectures passe par **une seule connexion**, dans une transaction ouverte par la porte d'écriture du noyau — parce qu'elle pose l'accusé de lecture. L'état « déjà ouvert ou non » est lu **avant** l'appel qui le pose.

**Pourquoi** : B3 avait choisi une transaction en **lecture seule** pour composer ses six onglets, et c'était juste — il ne s'y écrivait rien. Ici, `record_proposal_read()` écrit, et le principe VII exige que toute écriture pose l'acteur et l'identifiant de requête. Une lecture qui écrit est assumée par le modèle lui-même ; on ne la déguise pas en deux appels dont l'un serait hors contexte.

**Alternative écartée** : *composer en lecture seule, puis poser l'accusé dans une seconde transaction* — deux allers-retours, et un accusé qui peut manquer alors que la page s'est affichée. La leçon de B2 vaut ici : une transaction perdante retenait deux connexions du pool ; une connexion, une transaction.

---

## R4 — Le voile de l'aveugle **n'est pas un filtre** : ce qui est masqué n'est pas lu

**Décision** : quand le voile est baissé, la requête qui lit les revues des pairs **n'est pas exécutée**. Le décompte, lui, l'est — une requête d'agrégat qui ne rend aucun texte.

**Pourquoi** : le contrat du front l'écrit en toutes lettres — « ce qui n'est pas envoyé ne peut pas fuiter ». Lire puis filtrer laisse la donnée à portée d'un champ oublié dans un type de sortie, d'une trace de débogage, d'un message d'erreur enrichi. Ne pas lire supprime la classe entière de défauts.

**Alternative écartée** : *lire et vider les champs sensibles* — c'est le patron qui a produit, en v1, des notes internes visibles dans une réponse JSON que l'écran n'affichait pas.

**La règle exacte, telle que le contrat la pose** : le voile est baissé quand l'appel est en aveugle **et** que la personne est affectée **et** que sa revue n'est pas déposée. Un administrateur qui décide sans noter n'est pas concerné : l'ancrage vise celui qui va poser une note, et masquer les notes à qui doit trancher rendrait la décision impossible.

---

## R5 — L'adresse d'URL du dossier est dérivée en base, avec repli, et suffixée sur collision

**Décision** : `platform.slugify(titre)` avec repli `'dossier'` quand elle rend une valeur vide ; sur violation de `ux_proposals_slug`, réessai avec un suffixe numérique croissant, au plus dix fois. Le slug **suit le titre tant que le dossier est en brouillon**, et se fige au dépôt.

**Pourquoi** : la colonne est obligatoire, unique par édition, et le contrat du formulaire ne la porte pas (écart n° 95) — le client ne peut pas la calculer, il ignore les autres dossiers de l'édition. Le repli n'est pas un ornement : le premier enregistrement a lieu **à la première frappe**, quand le titre est encore vide et que la fonction rend NULL. Le figer au dépôt évite qu'une adresse déjà communiquée change sous une correction de titre.

**Pourquoi en base** : c'est la même fonction que le reste de la plateforme emploie, elle enlève les accents et normalise selon les règles de PostgreSQL. La réécrire en Rust produirait deux normalisations divergentes du même texte.

**Alternatives écartées** : *demander le slug au client* — il n'a pas de quoi le calculer, et le front ne l'enverra jamais. *Refuser tant que le titre est vide* — c'est refuser le premier enregistrement automatique, donc la fonctionnalité. *Compter les homonymes avant d'insérer* — la course entre deux dépôts simultanés ferait échouer le second de toute façon ; le réessai sur collision est le patron déjà employé par le noyau pour les empreintes de jeton (`is_token_hash_collision`).

---

## R6 — L'heure murale se convertit en base, dans le fuseau de l'édition, dans les deux sens

**Décision** : à l'écriture, `(date || ' ' || heure)::timestamp AT TIME ZONE (fuseau de l'édition)` ; à la lecture pour le formulaire, l'opération inverse. Aucune arithmétique de fuseau en Rust.

**Pourquoi** : c'est la décision R5 de B3, reprise pour la même raison — deux bases de fuseaux dans une même chaîne divergent, et c'est exactement le défaut qui a fait tomber le formulaire du front sur `Europe/Geneva`. Le coût d'une erreur est ici parfaitement concret : un créneau saisi à 14:30 à Belém se rouvre à 11:30 pour qui corrige depuis Dakar, **sans qu'aucune erreur ne soit levée**.

**Où le fuseau est lu** : sur l'édition du dossier, jamais sur l'appel ni sur la requête. Une lecture hors schéma, réunie avec les autres dans `repo/cross.rs`.

---

## R7 — Les transitions offertes tiennent en une requête

**Décision** : une seule requête joint la table des règles sur l'état courant du dossier, et évalue pour chaque ligne, en jointure latérale, la permission requise avec `identity.has_permission(personne, permission, 'event', édition)` et la qualité de porteur. Elle rend l'état cible et l'obligation de motif.

**Pourquoi** : quatorze règles, dont au plus quatre applicables à un état donné ; les évaluer une par une ferait autant d'allers-retours pour composer un menu. Et le croisement doit se faire **au même instant** que la lecture de l'état : deux requêtes séparées offriraient une transition depuis un état déjà changé.

**La portée est celle de l'édition du dossier**, pas la portée globale : c'est le principe V, et c'est ce qui fait qu'un responsable détaché sur un webinaire ne décide pas sur la COP31.

**Alternative écartée** : *charger la table entière et l'évaluer en Rust* — c'est réimplémenter la sélection, et surtout appeler la fonction d'autorisation autant de fois qu'il y a de règles.

---

## R8 — Deux refus du même code d'erreur PostgreSQL, et comment on les distingue

**Le fait** : le garde de la machine à états lève `restrict_violation` (23001) pour une transition non déclarée, et `not_null_violation` (23502) pour un motif manquant. Mais le contrôle de recevabilité lève **aussi** 23001, pour trois causes distinctes — appel clos, plafond atteint, organisation non vérifiée.

**Décision** : les deux gardes sont distingués **par le moment**, pas par le texte.

- Un motif manquant sort en 23502 : dans une transaction qui n'écrit que l'état et le motif — deux colonnes nullables —, **aucune autre violation de non-nullité n'est possible**. La traduction est sûre.
- Les trois refus de recevabilité sont **classés avant l'écriture** (R9), de sorte qu'un 23001 qui remonte malgré tout est nécessairement une transition non déclarée, ou une course — traduit comme tel, avec le message français du déclencheur repris **mot pour mot** par l'outil du noyau prévu pour cela.

**Alternative écartée** : *reconnaître la cause au texte du message* — trois messages français, dont deux portent des valeurs interpolées, et qui changent à la première reformulation du SQL. C'est la dépendance la plus fragile qu'on puisse écrire.

---

## R9 — La recevabilité est classée avant l'écriture, et c'est une entorse assumée

**Décision** : avant de tenter le passage à « déposé », le service lit dans la même transaction l'état de l'appel, sa fenêtre effective, le décompte des dossiers de l'organisation et le drapeau d'organisation vérifiée, et compose la réponse nommée que le contrat exige. Le déclencheur reste le dernier mot.

**Pourquoi c'est nécessaire** : le contrat du front n'attend pas une erreur mais **deux réponses portant des valeurs** — l'échéance pour un appel clos, le plafond pour un quota atteint. Le déclencheur ne les rend que dans une phrase française. Les extraire d'un message est la dépendance écartée en R8.

**Pourquoi c'est une entorse au principe VIII** : le service évalue une condition que la base évalue déjà. Elle est **bornée** — trois conditions, lues et non recalculées, dans la même transaction que l'écriture qu'elles précèdent — et le déclencheur n'est ni désactivé ni contourné : une course entre la lecture et l'écriture retombe sur lui, et son refus sort tel quel.

**Alternative écartée** : *laisser passer et traduire l'exception* — on ne saurait pas laquelle des trois causes s'applique, ni avec quelle valeur ; l'écran afficherait « le dépôt a été refusé » sans dire quand l'appel a fermé, ce qui est précisément ce dont l'organisation a besoin.

**Le renvoi d'un dossier corrigé n'en est pas concerné** : la fenêtre ne s'y applique pas (le déclencheur ne la vérifie que sur un premier dépôt), le plafond si.

---

## R10 — La consolidation des notes est appelée par le service, faute de déclencheur

**Décision** : `programme.refresh_proposal_score(dossier)` est appelée dans la **même transaction** que le dépôt d'une revue, juste après l'écriture des notes par critère, et la réponse rend les agrégats **relus** — pas ceux calculés en Rust.

**Pourquoi** : aucun déclencheur ne l'appelle (écart n° 98). Ce n'est pas une réimplémentation d'invariant : la base ne tient pas celui-là, elle fournit seulement la fonction qui le rétablit. Sans appel explicite, la note du dossier, sa moyenne, son nombre de revues et son élimination restent aux valeurs de la ligne, et **le classement du comité est faux sans qu'aucune erreur ne le signale**.

**Alternative écartée** : *ajouter un déclencheur sur les notes* — ce serait modifier le modèle, ce que le prompt interdit sans justification ; et la fonction agrège toutes les revues d'un dossier, la déclencher par ligne de note la ferait tourner une fois par critère.

**Le brouillon d'une revue ne la déclenche pas** : la fonction ne retient que les revues déposées, l'appeler pour un brouillon serait un calcul sans effet.

---

## R11 — Les thématiques : la seule écriture hors schéma qui n'a pas d'autre porte

**Décision** : le service écrit dans `reference.entity_terms` avec le triplet `('programme', 'proposals', identifiant)` **posé littéralement**, jamais reçu du client. Tout est réuni dans `repo/themes.rs`. Les codes reçus sont vérifiés comme appartenant à la taxonomie attendue avant écriture. L'effacement d'un dossier purge ses liens.

**Pourquoi c'est admis** : la table est polymorphe et sans clé étrangère vers les propositions ; aucun autre module ne peut poser les thématiques d'un dossier. C'est exactement la dérogation que B3 s'est accordée pour les fils de programmation, bornée de la même façon — un seul fichier, où un ajout se discute.

**Pourquoi la purge est à nous** : le commentaire du modèle annonce une fonction de nettoyage qui **n'existe pas** (écart n° 94). Sans cette purge, un dossier effacé laisse ses liens derrière lui.

**Alternative écartée** : *accepter le triplet dans la charge utile* — un client pourrait alors rattacher des thématiques à n'importe quelle entité de n'importe quel schéma. C'est l'écart n° 3 dans son intégralité, et il n'a pas d'autre remède que de ne jamais lire ces trois champs.

---

## R12 — La création d'une personne inconnue suit un précédent, pas une invention

**Décision** : le service crée la personne quand l'adresse d'un intervenant est inconnue, par une insertion directe et bornée — adresse, prénom, nom saisis par le déposant, statut actif ; **jamais de compte, jamais de rôle, jamais d'adresse secondaire**. Réunie avec les autres écritures hors schéma dans `repo/people.rs`.

**Pourquoi c'est la bonne voie** : le précédent existe et il est livré — le module Organisations crée déjà la personne visée par une invitation dont l'adresse est inconnue. Le refuser ici obligerait à passer par un contrat d'événement, donc à **rendre l'intervenant absent de la réponse d'enregistrement** : le formulaire ne pourrait ni l'afficher, ni le rattacher, ni détecter le doublon au clavier suivant. Le contrat exige une réponse synchrone.

**La différence avec le précédent, et elle compte** : l'invitation ne connaît que l'adresse et pose donc un libellé neutre. Ici, le déposant a saisi le prénom et le nom — on les écrit, et **on ne déduit rien de l'adresse** (FR-026). Un « a.diallo » extrait d'un courriel est un nom que plus personne ne corrigera.

**Alternative écartée** : *un contrat d'événement consommé par le module Identité* — création différée, réponse sans identifiant, doublon impossible à détecter au moment où le déposant est encore devant son écran.

---

## R13 — Le périmètre remonte du dossier à son édition avant de refuser

**Décision** : toute route paramétrée par un identifiant de dossier, de commentaire ou de revue **résout d'abord son édition**, puis vérifie le périmètre. Un dossier hors périmètre est refusé **comme un dossier inexistant** : même code, même forme.

**Pourquoi** : c'est la décision R2 de B3, et le principe V l'exige — « y compris en forgeant une URL ». La nuance de B4 est qu'il y a **trois niveaux** : un commentaire appartient à un dossier, qui appartient à une édition. La résolution d'ascendance est donc une requête, réunie avec les autres dans `repo/cross.rs`.

**Ce que la résolution ne divulgue jamais** : elle ne rend rien à l'appelant avant que le contrôle ne soit passé. Un identifiant hors périmètre ne doit pas se distinguer d'un inexistant par le temps de réponse non plus — mais la différence, ici, est une lecture d'index, pas une opération coûteuse.

**Le côté organisation ne passe pas par là** : l'espace organisation est borné par l'**adhésion active**, pas par le périmètre d'administration. Une organisation n'administre rien.

---

## R14 — L'assainissement du HTML : une dépendance nouvelle, et laquelle

**Décision** : `ammonia`, bibliothèque d'assainissement HTML fondée sur `html5ever`. Liste blanche alignée **exactement** sur la barre d'outils de l'éditeur du front : gras, italique, `h3`, `h4`, listes, citation, lien, séparateur, paragraphe, saut de ligne. Aucun attribut hormis `href` sur un lien, restreint aux schémas `http` et `https`. Assainissement **à l'écriture**.

**Pourquoi une bibliothèque** : le filtrage de HTML écrit à la main est le cas d'école du contrôle qu'on croit avoir. Un analyseur conforme à la spécification HTML est le seul moyen de refuser ce qu'un navigateur accepterait — attributs d'événement, `javascript:` encodé, balises mal fermées qui rouvrent un contexte.

**Pourquoi à l'écriture** : c'est l'écart n° 32, et sa raison est écrite — un contenu stocké propre se rend partout ; un contenu filtré à l'affichage doit l'être dans chaque écran, chaque courriel et chaque export, et le premier oubli est une injection.

**Dépendance nouvelle, donc décision consignée** dans `docs/progression/decisions/2026-08-20.md`, comme la constitution l'exige. Elle n'est ni un framework, ni un runtime, ni une couche d'accès : une fonction pure sur une chaîne.

**Alternative écartée** : *échapper tout le HTML et stocker du texte* — la colonne est documentée comme un fragment de HTML restreint et l'éditeur du front en produit ; échapper détruirait la mise en forme de tous les dossiers.

---

## R15 — Les longueurs maximales vivent dans le domaine, pas dans la configuration

**Décision** : les huit bornes de l'écart n° 28 sont des constantes du domaine du module, à un seul endroit, alignées sur celles du front.

**Pourquoi pas la configuration** : le contraste est utile. Les seuils de verrouillage de B1 sont allés en configuration parce que ce sont des réglages **d'exploitation** — on les ajuste sans redéployer. Une longueur maximale de résumé est une règle de **contrat** : la changer change ce que le front affiche, ce que la carte de programmation peut rendre, et ce que l'export produit. La rendre modifiable par variable d'environnement, c'est permettre à deux déploiements de refuser des dossiers différents.

**Alternative écartée** : *les poser en base* — la base n'a pas à trancher ce qu'est un résumé lisible, et c'est écrit dans l'écart n° 28.

---

## R16 — Les facettes se comptent sur les lignes déjà lues

**Décision** : la liste du back-office lit ses lignes **une fois**, puis compte ses sept facettes en mémoire. Aucune seconde requête d'agrégat.

**Pourquoi** : le contrat du front l'exige et en donne la raison — « demandées à part, elles seraient mesurées à un autre instant, et le "Retenu (17)" du filtre finirait par ne plus correspondre aux lignes affichées ». C'est aussi ce que B3 a fait pour les facettes de sa liste d'éditions.

**Le décompte des dossiers non lus est la seule exception** : il vient de la fonction du modèle, qui prend le lecteur en paramètre. Ce n'est pas une facette mais une relation entre un dossier et une personne.

---

## R17 — Ni pagination, ni tri serveur, et c'est le contrat qui le dit

**Décision** : la liste rend toutes les lignes de l'édition. Aucun paramètre de tri, de page ni de filtre.

**Pourquoi** : le contrat du front est explicite — le filtrage et le tri restent côté écran tant que les données tiennent en mémoire, et « ces paramètres deviendront ceux de la requête » **au raccordement (B7)**, ce qui est le prompt suivant. Les livrer maintenant produirait une surface que personne n'appelle, et deux implémentations du tri à réconcilier.

**Le plafond est annoncé, pas ignoré** : la volumétrie de référence est de quelques dizaines de dossiers par édition (quarante dans le jeu de données). Au-delà de quelques centaines, la réponse deviendrait lourde ; c'est le moment où B7 posera la pagination, et c'est écrit dans le quickstart.

---

## R18 — Deux préfixes de route sont déjà pris, et l'un doit être refactorisé

**Le fait** : le contrat du front place deux routes de ce module sous `/organizations` (l'espace et ses éditions) et une sous `/people` (la recherche d'un intervenant par son adresse). Or `/people` est **déjà composé une seule fois dans `api`** — patron posé en B1 —, et `/organizations` est un `web::scope` **unique appartenant au module Organisations**.

**Décision** : `/people` reçoit un troisième contributeur par le patron existant. **`/organizations` est refactorisé sur le même patron** : le module Organisations expose ses routes de ce préfixe séparément, et `api` compose le scope une seule fois.

**Pourquoi ce n'est pas optionnel** : deux `web::scope` du même préfixe **ne se complètent pas** — Actix retient le premier et rend 404 sur les routes du second, sans essayer. Le défaut s'est déjà produit sur `/organizations` en B2, il a coûté trois routes muettes sur vingt et une, et le commentaire qui le raconte est dans le code d'`api`. Le reproduire alors qu'il est documenté à l'endroit exact où l'on écrit serait difficile à défendre.

**Ce que cela touche** : `backend/crates/modules/org/src/lib.rs` (une fonction exposée, aucune route déplacée) et `backend/crates/api/src/lib.rs` (la composition). **Du code livré est modifié** — à annoncer en tête du découpage, comme B2 l'avait fait pour la remontée du service de jetons.

**Alternative écartée** : *placer l'espace organisation sous `/proposals`* — le contrat du front ne se renégocie pas, et c'est une règle de la constitution, pas une préférence.

---

## R19 — Le chemin des transitions offertes est déjà pris par le journal

**Le fait** : le prompt propose `GET /proposals/:id/transitions` pour les transitions offertes. **Ce chemin est déjà celui du journal** dans le contrat du front (`history:` de la fabrique des propositions), qui rend les lignes de `proposal_transitions`.

**Décision** : le journal garde son chemin. Les transitions offertes sont exposées **deux fois, et c'est voulu** : comme champ `available_transitions` de la fiche d'évaluation — où l'en-tête en a besoin sans requête de plus — et comme `GET /proposals/{id}/available-transitions`, pour l'espace organisation et pour rafraîchir un menu après une décision.

**Pourquoi pas seulement le champ** : la liste du back-office change l'état de douze dossiers d'un coup, et l'écran doit recalculer ce qui reste offert sans recharger onze tables.

**Écart n° 101**, consigné : le prompt proposait un chemin que le contrat du front occupait déjà, et il offrait lui-même l'alternative retenue.

---

## R20 — Aucun travail différé, et la déduction v1 est une opération synchrone

**Décision** : ce module ne déclare **aucun travail récurrent**. La déduction des transitions des dossiers repris de la v1 est une **opération d'administration synchrone**, exigeant la portée globale, rejouable, qui rend le nombre de dossiers traités et de lignes semées.

**Pourquoi aucun travail** : rien dans ce module n'a d'effet à échéance. Les rappels de revue et les avis de dépôt appartiennent au module Engagement (B6) et se déclencheront sur les événements de R2 ; la clôture automatique d'un appel échu appartient à B3 et y est livrée. Déclarer un travail « pour la forme » créerait une cadence à configurer et à surveiller, sans effet.

**Pourquoi la déduction est synchrone** : elle est ponctuelle — une fois, au moment de la bascule — et son résultat doit être **lu** par celui qui la lance. Un travail différé rendrait un identifiant de tâche là où l'on attend « 3 812 dossiers, 11 436 lignes semées ».

**Pourquoi elle est ici et pas dans le fichier de reprise** : la fonction de reprise des activités est une coquille vide dont le corps est annoncé dans un fichier absent du dépôt (écart n° 100). L'écrire là serait l'écrire nulle part.

**Rejouable sans doublon** : elle ne sème que sur les dossiers dont le journal est **vide**, condition vérifiée dans la même requête que l'insertion.

---

## R21 — Noter exige une affectation ; lire n'en exige pas

**Décision** : l'enregistrement d'une revue exige la permission de noter **et** une affectation non déportée sur ce dossier. La lecture de la fiche exige seulement la permission de lecture générale et le périmètre.

**Pourquoi** : le contrat du front le pose et l'explique — la permission et l'affectation sont **décorrélées**, « un membre du comité peut lire un dossier qu'on ne lui a pas confié, sans le noter ». Le modèle ne l'impose pas : rien n'empêche une revue sans affectation. Mais une revue posée hors affectation compterait dans les agrégats sans figurer dans l'avancement du comité — le « 2/3 » deviendrait faux dans l'autre sens.

**Alternative écartée** : *laisser noter quiconque a la permission* — c'est perdre le sens de la répartition de la charge, qui est la raison d'être de la table des affectations.

---

## R22 — Le semis de test, et ce que chaque test doit créer lui-même

**Décision** : aucun dossier n'est semé par le modèle (`900_seed.sql` ne pose rien dans `programme` hormis un drapeau de fonctionnalité). Chaque test crée son édition, son appel, sa grille, son organisation et ses dossiers, sur la base réelle et jetable du harnais de B1.

**Ce que cela impose** : une fabrique de test partagée par le module — édition avec fuseau, appel ouvert avec sa grille par défaut semée par la fonction du modèle, organisation vérifiée, personne membre. Sans elle, chaque test d'écriture recommencerait quarante lignes de préparation.

**Le harnais existe** : `kernel::testing`, employé par B1, B2 et B3. On ne le réinvente pas.

---

## R23 — Valkey reste inutilisé, et rien ici ne le réclame

**Décision** : aucun cache. Les lectures de ce module sont des écrans d'administration ouverts à la main et un formulaire de dépôt ; la seule lecture répétée est la liste d'une édition, qui tient en une requête sur une vue.

**Pourquoi le dire** : B1, B2 et B3 ont pris la même décision, et la répéter évite qu'un quatrième module l'introduise « parce que c'est dans la pile ». Le premier cache de la plateforme devra se justifier par une mesure, pas par une disponibilité.

---

## R24 — Les notes traversent la frontière SQL en nombre à virgule flottante, et l'autorité reste en base

**Décision** : les colonnes `numeric` — notes par critère, notes pondérées, moyennes, poids et barèmes — sont traversées en `float8` à la frontière SQL, dans les deux sens, exactement comme B3 le fait déjà pour la grille (`max_score::float8`, `$5::float8::numeric(5,2)`).

**Pourquoi** : le workspace ne déclare aucune caractéristique décimale pour SQLx (`bigdecimal`, `rust_decimal`) et ne l'a pas fait en trois modules. Ajouter une dépendance décimale pour ce module ferait diverger la traversée d'un même type selon le crate qui le lit.

**Pourquoi c'est sans risque ici** : le service ne **calcule** aucune moyenne. Les agrégats sont produits par la fonction de consolidation, **en `numeric`, dans la base** (R10), et relus. Le flottant ne sert qu'à transporter des valeurs à deux décimales, que la base arrondit avant de les rendre. Le seul calcul en direct — le total pondéré affiché pendant la saisie — appartient déjà au front, qui le refait en JavaScript.

**Ce qu'il ne faut pas en conclure** : qu'on peut comparer deux notes en flottant pour décider d'un classement. Le rang vient de la vue, qui l'ordonne en `numeric`.

**Les trois autres traversées, héritées et non réinventées** : une énumération PostgreSQL passe en `text` (`status::text`, `$1::text::programme.proposal_status`), un `platform.i18n_text` passe en `jsonb`, et un domaine comme `platform.email` ou `platform.slug` se reconstitue par un double transtypage depuis `text` — ce qui fait porter le refus par le domaine, avec son nom exploitable par l'outil du noyau prévu pour cela.
