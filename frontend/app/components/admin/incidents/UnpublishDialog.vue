<script setup lang="ts">
import type { ManagedIncident } from '~/types/admin-incidents'

/**
 * DÉPUBLIER UN MESSAGE.
 *
 * UN SEUL CHAMP, ET IL EST FACULTATIF. Le prompt demande « dépublication en un
 * clic » ; la base, elle, accepte un motif (`unpublish_reason`) et c'est lui qui
 * rend l'historique lisible six mois plus tard. Le compromis tenu ici : le
 * bouton de la ligne mène DIRECTEMENT ici, le motif est offert mais jamais
 * exigé, et la touche Entrée suffit à valider. Un dialogue à quatre champs
 * aurait fait de la dépublication un geste qu'on remet à plus tard — pendant que
 * le bandeau rouge reste en ligne.
 *
 * LE MESSAGE EST RAPPELÉ, pas seulement son titre : on ne retire pas un bandeau
 * de mémoire pendant une COP, où trois messages peuvent se ressembler.
 */

interface Props {
  open: boolean
  incident: ManagedIncident | null
  submitting?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [reason: string | null]
}>()

const { t } = useI18n()
const { tr } = useI18nText()

const reason = ref('')

// Rouvrir le dialogue sur un autre message ne doit pas traîner le motif du
// précédent : ce serait signer une décision avec la justification d'une autre.
watch(
  () => [props.open, props.incident?.incident_id],
  ([open]) => {
    if (open) reason.value = ''
  },
)

function submit(): void {
  emit('submit', reason.value.trim() || null)
}
</script>

<template>
  <UiModal
    :open="open"
    :title="t('admin.incident.list.unpublish.title')"
    :description="t('admin.incident.list.unpublish.description')"
    @update:open="emit('update:open', $event)"
  >
    <div v-if="incident" class="space-y-4">
      <UiAlert
        v-if="incident.state === 'active'"
        intent="warning"
        compact
        :message="t('admin.incident.list.unpublish.activeWarning')"
      />

      <div class="rounded-md border border-border bg-surface-sunken p-3">
        <p v-if="incident.title" class="font-medium">{{ tr(incident.title) }}</p>
        <p class="max-w-(--measure) text-sm text-text-muted" :class="incident.title ? 'mt-1' : ''">
          {{ tr(incident.message) }}
        </p>
      </div>

      <UiTextarea
        v-model="reason"
        :label="t('admin.incident.list.unpublish.reason')"
        :hint="t('admin.incident.list.unpublish.reasonHint')"
        :rows="2"
        :maxlength="280"
        :disabled="submitting"
      />

      <UiAlert v-if="error" intent="danger" compact :message="error" />
    </div>

    <template #footer>
      <UiButton variant="ghost" :disabled="submitting" @click="emit('update:open', false)">
        {{ t('common.actions.cancel') }}
      </UiButton>
      <UiButton variant="danger" :loading="submitting" icon="eye-off" @click="submit">
        {{ t('admin.incident.list.unpublish.submit') }}
      </UiButton>
    </template>
  </UiModal>
</template>
