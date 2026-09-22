# Contrat — le hors-connexion de l'étape 0c

**Décisions** : [research.md § R5, R8](../research.md) · **Principe** : constitution XI, « Hors connexion d'abord »

> Ce qui existe depuis 0a : le service worker engendré, la garde des données lues en IndexedDB
> (magasin `lectures`), `useGnLecture` qui rend la garde avant d'attendre le réseau,
> `useGnMomentLecture` pour « lu à 11:35 », et le bandeau qui paraît une fois par épisode.
> **Rien de cela ne se réécrit.** Ce document ne porte que ce que 0c ajoute.

---

## 1. Ce qui se lit sans réseau

| Écran | Ce qui s'affiche hors connexion | Clé de garde |
|---|---|---|
| Mes thématiques | La liste du vocabulaire et les suivis, tels que lus la dernière fois | `thematiques`, `mes-thematiques` |
| Ma journée | L'écran entier, tous blocs vides — il ne lit rien qui puisse manquer | — |
| Profil et réglages | Le nom, le pays, les thématiques suivies, l'accès | Celles déjà posées en 0b, plus `mes-thematiques` |
| À propos | L'écran et ses textes déjà lus | `texte-privacy`, `texte-terms` |

**Le vocabulaire se lit à la première ouverture en ligne, avec le drapeau, et va dans la garde comme
lui** : dix termes, une lecture publique qui ne demande même pas de session. La ligne « Mes thématiques »
du profil a ainsi toujours ses noms, y compris sur un appareil qui n'a jamais ouvert l'écran des
thématiques — sans quoi elle devrait se rabattre sur un simple compte, « 2 thématiques suivies ». Ce
repli reste écrit, pour le cas où la toute première ouverture se ferait sans réseau.

Chaque écran porte l'heure de sa lecture. Le format est celui de l'écart 32 : heure du téléphone, sans
fuseau — « à 11:35 », « hier à 23:10 », « le 11 nov. à 23:10 ».

**« Ma journée » ne peut pas échouer** : à cette étape elle n'attend aucune donnée. Ses blocs vides ne
sont pas un état d'erreur déguisé, et l'état d'erreur ne sert qu'à une lecture qui échoue alors que le
réseau est là.

---

## 2. Ce qui s'écrit sans réseau — la file

**Elle n'existe pas aujourd'hui, et 0c la construit.** Un second magasin IndexedDB, `ecritures`, dans la
même base `guide-nego`. Ce que cette étape y pose servira au parcours « Ma première COP » (étape 2) et
aux signalements (3b) : les règles ci-dessous sont écrites pour tenir au-delà des thématiques.

### Ce que porte une intention

| Champ | Pourquoi |
|---|---|
| La **clé** de ce qu'elle écrit | Une seule intention par clé (voir ci-dessous) |
| Le **corps** à envoyer | L'état entier voulu, jamais un delta |
| L'**empreinte de l'état** sur lequel elle a été prise | Envoyée en `If-Match` : c'est ce qui empêche un choix en retard d'écraser un choix plus récent |
| L'**identifiant de la personne** qui l'a prise | Une intention ne part **jamais** sous un autre compte |
| L'heure de sa prise | Pour le dire à la personne si l'envoi tarde |

### Les règles

| Règle | Pourquoi |
|---|---|
| **Une seule entrée par clé** : une nouvelle intention remplace la précédente | Le corps porte l'état entier ; garder trois intentions successives ferait trois écritures pour un seul état voulu. **L'empreinte gardée reste celle de la première** — c'est l'état qu'a vu la personne avant de commencer à changer d'avis |
| L'écran affiche le choix **aussitôt**, sans attendre le départ | Sinon la personne ne sait pas si son geste a pris |
| Départ déclenché par **l'ouverture de l'application**, par l'événement `online`, et par le retour au premier plan | Les trois sont nécessaires : un téléphone rouvert le lendemain n'émet pas d'`online`, et une application restée ouverte ne se réouvre pas. `useGnConnexion` n'écoute aujourd'hui que `offline` |
| L'entrée n'est retirée de la file **qu'après succès** | Un échec la laisse en place, et elle repart au prochain déclenchement |
| Un **`412`** retire l'entrée, déclenche une relecture et **le dit à la personne** | L'intention est abandonnée, jamais fusionnée : personne n'aurait choisi la liste qu'une fusion produirait |
| Un refus **définitif** (`400`, `401`, `403`) retire aussi l'entrée, et le dit | Une intention qui ne peut pas aboutir n'a rien à faire en file : elle repartirait à chaque ouverture |
| Une panne réseau ou un `5xx` **garde** l'entrée | C'est exactement ce pour quoi la file existe |
| La file se **vide à la déconnexion** | Ce qu'une personne a choisi ne part pas sous le compte de la suivante, sur un téléphone partagé au stand |
| Aucune fonction de la file ne lève | Comme `garde.ts` : un stockage refusé ne casse pas l'application |
| L'idempotence vient de la forme de la route, `If-Match` de la fraîcheur | L'idempotence protège du **rejeu**, pas de l'**ancienneté**. Il faut les deux |

### Ce qui ne s'y met pas

La doctrine de 0b reste entière — un accès ne s'annonce pas avant d'être obtenu (FR-019 de 0b). La file
est réservée à ce dont la personne est elle-même l'autorité : ses thématiques. Un accord de consentement
l'y rejoindra à l'étape qui l'offrira.

**Ce que la file n'est pas** : un mécanisme général de synchronisation. Elle porte une clé, une
intention, une empreinte et un compte, et repart. Ce qui dépasserait ce besoin est à écrire quand le
besoin existera.

### Ce que les tests doivent prouver, sans navigateur

1. Deux intentions successives sur la même clé : **une seule entrée**, et c'est **l'empreinte de la
   première** qui est gardée.
2. Un `412` : l'entrée part de la file, la relecture est déclenchée, le message est produit.
3. Un `5xx` : l'entrée reste, et repart au déclenchement suivant.
4. Une déconnexion : la file est vide, et rien ne part ensuite.
5. Une intention prise par une personne, une autre connectée : **rien ne part**.
6. Trois déclencheurs — ouverture, `online`, retour au premier plan — provoquent chacun un départ, et
   deux déclencheurs simultanés n'envoient **qu'une fois**.

## 3. La place occupée, et comment on la libère

**Mesure** : l'estimation du navigateur, seule mesure qui compte l'ensemble — coquille, données lues,
documents à venir. Là où elle n'est pas disponible, l'écran **dit qu'il ne peut pas mesurer sur cet
appareil** plutôt que d'annoncer un zéro faux.

**Libérer** vide les données lues et, à partir de l'étape 1, les documents téléchargés. **La coquille
reste** : c'est elle qui permet d'ouvrir l'application sans réseau, et la vider retirerait exactement ce
que la personne est venue chercher en salle. La confirmation le dit en toutes lettres, comme celle de la
déconnexion dit ce qui reste sur le téléphone.

---

## 4. Ce qui se vérifie à la recette

1. Mode avion : les quatre écrans s'ouvrent, chacun avec son heure de lecture.
2. Mode avion : choisir deux thématiques, fermer l'application, rétablir le réseau, rouvrir — le choix
   est parti, **une seule fois**, et la base le porte.
3. Mode avion : choisir, changer d'avis, choisir encore — une seule écriture part.
4. **Deux appareils.** Téléphone hors réseau, choisir. Sur un second navigateur en ligne, même compte,
   choisir autre chose. Rendre le réseau au téléphone : **le choix du second survit**, le téléphone
   affiche l'état vrai et dit « Vos thématiques ont changé sur un autre appareil ».
5. **Téléphone partagé.** Hors réseau, choisir, se déconnecter, connecter un autre compte, rendre le
   réseau : **rien ne part**, et le second compte garde ses thématiques.
6. Réseau rétabli pendant que l'écran est ouvert : le bandeau cède à « Synchronisé à … » sans rechargement.
7. Stockage refusé par le navigateur : l'application s'ouvre, rien ne lève, la place occupée le dit.
8. La liste des adresses gardées par le service worker couvre les écrans nouveaux — `verifier-garde`
   les sert toutes en 200 contre la version construite.
