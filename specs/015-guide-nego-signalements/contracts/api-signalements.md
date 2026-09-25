# Contrat — signalements, validation, réglages de notification

Famille `/negotiation`. Formes TypeScript dans `frontend/app/types/negotiation-reports.ts` ; chemins
engendrés par `make openapi`. Codes au format du catalogue de `kernel/src/error.rs`.

## Signaler — accès négociateur (`negotiation.space.access`, portée globale)

### `POST /negotiation/reports`

```ts
type ReportPayload = {
  client_ref: string                      // UUID posé par le téléphone
  edition: string                         // slug de la COP
  reason: 'cancelled' | 'time' | 'venue' | 'other' | 'unannounced'
  session_id?: string                     // requis sauf 'unannounced'
  proposed_start?: string                 // 'time', et début d'une 'unannounced'
  proposed_venue?: string                 // 'venue', et « Où »
  what?: string                           // requis pour 'unannounced'
  day?: string                            // AAAA-MM-JJ, requis pour 'unannounced' → proposed_day
  theme?: string | null                   // 'unannounced'
  detail?: string                         // ≤ 600
}
```

- `201` → `MyReport` ; **rejoué avec le même `client_ref` → `200` et le même `MyReport`** (R5).
- `403 NEGOTIATION_REPORT_FORBIDDEN` sans accès négociateur ; `401` sans compte.
- `409 NEGOTIATION_REPORT_DUPLICATE` : un signalement de cette personne attend déjà sur cette session.
- `404 NEGOTIATION_SESSION_UNKNOWN` ; `400 NEGOTIATION_REPORT_INVALID` (champ requis manquant, précision trop longue — le message nomme le champ).

### `GET /negotiation/me/reports?edition=`

```ts
type MyReport = {
  id: string; client_ref: string; reason: ReportReason
  session: { id: string; title_en: string; title_fr: string | null; start_at: string; venue: string | null } | null
  what: string | null; proposed_start: string | null; proposed_venue: string | null; day: string | null
  detail: string | null
  status: 'submitted' | 'validated' | 'rejected'   // 'validated' seulement une fois publié (R3) ; avant, 'submitted'
  submitted_at: string; decided_at: string | null
  reject_reason: 'source_maintains' | 'already_known' | 'not_precise' | null
  reject_detail: string | null
}
type MyReports = { reports: MyReport[] }   // plus récent d'abord ; ETag/304
```

## Ce que lit tout le monde — dans `GET /negotiation/sessions` (3a, étendu)

```ts
// OfficialSession gagne :
network_reports: {                        // publiés, non retirés, session non terminée
  reason: 'cancelled' | 'time' | 'venue' | 'other'
  proposed_start: string | null; proposed_venue: string | null
  detail: string | null; validated_at: string
}[]
// OfficialSessions gagne, servi MÊME quand state = 'cut' (FR-022) :
network_meetings: {
  id: string; title: string; venue: string | null; start_at: string | null; day: string
  theme: string | null; validated_at: string
}[]                                       // publiées, non retirées, jour non passé
```

**Jamais de nom d'auteur** dans ces formes (SC-008).

## Valider — `negotiation.report.validate`, portée globale

Sous **`/admin/negotiation/reports`**, montées dans `admin_routes()` comme tout le back-office de
Guide Négo (même si l'écran vit dans l'application).

### `GET /admin/negotiation/reports?edition=`

```ts
type ReportQueueItem = MyReport & {
  author: { name: string; country: string | null }
  source_now: { status: string; start_at: string; end_at: string | null; venue: string | null; read_at: string | null } | null
  decided_by: string | null            // nom du décideur — l'historique (FR-014), lu ici seulement
  published_at: string | null
}
type ReportQueue = { pending: ReportQueueItem[]; decided_today: ReportQueueItem[] }  // pending : plus anciens d'abord
```

### `POST /admin/negotiation/reports/{id}/validate`

`200` → l'élément, `status: 'validated'`, `published_at: null`. Écrit `decided_by`, `decided_at`,
`source_snapshot` ; pose `negotiation.report.publish` à `now() + 30 s` (horloge de la base, R3). Rien
n'est public avant la publication. `409 NEGOTIATION_REPORT_ALREADY_DECIDED` si déjà tranché.

### `POST /admin/negotiation/reports/{id}/undo`

Tant que **rien n'est publié** (`published_at` nul) : repasse à `submitted` et efface la décision —
aucune borne de temps côté serveur. Publié, ou heurtant un nouveau signalement en attente de la même
autrice : `409 NEGOTIATION_REPORT_UNDO_EXPIRED`.

### `POST /admin/negotiation/reports/{id}/reject`

Corps `{ reason: 'source_maintains' | 'already_known' | 'not_precise', detail?: string }`. `200`. Émet
`negotiation.report.decided` pour l'autrice.

### `POST /admin/negotiation/reports/{id}/withdraw`

Retire un encart validé ou une réunion non annoncée (`withdrawal = 'admin'`). `200`.

## Réglages — compte connecté

- `GET /negotiation/me/themes` rend désormais `notify: string[]` (thématiques allumées).
- `PUT /negotiation/me/themes/notifications` — `{ codes: string[] }` parmi les thématiques suivies ;
  `400 NEGOTIATION_THEME_UNKNOWN` sinon. Idempotent.
- `GET /negotiation/me/notifications` → `{ email: boolean, version: string }` (aucune ligne = `true`).
- `PUT /negotiation/me/notifications` — `{ email: boolean }` : écrit une ligne dans `identity.consents`
  (`guide_nego_notifications`, version servie). Idempotent.

## Agenda — réunions non annoncées (3a, étendu)

- `PUT /negotiation/me/agenda/network/{id}` `{ remind }` et `DELETE /negotiation/me/agenda/network/{id}`,
  idempotents, sur `network_agenda_entries` ; `404 NEGOTIATION_SESSION_UNKNOWN` si la réunion n'est pas publiée.
- `GET /negotiation/me/agenda` gagne `network_entries: { network_meeting_id, remind, added_at }[]`.

## `GET /negotiation/me/access` (0b, étendu)

Gagne `can_validate_reports: boolean` et `reports_to_review: number | null` (nul sans la permission) (R6).

## Codes ajoutés

`NEGOTIATION_REPORT_FORBIDDEN` (403) · `NEGOTIATION_REPORT_DUPLICATE` (409) · `NEGOTIATION_REPORT_INVALID`
(400) · `NEGOTIATION_REPORT_UNKNOWN` (404) · `NEGOTIATION_REPORT_ALREADY_DECIDED` (409) ·
`NEGOTIATION_REPORT_UNDO_EXPIRED` (409).
