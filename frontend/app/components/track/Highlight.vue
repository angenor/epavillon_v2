<script setup lang="ts">
import type { PublicTrackActivity, PublicTrackEntry } from '~/types/programme/track-page'

/**
 * Le temps fort de la journée en pleine couleur (le hackathon de la journée
 * jeunesse) : ce qui rend une journée plus vive qu'une autre sans changer de
 * système. Le texte posé sur la couleur est choisi d'après sa luminance.
 *
 * Une activité ou un moment du déroulé (02/11) : un moment n'a ni page, ni
 * organisation, ni image ; le bandeau garde son titre et ses heures.
 */

interface Props {
  activity?: PublicTrackActivity | null
  moment?: Extract<PublicTrackEntry, { kind: 'moment' }> | null
  editionSlug: string
  timezone: string
}

const props = withDefaults(defineProps<Props>(), { activity: null, moment: null })

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { link } = useProgrammeSession()

const session = computed(() => props.activity?.session ?? null)
const title = computed(() => {
  if (session.value) return tr(session.value.title)
  return props.moment ? tr(props.moment.moment.label) : ''
})
const hours = computed(() => {
  const start = session.value?.starts_at ?? props.moment?.starts_at
  const end = session.value?.ends_at ?? props.moment?.ends_at
  if (!start) return ''
  return end
    ? t('track.part.hours', { start: time(start, props.timezone), end: time(end, props.timezone) })
    : time(start, props.timezone)
})
const by = computed(() => {
  if (!props.activity) return ''
  const lead = props.activity.organizations[0]
  return lead?.name ?? props.activity.session.organization_name ?? ''
})
const note = computed(() => (props.moment?.moment.description ? tr(props.moment.moment.description) : ''))
const cover = computed(() => session.value?.cover ?? null)

/** Le lien d'un moment (10/11) : l'article du hackathon, faute de fiche. Un chemin du site reste dans l'onglet. */
const localePath = useLocalePath()
const momentLink = computed<{ label: string; to?: string; href?: string } | null>(() => {
  const moment = props.moment?.moment
  if (session.value || !moment?.link_url || !moment.link_label) return null
  const label = tr(moment.link_label)
  return moment.link_url.startsWith('/') ? { label, to: localePath(moment.link_url) } : { label, href: moment.link_url }
})
</script>

<template>
  <section
    class="grid gap-7 rounded-lg bg-(--color-day-solid) p-5.5 text-(--color-day-on-solid) md:items-center md:p-7"
    :class="cover ? 'md:grid-cols-[minmax(0,1fr)_240px]' : ''"
    aria-labelledby="journee-temps-fort-titre"
  >
    <div class="min-w-0">
      <p class="heading-condensed inline-flex items-center gap-2 text-[1.1rem] tracking-[0.06em]">
        <UiIcon name="star" size="1.05em" :stroke-width="2" />
        {{ t('track.highlight.mark') }}
      </p>
      <h2 id="journee-temps-fort-titre" class="mt-2 font-condensed text-[1.6rem] leading-[1.08] font-semibold text-balance text-inherit md:text-[1.9rem]">
        {{ title }}
      </h2>
      <p class="mt-3 font-condensed text-2xl font-bold tabular-nums">{{ hours }}</p>
      <p v-if="by" class="mt-1 max-w-(--measure) text-[0.95rem]">{{ t('track.highlight.by', { organization: by }) }}</p>
      <p v-else-if="note" class="mt-1 max-w-(--measure) text-[0.95rem]">{{ note }}</p>
      <UiButton v-if="session" class="mt-4" variant="institutional" icon-trailing="arrow-right" :to="link(props.editionSlug, session)">
        {{ t('track.highlight.open') }}
      </UiButton>
      <UiButton
        v-else-if="momentLink"
        class="mt-4"
        variant="institutional"
        :icon-trailing="momentLink.to ? 'arrow-right' : 'external-link'"
        :to="momentLink.to"
        :href="momentLink.href"
      >
        {{ momentLink.label }}
      </UiButton>
    </div>
    <UiImage v-if="cover" :image="cover" ratio="4 / 3" rounded="var(--radius-md)" sizes="240px" />
  </section>
</template>
