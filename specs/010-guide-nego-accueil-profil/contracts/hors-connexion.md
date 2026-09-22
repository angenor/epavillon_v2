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

Chaque écran porte l'heure de sa lecture. Le format est celui de l'écart 32 : heure du téléphone, sans
fuseau — « à 11:35 », « hier à 23:10 », « le 11 nov. à 23:10 ».

**« Ma journée » ne peut pas échouer** : à cette étape elle n'attend aucune donnée. Ses blocs vides ne
sont pas un état d'erreur déguisé, et l'état d'erreur ne sert qu'à une lecture qui échoue alors que le
réseau est là.

---

## 2. Ce qui s'écrit sans réseau — la file

**Elle n'existe pas aujourd'hui, et 0c la construit.** Un second magasin IndexedDB, `ecritures`, dans la
même base `guide-nego`.

| Règle | Pourquoi |
|---|---|
| **Une seule entrée par clé** : une nouvelle intention remplace la précédente | Le `PUT` porte l'état entier ; garder trois intentions successives ferait trois écritures pour un seul état voulu |
| L'écran affiche le choix **aussitôt**, sans attendre le départ | Sinon la personne ne sait pas si son geste a pris |
| Départ déclenché par l'événement `online` **et** par le retour au premier plan | `useGnConnexion` n'écoute aujourd'hui que `offline` : l'écoute de `online` est à ajouter |
| L'entrée n'est retirée de la file **qu'après succès** | Un échec la laisse en place, et elle repart au prochain déclenchement |
| Aucune fonction de la file ne lève | Comme `garde.ts` : un stockage refusé ne casse pas l'application |
| L'idempotence vient de la forme de la route, pas d'un jeton | Voir le contrat des thématiques |

**Ce qui ne se met pas en file** : la doctrine de 0b reste entière — un accès ne s'annonce pas avant
d'être obtenu (FR-019 de 0b). La file est réservée à ce dont la personne est elle-même l'autorité : ses
thématiques. Un accord de consentement l'y rejoindra à l'étape qui l'offrira.

**Ce que la file n'est pas** : un mécanisme général de synchronisation. Elle porte une clé, une intention,
et repart. Ce qui dépasserait ce besoin est à écrire quand le besoin existera.

---

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
4. Réseau rétabli pendant que l'écran est ouvert : le bandeau cède à « Synchronisé à … » sans rechargement.
5. Stockage refusé par le navigateur : l'application s'ouvre, rien ne lève, la place occupée le dit.
6. La liste des adresses gardées par le service worker couvre les écrans nouveaux — `verifier-garde`
   les sert toutes en 200 contre la version construite.
