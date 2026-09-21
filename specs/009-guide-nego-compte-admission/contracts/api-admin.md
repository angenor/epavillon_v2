# Contrat — le back-office de l'admission

> Routes du crate `negotiation`, sous `/api/admin/negotiation`. **Toutes** exigent
> `negotiation.space.manage` **sur la portée globale** — permission déjà semée dans
> `100_negotiations.sql`. Aucune ne se contente de filtrer à l'affichage.

## Le périmètre, d'abord — et pour Guide Négo, il est global

**Tranché par le commanditaire le 21/09** : seul un administrateur **global** tient les codes, les
usages, les demandes et le mode d'admission.

La garde des douze routes est donc, et uniquement :

```
identity.has_permission(personne, 'negotiation.space.manage', 'global', NULL)
```

c'est-à-dire l'extracteur `Requires<SpaceManage>` du noyau, qui teste `Scope::Global`.

**Et surtout pas `RequiresAnyScope`.** Le rôle `admin` porte `negotiation.space.manage` et
s'attribue aussi **sur un événement** : un administrateur d'une seule édition passerait « n'importe
quelle portée » et ouvrirait tout le back-office de Guide Négo — alors qu'aucun espace de
négociation n'est rattaché à un événement, et qu'il n'a donc rien à y voir. Le piège est d'autant
plus sournois que la route paraîtrait gardée.

**`Perimeter` ne sert pas ici**, et `identity.administered_events()` n'est pas touchée : elle est
bâtie sur `programme.proposal.read_all` et ne rend que des portées `event`, quand un code
d'invitation porte `global` ou `negotiation_space`. Aucune fonction de périmètre nouvelle, aucun
lien espace ↔ événement, aucun semis de rôle.

| Qui frappe | Réponse de ces douze routes |
|---|---|
| `negotiation.space.manage` sur `global` | tous les codes, tous les usages, toutes les demandes |
| La même permission sur un **événement** | **refus**, comme si la route n'existait pas pour cette personne |
| Aucune de ces deux | refus |

Une route paramétrée par un identifiant — `{id}` d'un code, d'un usage, d'une demande — se refuse
donc **avant de lire quoi que ce soit** : un administrateur d'événement qui forge l'adresse d'un
code reçoit le même refus que pour un identifiant inexistant, sans que la forme de la réponse les
distingue (principe IX). C'est SC-008, et c'est le test « URL forgée » exigé par le principe X.

**Le menu du back-office suit la même règle** : sans la permission sur la portée globale, aucune
entrée « Guide Négo » n'apparaît. Un menu qui l'afficherait pour la faire refuser ensuite dirait à
la personne qu'il existe quelque chose qu'elle ne peut pas voir.

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
