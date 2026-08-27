# Contrat — les refus

**Module** : Direct + Tableaux de bord (B9) · **Date** : 2026-08-27 · [routes.md](routes.md) · [research.md](../research.md)

Le catalogue général vit dans [`specs/001-socle-identite/contracts/errors.md`](../../001-socle-identite/contracts/errors.md). Ce fichier ne porte que ce que B9 ajoute — **trois codes** — et, surtout, **la ligne qui sépare un refus HTTP d'un refus rendu en 200**.

---

## 1. La ligne, et pourquoi elle est là

Le contrat du site range **dix issues d'écriture sous un seul discriminant** (`IncidentWriteStatus`), `forbidden` et `not_found` compris, et l'écran les traduit une par une : `admin.incident.form.error.<statut>` (`pages/admin/incidents/nouveau.vue`, ligne 148). Répondre 403 à ces deux-là ferait **lever le client** là où il attend un message posé sous un champ.

La règle qui tranche :

> **Le périmètre est un contrôle d'accès qui ne figure pas au contrat du site et ne doit rien révéler : il répond en HTTP. Tout le reste est une issue prévue par le contrat, qui s'affiche dans le formulaire : elle répond en 200.**

C'est la même ligne que celle tenue par la vitrine (B8) et par les événements (B3).

| Cas | Réponse | Pourquoi |
|---|---|---|
| Aucune session | **401** `UNAUTHENTICATED` | hors contrat |
| Périmètre d'administration **vide** | **403** `FORBIDDEN` | jamais une liste vide, jamais un corps de contrat — les trois cas du périmètre restent distincts |
| Édition **hors périmètre** | **404** `NOT_FOUND` | un identifiant hors périmètre se refuse **comme un identifiant inexistant** : la forme ne les distingue pas (principe IX) |
| `analytics.dashboard.read` absente sur l'édition | **403** `FORBIDDEN` | le tableau de bord n'a pas d'issue de contrat : il s'ouvre ou il se refuse |
| `live.incident.publish` absente sur la portée visée | **200** `{ status: 'forbidden' }` | le contrat le nomme, l'écran le traduit |
| Message introuvable, sur une **écriture** | **200** `{ status: 'not_found' }` | idem |
| Message introuvable, sur `GET /admin/incidents/{id}` | **404** | le site le lit par `callOrNull`, pour qui un 404 **est** une réponse |
| Cible manquante ou orpheline | **200** `{ status: 'missing_target' }` | |
| Message absent dans l'une des deux langues | **200** `{ status: 'missing_message' }` | |
| Fin d'affichage antérieure au début | **200** `{ status: 'invalid_window' }` | |
| Retrait d'un message jamais publié | **200** `{ status: 'not_published' }` | |

**Sur une issue de refus, `incident` vaut `null`.** Sur une issue de succès, il porte la ligne de gestion **relue par `live.event_incidents()`** : l'état affiché est celui que la base calcule, jamais un état recomposé côté service.

---

## 2. Les trois codes ajoutés au catalogue

Ils ne répondent **jamais** sur le chemin nominal : le service valide en amont et rend l'issue de contrat. Ils existent pour le cas où un refus de la base **échappe** à ce chemin — une écriture concurrente, une donnée reprise, un chemin ajouté plus tard qui oublierait la validation. C'est le parti déjà pris par B3 pour ses unicités.

| Code | Statut | Message français | Champ | Contrainte traduite |
|---|---|---|---|---|
| `LIVE_INCIDENT_SCOPE_TARGET_MISMATCH` | 422 | « La portée choisie et la cible renseignée ne correspondent pas : une portée vise exactement une cible, et la portée globale n'en vise aucune. » | `scope` | `ck_incidents_scope_target` |
| `LIVE_INCIDENT_WINDOW_INVALID` | 422 | « La fin d'affichage doit être postérieure au début. » | `display_until` | `ck_incidents_window` |
| `LIVE_INCIDENT_NOT_PUBLISHED` | 422 | « Ce message n'a jamais été publié : il n'y a rien à retirer. » | — | `no_data_found` levée par `live.unpublish_incident()` |

**Une quatrième contrainte est traduite sans qu'aucun chemin puisse l'atteindre**, et c'est délibéré :

| Contrainte | Traduction | Pourquoi la déclarer |
|---|---|---|
| `ck_incidents_unpublish_shape` | `CONFLICT` | `live.unpublish_incident()` exige déjà `published_at IS NOT NULL` : la contrainte est **inatteignable par les fonctions**. La déclarer est le seul moyen de s'apercevoir qu'elle a remonté — c'est-à-dire qu'une écriture a contourné les fonctions. Le précédent existe dans `pg_error.rs` : « ces trois-là ne doivent jamais remonter, et le dire ici est le seul moyen de s'en apercevoir » |

**Aucun code n'est ajouté pour le module analytique.** Ses refus sont ceux du noyau : 401, 403, 404.

---

## 3. Le double chemin de la validation, et pourquoi ce n'est pas une réimplémentation

Le principe VIII interdit de redoubler une contrainte de la base. Ici, deux règles sont vérifiées **avant** l'écriture — la cohérence portée/cible et la fenêtre d'affichage — alors que la base les porte.

**Ce n'est pas un redoublement, parce que les deux chemins ne répondent pas la même chose** :

| | Le service, en amont | La base, en aval |
|---|---|---|
| Ce qu'il rend | l'issue que le **contrat du site nomme**, posée sur le bon champ du formulaire | un code stable et un message français |
| Quand il répond | sur le chemin nominal, **toujours** | seulement si un refus échappe |
| Ce qu'il garantit | que l'écran affiche « choisissez une activité » sous le bon champ | que rien d'incohérent n'entre en base, quel que soit le chemin |

La barrière reste la contrainte. Ce que le service ajoute, c'est **la forme du refus**, que la base ne peut pas connaître — elle ignore qu'un formulaire a un champ `session_id` et un champ `scope`.

**Ce que le service ne fait pas** : rejouer la machine à états de la publication. Un message jamais publié qu'on tente de retirer n'est **pas** détecté en amont — l'appel à `live.unpublish_incident()` est fait, et sa levée est traduite en `not_published`. La condition vit à un seul endroit.

---

## 4. Ce que les refus ne disent jamais

| Interdit | Pourquoi |
|---|---|
| Distinguer par la **forme** une édition hors périmètre d'une édition inexistante | les deux rendent 404 avec le même corps : la forme de la réponse ne révèle pas ce qui existe ailleurs |
| Nommer l'édition, l'activité ou l'organisation d'une cible hors périmètre | un message d'erreur qui cite « COP30 — Bakou » à un compte détaché sur la COP31 lui apprend qu'elle existe |
| Renvoyer le texte brut d'une erreur PostgreSQL | il porte des noms de tables et parfois des valeurs. Tout ce qui n'est pas répertorié sort en `INTERNAL` |
| Reformuler un message levé par un déclencheur du modèle | il est déjà en français ; le reformuler produirait deux libellés pour un même refus, dont le second se périmerait |

---

## 5. La lecture publique ne refuse rien

`GET /events/{event_id}/incidents` n'a aucune garde et **ne rend jamais 404** : une édition inconnue rend une liste vide.

**C'est délibéré.** Cette route ne doit pas devenir un moyen de savoir si une édition existe, et un bandeau absent se lit exactement comme une édition sans incident — ce qui est le cas normal.

Elle peut rendre **503** si la base est injoignable, comme toute route.
