<script setup lang="ts">
import type { AccessRequestRow } from '~/types/admin-negotiation'
import type { EffectivePermission } from '~/types/identity'
import type { TableColumn } from '~/types/ui'
import type { TimeZoneName } from '~/types/shared'

/**
 * LA FILE DES DEMANDES D'ACCÈS.
 *
 * LES PLUS ANCIENNES EN ATTENTE D'ABORD, et l'API les rend déjà dans cet
 * ordre : une file se traite par le début. Offrir un tri par date ferait
 * remonter une demande d'hier devant celle qui attend depuis trois jours.
 *
 * LE CODE PRÉSENTÉ EST LA PREMIÈRE COLONNE QU'ON LIT après le nom : « cette
 * personne détient le code du réseau des négociatrices » n'est pas la même
 * information que « cette personne dit faire partie d'une délégation », et
 * c'est sur elle que la décision se prend le plus souvent.
 *
 * ADMETTRE ET REFUSER SONT DANS LA LIGNE, pas dans un menu : ce sont les deux
 * seuls gestes de cet écran, et ils s'enchaînent.
 *
 * QUATRE ÉTATS : chargement, vide (personne n'attend — la situation normale
 * tant que l'admission se fait par code seul), erreur, accès refusé.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationRequests' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const { dateTime } = useDateTime()

useHead(() => ({ title: t('admin.negociations.demandes.title') }))

const timezone = computed<TimeZoneName>(
  () => (Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC') as TimeZoneName,
)

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canManage = computed(() => hasPermission(granted.value, 'negotiation.space.manage'))

/** Le filtre est en FRANÇAIS : il apparaît dans une URL qu'on partage. */
const etat = computed(() => (typeof route.query.etat === 'string' ? route.query.etat : ''))

function setEtat(value: string): void {
  const next = { ...route.query }
  if (value) next.etat = value
  else delete next.etat
  router.replace({ query: next })
}

const { data: queue, status, error, refresh } = await useAsyncData(
  'admin-negotiation-requests',
  () => api.adminNegotiations.demandes(etat.value || undefined),
  { watch: [etat], lazy: true },
)

const rows = computed<AccessRequestRow[]>(() => queue.value?.rows ?? [])

const stateOptions = computed(() => [
  { value: '', label: t('admin.negociations.demandes.filters.allStates') },
  ...(['pending', 'approved', 'rejected', 'cancelled'] as const).map((valeur) => ({
    value: valeur,
    label: t(`admin.negociations.demandes.state.${valeur}`),
  })),
])

const columns = computed<TableColumn[]>(() => [
  { key: 'person', label: t('admin.negociations.demandes.columns.person') },
  { key: 'country', label: t('admin.negociations.demandes.columns.country'), hideBelow: 'lg', width: '11rem' },
  { key: 'code', label: t('admin.negociations.demandes.columns.code'), width: '12rem' },
  { key: 'scope', label: t('admin.negociations.demandes.columns.scope'), hideBelow: 'xl', width: '13rem' },
  { key: 'submittedAt', label: t('admin.negociations.demandes.columns.submittedAt'), width: '14rem' },
  { key: 'state', label: t('admin.negociations.demandes.columns.state'), width: '9rem' },
  {
    key: 'actions',
    label: t('admin.negociations.demandes.columns.actions'),
    align: 'end',
    width: '13rem',
  },
])

const STATE_INTENT = {
  pending: 'warning',
  approved: 'success',
  rejected: 'danger',
  cancelled: 'neutral',
} as const

function scopeLabel(row: AccessRequestRow): string {
  return row.scope.type === 'global'
    ? t('admin.negociations.demandes.scope.global')
    : t('admin.negociations.demandes.scope.space', { name: row.scope.name ?? '' })
}

const approveFor = ref<AccessRequestRow | null>(null)
const rejectFor = ref<AccessRequestRow | null>(null)
const reason = ref('')
const submitting = ref(false)
const writeError = ref<string | null>(null)
const result = ref<string | null>(null)

/** Le motif se vide à chaque ouverture : celui d'une autre demande n'a rien à y faire. */
watch([approveFor, rejectFor], () => {
  reason.value = ''
  writeError.value = null
})

async function trancher(issue: 'approve' | 'reject'): Promise<void> {
  const demande = issue === 'approve' ? approveFor.value : rejectFor.value
  if (!demande || submitting.value) return

  submitting.value = true
  writeError.value = null
  try {
    if (issue === 'approve') {
      await api.adminNegotiations.admettre(demande.id, reason.value || null)
      result.value = t('admin.negociations.demandes.result.approved')
    } else {
      await api.adminNegotiations.refuser(demande.id, reason.value || null)
      result.value = t('admin.negociations.demandes.result.rejected')
    }
    approveFor.value = null
    rejectFor.value = null
    await refresh()
  } catch (erreur) {
    // Le message vient de l'API et s'affiche tel quel : une demande tranchée
    // entre-temps par quelqu'un d'autre y est déjà dite en français.
    writeError.value = erreur instanceof Error ? erreur.message : t('api.unreachable.network')
    await refresh()
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="!canManage && permissionStatus !== 'pending'"
      :required-scope="t('admin.negociations.demandes.forbidden.scope')"
      :description="t('admin.negociations.demandes.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.negociations.demandes.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.negociations.demandes.subtitle') }}
          </p>
        </div>
        <UiButton variant="secondary" icon="sliders" to="/admin/negociations/admission">
          {{ t('nav.admin.negotiationAdmission') }}
        </UiButton>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <template v-else>
        <UiAlert v-if="writeError" class="mt-6" intent="danger" live :message="writeError" />
        <UiAlert
          v-else-if="result"
          class="mt-6"
          intent="success"
          live
          compact
          dismissible
          :message="result"
        />
        <UiAlert
          v-else-if="queue && queue.pending > 0"
          class="mt-6"
          intent="info"
          compact
          :message="t('admin.negociations.demandes.pending', { count: queue.pending })"
        />

        <UiEmptyState
          v-if="rows.length === 0 && !etat && status !== 'pending'"
          class="mt-8"
          icon="inbox"
          :title="t('admin.negociations.demandes.empty.title')"
          :description="t('admin.negociations.demandes.empty.description')"
        />

        <template v-else>
          <section
            class="mt-6 flex flex-wrap items-end gap-3"
            :aria-label="t('admin.negociations.demandes.filters.label')"
          >
            <UiFormField :label="t('admin.negociations.demandes.filters.state')" hide-optional>
              <template #default="{ control }">
                <UiSelect
                  v-bind="control"
                  :model-value="etat"
                  :options="stateOptions"
                  :disabled="status === 'pending'"
                  @update:model-value="setEtat"
                />
              </template>
            </UiFormField>
          </section>

          <UiTable
            class="mt-4"
            :columns="columns"
            :rows="rows"
            row-key="id"
            row-label-key="display_name"
            :caption="t('admin.negociations.demandes.caption')"
            :loading="status === 'pending'"
          >
            <template #cell-person="{ row }">
              <span class="font-medium">{{ row.display_name }}</span>
              <span class="mt-0.5 block text-sm text-text-muted">{{ row.email }}</span>
              <span v-if="row.message" class="mt-1 block text-sm text-text-muted">
                {{ t('admin.negociations.demandes.message', { message: row.message }) }}
              </span>
            </template>

            <template #cell-country="{ row }">
              {{ row.country ?? t('admin.negociations.demandes.noCountry') }}
            </template>

            <template #cell-code="{ row }">
              <template v-if="row.invitation_code">
                <span class="font-mono text-sm font-semibold">{{ row.invitation_code }}</span>
                <span class="mt-0.5 block text-sm text-text-muted">
                  {{ row.invitation_code_label }}
                </span>
              </template>
              <span v-else class="text-text-muted">
                {{ t('admin.negociations.demandes.noCode') }}
              </span>
            </template>

            <template #cell-scope="{ row }">{{ scopeLabel(row) }}</template>

            <template #cell-submittedAt="{ row }">
              {{ dateTime(row.submitted_at, timezone) }}
            </template>

            <template #cell-state="{ row }">
              <UiBadge
                :intent="STATE_INTENT[row.status]"
                :label="t(`admin.negociations.demandes.state.${row.status}`)"
              />
              <span v-if="row.decided_by_name" class="mt-0.5 block text-sm text-text-muted">
                {{
                  t('admin.negociations.demandes.decided', {
                    date: dateTime(row.decided_at, timezone),
                    author: row.decided_by_name,
                  })
                }}
              </span>
              <span v-if="row.decision_reason" class="mt-0.5 block text-sm text-text-muted">
                {{ t('admin.negociations.demandes.reason', { reason: row.decision_reason }) }}
              </span>
            </template>

            <template #cell-actions="{ row }">
              <div v-if="row.status === 'pending'" class="flex justify-end gap-2">
                <UiButton size="sm" @click="approveFor = row">
                  {{ t('admin.negociations.demandes.actions.approve') }}
                </UiButton>
                <UiButton size="sm" variant="ghost" @click="rejectFor = row">
                  {{ t('admin.negociations.demandes.actions.reject') }}
                </UiButton>
              </div>
            </template>

            <template #empty>
              <UiEmptyState
                icon="search"
                filtered
                :title="t('admin.negociations.demandes.noResults.title')"
                :description="
                  t('admin.negociations.demandes.noResults.description', {
                    total: queue?.total ?? 0,
                  })
                "
                :action-label="t('admin.negociations.demandes.noResults.action')"
                @action="setEtat('')"
              />
            </template>
          </UiTable>
        </template>
      </template>
    </template>

    <UiModal
      :open="approveFor !== null"
      :title="
        t('admin.negociations.demandes.confirm.approve.title', {
          name: approveFor?.display_name ?? '',
        })
      "
      :description="t('admin.negociations.demandes.confirm.approve.question')"
      @update:open="(value: boolean) => !value && (approveFor = null)"
    >
      <UiAlert
        v-if="approveFor?.invitation_code"
        intent="info"
        compact
        :message="t('admin.negociations.demandes.confirm.approve.network')"
      />
      <div class="mt-5 flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="approveFor = null">
          {{ t('admin.negociations.demandes.confirm.approve.cancel') }}
        </UiButton>
        <UiButton :loading="submitting" @click="trancher('approve')">
          {{ t('admin.negociations.demandes.confirm.approve.confirm') }}
        </UiButton>
      </div>
    </UiModal>

    <UiModal
      :open="rejectFor !== null"
      :title="
        t('admin.negociations.demandes.confirm.reject.title', {
          name: rejectFor?.display_name ?? '',
        })
      "
      :description="t('admin.negociations.demandes.confirm.reject.question')"
      @update:open="(value: boolean) => !value && (rejectFor = null)"
    >
      <UiFormField
        :label="t('admin.negociations.demandes.confirm.reject.reason')"
        :hint="t('admin.negociations.demandes.confirm.reject.reasonHint')"
      >
        <template #default="{ control }">
          <UiTextarea v-bind="control" v-model="reason" :rows="3" :maxlength="280" />
        </template>
      </UiFormField>
      <div class="mt-5 flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="rejectFor = null">
          {{ t('admin.negociations.demandes.confirm.reject.cancel') }}
        </UiButton>
        <UiButton variant="danger" :loading="submitting" @click="trancher('reject')">
          {{ t('admin.negociations.demandes.confirm.reject.confirm') }}
        </UiButton>
      </div>
    </UiModal>
  </div>
</template>
