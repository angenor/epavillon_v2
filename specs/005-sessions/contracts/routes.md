# Contrat — Routes

**Fonctionnalité** : Sessions (B5) · **Date** : 2026-08-21

> **Ce document ne définit aucune forme de réponse.** Les formes vivent dans `frontend/app/types/` et n'y ont qu'une seule source. On indique ici, pour chaque route : le verbe, le chemin, l'autorisation exigée, le type de requête et de réponse **par son nom TypeScript**, et la politique de statut.
>
> La documentation OpenAPI est **engendrée depuis le code**. Ce fichier est la carte, pas la documentation.

---

## Préfixe, transport et politique de statut

Rien ne change depuis B1 à B4 : préfixe `/api`, `Accept-Language` sur chaque requête, `X-Request-Id` sur chaque réponse, session par cookies, vérification de l'origine sur toute écriture, en-têtes CORS posés. Les chemins sont donnés **tels que le front les écrit**, sans le préfixe.

**La règle de statut est celle de B1** : un refus **exprimé par le contrat** sort en **200** avec son discriminant ; un refus **non exprimé** sort en statut d'erreur avec un corps d'erreur ([`errors.md`](errors.md)).

**Les paramètres que le front passe encore et que l'API ignore** : le périmètre d'administration passé en argument, et l'identifiant de personne dans `setTracks`. **L'API lit sa propre session et remonte elle-même à l'édition de la séance.** Ils disparaîtront au raccordement (B7), comme en B1 à B4.

---

## Les deux permissions consommées

| Permission | Ce qu'elle garde ici | Portée exigée |
|---|---|---|
| `programme.session.schedule` | l'écran du planificateur, la liste des séances, les conflits, et les trois écritures | l'édition de la séance, ou globale |
| `programme.registration.manage` | la liste **nominative** des inscrits, et l'annulation faite par l'administration | l'édition de la séance, ou globale |

**Détenir l'une n'accorde pas l'autre, et le modèle en tire une conséquence** : le rôle de programmation détient la première et **pas** la seconde (écart n° 119). Une chargée de programmation compose la grille sans pouvoir ouvrir la liste des inscrits. C'est une ligne de la table des droits, pas une fatalité — l'écran de gestion des rôles permet de l'accorder sans toucher au code.

**L'écart n° 56 reste ouvert** : aucune permission ne distingue « composer la grille » de « publier le programme ». La publication (B3) est gardée par la même permission que la composition.

---

## Les dix-sept routes

### Lecture publique — aucune session exigée

| # | Verbe | Chemin | Requête | Réponse | Notes |
|---|---|---|---|---|---|
| 1 | GET | `/schedule?event_id=` | — | `PublicScheduleRow[]` | `programme.v_public_schedule`, telle quelle. Vide — jamais une erreur — quand le programme n'est pas paru |
| 2 | GET | `/events/{event_id}/sessions/{slug}` | — | `{ session, speakers, organizations }` | Séance **publiée** seulement. Une adresse inconnue et une séance non publiée rendent **le même** 404 |
| 3 | GET | `/sessions/{id}/registration-form` | — | `{ form, fields }` | Le formulaire **applicable** : séance, à défaut édition, à défaut plateforme. Champs **actifs** seulement, options de taxonomie résolues |

### Le planificateur — permission de planifier, périmètre exigé

| # | Verbe | Chemin | Requête | Réponse | Notes |
|---|---|---|---|---|---|
| 4 | GET | `/admin/planner?event_id=` | — | `PlannerScreen` | **Tout l'écran en une réponse, conflits compris.** Lu dans une transaction en lecture seule, sur une connexion (R10) |
| 5 | GET | `/sessions?event_id=` | — | `PlannerSession[]` | Les séances de l'édition, placées ou non |
| 6 | GET | `/sessions/conflicts?event_id=` | — | `ScheduleConflict[]` | `programme.detect_conflicts()`, telle quelle |
| 7 | PUT | `/sessions/{id}/schedule` | `ScheduleSessionPayload` | `PlannerMutationResult` | **Placer, déplacer, redimensionner, retirer — une seule écriture.** Jamais refusée pour chevauchement |
| 8 | PUT | `/sessions/{id}/tracks` | `SessionTracksPayload` | `PlannerMutationResult` | La liste **remplace** la précédente |
| 9 | PUT | `/sessions/{id}/broadcast` | `SessionBroadcastPayload` | `PlannerMutationResult` | Le canal **est** saisissable (R8) |
| 10 | GET | `/sessions/{id}/speakers` | — | `SessionSpeaker[]` | |
| 11 | GET | `/sessions/{id}/organizations` | — | `SessionOrganization[]` | |
| 12 | GET | `/sessions/{id}/tracks` | — | `SessionTrack[]` | |

**Les trois écritures rendent la séance ET les conflits de toute l'édition**, lus dans la transaction, après l'écriture (R11).

### Inscriptions

| # | Verbe | Chemin | Autorisation | Requête | Réponse |
|---|---|---|---|---|---|
| 13 | POST | `/sessions/{id}/registrations` | session, **ou** aucune si le formulaire admet l'anonyme | `RegisterPayload` | `RegistrationResult` |
| 14 | GET | `/registrations?session_id=` | `programme.registration.manage` sur l'édition | — | `RegistrationRow[]` — **nominative** |
| 15 | GET | `/registrations/mine` | session | — | `Registration[]` |
| 16 | POST | `/registrations/{id}/cancel` | l'inscrit lui-même, **ou** `programme.registration.manage` sur l'édition | `{ reason? }` | `CancelRegistrationResult` |
| 17 | POST | `/registrations/{id}/join` | l'inscrit lui-même | — | `{ joined_at }` |

---

## Les formes que ce jalon **ajoute** au contrat du front

Aucun écran d'inscription n'existe : ces quatre formes sont posées ici et n'ont donc rien à contredire. Elles suivent les conventions du dépôt — champs en `snake_case`, refus prévus en membres d'union.

```ts
/** POST /sessions/{id}/registrations */
interface RegisterPayload {
  /** Clés = `code` des champs ACTIFS du formulaire applicable. Une clé inconnue est refusée. */
  answers: Record<string, unknown>
  /** Langue des envois ultérieurs ; défaut : la langue négociée de la requête. */
  locale?: string
  /** Identité, UNIQUEMENT sans session et si le formulaire admet l'anonyme.
   *  Jamais prise dans `answers` : les codes de champs sont renommables (R23). */
  guest?: { email: Email; first_name: string; last_name: string; civility?: string | null }
  /** Exigé dès qu'une réponse est donnée à un champ marqué sensible. */
  sensitive_data_consent?: boolean
}

/** Les six issues d'une tentative bien formée. */
type RegistrationResult =
  | { status: 'registered';         registration: Registration }
  | { status: 'waitlisted';         registration: Registration; position: number }
  | { status: 'already_registered'; registration: Registration }
  | { status: 'full';               capacity: number }
  | { status: 'closed';             closed_at: IsoDateTime }
  | { status: 'not_open_yet';       opens_at: IsoDateTime }

/** POST /registrations/{id}/cancel */
interface CancelRegistrationResult {
  registration: Registration
  /** Personnes promues depuis la liste d'attente : 0 ou 1 (R20). */
  promoted: number
}

/** Une ligne de la liste NOMINATIVE du back-office. */
interface RegistrationRow {
  registration: Registration
  person: Person
  /** Nom de l'organisation liée, quand il y en a une. */
  organization_name: string | null
}
```

---

## Ce qui n'est **pas** servi, et pourquoi

| Chemin déclaré par le front | Décision |
|---|---|
| `/sessions/publication-readiness` | **Non servi** (écart n° 121). B3 rend la même réponse sous `/admin/planner/readiness`, aucun écran n'appelle le premier, et deux chemins pour une même lecture dans deux modules divergeraient. À retirer du front en B7 |

| Ce que le modèle porte et que ce jalon ne livre pas | Motif |
|---|---|
| Créer une séance sans dossier | Aucun écran, absent du prompt. La colonne reste facultative |
| Annuler ou reporter une séance | Aucun écran ; les états existent et la vue publique les rend déjà |
| Écrire le compte rendu d'une séance | Aucun écran ne l'écrit (écart n° 122). L'**action** « compte rendu manquant » est servie |
| Les questions du public | Trois tables, aucun écran, absent du prompt |
| Composer un formulaire d'inscription | Aucun back-office de formulaire n'existe. Les formulaires sont **lus** |

---

## Ce que ce jalon **remplit** sans ajouter de route

| Route livrée par B4 | Ce qui change |
|---|---|
| `GET /organizations/{id}/workspace` | Le bloc « ce qui attend une action » gagne l'action **compte rendu manquant** |
| `GET /proposals/{id}/file` | `tracking.sessions` cesse d'être vide : chaque séance avec sa salle et **trois nombres**. `reminders` reste vide jusqu'à B6 (écart n° 108) |

---

## Montage — et le préfixe partagé que B3 a préparé

```rust
// api/src/lib.rs — le scope /admin/planner est composé UNE fois, à partir des DEUX modules.
// Deux `web::scope` du même préfixe ne se complètent pas : Actix retient le premier
// et rend 404 sur les routes du second. Le défaut a coûté trois routes muettes en B2.
if evenements || propositions {
    portee = portee.service(web::scope("/admin/planner").configure(move |cfg| {
        if evenements   { event::planner_routes(cfg); }      // readiness, publish  (B3)
        if propositions { programme::planner_routes(cfg); }  // l'écran             (B5)
    }));
}
```

Les scopes `/sessions` et `/registrations` n'appartiennent qu'à ce module : rien à composer.

**Chemins littéraux avant chemins paramétrés.** `/sessions/conflicts` et `/registrations/mine` sont déclarés avant leurs homologues à identifiant. Le risque de capture n'existe pas ici — les segments ne coïncident pas —, mais l'ordre est tenu par la structure plutôt que par la vigilance, comme en B4.

**Un test frappe les dix-sept routes sur la vraie application**, comme en B2, B3 et B4 : c'est le seul contrôle qui voit une route écrite mais non montée, et il a déjà attrapé ce défaut deux fois.
