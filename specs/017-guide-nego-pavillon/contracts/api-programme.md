# Contrat — ce que le module `programme` ajoute (ajouts seulement)

Aucune route nouvelle, aucun champ retiré ni renommé. Formes TS existantes étendues de champs
facultatifs (`frontend/app/types/views.ts`, `types/programme/session.ts`).

## `GET /schedule?event_id=` — `PublicScheduleRow` gagne

```ts
waitlist_enabled?: boolean
registration_required?: boolean
registration_opens_at?: string | null
registration_closes_at?: string | null
waitlisted_count?: number
listing_changed_at?: string | null
language_codes?: string[] | null       // du dossier de proposition, seul champ qui en vient ; nul sans dossier
replay_url?: string | null             // rediffusion disponible seulement
replay_duration_seconds?: number | null
```

`ETag` sur l'empreinte du corps, `304` sur `If-None-Match`.

## `GET /events/{event_id}/sessions/{slug}` — détail

- `speakers[]` gagne `display_name: string` (nom d'affichage de `identity.people`).
- `organizations[]` gagne `name: string` (← `legal_name`), `acronym: string | null`, `country_code: string | null`, `country: I18nText | null`.
- **À trancher** : `speakers[]` ne sert plus `attended`, `confirmed_at` (fuite existante, aucun écran du site ne lit cette route).
- La séance (`session`) gagne les champs ci-dessus.

## `GET /registrations/mine`

`ETag`/`304` et `Cache-Control: private, no-cache` ajoutés. Forme inchangée.

## Inchangés, employés tels quels

`GET /sessions/{id}/registration-form` · `POST /sessions/{id}/registrations` · `POST /registrations/{id}/cancel`
· `GET /events/{id}/venues`.

## Garanties testées

- Les clés rendues avant l'étape sont toutes encore rendues, identiques (test de non-régression).
- Aucune coordonnée ni pièce du dossier de proposition n'est servie.
- `npm run typecheck`, `test:site`, `check-api-contract` verts ; `/schedule` rend autant de lignes qu'avant (une rediffusion par séance) ; page `/programme` du site vérifiée au navigateur.
