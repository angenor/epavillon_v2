# Contrat — réunions de la Francophonie

Formes TS dans `frontend/app/types/negotiation-meetings.ts` ; chemins par `make openapi`.

## `GET /negotiation/meetings?edition={slug}` — publique

```ts
type FrancophoneMeetings = {
  edition: { slug: string; timezone: string; city: string | null }
  read_at: string; server_time: string
  meetings: FrancophoneMeeting[]            // publiées ; brouillons jamais
}
type FrancophoneMeeting = {
  id: string
  type: { code: string; label: I18nText }   // francophone_meeting_type
  title: I18nText; description: I18nText | null
  start_at: string; end_at: string
  format: 'onsite' | 'online' | 'hybrid'
  venue: string | null
  has_video: boolean                        // jamais le lien ici (R6)
  organizer: string                         // « IFDD » ou le nom de l'organisation (R9 bis)
  open_access: boolean; access_audience: I18nText | null
  requires_registration: boolean
  capacity: number | null; registered_count: number; waitlist_enabled: boolean
  registration_opens_at: string | null; registration_closes_at: string | null
  status: 'scheduled' | 'cancelled'        // « Terminée » = end_at < server_time
  cancellation_reason: string | null
  pavilion_session_id: string | null
}
```

`ETag`/`304`. « Complet » = `capacity` atteinte. Bornes de fenêtre nulles = sans limite ; inscription close après `start_at`. `404 NEGOTIATION_EDITION_UNKNOWN`.

## `GET /negotiation/me/meeting-registrations?edition=` — connectée, `Cache-Control: private, no-store`, empreinte propre à la personne

```ts
type MyMeetingRegistrations = {
  registrations: { meeting_id: string; status: 'registered' | 'waitlisted'; waitlist_position: number | null;
                   client_ref: string | null; registered_at: string }[]
  video: { meeting_id: string; url: string }[]   // inscrites ; réunions sans inscription si admise (R6)
}
```

## `PUT /negotiation/me/meeting-registrations/{meeting_id}` — accès négociateur

Corps `{ client_ref: string }` — **un `client_ref` neuf par geste**. Inscrit ou met en attente : `200` →
`{ status, waitlist_position }`. Même `client_ref` que la ligne → `200`, état courant, rien d'écrit ;
ligne désinscrite et `client_ref` différent → réinscription (en fin de liste d'attente si elle n'est pas
vide).
`403 NEGOTIATION_MEETING_FORBIDDEN` sans accès négociateur, `401` sans compte ; `409
NEGOTIATION_MEETING_FULL` (complet sans liste d'attente) ; `409 NEGOTIATION_MEETING_CLOSED` (fenêtre
close, pas d'inscription requise) ; `409 NEGOTIATION_MEETING_UNAVAILABLE` (annulée, terminée, brouillon) ;
`404 NEGOTIATION_MEETING_UNKNOWN`.

## `DELETE /negotiation/me/meeting-registrations/{meeting_id}`

Se désinscrit (`cancelled`), promeut la première en attente **dans la même transaction**, prévient la
personne promue (R8). Idempotent (`204`).

## Back-office — `negotiation.meeting.manage` sur la portée de l'espace de la réunion (R9) ; adresse forgée → `404`

- `GET /admin/negotiation/meetings?edition=` — liste (brouillons compris), filtrée par les espaces
  administrés.
- `POST /admin/negotiation/meetings` · `PUT /admin/negotiation/meetings/{id}` — corps : nature, titre,
  description, début, fin, format, lieu, `external_url`, capacité, liste d'attente, inscription requise,
  fenêtre, accès ouvert ou limité et public, organisée par l'IFDD ou organisation ; le serveur pose
  espace, slug, édition, fuseau, `kind` (R9 bis) ; les invariants se traduisent en `400
  NEGOTIATION_MEETING_INVALID` qui nomme le champ ; relever ou retirer la capacité promeut la liste
  d'attente. Pas de garde sur `updated_at` (R4).
- `POST …/{id}/publish` (brouillon → publiée ; les contraintes de publication traduites en `400`) · `POST …/{id}/cancel` `{ reason }` — prévient inscrites
  et liste d'attente.
- `PUT …/{id}/pavilion` `{ pavilion_session_id | null }` — activité de la même édition, sinon `400`.
- `GET …/{id}/registrations` — inscrites, liste d'attente (nom, pays, date).
- `GET /admin/negotiation/pavilion-activities?edition=` — `{ id, title, starts_at }[]` lus dans
  `programme.sessions` de l'édition, pour le sélecteur.

Un changement d'heure ou de lieu d'une réunion publiée prévient inscrites et liste d'attente (R8).

## Codes ajoutés

`NEGOTIATION_MEETING_UNKNOWN` (404) · `NEGOTIATION_MEETING_FORBIDDEN` (403) · `NEGOTIATION_MEETING_FULL`
(409) · `NEGOTIATION_MEETING_CLOSED` (409) · `NEGOTIATION_MEETING_UNAVAILABLE` (409) ·
`NEGOTIATION_MEETING_INVALID` (400).
