<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'

/** L'activité en direct, à la une au-dessus de la liste ; `compact` en tête de la page d'une journée spéciale. */

interface Props {
  session: PublicScheduleRow
  editionSlug: string
  timezone: TimeZoneName
  compact?: boolean
}

const props = withDefaults(defineProps<Props>(), { compact: false })

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()
const { link, duration, titleParts } = useProgrammeSession()

const to = computed(() => link(props.editionSlug, props.session))
const title = computed(() => titleParts(tr(props.session.title)))
const summary = computed(() => (props.session.summary ? tr(props.session.summary) : ''))
const country = computed(() => (props.session.organization_country ? tr(props.session.organization_country) : ''))
</script>

<template>
  <article
    class="grid items-center font-sans"
    :class="
      props.compact
        ? 'gap-5 md:grid-cols-[minmax(0,300px)_minmax(0,1fr)]'
        : 'gap-6 border-b border-text py-10 lg:grid-cols-[minmax(0,600px)_minmax(0,1fr)] lg:gap-x-12'
    "
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
        :sizes="props.compact ? '(min-width: 768px) 300px, 100vw' : '(min-width: 1024px) 600px, 100vw'"
      />
      <span
        v-else
        class="absolute inset-0 flex items-center justify-center font-bold text-text-on-inverse/20"
        :class="props.compact ? 'text-3xl' : 'text-4xl lg:text-6xl'"
        aria-hidden="true"
      >
        {{ props.session.organization_acronym }}
      </span>
      <UiStatusBadge
        state="live"
        :label="t('programme.list.live')"
        class="absolute"
        :class="props.compact ? 'top-3 left-3' : 'top-4 left-4'"
      />
    </div>

    <div class="min-w-0">
      <p class="flex flex-wrap items-end gap-x-4 gap-y-1">
        <span class="text-[44px] leading-[0.9] font-light text-live tabular-nums" :class="{ 'lg:text-[56px]': !props.compact }">
          {{ time(props.session.starts_at, props.timezone) }}
        </span>
        <span class="pb-0.5 text-sm text-text-muted tabular-nums">
          {{ t('programme.list.until', { end: time(props.session.ends_at, props.timezone), duration: duration(props.session) }) }}
        </span>
      </p>
      <h3
        class="line-clamp-3 font-sans leading-[1.12] font-light text-balance text-text"
        :class="props.compact ? 'mt-3 text-xl' : 'mt-4.5 text-[26px] lg:text-[36px]'"
      >
        <b class="font-bold">{{ title.lead }}</b>{{ title.rest }}
      </h3>
      <p v-if="summary && !props.compact" class="mt-3.5 max-w-[52ch] text-[17px] leading-[1.5] text-text-muted">{{ summary }}</p>
      <p v-if="props.session.organization_name || props.session.organization_acronym" class="mt-3 text-sm text-text-muted">
        <b v-if="props.session.organization_acronym" class="font-bold text-text">{{ props.session.organization_acronym }}</b>
        <template v-if="props.session.organization_acronym && props.session.organization_name"> · </template>{{ props.session.organization_name }}<template v-if="country"> · {{ country }}</template>
      </p>
      <div class="flex flex-wrap gap-2.5" :class="props.compact ? 'mt-4' : 'mt-6'">
        <UiButton :to="`${to}#direct`" icon-trailing="arrow-right">{{ t('programme.lead.watch') }}</UiButton>
        <UiButton :to="to" variant="secondary">{{ t('programme.lead.open') }}</UiButton>
      </div>
    </div>
  </article>
</template>
