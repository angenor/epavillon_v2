# Contrat — Routes

**Fonctionnalité** : Organisations (B2) · **Date** : 2026-08-20

> **Ce document ne définit aucune forme de réponse.** Les formes sont dans `frontend/app/types/` et n'y ont qu'une seule source. On indique ici, pour chaque route : le verbe, le chemin, l'autorisation exigée, le type de requête et de réponse **par son nom TypeScript**, et la politique de statut HTTP.
>
> La documentation OpenAPI est **engendrée depuis le code**. Ce fichier est la carte, pas la documentation.

---

## Préfixe, transport et politique de statut

Rien ne change depuis B1 et rien ne doit changer : préfixe `/api`, `Accept-Language` sur chaque requête, `X-Request-Id` sur chaque réponse, session par cookies, vérification de l'origine sur toute écriture, en-têtes CORS posés. Les chemins ci-dessous sont donnés **tels que le front les écrit**, sans le préfixe.

**La règle de statut est celle de B1, reprise sans changement** :

| Le refus est… | Réponse |
|---|---|
| **exprimé par le contrat** comme membre d'union | **200**, avec le discriminant `status` |
| **non exprimé** par le contrat | statut d'erreur HTTP + corps d'erreur ([`errors.md`](errors.md)) |

Six refus de ce module sont dans le contrat et sortent donc en 200 : `already_member`, `name_taken`, `already_invited`, `domain_taken`, `confirmation_mismatch`, `already_merged`.

**Les paramètres que le front passe encore et que l'API ignore.** Les données simulées transmettent `personId`, `actorId` et le périmètre d'administration parce qu'elles n'ont pas de session. **L'API lit la sienne** : ces paramètres sont ignorés, jamais lus, et disparaîtront des appels au raccordement (B7). C'est le motif « les droits déclarés par le client sont ignorés », éprouvé en B1.

---

## Les deux lectures de la recherche — l'écart n° 23, et sa raison

**Elles sont déclarées côte à côte, et c'est délibéré.** Le prompt B2 demande de « documenter la différence à l'endroit où elles sont déclarées » ; l'endroit, c'est ici et l'annotation OpenAPI de chaque gestionnaire.

| | `GET /organizations/similar` | `GET /admin/organizations/similar` |
|---|---|---|
| **La question posée** | « Ce que j'ai tapé, est-ce que ça existe déjà ? » | « Qu'est-ce qui pourrait être la même entité ? » |
| **Qui la pose** | Une personne, sur l'écran de rattachement ou de dépôt | Le back-office, et le balayage de détection |
| **Filtre** | Seules les fiches portant `name_similarity` dans leurs motifs | **Aucun** |
| **Le domaine de l'appelant** | Alimente le score, **ne fait pas entrer** une fiche sans rapport | Fait entrer la fiche : c'est le signal le plus fiable |
| **Autorisation** | Session | Permission de consultation **et** périmètre non vide |
| **Pourquoi** | Chercher « Agence spatiale du Sahel » ne doit pas ramener l'organisation du domaine de la personne, qu'un bandeau lui propose déjà nommément | Deux fiches qui déclarent `osed-sahel.org` sont la même maison, quels que soient les libellés saisis |

La lecture filtrée **sur-lit** la fonction (limite + 5) puis tronque, sinon la limite demandée ne serait pas tenue — voir [`../research.md`](../research.md) § R1.

---

## Recherche et lectures ouvertes

Session requise, et **rien de plus** : la permission de consultation des organisations est détenue par le rôle d'utilisateur ordinaire, que rien n'attribue aujourd'hui — l'exiger refuserait tout nouvel inscrit (FR-014, écart n° 74).

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `GET` | `/organizations/similar` | `name`, `country_id`, `email`, `website`, `limit` | `SimilarOrganization[]` | 200 | **Filtrée** (ci-dessus). `limit` borné : défaut 10, maximum 50. **Terme sous deux caractères → liste vide, pas une erreur** : le front ne le demande jamais, le garde existe pour qu'un appel forgé ne balaie pas la table |
| `GET` | `/organizations/by-email-domain` | `email` **ignoré** | `EmailDomainMatch \| null` | 200 | Le domaine vient de **la session** (FR-017). `null` sur messagerie grand public ou domaine inconnu |
| `GET` | `/organizations` | `limit`, `offset` | `Organization[]` | 200 | **Bornée** : défaut 50, maximum 200, triée par nom légal, fiches vivantes seulement. Seule la page de guide de style l'appelle ; livrée pour ne pas la casser |
| `GET` | `/organizations/{id}` | — | `Organization \| null` | 200 | Rend la fiche **telle quelle**, absorbée comprise : elle porte alors `merged_into_id`, et l'appelant sait quoi en faire |

---

## Rattachement et création

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `POST` | `/organizations` | `CreateOrganizationPayload` | `CreateOrganizationResult` | 200 | Fiche `candidate`, créateur **référent** actif. `name_taken` porte la fiche en cause. Une ressemblance ne bloque **jamais** |
| `POST` | `/organizations/{id}/members` | `JoinOrganizationPayload` | `JoinOrganizationResult` | 200 | Trois issues. L'organisation visée est **résolue** : rejoindre une fiche absorbée mène à la fiche vivante (FR-024) |
| `GET` | `/people/{id}/memberships` | — | `Membership[]` | 200 | **Soi-même**, ou permission de consultation des utilisateurs. Adhésions vivantes : actives et en attente |

---

## Adhésions — deux files, deux autorisations

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `POST` | `/organizations/{id}/invitations` | `InviteMemberPayload` | `InviteMemberResult` | 200 | **Référent actif de cette organisation.** Crée la personne si l'adresse est inconnue, sans compte et **sans nom déduit**. Trois issues, dont `already_invited` |
| `PUT` | `/memberships/{id}/decision` | `DecideMembershipPayload` | `Membership \| null` | 200 | **Référent actif.** Ne porte que sur une **demande** (`invited_at` nul) : sur une invitation, refus explicite. Un refus **révoque** |
| `POST` | `/organizations/invitations/accept` | `{ token }` | `{ status, membership, organization }` | 200 | **Route nouvelle, absente du front** — sans elle, une invitation ne peut pas s'honorer. **N'exige pas de session** (R10) : le jeton est la preuve d'adresse, comme pour la vérification d'adresse de B1. Si une session existe, elle doit désigner la même personne |
| `DELETE` | `/memberships/{id}` | — | `{ status: 'revoked' \| 'last_manager' }` | 200 | **Route nouvelle, absente du front.** Un référent révoque un membre, ou une personne quitte l'organisation. C'est le **seul point d'application de FR-041** : sans elle, la règle du dernier référent n'aurait aucun endroit où s'exercer |

**Pourquoi ces deux routes-là sont livrées alors qu'aucun écran ne les appelle**, quand quatre lectures ne le sont pas (écart n° 79) : celles-là étaient des **lectures** dont le contenu est servi ailleurs ; celles-ci sont des **écritures sans lesquelles une règle spécifiée n'existerait pas**. Une invitation qu'on ne peut pas accepter n'est pas une fonctionnalité, c'est une moitié de fonctionnalité.

---

## Back-office

Sauf mention contraire : **permission de consultation des organisations sur une portée quelconque, ET périmètre d'administration non vide.** Les trois cas du périmètre restent distincts — global, éditions listées, aucun droit → **refus explicite**, jamais une liste vide (FR-043).

| Verbe | Chemin | Requête | Réponse | Statut | Notes |
|---|---|---|---|---|---|
| `GET` | `/admin/organizations` | — | `OrganizationListScreen` | 200 | Filtrée par périmètre — **organisations ayant déposé ou tenu une activité** dans les éditions administrées. `scoped_to_events` dit que la liste est restreinte. Facettes comptées sur le même jeu |
| `GET` | `/admin/organizations/similar` | idem `/organizations/similar` | `SimilarOrganization[]` | 200 | **Non filtrée** — la seconde lecture de l'écart n° 23 |
| `GET` | `/admin/organizations/{id}` | — | `OrganizationDetail \| null` | 200 | Huit lectures assemblées. Une fiche **absorbée** s'ouvre normalement ; une fiche **hors périmètre** rend un refus indiscernable d'une fiche inexistante, **URL forgée comprise** |
| `PUT` | `/admin/organizations/{id}/verification` | `OrganizationVerificationPayload` | `OrganizationWriteResult` | 200 | Permission de **gestion**. Poser le sceau sur une fiche `candidate` l'**admet** ; le retirer ne change pas le statut |
| `PUT` | `/admin/organizations/{id}/domains/{domainId}` | `DomainVerificationPayload` | `OrganizationWriteResult` | 200 | Permission de gestion. `domain_taken` **nomme** la fiche qui détient le domaine vérifié |
| `PUT` | `/admin/organizations/{id}/names/{nameId}` | `NameConfirmationPayload` | `OrganizationWriteResult` | 200 | Permission de gestion. Une dénomination **posée par la base** ne se retire pas |
| `GET` | `/admin/organizations/duplicates` | — | `DuplicateQueueScreen` | 200 | **Permission de fusion, portée GLOBALE.** La file n'est pas filtrée par périmètre, et ce n'est pas un oubli : une paire ne relève d'aucune édition, et sa résolution exige de toute façon la portée globale. Un administrateur détaché n'y accède **pas du tout**, plutôt que d'en voir une part |
| `PUT` | `/admin/organizations/duplicates/{pairId}` | `DuplicateDecisionPayload` | `DuplicateDecisionResult` | 200 | Idem. `distinct` retire la paire pour de bon, `deferred` la remet à plus tard |
| `GET` | `/admin/organizations/{id}/merge-preview` | `target_id`, `pair_id` | `MergePreview \| null` | 200 | Idem. Calculé **pour un sens**, recalculé à l'inversion : le décompte n'est pas symétrique. `null` si l'une des fiches est introuvable ou déjà absorbée |
| `POST` | `/admin/organizations/merge` | `MergePayload` | `MergeResult` | 200 | Idem. Quatre issues, dont `confirmation_mismatch` et `already_merged`. Un choix portant sur l'**adresse d'URL** est refusé en 422, champ nommé (R6) |

---

## Ce que ces routes ne rendent jamais

- **Le score brut d'une fiche recalculé à la volée** : la liste rend celui de la table, le travail différé le tient à jour.
- **Une liste vide en guise de refus** : les trois cas du périmètre restent distincts jusqu'au bout.
- **L'existence d'une fiche hors périmètre** : le refus est indiscernable de « inexistante ».
- **Le domaine d'une adresse qui n'est pas celle de l'appelant** : le paramètre est ignoré.
- **Une liste nominative de membres à qui n'a pas de raison de la voir** : la fiche du back-office la rend, la recherche ouverte n'en rend que le **nombre**.
