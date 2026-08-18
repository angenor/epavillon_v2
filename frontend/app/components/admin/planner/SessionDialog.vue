<script setup lang="ts">
import type {
  PlannerChannel,
  PlannerDay,
  PlannerRoom,
  PlannerSession,
  PlannerTrack,
} from '~/types/admin-planner'
import type { ScheduleConflict } from '~/types/programme/session'
import type { SelectOption } from '~/types/ui'
import type { IsoDateTime, RoomId, TimeZoneName, TrackId, Uuid } from '~/types/shared'

/**
 * LE PANNEAU D'UNE SÉANCE — placer, déplacer, rattacher, diffuser.
 *
 * IL EST LE CHEMIN SANS SOURIS. Le glisser-déposer de la grille ne fonctionne ni
 * au clavier, ni sur une tablette : ce panneau fait tout ce que la grille fait,
 * en deux temps — on choisit une activité, on choisit son jour, sa salle et son
 * heure. C'est aussi la seule façon de placer une activité sur un écran étroit,
 * où traîner un bloc dans une grille de 40 rem n'a aucun sens.
 *
 * CE QU'IL ÉCRIT, ET DANS QUEL ORDRE. Trois écritures distinctes du modèle — le
 * créneau (`sessions.room_id`, `starts_at`, `ends_at`), le rattachement aux
 * journées spéciales (`programme.session_tracks`), la diffusion
 * (`sessions.is_streamed`, `broadcast_channel_id`). Le panneau n'envoie que ce
 * qui a changé : rouvrir une séance pour lire ses conflits ne doit rien réécrire.
 *
 * IL NE REFUSE AUCUN CRÉNEAU. Aucune vérification de chevauchement ici : les
 * conflits de cette séance sont AFFICHÉS en bas du panneau, et le bouton
 * d'enregistrement reste actif quoi qu'ils disent.
 *
 * LE RATTACHEMENT À UNE JOURNÉE SPÉCIALE EST MANUEL et indépendant de la date :
 * la « Journée finance durable » ne prend pas toutes les activités du
 * 12 novembre. Le panneau signale seulement, en gris, quand la date de la séance
 * sort de la portée annoncée du fil — il ne coche ni ne décoche à la place de
 * l'équipe.
 */

interface Props {
  open: boolean
  session: PlannerSession | null
  rooms: PlannerRoom[]
  days: PlannerDay[]
  tracks: PlannerTrack[]
  channels: PlannerChannel[]
  timezone: TimeZoneName
  zoneLabel?: string
  /** Les conflits de CETTE séance, pour les lire avant de trancher. */
  conflicts: ScheduleConflict[]
  busy?: boolean
  error?: string | null
  /** Faux quand la personne n'a pas le droit d'arbitrer : panneau en lecture. */
  editable?: boolean
}

const props = withDefaults(defineProps<Props>(), { editable: true })

const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [
    changes: {
      schedule?: { room_id: RoomId | null; starts_at: IsoDateTime; ends_at: IsoDateTime }
      track_ids?: TrackId[]
      broadcast?: { is_streamed: boolean; broadcast_channel_id: Uuid | null }
    },
  ]
  /** Retirer du calendrier : la séance retourne au panneau, elle n'est pas supprimée. */
  unplace: [session: PlannerSession]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date: formatDay, timeRange } = useDateTime()
const localePath = useLocalePath()

// ---------------------------------------------------------------------------
// État du formulaire
// ---------------------------------------------------------------------------

/** Heure MURALE du pavillon (`AAAA-MM-JJTHH:MM`) : jamais celle de la machine. */
const startWallClock = ref('')
const durationMinutes = ref(90)
const roomId = ref<string>('')
const trackIds = ref<string[]>([])
const isStreamed = ref(false)
const channelId = ref<string>('')

const DURATIONS = [30, 45, 60, 75, 90, 120, 150, 180, 240]

watch(
  () => [props.open, props.session?.id] as const,
  () => {
    const session = props.session
    if (!props.open || !session) return

    startWallClock.value = wallClockInZone(session.starts_at, props.timezone).replace(' ', 'T')
    durationMinutes.value = plannedDuration(session)
    roomId.value = session.room_id ?? ''
    trackIds.value = [...session.track_ids]
    isStreamed.value = session.is_streamed
    channelId.value =
      session.broadcast_channel_id ?? props.channels.find((channel) => channel.is_default)?.id ?? ''
  },
  { immediate: true },
)

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/**
 * « À placer » est une VALEUR, pas une invite : c'est ainsi qu'on retire une
 * séance du calendrier sans la supprimer, et une option désactivée ne se
 * re-sélectionnerait pas.
 */
const roomOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.planner.dialog.noRoom') },
  ...props.rooms.map((room) => ({
    value: room.id,
    label: tr(room.name),
    description: room.is_virtual
      ? t('admin.planner.dialog.virtualRoom')
      : room.capacity
        ? t('admin.planner.dialog.roomCapacity', { capacity: room.capacity })
        : undefined,
  })),
])

const durationOptions = computed<SelectOption[]>(() =>
  DURATIONS.map((minutes) => ({
    value: String(minutes),
    label:
      minutes % 60 === 0
        ? t('admin.planner.card.durationHours', { hours: minutes / 60 })
        : minutes < 60
          ? t('admin.planner.card.durationMinutes', { minutes })
          : t('admin.planner.card.durationHoursMinutes', {
              hours: Math.floor(minutes / 60),
              minutes: minutes % 60,
            }),
  })),
)

const channelOptions = computed<SelectOption[]>(() =>
  props.channels.map((channel) => ({ value: channel.id, label: tr(channel.name) })),
)

/** Bornes du sélecteur de date : les jours de l'édition. */
const minDate = computed(() => (props.days[0] ? `${props.days[0].day_date}T00:00` : undefined))
const maxDate = computed(() => {
  const last = props.days[props.days.length - 1]
  return last ? `${last.day_date}T23:59` : undefined
})

// ---------------------------------------------------------------------------
// Ce que le panneau enverra
// ---------------------------------------------------------------------------

const startsAt = computed<IsoDateTime | null>(() =>
  instantFromWallClock(startWallClock.value, props.timezone),
)
const endsAt = computed<IsoDateTime | null>(() =>
  startsAt.value ? endOfSlot(startsAt.value, durationMinutes.value) : null,
)

const slotLabel = computed(() => {
  if (!startsAt.value || !endsAt.value) return ''
  return `${formatDay(startsAt.value, props.timezone)} · ${timeRange(startsAt.value, endsAt.value, props.timezone, props.zoneLabel)}`
})

/**
 * LES INSTANTS SE COMPARENT PAR LEUR VALEUR, JAMAIS PAR LEUR CHAÎNE.
 * `2027-11-12T14:00:00-03:00` et `2027-11-12T17:00:00.000Z` désignent la même
 * seconde et ne s'écrivent pas pareil : comparer les textes rendait le bouton
 * « Enregistrer » actif à l'ouverture du panneau, sans qu'on ait rien touché.
 */
function sameInstant(a: string | null, b: string | null): boolean {
  if (a === null || b === null) return a === b
  return Date.parse(a) === Date.parse(b)
}

const scheduleChanged = computed(() => {
  const session = props.session
  if (!session || !startsAt.value || !endsAt.value) return false
  return (
    (roomId.value || null) !== session.room_id ||
    !sameInstant(startsAt.value, session.starts_at) ||
    !sameInstant(endsAt.value, session.ends_at)
  )
})

const tracksChanged = computed(() => {
  const before = [...(props.session?.track_ids ?? [])].sort()
  const after = [...trackIds.value].sort()
  return before.join('|') !== after.join('|')
})

const broadcastChanged = computed(() => {
  const session = props.session
  if (!session) return false
  const nextChannel = isStreamed.value ? channelId.value || null : null
  return isStreamed.value !== session.is_streamed || nextChannel !== session.broadcast_channel_id
})

const hasChanges = computed(() => scheduleChanged.value || tracksChanged.value || broadcastChanged.value)

/** Une date hors des jours de l'édition reste possible : on la SIGNALE. */
const outsideEdition = computed(() => {
  if (!startsAt.value || props.days.length === 0) return false
  const day = wallClockInZone(startsAt.value, props.timezone).slice(0, 10)
  return !props.days.some((entry) => entry.day_date === day)
})

/** Le fil couvre-t-il la date retenue ? Purement indicatif — voir l'en-tête. */
function trackOutOfRange(track: PlannerTrack): boolean {
  if (!startsAt.value || (!track.starts_on && !track.ends_on)) return false
  const day = wallClockInZone(startsAt.value, props.timezone).slice(0, 10)
  if (track.starts_on && day < track.starts_on) return true
  if (track.ends_on && day > track.ends_on) return true
  return false
}

function toggleTrack(trackId: string, checked: boolean): void {
  trackIds.value = checked
    ? [...trackIds.value, trackId]
    : trackIds.value.filter((id) => id !== trackId)
}

function submit(): void {
  if (!props.session || !startsAt.value || !endsAt.value) return

  emit('submit', {
    ...(scheduleChanged.value
      ? { schedule: { room_id: (roomId.value || null) as RoomId | null, starts_at: startsAt.value, ends_at: endsAt.value } }
      : {}),
    ...(tracksChanged.value ? { track_ids: [...trackIds.value] as TrackId[] } : {}),
    ...(broadcastChanged.value
      ? {
          broadcast: {
            is_streamed: isStreamed.value,
            broadcast_channel_id: isStreamed.value ? channelId.value || null : null,
          },
        }
      : {}),
  })
}

const title = computed(() => (props.session ? tr(props.session.title) : ''))
const isPlaced = computed(() => props.session?.room_id !== null && props.session?.room_id !== undefined)
</script>

<template>
  <UiDrawer
    :open="props.open"
    :title="title"
    :description="props.session?.reference_code ?? undefined"
    width="30rem"
    @update:open="emit('update:open', $event)"
  >
    <div v-if="props.session" class="space-y-6">
      <!-- CE QUE L'ÉQUIPE A SOUS LES YEUX AVANT DE TRANCHER : qui porte
           l'activité, ce que le comité en a pensé, ce que l'organisation avait
           demandé. -->
      <section class="rounded-lg border border-border bg-surface-sunken p-3 text-sm">
        <p class="font-medium text-text">
          {{ props.session.organization_name ?? t('admin.planner.dialog.noOrganization') }}
          <span v-if="props.session.organization_country_code" class="text-text-muted">
            · {{ props.session.organization_country_code }}
          </span>
        </p>
        <dl class="mt-2 grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
          <div>
            <dt class="text-text-muted">{{ t('admin.planner.dialog.score') }}</dt>
            <dd class="font-mono tabular-nums text-text">
              {{ props.session.average_score !== null
                ? props.session.average_score.toFixed(2)
                : t('admin.planner.dialog.noScore') }}
            </dd>
          </div>
          <div>
            <dt class="text-text-muted">{{ t('admin.planner.dialog.format') }}</dt>
            <dd class="text-text">{{ t(`admin.planner.format.${props.session.format}`) }}</dd>
          </div>
          <div v-if="props.session.preferred_start_at" class="col-span-2">
            <dt class="text-text-muted">{{ t('admin.planner.dialog.preferred') }}</dt>
            <dd class="text-text">{{ formatDay(props.session.preferred_start_at, props.timezone) }}</dd>
          </div>
          <div v-if="props.session.scheduling_constraints" class="col-span-2">
            <dt class="text-text-muted">{{ t('admin.planner.dialog.constraints') }}</dt>
            <dd class="text-text">{{ props.session.scheduling_constraints }}</dd>
          </div>
        </dl>
        <p v-if="props.session.proposal_id" class="mt-2">
          <NuxtLink
            :to="localePath(`/admin/propositions/${props.session.proposal_id}`)"
            class="text-xs text-accent underline"
          >
            {{ t('admin.planner.dialog.openProposal') }}
          </NuxtLink>
        </p>
      </section>

      <!-- LE CRÉNEAU. Jour, heure et durée dans le fuseau du PAVILLON : le champ
           porte le libellé du fuseau, sans quoi il laisserait croire à une heure
           locale. -->
      <section class="space-y-3">
        <h3 class="text-sm font-semibold tracking-wide text-text uppercase">
          {{ t('admin.planner.dialog.slot') }}
        </h3>

        <UiDatePicker
          v-model="startWallClock"
          with-time
          :label="t('admin.planner.dialog.start')"
          required
          :timezone-label="props.zoneLabel || props.timezone"
          :min="minDate"
          :max="maxDate"
          :disabled="!props.editable || props.busy"
        />

        <UiSelect
          :model-value="String(durationMinutes)"
          :options="durationOptions"
          :label="t('admin.planner.dialog.duration')"
          required
          :disabled="!props.editable || props.busy"
          @update:model-value="(value: string) => (durationMinutes = Number(value))"
        />

        <UiSelect
          v-model="roomId"
          :options="roomOptions"
          :label="t('admin.planner.dialog.room')"
          hide-optional
          :hint="t('admin.planner.dialog.roomHint')"
          :disabled="!props.editable || props.busy"
        />

        <p v-if="slotLabel" class="text-sm text-text-secondary">{{ slotLabel }}</p>

        <UiAlert
          v-if="outsideEdition"
          intent="warning"
          compact
          :message="t('admin.planner.dialog.outsideEdition')"
        />
      </section>

      <!-- JOURNÉES SPÉCIALES. Rattachement MANUEL : toutes les activités du jour
           n'en font pas partie, et la portée annoncée d'un fil est indicative. -->
      <section v-if="props.tracks.length" class="space-y-2">
        <h3 class="text-sm font-semibold tracking-wide text-text uppercase">
          {{ t('admin.planner.dialog.tracks') }}
        </h3>
        <p class="text-xs text-text-muted">{{ t('admin.planner.dialog.tracksHint') }}</p>

        <UiCheckbox
          v-for="track in props.tracks"
          :key="track.id"
          :model-value="trackIds.includes(track.id)"
          :label="tr(track.title)"
          :hint="trackOutOfRange(track) ? t('admin.planner.dialog.trackOutOfRange') : undefined"
          :disabled="!props.editable || props.busy"
          @update:model-value="(checked: boolean) => toggleTrack(track.id, checked)"
        />
      </section>

      <!-- DIFFUSION. Un seul direct à la fois, tous événements confondus : deux
           séances diffusées sur le même canal au même moment restent écrivables,
           et remontent au bandeau en gravité bloquante. -->
      <section class="space-y-2">
        <h3 class="text-sm font-semibold tracking-wide text-text uppercase">
          {{ t('admin.planner.dialog.broadcast') }}
        </h3>

        <UiSwitch
          v-model="isStreamed"
          :label="t('admin.planner.dialog.isStreamed')"
          :hint="t('admin.planner.dialog.isStreamedHint')"
          :disabled="!props.editable || props.busy"
        />

        <UiSelect
          v-if="isStreamed && channelOptions.length > 1"
          v-model="channelId"
          :options="channelOptions"
          :label="t('admin.planner.dialog.channel')"
          :disabled="!props.editable || props.busy"
        />
        <p v-else-if="isStreamed" class="text-xs text-text-muted">
          {{ t('admin.planner.dialog.defaultChannel', {
            channel: channelOptions[0]?.label ?? '',
          }) }}
        </p>
      </section>

      <!-- LES CONFLITS DE CETTE SÉANCE : on les lit, on ne les subit pas. Ils
           n'empêchent pas l'enregistrement. -->
      <section v-if="props.conflicts.length" class="space-y-2">
        <h3 class="text-sm font-semibold tracking-wide text-text uppercase">
          {{ t('admin.planner.dialog.conflicts', props.conflicts.length) }}
        </h3>
        <ul class="space-y-1">
          <li
            v-for="(conflict, index) in props.conflicts"
            :key="`${conflict.conflict_kind}-${index}`"
            class="flex items-start gap-2 text-xs"
          >
            <UiBadge
              size="sm"
              :intent="conflict.severity === 'blocking' ? 'danger' : 'warning'"
              :label="t(`admin.planner.conflict.severity.${conflict.severity}`)"
            />
            <span class="text-text-secondary">
              {{ t(`admin.planner.conflict.kind.${conflict.conflict_kind}`) }}
              <span v-if="conflict.subject_label">— {{ conflict.subject_label }}</span>
            </span>
          </li>
        </ul>
        <p class="text-xs text-text-muted">{{ t('admin.planner.conflict.neverBlocked') }}</p>
      </section>

      <!-- SIGNALER UN DÉBORDEMENT — le pont vers l'écran des messages
           d'incident (A13). Il n'écrit rien ici : il ouvre le formulaire de
           publication déjà pointé sur cette séance, avec la nature « débordement
           sur le créneau suivant » et la fin du créneau comme fin d'affichage.
           C'est le geste d'une équipe qui a trente secondes, pas d'un formulaire
           à remplir de mémoire. -->
      <section v-if="props.session && props.editable" class="border-t border-border pt-4">
        <UiButton
          variant="secondary"
          icon="warning"
          size="sm"
          :to="localePath({
            path: '/admin/incidents/nouveau',
            query: { portee: 'session', cible: props.session.id, nature: 'overrun' },
          })"
        >
          {{ t('admin.planner.dialog.reportOverrun') }}
        </UiButton>
        <p class="mt-1.5 text-xs text-text-muted">{{ t('admin.planner.dialog.reportOverrunHint') }}</p>
      </section>

      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />
    </div>

    <template #footer>
      <div class="flex flex-wrap items-center gap-2">
        <UiButton :loading="props.busy" :disabled="!props.editable || !hasChanges" @click="submit">
          {{ t('admin.planner.dialog.save') }}
        </UiButton>
        <UiButton variant="ghost" :disabled="props.busy" @click="emit('update:open', false)">
          {{ t('common.actions.cancel') }}
        </UiButton>

        <!-- RETIRER, ce n'est pas SUPPRIMER : la séance retourne au panneau avec
             son créneau souhaité, et pourra être replacée. -->
        <UiButton
          v-if="isPlaced && props.editable"
          class="ml-auto"
          variant="danger"
          icon="ban"
          :disabled="props.busy"
          @click="props.session && emit('unplace', props.session)"
        >
          {{ t('admin.planner.dialog.unplace') }}
        </UiButton>
      </div>
    </template>
  </UiDrawer>
</template>
