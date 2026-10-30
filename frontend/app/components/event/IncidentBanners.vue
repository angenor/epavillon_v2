<script setup lang="ts">
import type { ActiveIncident } from '~/types/live'
import type { TimeZoneName } from '~/types/shared'

/**
 * Les messages d'incident d'une page publique : programmation, activité, salle,
 * journée spéciale. Trois au plus, le plus grave en tête, le reste en « +N »
 * (règle des pastilles de la charte). L'API trie ; on affiche dans l'ordre reçu.
 */

interface Props {
  incidents: ActiveIncident[]
  timezone?: TimeZoneName
  zoneLabel?: string
}

const props = defineProps<Props>()
const { t } = useI18n()

const AFFICHES = 3
const shown = computed(() => props.incidents.slice(0, AFFICHES))
const more = computed(() => Math.max(props.incidents.length - AFFICHES, 0))
</script>

<template>
  <div v-if="shown.length" class="flex flex-col gap-2">
    <UiIncidentBanner
      v-for="incident in shown"
      :key="incident.incident_id"
      :severity="incident.severity"
      :title="incident.title"
      :message="incident.message"
      :scope="incident.scope"
      :action-url="incident.action_url"
      :target-label="incident.target_label"
      :dismissible="incident.is_dismissible"
      :display-until="incident.display_until"
      :timezone="timezone"
      :zone-label="zoneLabel"
      standalone
    />
    <p v-if="more" class="text-sm text-text-muted">
      {{ t('incident-banners.more', { count: more }) }}
    </p>
  </div>
</template>
