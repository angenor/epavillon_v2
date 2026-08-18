<script setup lang="ts">
import type { SetPersonStatusPayload, UserDetail } from '~/types/admin-users'
import type { PersonStatus } from '~/types/identity'

/**
 * SUSPENDRE, BLOQUER, RÉTABLIR — avec motif.
 *
 * TROIS STATUTS, PAS QUATRE. `anonymized` existe dans l'ENUM et n'apparaît pas
 * ici : il ne se pose que par `identity.anonymize_person()`, depuis une demande
 * d'effacement. L'offrir à côté d'une suspension de quinze jours reviendrait à
 * poser la destruction irréversible d'une identité dans un menu de modération.
 *
 * TOUTE SUSPENSION A UN TERME — `ck_people_suspension_window` l'exige, et c'est
 * une décision du modèle plutôt qu'une formalité : une suspension sans date de
 * fin est un blocage qui n'ose pas dire son nom, et c'est ainsi qu'un compte
 * reste fermé trois ans parce que personne ne se souvient pourquoi. Le blocage,
 * lui, est durable et assumé.
 *
 * LE MOTIF EST CE QUE LA PERSONNE LIRA. `people.status_reason` n'est pas une note
 * interne : c'est le texte qui explique une porte fermée. Un « RAS » y serait
 * pire que rien.
 *
 * LE COMPTE ET LA PERSONNE RESTENT DEUX CHOSES. Un verrou après échecs
 * (`accounts.locked_until`) n'est pas une suspension : il expire seul, et ce
 * dialogue ne le touche pas. Le dire évite qu'on suspende quelqu'un dont le
 * compte était simplement verrouillé douze minutes.
 */

interface Props {
  open: boolean
  user: UserDetail | null
  submitting?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [payload: Omit<SetPersonStatusPayload, 'person_id'>]
}>()

const { t } = useI18n()

const status = ref<Exclude<PersonStatus, 'anonymized'>>('suspended')
const reason = ref('')
const until = ref('')
const revokeSessions = ref(true)

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return
    // Le dialogue s'ouvre sur le geste ATTENDU : rétablir un compte restreint,
    // suspendre un compte actif. Proposer « bloquer » par défaut à quelqu'un qui
    // vient rétablir un compte est la meilleure façon de produire une erreur.
    status.value = props.user && props.user.status !== 'active' ? 'active' : 'suspended'
    reason.value = ''
    until.value = ''
    revokeSessions.value = true
  },
)

const needsDeadline = computed(() => status.value === 'suspended')
const needsReason = computed(() => status.value !== 'active')

const isValid = computed(() => {
  if (needsReason.value && reason.value.trim().length < 5) return false
  if (needsDeadline.value && !until.value) return false
  return true
})

function submit(): void {
  if (!isValid.value) return
  emit('submit', {
    status: status.value,
    reason: reason.value.trim(),
    suspended_until: needsDeadline.value ? `${until.value}T23:59:59Z` : null,
    revoke_sessions: revokeSessions.value && status.value !== 'active',
  })
}
</script>

<template>
  <UiModal
    :open="open"
    :title="t('admin.user.status.dialog.title')"
    :description="user ? t('admin.user.status.dialog.description', { name: user.display_name }) : ''"
    @update:open="emit('update:open', $event)"
  >
    <form v-if="user" class="space-y-4" @submit.prevent="submit">
      <UiRadio
        v-model="status"
        :label="t('admin.user.status.dialog.field.status')"
        :options="[
          { value: 'active', label: t('admin.user.status.dialog.option.active'), description: t('admin.user.status.dialog.help.active') },
          { value: 'suspended', label: t('admin.user.status.dialog.option.suspended'), description: t('admin.user.status.dialog.help.suspended') },
          { value: 'blocked', label: t('admin.user.status.dialog.option.blocked'), description: t('admin.user.status.dialog.help.blocked') },
        ]"
        required
      />

      <UiDatePicker
        v-if="needsDeadline"
        v-model="until"
        :label="t('admin.user.status.dialog.field.until')"
        :hint="t('admin.user.status.dialog.field.untilHint')"
        required
      />

      <UiFormField
        v-if="needsReason"
        :label="t('admin.user.status.dialog.field.reason')"
        :hint="t('admin.user.status.dialog.field.reasonHint')"
        required
      >
        <UiTextarea v-model="reason" :rows="3" :maxlength="500" auto-grow required />
      </UiFormField>

      <UiSwitch
        v-if="needsReason"
        v-model="revokeSessions"
        :label="t('admin.user.status.dialog.field.revokeSessions')"
        :hint="t('admin.user.status.dialog.field.revokeSessionsHint')"
      />

      <!-- LE VERROU DE COMPTE N'EST PAS UNE SUSPENSION. -->
      <UiAlert
        v-if="user.accounts.some((account) => account.locked_until)"
        intent="info"
        compact
        :message="t('admin.user.status.dialog.lockedNotice')"
      />

      <UiAlert v-if="error" intent="danger" :message="error" />

      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" type="button" @click="emit('update:open', false)">
          {{ t('common.actions.cancel') }}
        </UiButton>
        <UiButton
          :variant="status === 'active' ? 'primary' : 'danger'"
          type="submit"
          :disabled="!isValid"
          :loading="submitting"
        >
          {{ t(`admin.user.status.dialog.submit.${status}`) }}
        </UiButton>
      </div>
    </form>
  </UiModal>
</template>
