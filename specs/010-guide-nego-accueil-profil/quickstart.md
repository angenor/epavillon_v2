# Recette — étape 0c

**Spécification** : [spec.md](spec.md) · **Plan** : [plan.md](plan.md) · **Contrats** : [contracts/](contracts/)

Ce document dit **comment prouver que l'étape est livrée**. Il ne porte pas de code : les détails vivent
dans les contrats et le modèle, l'implémentation dans `tasks.md`.

> **`make check` et `make check-db` ne se lancent pas** : ils commencent par détruire la base locale, qui
> n'a pas de sauvegarde. `make check-safe` seulement, et **en fin de cycle** — entre-temps, les contrôles
> ciblés ci-dessous.

---

## 0. Avant de commencer

```bash
docker compose -f ops/docker-compose.dev.yml up -d      # base, Valkey, Mailpit, Garage
```

La base locale porte le schéma de l'étape 0b. Elle **se migre, elle ne se recharge pas** :

```bash
psql "$DATABASE_URL" -f specs/010-guide-nego-accueil-profil/migration.sql   # rejouable
psql "$DATABASE_URL" -f specs/010-guide-nego-accueil-profil/migration.sql   # deux fois : rien ne change
pg_dump --schema-only "$DATABASE_URL" > /tmp/apres.sql                      # à comparer au modèle
```

Trois codes d'invitation sont déjà semés pour la recette de 0b : `NEGO-024` (réseau), `GENE-100`
(général, espace Climat), `STOP-999` (révoqué).

---

## 1. Le vocabulaire est servi sans une ligne de Rust

```bash
curl -s "$API/api/reference/taxonomies/negotiation_theme/terms" | jq '.[] | {code, label}'
```

**Attendu** : dix termes, dans l'ordre de la maquette — Adaptation, Atténuation, Finance, Pertes et
préjudices, Article 6, Transparence, Genre, Transition juste, Agriculture, Technologie —, libellés `fr`
et `en`, sans session.

**Et la preuve que les deux vocabulaires ne se mêlent pas** :

```bash
curl -s "$API/api/reference/taxonomies/activity_theme/terms" | jq 'length'   # 17, inchangé
```

---

## 2. Suivre des thématiques, et les retrouver ailleurs

1. Se connecter dans l'application, entrer avec `NEGO-024`.
2. L'écran « Mes thématiques » est proposé : « Étape 3 sur 3 — Thématiques ».
3. Cocher Adaptation et Genre. **Attendu** : le pied dit « 2 thématiques suivies — Adaptation, Genre ».
4. Tout décocher. **Attendu** : la validation n'est pas possible, et l'écran dit qu'il en faut au moins une.
5. Recocher les deux, valider. **Attendu** : « Ma journée » s'ouvre.
6. Se connecter **sur un second navigateur** avec le même compte. **Attendu** : les deux mêmes
   thématiques, sans geste.

En base :

```sql
SELECT t.code, s.followed_at, s.left_at
FROM negotiation.theme_subscriptions s
JOIN reference.taxonomy_terms t ON t.id = s.theme_term_id
WHERE s.person_id = :personne ORDER BY t.sort_order;
```

**Attendu** : deux lignes vivantes. Puis retirer Genre depuis le profil et relire : **`left_at` est
posé, la ligne n'a pas disparu**.

Et l'auteur de l'écriture :

```sql
SELECT action, actor_id FROM platform.audit_log
WHERE table_name = 'theme_subscriptions' ORDER BY occurred_at DESC LIMIT 3;
```

**Attendu** : `actor_id` renseigné à chaque ligne — jamais une trace anonyme.

---

## 3. Ce que la base refuse, et ce que l'API répond

| Geste | Attendu |
|---|---|
| `PUT` avec une liste vide | `400`, code stable, message français nommant la règle |
| `PUT` avec un code inexistant | `400`, **et le message nomme le code refusé** |
| `PUT` avec un code d'`activity_theme` (`sustainable_livestock`) | `400` — le vocabulaire est gardé par la base, pas par le code |
| `INSERT` direct d'un terme d'un autre vocabulaire | La base lève : `tg_theme_subscriptions_check_theme` |
| Deux `PUT` identiques d'affilée | Même état, **aucune écriture de plus** dans l'audit |
| `GET` puis `GET` avec l'`ETag` en `If-None-Match` | `304`, sans corps |
| `PUT` sans session | `401` |

---

## 4. Hors connexion

1. **Lecture.** Charger les quatre écrans, couper le réseau, les rouvrir. **Attendu** : tous s'affichent,
   chacun avec son heure de lecture ; le bandeau dit « Hors connexion — lu à … », une fois par épisode.
2. **Écriture différée.** Réseau coupé, changer ses thématiques. **Attendu** : le choix s'affiche
   aussitôt. Fermer l'application, rétablir le réseau, rouvrir. **Attendu** : le choix est parti, et la
   base porte **une seule** écriture.
3. **Intentions successives.** Réseau coupé, choisir, changer d'avis, choisir encore. Rétablir.
   **Attendu** : une seule écriture, celle du dernier état.
4. **Retour en cours de route.** Réseau rétabli pendant que l'écran est ouvert. **Attendu** : le bandeau
   cède à « Synchronisé à … », sans rechargement.
5. **Stockage refusé** (navigation privée, site data bloqué). **Attendu** : l'application s'ouvre, rien
   ne lève, la place occupée dit qu'elle ne peut pas mesurer.

```bash
node frontend/scripts/guide-nego-verifier-garde.mjs "$SITE"   # toutes les adresses gardées en 200
```

---

## 5. « Ma journée » n'a rien, et ne le cache pas

Compte neuf, en ligne puis en mode avion.

**Attendu**, dans cet ordre exact : prochaine session de négociation · changements du jour ·
aujourd'hui vos trois agendas · documents récents · accès au lexique.

Chacun des quatre premiers porte **son** état vide : ce qui manque, quand cela reviendra, une sortie
quand il y en a une. **Aucun n'affiche d'erreur, aucun ne reste en chargement, aucun ne disparaît.**

L'en-tête porte l'avatar à gauche — il ouvre « Profil et réglages » — et le bouton « Aa ». **Aucune
cloche** : elle vient à l'étape 3b.

Sans compte, l'écran s'ouvre aussi, et invite à créer un compte ou à se connecter.

---

## 6. Profil et réglages

| Point | Attendu |
|---|---|
| Deux chemins | L'avatar de « Ma journée » et la ligne de l'onglet Ressources mènent au même écran |
| Mon suivi | « Mes thématiques » (valeur : les thématiques suivies) · « Mes téléchargements » · « Mon accès » (livré en 0b, **état jamais rôle**) |
| Mes téléchargements | Aucun document gardé, et la place occupée réelle |
| Libérer | La confirmation dit ce qui part et ce qui reste lisible sans réseau ; la place diminue |
| Affichage | Le thème garde ses trois valeurs et **reste propre à l'appareil** — vérifier sur un second navigateur |
| Dernière synchronisation | L'heure, **sans fuseau** (écart 32), et ce n'est pas une action |
| Notifications par thématique | **Absent** — étape 3b |

---

## 7. À propos, et les textes

1. Ouvrir « À propos ». **Attendu** : nom, version, édition, éditeur, l'étiquette de source, le
   paragraphe de confidentialité, les trois textes — **et aucun interrupteur d'accord** (écart 40).
2. Le paragraphe distingue ce qui reste sur le téléphone de ce qui suit le compte.
3. Ouvrir la politique de confidentialité. **Attendu** : le texte entier, dans le dessin de Guide Négo,
   avec sa version.
4. Couper le réseau, la rouvrir. **Attendu** : elle s'affiche, avec l'heure de sa lecture.
5. Les licences s'ouvrent et n'appellent aucun accord.
6. Sans compte, l'écran et ses textes se lisent.

**La source est unique** :

```bash
curl -s "$API/api/legal/privacy" | jq '{cle, langue, version}'
curl -s -H 'Accept-Language: en' "$API/api/legal/privacy" | jq '.version'   # même version
curl -s "$API/api/legal/inconnu" -o /dev/null -w '%{http_code}\n'           # 404
```

**Et le contrôle mord** : modifier une phrase d'un texte sans toucher sa version, puis
`cargo test -p kernel` — **le test échoue**. Remettre la phrase, ou lever la version.

En base, après une inscription qui recueille un consentement :

```sql
SELECT purpose, policy_version FROM identity.current_consents WHERE person_id = :personne;
```

**Attendu** : la version servie par l'API, et non `2026-01` venu de la configuration. Les lignes
anciennes, elles, portent toujours `2026-01` — un historique ne se réécrit pas.

---

## 8. L'apparence, mesurée

À **320, 360 et 390 px**, en thème clair et en thème sombre, sur les quatre écrans nouveaux plus le
profil : aucun débordement horizontal, aucune cible sous 44 px, anneau de focus partout.

*Deux fausses alertes connues depuis 0b* : `GnPilule` fait 40 px de dessin dans une cible de 48 gagnée
par un `::after` en débord, et les champs natifs des interrupteurs sont plus petits que la surface
touchée — une sonde qui lit `getBoundingClientRect` les accuse à tort.

La page interne des composants montre `GnAvatar`, `GnJauge` et `GnTexteLong` dans les deux thèmes.

---

## 9. Les portes

Pendant le cycle, contrôles ciblés :

```bash
cd frontend && npm run typecheck && npm run test:guide-nego && npm run check:guide-nego
cd backend  && cargo test -p negotiation && cargo test -p kernel
make openapi && make check-api-contract
```

En fin de cycle seulement, **API arrêtée** — deux tests d'`identity` sont sensibles à une activité
concurrente sur la base :

```bash
make check-safe
```

**Attendu** : tout au vert, zéro route en attente au contrat, et les clés `fr`/`en` concordantes sur
tous les fichiers de Guide Négo.

---

## 10. Ce qui ne se fait que sur un appareil réel

- Installer sur un Android puis un iPhone, choisir ses thématiques en 3G bridée, puis en mode avion.
- Vérifier que la file repart quand le téléphone retrouve le réseau **après une nuit en veille**.
- **T112 de l'étape 0b reste due** et se déroule au même moment.
