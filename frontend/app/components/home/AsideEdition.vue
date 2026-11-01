<script setup lang="ts">
import type { IsoDateTime } from '~/types/shared'
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'

/**
 * UNE PROCHAINE ÉDITION du panneau « À venir », en une ligne dense : nom, état,
 * dates avec fuseau, puis la prochaine séance ou le rebours.
 *
 * `edition_label` en tête quand il existe : « CdP31 » est ce que les gens
 * disent. Le rebours se compte en JOURNÉES CIVILES dans le fuseau de l'édition.
 */

interface Props {
  edition: PublicEditionRow
  /** La prochaine séance connue de cette édition, s'il y en a une en main. */
  nextSession?: PublicScheduleRow | null
  /** Instant de composition de la réponse : l'horloge qui fait autorité. */
  now: IsoDateTime
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateRange, zoneLabel, date, time } = useDateTime()
const localePath = useLocalePath()

const name = computed(() => props.edition.edition_label ?? tr(props.edition.title))

const dates = computed(() =>
  dateRange(props.edition.starts_at, props.edition.ends_at, props.edition.timezone),
)

const zone = computed(() => zoneLabel(props.edition.timezone, props.edition.city ?? undefined))

/** Le décompte n'a de sens qu'avant l'ouverture. */
const countdown = computed(() => {
  if (props.edition.temporal_state !== 'upcoming') return ''
  const days = daysBetweenInZone(props.now, props.edition.starts_at, props.edition.timezone)
  if (days < 0) return ''
  if (days === 0) return t('home.aside.editions.startsToday')
  return t('home.aside.editions.inDays', days)
})

const nextSessionLabel = computed(() => {
  const session = props.nextSession
  if (!session) return ''
  return t('home.aside.editions.nextSession', {
    date: date(session.starts_at, session.timezone),
    time: time(session.starts_at, session.timezone),
    // La ville de l'ÉDITION, sans quoi le repli rend « Belem » sans accent.
    zone: zoneLabel(session.timezone, props.edition.city ?? undefined),
  })
})
</script>

<template>
  <article data-bird-perch class="py-2">
    <div class="flex items-start justify-between gap-2">
      <h4 class="min-w-0 font-condensed text-[1.12rem] leading-tight font-semibold">
        <NuxtLink
          :to="localePath(`/evenements/${props.edition.slug}`)"
          class="text-text-on-inverse no-underline hover:underline"
        >
          {{ name }}
        </NuxtLink>
      </h4>
      <UiStatusBadge
        :state="props.edition.temporal_state"
        size="sm"
        :label="t(`home.history.state.${props.edition.temporal_state}`)"
      />
    </div>
    <p class="mt-0.5 text-[0.8rem] text-text-on-inverse-muted">{{ dates }} ({{ zone }})</p>
    <p v-if="nextSessionLabel || countdown" class="text-[0.8rem] text-text-on-inverse-muted">
      {{ nextSessionLabel || countdown }}
    </p>
  </article>
</template>
