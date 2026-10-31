<script setup lang="ts">
import type { LiveDesk } from '~/types/admin-incidents'
import type { I18nText } from '~/types/shared'

/**
 * LES ALERTES DE L'ÉQUIPE DU DIRECT, sur tout le back-office : une notification
 * brève quand Zoom refuse une inscription ou ne crée pas une réunion, pour
 * l'édition administrée. Ne rend rien.
 *
 * Suit `edition:<id>:desk`, que l'API n'accorde qu'avec `live.meeting.manage`
 * sur l'édition : sans elle, aucun signal, aucune lecture. Au signal, relit le
 * poste et compare à la lecture précédente ; la première ne prévient de rien
 * (ce qui était déjà en échec se voit au poste). Relecture espacée de 5 s au
 * moins : l'entrée de trois cents personnes en salle est une rafale de signaux.
 */

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const toast = useToast()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()

interface Seen {
  failures: number
  meetingFailed: boolean
}

const SPACING_MS = 5_000
let known: Map<string, Seen> | null = null
let knownEvent: string | null = null
let last = 0
let timer: ReturnType<typeof setTimeout> | null = null

function snapshot(desk: LiveDesk): Map<string, Seen & { title: I18nText }> {
  const seen = new Map<string, Seen & { title: I18nText }>()
  for (const session of desk.sessions) {
    const meeting = session.meeting
    if (!meeting) continue
    seen.set(meeting.meeting_id, {
      failures: meeting.failure_count,
      meetingFailed: meeting.status === 'failed' || meeting.sync_status === 'failed' || meeting.sync_status === 'abandoned',
      title: session.title,
    })
  }
  return seen
}

async function check(): Promise<void> {
  const eventId = adminScope.currentEventId
  if (!eventId) return
  last = Date.now()
  let desk: LiveDesk | undefined
  try {
    desk = (await api.adminIncidents.list(eventId, adminScope.scope))?.desk
  } catch {
    return
  }
  if (!desk) return
  // Sans `live.meeting.manage`, le poste ne montre aucune réunion : rien à suivre.
  if (!desk.meetings_visible) return
  const next = snapshot(desk)
  const previous = knownEvent === eventId ? known : null
  known = next
  knownEvent = eventId
  if (!previous) return

  const link = { to: localePath('/admin/incidents'), label: t('admin.incident.list.desk.alert.open') }
  for (const [id, now] of next) {
    const before = previous.get(id)
    if (now.meetingFailed && !before?.meetingFailed) {
      toast.error(t('admin.incident.list.desk.alert.meetingFailed'), { message: tr(now.title), link })
    }
    const added = now.failures - (before?.failures ?? 0)
    if (added > 0) {
      toast.error(t('admin.incident.list.desk.alert.registrationFailed', { count: added }), { message: tr(now.title), link })
    }
  }
}

function onSignal(): void {
  if (timer) return
  const wait = Math.max(0, last + SPACING_MS - Date.now())
  timer = setTimeout(() => {
    timer = null
    void check()
  }, wait)
}

useRealtime(() => (adminScope.currentEventId ? [deskTopic(adminScope.currentEventId)] : []), onSignal)

// La référence se pose à l'ouverture : un échec ancien ne se signale pas, un nouveau oui.
watch(
  () => adminScope.currentEventId,
  (eventId) => {
    known = null
    if (eventId && import.meta.client) void check()
  },
  { immediate: true },
)
onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})
</script>

<template>
  <span hidden />
</template>
