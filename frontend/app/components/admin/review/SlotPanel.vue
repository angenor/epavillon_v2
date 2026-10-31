<script setup lang="ts">
import type { PlannerScreen, PlannerSession } from '~/types/admin-planner'
import type { ScheduleConflict } from '~/types/programme/session'
import type { EventId, IsoDateTime, RoomId, SessionId } from '~/types/shared'

/**
 * MODIFIER LE CRÉNEAU DEPUIS LE DOSSIER : le panneau de la programmation
 * (`AdminPlannerSessionDialog`), ouvert sur place. Il lit l'écran du
 * planificateur de l'édition (salles, jours, journées spéciales, canaux,
 * conflits) et écrit par les mêmes routes ; rien n'est redit ici.
 *
 * Un chevauchement est signalé, jamais bloqué (règles 2 et 3) : il s'affiche
 * dans le panneau avant d'enregistrer, et dans l'avis après.
 */

interface Props {
  open: boolean
  eventId: EventId
  sessionId: SessionId | null
  /** « Reprendre le créneau souhaité » : le formulaire part du souhait du dépôt. */
  applyPreferred: boolean
  /** La séance a une réunion : la case « Prévenir Zoom tout de suite » a un sens. */
  hasMeeting: boolean
  writeTo: string
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  saved: [summary: { message: string; conflicts: number }]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()

const screen = ref<PlannerScreen | null>(null)
const loading = ref(false)
const loadError = ref<string | null>(null)
const busy = ref(false)
const error = ref<string | null>(null)
const pushZoom = ref(true)
const writeAfter = ref(false)

function refusal(thrown: unknown): string {
  return thrown instanceof ForbiddenError ? thrown.message : apiErrorMessage(thrown, t)
}

async function load(): Promise<void> {
  loading.value = true
  loadError.value = null
  try {
    screen.value = await api.planner.screen(props.eventId, adminScope.scope)
  } catch (thrown) {
    loadError.value = refusal(thrown)
  } finally {
    loading.value = false
  }
}

watch(
  () => props.open,
  (open) => {
    if (!open) return
    error.value = null
    pushZoom.value = true
    writeAfter.value = false
    void load()
  },
  { immediate: true },
)

const session = computed<PlannerSession | null>(() => {
  const current = screen.value
  if (!current || !props.sessionId) return null
  return [...current.placed, ...current.unplaced].find((entry) => entry.id === props.sessionId) ?? null
})

const conflicts = computed<ScheduleConflict[]>(() =>
  (screen.value?.conflicts ?? []).filter(
    (conflict) => conflict.session_a === props.sessionId || conflict.session_b === props.sessionId,
  ),
)

function mine(list: ScheduleConflict[], id: SessionId): number {
  return list.filter((conflict) => conflict.session_a === id || conflict.session_b === id).length
}

async function afterWrite(id: SessionId, nextConflicts: ScheduleConflict[], message: string): Promise<void> {
  let note = message
  if (props.hasMeeting && pushZoom.value) {
    try {
      await api.meetings.push(id)
      note = `${message} ${t('admin.proposal.dashboard.panel.zoomPushed')}`
    } catch (thrown) {
      note = `${message} ${t('admin.proposal.dashboard.panel.zoomFailed', { message: refusal(thrown) })}`
    }
  }
  emit('saved', { message: note, conflicts: mine(nextConflicts, id) })
  emit('update:open', false)
  if (writeAfter.value) await navigateTo(props.writeTo)
}

async function submit(changes: {
  schedule?: { room_id: RoomId | null; starts_at: IsoDateTime; ends_at: IsoDateTime }
  track_ids?: string[]
  broadcast?: { is_streamed: boolean; broadcast_channel_id: string | null }
}): Promise<void> {
  const current = session.value
  if (!current) return
  // Sans salle, la séance resterait « à placer » sans que rien ne le dise.
  if (changes.schedule && changes.schedule.room_id === null && current.room_id === null) {
    error.value = t('admin.proposal.dashboard.panel.noRoom')
    return
  }
  busy.value = true
  error.value = null
  let latest = screen.value?.conflicts ?? []
  try {
    if (changes.schedule) {
      latest = (await api.planner.schedule({ session_id: current.id, ...changes.schedule })).conflicts
    }
    if (changes.track_ids) {
      latest = (await api.planner.setTracks(auth.person?.id ?? null, { session_id: current.id, track_ids: changes.track_ids })).conflicts
    }
    if (changes.broadcast) {
      latest = (await api.planner.setBroadcast({ session_id: current.id, ...changes.broadcast })).conflicts
    }
    await afterWrite(current.id, latest, t('admin.planner.notice.saved', { title: tr(current.title) }))
  } catch (thrown) {
    // Le panneau reste ouvert : la seconde écriture peut échouer après la première.
    error.value = refusal(thrown)
  } finally {
    busy.value = false
  }
}

async function unplace(target: PlannerSession): Promise<void> {
  busy.value = true
  error.value = null
  try {
    const result = await api.planner.schedule({
      session_id: target.id,
      room_id: null,
      starts_at: target.starts_at,
      ends_at: target.ends_at,
    })
    await afterWrite(target.id, result.conflicts, t('admin.planner.notice.unplaced', { title: tr(target.title) }))
  } catch (thrown) {
    error.value = refusal(thrown)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <UiDrawer
    v-if="props.open && !session"
    :open="props.open"
    :title="t('admin.proposal.dashboard.panel.title')"
    width="30rem"
    @update:open="emit('update:open', $event)"
  >
    <div v-if="loading" class="flex flex-col gap-3">
      <UiSkeletonLoader height="3rem" />
      <UiSkeletonLoader height="12rem" />
    </div>
    <UiErrorState v-else-if="loadError" :description="loadError" :retry-label="t('common.actions.retry')" @retry="load" />
    <UiEmptyState
      v-else
      icon="calendar"
      :title="t('admin.proposal.dashboard.panel.notFound')"
      :description="t('admin.proposal.dashboard.panel.notFoundHint')"
    />
  </UiDrawer>

  <AdminPlannerSessionDialog
    v-else-if="screen"
    :open="props.open"
    :session="session"
    :rooms="screen.rooms"
    :days="screen.days"
    :tracks="screen.tracks"
    :channels="screen.channels"
    :timezone="screen.timezone"
    :zone-label="screen.zone_label ?? undefined"
    :conflicts="conflicts"
    :busy="busy"
    :error="error"
    offer-preferred
    :apply-preferred="props.applyPreferred"
    @update:open="emit('update:open', $event)"
    @submit="submit"
    @unplace="unplace"
  >
    <template #options>
      <div class="flex flex-col gap-3 border-t border-border pt-4">
        <UiCheckbox
          v-if="props.hasMeeting"
          v-model="pushZoom"
          :label="t('admin.proposal.dashboard.panel.pushZoom')"
          :hint="t('admin.proposal.dashboard.panel.pushZoomHint')"
          :disabled="busy"
        />
        <UiCheckbox
          v-model="writeAfter"
          :label="t('admin.proposal.dashboard.panel.writeAfter')"
          :hint="t('admin.proposal.dashboard.panel.writeAfterHint')"
          :disabled="busy"
        />
      </div>
    </template>
  </AdminPlannerSessionDialog>
</template>
