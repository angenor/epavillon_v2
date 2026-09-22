# Contrat — les textes qui engagent

**Servi par** : `crates/api` · **Source** : `crates/kernel` · **Décisions** : [research.md § R6, R7](../research.md)

> **Première route du dépôt à servir un contenu embarqué dans le binaire.** Aucun précédent, aucun
> utilitaire à réutiliser : le patron se compose du corps précalculé au montage de `/docs` et de
> l'empreinte de contenu de `routes/acces.rs`.

---

## 1. Où vivent les textes

Dans **`kernel`**, embarqués par `include_str!` — un fichier Markdown par texte et par langue, portant sa
version en tête.

**Pourquoi `kernel` et pas un module** : `programme` a besoin de la version à l'inscription et ne peut
dépendre d'aucun autre crate de module (principe II). `kernel` est le seul crate commun. Et la route,
qui ne relève d'aucun métier, se pose dans `crates/api/src/routes/`, à côté de `reference.rs` — le
précédent exact d'une route publique n'appartenant à personne.

**Deux textes seulement** : la politique de confidentialité et les conditions d'utilisation. Les licences
ne sont pas un texte qui engage : elles disent la police et les bibliothèques embarquées, n'appellent
aucun accord, n'ont pas de version opposable, et vivent dans les traductions de l'écran.

---

## 2. `GET /legal/{cle}` — servir un texte

**Sans session.** `cle` vaut `privacy` ou `terms`.

| Élément | Valeur |
|---|---|
| `operation_id` | `legal_texte` |
| Forme rendue | `LegalText` |
| Paramètre de langue | La langue de l'appelant ; **repli sur le français**, et la réponse dit la langue servie |
| En-tête de réponse | `ETag` — même mécanique que partout ailleurs |
| Clé inconnue | `404` |

`LegalText` porte la clé, la langue servie, **l'état**, la version, la date d'entrée en vigueur et le
corps en Markdown.

**Tant que l'IFDD n'a pas fourni un texte, il est « en attente »** (arbitré le 22/09) : l'en-tête du
fichier le déclare (`etat: en_attente`), il n'a **aucun corps**, la route rend `status: "pending"` avec
`body` et `effective_date` nuls, et l'écran affiche « Texte en préparation par l'IFDD ». **Aucun texte
n'est écrit par un outil** : il serait servi publiquement, et chaque inscription sur le site
enregistrerait un accord à un texte que personne à l'IFDD n'a lu. La version reste `2026-01`, celle que
l'inscription enregistrait déjà. Quand le texte arrive (`etat: publie`), **sa date d'entrée en vigueur
devient sa version**, et le test d'empreinte oblige à la lever. Le corps est **sérialisé une fois au montage**, comme le document OpenAPI : un texte embarqué
ne change pas entre deux démarrages.

**Le site et l'application appellent la même route.** C'est ce qui garantit qu'ils opposent le même texte
et la même version — les pages `/confidentialite` et `/conditions-utilisation` du site, aujourd'hui
liées et absentes, se construiront dessus, hors de cette étape.

---

## 3. La version, et la preuve

La version servie par cette route est **celle que `identity.consents` enregistre**. Le réglage
`PRIVACY_POLICY_VERSION` disparaît de la configuration : un réglage d'environnement pouvait nommer une
version dont personne ne pouvait produire le texte.

Ce qui change chez l'appelant tient en une ligne : `programme/src/service/registration.rs:122` lit la
version depuis `kernel` au lieu de la configuration. Les signatures de `exiger_le_consentement` et de
`repo::consents::accorder` ne bougent pas — elles prennent déjà la version par argument.

**Les lignes déjà écrites sous `2026-01` ne sont pas réécrites.** `identity.consents` est un historique :
une preuve dit ce qui a été accepté au moment où ça l'a été.

---

## 4. Le contrôle qui refuse un texte modifié sans nouvelle version

Un test garde, à côté des fichiers, l'empreinte attendue de chaque texte pour sa version déclarée.
Modifier le corps sans toucher la version fait diverger l'empreinte, et **le test échoue**.

C'est volontairement pénible : une preuve de consentement qui nomme une version dont le contenu a changé
n'oppose plus rien. Le même test vérifie que chaque texte porte bien une version et une date d'entrée en
vigueur, et que les deux langues d'un texte déclarent **la même version** — deux langues qui divergeraient
feraient deux textes.

**Et un second test rend les fichiers réels avec la grammaire close, et échoue sur toute construction
qu'elle ne couvre pas.** Ces textes seront écrits par quelqu'un d'autre, qui n'a aucune raison de
connaître les bornes du rendu : un tableau, une note de bas de page, une image ou un bloc de code
produiraient du charabia à l'écran **sans prévenir personne**. Le test échoue à la place, avec le
numéro de ligne et la construction en cause. C'est le prix d'un rendu écrit à la main plutôt
qu'emprunté — et il se paye une fois, à la construction, pas devant une négociatrice.

---

## 5. Le rendu côté application

La route sert du **Markdown**, et l'application le rend avec une grammaire close : titres de niveau 2 et
3, paragraphes, listes, liens, emphase. Tout le reste est échappé.

Pas de bibliothèque de rendu : le texte vient du binaire de l'API, écrit par l'IFDD — ce n'est pas un
contenu hostile, et la grammaire employée est connue. Le rendu se teste sans navigateur.

Une fois lu, un texte est gardé et **reste lisible hors connexion**, avec l'heure de sa lecture, par le
mécanisme de garde existant.
