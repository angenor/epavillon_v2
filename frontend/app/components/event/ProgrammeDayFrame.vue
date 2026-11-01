<script setup lang="ts">
import type { DayFrame } from '~/utils/programme-tracks'
import type { ProgrammeRowData } from './ProgrammeRow.vue'
import type { TimeZoneName } from '~/types/shared'

/**
 * Le cadre d'une journée spéciale publiée dans `/programmations` (direction
 * condensée, 01/11) : son nom en étiquette posée sur un filet à sa couleur,
 * puis un fond teinté qui porte le lien vers sa page, ses moments en lignes
 * pointillées et ses activités, les lignes de la liste.
 */

interface Props {
  frame: DayFrame<ProgrammeRowData>
  /** La page de la journée. */
  to: string
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()

const style = computed(() => ({
  ...trackDayStyle(props.frame.track.color_hex),
  '--rule-solid': 'var(--color-day-solid)',
  '--rule-on': 'var(--color-day-on-solid)',
}))
const icon = computed(() => trackIcon(props.frame.track.icon_code))
const title = computed(() => tr(props.frame.track.title))
const kicker = computed(() => {
  const { start, end } = props.frame
  if (!start || !end) return t('programme.tracks.kicker')
  return t('programme.tracks.kickerHours', { start: time(start, props.timezone), end: time(end, props.timezone) })
})
</script>

<template>
  <div role="group" :aria-label="title" :style="style">
    <div class="rule-line">
      <span class="rule-label">
        <UiIcon v-if="icon" :name="icon" size="1.05em" />
        {{ title }}
      </span>
      <span class="pb-1.5 text-sm text-text-secondary tabular-nums">{{ kicker }}</span>
    </div>

    <div class="rounded-b-xl rounded-tr-xl bg-(--color-day-surface) px-3.5 pt-1 pb-2 sm:px-6 sm:pb-3">
      <div class="flex flex-wrap items-center gap-x-6 gap-y-2.5 pt-4 pb-2">
        <!-- Contour à la couleur de la journée : l'accent du site jurerait sur sa teinte. -->
        <UiButton
          :to="props.to"
          variant="secondary"
          class="border-(--color-day-line)! text-(--color-day)! hover:bg-(--color-day-solid)! hover:text-(--color-day-on-solid)! sm:ml-auto"
        >
          {{ t('programme.tracks.open') }}
        </UiButton>
      </div>

      <ol>
        <li
          v-for="item in props.frame.items"
          :key="item.kind === 'activity' ? item.row.id : item.key"
          class="border-b border-border-subtle"
          :class="item.kind === 'activity' ? 'last:border-b-0' : 'border-dashed'"
        >
          <EventProgrammeRow v-if="item.kind === 'activity'" :entry="item.row" tone="day" />
          <template v-else>
            <p
              v-for="moment in item.moments"
              :key="moment.starts_at"
              class="grid grid-cols-[76px_minmax(0,1fr)] items-baseline gap-x-3.5 py-3 text-[0.92rem] text-text-secondary min-[860px]:grid-cols-[112px_minmax(0,1fr)] min-[860px]:gap-x-5.5"
            >
              <b class="font-condensed text-[1.3rem] font-bold tabular-nums">{{ time(moment.starts_at, props.timezone) }}</b>
              <span>{{ tr(moment.label) }}</span>
            </p>
          </template>
        </li>
      </ol>
    </div>
  </div>
</template>
