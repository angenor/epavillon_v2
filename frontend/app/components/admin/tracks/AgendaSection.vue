<script setup lang="ts">
import type {
  TrackCompositionEntry,
  TrackMoment,
  TrackMomentPayload,
  TrackPageSettings,
  TrackPart,
  TrackPartPayload,
} from '~/types/event/track-page'
import type { TrackSessionPlacementPayload } from '~/types/programme/track-page'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * Le déroulé de la page : ses parties, les activités du fil rangées dans une
 * partie (temps fort compris) et les moments sans activité. Chaque geste part
 * aussitôt par sa route, qui rend les réglages entiers.
 *
 * Les activités ne s'ajoutent pas ici : le rattachement au fil reste au
 * planificateur (règle n° 7). Leur ordre est celui de l'horloge, comme sur la
 * page publique ; seules les parties se réordonnent.
 */

const props = defineProps<{
  settings: TrackPageSettings
  timezone: TimeZoneName
  zoneLabel: string
  defaultDay: IsoDate
  plannerTo: string
  canManage: boolean
  canPlace: boolean
  busy?: boolean
  /** Refus de la dernière écriture d'une partie ou d'un moment. */
  error?: string | null
}>()
const emit = defineEmits<{
  savePart: [partId: string | null, payload: TrackPartPayload]
  removePart: [partId: string]
  movePart: [index: number, delta: -1 | 1]
  saveMoment: [momentId: string | null, payload: TrackMomentPayload]
  removeMoment: [momentId: string]
  place: [entry: TrackCompositionEntry, payload: TrackSessionPlacementPayload]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time, dayLong } = useDateTime()

const parts = computed(() => [...props.settings.parts].sort((a, b) => a.sort_order - b.sort_order))

function partHeading(part: TrackPart, index: number): string {
  return part.label ? tr(part.label) : part.title ? tr(part.title) : t('admin.event.track.parts.numbered', { n: index + 1 })
}

const partOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.event.track.agenda.noPart') },
  ...parts.value.map((part, index) => ({ value: part.id, label: partHeading(part, index) })),
])

type Row =
  | { kind: 'activity'; key: string; starts_at: string; ends_at: string | null; entry: TrackCompositionEntry }
  | { kind: 'moment'; key: string; starts_at: string; ends_at: string | null; moment: TrackMoment }

const rows = computed<Row[]>(() =>
  [
    ...props.settings.sessions.map((entry): Row => ({
      kind: 'activity',
      key: entry.session_id,
      starts_at: entry.starts_at,
      ends_at: entry.ends_at,
      entry,
    })),
    ...props.settings.moments.map((moment): Row => ({
      kind: 'moment',
      key: moment.id,
      starts_at: moment.starts_at,
      ends_at: moment.ends_at,
      moment,
    })),
  ].sort((a, b) => Date.parse(a.starts_at) - Date.parse(b.starts_at)),
)

const partOf = (row: Row) => (row.kind === 'activity' ? row.entry.part_id : row.moment.part_id)

/** Les parties dans leur ordre, puis ce qui n'en a pas, s'il y en a. */
const groups = computed(() => {
  const known = new Set(parts.value.map((p) => p.id))
  const list = parts.value.map((part, index) => ({
    key: part.id,
    heading: partHeading(part, index),
    number: index + 1,
    rows: rows.value.filter((row) => partOf(row) === part.id),
  }))
  const loose = rows.value.filter((row) => !partOf(row) || !known.has(partOf(row) as string))
  if (loose.length || list.length === 0) {
    list.push({ key: 'none', heading: t('admin.event.track.agenda.noPart'), number: 0, rows: loose })
  }
  return list
})

const multiDay = computed(() => {
  const days = new Set(rows.value.map((row) => dayLong(row.starts_at, props.timezone)))
  return days.size > 1
})

function range(row: Row): string {
  const start = time(row.starts_at, props.timezone)
  const prefix = multiDay.value ? `${dayLong(row.starts_at, props.timezone)}, ` : ''
  if (!row.ends_at) return prefix + start
  return prefix + t('common.datetime.timeRange', { start, end: time(row.ends_at, props.timezone) })
}

function place(entry: TrackCompositionEntry, change: Partial<TrackSessionPlacementPayload>): void {
  emit('place', entry, {
    part_id: entry.part_id,
    is_highlight: entry.is_highlight,
    sort_order: entry.sort_order,
    ...change,
  })
}

// Dialogues -----------------------------------------------------------------

const editedPart = ref<TrackPart | null>(null)
const partOpen = ref(false)
const editedMoment = ref<TrackMoment | null>(null)
const momentOpen = ref(false)
const doomedPart = ref<TrackPart | null>(null)
const doomedMoment = ref<TrackMoment | null>(null)

function openPart(part: TrackPart | null): void {
  editedPart.value = part
  partOpen.value = true
}

function openMoment(moment: TrackMoment | null): void {
  editedMoment.value = moment
  momentOpen.value = true
}

const nextPartOrder = computed(() => (parts.value.at(-1)?.sort_order ?? 0) + 10)

/** Fermer un dialogue une fois l'écriture acceptée, pas avant. */
watch(
  () => props.settings,
  () => {
    if (props.error) return
    partOpen.value = false
    momentOpen.value = false
  },
)
</script>

<template>
  <div class="space-y-8">
    <!-- PARTIES -->
    <section aria-labelledby="track-parts-title">
      <div class="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h3 id="track-parts-title" class="text-base font-semibold">{{ t('admin.event.track.parts.heading') }}</h3>
          <p class="mt-1 max-w-(--measure) text-sm text-text-muted">{{ t('admin.event.track.parts.intro') }}</p>
        </div>
        <UiButton v-if="props.canManage" variant="secondary" icon="plus" :disabled="props.busy" @click="openPart(null)">
          {{ t('admin.event.track.parts.add') }}
        </UiButton>
      </div>

      <ol v-if="parts.length" class="mt-4 divide-y divide-border-subtle rounded-md border border-border-subtle">
        <li v-for="(part, index) in parts" :key="part.id" class="flex flex-wrap items-center gap-x-3 gap-y-2 p-3">
          <span class="font-mono text-2xl leading-none font-light text-text-subtle tabular-nums">{{ index + 1 }}</span>
          <div class="min-w-0 flex-1 basis-48">
            <p class="font-semibold">{{ partHeading(part, index) }}</p>
            <p v-if="part.label && part.title" class="text-sm text-text-secondary">{{ tr(part.title) }}</p>
            <p v-if="part.summary" class="text-sm text-text-muted">{{ tr(part.summary) }}</p>
          </div>
          <div v-if="props.canManage" class="flex items-center gap-1">
            <UiMoveButtons
              :item="partHeading(part, index)"
              :first="index === 0"
              :last="index === parts.length - 1"
              :disabled="props.busy"
              @up="emit('movePart', index, -1)"
              @down="emit('movePart', index, 1)"
            />
            <UiButton
              variant="ghost"
              icon="edit"
              icon-only
              :label="t('admin.event.track.parts.edit', { part: partHeading(part, index) })"
              :disabled="props.busy"
              @click="openPart(part)"
            />
            <UiButton
              variant="ghost"
              icon="trash"
              icon-only
              :label="t('admin.event.track.parts.remove', { part: partHeading(part, index) })"
              :disabled="props.busy"
              @click="doomedPart = part"
            />
          </div>
        </li>
      </ol>
      <p v-else class="mt-4 text-sm text-text-muted">{{ t('admin.event.track.parts.empty') }}</p>
    </section>

    <!-- DÉROULÉ -->
    <section aria-labelledby="track-agenda-title">
      <div class="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h3 id="track-agenda-title" class="text-base font-semibold">{{ t('admin.event.track.agenda.title') }}</h3>
          <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
            {{ t('admin.event.track.agenda.intro', { zone: props.zoneLabel }) }}
          </p>
        </div>
        <div class="flex flex-wrap gap-2">
          <UiButton variant="ghost" icon="calendar" :to="props.plannerTo">
            {{ t('admin.event.track.agenda.attach') }}
          </UiButton>
          <UiButton v-if="props.canManage" variant="secondary" icon="plus" :disabled="props.busy" @click="openMoment(null)">
            {{ t('admin.event.track.moments.add') }}
          </UiButton>
        </div>
      </div>

      <UiEmptyState
        v-if="rows.length === 0"
        class="mt-4"
        icon="calendar"
        :title="t('admin.event.track.agenda.empty.title')"
        :description="t('admin.event.track.agenda.empty.description')"
      />

      <div v-else class="mt-4 space-y-6">
        <div v-for="group in groups" :key="group.key">
          <h4 class="flex items-baseline gap-2 border-b border-border pb-2 text-sm font-semibold">
            <span v-if="group.number" class="font-mono text-text-subtle tabular-nums">{{ group.number }}</span>
            {{ group.heading }}
          </h4>
          <p v-if="group.rows.length === 0" class="py-3 text-sm text-text-subtle">
            {{ t('admin.event.track.agenda.partEmpty') }}
          </p>
          <ol class="divide-y divide-border-subtle">
            <li
              v-for="row in group.rows"
              :key="row.key"
              class="grid gap-x-4 gap-y-2 py-3 sm:grid-cols-[9rem_minmax(0,1fr)] lg:grid-cols-[9rem_minmax(0,1fr)_auto] lg:items-center"
            >
              <p class="text-sm text-text-secondary tabular-nums">{{ range(row) }}</p>

              <template v-if="row.kind === 'activity'">
                <div class="min-w-0">
                  <p class="font-semibold text-text">{{ tr(row.entry.title) }}</p>
                  <div class="mt-1 flex flex-wrap gap-1.5">
                    <UiBadge size="sm" intent="neutral" :label="t('admin.event.track.agenda.activity')" />
                    <UiBadge
                      v-if="!row.entry.is_published"
                      size="sm"
                      intent="warning"
                      :label="t('admin.event.track.agenda.unpublished')"
                    />
                  </div>
                </div>
                <div class="flex flex-wrap items-end gap-2 sm:col-start-2 lg:col-start-auto">
                  <UiSelect
                    class="min-w-48"
                    :model-value="row.entry.part_id ?? ''"
                    :options="partOptions"
                    :label="t('admin.event.track.agenda.partOf', { title: tr(row.entry.title) })"
                    hide-label
                    hide-optional
                    :disabled="!props.canPlace || props.busy || parts.length === 0"
                    @update:model-value="(next: string) => place(row.entry, { part_id: next || null })"
                  />
                  <UiButton
                    variant="secondary"
                    icon="star"
                    :pressed="row.entry.is_highlight"
                    :disabled="!props.canPlace || props.busy"
                    @click="place(row.entry, { is_highlight: !row.entry.is_highlight })"
                  >
                    {{ t('admin.event.track.agenda.highlight') }}
                  </UiButton>
                </div>
              </template>

              <template v-else>
                <div class="min-w-0">
                  <p class="font-medium text-text-secondary">{{ tr(row.moment.label) }}</p>
                  <p v-if="row.moment.description" class="text-sm text-text-muted">{{ tr(row.moment.description) }}</p>
                  <UiBadge class="mt-1" size="sm" intent="neutral" :label="t('admin.event.track.agenda.moment')" />
                </div>
                <div v-if="props.canManage" class="flex items-center gap-1 sm:col-start-2 lg:col-start-auto">
                  <UiButton
                    variant="ghost"
                    icon="edit"
                    icon-only
                    :label="t('admin.event.track.moments.edit', { moment: tr(row.moment.label) })"
                    :disabled="props.busy"
                    @click="openMoment(row.moment)"
                  />
                  <UiButton
                    variant="ghost"
                    icon="trash"
                    icon-only
                    :label="t('admin.event.track.moments.remove', { moment: tr(row.moment.label) })"
                    :disabled="props.busy"
                    @click="doomedMoment = row.moment"
                  />
                </div>
              </template>
            </li>
          </ol>
        </div>
      </div>
    </section>

    <AdminTracksPartDialog
      :open="partOpen"
      :part="editedPart"
      :sort-order="nextPartOrder"
      :busy="props.busy"
      :error="partOpen ? props.error : null"
      @close="partOpen = false"
      @save="(payload: TrackPartPayload) => emit('savePart', editedPart?.id ?? null, payload)"
    />

    <AdminTracksMomentDialog
      :open="momentOpen"
      :moment="editedMoment"
      :parts="parts"
      :timezone="props.timezone"
      :zone-label="props.zoneLabel"
      :default-day="props.defaultDay"
      :busy="props.busy"
      :error="momentOpen ? props.error : null"
      @close="momentOpen = false"
      @save="(payload: TrackMomentPayload) => emit('saveMoment', editedMoment?.id ?? null, payload)"
    />

    <UiModal
      :open="doomedPart !== null"
      :title="t('admin.event.track.parts.removeTitle')"
      :description="t('admin.event.track.parts.removeDescription')"
      @update:open="(next: boolean) => { if (!next) doomedPart = null }"
    >
      <template #footer>
        <UiButton variant="ghost" @click="doomedPart = null">{{ t('common.actions.cancel') }}</UiButton>
        <UiButton
          variant="danger"
          :loading="props.busy"
          @click="() => { if (doomedPart) emit('removePart', doomedPart.id); doomedPart = null }"
        >
          {{ t('admin.event.track.parts.removeAction') }}
        </UiButton>
      </template>
    </UiModal>

    <UiModal
      :open="doomedMoment !== null"
      :title="t('admin.event.track.moments.removeTitle')"
      @update:open="(next: boolean) => { if (!next) doomedMoment = null }"
    >
      <template #footer>
        <UiButton variant="ghost" @click="doomedMoment = null">{{ t('common.actions.cancel') }}</UiButton>
        <UiButton
          variant="danger"
          :loading="props.busy"
          @click="() => { if (doomedMoment) emit('removeMoment', doomedMoment.id); doomedMoment = null }"
        >
          {{ t('admin.event.track.moments.removeAction') }}
        </UiButton>
      </template>
    </UiModal>
  </div>
</template>
