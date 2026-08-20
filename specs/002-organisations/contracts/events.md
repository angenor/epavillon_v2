# Contrat — Événements et travaux différés

**Fonctionnalité** : Organisations (B2) · **Date** : 2026-08-20

Principe IV : **les effets de bord inter-modules passent par `platform.emit_event()`, appelée dans la même transaction que le changement d'état.** Jamais d'insertion à la main dans l'outbox, jamais d'appel direct d'un module à un autre.

---

## 1. Les six événements émis par `org`

Agrégats : `org.organization` et `org.membership`. Le type respecte la forme à trois segments imposée par `ck_outbox_event_type_format`.

| Type | Agrégat | Charge utile | Quand |
|---|---|---|---|
| `org.organization.created` | organisation | identifiant, statut, pays, type, **nombre de fiches proches montrées avant création** | Une fiche naît, en attente de rapprochement |
| `org.organization.verified` | organisation | identifiant, instant | Le sceau est posé — et la fiche est admise du même geste |
| `org.organization.unverified` | organisation | identifiant | Le sceau est retiré. **Deux événements et non un seul portant un booléen** : un événement nommé « vérifiée » portant « non » est un mensonge que personne ne relit correctement |
| `org.membership.requested` | adhésion | organisation, personne, **rattachement automatique oui/non** | Une personne demande à rejoindre, ou est rattachée d'office |
| `org.membership.approved` | adhésion | organisation, personne, rôle | Un référent approuve, ou une invitation est acceptée |
| `org.membership.revoked` | adhésion | organisation, personne, **motif : refus, retrait, départ** | Un refus, un retrait, un départ |

Une invitation émise **n'a pas d'événement de domaine à elle** : elle produit une adhésion en attente, annoncée par `org.membership.requested` avec la direction dans sa charge utile. Ajouter un septième type pour un état déjà décrit ferait deux vérités.

**Deux de ces types portent le nom exact d'un type de notification déjà semé** dans `110_engagement.sql` § 11 — `org.membership.requested` et `org.membership.approved`. Ce n'est pas une coïncidence à défaire : le modèle avait prévu que ce module annonce ces deux faits-là, et B6 y branchera ses modèles de message sans rien renommer.

### `org.organization.merged` est émis par la base — ne pas l'émettre une seconde fois

`org.merge_organizations()` appelle **elle-même** `platform.emit_event()` avant de rendre la main, et marque **elle-même** la paire de la file des doublons.

**C'est le piège n° 1 du module `identity`, répété à l'identique.** Un service qui émettrait l'événement après l'appel en écrirait deux, **sans qu'aucune erreur ne le signale** : l'outbox accepte les deux, et un consommateur idempotent traiterait la première ligne puis ignorerait la mauvaise. Le défaut ne se voit qu'en relisant l'outbox d'un agrégat qui aurait deux fois la même histoire.

Le service de fusion **n'émet rien et ne marque rien**, et le dit à l'endroit où l'on serait tenté d'ajouter la ligne. Le test `outbox_une_seule_fusion` **compte** les événements ; il ne se contente pas de vérifier leur présence.

La règle générale, apprise deux fois maintenant : **avant d'émettre après un appel de fonction SQL, lire la fonction.**

### Ce que les charges utiles ne portent jamais

Ni jeton, ni adresse électronique — sauf quand l'adresse **est** le sujet, ce qui n'arrive dans aucun des six. `platform.outbox_events` est durable, indexée par agrégat, faite pour être relue et rejouée : ce qu'on y dépose est là pour longtemps et pour beaucoup de monde.

Le jeton d'une invitation ne franchit donc **pas** l'outbox : il vit dans la charge utile du travail d'envoi, née dans la transaction du changement d'état et vidée dès l'envoi réussi — la règle de B1 (§ R8), appliquée telle quelle.

---

## 2. Les consommateurs

**Ce module n'en enregistre aucun.** Les trois travaux de fond sont mis en file **dans la transaction qui les rend nécessaires**, ce qui est plus simple et plus sûr qu'un consommateur d'événement : si la transaction est annulée, le travail ne naît pas. Un consommateur ne se justifierait que pour un effet appartenant à un **autre** module — il n'y en a aucun dans ce jalon.

Le jour où B6 enverra les courriels d'adhésion depuis `engagement`, ce sera par un consommateur des six événements ci-dessus, et rien de ce module n'aura à changer.

---

## 3. Les travaux différés

| Tâche | Déclenchement | Clé d'unicité | Ce qu'elle fait |
|---|---|---|---|
| `org.duplicates.scan` | Récurrente, une passe par jour, **par tranches** | jour + curseur | Balaie les fiches vivantes, consigne les paires suspectes au-dessus du seuil, **sans ressusciter une paire arbitrée** (R11). Chaque tranche pose la suivante ; la dernière planifie le lendemain. Le démarrage du worker ne fait que **réarmer** la chaîne, au cas où sa dernière occurrence serait morte avant d'avoir posé la suivante — le motif de la purge des jetons de B1 |
| `org.trust_score.recompute` | Création, modification, sceau, domaine vérifié, adhésion activée ou révoquée, fusion | **organisation** | Recalcule et n'écrit **que si la valeur change** (R12). Cent adhésions approuvées coup sur coup produisent un recalcul. **Sans acteur** : personne ne l'a demandé |
| `org.scorecard.refresh` | Les mêmes écritures, plus la fusion | fenêtre de quelques minutes | Rafraîchit la projection analytique **en concurrence**, donc hors transaction (R13) |
| `org.membership.invitation_email` | Invitation émise | adhésion + jour | Compose et remet le message au serveur du site, par le contrat d'envoi du noyau. **Porte le jeton en clair**, et cette charge utile est vidée dès l'envoi réussi |
| `org.membership.request_email` | Demande spontanée reçue | adhésion + jour | Prévient les référents de l'organisation |
| `org.membership.approved_email` | Adhésion approuvée ou invitation acceptée | adhésion + jour | Prévient la personne |

**Les trois courriels empruntent la chaîne éprouvée en B1** — file de travaux, worker, route privée du site, SMTP. Ils sont volontairement simples : la composition multilingue riche, les préférences de canal et le suivi des envois appartiennent au module Engagement (B6), qui n'existe pas encore. Le seul point d'attention est celui que B1 a payé : **l'identifiant du message se réserve avant l'envoi**, pas après — le doublon réel est concurrent, pas séquentiel.

**Le travail de balayage déclare qu'il ne transporte aucun secret**, les trois courriels déclarent qu'ils en transportent un : c'est le mécanisme que B1 a posé pour que la charge utile d'un travail mort soit effacée sans coûter leur diagnostic aux tâches qui n'ont rien à cacher.

---

## 4. L'ordre des écritures d'une fusion, vu depuis l'outbox

C'est le seul endroit du module où l'ordre compte, et il n'est pas celui qu'on croit :

```text
  transaction ouverte par la porte d'écriture du noyau (acteur posé)
    │
    ├─ contrôles : permission globale, nom de confirmation, statuts
    │
    ├─ org.merge_organizations(source, cible, motif)
    │     └─ réaffecte, passe la source en `merged`, écrit le journal,
    │        marque la paire, ET ÉMET org.organization.merged
    │
    ├─ arbitrages de champ sur la fiche SURVIVANTE      ← APRÈS, jamais avant :
    │     └─ le trigger de dénominations s'exécute        tant que la source est
    │                                                     vivante, l'unicité du nom
    ├─ relecture de rows_reassigned dans le journal       interdit de reprendre
    │                                                     le sien (R5)
    └─ COMMIT
         puis mise en file : recalcul de score, rafraîchissement de la projection
```

Si l'arbitrage échoue, la fusion est annulée avec lui : c'est la garantie que l'obligation d'A11 cherchait, et elle est conservée intacte — seul l'ordre change.
