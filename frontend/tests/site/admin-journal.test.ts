import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  JOURNAL_KIND_PARAM,
  JOURNAL_PERIOD_PARAM,
  journalCodeKey,
  journalCsvPath,
  journalPageOf,
  journalParam,
  journalParamFor,
  journalTabOf,
  journalTraceItems,
  journalValueText,
} from '../../app/utils/admin-journal.ts'
import type { JournalChange, JournalEntry, JournalOutboxEvent } from '../../app/types/admin-journal.ts'

const AT = '2026-11-02T12:34:46.000Z'

function entry(occurred_at: string): JournalEntry {
  return {
    id: 'e1', occurred_at, source: 'api', kind: 'http_error', level: 'error', request_id: 'r1', actor: null,
    method: 'PUT', route: '/api/proposals/{id}/draft', status: 500, duration_ms: 214, error_code: 'INTERNAL',
    message: null, task: null, job_id: null,
  }
}

function change(occurred_at: string): JournalChange {
  return {
    id: 'c1', occurred_at, actor: null, request_id: 'r1', entity_schema: 'programme', entity_table: 'proposals',
    entity_id: null, action: 'update', fields: [],
  }
}

function event(occurred_at: string): JournalOutboxEvent {
  return {
    id: 'o1', event_type: 'programme.session.created', aggregate_schema: 'programme', aggregate_type: 'session',
    aggregate_id: 'a1', occurred_at, published_at: null, available_at: occurred_at, attempts: 0, last_error: null,
  }
}

test('les trois sources se fusionnent par heure, l\'écriture avant la ligne du journal à heure égale', () => {
  const items = journalTraceItems({
    request_id: 'r1',
    entries: [entry(AT)],
    changes: [change(AT)],
    events: [event('2026-11-02T12:34:45.900Z')],
  })
  assert.deepEqual(items.map((item) => item.kind), ['event', 'change', 'entry'])
})

test('les paramètres d\'URL en français se traduisent dans les deux sens', () => {
  assert.equal(journalParam(JOURNAL_PERIOD_PARAM, '7j'), '7d')
  assert.equal(journalParam(JOURNAL_KIND_PARAM, 'inconnu'), undefined)
  assert.equal(journalParamFor(JOURNAL_KIND_PARAM, 'http_refused'), 'refus')
  assert.equal(journalTabOf('taches'), 'taches')
  assert.equal(journalTabOf('autre'), 'erreurs')
  assert.equal(journalPageOf('3'), 3)
  assert.equal(journalPageOf('-1'), 1)
})

test('l\'export garde les filtres et laisse tomber les vides', () => {
  assert.equal(journalCsvPath({ period: '7d', q: '' }), '/admin/journal/entries.csv?period=7d')
  assert.equal(journalCsvPath({}), '/admin/journal/entries.csv')
})

test('codes et valeurs se rendent lisibles', () => {
  assert.equal(journalCodeKey('engagement.notification_email'), 'engagement_notification_email')
  assert.equal(journalValueText(null), null)
  assert.equal(journalValueText(''), null)
  assert.equal(journalValueText(false), 'false')
  assert.equal(journalValueText({ a: 1 }), '{"a":1}')
})
