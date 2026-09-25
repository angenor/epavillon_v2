<script setup lang="ts">
import type { ExpertQueueKind, FaqReportOutcome } from '~/types/admin-negotiation-queue'
import type { EffectivePermission } from '~/types/identity'
import type { Uuid } from '~/types/shared'
import type { TabItem } from '~/types/ui'

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'admin.negociations.file.nav' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()

useHead(() => ({ title: t('admin.negociations.file.title') }))

// Les questions (phase 8) et les termes proposés (phase 11) s'ajoutent ici, une ligne chacun.
const ONGLETS: ExpertQueueKind[] = ['reports']

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canReview = computed(() => hasPermission(granted.value, 'negotiation.knowledge.review'))

const onglet = ref<ExpertQueueKind>('reports')

const { data: queue, status, error, refresh } = await useAsyncData(
  'admin-negotiation-queue',
  () => api.adminNegotiationSavoir.fileDesExperts(onglet.value),
  { lazy: true, watch: [onglet] },
)

const isForbidden = computed(
  () => (!canReview.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)

const onglets = computed<TabItem[]>(() =>
  ONGLETS.map((kind) => ({
    value: kind,
    label: t(`admin.negociations.file.tabs.${kind}`),
    count: queue.value?.counts[kind],
  })),
)

function choisirOnglet(valeur: string): void {
  const kind = ONGLETS.find((k) => k === valeur)
  if (kind) onglet.value = kind
}

const groupes = computed(() => queue.value?.reports ?? [])

const aClore = ref<{ reportId: Uuid; outcome: FaqReportOutcome } | null>(null)
const cloture = ref(false)
const echec = ref<string | null>(null)
const resultat = ref<string | null>(null)

const modaleOuverte = computed({
  get: () => aClore.value !== null,
  set: (ouverte: boolean) => {
    if (!ouverte && !cloture.value) aClore.value = null
  },
})

function demanderCloture(reportId: Uuid, outcome: FaqReportOutcome): void {
  echec.value = null
  resultat.value = null
  aClore.value = { reportId, outcome }
}

async function clore(): Promise<void> {
  if (!aClore.value || cloture.value) return
  const { reportId, outcome } = aClore.value
  cloture.value = true
  try {
    await api.adminNegotiationSavoir.cloreUnSignalement(reportId, { outcome })
    resultat.value = t(`admin.negociations.file.close.done.${outcome}`)
  } catch (erreur) {
    echec.value = apiErrorMessage(erreur, t)
  } finally {
    cloture.value = false
    aClore.value = null
  }
  await refresh()
}
</script>

<template>
  <div class="mx-auto w-full max-w-5xl">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.negociations.file.forbidden.scope')"
      :description="t('admin.negociations.file.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.negociations.file.title') }}</h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.negociations.file.subtitle') }}</p>
      </header>

      <UiTabs
        class="mt-6"
        :items="onglets"
        :model-value="onglet"
        :label="t('admin.negociations.file.tabs.label')"
        @update:model-value="choisirOnglet"
      >
        <UiErrorState
          v-if="error"
          :description="apiErrorMessage(error, t)"
          :request-id="incidentReference(error) ?? undefined"
          :retry-label="t('common.actions.retry')"
          @retry="refresh()"
        />

        <UiLoadingState
          v-else-if="status === 'pending' && !queue"
          variant="card"
          :lines="2"
          :label="t('admin.negociations.file.loading')"
        />

        <template v-else>
          <UiAlert v-if="echec" class="mb-4" intent="danger" live :message="echec" />
          <UiAlert v-else-if="resultat" class="mb-4" intent="success" live compact dismissible :message="resultat" />

          <UiEmptyState
            v-if="groupes.length === 0"
            icon="check-circle"
            :title="t('admin.negociations.file.empty.title')"
            :description="t('admin.negociations.file.empty.description')"
          />

          <template v-else>
            <UiAlert intent="info" compact class="mb-4" :message="t('admin.negociations.file.close.notice')" />
            <ul class="space-y-4">
              <li v-for="groupe in groupes" :key="groupe.entry.id">
                <AdminNegotiationQueueItem :group="groupe" @close="demanderCloture" />
              </li>
            </ul>
          </template>
        </template>
      </UiTabs>
    </template>

    <UiModal
      v-model:open="modaleOuverte"
      :title="t('admin.negociations.file.close.title')"
      :description="aClore ? t(`admin.negociations.file.close.question.${aClore.outcome}`) : undefined"
    >
      <p class="text-sm text-text-secondary">{{ t('admin.negociations.file.close.notice') }}</p>
      <div class="mt-5 flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" :disabled="cloture" @click="aClore = null">
          {{ t('admin.negociations.file.close.cancel') }}
        </UiButton>
        <UiButton :loading="cloture" @click="clore">
          {{ aClore ? t(`admin.negociations.file.outcome.${aClore.outcome}`) : '' }}
        </UiButton>
      </div>
    </UiModal>
  </div>
</template>
