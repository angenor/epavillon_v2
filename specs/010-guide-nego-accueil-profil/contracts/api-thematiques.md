# Contrat — les thématiques suivies

**Crate** : `negotiation` · **Décisions** : [research.md § R2, R3, R4](../research.md)

> Le contrat servi est **engendré** par `make openapi`. Ce document dit ce qu'il doit porter et
> pourquoi ; il ne le remplace pas. Les corps sont **nommés**, pas décrits : leur définition vit dans
> `frontend/app/types/`, et `make check-api-contract` vérifie le lien.

---

## 1. Lire le vocabulaire — route existante, rien à écrire

```
GET /api/reference/taxonomies/negotiation_theme/terms
```

Déjà servie par `backend/crates/api/src/routes/reference.rs` (`operation_id = "reference_termes"`),
sans session, termes actifs triés par `sort_order` puis `code`, libellés multilingues rendus entiers.
Semer le vocabulaire suffit à la servir.

Côté client : `api.reference.terms('negotiation_theme')`, déjà montée dans `useApi()`. **Aucune méthode
nouvelle, aucune route nouvelle.** Une seconde fabrique qui rendrait les mêmes termes serait un contrat
de trop.

---

## 2. `GET /negotiation/me/themes` — ce que je suis

**Session requise.** Rend les thématiques suivies par la personne connectée.

| Élément | Valeur |
|---|---|
| `operation_id` | `negotiation_mes_thematiques` |
| Forme rendue | `MyThemes` |
| En-tête de réponse | `ETag` — empreinte du corps sérialisé, `kernel::crypto::token_hash`, 16 octets en hexadécimal, entre guillemets |
| `If-None-Match` correspondant | `304`, sans corps |
| Sans thématique suivie | `200` avec une liste vide — **jamais un 404** |

`MyThemes` porte la liste des thématiques suivies, chacune avec son code, son libellé résolu dans la
langue de l'appelant et son ordre, plus la date depuis laquelle elle est suivie. Le libellé est **résolu
par l'API** : l'écran n'a pas à connaître le repli sur le français.

**Pourquoi une empreinte de contenu et pas un « modifié depuis »** : l'état se compose de plusieurs
lignes et aucune colonne ne porte l'instant où l'ensemble a changé. Une empreinte de contenu ne peut pas
se tromper ; un 304 fautif laisserait sur le téléphone des thématiques que la personne a retirées ailleurs.
C'est le raisonnement déjà inscrit dans `routes/acces.rs` à l'étape 0b.

---

## 3. `PUT /negotiation/me/themes` — remplacer la liste

**Session requise. Remplacement en bloc, jamais un ajout ni un retrait unitaire.**

| Élément | Valeur |
|---|---|
| `operation_id` | `negotiation_suivre_des_thematiques` |
| Forme reçue → rendue | `ThemesPayload` → `MyThemes` |
| Corps reçu | La liste **entière** des codes suivis |
| Réponse | `200` avec le nouvel état, et son `ETag` |

**Comportement**

1. La transaction pose le contexte d'écriture (`app.actor_id`, `app.request_id`) avant la première
   écriture, par `Db::write(&ctx)` — sans quoi l'audit de la table serait anonyme.
2. Les suivis vivants absents de la liste reçue sont **fermés** (`left_at = now()`), jamais supprimés.
3. Les codes reçus sans suivi vivant en ouvrent un.
4. Les codes reçus déjà suivis ne sont pas touchés — leur `followed_at` ne bouge pas, et l'audit ne
   s'encombre pas d'une écriture sans changement.

**Idempotence.** Rejouer le même corps donne le même état et n'écrit rien de plus. C'est ce qui rend sûre
la file d'écritures différées : une intention repartie au retour du réseau ne peut pas produire un double
effet.

**Refus**

| Cas | Réponse |
|---|---|
| Liste vide | `400`, code stable, message français nommant la règle : au moins une thématique |
| Code inconnu, ou d'un autre vocabulaire | `400`, et le message **nomme le code refusé** — patron de `programme/src/repo/themes.rs`, qui ne dit jamais « une thématique est inconnue » |
| Terme désactivé | Accepté s'il est déjà suivi, refusé s'il est nouveau : on ne retire pas à quelqu'un ce qu'il suivait, on ne laisse pas en choisir un qui n'est plus proposé |
| Doublons dans la liste reçue | Acceptés et réduits — le client n'a pas à s'en soucier |

**Codes d'erreur.** Le catalogue de `kernel/src/error.rs` **s'étend, il ne se double pas** : deux codes
nouveaux, l'un pour la liste vide, l'autre pour le code de thématique inconnu. Ils remontent seuls dans
`frontend/app/types/api.ts` par le schéma du catalogue.

---

## 4. Ce que cette étape n'ajoute pas

- **Aucune route `/admin`** : rien ne s'administre ici ([research.md § R14](../research.md)).
- **Aucune route de profil** : le nom, le pays et la langue se lisent déjà par la session
  (`GET /auth/me`), et « Mon accès » par `GET /negotiation/me/access`, livrée en 0b.
- **Aucune route d'accord** : aucun consentement n'est offert à l'écran (écart 40).
