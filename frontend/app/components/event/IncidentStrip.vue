<script setup lang="ts">
import type { ActiveIncident } from '~/types/live'

/**
 * Le message qui vaut pour toute l'édition, une journée ou la plateforme : une
 * bande pleine largeur sous la barre de navigation. Un seul à la fois, le plus
 * important (l'API trie). Les messages d'une activité vont dans son billet et
 * sur sa carte, jamais ici.
 */

interface Props {
  incidents: ActiveIncident[]
}

const props = defineProps<Props>()
const { t } = useI18n()
const { tr } = useI18nText()

const STORAGE_KEY = 'epavillon:messages-masques'
const dismissed = ref<string[]>([])

onMounted(() => {
  try {
    dismissed.value = JSON.parse(sessionStorage.getItem(STORAGE_KEY) ?? '[]') as string[]
  } catch {
    dismissed.value = []
  }
})

const shown = computed(
  () => props.incidents.filter((incident) => isEditionWide(incident) && !dismissed.value.includes(incident.incident_id))[0] ?? null,
)
const level = computed(() => (shown.value ? incidentLevel(shown.value.severity) : 'information'))

function dismiss(): void {
  if (!shown.value) return
  dismissed.value = [...dismissed.value, shown.value.incident_id]
  try {
    sessionStorage.setItem(STORAGE_KEY, JSON.stringify(dismissed.value))
  } catch {
    // Navigation privée : le message reviendra au prochain chargement, rien de plus.
  }
}
</script>

<template>
  <div
    v-if="shown"
    data-direction="condensee"
    class="incident-strip full-bleed border-b"
    :class="`is-${level}`"
    :role="level === 'interruption' ? 'alert' : 'status'"
  >
    <div class="mx-auto flex max-w-[1280px] items-center gap-3 px-4 py-2 sm:px-6">
      <UiIcon :name="INCIDENT_LEVEL_ICONS[level]" size="1.25rem" :stroke-width="2.2" class="strip-ink shrink-0" />
      <p class="min-w-0 flex-1 py-1.5 text-[0.92rem] leading-snug">
        <b class="strip-ink mr-2 font-condensed text-[1.02rem] font-bold tracking-[0.04em] uppercase">{{ t(`incident-notice.level.${level}`) }}</b>
        <span v-if="shown.scope === 'event_day' && shown.target_label" class="mr-1.5 font-semibold">{{ shown.target_label }} :</span>
        <span v-if="shown.title" class="mr-1.5 font-semibold">{{ tr(shown.title) }}.</span>{{ tr(shown.message) }}
        <a v-if="shown.action_url" :href="shown.action_url" class="ml-1.5 underline" target="_blank" rel="noopener">{{ t('incident-notice.more') }}</a>
      </p>
      <button
        v-if="canDismissIncident(shown)"
        type="button"
        class="grid size-(--target-min) shrink-0 cursor-pointer place-items-center rounded-md text-text-muted hover:bg-surface-hover"
        :aria-label="t('incident-notice.dismiss')"
        @click="dismiss"
      >
        <UiIcon name="close" size="1.1rem" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.incident-strip.is-information {
  border-color: var(--color-info-border);
  background: var(--color-info-surface);
  color: var(--color-text);
  --strip-ink: var(--color-info);
}

.incident-strip.is-attention {
  border-color: var(--color-warning-border);
  background: var(--color-warning-surface);
  color: var(--color-text);
  --strip-ink: var(--color-warning);
}

.incident-strip.is-interruption {
  border-color: var(--color-danger-solid);
  background: var(--color-danger-solid);
  color: var(--color-danger-contrast);
  --strip-ink: var(--color-danger-contrast);
}

.strip-ink {
  color: var(--strip-ink);
}
</style>
