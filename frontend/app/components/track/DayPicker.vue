<script setup lang="ts">
import type { PublicTrackActivity, PublicTrackPage } from '~/types/programme/track-page'

/**
 * Les activités d'une journée à garder ou décocher, rangées par partie : la
 * liste de la fenêtre d'inscription à la journée. Les moments (accueil, pauses,
 * clôture) n'ont pas de case : ils sont compris dans le direct, comme tout le
 * reste ; garder une activité ne sert qu'à recevoir son rappel.
 */

interface Props {
  page: PublicTrackPage
  modelValue: string[]
  /** L'activité dont on vient, mise en avant. */
  highlight?: string | null
}

const props = withDefaults(defineProps<Props>(), { highlight: null })
const emit = defineEmits<{ 'update:modelValue': [value: string[]] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const k = (key: string, params: Record<string, unknown> = {}) => t(`registration-dialog.day.${key}`, params)

const timezone = computed(() => props.page.edition.timezone)
const groups = computed(() => dayPickGroups(props.page))
const all = computed(() => pickableIds(props.page))
const kept = computed(() => new Set(props.modelValue))
const count = computed(() => all.value.filter((id) => kept.value.has(id)).length)

function partLabel(group: (typeof groups.value)[number]): string {
  const part = group.part
  if (!part) return ''
  const label = part.label ? tr(part.label) : t('track.part.default', { number: part.number })
  return part.starts_at && part.ends_at
    ? k('part', { label, start: time(part.starts_at, timezone.value), end: time(part.ends_at, timezone.value) })
    : label
}

function byLine(activity: PublicTrackActivity): string {
  const [lead] = activity.organizations
  return lead?.acronym ?? lead?.name ?? activity.session.organization_acronym ?? activity.session.organization_name ?? ''
}

function toggle(id: string, checked: boolean): void {
  const next = new Set(props.modelValue)
  if (checked) next.add(id)
  else next.delete(id)
  emit('update:modelValue', all.value.filter((value) => next.has(value)))
}

function toggleAll(): void {
  emit('update:modelValue', count.value ? [] : [...all.value])
}

const list = useTemplateRef<HTMLElement>('list')
onMounted(() => {
  if (!props.highlight) return
  list.value?.querySelector('[data-highlight]')?.scrollIntoView({ block: 'nearest' })
})
</script>

<template>
  <div ref="list">
    <div class="flex items-center justify-between gap-3 text-sm text-text-secondary">
      <span aria-live="polite">{{ t('registration-dialog.day.count', { count, total: all.length }, count) }}</span>
      <button type="button" class="min-h-(--target-min) cursor-pointer font-semibold text-text-link" @click="toggleAll">
        {{ count ? k('uncheckAll') : k('checkAll') }}
      </button>
    </div>

    <template v-for="(group, index) in groups" :key="group.part?.id ?? `hors-partie-${index}`">
      <p
        v-if="group.part"
        class="mt-3 mb-0.5 font-condensed text-base font-bold tracking-[0.04em] text-[var(--color-day,var(--color-heading))] uppercase"
      >
        {{ partLabel(group) }}
      </p>
      <template v-for="entry in group.entries" :key="entry.kind === 'activity' ? entry.activity.session.id : entry.key">
        <div
          v-if="entry.kind === 'moment'"
          class="grid grid-cols-[22px_54px_minmax(0,1fr)] items-start gap-x-3 border-t border-border-subtle px-1 py-2.5"
        >
          <span aria-hidden="true" />
          <time :datetime="entry.startsAt" class="font-condensed text-[1.15rem] leading-tight font-bold text-text-muted tabular-nums">
            {{ time(entry.startsAt, timezone) }}
          </time>
          <div>
            <b class="block text-[0.92rem] leading-snug font-medium text-text-secondary">{{ tr(entry.entry.moment.label) }}</b>
            <em class="text-[0.72rem] font-bold tracking-[0.04em] text-text-muted uppercase not-italic">{{ k('included') }}</em>
          </div>
        </div>
        <label
          v-else
          class="grid min-h-(--target-min) cursor-pointer grid-cols-[22px_54px_minmax(0,1fr)] items-start gap-x-3 border-t border-border-subtle px-1 py-2.5"
          :class="entry.activity.session.id === props.highlight ? 'rounded-md bg-(--color-day-surface)' : ''"
          :data-highlight="entry.activity.session.id === props.highlight ? '' : undefined"
        >
          <input
            type="checkbox"
            class="mt-0.5 size-5 cursor-pointer accent-[var(--color-day-solid,var(--color-accent))]"
            :checked="kept.has(entry.activity.session.id)"
            @change="toggle(entry.activity.session.id, ($event.target as HTMLInputElement).checked)"
          >
          <time
            :datetime="entry.startsAt"
            class="font-condensed text-[1.15rem] leading-tight font-bold tabular-nums"
            :class="kept.has(entry.activity.session.id) ? 'text-heading' : 'text-text-muted'"
          >
            {{ time(entry.startsAt, timezone) }}
          </time>
          <div class="min-w-0">
            <b
              class="block text-[0.92rem] leading-snug font-semibold"
              :class="kept.has(entry.activity.session.id) ? 'text-text' : 'text-text-muted'"
            >
              {{ tr(entry.activity.session.title) }}
            </b>
            <span v-if="byLine(entry.activity)" class="block text-[0.8rem] text-text-muted">{{ byLine(entry.activity) }}</span>
            <span
              v-if="entry.activity.session.id === props.highlight"
              class="mt-1 inline-block text-[0.72rem] font-bold tracking-[0.04em] text-[var(--color-day,var(--color-accent))] uppercase"
            >
              {{ k('fromActivity') }}
            </span>
          </div>
        </label>
      </template>
    </template>
  </div>
</template>
