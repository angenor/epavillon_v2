<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'

/** L'activité en direct, à la une au-dessus de la liste. */

interface Props {
  session: PublicScheduleRow
  editionSlug: string
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { link, duration, titleParts } = useProgrammeSession()

// Mêmes règles que les lignes de `EventProgrammeDayList` : durée et gras du titre.


const to = computed(() => link(props.editionSlug, props.session))
const title = computed(() => titleParts(tr(props.session.title)))
const summary = computed(() => (props.session.summary ? tr(props.session.summary) : ''))
const country = computed(() => (props.session.organization_country ? tr(props.session.organization_country) : ''))
</script>

<template>
  <article
    class="grid items-center gap-6 border-b border-text py-10 font-sans lg:grid-cols-[minmax(0,600px)_minmax(0,1fr)] lg:gap-x-12"
    :aria-label="t('programme.lead.label')"
  >
    <div class="relative aspect-video overflow-hidden rounded-lg bg-surface-inverse">
      <UiImage
        v-if="props.session.cover"
        :image="props.session.cover"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0"
        loading="eager"
        sizes="(min-width: 1024px) 600px, 100vw"
      />
      <span
        v-else
        class="absolute inset-0 flex items-center justify-center text-4xl font-bold text-text-on-inverse/20 lg:text-6xl"
        aria-hidden="true"
      >
        {{ props.session.organization_acronym }}
      </span>
      <UiStatusBadge state="live" :label="t('programme.list.live')" class="absolute top-4 left-4" />
    </div>

    <div class="min-w-0">
      <p class="flex flex-wrap items-end gap-x-4 gap-y-1">
        <span class="text-[44px] leading-[0.9] font-light text-live tabular-nums lg:text-[56px]">
          {{ time(props.session.starts_at, props.timezone) }}
        </span>
        <span class="pb-0.5 text-sm text-text-muted tabular-nums">
          {{ t('programme.list.until', { end: time(props.session.ends_at, props.timezone), duration: duration(props.session) }) }}
        </span>
      </p>
      <h3 class="mt-4.5 font-sans text-[26px] leading-[1.12] font-light text-balance text-text lg:text-[36px]">
        <b class="font-bold">{{ title.lead }}</b>{{ title.rest }}
      </h3>
      <p v-if="summary" class="mt-3.5 max-w-[52ch] text-[17px] leading-[1.5] text-text-muted">{{ summary }}</p>
      <p v-if="props.session.organization_name || props.session.organization_acronym" class="mt-3 text-sm text-text-muted">
        <b v-if="props.session.organization_acronym" class="font-bold text-text">{{ props.session.organization_acronym }}</b>
        <template v-if="props.session.organization_acronym && props.session.organization_name"> — </template>{{ props.session.organization_name }}<template v-if="country"> · {{ country }}</template>
      </p>
      <div class="mt-6 flex flex-wrap gap-2.5">
        <UiButton :to="`${to}#direct`" icon-trailing="arrow-right">{{ t('programme.lead.watch') }}</UiButton>
        <UiButton :to="to" variant="secondary">{{ t('programme.lead.open') }}</UiButton>
      </div>
    </div>
  </article>
</template>
