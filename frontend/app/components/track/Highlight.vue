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
const { link, titleParts } = useProgrammeSession()

const session = computed(() => props.activity.session)
const title = computed(() => titleParts(tr(session.value.title)))
const by = computed(() => {
  const lead = props.activity.organizations[0]
  return lead?.name ?? session.value.organization_name ?? ''
})
</script>

<template>
  <section
    class="grid gap-5 rounded-lg bg-(--color-day-solid) p-6 text-(--color-day-on-solid) md:items-center md:p-8"
    :class="session.cover ? 'md:grid-cols-[minmax(0,1fr)_minmax(0,260px)]' : ''"
    aria-labelledby="journee-temps-fort-titre"
  >
    <div class="min-w-0">
      <p class="text-xs font-bold uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('track.highlight.kicker', { start: time(session.starts_at, props.timezone), end: time(session.ends_at, props.timezone) }) }}
      </p>
      <h2 id="journee-temps-fort-titre" class="mt-2 font-sans text-[1.5rem] leading-tight font-light text-balance text-inherit md:text-[2.25rem]">
        <b class="font-bold">{{ title.lead }}</b>{{ title.rest }}
      </h2>
      <p v-if="by" class="mt-3 max-w-(--measure)">{{ t('track.highlight.by', { organization: by }) }}</p>
      <UiButton class="mt-5" variant="institutional" icon-trailing="arrow-right" :to="link(props.editionSlug, session)">
        {{ t('track.highlight.open') }}
      </UiButton>
    </div>
    <UiImage v-if="session.cover" :image="session.cover" ratio="4 / 3" rounded="var(--radius-lg)" sizes="260px" />
  </section>
</template>
