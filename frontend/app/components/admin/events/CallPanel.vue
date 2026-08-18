<script setup lang="ts">
import type {
  CallFormError,
  EditionCallPayload,
  EditionCriterion,
  EditionDetail,
} from '~/types/admin-events'
import type { CallStatus } from '~/types/event/call'
import type { ParticipationMode } from '~/types/event/edition'
import type { SelectOption } from '~/types/ui'

/**
 * ONGLET « APPEL À PROPOSITIONS ».
 *
 * ── UN SEUL PAR ÉDITION, ZÉRO S'IL N'Y A PAS DE PAVILLON ────────────────────
 *
 * Règle métier n° 5, et cardinalité 0..1 tenue par `ux_calls_one_per_event`. Cet
 * écran n'offre donc jamais « ajouter un second appel » : il montre l'appel, ou il
 * propose d'en ouvrir un. Et quand l'édition ne tient pas de pavillon, il l'explique
 * plutôt que d'inviter à ouvrir un appel qui n'a pas lieu d'être — sans pavillon,
 * l'IFDD n'envoie qu'un représentant.
 *
 * ── L'APPEL ET SA GRILLE S'ENREGISTRENT ENSEMBLE ────────────────────────────
 *
 * Un appel sans critère ne peut recevoir aucune évaluation :
 * `refresh_proposal_score()` n'aurait rien à pondérer, et le comité se retrouverait
 * devant une fiche vide. Deux enregistrements distincts laisseraient exister cet
 * état le temps d'un oubli, d'où un seul formulaire et un seul envoi.
 *
 * ── CE QUI SE DIT AVANT D'ÊTRE FAIT ─────────────────────────────────────────
 *
 * Un critère DÉJÀ NOTÉ dont on change le barème déplace des moyennes et un
 * classement. `programme.review_scores` référence le critère, rien n'est perdu —
 * mais un rang qui bouge sans explication est une conversation difficile avec le
 * comité. Chaque ligne porte donc son nombre de notes, et la ligne notée prévient.
 *
 * ── LA PROLONGATION EST UNE COLONNE À PART, ET C'EST VOULU ──────────────────
 *
 * `extended_until` conserve la trace de l'échéance INITIALEMENT ANNONCÉE aux
 * organisations : écraser `closes_at` effacerait ce qui leur avait été communiqué.
 * Les deux dates s'affichent donc côte à côte.
 */

interface Props {
  detail: EditionDetail
  canManage: boolean
  busy?: boolean
  errors: CallFormError[]
  /** La grille par défaut du modèle — `event.seed_default_criteria()`. */
  defaultCriteria: EditionCriterion[]
  scoresAffected?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ save: [payload: EditionCallPayload] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()

const call = computed(() => props.detail.call)
const edition = computed(() => props.detail.edition)
const timezone = computed(() => edition.value.timezone)
const zoneCity = computed(() => edition.value.city ?? timeZoneCityLabel(timezone.value))

const STATUSES: CallStatus[] = ['draft', 'open', 'closed', 'under_review', 'published', 'cancelled']
const MODES: ParticipationMode[] = ['in_person', 'hybrid', 'online']

const statusOptions = computed<SelectOption[]>(() =>
  STATUSES.map((status) => ({
    value: status,
    label: t('admin.event.list.callStatus.' + status),
  })),
)

// ---------------------------------------------------------------------------
// Formulaire
// ---------------------------------------------------------------------------

const editing = ref(false)
const draft = ref<EditionCallPayload | null>(null)

/** Heures murales des quatre instants : le fuseau est celui de l'édition. */
const wall = ref({ opens_at: '', closes_at: '', extended_until: '' })

function toWall(instant: string | null): string {
  return instant ? wallClockInZone(instant, timezone.value).replace(' ', 'T') : ''
}

function openForm(): void {
  const current = call.value
  draft.value = current
    ? {
        ...current,
        allowed_formats: [...current.allowed_formats],
        criteria: current.criteria.map((c) => ({ ...c })),
      }
    : {
        id: null,
        event_id: edition.value.id,
        code: `${(edition.value.acronym ?? 'appel').toLowerCase().replace(/[^a-z0-9]+/g, '_')}_activites`,
        title: { fr: '' },
        description: null,
        status: 'draft',
        // La fenêtre part de la période de l'édition : un appel se clôt avant la
        // COP, jamais après. On propose, l'équipe ajuste.
        opens_at: new Date().toISOString(),
        closes_at: edition.value.starts_at,
        extended_until: null,
        results_expected_at: null,
        max_proposals_per_organization: null,
        requires_verified_organization: false,
        min_speakers: 1,
        max_speakers: 10,
        default_duration_minutes: 60,
        min_duration_minutes: 45,
        max_duration_minutes: 150,
        daily_start_time: '09:00:00',
        daily_end_time: '17:00:00',
        allowed_formats: ['in_person', 'hybrid', 'online'],
        required_reviews: 2,
        blind_review: true,
        guidelines_url: null,
        // La grille par défaut du MODÈLE, lue et non recopiée : six lignes vides
        // devant un formulaire de grille se remplissent mal.
        criteria: props.defaultCriteria.map((c) => ({ ...c })),
      }

  wall.value = {
    opens_at: toWall(draft.value.opens_at),
    closes_at: toWall(draft.value.closes_at),
    extended_until: toWall(draft.value.extended_until),
  }
  editing.value = true
}

function setWall(key: 'opens_at' | 'closes_at' | 'extended_until', value: string): void {
  wall.value[key] = value
  if (!draft.value) return
  const instant = instantFromWallClock(value, timezone.value)
  if (key === 'extended_until') draft.value.extended_until = instant
  else draft.value[key] = instant ?? ''
}

/** `time` PostgreSQL vaut `HH:MM:SS` ; le contrôle natif rend `HH:MM`. */
function setDailyTime(key: 'daily_start_time' | 'daily_end_time', value: string): void {
  if (draft.value) draft.value[key] = value.length === 5 ? `${value}:00` : value
}

function dailyValue(value: string): string {
  return value.slice(0, 5)
}

function toggleFormat(mode: ParticipationMode, on: boolean): void {
  if (!draft.value) return
  const set = new Set(draft.value.allowed_formats)
  if (on) set.add(mode)
  else set.delete(mode)
  // Au moins un format : un appel qui n'accepte rien ne peut recevoir aucun dossier.
  draft.value.allowed_formats = set.size > 0 ? MODES.filter((m) => set.has(m)) : [mode]
}

// ---------------------------------------------------------------------------
// Grille de critères
// ---------------------------------------------------------------------------

function addCriterion(): void {
  if (!draft.value) return
  draft.value.criteria.push({
    id: null,
    code: '',
    label: { fr: '' },
    description: null,
    max_score: 5,
    weight: 1,
    is_knockout: false,
    sort_order: (draft.value.criteria.length + 1) * 10,
    score_count: 0,
  })
}

function removeCriterion(index: number): void {
  draft.value?.criteria.splice(index, 1)
}

function loadDefaultGrid(): void {
  if (draft.value) draft.value.criteria = props.defaultCriteria.map((c) => ({ ...c }))
}

/** `event.max_weighted_score()`, recalculée à la frappe : c'est la note affichée. */
const draftMaxScore = computed(
  () =>
    draft.value?.criteria.reduce(
      (sum, criterion) => sum + Number(criterion.max_score) * Number(criterion.weight),
      0,
    ) ?? 0,
)

// ---------------------------------------------------------------------------
// Erreurs
// ---------------------------------------------------------------------------

function errorOf(field: string): string | undefined {
  const found = props.errors.find(
    (entry) => entry.field === field && entry.criterion_index === null,
  )
  return found ? t('admin.event.tabs.callTab.errors.' + found.code) : undefined
}

function criterionError(index: number): string | undefined {
  const found = props.errors.find((entry) => entry.criterion_index === index)
  return found ? t('admin.event.tabs.callTab.errors.' + found.code) : undefined
}

const globalErrors = computed(() =>
  props.errors
    .filter((entry) => entry.field === null || entry.field === 'criteria')
    .map((entry) => t('admin.event.tabs.callTab.errors.' + entry.code)),
)

function submit(): void {
  if (draft.value) emit('save', { ...draft.value })
}

/** Fermer le formulaire quand l'appel a effectivement changé et qu'aucune erreur ne reste. */
watch(
  () => props.detail.call,
  () => {
    if (props.errors.length === 0) editing.value = false
  },
)
</script>

<template>
  <section>
    <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
      <div class="min-w-0">
        <h2 class="font-display text-xl font-semibold">
          {{ t('admin.event.tabs.callTab.title') }}
        </h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.event.tabs.callTab.intro') }}
        </p>
      </div>

      <UiButton
        v-if="props.canManage && call && !editing"
        variant="secondary"
        icon="edit"
        @click="openForm"
      >
        {{ t('admin.event.tabs.callTab.edit') }}
      </UiButton>
    </header>

    <UiAlert
      v-if="props.scoresAffected"
      class="mt-4"
      intent="warning"
      live
      :message="t('admin.event.tabs.callTab.scoresAffected')"
    />

    <!-- SANS PAVILLON, PAS D'APPEL. On l'explique, on n'invite pas. ------------>
    <UiEmptyState
      v-if="!call && !edition.has_pavilion && !editing"
      class="mt-5"
      icon="info"
      :title="t('admin.event.tabs.callTab.noPavilion.title')"
      :description="t('admin.event.tabs.callTab.noPavilion.description')"
    />

    <UiEmptyState
      v-else-if="!call && !editing"
      class="mt-5"
      icon="inbox"
      :title="t('admin.event.tabs.callTab.empty.title')"
      :description="t('admin.event.tabs.callTab.empty.description')"
      :action-label="props.canManage ? t('admin.event.tabs.callTab.create') : undefined"
      @action="openForm"
    />

    <!-- RÉCAPITULATIF DE L'APPEL ---------------------------------------------->
    <div v-else-if="call && !editing" class="mt-5 space-y-5">
      <UiCard :title="tr(call.title)" :eyebrow="call.code">
        <div class="flex flex-wrap items-center gap-2">
          <UiBadge
            :intent="call.is_open ? 'success' : 'neutral'"
            :label="t(call.is_open ? 'admin.event.tabs.callTab.summary.isOpen' : 'admin.event.tabs.callTab.summary.isClosed')"
          />
          <UiBadge
            intent="info"
            :label="t('admin.event.list.callStatus.' + call.status)"
          />
          <UiBadge
            :intent="call.blind_review ? 'info' : 'neutral'"
            :label="t(call.blind_review ? 'admin.event.tabs.callTab.summary.blind' : 'admin.event.tabs.callTab.summary.notBlind')"
          />
        </div>

        <p v-if="call.description" class="mt-3 max-w-(--measure) text-sm text-text-secondary">
          {{ tr(call.description) }}
        </p>

        <dl class="mt-4 grid gap-x-6 gap-y-3 sm:grid-cols-2">
          <div>
            <dt class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
              {{ t('admin.event.tabs.callTab.form.sections.window') }}
            </dt>
            <dd class="mt-0.5 text-sm text-text">
              {{ t('admin.event.tabs.callTab.summary.window', {
                open: dateTime(call.opens_at, timezone),
                close: dateTime(call.closes_at, timezone),
              }) }}
            </dd>
            <!-- La prolongation ET l'échéance annoncée à l'origine : c'est ce qui a
                 été communiqué aux organisations, et cela reste lisible. -->
            <dd v-if="call.extended_until" class="mt-0.5 text-sm font-medium text-warning">
              {{ t('admin.event.tabs.callTab.summary.extended', {
                date: dateTime(call.extended_until, timezone),
              }) }}
            </dd>
            <dd v-if="call.extended_until" class="text-xs text-text-subtle">
              {{ t('admin.event.tabs.callTab.summary.originalDeadline', {
                date: dateTime(call.closes_at, timezone),
              }) }}
            </dd>
            <dd v-if="call.results_expected_at" class="mt-0.5 text-sm text-text-muted">
              {{ t('admin.event.tabs.callTab.summary.results', {
                date: date(`${call.results_expected_at}T12:00:00Z`, timezone),
              }) }}
            </dd>
          </div>

          <div>
            <dt class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
              {{ t('admin.event.tabs.callTab.form.sections.eligibility') }}
            </dt>
            <dd class="mt-0.5 text-sm text-text">
              {{
                call.max_proposals_per_organization
                  ? t('admin.event.tabs.callTab.summary.perOrganization', call.max_proposals_per_organization)
                  : t('admin.event.tabs.callTab.summary.perOrganizationNone')
              }}
            </dd>
            <dd v-if="call.requires_verified_organization" class="text-sm text-text-muted">
              {{ t('admin.event.tabs.callTab.summary.verifiedOnly') }}
            </dd>
            <dd class="text-sm text-text-muted">
              {{ t('admin.event.tabs.callTab.summary.speakers', {
                min: call.min_speakers,
                max: call.max_speakers,
              }) }}
            </dd>
          </div>

          <div>
            <dt class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
              {{ t('admin.event.tabs.callTab.form.sections.duration') }}
            </dt>
            <dd class="mt-0.5 text-sm text-text">
              {{ t('admin.event.tabs.callTab.summary.duration', {
                min: call.min_duration_minutes,
                max: call.max_duration_minutes,
                default: call.default_duration_minutes,
              }) }}
            </dd>
            <!-- La plage d'accueil PORTE SON FUSEAU : c'est une heure locale du
                 pavillon, pas celle du navigateur. -->
            <dd class="text-sm text-text-muted">
              {{ t('admin.event.tabs.callTab.summary.dailyWindow', {
                start: dailyValue(call.daily_start_time),
                end: dailyValue(call.daily_end_time),
                zone: zoneCity,
              }) }}
            </dd>
          </div>

          <div>
            <dt class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
              {{ t('admin.event.tabs.callTab.form.sections.review') }}
            </dt>
            <dd class="mt-0.5 text-sm text-text">
              {{ t('admin.event.tabs.callTab.summary.reviews', call.required_reviews) }}
            </dd>
            <dd class="text-sm text-text-muted">
              {{ t('admin.event.tabs.callTab.summary.maxScore', {
                score: call.max_weighted_score.toLocaleString('fr-FR'),
              }) }}
            </dd>
            <dd class="text-sm text-text-muted">
              {{ t('admin.event.tabs.callTab.summary.proposals', call.proposal_count) }}
            </dd>
          </div>
        </dl>

        <UiButton
          v-if="call.guidelines_url"
          class="mt-4"
          variant="link"
          size="sm"
          :href="call.guidelines_url"
          icon="document"
        >
          {{ t('admin.event.tabs.callTab.summary.guidelines') }}
        </UiButton>
      </UiCard>

      <!-- LA GRILLE, EN LECTURE ------------------------------------------------>
      <UiCard :title="t('admin.event.tabs.callTab.form.sections.criteria')">
        <ul class="divide-y divide-border">
          <li
            v-for="criterion in call.criteria"
            :key="criterion.id ?? criterion.code"
            class="flex flex-wrap items-start gap-x-4 gap-y-1 py-3 first:pt-0"
          >
            <div class="min-w-0 flex-1">
              <p class="flex flex-wrap items-center gap-2 text-sm font-medium text-text">
                {{ tr(criterion.label) }}
                <!-- Rouge : le critère éliminatoire écarte un dossier. -->
                <UiBadge
                  v-if="criterion.is_knockout"
                  intent="danger"
                  size="sm"
                  :label="t('admin.event.tabs.callTab.criteria.columns.knockout')"
                />
              </p>
              <p v-if="criterion.description" class="mt-0.5 text-sm text-text-muted">
                {{ tr(criterion.description) }}
              </p>
              <p v-if="criterion.score_count > 0" class="mt-0.5 text-xs text-text-subtle">
                {{ t('admin.event.tabs.callTab.criteria.scored', criterion.score_count) }}
              </p>
            </div>
            <p class="shrink-0 font-mono text-sm tabular-nums text-text-muted">
              {{ criterion.max_score }} × {{ criterion.weight }}
            </p>
          </li>
        </ul>

        <p class="mt-3 border-t border-border pt-3 text-sm font-semibold text-text">
          {{ t('admin.event.tabs.callTab.criteria.maxWeighted', {
            score: call.max_weighted_score.toLocaleString('fr-FR'),
          }) }}
        </p>
      </UiCard>
    </div>

    <!-- FORMULAIRE ------------------------------------------------------------>
    <form
      v-if="editing && draft"
      class="mt-5 space-y-6"
      novalidate
      @submit.prevent="submit"
    >
      <UiAlert
        v-if="props.errors.length > 0"
        intent="danger"
        live
        :title="t('admin.event.tabs.callTab.errors.title')"
      >
        <ul v-if="globalErrors.length > 0" class="mt-1 space-y-0.5 text-sm">
          <li v-for="message in globalErrors" :key="message">{{ message }}</li>
        </ul>
      </UiAlert>

      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
        <legend class="px-2 font-semibold">
          {{ t('admin.event.tabs.callTab.form.sections.identity') }}
        </legend>

        <div class="mt-3 space-y-5">
          <AdminEventsI18nField
            :model-value="draft.title"
            :label="t('admin.event.tabs.callTab.form.titleField')"
            :error="errorOf('title')"
            required
            @update:model-value="(next) => (draft!.title = next ?? { fr: '' })"
          />

          <div class="grid gap-4 sm:grid-cols-2">
            <UiInput
              :model-value="draft.code"
              :label="t('admin.event.tabs.callTab.form.codeField')"
              :error="errorOf('code')"
              required
              @update:model-value="(next: string) => (draft!.code = next)"
            />
            <UiSelect
              :model-value="draft.status"
              :label="t('admin.event.tabs.callTab.form.statusField')"
              :options="statusOptions"
              hide-optional
              @update:model-value="(next: string) => (draft!.status = next as CallStatus)"
            />
          </div>

          <AdminEventsI18nField
            :model-value="draft.description"
            :label="t('admin.event.tabs.callTab.form.descriptionField')"
            multiline
            :rows="4"
            @update:model-value="(next) => (draft!.description = next)"
          />

          <UiInput
            :model-value="draft.guidelines_url ?? ''"
            type="url"
            :label="t('admin.event.tabs.callTab.form.guidelinesUrl')"
            @update:model-value="(next: string) => (draft!.guidelines_url = next || null)"
          />
        </div>
      </fieldset>

      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
        <legend class="px-2 font-semibold">
          {{ t('admin.event.tabs.callTab.form.sections.window') }}
        </legend>

        <div class="mt-3 grid gap-4 lg:grid-cols-2">
          <UiDatePicker
            :model-value="wall.opens_at"
            with-time
            :label="t('admin.event.tabs.callTab.form.opensAt')"
            :timezone-label="t('common.datetime.zoneOf', { zone: zoneCity })"
            :error="errorOf('opens_at')"
            required
            @update:model-value="(next: string) => setWall('opens_at', next)"
          />
          <UiDatePicker
            :model-value="wall.closes_at"
            with-time
            :label="t('admin.event.tabs.callTab.form.closesAt')"
            :timezone-label="t('common.datetime.zoneOf', { zone: zoneCity })"
            :min="wall.opens_at || undefined"
            :error="errorOf('closes_at')"
            required
            @update:model-value="(next: string) => setWall('closes_at', next)"
          />
          <UiDatePicker
            :model-value="wall.extended_until"
            with-time
            :label="t('admin.event.tabs.callTab.form.extendedUntil')"
            :hint="t('admin.event.tabs.callTab.form.extendedHint')"
            :timezone-label="t('common.datetime.zoneOf', { zone: zoneCity })"
            :min="wall.closes_at || undefined"
            :error="errorOf('extended_until')"
            @update:model-value="(next: string) => setWall('extended_until', next)"
          />
          <UiDatePicker
            :model-value="draft.results_expected_at"
            :label="t('admin.event.tabs.callTab.form.resultsExpectedAt')"
            @update:model-value="(next: string) => (draft!.results_expected_at = next || null)"
          />
        </div>
      </fieldset>

      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
        <legend class="px-2 font-semibold">
          {{ t('admin.event.tabs.callTab.form.sections.eligibility') }}
        </legend>

        <div class="mt-3 grid gap-4 lg:grid-cols-3">
          <UiInput
            :model-value="draft.max_proposals_per_organization ?? ''"
            type="number"
            :min="1"
            :label="t('admin.event.tabs.callTab.form.maxPerOrganization')"
            :hint="t('admin.event.tabs.callTab.form.maxPerOrganizationHint')"
            @update:model-value="
              (next: string) => (draft!.max_proposals_per_organization = next ? Number(next) : null)
            "
          />
          <UiInput
            :model-value="draft.min_speakers"
            type="number"
            :min="0"
            :label="t('admin.event.tabs.callTab.form.minSpeakers')"
            @update:model-value="(next: string) => (draft!.min_speakers = Number(next))"
          />
          <UiInput
            :model-value="draft.max_speakers"
            type="number"
            :min="1"
            :label="t('admin.event.tabs.callTab.form.maxSpeakers')"
            :error="errorOf('max_speakers')"
            @update:model-value="(next: string) => (draft!.max_speakers = Number(next))"
          />
        </div>

        <UiSwitch
          class="mt-4"
          :model-value="draft.requires_verified_organization"
          :label="t('admin.event.tabs.callTab.form.requiresVerified')"
          @update:model-value="(next: boolean) => (draft!.requires_verified_organization = next)"
        />

        <fieldset class="mt-4">
          <legend class="mb-2 text-sm font-bold text-text">
            {{ t('admin.event.tabs.callTab.form.allowedFormats') }}
          </legend>
          <div class="flex flex-wrap gap-4">
            <UiCheckbox
              v-for="mode in MODES"
              :key="mode"
              :model-value="draft.allowed_formats.includes(mode)"
              :label="t('admin.event.form.mode.' + mode)"
              @update:model-value="(next: boolean) => toggleFormat(mode, next)"
            />
          </div>
        </fieldset>
      </fieldset>

      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
        <legend class="px-2 font-semibold">
          {{ t('admin.event.tabs.callTab.form.sections.duration') }}
        </legend>
        <p class="mb-4 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.event.tabs.callTab.form.durationHint') }}
        </p>

        <div class="grid gap-4 lg:grid-cols-3">
          <UiInput
            :model-value="draft.min_duration_minutes"
            type="number"
            :min="15"
            :max="600"
            :label="t('admin.event.tabs.callTab.form.minDuration')"
            @update:model-value="(next: string) => (draft!.min_duration_minutes = Number(next))"
          />
          <UiInput
            :model-value="draft.max_duration_minutes"
            type="number"
            :min="15"
            :max="600"
            :label="t('admin.event.tabs.callTab.form.maxDuration')"
            @update:model-value="(next: string) => (draft!.max_duration_minutes = Number(next))"
          />
          <UiInput
            :model-value="draft.default_duration_minutes"
            type="number"
            :min="15"
            :max="600"
            :label="t('admin.event.tabs.callTab.form.defaultDuration')"
            :error="errorOf('default_duration_minutes')"
            @update:model-value="(next: string) => (draft!.default_duration_minutes = Number(next))"
          />
        </div>

        <div class="mt-4 grid gap-4 lg:grid-cols-2">
          <UiFormField :label="t('admin.event.tabs.callTab.form.dailyStart')">
            <template #default="{ control }">
              <input
                :id="control.id"
                type="time"
                :value="dailyValue(draft.daily_start_time)"
                class="w-full min-h-(--target-min) rounded-md border border-border bg-surface-raised px-3 tabular-nums"
                @input="setDailyTime('daily_start_time', ($event.target as HTMLInputElement).value)"
              >
            </template>
          </UiFormField>
          <UiFormField
            :label="t('admin.event.tabs.callTab.form.dailyEnd')"
            :hint="t('admin.event.tabs.callTab.form.dailyHint')"
            :error="errorOf('daily_end_time')"
          >
            <template #default="{ control }">
              <input
                :id="control.id"
                type="time"
                :value="dailyValue(draft.daily_end_time)"
                class="w-full min-h-(--target-min) rounded-md border border-border bg-surface-raised px-3 tabular-nums"
                @input="setDailyTime('daily_end_time', ($event.target as HTMLInputElement).value)"
              >
            </template>
          </UiFormField>
        </div>
      </fieldset>

      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
        <legend class="px-2 font-semibold">
          {{ t('admin.event.tabs.callTab.form.sections.review') }}
        </legend>

        <div class="mt-3 space-y-4">
          <UiInput
            :model-value="draft.required_reviews"
            type="number"
            :min="0"
            :label="t('admin.event.tabs.callTab.form.requiredReviews')"
            @update:model-value="(next: string) => (draft!.required_reviews = Number(next))"
          />
          <UiSwitch
            :model-value="draft.blind_review"
            :label="t('admin.event.tabs.callTab.form.blindReview')"
            :hint="t('admin.event.tabs.callTab.form.blindReviewHint')"
            @update:model-value="(next: boolean) => (draft!.blind_review = next)"
          />
        </div>
      </fieldset>

      <!-- LA GRILLE PONDÉRÉE ------------------------------------------------->
      <fieldset class="rounded-lg border border-border bg-surface-raised p-5" :disabled="props.busy">
        <legend class="px-2 font-semibold">
          {{ t('admin.event.tabs.callTab.form.sections.criteria') }}
        </legend>

        <p class="mb-4 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.event.tabs.callTab.criteria.knockoutHint') }}
        </p>

        <p v-if="draft.criteria.length === 0" class="rounded-md border border-danger-border bg-danger-surface p-3 text-sm text-danger">
          {{ t('admin.event.tabs.callTab.criteria.empty') }}
        </p>

        <ul v-else class="space-y-4">
          <li
            v-for="(criterion, index) in draft.criteria"
            :key="criterion.id ?? `new-${index}`"
            class="rounded-md border border-border p-4"
          >
            <div class="space-y-4">
              <AdminEventsI18nField
                :model-value="criterion.label"
                :label="t('admin.event.tabs.callTab.criteria.columns.label')"
                :error="criterionError(index)"
                required
                @update:model-value="(next) => (criterion.label = next ?? { fr: '' })"
              />

              <div class="grid gap-4 sm:grid-cols-4">
                <UiInput
                  :model-value="criterion.code"
                  :label="t('admin.event.tabs.callTab.criteria.columns.code')"
                  required
                  @update:model-value="(next: string) => (criterion.code = next)"
                />
                <UiInput
                  :model-value="criterion.max_score"
                  type="number"
                  :min="0.5"
                  step="0.5"
                  :label="t('admin.event.tabs.callTab.criteria.columns.maxScore')"
                  @update:model-value="(next: string) => (criterion.max_score = Number(next))"
                />
                <UiInput
                  :model-value="criterion.weight"
                  type="number"
                  :min="0.5"
                  step="0.5"
                  :label="t('admin.event.tabs.callTab.criteria.columns.weight')"
                  @update:model-value="(next: string) => (criterion.weight = Number(next))"
                />
                <div class="flex items-end">
                  <UiCheckbox
                    :model-value="criterion.is_knockout"
                    :label="t('admin.event.tabs.callTab.criteria.columns.knockout')"
                    @update:model-value="(next: boolean) => (criterion.is_knockout = next)"
                  />
                </div>
              </div>

              <!-- UN CRITÈRE DÉJÀ NOTÉ : le dire avant, pas après. -->
              <UiAlert
                v-if="criterion.score_count > 0"
                intent="warning"
                compact
                :message="t('admin.event.tabs.callTab.criteria.scoredWarning')"
              />

              <div class="flex justify-end">
                <UiButton
                  variant="ghost"
                  size="sm"
                  icon="trash"
                  @click="removeCriterion(index)"
                >
                  {{ t('admin.event.tabs.callTab.criteria.remove') }}
                </UiButton>
              </div>
            </div>
          </li>
        </ul>

        <div class="mt-4 flex flex-wrap items-center gap-3 border-t border-border pt-4">
          <UiButton variant="secondary" size="sm" icon="plus" @click="addCriterion">
            {{ t('admin.event.tabs.callTab.criteria.add') }}
          </UiButton>
          <UiButton variant="ghost" size="sm" icon="refresh" @click="loadDefaultGrid">
            {{ t('admin.event.tabs.callTab.criteria.loadDefault') }}
          </UiButton>
          <p class="ml-auto text-sm font-semibold text-text">
            {{ t('admin.event.tabs.callTab.criteria.maxWeighted', {
              score: draftMaxScore.toLocaleString('fr-FR'),
            }) }}
          </p>
        </div>
      </fieldset>

      <div class="flex flex-wrap gap-3">
        <UiButton type="submit" :loading="props.busy">{{ t('common.actions.save') }}</UiButton>
        <UiButton variant="ghost" :disabled="props.busy" @click="editing = false">
          {{ t('admin.event.tabs.confirm.cancel') }}
        </UiButton>
      </div>
    </form>
  </section>
</template>
