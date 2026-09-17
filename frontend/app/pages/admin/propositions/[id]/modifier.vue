<script setup lang="ts">
import type { EditableProposal, ProposalDraft, ProposalFormStep } from '~/types/proposal-form'
import type { ReviewDeskScreen } from '~/types/admin-review'
import type { Organization } from '~/types/org'
import type { Country, Locale, TaxonomyTerm } from '~/types/reference'
import type { SelectOption, TabItem } from '~/types/ui'

/**
 * L'ÉQUIPE CORRIGE UN DOSSIER DÉPOSÉ, à la demande de son organisation.
 *
 * Les étapes du formulaire de dépôt, sans enregistrement automatique : un
 * dossier déposé est lu par le comité, une correction part quand l'agent la
 * valide. L'état et le contact du dossier ne changent pas.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.proposals', to: '/admin/propositions' },
    { labelKey: 'admin.proposal.edit.title' },
  ],
})

const EDIT_STEPS: ProposalFormStep[] = [
  'organizations',
  'presentation',
  'classification',
  'speakers',
  'schedule',
]

const { t, locale } = useI18n()
const { tr } = useI18nText()
const route = useRoute()
const api = useApi()
const auth = useAuthStore()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.proposal.edit.title') }))

const proposalId = computed(() => String(route.params.id ?? ''))
const fileTo = computed(() => localePath(`/admin/propositions/${proposalId.value}`))

await adminScope.ensureLoaded()

interface EditContext {
  screen: ReviewDeskScreen
  editable: EditableProposal
  themes: TaxonomyTerm[]
  categories: TaxonomyTerm[]
  locales: Locale[]
  countries: Country[]
}

const { data, status, error, refresh } = await useAsyncData<EditContext | null>(
  `admin-proposal-edit-${proposalId.value}`,
  async () => {
    const [screen, editable] = await Promise.all([
      api.review.desk(proposalId.value, auth.person?.id ?? null),
      api.proposals.forEdit(proposalId.value),
    ])
    if (!screen || !editable) return null
    const [themes, categories, locales, countries] = await Promise.all([
      api.reference.terms('activity_theme'),
      api.reference.terms('activity_category'),
      api.reference.locales(),
      api.reference.countries(),
    ])
    return {
      screen,
      editable,
      themes,
      categories,
      locales: locales.filter((entry) => entry.is_active),
      countries,
    }
  },
  { lazy: true },
)

const screen = computed(() => data.value?.screen ?? null)
const call = computed(() => screen.value?.call ?? null)
const edition = computed(() => screen.value?.edition ?? null)

const forbidden = ref(false)
const isForbidden = computed(
  () =>
    forbidden.value ||
    (!adminScope.isLoading && !adminScope.canAdminister) ||
    isForbiddenError(error.value) ||
    (screen.value !== null && !screen.value.permissions.can_edit),
)

type BlockedReason = 'draft' | 'file_closed' | 'edition_over' | 'no_call'

const blockedReason = computed<BlockedReason | null>(() => {
  const current = screen.value
  if (!current) return null
  if (current.proposal.status === 'draft') return 'draft'
  // Les bornes de l'appel valident la saisie : sans appel, rien à quoi la confronter.
  if (!current.call) return 'no_call'
  return proposalEditBlockedReason(current.proposal, current.edition)
})

// ---------------------------------------------------------------------------
// Le dossier en cours de correction
// ---------------------------------------------------------------------------

const draft = ref<ProposalDraft | null>(null)
const baseline = ref('')

watch(
  () => data.value,
  (ready) => {
    if (!ready || draft.value) return
    draft.value = draftFromReopened(ready.editable.draft)
    baseline.value = JSON.stringify(draft.value)
  },
  { immediate: true },
)

const isDirty = computed(() => draft.value !== null && JSON.stringify(draft.value) !== baseline.value)

const leadCandidates = computed<Organization[]>(() => {
  const leadId = draft.value?.organization_id
  const lead = screen.value?.organizations.find((entry) => entry.organization?.id === leadId)
  return lead?.organization ? [lead.organization] : []
})

function countryNameOf(countryId: string | null): string | null {
  if (!countryId) return null
  const country = data.value?.countries.find((entry) => entry.id === countryId)
  return country ? tr(country.name) : null
}

const countryOptions = computed<SelectOption[]>(() =>
  (data.value?.countries ?? [])
    .filter((country) => country.is_active)
    .map((country) => ({ value: country.id, label: tr(country.name) }))
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
)

// ---------------------------------------------------------------------------
// Défauts et onglets
// ---------------------------------------------------------------------------

const hasTriedToSave = ref(false)

const issues = computed(() =>
  draft.value && call.value && edition.value
    ? validateProposalDraft(draft.value, call.value, edition.value).filter(
        (issue) => issue.severity === 'error' && EDIT_STEPS.includes(issue.step),
      )
    : [],
)

function issuesOf(step: ProposalFormStep) {
  return hasTriedToSave.value ? issues.value.filter((issue) => issue.step === step) : []
}

const activeStep = ref<ProposalFormStep>('organizations')

const tabs = computed<TabItem[]>(() =>
  EDIT_STEPS.map((step) => {
    const count = issuesOf(step).length
    return {
      value: step,
      label: t(`proposal.form.steps.${step}.label`),
      count: count > 0 ? count : undefined,
    }
  }),
)

function selectStep(value: string): void {
  const step = EDIT_STEPS.find((entry) => entry === value)
  if (step) activeStep.value = step
}

// ---------------------------------------------------------------------------
// Enregistrement
// ---------------------------------------------------------------------------

const busy = ref(false)
const saveError = ref<string | null>(null)
let leavingAfterSave = false

async function save(): Promise<void> {
  const current = draft.value
  const context = data.value
  const currentCall = call.value
  if (!current || !context || !currentCall) return
  hasTriedToSave.value = true
  saveError.value = null

  const firstIssue = issues.value[0]
  if (firstIssue) {
    activeStep.value = firstIssue.step
    return
  }

  busy.value = true
  try {
    await api.review.editContent({
      proposal_id: context.editable.proposal_id,
      call_id: currentCall.id,
      event_id: context.editable.event_id,
      draft: current,
    })
    leavingAfterSave = true
    await navigateTo({ path: fileTo.value, query: { modifie: '1' } })
  } catch (thrown) {
    if (isSessionLost(thrown)) {
      await navigateTo({ path: localePath('auth-login'), query: { redirect: route.fullPath } })
      return
    }
    if (thrown instanceof ForbiddenError) {
      forbidden.value = true
      return
    }
    saveError.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}

onBeforeRouteLeave(() => {
  if (leavingAfterSave || !isDirty.value) return true
  return window.confirm(t('admin.proposal.edit.leaveConfirm'))
})
</script>

<template>
  <div class="mx-auto w-full max-w-5xl pb-20">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.proposal.edit.forbiddenScope')"
      action-to="/admin/propositions"
      :action-label="t('admin.proposal.review.backToList')"
    />

    <template v-else>
      <UiErrorState v-if="error" :retry-label="t('common.actions.retry')" @retry="refresh()" />

      <div v-else-if="status === 'pending' && !data" class="flex flex-col gap-4">
        <UiSkeletonLoader height="6rem" />
        <UiSkeletonLoader height="28rem" />
      </div>

      <UiEmptyState
        v-else-if="!data || !screen"
        icon="inbox"
        :title="t('admin.proposal.review.notFound.title')"
        :description="t('admin.proposal.review.notFound.description')"
        :action-label="t('admin.proposal.review.notFound.action')"
        :action-to="localePath('/admin/propositions')"
      />

      <template v-else>
        <NuxtLink :to="fileTo" class="inline-flex items-center gap-1.5 text-sm no-underline">
          <UiIcon name="arrow-left" size="1rem" :stroke-width="1.8" />
          {{ t('admin.proposal.edit.backToFile') }}
        </NuxtLink>

        <header class="mt-3">
          <p class="font-mono text-sm text-text-muted">{{ screen.proposal.reference_code }}</p>
          <h1 class="mt-1 text-3xl leading-tight font-semibold text-balance">
            {{ t('admin.proposal.edit.title') }}
          </h1>
          <p class="mt-1 max-w-(--measure) text-text-muted">
            {{ t('admin.proposal.edit.subtitle') }}
          </p>
        </header>

        <UiEmptyState
          v-if="blockedReason"
          class="mt-8"
          icon="lock"
          :title="t('admin.proposal.edit.blocked.title')"
          :description="t(`admin.proposal.edit.blocked.${blockedReason}`)"
          :action-label="t('admin.proposal.edit.backToFile')"
          :action-to="fileTo"
        />

        <form v-else-if="draft && call && edition" class="mt-8 flex flex-col gap-6" novalidate @submit.prevent="save">
          <UiAlert
            v-if="saveError"
            intent="danger"
            live
            dismissible
            :message="saveError"
            @dismiss="saveError = null"
          />
          <UiAlert
            v-if="hasTriedToSave && issues.length > 0"
            intent="danger"
            live
            :title="t('admin.proposal.edit.issues.title')"
            :message="t('admin.proposal.edit.issues.hint', issues.length)"
          />

          <UiTabs
            :items="tabs"
            :model-value="activeStep"
            :label="t('admin.proposal.edit.title')"
            @update:model-value="selectStep"
          />

          <section class="rounded-lg border border-border bg-surface-raised px-4 py-5 sm:px-6 sm:py-6">
            <ProposalStepOrganizations
              v-if="activeStep === 'organizations'"
              v-model="draft"
              :lead-candidates="leadCandidates"
              :lead-note="t('admin.proposal.edit.leadNote')"
              :issues="issuesOf('organizations')"
              :country-name-of="countryNameOf"
            />
            <ProposalStepPresentation
              v-else-if="activeStep === 'presentation'"
              v-model="draft"
              :issues="issuesOf('presentation')"
            />
            <ProposalStepClassification
              v-else-if="activeStep === 'classification'"
              v-model="draft"
              :call="call"
              :themes="data.themes"
              :categories="data.categories"
              :locales="data.locales"
              :country-options="countryOptions"
              :issues="issuesOf('classification')"
            />
            <ProposalStepSpeakers
              v-else-if="activeStep === 'speakers'"
              v-model="draft"
              :call="call"
              :issues="issuesOf('speakers')"
            />
            <ProposalStepSchedule
              v-else-if="activeStep === 'schedule'"
              v-model="draft"
              :call="call"
              :edition="edition"
              :issues="issuesOf('schedule')"
            />
          </section>

          <div
            class="sticky bottom-0 z-10 flex flex-wrap items-center justify-end gap-3 border-t border-border bg-surface py-3"
          >
            <p v-if="isDirty" class="me-auto text-sm text-text-muted">
              {{ t('admin.proposal.edit.unsaved') }}
            </p>
            <UiButton variant="ghost" :to="fileTo" :label="t('common.actions.cancel')" />
            <UiButton
              type="submit"
              variant="primary"
              icon="check"
              :loading="busy"
              :disabled="!isDirty"
              :label="t('admin.proposal.edit.save')"
            />
          </div>
        </form>
      </template>
    </template>
  </div>
</template>
