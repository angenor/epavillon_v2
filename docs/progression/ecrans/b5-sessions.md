# B5 — Sessions

> Extrait de la [progression](../../PROGRESSION.md). Le prompt de ce module est dans [PROMPTS_DEVELOPPEMENT.md](../../PROMPTS_DEVELOPPEMENT.md) § PHASE B — B5.

**État** : ✅ **LIVRÉ** le 21/08 — **les 156 tâches**, `make check` au vert **en entier** depuis la racine : base détruite et rechargée de zéro, seize schémas, rapport de frontières **vide**, `clippy -D warnings` sans un avertissement, **627 tests** dans le workspace. Le crate `programme` passe de 17 372 à **27 318 lignes**, et **aucun fichier de `backend/` ne dépasse mille lignes** — c'était le module où la marge était la plus mince. **Le SQL n'a pas bougé d'une ligne.** Détail au § « Ce qui a été livré » ci-dessous.

Avant cela, le même jour : 🧩 **DÉCOUPÉ** — **156 tâches en 12 phases, dont 67 de test**, une phase par histoire et **quatre jalons de livraison**. **Cinq avertissements sont écrits en tête du découpage**, chacun parce qu'il coûterait cher découvert en chemin. **Le jalon que le prompt demande hors inscriptions est à T001–T112** : à ce point, le programme de la COP31 est public. **Le découpage a fait apparaître une dépendance que le plan n'avait pas nommée** : la programmation publique doit être livrée **avant** la publication, faute de quoi celle-ci se prouverait sans qu'on puisse en constater l'effet. Prochain pas : `/speckit-implement`.

Avant cela, le même jour : 📐 **PLANIFIÉ** — [`specs/005-sessions/`](../../../specs/005-sessions/plan.md) : **27 décisions techniques, 17 routes, 8 codes d'erreur, 0 événement émis, 1 événement consommé — le premier du dépôt —, 0 travail différé, aucune modification du SQL.** Contrôle constitutionnel passé **deux fois**, **une entorse** et **une écriture hors schéma nouvelle**, toutes deux justifiées. **Deux écarts de plus, n° 124 et n° 125, que seul le plan pouvait révéler** : le contrôle de jauge de la base **ne tient pas sous concurrence**, et le déclencheur d'inscription **revalide à chaque changement d'état**, ce qui interdit d'annuler son inscription à une séance annulée. Prochain pas : `/speckit-tasks`. *(fait le même jour)*

Avant cela, le même jour : 📝 **SPÉCIFIÉ** — [`specs/005-sessions/`](../../../specs/005-sessions/spec.md). **9 histoires utilisateur, 14 cas limites, 114 exigences fonctionnelles, 29 critères de réussite, et AUCUNE modification du SQL.** **Treize écarts nouveaux, n° 111 à 123**, dont trois qui produiraient un défaut silencieux dès le premier usage réel — et **un quatrième qui corrige la consigne du prompt elle-même**. Prochain pas : `/speckit-plan`. *(fait le même jour)*

---

## Ce qui a été livré (21/08)

**Les quatre jalons du découpage sont atteints.** Le module transforme une décision en programme : la séance naît de l'acceptation, l'équipe l'arbitre, la publication la rend publique, et le public s'y inscrit.

### Les trois écarts ouverts sont refermés

**L'écart n° 57** — le planificateur n'avait rien à placer depuis le 18/08. Retenir un dossier fait désormais naître ses séances **dans la transaction de la décision**, sur **un seul hameçon** : les deux chemins d'acceptation — la décision individuelle et l'action groupée — passent tous deux par `service/transition.rs::tenter`, et la reprise v1 n'écrit pas l'état, donc ne programme rien. Une acceptation rejouée ne double aucune séance : l'idempotence tombe de `ux_sessions_proposal_sequence`, jamais d'un décompte préalable.

**La programmation publique** cesse d'être vide. C'est l'autre moitié d'un geste partagé entre deux schémas : B3 contrôle, estampille l'édition et **annonce** ; B5 reçoit et rend publiques les séances désignées. **Le module est le premier consommateur d'outbox du dépôt** — la machinerie du noyau n'avait jamais servi qu'à la télémétrie —, et la garde de rejeu vient du noyau : aucun code d'idempotence n'a été écrit ici.

**L'écart n° 108** — l'espace organisation servait une liste vide. Chaque séance y porte désormais sa salle et **trois nombres** ; `reminders` reste **vide, jamais absente**, jusqu'à B6.

### Ce que le service corrige, et que rien d'autre n'aurait vu

| Écart | Ce qui a été fait | Comment il est éprouvé |
|---|---|---|
| **n° 113** — la journée de rattachement ne suit pas une séance déplacée | Elle est **remise à nul** à chaque écriture de créneau sans journée fournie, pour que la base la redéduise | Une séance déplacée du 12 au 14 novembre, et `event_days.day_date` **relue en base** |
| **n° 114** — le déclencheur ne valide rien sans formulaire **attaché** | La validation porte sur le formulaire **résolu** — séance, à défaut édition, à défaut plateforme | Une séance sans formulaire attaché, une inscription **sans le pays** : refusée en nommant le champ |
| **n° 124** — le contrôle de jauge ne tient pas sous concurrence | Toute écriture d'inscription prend la ligne de la séance **en verrou** | **Mesuré, pas supposé** : cent inscriptions concurrentes sur dix places rendent **onze** confirmées verrou retiré, **dix** verrou posé — et aucun rang d'attente en double |
| **n° 115** — deux fenêtres que la base ignore | Les **quatre** fenêtres sont décidées par le service, chacune avec son motif | Quatre motifs distincts, sur des valeurs relues sous verrou |
| **n° 116** — la promotion ne vérifie pas la jauge | Le service promeut **exactement le nombre de places libérées** : une annulation confirmée, une place, une promotion | Annuler une inscription **en attente** ne promeut personne |
| **n° 7 / n° 111** — le canal de diffusion | **La consigne a été corrigée, pas suivie** : le canal **reste saisissable**, le déclencheur ne le posant que lorsqu'il est nul. Ce qui est refusé, c'est un canal désigné **quand la diffusion est retirée** — le seul endroit où la base efface un choix en silence | Un canal choisi est relu en base et n'est **pas** remplacé par celui par défaut |
| **n° 117** — les deux déclencheurs émettent déjà | **Le service n'émet aucun événement** : `grep -rn 'events::emit'` ne rend rien dans les fichiers de B5 | **Une** ligne d'outbox par inscription ; **trois** après une acceptation à trois séances, jamais six |
| **n° 125** — le déclencheur revalide à chaque changement d'état | **Consigné, non contourné** : le refus est traduit en `REGISTRATION_LOCKED`, jamais laissé sortir en 500 | Annuler son inscription à une séance annulée rend un code nommé |

### Ce que les tests éprouvent, et qui ne se déduit d'aucun autre

- **Un test frappe les dix-sept routes sur la vraie application**, intergiciels compris : c'est le seul contrôle qui voit une route écrite mais non montée, et il a déjà attrapé ce défaut deux fois. **Les deux routes de B3 sous le préfixe partagé répondent toujours** — le scope est composé à partir de deux modules.
- **Un test de bout en bout reproduit le symptôme « publié mais rien de public »**, worker arrêté, puis le fait disparaître : c'est l'obligation inscrite aux points bloqués le 20/08. L'écart entre le nombre annoncé et l'effet est **mesuré**, jamais supposé nul.
- **Un balayage de la charge utile entière** cherche, en texte, le nom d'un inscrit, son adresse et une réponse au formulaire dans l'espace organisation : aucun ne s'y trouve.
- **Le chevauchement est éprouvé dans les deux sens** : deux séances superposées en salle physique s'écrivent et remontent un conflit bloquant ; une salle **virtuelle** n'en produit aucun, et une séance **sans salle** n'occupe rien.

### Les chiffres

| | |
|---|---|
| Tâches | **156**, toutes livrées |
| Routes | **17**, toutes montées et documentées |
| Codes d'erreur ajoutés | **8** |
| Événements émis par le service | **0** — les déclencheurs émettent quatorze types |
| Événements consommés | **1**, le premier du dépôt |
| Travaux différés | **0** |
| Écritures hors schéma | **3 fichiers**, et pas un de plus |
| Fichiers de `docs/database/` modifiés | **0** |
| Dépendance nouvelle | **1** — `regex`, déjà transitive, consignée aux décisions |
| Tests du workspace | **627** |
| Plus gros fichier de `backend/` | **892 lignes** (`event/repo/cross.rs`, inchangé) |

---

## Ce qui a été spécifié

**Aucun crate n'est créé.** `backend/crates/modules/programme` existe depuis B4 et porte déjà tout le schéma `programme` : la décision est écrite en tête de son `lib.rs` et dans `specs/004-propositions/research.md` § R1 — un seul crate pour le schéma entier, ses dossiers internes nommés par agrégat justement pour recevoir ce jalon sans réorganisation.

**Ce module transforme une décision en programme.** B4 a livré les dossiers et leur cycle de vie ; à l'acceptation, il ne se passe aujourd'hui rien. Trois choses restent donc ouvertes et se referment ici :

- Le **planificateur** du back-office existe et n'a rien à placer (écart n° 57, ouvert depuis le 18/08).
- La **programmation publique** existe et reste vide : c'est ce module qui pose la visibilité de chaque séance, l'autre moitié de la publication livrée par B3.
- L'**espace organisation** annonce des séances et sert une liste vide (écart n° 108, assumé en B4).

**Les neuf histoires** : la naissance d'une séance à l'acceptation d'un dossier (P1) ; l'écran du planificateur en une réponse, conflits compris (P1) ; placer, déplacer, redimensionner et retirer par une seule écriture jamais refusée pour chevauchement (P1) ; la composition manuelle des journées spéciales (P2) ; la diffusion et la règle « un seul direct » (P2) ; la publication effective des séances, consommée depuis l'annonce de B3 (P1) ; la programmation publique lue sans session (P1) ; l'inscription avec formulaire configurable et validation dynamique (P2) ; et les trois nombres rendus à l'organisation, sans un seul nom (P2).

**Le périmètre dit aussi ce qu'il ne livre pas**, plutôt que de le laisser deviner : les questions du public, l'annulation et le report d'une séance, la création d'une séance sans dossier, l'écriture du compte rendu — et les rappels, qui appartiennent à B6 et dont la liste part **vide, jamais absente**.

---

## Ce qui a été planifié (21/08)

`plan.md`, `research.md` (**27 décisions**), `data-model.md`, `contracts/` (routes, erreurs, événements) et `quickstart.md`. **Aucune dépendance d'ampleur**, **aucun SQL composé dynamiquement**, **aucune modification du modèle**.

**Six décisions ont été prises dans le plan plutôt que laissées à l'implémentation**, parce qu'une décision tacite y aurait produit une faute **à l'exécution** :

1. **Le service n'émet AUCUN événement** — les **deux** déclencheurs du fichier `075` émettent déjà, quatorze types à eux deux. Vérifié dans le corps des fonctions. Émettre à son tour enverrait tout en double, visible seulement en production.
2. **La naissance des séances s'accroche à UN SEUL point.** Mesuré : la décision individuelle et l'action groupée passent **toutes deux** par `service/transition.rs::tenter`, et la reprise v1 n'écrit pas l'état — donc pas trois hameçons, un.
3. **La journée de rattachement se remet à nul quand le créneau change**, pour que la base la redéduise (écart n° 113).
4. **La validation des réponses porte sur le formulaire RÉSOLU**, pas sur celui qui est attaché (écart n° 114).
5. **Toute écriture d'inscription prend la ligne de la séance en verrou** (écart n° 124).
6. **La publication pose la date ET fait passer « pressenti » à « programmé »** — trois preuves concordantes le demandent : le modèle nomme l'état (« programmé et publié »), la feuille de style du front dit que `planned` est « programmée mais pas encore publique », et les données simulées font de même. Ne poser que la date laisserait un état du modèle **mort**, et le calendrier du back-office colorerait en état de travail des séances déjà publiques.

**Trois faits structurants du plan.**

**Le module est le premier consommateur d'outbox du dépôt.** La machinerie du noyau — `EventConsumer`, `ConsumerRegistry`, `claim()` — existe depuis B1 et n'avait jamais servi qu'à la télémétrie. Elle **porte la garde de rejeu** : le relais réserve le couple (consommateur, événement) **avant** d'appeler le consommateur, et le module n'a donc aucun code d'idempotence à écrire. Le noyau gagne cinq lignes, `register_all()`, par symétrie avec les travaux différés. Conséquence pratique à connaître : **sans worker, le bouton « Publier » estampille l'édition et rien ne devient public.**

**Une écriture hors schéma nouvelle, la troisième du crate** : `identity.consents`, pour la preuve du consentement à une réponse sensible. Le modèle prévoit exactement cet usage — sa colonne d'origine documente « formulaire d'inscription » —, et la preuve doit vivre dans la **transaction de la donnée qu'elle couvre** : un contrat d'événement l'écrirait après coup, et refuser l'inscription faute de consentement deviendrait impossible à garantir. Bornée à un fichier, comme les deux de B4.

**Une entorse au principe VIII, et une seule** : le service revérifie la **présence** des réponses obligatoires que le déclencheur vérifie déjà — parce que celui-ci ne vérifie **rien** quand la séance ne porte pas de formulaire attaché, et parce que sa phrase française ne donne pas le champ qu'un formulaire doit souligner.

---

## Écarts relevés en planifiant B5 (21/08)

| N° | Écart | Où | Ce qu'il coûte | Suite donnée |
|---|---|---|---|---|
| **124** | **LE CONTRÔLE DE JAUGE DE LA BASE NE TIENT PAS SOUS CONCURRENCE.** `tg_validate_registration()` exécute un `count(*)` **sans verrou** : sous `READ COMMITTED`, deux transactions simultanées lisent toutes deux neuf places prises sur dix et insèrent toutes deux. La position en liste d'attente souffre du même défaut — `max(...) + 1` sans verrou — et **aucun index unique ne porte sur cette colonne** | `075` § 4 | Onze inscrits sur dix places, et deux personnes au même rang d'attente. Le dépassement ne se verrait **que le jour de l'activité** ; le doublon de rang, jamais | **Toute écriture d'inscription prend d'abord la ligne de la séance en verrou** (R19). Les inscriptions à une même séance se sérialisent ; celles de séances différentes ne se gênent pas. Éprouvé par un test à **cent inscriptions concurrentes sur dix places** |
| **125** | **LE DÉCLENCHEUR D'INSCRIPTION REVALIDE À CHAQUE CHANGEMENT D'ÉTAT, ANNULATION COMPRISE.** Deux de ses contrôles ne sont pas bornés à l'insertion : le refus d'une séance annulée, et le contrôle des réponses obligatoires | `075` § 4 | Deux conséquences. **On ne peut pas annuler son inscription à une séance annulée** — sans conséquence pratique, une séance annulée ne réunissant personne. Et **une question rendue obligatoire après coup bloque l'annulation d'une inscription ancienne**, qui ne la porte pas — celle-là est réelle, et se réveillerait si l'IFDD ajoutait une question obligatoire en cours de campagne | **Consigné, non corrigé** : il n'y a pas de contournement sans modifier le déclencheur. Le service **traduit** les deux refus en réponse nommée (`REGISTRATION_LOCKED`) plutôt que de laisser sortir un 500 |

---

## Ce qui a été découpé (21/08)

`tasks.md` porte **156 tâches en 12 phases, dont 67 de test**, une phase par histoire utilisateur et **quatre jalons de livraison**.

**Cinq avertissements sont écrits en tête**, chacun parce qu'il coûterait cher découvert en chemin : les **deux** déclencheurs qui émettent déjà ; la journée de rattachement qui ne suit pas une séance déplacée ; le déclencheur d'inscription qui ne valide **rien** sans formulaire attaché ; le contrôle de jauge qui ne tient pas sous concurrence ; et **le canal de diffusion, sur lequel l'écart n° 7 se trompe** — le refuser casserait une fonctionnalité livrée du planificateur.

**Le découpage a fait apparaître une dépendance que le plan n'avait pas nommée** : **US7 doit précéder US6**. La programmation publique se démontre en posant la date de publication à la main, et devient alors l'**instrument de mesure** de la publication effective. L'ordre inverse obligerait à prouver que la publication a marché sans pouvoir en constater l'effet.

**Une seconde entorse à l'ordre des priorités, assumée et dite** : US4 et US5 (P2) sont traitées **avant** US7 et US6 (P1). Elles écrivent dans le même fichier de service que US3 et rendent la **même forme de réponse** ; les séparer rouvrirait deux fois le même fichier et donnerait deux occasions de diverger sur la composition des conflits — ce que le contrat du front interdit précisément.

**Quatre jalons** :

| Jalon | Tâches | Ce qui devient possible |
|---|---|---|
| **1** | T001–T078 | Retenir un dossier fait naître ses séances ; l'équipe compose la grille et voit ses conflits. **L'écart n° 57 se referme** |
| **2** | T001–T094 | L'écran d'arbitrage est complet — journées spéciales et diffusion comprises |
| **3** | T001–T112 | **Le programme de la COP31 est public.** C'est le jalon que le prompt demande hors inscriptions |
| **4** | T001–T156 | Le module est complet : inscriptions, décomptes, contrôles transverses |

**La question restée ouverte n'est pas tranchée par le découpage, et il le dit** : que devient la séance d'un dossier annulé après acceptation ? Aucune tâche ne l'implémente — l'écrire avant l'arbitrage reviendrait à trancher à la place du commanditaire. En attendant, **rien ne se propage**, et si l'option recommandée est retenue, **une seule fonction s'ajoute** dans `service/birth.rs`, appelée depuis le même hameçon que la naissance.

**Quatre contrôles mécaniques** ferment le découpage, et deux sont propres à ce module : les fichiers écrivant hors du schéma `programme` doivent être **exactement trois**, et `grep -rn 'events::emit'` doit ne **rien** rendre — un `grep` qui vaut un test, et qui se relit en une seconde.

---

## Écarts relevés en écrivant la spécification de B5 (21/08)

Numérotation à la suite de B4, qui s'arrêtait à 110. Le détail complet, avec la suite donnée à chacun, est dans [`specs/005-sessions/spec.md`](../../../specs/005-sessions/spec.md) § « Écarts relevés ». Ce qui suit en retient l'essentiel.

| N° | Écart | Gravité |
|---|---|---|
| **111** | **L'écart n° 7, recopié dans le prompt, est partiellement INEXACT — et le front a raison.** Le canal de diffusion **est** saisissable : le déclencheur ne le pose que lorsqu'il est nul, et n'écrase jamais un canal choisi. Le refuser à l'écriture, comme la consigne le demande à la lettre, aurait **cassé une fonctionnalité livrée** du planificateur — sur une édition à deux canaux, l'écran ne pourrait plus en désigner un. La consigne est traitée dans son **intention** : le contrat refuse l'intervalle dérivé et l'exclusivité de salle, accepte le canal, et refuse un canal désigné alors que la diffusion est **retirée** — là, et là seulement, la base efface en silence une valeur choisie | Corrige la consigne |
| **112** | **L'exclusivité de salle n'est pas toujours recalculée** : le déclencheur ne se déclenche pas sur cette colonne. Une écriture qui ne toucherait qu'elle passerait sans être corrigée — la valeur fausse **tiendrait**, au lieu d'être écrasée | Haute |
| **113** | **La journée de rattachement ne se recalcule pas quand on déplace une séance** : le déclencheur ne la déduit que lorsqu'elle est nulle. Une séance déplacée du 12 au 14 novembre **reste rattachée au 12**, et c'est le geste le plus fréquent de tout l'écran. Le calendrier et la programmation publique la rangeraient au mauvais jour, sans qu'aucune erreur ne le signale | Haute |
| **114** | **Le déclencheur d'inscription ne valide RIEN lorsque la séance ne porte pas de formulaire attaché**, alors que le formulaire applicable peut venir de l'édition ou de la plateforme. Une inscription sans aucune réponse obligatoire passerait, quand l'écran en aura posé quatre. La garantie de la base est un **plancher**, pas la règle | Haute |
| **115** | Le déclencheur d'inscription **ignore la date d'ouverture** des inscriptions et le fait qu'une séance n'en prenne pas : on pourrait s'inscrire trois mois avant l'ouverture annoncée | Moyenne |
| **116** | **La promotion depuis la liste d'attente ne vérifie pas la jauge** — le contrôle de capacité ne porte que sur l'insertion. Le dépassement ne se verrait que le jour de l'activité | Haute |
| **117** | **Les deux déclencheurs du fichier émettent déjà** : la séance à la création, à chaque changement d'état et à chaque report ; l'inscription à la création et à chaque changement d'état. C'est le piège de B1, B2 et B4 à l'identique — un service qui émettrait à son tour enverrait tout en double, visible seulement en production | Haute |
| **118** | **Un changement de salle n'émet rien** : une séance déplacée du stand vers une salle virtuelle ne prévient aucun inscrit. Consigné, non corrigé — le corriger demanderait de modifier le SQL, et B6 décidera s'il lui faut cet événement | Consigné |
| **119** | **Le rôle de programmation ne peut pas voir les inscrits** : il détient la permission de planifier, pas celle de gérer les inscriptions. Même famille que l'écart n° 56, qui pose la même question sur la publication | Consigné |
| **120** | **La séance perd le lien vers l'organisation de ses intervenants** : le dossier porte une organisation liée en plus du libellé figé, la séance ne porte que le libellé. Consigné — le libellé figé est ce dont l'archive a besoin | Consigné |
| **121** | **Deux chemins du front désignent la même lecture** : le contrat déclare un contrôle avant publication sous les séances, alors que B3 l'a livré sous le planificateur, et aucun écran n'appelle le premier. **Un seul est servi** ; le second est à retirer du front en B7 | Faible |
| **122** | **Aucun écran n'écrit le compte rendu d'une séance**, alors que l'espace organisation produit l'action « compte rendu manquant » et affiche son bandeau. L'action est servie, l'écriture n'appartient pas à ce jalon | Consigné |
| **123** | **Que devient la séance d'un dossier annulé après acceptation ?** Le seul chemin qui sort de l'état « retenu » est l'annulation, et rien ne dit ce qu'il advient de la séance, de ses inscrits et de sa place au programme. **Question au commanditaire**, inscrite aux points bloqués ; l'option tenue provisoirement annule la séance avec le même motif | Question |

**Deux écarts anciens sont traités par cette spécification.** L'écart n° 57 — rien ne crée la séance à l'acceptation d'un dossier — devient l'histoire n° 1 et ses quatorze exigences. L'écart n° 11 — la forme d'une réponse à un champ « pays » n'est fixée nulle part, et deux implémentations divergentes sont possibles — est **tranché** : le code ISO à deux lettres, comme les données simulées, validé contre le référentiel des pays. C'est une décision d'API, pas un arbitrage du commanditaire, et la fixer maintenant est ce qui empêche un export mêlant deux formes.

**Deux écarts se referment à la livraison, pas à la spécification** : l'écart n° 36 (le décompte des inscrits exposé séparément de la liste) et l'écart n° 108 (les deux blocs que B4 ne pouvait pas remplir) sont portés par les histoires n° 9 et n° 8.

---

## Ce qui a été vérifié le 21/08 en spécifiant, et comment

| Contrôle | Résultat |
|---|---|
| **Le fichier `075_programme_sessions.sql` a-t-il été lu en entier ?** | Oui, ses 1 137 lignes et ses neuf sections. Les treize écarts en sortent, et chacun cite la ligne ou la fonction qui le porte |
| **Les déclencheurs émettent-ils ?** | **Vérifié dans le corps des deux fonctions**, pas déduit du précédent de B4 : `tg_sessions_emit_events()` émet à la création, sur changement d'état et sur report ; `tg_registrations_emit_events()` émet à la création et sur changement d'état. Le service n'émettra donc **aucun** de ces événements (écart n° 117) |
| **Le déclencheur de dérivation écrase-t-il vraiment le canal ?** | **Non**, et c'est la lecture qui corrige la consigne : `IF NEW.is_streamed AND NEW.broadcast_channel_id IS NULL` — il **complète**, il n'écrase pas. Seule la branche `ELSIF NOT NEW.is_streamed` efface (écart n° 111) |
| **Sur quelles colonnes le déclencheur de dérivation se déclenche-t-il ?** | Salle, début, édition, diffusion, canal. **Ni l'exclusivité de salle, ni la journée de rattachement, ni la fin** — d'où les écarts n° 112 et n° 113 |
| **Le déclencheur de validation d'inscription couvre-t-il le formulaire applicable ?** | **Non** : `IF v_session.registration_form_id IS NOT NULL`. Une séance sans formulaire attaché n'est pas contrôlée (écart n° 114) |
| **La promotion depuis la liste d'attente contrôle-t-elle la jauge ?** | **Non** : le contrôle de capacité du déclencheur est borné à `TG_OP = 'INSERT'`, et une promotion est une mise à jour (écart n° 116) |
| **Le contrat de publication est-il celui des points bloqués ?** | Oui, repris mot pour mot depuis `contracts/src/event.rs` : garde de rejeu, prédicat porté par l'annonce, pas d'écriture sur l'édition, seconde livraison sans effet, et le test de bout en bout comparant l'annonce à l'effet |
| **Le contrat du front a-t-il été relu, et non supposé ?** | Oui : `types/programme/{session,registration}.ts`, `types/admin-planner.ts`, `types/event-programme.ts`, `types/views.ts`, `types/organization-workspace.ts`, `composables/api/planner.ts` et les blocs `sessions` et `registrations` de `composables/useApi.ts`. **Aucun nom de champ n'est renégocié** |
| **Quels écrans appellent réellement ces routes ?** | Mesuré : la programmation publique par trois écrans, le planificateur par un, **les inscriptions par aucun** — il n'existe pas encore d'écran d'inscription. Le contrat d'écriture d'une inscription est donc **posé par cette spécification**, sans front à contredire |
| **Les permissions existent-elles, et qui les détient ?** | Vérifié dans le semis : `programme.session.schedule` et `programme.registration.manage` existent ; le rôle `admin` détient les deux, le rôle `programmer` **seulement la première** (écart n° 119) |
| **Le SQL a-t-il été modifié ?** | **Non.** Aucun fichier de `docs/database/` n'a bougé, et les treize écarts se traitent dans le service ou se consignent |
| **Du code Rust a-t-il été écrit ?** | **Non.** `backend/crates/modules/programme` est inchangé : 17 372 lignes, celles de B4 |

---

## Ce qui reste ouvert

- **La question du commanditaire (écart n° 123)** : que devient la séance d'un dossier annulé après acceptation ? Posée en mots simples aux [points bloqués](../points-bloques.md), avec sa recommandation.
- ~~**Deux frontières à trancher au plan**~~ — **TRANCHÉES le même jour.** La preuve de consentement est une **dérogation bornée**, troisième écriture hors schéma du crate, isolée dans un fichier (R22) ; la naissance des séances est **synchrone**, dans la transaction d'acceptation, sur un hameçon unique (R3).
- **L'écart n° 56, toujours ouvert** : aucune permission ne distingue « composer la grille » de « publier le programme ». L'écart n° 119 en est le pendant sur les inscriptions.
