# Contrat — l'import et l'ordre du jour (back-office)

Sous `/admin/negotiation`, gardées par `Requires<SpaceManage>` **sur la portée globale** — jamais
`RequiresAnyScope` (tranché le 21/09, voir `progress.md`). Écritures sous `Db::write(&ctx)`.

## `GET /admin/negotiation/import?edition={slug}`

```ts
type OfficialImportAdmin = {
  edition: { slug: string; name: I18nText; timezone: string }
  enabled: boolean
  reader: 'archive' | 'live'
  archive_name: string | null
  archive_first_day: string | null        // AAAA-MM-JJ
  archives: string[]                      // les jeux archivés présents dans le binaire
  live_url: string | null
  time_correction_minutes: number
  official_programme_url: string
  interval_seconds: number
  missed_threshold: number
  missed_reads: number
  serving: boolean                        // negotiation.import_is_serving()
  last_success_at: string | null
  last_attempt_at: string | null
  last_error: string | null
  failing_since: string | null
  last_change_count: number | null
  session_count: number
  agenda_items_without_theme: number
  runs: { started_at: string; outcome: 'success' | 'failure'; error: string | null;
          session_count: number | null; change_count: number | null }[]   // les 20 dernières
}
```

## `PUT /admin/negotiation/import?edition={slug}`

Corps : `enabled`, `reader`, `archive_name`, `archive_first_day`, `live_url`, `time_correction_minutes`,
`official_programme_url`, `interval_seconds`, `missed_threshold`. Rend `OfficialImportAdmin`.

- **Allumer** pose la première lecture dans la même transaction (clé d'idempotence par créneau) ;
  **éteindre** coupe l'affichage aussitôt — la règle est lue, pas recopiée.
- Le seuil et l'intervalle valent dès la lecture suivante (FR-017, US2 sc. 6).
- `reader = 'live'` sans `live_url`, `archive_name` absent des `archives`, intervalle < 60 s ou seuil < 1 →
  `400 NEGOTIATION_IMPORT_CONFIG_INVALID`, le message nomme le champ.

## `POST /admin/negotiation/import/read?edition={slug}`

« Lire maintenant » : pose une lecture immédiate, `202`, à sa propre clé (`import:<éd>:manuel:<request_id>`), **sans
replanifier** — deux appels de suite font deux lectures, et la chaîne récurrente reste unique ([R7](../research.md)).
Possible import éteint : la lecture a lieu, l'affichage reste coupé.

## `GET /admin/negotiation/agenda-items?edition={slug}`

```ts
type AgendaItemAdmin = { id: string; code: string; title: string; theme: string | null;
                         session_count: number; theme_set_at: string | null }
```

Triés par code ; ceux sans thématique d'abord.

## `PUT /admin/negotiation/agenda-items/{id}`

Corps `{ theme: string | null }`. `400 NEGOTIATION_THEME_UNKNOWN` (code existant de 0c) ;
`404 NEGOTIATION_AGENDA_ITEM_UNKNOWN`. Les sessions du point en héritent à la lecture — rien n'est
recopié sur elles.

**Aucune route ne modifie une session** (FR-041).
