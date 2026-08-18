<script setup lang="ts">
import type { HandlePrivacyRequestPayload, PrivacyRequestView } from '~/types/admin-users'
import type { TimeZoneName } from '~/types/shared'

/**
 * TRAITER UNE DEMANDE RGPD.
 *
 * QUATRE ACTIONS, ET LA QUATRIÈME N'EST PAS UN STATUT. « Prendre en charge »,
 * « clore » et « rejeter » font avancer un dossier ; « anonymiser » DÉTRUIT une
 * identité. `identity.anonymize_person()` purge le nom, l'adresse, le téléphone,
 * la biographie, supprime les comptes et révoque les sessions — en conservant les
 * agrégats de participation, pour que les compteurs d'une COP passée ne
 * s'effondrent pas. C'est irréversible, et c'est pourquoi elle est séparée,
 * rouge, et exige une confirmation par le nom.
 *
 * ELLE NE VAUT QUE POUR UN EFFACEMENT. L'exécuter sur une demande d'export
 * détruirait l'identité de quelqu'un qui ne demandait qu'une copie de ses
 * données : l'action n'apparaît donc pas pour les deux autres natures, et l'API
 * la refuse (`wrong_type`) si l'écran se trompait.
 *
 * LA RÉSOLUTION EST OBLIGATOIRE POUR CLORE OU REJETER. `privacy_requests.resolution`
 * est ce qu'on relira si l'autorité de contrôle demande des comptes — et le rejet
 * d'une demande sans motif écrit est indéfendable.
 */

interface Props {
  open: boolean
  request: PrivacyRequestView | null
  timezone: TimeZoneName
  submitting?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [payload: Omit<HandlePrivacyRequestPayload, 'request_id'>]
}>()

const { t } = useI18n()
const { date } = useDateTime()

const action = ref<HandlePrivacyRequestPayload['action']>('start')
const resolution = ref('')
const confirmation = ref('')

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return
    action.value = props.request?.status === 'received' ? 'start' : 'complete'
    resolution.value = ''
    confirmation.value = ''
  },
)

const isClosed = computed(
  () => props.request?.status === 'completed' || props.request?.status === 'rejected',
)

const canAnonymize = computed(() => props.request?.request_type === 'erasure')

const needsResolution = computed(() => action.value !== 'start')

/** Confirmation par le nom : le même garde-fou que la fusion d'organisations. */
const needsNameConfirmation = computed(() => action.value === 'anonymize')

const isValid = computed(() => {
  if (needsResolution.value && resolution.value.trim().length < 5) return false
  if (needsNameConfirmation.value && confirmation.value.trim() !== props.request?.person_name) return false
  return true
})
</script>

<template>
  <UiModal
    :open="open"
    size="lg"
    :title="t('admin.user.privacy.dialog.title')"
    :description="
      request
        ? t('admin.user.privacy.dialog.description', {
            type: t(`admin.user.privacy.type.${request.request_type}`),
            name: request.person_name,
          })
        : ''
    "
    @update:open="emit('update:open', $event)"
  >
    <div v-if="request" class="space-y-4">
      <dl class="grid gap-3 rounded-md border border-border bg-surface-sunken p-3 text-sm sm:grid-cols-3">
        <div>
          <dt class="text-text-muted">{{ t('admin.user.privacy.dialog.received') }}</dt>
          <dd>{{ date(request.created_at, timezone) }}</dd>
        </div>
        <div>
          <dt class="text-text-muted">{{ t('admin.user.privacy.dialog.due') }}</dt>
          <dd :class="request.is_overdue && 'text-danger'">{{ date(request.due_at, timezone) }}</dd>
        </div>
        <div>
          <dt class="text-text-muted">{{ t('admin.user.privacy.columns.status') }}</dt>
          <dd>{{ t(`admin.user.privacy.status.${request.status}`) }}</dd>
        </div>
      </dl>

      <!-- DÉJÀ TRAITÉE : on lit, on ne réécrit pas. -->
      <template v-if="isClosed">
        <p v-if="request.resolution" class="max-w-(--measure)">« {{ request.resolution }} »</p>
        <div class="flex justify-end">
          <UiButton variant="ghost" @click="emit('update:open', false)">
            {{ t('common.actions.close') }}
          </UiButton>
        </div>
      </template>

      <form v-else class="space-y-4" @submit.prevent="isValid && emit('submit', { action, resolution: resolution.trim() })">
        <UiRadio
          v-model="action"
          :label="t('admin.user.privacy.dialog.field.action')"
          :options="[
            ...(request.status === 'received'
              ? [{ value: 'start', label: t('admin.user.privacy.dialog.action.start'), description: t('admin.user.privacy.dialog.help.start') }]
              : []),
            { value: 'complete', label: t('admin.user.privacy.dialog.action.complete'), description: t('admin.user.privacy.dialog.help.complete') },
            { value: 'reject', label: t('admin.user.privacy.dialog.action.reject'), description: t('admin.user.privacy.dialog.help.reject') },
            ...(canAnonymize
              ? [{ value: 'anonymize', label: t('admin.user.privacy.dialog.action.anonymize'), description: t('admin.user.privacy.dialog.help.anonymize') }]
              : []),
          ]"
          required
        />

        <UiFormField
          v-if="needsResolution"
          :label="t('admin.user.privacy.dialog.field.resolution')"
          :hint="t('admin.user.privacy.dialog.field.resolutionHint')"
          required
        >
          <UiTextarea v-model="resolution" :rows="3" :maxlength="1000" auto-grow required />
        </UiFormField>

        <!-- L'ACTE IRRÉVERSIBLE, ANNONCÉ EN ENTIER. -->
        <template v-if="action === 'anonymize'">
          <UiAlert
            intent="danger"
            :title="t('admin.user.privacy.dialog.anonymize.title')"
            :message="t('admin.user.privacy.dialog.anonymize.message')"
          />
          <UiFormField
            :label="t('admin.user.privacy.dialog.anonymize.confirm', { name: request.person_name })"
            required
          >
            <UiInput v-model="confirmation" autocomplete="off" required />
          </UiFormField>
        </template>

        <UiAlert v-if="error" intent="danger" :message="error" />

        <div class="flex flex-wrap justify-end gap-3">
          <UiButton variant="ghost" type="button" @click="emit('update:open', false)">
            {{ t('common.actions.cancel') }}
          </UiButton>
          <UiButton
            :variant="action === 'anonymize' ? 'danger' : 'primary'"
            type="submit"
            :disabled="!isValid"
            :loading="submitting"
          >
            {{ t(`admin.user.privacy.dialog.submit.${action}`) }}
          </UiButton>
        </div>
      </form>
    </div>
  </UiModal>
</template>
