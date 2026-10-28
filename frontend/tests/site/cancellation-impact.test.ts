import { test } from 'node:test'
import assert from 'node:assert/strict'
import { sumCancellationImpacts, touchesPublicProgramme } from '../../app/utils/cancellation-impact.ts'

test("l'action groupée additionne les activités et les inscrits de chaque dossier", () => {
  assert.deepEqual(
    sumCancellationImpacts([
      { published_sessions: 1, registrants: 12 },
      { published_sessions: 0, registrants: 0 },
      { published_sessions: 2, registrants: 3 },
    ]),
    { published_sessions: 3, registrants: 15 },
  )
  assert.deepEqual(sumCancellationImpacts([]), { published_sessions: 0, registrants: 0 })
})

test("sans activité publiée, l'annulation ne change pas le programme et rien n'est annoncé", () => {
  assert.equal(touchesPublicProgramme(null), false)
  assert.equal(touchesPublicProgramme({ published_sessions: 0, registrants: 0 }), false)
  assert.equal(touchesPublicProgramme({ published_sessions: 1, registrants: 0 }), true)
})
