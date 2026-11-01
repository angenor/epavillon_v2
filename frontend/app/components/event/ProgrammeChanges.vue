<script setup lang="ts">
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'
import type { ProgrammeChange } from '~/types/programme/changes'
import type { IsoDateTime } from '~/types/shared'

/**
 * La fenêtre « Ce qui a changé » de `/programmations` (maquette du 01/11) :
 * les quinze derniers jours, par jour, le plus récent en haut, heures dans le
 * fuseau de l'édition. Ouverte depuis « Mis à jour le » du bandeau et depuis le
 * lien rouge de la barre d'outils.
 */

interface Props {
  open: boolean
  changes: ProgrammeChange[]
  status: 'idle' | 'loading' | 'ready' | 'error'
  error: string | null
  hasSince: boolean
  isNew: (change: ProgrammeChange) => boolean
  timezone: string
  city: string | null
  editionSlug: string
  /** Un instant de l'édition, pour le décalage horaire affiché. */
  editionStart: IsoDateTime
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean]; retry: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time, dayLong, zoneLabel, zoneOffsetShort, intlLocale } = useDateTime()
const localePath = useLocalePath()

const description = computed(() => {
  const head = t('programme.changes.description', {
    zone: zoneLabel(props.timezone, props.city ?? undefined),
    offset: zoneOffsetShort(props.timezone, props.editionStart),
  })
  return props.hasSince ? `${head} ${t('programme.changes.sinceNote')}` : head
})

const days = computed(() => {
  const groups = new Map<string, { label: string; changes: ProgrammeChange[] }>()
  for (const change of props.changes) {
    const key = dayKeyInZone(change.occurred_at, props.timezone)
    const group = groups.get(key) ?? { label: dayLong(change.occurred_at, props.timezone), changes: [] }
    group.changes.push(change)
    groups.set(key, group)
  }
  return [...groups.entries()].sort(([a], [b]) => b.localeCompare(a)).map(([key, group]) => ({ key, ...group }))
})

const KIND_STATE: Record<ProgrammeChange['kind'], SessionDisplayState> = {
  rescheduled: 'ongoing',
  retitled: 'ongoing',
  cancelled: 'cancelled',
  postponed: 'postponed',
  added: 'upcoming',
}

function kindLabel(change: ProgrammeChange): string {
  if (change.kind === 'rescheduled') return t(change.day_changed ? 'programme.changes.kind.date' : 'programme.changes.kind.time')
  if (change.kind === 'retitled') return t('programme.changes.kind.title')
  return t(`programme.changes.kind.${change.kind}`)
}

/** « ven. 13, 13:00 à 14:00 », ou « vendredi 13, … » en toutes lettres. */
function slot(start: IsoDateTime | null, end: IsoDateTime | null, weekday: 'short' | 'long'): string {
  if (!start || !end) return ''
  const day = new Intl.DateTimeFormat(intlLocale.value, { weekday, day: 'numeric', timeZone: props.timezone }).format(
    new Date(start),
  )
  return t('programme.changes.slot', { day, start: time(start, props.timezone), end: time(end, props.timezone) })
}

function note(change: ProgrammeChange): string {
  if (change.kind === 'added') return t('programme.changes.addedNote', { slot: slot(change.starts_at, change.ends_at, 'long') })
  const previous = slot(change.previous_starts_at, change.previous_ends_at, 'long')
  return t(change.kind === 'cancelled' ? 'programme.changes.cancelledNote' : 'programme.changes.postponedNote', { slot: previous })
}

const linkOf = (change: ProgrammeChange) =>
  localePath({ name: 'activity-edition-session', params: { edition: props.editionSlug, session: change.slug } })
</script>

<template>
  <UiModal
    :open="props.open"
    size="lg"
    :title="t('programme.changes.title')"
    :description="description"
    @update:open="emit('update:open', $event)"
  >
    <UiLoadingState v-if="props.status === 'loading' || props.status === 'idle'" variant="list" :lines="4" :label="t('programme.changes.loading')" />

    <UiErrorState
      v-else-if="props.status === 'error'"
      compact
      :title="t('programme.changes.error')"
      :description="props.error ?? undefined"
      @retry="emit('retry')"
    />

    <UiEmptyState v-else-if="!props.changes.length" compact icon="calendar" :title="t('programme.changes.empty')" />

    <div v-else class="-mt-2">
      <section v-for="day in days" :key="day.key" :aria-label="day.label">
        <h3 class="mt-4 mb-1.5 font-condensed text-base font-bold tracking-[0.05em] text-text-muted uppercase">
          {{ day.label }}
        </h3>
        <article
          v-for="(change, index) in day.changes"
          :key="`${change.session_id}-${change.kind}-${change.occurred_at}-${index}`"
          class="grid grid-cols-1 gap-x-3.5 gap-y-1 border-t border-border-subtle py-3 sm:grid-cols-[112px_minmax(0,1fr)]"
          :class="props.isNew(change) && '-mx-2.5 rounded-md bg-live/5 px-2.5'"
        >
          <span class="self-start justify-self-start">
            <UiStatusBadge :state="KIND_STATE[change.kind]" size="sm" :label="kindLabel(change)" />
          </span>
          <h4 class="text-[1.05rem] leading-tight font-semibold text-text sm:col-start-2">
            <span v-if="change.kind === 'cancelled'">{{ tr(change.title) }}</span>
            <NuxtLink v-else :to="linkOf(change)" class="text-text no-underline hover:text-accent hover:underline">{{ tr(change.title) }}</NuxtLink>
          </h4>
          <p v-if="props.isNew(change)" class="text-[0.72rem] font-bold tracking-[0.06em] text-live uppercase sm:col-start-2">
            {{ t('programme.changes.since') }}
          </p>

          <p
            v-if="change.kind === 'rescheduled' || change.kind === 'retitled'"
            class="flex flex-wrap items-baseline gap-x-2.5 gap-y-1 text-[0.92rem] sm:col-start-2"
          >
            <del class="text-text-muted decoration-[1.5px]">
              <span class="sr-only">{{ t('programme.changes.before') }}</span>
              {{ change.kind === 'retitled' ? tr(change.previous_title) : slot(change.previous_starts_at, change.previous_ends_at, 'short') }}
            </del>
            <UiIcon name="arrow-right" size="1em" class="shrink-0 self-center text-text-muted" />
            <ins class="font-bold text-text no-underline">
              <span class="sr-only">{{ t('programme.changes.after') }}</span>
              {{ change.kind === 'retitled' ? tr(change.new_title) : slot(change.starts_at, change.ends_at, 'short') }}
            </ins>
          </p>
          <p v-else class="text-[0.85rem] text-text-secondary sm:col-start-2">{{ note(change) }}</p>
        </article>
      </section>
    </div>

    <template #footer>
      <p class="mr-auto text-xs text-text-muted">{{ t('programme.changes.footer') }}</p>
    </template>
  </UiModal>
</template>
