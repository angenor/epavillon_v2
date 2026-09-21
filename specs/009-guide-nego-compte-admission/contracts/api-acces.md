# Contrat — l'accès, côté application

> Routes du crate `negotiation`, sous `/api/negotiation`. Toutes exigent une session ; aucune n'est
> publique. Le contrat réel est engendré par `make openapi`.

## `GET /api/negotiation/me/access`

**La seule route que lisent le parcours d'entrée, le verrou et « Mon accès ».** Une lecture, un écran.

```jsonc
{
  "admission_mode": "code",                 // "code" | "approval" | "code_and_approval"
  "state": "granted",                       // "visitor" | "pending" | "granted" | "rejected" | "revoked"
  "granted": {                              // présent si state = granted
    "scope": { "type": "negotiation_space", "id": "…", "name": "COP31 — Climat" },
    // ou   { "type": "global", "id": null, "name": null }  →  tout Guide Négo
    "granted_at": "2026-11-08T10:12:00Z",
    "source_code_label": "Réseau des négociatrices — COP31"
  },
  "networks": [                             // vide si aucune appartenance
    { "code": "women_negotiators", "label": "Réseau des négociatrices francophones" }
  ],
  "request": {                              // présent si une demande existe
    "id": "…", "status": "pending",
    "submitted_at": "2026-11-12T09:20:00Z",
    "decided_at": null, "decision_reason": null
  }
}
```

- Réponse **`ETag`** + `If-None-Match` : c'est ce qui la rend lisible hors connexion par
  `useGnLecture`, avec l'heure de sa lecture (principe XI). **L'empreinte est calculée sur le contenu
  rendu**, et `?since=` a été écarté en construisant (21/09) : l'état vient de quatre tables et de
  `now()`, aucune colonne ne porte l'instant où il a changé, et un `304` fautif laisserait ouverts,
  sur le téléphone, les modules d'un accès retiré — exactement ce qu'ADR-006 interdit. Une empreinte
  de contenu, elle, ne peut pas se tromper.
- `state` est **dérivé du RBAC**, jamais d'une colonne d'état : `identity.has_permission` fait foi.
- `networks` ne commande aucun droit à cette étape (FR-014).
- **Rien dans cette réponse ne nomme ni ne suppose un genre** (SC-006).

## `POST /api/negotiation/invitation-codes/redeem`

```jsonc
// requête
{ "code": "nego-024", "device_id": "9f2c…" }
```

`device_id` est **une information, pas une borne** : il est enregistré avec l'essai pour qu'un
administrateur puisse lire une série d'échecs, mais **le comptage se fait par personne**. Il vient du
corps de la requête, donc il se forge : borner la fenêtre « par personne et par appareil » suffirait à
changer d'identifiant pour repartir de zéro.

Réponse **200 dans tous les cas prévus par le parcours** — voir la recherche, R3 :

```jsonc
{
  "issue": "revoked",
  "message": "Ce code a été révoqué le 8 novembre. Le réseau en a reçu un nouveau dans son groupe WhatsApp.",
  "revoked_at": "2026-11-08T00:00:00Z"
}
```

| `issue` | Quand | Ce que porte la réponse en plus |
|---|---|---|
| `accepted` | code juste, mode `code` | `granted`, `networks` — l'accès est ouvert |
| `pending_approval` | code juste, mode `code_and_approval` | `request` — une demande est ouverte, portant ce code |
| `already_granted` | la personne avait déjà l'accès | `granted`, `networks` — l'appartenance au réseau a pu s'ajouter (FR-018) |
| `unknown` | aucun code ne correspond | — |
| `revoked` | code révoqué | `revoked_at` |
| `exhausted` | quota atteint | — |
| `expired` | validité passée | — |
| `not_yet_valid` | validité pas encore ouverte | — |
| `throttled` | trop d'essais **de cette personne**, tous appareils confondus | `retry_after_seconds` |

**Le `message` est composé par l'API et s'affiche tel quel** (FR-020) : elle seule connaît la date de
révocation et le temps d'attente. Les titres, aides et boutons autour restent de l'i18n.

Le code est comparé **insensiblement à la casse et aux séparateurs** (FR-010) : `nego-024`, `NEGO 024`
et `Nego024` désignent le même code. Il fait **huit caractères, tirets compris** — l'exemple `NEGO-24`
de la maquette en porte sept et se corrige.

Vraies erreurs, avec leur code stable : session absente, adresse non vérifiée, compte suspendu, corps
malformé.

**Le quota est tenu par la base, pas par une lecture préalable.** `ck_invitation_codes_quota` et le
verrou de ligne pris par l'incrément font qu'un code de 120 usages en accorde 120, jamais 121 — même
si deux personnes entrent à la même seconde sur le dernier usage. L'API **traduit** l'échec de la
contrainte en `exhausted` (principe VIII) ; elle ne le prévient pas par un `SELECT` que la seconde
requête contournerait.

## `POST /api/negotiation/access-requests`

```jsonc
{ "space_id": "…", "message": "Je participe à l'atelier préparatoire de Dakar." }
```

`space_id` absent vaut une demande de portée globale. Rend la demande créée. Une demande déjà en
attente sort en erreur `NEGOTIATION_ACCESS_REQUEST_PENDING` — **l'unicité vient de l'index partiel de
la base**, le code traduit le conflit, il ne le prévient pas par une lecture préalable (principe VIII).

## `DELETE /api/negotiation/access-requests/{id}`

La personne retire sa demande — c'est ce qui se passe quand elle reçoit un code et entre par lui. La
demande passe à `cancelled`, qui se dit **« annulée »** à l'écran ; « révoquée » est réservé à un
accès retiré. Une demande déjà tranchée sort en `NEGOTIATION_ACCESS_REQUEST_DECIDED`.

## Événements émis

| Geste | Événement |
|---|---|
| Demande envoyée | `negotiation.access_request.submitted` |
| Demande admise / refusée | `negotiation.access_request.approved` / `.rejected` |
| Accès accordé / retiré | `negotiation.space_access.granted` / `.revoked` |
| Code révoqué | `negotiation.invitation_code.revoked` |

Tous par `platform.emit_event()`, **dans la transaction du changement d'état**. Les deux décisions
mettent en outre le courriel en file par `jobs::enqueue`, dans la même transaction — recherche R6.

---

## Une règle que l'étape 1 devra tenir

Quand `GET /api/negotiation/me/access` revient avec `state = "revoked"` — ou retombe à `visitor` —,
**les contenus réservés gardés sur le téléphone s'effacent**. C'est la même règle que la déconnexion,
déjà inscrite à `01-stack.md` : « Documents réservés — effacés du téléphone à la déconnexion ».

Rien à construire ici : 0b n'a aucun contenu réservé à effacer. La règle est posée pour que l'étape 1,
qui introduit les documents réservés et leur téléchargement, l'applique **au retrait d'accès** et pas
seulement à la déconnexion. Un accès retiré qui laisserait les documents lisibles hors connexion
viderait le retrait de son sens.
