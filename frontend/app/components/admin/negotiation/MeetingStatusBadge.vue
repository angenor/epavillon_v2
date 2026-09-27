<script setup lang="ts">
import type { AdminMeetingStatus } from '~/types/negotiation-meetings'
import type { Intent } from '~/types/ui'

const props = defineProps<{ status: AdminMeetingStatus }>()

const { t } = useI18n()

// Règle des états du guide : « en cours » est une attention, pas une réussite.
const INTENT: Record<AdminMeetingStatus, Intent> = {
  draft: 'neutral',
  scheduled: 'success',
  ongoing: 'warning',
  completed: 'neutral',
  cancelled: 'danger',
}
</script>

<template>
  <UiBadge
    :intent="INTENT[props.status]"
    solid
    :label="t(`admin.negociations.reunions.status.${props.status}`)"
  />
</template>
