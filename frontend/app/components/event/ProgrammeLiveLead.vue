<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'activité en direct, à la une au-dessus de la liste : un bandeau bas (image
 * à gauche, étiquette, heure, titre, porteur et boutons à droite), sans
 * description, pour ne pas repousser la liste (direction condensée, 01/11).
 * `compact` : en tête de la page d'une journée spéciale.
 */

interface Props {
  session: PublicScheduleRow
  editionSlug: string
  timezone: TimeZoneName
  /** Ville de l'édition : « heure de Belém », plutôt que le nom technique du fuseau. */
  city?: string | null
  compact?: boolean
}

const props = withDefaults(defineProps<Props>(), { city: null, compact: false })

const { t } = useI18n()
const { tr } = useI18nText()
const { time, zoneLabel } = useDateTime()
const { link, duration, titleParts } = useProgrammeSession()

const to = computed(() => link(props.editionSlug, props.session))
const title = computed(() => titleParts(tr(props.session.title)))
const country = computed(() => (props.session.organization_country ? tr(props.session.organization_country) : ''))
const organization = computed(() => props.session.organization_acronym ?? props.session.organization_name ?? '')
</script>

<template>
  <article
    v-if="!props.compact"
    class="grid overflow-hidden rounded-xl border border-live-border bg-surface-raised shadow-md min-[860px]:grid-cols-[340px_minmax(0,1fr)]"
    :aria-label="t('programme.lead.label')"
  >
    <div class="relative min-h-37.5 bg-surface-inverse min-[860px]:min-h-49">
      <UiImage
        v-if="props.session.cover"
        :image="props.session.cover"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0"
        loading="eager"
        sizes="(min-width: 860px) 340px, 100vw"
      />
      <span
        v-else
        class="absolute inset-0 flex items-center justify-center font-condensed text-5xl font-bold text-text-on-inverse/20"
        aria-hidden="true"
      >
        {{ props.session.organization_acronym }}
      </span>
      <UiStatusBadge state="live" :label="t('programme.list.live')" class="absolute top-3 left-3" />
    </div>

    <div class="flex min-w-0 flex-col justify-between gap-2 p-4 min-[860px]:px-6.5 min-[860px]:pt-4 min-[860px]:pb-4.5">
      <p class="font-condensed text-[0.95rem] font-bold tracking-[0.05em] text-live uppercase">{{ t('programme.lead.kicker') }}</p>
      <div>
        <p class="flex flex-wrap items-baseline gap-x-1.5 font-condensed">
          <span class="text-[2.1rem] leading-none font-bold text-live tabular-nums">{{ time(props.session.starts_at, props.timezone) }}</span>
          <span class="text-[0.95rem] font-semibold tracking-[0.04em] text-text-secondary uppercase tabular-nums">
            {{ t('programme.lead.until', { end: time(props.session.ends_at, props.timezone), zone: zoneLabel(props.timezone, props.city ?? undefined) }) }}
          </span>
        </p>
        <h3 class="mt-1 line-clamp-2 font-condensed text-[1.45rem] leading-[1.1] font-semibold text-pretty text-heading">
          {{ title.lead }}{{ title.rest }}
        </h3>
      </div>
      <div class="flex flex-wrap items-center gap-x-5 gap-y-3">
        <p v-if="organization" class="flex min-w-0 items-center gap-2.5 text-[0.85rem] text-text-secondary">
          <OrganizationLogoMark
            :name="props.session.organization_name ?? organization"
            :acronym="props.session.organization_acronym"
            :logo="props.session.organization_logo ?? null"
            size="sm"
          />
          <span class="min-w-0">
            <b class="font-bold tracking-[0.02em] text-text">{{ organization }}</b><template v-if="country">{{ ' ' }}{{ country }}</template>
          </span>
        </p>
        <div class="flex flex-wrap gap-2.5 min-[860px]:ml-auto">
          <UiButton :to="`${to}#direct`" variant="danger" icon="broadcast">{{ t('programme.lead.join') }}</UiButton>
          <UiButton :to="to" variant="secondary">{{ t('programme.lead.open') }}</UiButton>
        </div>
      </div>
    </div>
  </article>

  <!-- Version de la page d'une journée spéciale, inchangée. -->
  <article v-else class="grid items-center gap-5 font-sans md:grid-cols-[minmax(0,300px)_minmax(0,1fr)]" :aria-label="t('programme.lead.label')">
    <div class="relative aspect-video overflow-hidden rounded-lg bg-surface-inverse">
      <UiImage
        v-if="props.session.cover"
        :image="props.session.cover"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0"
        loading="eager"
        sizes="(min-width: 768px) 300px, 100vw"
      />
      <span v-else class="absolute inset-0 flex items-center justify-center text-3xl font-bold text-text-on-inverse/20" aria-hidden="true">
        {{ props.session.organization_acronym }}
      </span>
      <UiStatusBadge state="live" :label="t('programme.list.live')" class="absolute top-3 left-3" />
    </div>

    <div class="min-w-0">
      <p class="flex flex-wrap items-end gap-x-4 gap-y-1">
        <span class="text-[44px] leading-[0.9] font-light text-live tabular-nums">
          {{ time(props.session.starts_at, props.timezone) }}
        </span>
        <span class="pb-0.5 text-sm text-text-muted tabular-nums">
          {{ t('programme.list.until', { end: time(props.session.ends_at, props.timezone), duration: duration(props.session) }) }}
        </span>
      </p>
      <h3 class="mt-3 line-clamp-3 font-sans text-xl leading-[1.12] font-light text-balance text-text">
        <b class="font-bold">{{ title.lead }}</b>{{ title.rest }}
      </h3>
      <p v-if="props.session.organization_name || props.session.organization_acronym" class="mt-3 text-sm text-text-muted">
        <b v-if="props.session.organization_acronym" class="font-bold text-text">{{ props.session.organization_acronym }}</b>
        <template v-if="props.session.organization_acronym && props.session.organization_name"> · </template>{{ props.session.organization_name }}<template v-if="country"> · {{ country }}</template>
      </p>
      <div class="mt-4 flex flex-wrap gap-2.5">
        <UiButton :to="`${to}#direct`" icon-trailing="arrow-right">{{ t('programme.lead.watch') }}</UiButton>
        <UiButton :to="to" variant="secondary">{{ t('programme.lead.open') }}</UiButton>
      </div>
    </div>
  </article>
</template>
