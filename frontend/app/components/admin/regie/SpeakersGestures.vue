<script setup lang="ts">
import type { RegieSession, RegieSpeakersGesture, RegieSpeakersTarget } from '~/types/admin-regie'
import type { TimeZoneName } from '~/types/shared'

/** Les trois fenêtres de la rangée « Intervenants en ligne » ; la régie se relit en les fermant. */

interface Props {
  target: RegieSpeakersTarget | null
  timezone: TimeZoneName
  zoneLabel: string | null
  canManageMeeting: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:target': [value: RegieSpeakersTarget | null]; changed: [] }>()

const session = ref<RegieSession | null>(null)
watch(
  () => props.target,
  (target) => {
    if (target) session.value = target.session
  },
)

const isOpen = (gesture: RegieSpeakersGesture) => props.target?.gesture === gesture

function close(open: boolean): void {
  if (open) return
  emit('update:target', null)
  emit('changed')
}
</script>

<template>
  <AdminRegieSpeakersDialog
    :open="isOpen('register')"
    :session="session"
    :timezone="props.timezone"
    :zone-label="props.zoneLabel"
    :can-manage-meeting="props.canManageMeeting"
    @update:open="close"
  />
  <AdminRegieResendLinksDialog :open="isOpen('resend')" :session="session" @update:open="close" />
  <AdminRegieOwnerLinkDialog :open="isOpen('owner')" :session="session" @update:open="close" />
</template>
