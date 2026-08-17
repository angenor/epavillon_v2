<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type { Organization } from '~/types/org'
import type { TaxonomyTerm } from '~/types/reference'
import type { DraftIssue, ProposalDraft, ProposalFormStep } from '~/types/proposal-form'

/**
 * ÉTAPE 7 — RELECTURE ET ENVOI.
 *
 * TOUT EST RÉCAPITULÉ, y compris ce qui manque. Un récapitulatif qui n'affiche
 * que les champs remplis laisse croire le dossier complet : les absences y sont
 * donc écrites en toutes lettres — « non renseigné » — et les défauts de l'étape
 * concernée sont rappelés sous chaque bloc.
 *
 * CHAQUE BLOC RAMÈNE À SON ÉTAPE, d'un seul geste. C'est la seule façon de
 * corriger un formulaire de sept étapes sans le reparcourir : on lit, on
 * repère, on clique, on corrige, on revient.
 *
 * LES LIBELLÉS VIENNENT D'OÙ ILS DOIVENT VENIR : thématiques, catégorie et pays
 * sont résolus depuis la base ; format, rôles et civilités sont des ENUM du
 * modèle, donc des libellés d'interface. Les deux ne se confondent jamais —
 * c'est le piège exact de la v1.
 *
 * LE BLOC DES DOCUMENTS A DISPARU avec son étape, masquée le 17/08 à la demande
 * du commanditaire : le comité n'aura pas le temps de lire des pièces jointes
 * pour cette campagne. Récapituler une étape qu'on ne peut plus remplir aurait
 * seulement rappelé ce qui manque.
 */

interface Props {
  draft: ProposalDraft
  call: CallForProposals
  edition: EventEdition
  /** Organisation porteuse, résolue par l'écran. */
  leadOrganization: Organization | null
  themes: TaxonomyTerm[]
  categories: TaxonomyTerm[]
  countryNameOf: (countryId: string | null) => string | null
  issues: DraftIssue[]
  stepLabels: Record<ProposalFormStep, string>
}

const props = defineProps<Props>()
const emit = defineEmits<{ goTo: [step: ProposalFormStep] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()

const zone = computed(() => props.edition.timezone)

/** Le texte, ou la mention d'absence : jamais un blanc. */
function orMissing(value: string): string {
  return value.trim().length > 0 ? value : t('common.labels.unknown')
}

function issuesOfStep(step: ProposalFormStep): DraftIssue[] {
  return props.issues.filter((issue) => issue.step === step)
}

const selectedThemes = computed(() =>
  props.themes.filter((term) => props.draft.theme_codes.includes(term.code)),
)

const categoryLabel = computed(() => {
  const term = props.categories.find((entry) => entry.code === props.draft.activity_type_code)
  return term ? tr(term.label) : t('common.labels.unknown')
})

const languagesLabel = computed(() =>
  props.draft.language_codes
    .map((code) => t(`proposal.form.step-review.languages.${code}`))
    .join(' · '),
)

const startLabel = computed(() => {
  const instant = instantFromWallClock(props.draft.preferred_start_at, zone.value)
  return instant ? dateTime(instant, zone.value) : t('common.labels.unknown')
})

const durationLabel = computed(() =>
  props.draft.duration_minutes
    ? t('proposal.form.step-schedule.duration.value', { minutes: props.draft.duration_minutes })
    : t('common.labels.unknown'),
)
</script>

<template>
  <div class="grid gap-6">
    <header>
      <h2 class="font-display text-xl text-text">{{ t('proposal.form.step-review.title') }}</h2>
      <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
        {{ t('proposal.form.step-review.description') }}
      </p>
    </header>

    <!-- ORGANISATIONS -->
    <section class="rounded-lg border border-border">
      <div class="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
        <h3 class="font-bold text-text">{{ props.stepLabels.organizations }}</h3>
        <UiButton
          variant="ghost"
          size="sm"
          icon="edit"
          :label="t('proposal.form.step-review.edit')"
          @click="emit('goTo', 'organizations')"
        />
      </div>
      <dl class="grid gap-3 px-4 py-4 text-sm">
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.lead') }}</dt>
          <dd class="text-text">
            {{ props.leadOrganization?.legal_name ?? t('common.labels.unknown') }}
            <span v-if="props.leadOrganization?.acronym" class="text-text-muted">
              ({{ props.leadOrganization.acronym }})
            </span>
          </dd>
        </div>
        <div v-if="props.draft.co_organizations.length > 0">
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.coOrganizations') }}</dt>
          <dd>
            <ul class="grid gap-1">
              <li v-for="entry in props.draft.co_organizations" :key="entry.organization_id" class="text-text">
                {{ entry.legal_name }}
                <span class="text-text-muted">
                  — {{ t(`proposal.form.step-organizations.roles.${entry.role}`) }}
                </span>
              </li>
            </ul>
          </dd>
        </div>
      </dl>
    </section>

    <!-- PRÉSENTATION -->
    <section class="rounded-lg border border-border">
      <div class="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
        <h3 class="font-bold text-text">{{ props.stepLabels.presentation }}</h3>
        <UiButton
          variant="ghost"
          size="sm"
          icon="edit"
          :label="t('proposal.form.step-review.edit')"
          @click="emit('goTo', 'presentation')"
        />
      </div>
      <dl class="grid gap-4 px-4 py-4 text-sm">
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.title') }}</dt>
          <dd class="font-display text-lg text-text">{{ orMissing(props.draft.title) }}</dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.summary') }}</dt>
          <dd class="max-w-(--measure) whitespace-pre-line text-text-secondary">
            {{ orMissing(props.draft.summary) }}
          </dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.objectives') }}</dt>
          <dd class="max-w-(--measure) whitespace-pre-line text-text-secondary">
            {{ orMissing(props.draft.objectives) }}
          </dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.presentation') }}</dt>
          <dd>
            <!-- Rendu avec LA MÊME feuille que l'éditeur : ce que le comité lira
                 est exactement ce qui a été composé. -->
            <UiRichContent :html="props.draft.detailed_presentation" />
            <span v-if="plainTextOf(props.draft.detailed_presentation).length === 0" class="text-text-muted">
              {{ t('common.labels.unknown') }}
            </span>
          </dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.outcomes') }}</dt>
          <dd class="max-w-(--measure) whitespace-pre-line text-text-secondary">
            {{ orMissing(props.draft.expected_outcomes) }}
          </dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.audience') }}</dt>
          <dd v-if="props.draft.target_audiences.length > 0" class="mt-1 flex flex-wrap gap-1.5">
            <UiBadge v-for="audience in props.draft.target_audiences" :key="audience" size="sm">
              {{ audience }}
            </UiBadge>
          </dd>
          <dd v-else class="text-text-muted">{{ t('common.labels.unknown') }}</dd>
        </div>
      </dl>
    </section>

    <!-- CLASSIFICATION -->
    <section class="rounded-lg border border-border">
      <div class="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
        <h3 class="font-bold text-text">{{ props.stepLabels.classification }}</h3>
        <UiButton
          variant="ghost"
          size="sm"
          icon="edit"
          :label="t('proposal.form.step-review.edit')"
          @click="emit('goTo', 'classification')"
        />
      </div>
      <dl class="grid gap-3 px-4 py-4 text-sm sm:grid-cols-2">
        <div class="sm:col-span-2">
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.themes') }}</dt>
          <dd v-if="selectedThemes.length > 0" class="mt-1 flex flex-wrap gap-1.5">
            <UiBadge
              v-for="term in selectedThemes"
              :key="term.code"
              size="sm"
              :dot-color="term.color_hex"
            >
              {{ tr(term.label) }}
            </UiBadge>
          </dd>
          <dd v-else class="text-text-muted">{{ t('common.labels.unknown') }}</dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.category') }}</dt>
          <dd class="text-text">{{ categoryLabel }}</dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.format') }}</dt>
          <dd class="text-text">
            {{
              props.draft.format
                ? t(`proposal.form.step-classification.formats.${props.draft.format}.label`)
                : t('common.labels.unknown')
            }}
          </dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.languages') }}</dt>
          <dd class="text-text">{{ orMissing(languagesLabel) }}</dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.country') }}</dt>
          <dd class="text-text">
            {{ props.countryNameOf(props.draft.country_id) ?? t('common.labels.unknown') }}
          </dd>
        </div>
      </dl>
    </section>

    <!-- INTERVENANTS -->
    <section class="rounded-lg border border-border">
      <div class="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
        <h3 class="font-bold text-text">
          {{ props.stepLabels.speakers }}
          <UiCounter class="ml-1" :value="props.draft.speakers.length" />
        </h3>
        <UiButton
          variant="ghost"
          size="sm"
          icon="edit"
          :label="t('proposal.form.step-review.edit')"
          @click="emit('goTo', 'speakers')"
        />
      </div>
      <ul v-if="props.draft.speakers.length > 0" class="grid gap-3 px-4 py-4 text-sm">
        <li v-for="speaker in props.draft.speakers" :key="speaker.key">
          <p class="font-bold text-text">
            {{ speaker.first_name }} {{ speaker.last_name }}
            <span class="font-normal text-text-muted">
              — {{ t(`proposal.form.step-speakers.roles.${speaker.role}.label`) }}
            </span>
          </p>
          <p class="text-text-muted">
            <span v-if="speaker.job_title">{{ speaker.job_title }}</span>
            <span v-if="speaker.job_title && speaker.organization_name"> · </span>
            <span v-if="speaker.organization_name">{{ speaker.organization_name }}</span>
          </p>
        </li>
      </ul>
      <p v-else class="px-4 py-4 text-sm text-text-muted">
        {{ t('proposal.form.step-review.noSpeakers') }}
      </p>
    </section>

    <!-- CRÉNEAU -->
    <section class="rounded-lg border border-border">
      <div class="flex items-center justify-between gap-3 border-b border-border px-4 py-3">
        <h3 class="font-bold text-text">{{ props.stepLabels.schedule }}</h3>
        <UiButton
          variant="ghost"
          size="sm"
          icon="edit"
          :label="t('proposal.form.step-review.edit')"
          @click="emit('goTo', 'schedule')"
        />
      </div>
      <dl class="grid gap-3 px-4 py-4 text-sm sm:grid-cols-2">
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.start') }}</dt>
          <dd class="text-text">
            {{ startLabel }}
            <span class="block text-text-muted">
              {{ t('common.datetime.zoneOf', { zone: props.edition.city ?? props.edition.timezone }) }}
            </span>
          </dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.duration') }}</dt>
          <dd class="text-text">{{ durationLabel }}</dd>
        </div>
        <div>
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.sessions') }}</dt>
          <dd class="text-text">{{ props.draft.requested_sessions }}</dd>
        </div>
        <div v-if="props.draft.scheduling_constraints">
          <dt class="text-text-subtle">{{ t('proposal.form.step-review.fields.constraints') }}</dt>
          <dd class="max-w-(--measure) whitespace-pre-line text-text-secondary">
            {{ props.draft.scheduling_constraints }}
          </dd>
        </div>
      </dl>
    </section>

    <!-- CE QUI SE PASSE APRÈS L'ENVOI, dit AVANT de cliquer. -->
    <UiAlert intent="info" :title="t('proposal.form.step-review.next.title')">
      <ul class="grid list-disc gap-1 ps-5">
        <li>
          {{ t('proposal.form.step-review.next.reviews', { count: props.call.required_reviews }, props.call.required_reviews) }}
        </li>
        <li>{{ t('proposal.form.step-review.next.editable') }}</li>
        <li>{{ t('proposal.form.step-review.next.notification') }}</li>
      </ul>
    </UiAlert>

    <p v-if="issuesOfStep('review').length === 0" class="sr-only">
      {{ t('proposal.form.step-review.ready') }}
    </p>
  </div>
</template>
