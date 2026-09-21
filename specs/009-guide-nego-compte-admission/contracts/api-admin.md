# Contrat — le back-office de l'admission

> Routes du crate `negotiation`, sous `/api/admin/negotiation`. **Toutes** passent par l'extracteur
> `Perimeter` et par `RequiresAnyScope<SpaceManage>` — permission `negotiation.space.manage`, déjà
> semée dans `100_negotiations.sql`. Aucune ne se contente de filtrer à l'affichage.

## Le périmètre, d'abord

**Les douze routes de ce contrat** — sept sur les codes, deux sur le mode d'admission, trois sur les
demandes — passent **toutes** par le périmètre, y compris celles qui ne listent rien. Un test d'URL
forgée qui n'en couvrirait que sept laisserait l'admission et les demandes ouvertes.

`identity.administered_events($1)` rend toujours une ligne, jamais NULL, et ses trois cas restent
distincts dans le code (principe V) :

| Retour | Réponse de ces routes |
|---|---|
| `(true, …)` | tous les codes, tous les usages, toutes les demandes |
| `(false, {…})` | seulement les espaces rattachés à ces éditions |
| `(false, '{}')` | **refus d'accès explicite**, jamais une liste vide |

Une route paramétrée par un identifiant — `{id}` d'un code, d'un usage, d'une demande — **vérifie le
périmètre avant de lire** : un identifiant hors périmètre se refuse comme un identifiant inexistant,
sans que la forme de la réponse les distingue (principe IX). C'est le test « URL forgée » exigé par
le principe X.

## Les codes

| Route | Ce qu'elle fait |
|---|---|
| `GET /api/admin/negotiation/invitation-codes` | Liste, filtres `?etat=`, `?espace=`, `?q=`, pagination. Sert la vue `negotiation.v_invitation_codes` : état, usages sur maximum, validité, portée, auteur |
| `POST /api/admin/negotiation/invitation-codes` | Crée. **Le code est engendré par l'API**, jamais choisi |
| `GET /api/admin/negotiation/invitation-codes/{id}` | Le détail, code en clair compris |
| `POST /api/admin/negotiation/invitation-codes/{id}/revoke` | Révoque. **Ne retire aucun accès** |
| `GET /api/admin/negotiation/invitation-codes/{id}/uses` | Qui est entré, quand, et si l'accès tient encore |
| `POST /api/admin/negotiation/invitation-codes/{id}/uses/{person_id}/revoke-access` | Retire l'accès d'une personne |
| `POST /api/admin/negotiation/invitation-codes/{id}/revoke-all-access` | Retire l'accès de toutes les personnes du code |

Création :

```jsonc
{
  "label": "Réseau des négociatrices — COP31",
  "scope": { "type": "negotiation_space", "id": "…" },   // ou { "type": "global" }
  "grants_network": "women_negotiators",                  // facultatif
  "max_uses": 120,                                        // null = sans limite
  "valid_from": "2026-11-01T00:00:00Z",
  "valid_until": "2026-11-22T23:59:59Z"                   // null = sans terme
}
```

- **Le `scope.type` ne peut valoir que `negotiation_space` ou `global`** — les deux `allowed_scopes`
  du rôle `negotiator`. La base le refuse deux fois : `ck_invitation_codes_scope` puis, à
  l'attribution, `tg_check_role_scope`.
- **Le code est engendré par l'API** : **huit caractères, tirets compris** — par exemple `NEGO-024` —,
  alphabet sans `0/O` ni `1/I/L`, un tiret pour la lisibilité : il se recopie à la main depuis WhatsApp. Il est rendu à la création **et reste
  lisible dans la liste** : c'est un code partagé, pas un secret nominatif (FR-037).
- Révoquer et retirer les accès sont **deux routes**, parce que ce sont deux décisions (ADR-006).
  La seconde demande confirmation à l'écran.

## Le mode d'admission

| Route | Ce qu'elle fait |
|---|---|
| `GET /api/admin/negotiation/admission` | Le mode courant, et ce que chaque valeur produit pour la personne qui entre |
| `PUT /api/admin/negotiation/admission` | `{ "mode": "approval" }` — prend effet **à la tentative suivante**, sans mise en ligne |

Une valeur hors des trois sort en `NEGOTIATION_ADMISSION_MODE_INVALID`, qui désigne le champ.

## Les demandes

| Route | Ce qu'elle fait |
|---|---|
| `GET /api/admin/negotiation/access-requests?etat=pending` | La file : nom, pays, heure d'envoi, code éventuel, message |
| `POST /api/admin/negotiation/access-requests/{id}/approve` | Admet. Accorde l'accès, l'appartenance au réseau si la demande portait un code du réseau, et met le courriel en file — **une transaction** |
| `POST /api/admin/negotiation/access-requests/{id}/reject` | Refuse, motif facultatif, courriel en file |

Une demande déjà tranchée sort en `NEGOTIATION_ACCESS_REQUEST_DECIDED` : la transition est refusée par
le trigger de la base, l'API la traduit (principe VIII).

## Ce que chaque écriture laisse derrière elle

Toute route d'écriture ouvre sa transaction par `Db::write(&ctx)` — `app.actor_id` et
`app.request_id` posés avant la première écriture (principe VII). `platform.tg_audit()` est déjà posé
sur `invitation_codes`, `access_requests` et `space_members` : **la trace de FR-046 ne demande aucun
code**, seulement d'écrire par la bonne porte.
