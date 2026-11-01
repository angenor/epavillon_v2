<script setup lang="ts">
import type { DayFrame } from '~/utils/programme-tracks'
import type { ProgrammeRowData } from './ProgrammeRow.vue'
import type { TimeZoneName } from '~/types/shared'
import type { PublicScheduleRow } from '~/types/views'
import type { AttachedImage } from '~/types/media'

/**
 * Le cadre d'une journée spéciale publiée dans `/programmations` (direction
 * condensée, 01/11) : son nom en étiquette posée sur un filet à sa couleur,
 * puis un fond teinté qui porte le lien vers sa page, ses moments en lignes
 * pointillées et ses activités, les lignes de la liste. Ses partenaires
 * précèdent le lien, au sigle ; le nom complet reste au survol et au lecteur d'écran.
 */

interface Props {
  frame: DayFrame<ProgrammeRowData>
  /** La page de la journée. */
  to: string
  timezone: TimeZoneName
  /** La photo de l'édition : la vignette des activités sans couverture. */
  editionImage?: AttachedImage | null
}

const props = withDefaults(defineProps<Props>(), { editionImage: null })

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
const partners = computed(() =>
  props.frame.track.partners.map((partner) => ({
    id: partner.organization_id,
    mark: partner.acronym ?? partner.name,
    name: partner.name,
  })),
)

/** Temps fort de cette journée-ci : une activité peut l'être dans une autre et pas dans celle-ci. */
const isHighlight = (session: PublicScheduleRow) =>
  session.tracks.some((badge) => badge.slug === props.frame.track.slug && badge.is_highlight)

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
        <p v-if="partners.length" class="min-w-0 text-[0.92rem] text-text-secondary">
          {{ t('programme.tracks.with') }}
          <template v-for="(partner, index) in partners" :key="partner.id">
            <template v-if="index">, </template>
            <b class="font-bold text-text" :title="partner.mark === partner.name ? undefined : partner.name">
              <template v-if="partner.mark === partner.name">{{ partner.name }}</template>
              <template v-else><span aria-hidden="true">{{ partner.mark }}</span><span class="sr-only">{{ partner.name }}</span></template>
            </b>
          </template>
        </p>
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
          <EventProgrammeRow v-if="item.kind === 'activity'" :entry="item.row" tone="day" :highlight="isHighlight(item.session)" :edition-image="props.editionImage" />
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
