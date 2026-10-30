<script setup lang="ts">
import type { DayFrame } from '~/utils/programme-tracks'
import type { ProgrammeRowData } from './ProgrammeRow.vue'
import type { TimeZoneName } from '~/types/shared'

/**
 * Le cadre d'une journée spéciale publiée dans `/programmations` (direction F,
 * 30/10) : un bandeau à sa couleur, puis ses lignes, celles de la liste, et
 * ses moments réunis en lignes discrètes.
 *
 * Tout le cadre mène à la page de la journée : le lien du bandeau s'étend au
 * cadre entier (`after:inset-0`), chaque activité pose son propre lien
 * au-dessus (vers son ancre dans le déroulé). Aucun lien imbriqué ; les
 * lignes de moments, sans lien, laissent passer le clic au bandeau.
 */

interface Props {
  frame: DayFrame<ProgrammeRowData>
  /** La page de la journée, en haut. */
  to: string
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()

const style = computed(() => trackDayStyle(props.frame.track.color_hex))
const title = computed(() => tr(props.frame.track.title))
const kicker = computed(() => {
  const { start, end } = props.frame
  if (!start || !end) return t('programme.tracks.kicker')
  return t('programme.tracks.kickerHours', { start: time(start, props.timezone), end: time(end, props.timezone) })
})
</script>

<template>
  <div
    role="group"
    :aria-label="title"
    class="group/frame relative overflow-hidden lg:-ml-3 rounded-[10px] border-2 border-(--color-day-line) bg-(--color-day-surface) transition-colors hover:bg-(--color-day-surface-hover)"
    :style="style"
  >
    <NuxtLink
      :to="props.to"
      class="flex flex-wrap items-center justify-between gap-x-6 gap-y-1.5 bg-(--color-day-solid) px-4 py-3 text-(--color-day-on-solid) no-underline after:absolute after:inset-0 focus-visible:outline-2 focus-visible:-outline-offset-4 focus-visible:outline-(--color-day-on-solid) sm:px-5"
    >
      <span class="min-w-0">
        <span class="block text-xs font-bold uppercase opacity-85 tabular-nums" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ kicker }}
        </span>
        <span class="mt-0.5 block text-lg leading-snug font-bold text-balance">{{ title }}</span>
      </span>
      <span class="inline-flex shrink-0 items-center gap-1.5 text-sm font-bold group-hover/frame:underline">
        {{ t('programme.tracks.open') }}
        <UiIcon name="arrow-right" size="18px" class="transition-transform duration-200 motion-safe:group-hover/frame:translate-x-1" />
      </span>
    </NuxtLink>

    <ol>
      <li
        v-for="item in props.frame.items"
        :key="item.kind === 'activity' ? item.row.id : item.key"
        :class="item.kind === 'activity' ? 'relative z-[1]' : ''"
      >
        <EventProgrammeRow v-if="item.kind === 'activity'" :entry="item.row" :striped="item.index % 2 === 1" tone="day" />
        <p v-else class="flex min-h-11 flex-wrap items-center gap-x-2 gap-y-0.5 px-3 py-2 text-sm text-text-muted">
          <template v-for="(moment, index) in item.moments" :key="moment.starts_at">
            <span v-if="index" aria-hidden="true">·</span>
            <span>
              <span class="font-bold text-text-secondary tabular-nums">{{ time(moment.starts_at, props.timezone) }}</span>
              {{ tr(moment.label) }}
            </span>
          </template>
        </p>
      </li>
    </ol>
  </div>
</template>
