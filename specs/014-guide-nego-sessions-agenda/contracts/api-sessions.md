# Contrat — sessions de négociation, groupes et agenda (application)

Famille `/negotiation` (ADR-008), à part de `/sessions` et `/schedule`, qui servent le Pavillon.
Formes nommées en TypeScript dans `frontend/app/types/negotiation-sessions.ts` ; chemins engendrés
par `make openapi`. Toute heure est un instant ISO 8601 en UTC ; le fuseau de la COP voyage à part.

## `GET /negotiation/sessions?edition={slug}` — publique

Toute la COP en une réponse — **toutes les fiches se lisent hors connexion dès la première lecture**
([R9](../research.md)). `ETag` sur l'empreinte du corps ; `If-None-Match` → `304`.

```ts
type OfficialSessions = {
  edition: { slug: string; timezone: string; city: string | null }
  official_programme_url: string
  state: 'serving' | 'cut'
  cut_reason: 'disabled' | 'unreachable' | null   // disabled : import éteint ou jamais lu
  failing_since: string | null                     // « n'a pas répondu depuis 06:40 »
  read_at: string | null                           // dernière lecture réussie
  server_time: string
  sessions: OfficialSession[]                      // [] quand state = 'cut'
}

type OfficialSession = {
  id: string
  title_en: string
  title_fr: string | null            // traduction automatique ; null = anglais seul
  start_at: string
  end_at: string | null
  venue: string | null
  previous: { start_at: string; end_at: string | null; venue: string | null; changed_at: string } | null
  type: { code: string; label: I18nText; term_en: string } | null
  group: { code: string; label: I18nText } | null   // coordination seulement
  theme: string | null                // code negotiation_theme ; null = « Thématique non précisée »
  agenda_item: { code: string; title: string } | null
  open_access: boolean | null
  status: 'scheduled' | 'cancelled'
  cancelled: { at: string; reason: 'source' | 'postponed' | 'removed' } | null
  source_url: string | null
  read_at: string                     // dernière lecture où elle figurait
}
```

- `previous` n'existe que si l'heure ou la salle a changé : c'est « Déplacée », et la valeur barrée.
- **Coupé** : `sessions` est vide, quel que soit l'âge des lignes — jamais une liste périmée
  (principe XII). Le client efface alors sa copie gardée de la liste (FR-039).
- `edition` inconnue → `404 NEGOTIATION_EDITION_UNKNOWN`.

## `GET /negotiation/me/groups` · `PUT /negotiation/me/groups` — connectée

Même patron que `/negotiation/me/themes` (étape 0c) : remplacement en bloc, empreinte sur les codes
triés, `If-Match` exigé, `412 NEGOTIATION_GROUPS_STALE` si l'état a changé ailleurs.

```ts
type MyGroups = { groups: string[]; etag: string }
// PUT { groups: string[] }  — liste vide permise : « aucun groupe » est un choix
```

`400 NEGOTIATION_GROUP_UNKNOWN` si un code n'est pas dans `negotiation_group` (même statut que `NEGOTIATION_THEME_UNKNOWN`).

## `GET /negotiation/me/agenda` — connectée

```ts
type MyAgenda = { entries: { session_id: string; remind: boolean; added_at: string }[] }
```

Les lignes elles-mêmes se lisent dans `OfficialSessions` : l'agenda ne recopie pas la session.
`ETag` / `304`.

## `PUT /negotiation/me/agenda/{session_id}` — connectée

Corps `{ remind: boolean }`. **Idempotent** : ajoute, ou change le rappel. `204`.
- session inconnue ou non importée → `404 NEGOTIATION_SESSION_UNKNOWN` ;
- session annulée et absente de l'agenda → `409 NEGOTIATION_SESSION_CANCELLED` ; déjà dans l'agenda,
  le `PUT` passe (désarmer un rappel reste possible) ;
- `remind: true` sur une session annulée → le rappel est enregistré désarmé (FR-035).
- `GET /me/agenda` rend `remind` **effectif** : faux dès que la session est annulée, quel que soit ce
  qui est enregistré — une annulation par l'import désarme sans réécrire la ligne.

## `DELETE /negotiation/me/agenda/{session_id}` — connectée

Idempotent : `204` même si la ligne n'existait pas.

## Codes stables ajoutés au catalogue (`kernel/src/error.rs`, format existant)

`NEGOTIATION_EDITION_UNKNOWN` (404) · `NEGOTIATION_SESSION_UNKNOWN` (404) ·
`NEGOTIATION_SESSION_CANCELLED` (409) · `NEGOTIATION_GROUP_UNKNOWN` (400) · `NEGOTIATION_GROUPS_STALE` (412) ·
`NEGOTIATION_IMPORT_CONFIG_INVALID` (400, back-office) · `NEGOTIATION_AGENDA_ITEM_UNKNOWN` (404, back-office).
