<script setup lang="ts">
import type { ActiveIncident } from '~/types/live'
import type { TimeZoneName } from '~/types/shared'

/**
 * Un message de la régie là où l'on regarde déjà : en tête du billet
 * (`block`), sur la barre collée du téléphone (`line`), sur la carte du
 * programme (`flag`). Maquette `direction-condensee/messages-regie.html`.
 */

interface Props {
  incident: ActiveIncident
  variant?: 'block' | 'line' | 'flag'
  timezone?: TimeZoneName
}

const props = withDefaults(defineProps<Props>(), { variant: 'block', timezone: undefined })

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()

const level = computed(() => incidentLevel(props.incident.severity))
const icon = computed(() => INCIDENT_LEVEL_ICONS[level.value])
const label = computed(() => t(`incident-notice.level.${level.value}`))
const title = computed(() => (props.incident.title ? tr(props.incident.title) : ''))
const message = computed(() => tr(props.incident.message))
const published = computed(() =>
  props.timezone ? t('incident-notice.published', { time: time(props.incident.display_from, props.timezone) }) : '',
)
</script>

<template>
  <div
    v-if="props.variant === 'block'"
    class="incident-notice grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 gap-y-1 border-l-4 px-4 py-3"
    :class="`is-${level}`"
    :role="level === 'interruption' ? 'alert' : 'status'"
  >
    <UiIcon :name="icon" size="1.35rem" :stroke-width="2.2" class="notice-ink row-span-2 mt-0.5" />
    <p class="flex flex-wrap items-baseline gap-x-2">
      <b class="notice-ink font-condensed text-[1.02rem] font-bold tracking-[0.04em] uppercase">{{ label }}</b>
      <small v-if="published" class="notice-muted text-xs">{{ published }}</small>
    </p>
    <div class="text-[0.92rem] leading-snug">
      <p v-if="title" class="font-semibold">{{ title }}</p>
      <p>{{ message }}</p>
      <a v-if="props.incident.action_url" :href="props.incident.action_url" class="mt-1 inline-block underline" target="_blank" rel="noopener">
        {{ t('incident-notice.more') }}
      </a>
    </div>
  </div>

  <p
    v-else-if="props.variant === 'line'"
    class="incident-notice flex items-center gap-2 border-t-3 px-4 py-2 text-sm sm:px-6 lg:px-12"
    :class="`is-${level}`"
    role="status"
  >
    <UiIcon :name="icon" size="1.1rem" :stroke-width="2.2" class="notice-ink shrink-0" />
    <span class="min-w-0 truncate">
      <b class="notice-ink mr-1.5 font-condensed font-bold tracking-[0.04em] uppercase">{{ label }}</b>{{ title || message }}
    </span>
  </p>

  <p v-else class="incident-flag flex items-center gap-1.5 text-[0.8rem] font-semibold" :class="`is-${level}`">
    <UiIcon :name="icon" size="1em" :stroke-width="2.2" class="shrink-0" />
    <span class="min-w-0 truncate">{{ title || message }}</span>
  </p>
</template>

<style scoped>
.incident-notice.is-information {
  --notice-bg: var(--color-info-surface);
  --notice-line: var(--color-info);
  --notice-ink: var(--color-info);
  --notice-text: var(--color-text);
  --notice-muted: var(--color-text-muted);
}

.incident-notice.is-attention {
  --notice-bg: var(--color-warning-surface);
  --notice-line: var(--color-warning);
  --notice-ink: var(--color-warning);
  --notice-text: var(--color-text);
  --notice-muted: var(--color-text-muted);
}

.incident-notice.is-interruption {
  --notice-bg: var(--color-danger-solid);
  --notice-line: var(--color-danger-solid);
  --notice-ink: var(--color-danger-contrast);
  --notice-text: var(--color-danger-contrast);
  --notice-muted: var(--color-danger-contrast);
}

.incident-notice {
  border-color: var(--notice-line);
  background: var(--notice-bg);
  color: var(--notice-text);
}

.notice-ink {
  color: var(--notice-ink);
}

.notice-muted {
  color: var(--notice-muted);
}

.is-interruption .notice-muted {
  opacity: 0.85;
}

.incident-flag.is-information {
  color: var(--color-info);
}

.incident-flag.is-attention {
  color: var(--color-warning);
}

.incident-flag.is-interruption {
  color: var(--color-danger);
}
</style>
