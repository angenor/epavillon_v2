<script setup lang="ts">
import type { PublicTrackActivity } from '~/types/programme/track-page'

/**
 * Le temps fort de la journée en pleine couleur (le hackathon de la journée
 * jeunesse) : ce qui rend une journée plus vive qu'une autre sans changer de
 * système. Le texte posé sur la couleur est choisi d'après sa luminance.
 */

interface Props {
  activity: PublicTrackActivity
  editionSlug: string
  timezone: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { link } = useProgrammeSession()

const session = computed(() => props.activity.session)
const by = computed(() => {
  const lead = props.activity.organizations[0]
  return lead?.name ?? session.value.organization_name ?? ''
})
</script>

<template>
  <section
    class="grid gap-7 rounded-lg bg-(--color-day-solid) p-5.5 text-(--color-day-on-solid) md:items-center md:p-7"
    :class="session.cover ? 'md:grid-cols-[minmax(0,1fr)_240px]' : ''"
    aria-labelledby="journee-temps-fort-titre"
  >
    <div class="min-w-0">
      <p class="heading-condensed inline-flex items-center gap-2 text-[1.1rem] tracking-[0.06em]">
        <UiIcon name="star" size="1.05em" :stroke-width="2" />
        {{ t('track.highlight.mark') }}
      </p>
      <h2 id="journee-temps-fort-titre" class="mt-2 font-condensed text-[1.6rem] leading-[1.08] font-semibold text-balance text-inherit md:text-[1.9rem]">
        {{ tr(session.title) }}
      </h2>
      <p class="mt-3 font-condensed text-2xl font-bold tabular-nums">
        {{ t('track.part.hours', { start: time(session.starts_at, props.timezone), end: time(session.ends_at, props.timezone) }) }}
      </p>
      <p v-if="by" class="mt-1 max-w-(--measure) text-[0.95rem]">{{ t('track.highlight.by', { organization: by }) }}</p>
      <UiButton class="mt-4" variant="institutional" icon-trailing="arrow-right" :to="link(props.editionSlug, session)">
        {{ t('track.highlight.open') }}
      </UiButton>
    </div>
    <UiImage v-if="session.cover" :image="session.cover" ratio="4 / 3" rounded="var(--radius-md)" sizes="240px" />
  </section>
</template>
