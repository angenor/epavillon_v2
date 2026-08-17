<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type { Country, Locale, TaxonomyTerm } from '~/types/reference'
import type { Organization } from '~/types/org'
import type {
  ProposalDraft,
  ProposalFormStep,
  SubmitProposalResult,
} from '~/types/proposal-form'
import { PROPOSAL_FORM_STEPS } from '~/types/proposal-form'
import type { SelectOption, StepItem, StepState } from '~/types/ui'

/**
 * FORMULAIRE DE SOUMISSION D'UNE PROPOSITION — `/deposer-une-proposition`,
 * `/en/submit-a-proposal`.
 *
 * LE FORMULAIRE LE PLUS LONG ET LE PLUS DÉTERMINANT DE LA PLATEFORME. Une
 * organisation qui abandonne ici ne participe pas à la COP : tout ce qui suit
 * découle de cette phrase. Il est rempli par des agents qui ne sont pas des
 * utilisateurs experts, souvent en plusieurs fois, parfois sur téléphone, et
 * l'échéance de l'appel ne se rappelle pas à eux toute seule.
 *
 * CINQ DÉCISIONS, ET CHACUNE SE PAIE SI ON L'INVERSE :
 *
 *  1. RIEN NE SE PERD. Le brouillon s'enregistre tout seul, une seconde et demie
 *     après la dernière frappe, et l'écran dit quand — à l'heure du SERVEUR.
 *     La v1 n'enregistrait qu'à la soumission : fermer l'onglet perdait tout.
 *  2. LES SEPT ÉTAPES SONT TOUTES ATTEIGNABLES, tout de suite. Un parcours
 *     linéaire suppose qu'on remplit dans l'ordre ; en réalité on saute les
 *     intervenants parce qu'on attend une réponse, et on revient. La barre
 *     d'étapes montre où l'on en est ET ce qui est en défaut, depuis n'importe
 *     quelle étape.
 *  3. LES ERREURS SONT DITES DEUX FOIS : regroupées en tête — combien il en
 *     reste et où —, et sur le champ concerné — ce qui ne va pas. L'une sans
 *     l'autre oblige soit à chercher, soit à deviner.
 *  4. L'ÉCHÉANCE NE QUITTE JAMAIS L'ÉCRAN. Un dossier se remplit sur plusieurs
 *     jours ; l'échéance apprise à l'étape 1 est oubliée à l'étape 5.
 *  5. RIEN N'EST BLOQUÉ QUI NE DOIVE L'ÊTRE. On peut avancer avec un dossier
 *     incomplet, quitter, revenir. Seul l'ENVOI vérifie, parce que c'est lui que
 *     la base refuserait.
 *
 * LA PAGE EST GARDÉE. Une proposition se dépose au nom d'une organisation, et le
 * rattachement n'est pas obligatoire pour avoir un compte : le middleware
 * `requires-organization` renvoie vers l'écran A2 qui explique pourquoi et
 * ramène ici. Ce test n'est PAS réécrit dans la page.
 *
 * QUATRE ÉTATS, comme partout : chargement, erreur (avec reprise), vide (aucun
 * appel ouvert — on ne dépose nulle part aujourd'hui), accès refusé (session
 * perdue en cours de route).
 */

definePageMeta({
  layout: 'public',
  middleware: ['auth', 'requires-organization'],
  organizationReason: 'proposal',
})

defineI18nRoute({ paths: { fr: '/deposer-une-proposition', en: '/submit-a-proposal' } })

const { t, locale } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()
const api = useApi()
const auth = useAuthStore()
const memberships = useMembershipStore()

useHead(() => ({ title: t('proposal.form.title') }))

// ---------------------------------------------------------------------------
// Le contexte : où l'on dépose, et avec quels référentiels
// ---------------------------------------------------------------------------

interface FormContext {
  call: CallForProposals | null
  edition: EventEdition | null
  countedProposals: number
  themes: TaxonomyTerm[]
  categories: TaxonomyTerm[]
  documentTypes: TaxonomyTerm[]
  locales: Locale[]
  countries: Country[]
}

const {
  data: context,
  status: contextStatus,
  error: contextError,
  refresh: reloadContext,
} = useAsyncData<FormContext>(
  'proposal-form-context',
  async () => {
    await memberships.ensureLoaded()
    const person = auth.person
    const organizationIds = memberships.active.map((entry) => entry.organization.id)

    const empty: FormContext = {
      call: null,
      edition: null,
      countedProposals: 0,
      themes: [],
      categories: [],
      documentTypes: [],
      locales: [],
      countries: [],
    }
    if (!person) return empty

    // L'ÉDITION N'EST PAS CHOISIE PAR L'ÉCRAN : il y a au plus un appel ouvert
    // à la fois (`ux_calls_one_per_event` + `event.is_call_open()`).
    const formContext = await api.proposals.formContext(person.id, organizationIds)
    if (!formContext.call_id || !formContext.event_id) return empty

    const [call, edition, themes, categories, documentTypes, locales, countries] =
      await Promise.all([
        api.events.call(formContext.event_id),
        // `publicList()` et non `list()` : cette dernière filtre par périmètre
        // d'ADMINISTRATION, et une organisation qui dépose n'en a aucun.
        api.events
          .publicList()
          .then((list) => list.find((event) => event.id === formContext.event_id) ?? null),
        api.reference.terms('activity_theme'),
        api.reference.terms('activity_category'),
        api.reference.terms('document_type'),
        api.reference.locales(),
        api.reference.countries(),
      ])

    return {
      call,
      edition,
      countedProposals: formContext.counted_proposals,
      themes,
      categories,
      documentTypes,
      locales: locales.filter((entry) => entry.is_active),
      countries,
    }
  },
  { lazy: true, watch: [() => auth.person?.id] },
)

const isLoading = computed(
  () => contextStatus.value === 'pending' || contextStatus.value === 'idle',
)

const call = computed(() => context.value?.call ?? null)
const edition = computed(() => context.value?.edition ?? null)

/** L'appel accepte-t-il un dépôt MAINTENANT ? — `event.is_call_open()`. */
const isOpen = computed(() => (call.value ? isCallOpen(call.value) : false))

/** Le plafond `max_proposals_per_organization` est-il déjà atteint ? */
const quotaReached = computed(() => {
  const max = call.value?.max_proposals_per_organization ?? null
  if (max === null || !context.value) return false
  return context.value.countedProposals >= max
})

// ---------------------------------------------------------------------------
// Résolution des libellés venus de la BASE — jamais des fichiers i18n
// ---------------------------------------------------------------------------

function countryNameOf(countryId: string | null): string | null {
  if (!countryId) return null
  const country = context.value?.countries.find((entry) => entry.id === countryId)
  return country ? tr(country.name) : null
}

const countryOptions = computed<SelectOption[]>(() =>
  (context.value?.countries ?? [])
    .filter((country) => country.is_active)
    .map((country) => ({ value: country.id, label: tr(country.name) }))
    // Trié dans la LANGUE AFFICHÉE : « Bénin » se range où on le cherche.
    .sort((a, b) => a.label.localeCompare(b.label, locale.value)),
)

const leadOrganization = computed<Organization | null>(
  () =>
    memberships.active.find((entry) => entry.organization.id === draft.value.organization_id)
      ?.organization ?? null,
)

// ---------------------------------------------------------------------------
// Le brouillon
// ---------------------------------------------------------------------------

const draft = ref<ProposalDraft>(emptyProposalDraft({ locale: locale.value }))

const {
  proposalId,
  referenceCode,
  savedAt,
  state: saveState,
  adopt: adoptDraft,
  arm: armAutosave,
  saveNow,
} = useProposalDraft({
  draft,
  callId: computed(() => call.value?.id ?? null),
  eventId: computed(() => call.value?.event_id ?? null),
  personId: computed(() => auth.person?.id ?? null),
})

/**
 * Amorçage : on reprend le brouillon en cours s'il y en a un, sinon on part
 * d'un dossier vide dont le porteur est déjà choisi quand la personne n'a
 * qu'une seule organisation. L'observation n'est armée qu'après — sans quoi
 * l'amorçage lui-même déclencherait une écriture.
 */
const isDraftReady = ref(false)

watch(
  () => [context.value, auth.person?.id] as const,
  async ([ready, personId]) => {
    if (!ready?.call || !personId || isDraftReady.value) return

    const existing = await api.proposals.myDraft(personId)
    const active = memberships.active
    if (existing) {
      draft.value = existing.draft
      adoptDraft(existing)
    } else {
      draft.value = emptyProposalDraft({
        organizationId: active.length === 1 ? active[0]?.organization.id : null,
        durationMinutes: ready.call.default_duration_minutes,
        locale: locale.value,
      })
    }

    isDraftReady.value = true
    await nextTick()
    armAutosave()
  },
  { immediate: true },
)

// ---------------------------------------------------------------------------
// Les défauts du dossier
// ---------------------------------------------------------------------------

const issues = computed(() =>
  call.value && edition.value ? validateProposalDraft(draft.value, call.value, edition.value) : [],
)

/** Les défauts affichés sur les champs d'une étape donnée. */
function issuesOf(step: ProposalFormStep) {
  return issues.value.filter((issue) => issue.step === step)
}

/**
 * Les erreurs ne s'affichent qu'après une TENTATIVE — d'envoi, ou de sortie de
 * l'étape. Peindre en rouge un champ qu'on n'a pas encore atteint est une
 * réprimande avant la faute ; le formulaire s'ouvrirait avec neuf erreurs.
 */
const visitedSteps = ref<Set<ProposalFormStep>>(new Set(['organizations']))
const hasTriedToSubmit = ref(false)

/**
 * QUAND UNE ERREUR S'AFFICHE-T-ELLE ?
 *
 * Trois déclencheurs, et aucun avant que la personne n'ait agi : une tentative
 * d'envoi, une étape déjà quittée, ou — depuis que le bouton « Suivant » bloque
 * — la moindre modification du dossier. Ce dernier cas est la contrepartie du
 * blocage : un bouton désactivé sans champ signalé pose une question sans
 * réponse (« pourquoi je ne peux pas continuer ? »), et l'on ne peut plus
 * quitter l'étape pour le découvrir.
 *
 * Ce qui reste évité : le formulaire vierge qui s'ouvre avec neuf reproches.
 * Tant que rien n'a été saisi, rien n'est rouge.
 */
function visibleIssuesOf(step: ProposalFormStep) {
  if (hasTriedToSubmit.value) return issuesOf(step)
  if (visitedSteps.value.has(step)) return issuesOf(step)
  if (step === currentStep.value && saveState.value !== 'untouched') return issuesOf(step)
  return []
}

/**
 * LES ERREURS DE L'ÉTAPE COURANTE, celles qui retiennent le bouton « Suivant ».
 *
 * On bloque le passage à l'étape suivante — arbitrage du commanditaire du 17/08.
 * Le raisonnement se tient : une étape laissée en défaut se retrouve corrigée à
 * la fin, dans la précipitation d'une échéance, et c'est là que les dossiers se
 * perdent. La barre d'étapes reste NAVIGABLE en revanche : on peut aller voir ce
 * que demande l'étape 5 et revenir. C'est l'avancée pas à pas qui exige un
 * dossier propre, pas la consultation.
 */
const currentStepErrors = computed(() =>
  issuesOf(currentStep.value).filter((issue) => issue.severity === 'error'),
)

const canGoNext = computed(() => currentStepErrors.value.length === 0)

const blockingIssues = computed(() => issues.value.filter((issue) => issue.severity === 'error'))

// ---------------------------------------------------------------------------
// La barre d'étapes
// ---------------------------------------------------------------------------

const currentStep = ref<ProposalFormStep>('organizations')

const stepLabels = computed(
  () =>
    Object.fromEntries(
      PROPOSAL_FORM_STEPS.map((step) => [step, t(`proposal.form.steps.${step}.label`)]),
    ) as Record<ProposalFormStep, string>,
)

const stepItems = computed<StepItem[]>(() =>
  PROPOSAL_FORM_STEPS.map((step) => {
    const stepIssues = issuesOf(step)
    const errors = stepIssues.filter((issue) => issue.severity === 'error').length
    const seen = hasTriedToSubmit.value || visitedSteps.value.has(step)

    let state: StepState = 'upcoming'
    if (step === currentStep.value) state = 'current'
    else if (seen && errors > 0) state = 'error'
    else if (seen) state = 'done'

    return {
      value: step,
      label: stepLabels.value[step],
      description:
        errors > 0 && seen
          ? t('proposal.form.steps.errorCount', { count: errors }, errors)
          : t(`proposal.form.steps.${step}.hint`),
      state,
    }
  }),
)

/**
 * TOUTES LES ÉTAPES SONT ATTEIGNABLES, y compris celles qu'on n'a pas encore
 * vues. `UiStepper` n'autorise par défaut que le retour en arrière ; ici le
 * dossier se remplit dans le désordre, et interdire d'aller voir l'étape 5 avant
 * d'avoir fini la 2 obligerait à saisir pour naviguer.
 */
async function goToStep(step: ProposalFormStep, field?: string): Promise<void> {
  visitedSteps.value = new Set([...visitedSteps.value, currentStep.value, step])
  currentStep.value = step
  await nextTick()

  // Le champ visé reçoit le focus : c'est ce qui rend le résumé d'erreurs utile.
  if (field) {
    const target = document.getElementById(`proposal-${field}`)
    if (target instanceof HTMLElement) target.focus()
  }
  if (import.meta.client) window.scrollTo({ top: 0, behavior: 'smooth' })
}

const stepIndex = computed(() => PROPOSAL_FORM_STEPS.indexOf(currentStep.value))
const isFirstStep = computed(() => stepIndex.value === 0)
const isLastStep = computed(() => stepIndex.value === PROPOSAL_FORM_STEPS.length - 1)

async function nextStep(): Promise<void> {
  const next = PROPOSAL_FORM_STEPS[stepIndex.value + 1]
  if (!next) return
  // On enregistre en quittant l'étape : le passage d'étape est le moment où l'on
  // se dit « c'est fait », et c'est celui-là qu'il faut sécuriser.
  await saveNow()
  await goToStep(next)
}

async function previousStep(): Promise<void> {
  const previous = PROPOSAL_FORM_STEPS[stepIndex.value - 1]
  if (!previous) return
  await goToStep(previous)
}

// ---------------------------------------------------------------------------
// L'envoi
// ---------------------------------------------------------------------------

const isSubmitting = ref(false)
const submitError = ref<Error | null>(null)
const refusal = ref<Extract<SubmitProposalResult, { status: 'call_closed' | 'quota_reached' }> | null>(null)
const outcome = ref<Extract<SubmitProposalResult, { status: 'submitted' }> | null>(null)

async function submit(): Promise<void> {
  hasTriedToSubmit.value = true
  const person = auth.person
  const currentCall = call.value
  if (!person || !currentCall) return

  if (blockingIssues.value.length > 0) {
    const first = blockingIssues.value[0]
    if (first) await goToStep(first.step, first.field)
    return
  }

  isSubmitting.value = true
  submitError.value = null
  refusal.value = null
  try {
    // Le brouillon part d'abord : l'API dépose une ligne déjà écrite, elle ne
    // découvre pas le dossier au moment de la transition d'état.
    await saveNow()

    const result = await api.proposals.submit(person.id, {
      proposal_id: proposalId.value ?? '',
      call_id: currentCall.id,
      event_id: currentCall.event_id,
      draft: draft.value,
    })

    if (result.status === 'submitted') {
      outcome.value = result
      if (import.meta.client) window.scrollTo({ top: 0 })
    } else {
      // Les deux refus de `tg_check_submission_eligibility()`. Ce ne sont pas
      // des erreurs de réseau : ils se rendent en clair, avec la suite possible.
      refusal.value = result
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error : new Error(String(error))
  } finally {
    isSubmitting.value = false
  }
}

// ---------------------------------------------------------------------------
// Sorties
// ---------------------------------------------------------------------------

/** Le plafond annoncé par le refus, extrait pour que le gabarit reste typé. */
const refusedQuota = computed(() =>
  refusal.value?.status === 'quota_reached' ? refusal.value.max : null,
)

const eventTo = computed(() =>
  edition.value ? localePath(`/evenements/${edition.value.slug}`) : localePath('/'),
)

/** L'espace organisation (A5) n'existe pas encore : on ne promet pas un lien mort. */
const organizationSpaceTo = computed<string | null>(() => null)
</script>

<template>
  <div class="mx-auto w-full max-w-260">
    <!-- ACCÈS REFUSÉ — la session a disparu APRÈS l'arrivée sur la page. Le
         middleware `auth` couvre l'entrée, pas l'expiration. -->
    <UiForbiddenState
      v-if="auth.isResolved && !auth.isAuthenticated"
      :title="t('common.states.forbidden.title')"
      :description="t('proposal.form.states.signedOut')"
      :action-label="t('proposal.form.states.signIn')"
      :action-to="localePath('auth-login')"
    />

    <UiErrorState
      v-else-if="contextError"
      :title="t('common.states.error.title')"
      :description="t('proposal.form.states.contextError')"
      :detail="contextError.message"
      :retry-label="t('common.actions.retry')"
      @retry="reloadContext()"
    />

    <UiLoadingState
      v-else-if="isLoading"
      variant="form"
      :lines="6"
      :label="t('proposal.form.states.loading')"
    />

    <!-- ENVOYÉ. L'écran ne montre plus le formulaire : il porte le numéro de
         dossier et la suite des opérations. -->
    <ProposalConfirmation
      v-else-if="outcome && edition"
      :reference-code="outcome.reference_code"
      :submitted-at="outcome.submitted_at"
      :required-reviews="outcome.required_reviews"
      :results-expected-at="outcome.results_expected_at"
      :edition="edition"
      :organization-space-to="organizationSpaceTo"
      :event-to="eventTo"
    />

    <!-- VIDE — aucun appel n'est ouvert aujourd'hui. Ce n'est pas une erreur :
         une COP sans pavillon n'ouvre pas d'appel, et entre deux campagnes il
         n'y a rien à déposer. -->
    <UiEmptyState
      v-else-if="!call || !edition"
      icon="inbox"
      :title="t('proposal.form.states.noCall.title')"
      :description="t('proposal.form.states.noCall.description')"
      :action-label="t('proposal.form.states.noCall.action')"
      :action-to="localePath('/')"
    />

    <!-- APPEL CLOS — la page reste atteignable par lien ou par signet. -->
    <UiEmptyState
      v-else-if="!isOpen"
      icon="clock"
      :title="t('proposal.form.states.closed.title')"
      :description="t('proposal.form.states.closed.description', {
        edition: tr(edition.title),
      })"
      :action-label="t('proposal.form.states.closed.action')"
      :action-to="eventTo"
    />

    <!-- PLAFOND ATTEINT — `max_proposals_per_organization`. Le dire ICI plutôt
         qu'après sept étapes de saisie. -->
    <UiEmptyState
      v-else-if="quotaReached"
      icon="ban"
      :title="t('proposal.form.states.quota.title')"
      :description="t('proposal.form.states.quota.description', {
        max: call.max_proposals_per_organization ?? 0,
      })"
      :action-label="t('proposal.form.states.quota.action')"
      :action-to="eventTo"
    />

    <div v-else class="grid gap-6">
      <header>
        <p class="text-sm text-text-muted">{{ tr(edition.title) }}</p>
        <h1 class="mt-1 font-display text-2xl leading-tight text-text sm:text-3xl">
          {{ t('proposal.form.title') }}
        </h1>
        <p class="mt-2 max-w-(--measure) text-text-muted">
          {{ t('proposal.form.description') }}
        </p>
      </header>

      <!-- LA PROGRESSION, visible en permanence et librement navigable. -->
      <UiStepper
        :model-value="currentStep"
        :steps="stepItems"
        :label="t('proposal.form.steps.label')"
        allow-skip-ahead
        @update:model-value="goToStep($event as ProposalFormStep)"
      />

      <div class="grid items-start gap-6 lg:grid-cols-[1fr_20rem]">
        <div class="grid min-w-0 gap-6">
          <ProposalIssueSummary
            v-if="hasTriedToSubmit"
            :issues="issues"
            :step-labels="stepLabels"
            show-warnings
            @go-to="goToStep"
          />

          <UiAlert
            v-if="submitError"
            intent="danger"
            live
            :title="t('validation.server.generic')"
            :message="submitError.message"
          />

          <!-- Les deux refus de la base, rendus en clair. -->
          <UiAlert
            v-if="refusal?.status === 'call_closed'"
            intent="warning"
            live
            :title="t('proposal.form.refusals.closed.title')"
            :message="t('proposal.form.refusals.closed.description')"
          />
          <UiAlert
            v-if="refusal?.status === 'quota_reached'"
            intent="warning"
            live
            :title="t('proposal.form.refusals.quota.title')"
            :message="t('proposal.form.refusals.quota.description', { max: refusedQuota ?? 0 })"
          />

          <section class="rounded-lg border border-border bg-surface-raised px-4 py-5 sm:px-6 sm:py-6">
            <ProposalStepOrganizations
              v-if="currentStep === 'organizations'"
              v-model="draft"
              :memberships="memberships.active"
              :issues="visibleIssuesOf('organizations')"
              :country-name-of="countryNameOf"
            />

            <ProposalStepPresentation
              v-else-if="currentStep === 'presentation'"
              v-model="draft"
              :issues="visibleIssuesOf('presentation')"
            />

            <ProposalStepClassification
              v-else-if="currentStep === 'classification' && context"
              v-model="draft"
              :call="call"
              :themes="context.themes"
              :categories="context.categories"
              :locales="context.locales"
              :country-options="countryOptions"
              :issues="visibleIssuesOf('classification')"
            />

            <ProposalStepSpeakers
              v-else-if="currentStep === 'speakers' && context"
              v-model="draft"
              :call="call"
              :issues="visibleIssuesOf('speakers')"
            />

            <ProposalStepSchedule
              v-else-if="currentStep === 'schedule'"
              v-model="draft"
              :call="call"
              :edition="edition"
              :issues="visibleIssuesOf('schedule')"
            />

            <ProposalStepDocuments
              v-else-if="currentStep === 'documents' && context"
              v-model="draft"
              :document-types="context.documentTypes"
              :issues="visibleIssuesOf('documents')"
            />

            <ProposalStepReview
              v-else-if="currentStep === 'review' && context"
              :draft="draft"
              :call="call"
              :edition="edition"
              :lead-organization="leadOrganization"
              :themes="context.themes"
              :categories="context.categories"
              :country-name-of="countryNameOf"
              :issues="issues"
              :step-labels="stepLabels"
              @go-to="goToStep"
            />
          </section>

          <!-- NAVIGATION ET ENVOI. L'envoi n'apparaît qu'à la dernière étape :
               un bouton « Envoyer » visible dès l'étape 1 se clique par erreur,
               et le refus qui suit se lit comme une panne. -->
          <div class="flex flex-wrap items-center justify-between gap-3">
            <UiButton
              variant="secondary"
              icon="arrow-left"
              :disabled="isFirstStep"
              :label="t('common.actions.previous')"
              @click="previousStep()"
            />

            <div class="flex flex-wrap items-center gap-3">
              <!-- Sans icône : le jeu n'en porte pas pour l'enregistrement, et
                   une disquette dessinée pour l'occasion serait un anachronisme
                   que personne ne lit plus. -->
              <UiButton
                variant="ghost"
                :loading="saveState === 'saving'"
                :label="t('common.actions.saveDraft')"
                @click="saveNow()"
              />
              <UiButton
                v-if="!isLastStep"
                variant="primary"
                icon-trailing="arrow-right"
                :disabled="!canGoNext"
                :label="t('common.actions.next')"
                @click="nextStep()"
              />
              <UiButton
                v-else
                variant="primary"
                size="lg"
                icon="check"
                :loading="isSubmitting"
                :label="t('proposal.form.submit')"
                @click="submit()"
              />
            </div>
          </div>

          <!-- POURQUOI LE BOUTON EST ÉTEINT. Un bouton désactivé sans explication
               est une porte fermée sans écriteau : on le dit, on le compte, et
               les champs concernés sont signalés juste au-dessus. -->
          <p
            v-if="!canGoNext && !isLastStep"
            class="flex items-start gap-2 text-sm text-danger"
            role="status"
          >
            <UiIcon name="error" size="1.05rem" class="mt-0.5 shrink-0" />
            {{
              t(
                'proposal.form.steps.blocked',
                { count: currentStepErrors.length },
                currentStepErrors.length,
              )
            }}
          </p>

          <ProposalAutosave
            :state="saveState"
            :saved-at="savedAt"
            :reference-code="referenceCode"
            :timezone="auth.person?.timezone ?? edition.timezone"
            @retry="saveNow()"
          />
        </div>

        <!-- L'ENCART PERMANENT. En colonne latérale sur écran large, en tête de
             la pile sur mobile — jamais replié, jamais fermable. -->
        <ProposalDeadline class="lg:sticky lg:top-6 order-first lg:order-last" :call="call" :edition="edition" />
      </div>
    </div>
  </div>
</template>
