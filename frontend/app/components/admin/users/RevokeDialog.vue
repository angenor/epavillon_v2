<script setup lang="ts">
import type { RoleAssignmentView } from '~/types/admin-users'

/**
 * RETIRER UN RÔLE — avec motif, et le motif est OBLIGATOIRE.
 *
 * Ce n'est pas une précaution d'écran : `revoked_reason` a été ajoutée au modèle
 * pour cet écran, parce qu'une date de retrait nue ne répond pas à la question
 * qu'on posera six mois plus tard — « pourquoi cette personne n'est-elle plus au
 * comité ? ». Laisser le champ facultatif, c'est garantir qu'il restera vide.
 *
 * LE RETRAIT NE SUPPRIME RIEN, et le dialogue le dit. `role_assignments` n'a pas
 * de suppression : la ligne reste, avec son octroi, son retrait, ses deux auteurs
 * et ses deux motifs. Sans cette phrase, on hésite à retirer un rôle de peur de
 * perdre la trace de l'avoir accordé.
 */

interface Props {
  open: boolean
  assignment: RoleAssignmentView | null
  personName: string
  submitting?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: [reason: string]
}>()

const { t } = useI18n()
const { tr } = useI18nText()

const reason = ref('')

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) reason.value = ''
  },
)

const isValid = computed(() => reason.value.trim().length >= 5)
</script>

<template>
  <UiModal
    :open="open"
    :title="t('admin.user.roles.revoke.title')"
    :description="
      assignment
        ? t('admin.user.roles.revoke.description', {
            role: tr(assignment.role_label),
            name: personName,
          })
        : ''
    "
    @update:open="emit('update:open', $event)"
  >
    <form v-if="assignment" class="space-y-4" @submit.prevent="isValid && emit('confirm', reason.trim())">
      <div class="rounded-md border border-border bg-surface-sunken p-3">
        <AdminUsersRoleBadge :assignment="assignment" />
        <p class="mt-2 text-sm text-text-muted">
          {{ t('admin.user.roles.revoke.effect', { count: assignment.role_permissions.length }) }}
        </p>
      </div>

      <UiFormField
        :label="t('admin.user.roles.revoke.reason')"
        :hint="t('admin.user.roles.revoke.reasonHint')"
        required
      >
        <UiTextarea v-model="reason" :rows="3" :maxlength="500" auto-grow required />
      </UiFormField>

      <!-- CE QUI RESTE APRÈS LE RETRAIT. -->
      <UiAlert intent="info" compact :message="t('admin.user.roles.revoke.kept')" />

      <UiAlert v-if="error" intent="danger" :message="error" />

      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" type="button" @click="emit('update:open', false)">
          {{ t('common.actions.cancel') }}
        </UiButton>
        <UiButton variant="danger" type="submit" :disabled="!isValid" :loading="submitting">
          {{ t('admin.user.roles.revoke.submit') }}
        </UiButton>
      </div>
    </form>
  </UiModal>
</template>
