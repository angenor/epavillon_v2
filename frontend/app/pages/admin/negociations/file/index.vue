<script setup lang="ts">
import type { AdminFaqEntry } from '~/types/admin-negotiation-savoir'
import type { ExpertQueueKind, FaqReportOutcome } from '~/types/admin-negotiation-queue'
import type { EffectivePermission } from '~/types/identity'
import type { TaxonomyTerm } from '~/types/reference'
import type { Uuid } from '~/types/shared'
import type { SelectOption, TabItem } from '~/types/ui'

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'admin.negociations.file.nav' }],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.negociations.file.title') }))

// Les termes proposés (phase 11) s'ajoutent ici.
const ONGLETS: ExpertQueueKind[] = ['reports', 'questions']

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canReview = computed(() => hasPermission(granted.value, 'negotiation.knowledge.review'))

const onglet = computed<ExpertQueueKind>(() => ONGLETS.find((k) => k === route.query.kind) ?? 'reports')

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
  if (!kind || kind === onglet.value) return
  echec.value = null
  resultat.value = null
  promue.value = null
  void router.replace({ query: { ...route.query, kind: kind === 'reports' ? undefined : kind } })
}

const chargement = computed(() => status.value === 'pending' && queue.value?.kind !== onglet.value)
const groupes = computed(() => queue.value?.reports ?? [])
const questions = computed(() => queue.value?.questions ?? [])
const vide = computed(() => (onglet.value === 'questions' ? questions.value : groupes.value).length === 0)

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

const { data: rubriques } = await useAsyncData<TaxonomyTerm[]>(
  'reference-terms-faq_section',
  () => api.reference.terms('faq_section'),
  { default: () => [], lazy: true },
)

const sections = computed<SelectOption[]>(() =>
  rubriques.value.map((r) => ({ value: r.code, label: resolveI18nText(r.label, locale.value) })),
)

const enReponse = ref<Uuid | null>(null)
const echecQuestion = ref<{ id: Uuid; message: string } | null>(null)
const aPromouvoir = ref<{ id: Uuid; sectionCode: string } | null>(null)
const promotion = ref(false)
const promue = ref<AdminFaqEntry | null>(null)

const modalePromotion = computed({
  get: () => aPromouvoir.value !== null,
  set: (ouverte: boolean) => {
    if (!ouverte && !promotion.value) aPromouvoir.value = null
  },
})

async function repondre(id: Uuid, answer: string): Promise<void> {
  if (enReponse.value) return
  enReponse.value = id
  echecQuestion.value = null
  resultat.value = null
  promue.value = null
  try {
    await api.adminNegotiationSavoir.repondreAUneQuestion(id, { answer })
    resultat.value = t('admin.negociations.file.question.answer.done')
  } catch (erreur) {
    echecQuestion.value = { id, message: apiErrorMessage(erreur, t) }
  } finally {
    enReponse.value = null
  }
  await refresh()
}

function demanderPromotion(id: Uuid, sectionCode: string): void {
  echecQuestion.value = null
  resultat.value = null
  promue.value = null
  aPromouvoir.value = { id, sectionCode }
}

async function promouvoir(): Promise<void> {
  if (!aPromouvoir.value || promotion.value) return
  const { id, sectionCode } = aPromouvoir.value
  promotion.value = true
  try {
    promue.value = await api.adminNegotiationSavoir.promouvoirUneQuestion(id, { section_code: sectionCode })
  } catch (erreur) {
    echecQuestion.value = { id, message: apiErrorMessage(erreur, t) }
  } finally {
    promotion.value = false
    aPromouvoir.value = null
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
          v-else-if="chargement"
          variant="card"
          :lines="2"
          :label="t('admin.negociations.file.loading')"
        />

        <template v-else>
          <UiAlert v-if="echec" class="mb-4" intent="danger" live :message="echec" />
          <UiAlert v-else-if="resultat" class="mb-4" intent="success" live compact dismissible :message="resultat" />
          <UiAlert v-if="promue" class="mb-4" intent="success" live :message="t('admin.negociations.file.question.promote.done')">
            <template #actions>
              <UiButton variant="secondary" icon-trailing="arrow-right" :to="localePath(`/admin/negociations/faq/${promue.id}`)">
                {{ t('admin.negociations.file.question.promote.open') }}
              </UiButton>
            </template>
          </UiAlert>

          <UiEmptyState
            v-if="vide"
            icon="check-circle"
            :title="t(`admin.negociations.file.empty.${onglet}.title`)"
            :description="t(`admin.negociations.file.empty.${onglet}.description`)"
          />

          <ul v-else-if="onglet === 'questions'" class="space-y-4">
            <li v-for="question in questions" :key="question.id">
              <AdminNegotiationQueueQuestion
                :question="question"
                :sections="sections"
                :answering="enReponse === question.id"
                :error="echecQuestion?.id === question.id ? echecQuestion.message : null"
                @answer="repondre"
                @promote="demanderPromotion"
              />
            </li>
          </ul>

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

    <UiModal
      v-model:open="modalePromotion"
      :title="t('admin.negociations.file.question.promote.title')"
      :description="t('admin.negociations.file.question.promote.question')"
    >
      <p class="text-sm text-text-secondary">{{ t('admin.negociations.file.question.promote.notice') }}</p>
      <div class="mt-5 flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" :disabled="promotion" @click="aPromouvoir = null">
          {{ t('admin.negociations.file.close.cancel') }}
        </UiButton>
        <UiButton :loading="promotion" @click="promouvoir">
          {{ t('admin.negociations.file.question.promote.confirm') }}
        </UiButton>
      </div>
    </UiModal>
  </div>
</template>
