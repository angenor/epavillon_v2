<script setup lang="ts">
import type { AdminFrancophoneMeeting } from '~/types/negotiation-meetings'

/** Annuler, motif obligatoire : il est montré aux inscrites telles quelles. */

const props = defineProps<{ open: boolean; meeting: AdminFrancophoneMeeting }>()
const emit = defineEmits<{ 'update:open': [value: boolean]; cancelled: [meeting: AdminFrancophoneMeeting] }>()

const { t } = useI18n()
const api = useApi()

const motif = ref('')
const envoi = ref(false)
const erreur = ref<string | null>(null)
const tente = ref(false)

watch(
  () => props.open,
  (ouvert) => {
    if (!ouvert) return
    motif.value = ''
    erreur.value = null
    tente.value = false
  },
)

const manque = computed(() => (tente.value && !motif.value.trim() ? t('admin.negociations.reunions.cancel.reasonRequired') : undefined))

async function annuler(): Promise<void> {
  tente.value = true
  if (!motif.value.trim() || envoi.value) return
  envoi.value = true
  erreur.value = null
  try {
    emit('cancelled', await api.adminNegotiations.annulerUneReunion(props.meeting.id, motif.value.trim()))
    emit('update:open', false)
  } catch (e) {
    erreur.value = apiErrorMessage(e, t)
  } finally {
    envoi.value = false
  }
}
</script>

<template>
  <UiModal
    :open="props.open"
    dismissible
    :title="t('admin.negociations.reunions.cancel.title')"
    :description="t('admin.negociations.reunions.cancel.question')"
    @update:open="(v: boolean) => emit('update:open', v)"
  >
    <UiAlert
      v-if="props.meeting.registered_count + props.meeting.waitlisted_count > 0"
      intent="warning"
      compact
      :message="
        t(
          'admin.negociations.reunions.cancel.people',
          props.meeting.registered_count + props.meeting.waitlisted_count,
        )
      "
    />
    <UiTextarea
      v-model="motif"
      class="mt-4"
      :label="t('admin.negociations.reunions.cancel.reason')"
      :hint="t('admin.negociations.reunions.cancel.reasonHint')"
      :error="manque"
      :rows="3"
      :maxlength="500"
      required
    />
    <UiAlert v-if="erreur" class="mt-4" intent="danger" live :message="erreur" />
    <div class="mt-5 flex flex-wrap justify-end gap-3">
      <UiButton variant="ghost" :disabled="envoi" @click="emit('update:open', false)">
        {{ t('admin.negociations.reunions.cancel.keep') }}
      </UiButton>
      <UiButton variant="danger" :loading="envoi" @click="annuler">
        {{ t('admin.negociations.reunions.cancel.confirm') }}
      </UiButton>
    </div>
  </UiModal>
</template>
