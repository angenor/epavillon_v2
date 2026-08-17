<script setup lang="ts">
import type { ProposalTracking } from '~/types/organization-workspace'
import type { Intent } from '~/types/ui'
import type { ProposalStatus } from '~/types/programme/proposal'

/**
 * UN DOSSIER DANS LA LISTE — numéro, titre, état, frise, et ce qui l'attend.
 *
 * LA FRISE EST HORIZONTALE ICI, verticale sur la fiche. C'est le même composant
 * et les mêmes étapes (`buildProposalTimeline`) : ce qui change est la place
 * disponible, pas la vérité. Sous 640 px, `UiStatusTimeline` retombe d'elle-même
 * en colonne — une frise à cinq jalons datés ne tient pas sur un téléphone.
 *
 * « CORRECTIONS DEMANDÉES » DOIT SAUTER AUX YEUX, et deux choses s'en chargent
 * ensemble : le liseré rouge à gauche de la carte, et le nombre de points à
 * traiter écrit en toutes lettres. La couleur seule ne dit jamais un état —
 * c'est une règle du guide, et elle vaut ici plus qu'ailleurs : c'est la seule
 * ligne de l'écran qui appelle une action dans la journée.
 */

interface Props {
  tracking: ProposalTracking
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()

/**
 * L'ÉTAT DU DOSSIER SUIT LA RÈGLE DE COULEUR DU GUIDE, sans exception :
 * cyan pour ce qui est en cours d'instruction, vert pour ce qui est acquis,
 * jaune pour ce qui demande une action, rouge pour ce qui est écarté, gris pour
 * ce qui est clos ou pas encore parti.
 *
 * `changes_requested` est JAUNE et non rouge : ce n'est pas un échec, c'est un
 * dossier vivant qui attend son auteur. Le rouge est réservé à ce sur quoi on ne
 * peut plus rien — et le confondre ferait renoncer des gens qui pouvaient encore
 * agir.
 */
const STATUS_TONES: Record<ProposalStatus, Intent> = {
  draft: 'neutral',
  submitted: 'info',
  under_review: 'info',
  changes_requested: 'warning',
  accepted: 'success',
  rejected: 'danger',
  withdrawn: 'neutral',
  cancelled: 'danger',
}

const steps = computed(() =>
  buildProposalTimeline(props.tracking, {
    draft: t('organization.workspace.proposals.timeline.draft'),
    submitted: t('organization.workspace.proposals.timeline.submitted'),
    under_review: t('organization.workspace.proposals.timeline.under_review'),
    changes_requested: t('organization.workspace.proposals.timeline.changes_requested'),
    decision: t('organization.workspace.proposals.timeline.decision'),
    accepted: t('organization.workspace.proposals.timeline.accepted'),
    rejected: t('organization.workspace.proposals.timeline.rejected'),
    withdrawn: t('organization.workspace.proposals.timeline.withdrawn'),
    cancelled: t('organization.workspace.proposals.timeline.cancelled'),
    scheduled: t('organization.workspace.proposals.timeline.scheduled'),
  }),
)

/** Inscrits sur l'ensemble des séances du dossier. */
const registered = computed(() =>
  props.tracking.sessions.reduce((total, session) => total + session.registered_count, 0),
)

const target = computed(() => localePath(`/mon-organisation/dossiers/${props.tracking.proposal.id}`))

const needsAttention = computed(() => props.tracking.proposal.status === 'changes_requested')
</script>

<template>
  <article
    class="rounded-lg border border-border bg-surface-raised transition-colors duration-(--duration-fast) hover:border-border-strong"
    :class="needsAttention ? 'border-l-(length:--border-thick) border-l-danger' : ''"
  >
    <div class="flex flex-col gap-4 p-5">
      <div class="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
        <div class="min-w-0">
          <p class="font-mono text-xs tracking-wide text-text-subtle">
            {{ t('organization.workspace.proposals.reference', { code: props.tracking.proposal.reference_code }) }}
          </p>
          <h3 class="mt-1 text-lg leading-snug font-semibold">
            <NuxtLink :to="target" class="text-heading no-underline hover:text-accent">
              {{ tr(props.tracking.proposal.title) }}
            </NuxtLink>
          </h3>
          <p class="mt-1 text-sm text-text-subtle">{{ tr(props.tracking.edition.title) }}</p>
        </div>

        <UiBadge
          :intent="STATUS_TONES[props.tracking.proposal.status]"
          :label="t(`organization.workspace.proposals.timeline.${props.tracking.proposal.status}`)"
          solid
        />
      </div>

      <!-- Le nombre de points à traiter, en toutes lettres et en rouge : c'est
           la seule information de la carte qui appelle une action aujourd'hui. -->
      <p
        v-if="needsAttention"
        class="flex items-center gap-2 rounded-md border border-danger-border bg-danger-surface px-3 py-2 text-sm font-semibold text-danger"
      >
        <UiIcon name="warning" size="1rem" class="shrink-0" />
        {{ t('organization.workspace.proposals.changeRequests', props.tracking.open_change_requests) }}
      </p>

      <!-- Frise RÉSUMÉE : les motifs de décision sont masqués ici et lus sur la
           fiche. Rendus dans une colonne de frise horizontale, ils débordaient
           sur leurs voisines et rendaient l'ensemble illisible. -->
      <UiStatusTimeline
        :steps="steps"
        orientation="horizontal"
        hide-details
        :timezone="props.tracking.edition.timezone"
        :label="t('organization.workspace.proposals.timeline.label')"
      />

      <div class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-t border-border-subtle pt-3">
        <p v-if="props.tracking.sessions.length > 0" class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-text-muted">
          <span class="inline-flex items-center gap-1.5">
            <UiIcon name="calendar" size="1rem" class="text-text-subtle" />
            {{ t('organization.workspace.proposals.sessions', props.tracking.sessions.length) }}
          </span>
          <span class="inline-flex items-center gap-1.5">
            <UiIcon name="users" size="1rem" class="text-text-subtle" />
            {{ t('organization.workspace.proposals.registered', registered) }}
          </span>
        </p>
        <span v-else />

        <UiButton variant="secondary" size="sm" :to="target" icon-trailing="chevron-right">
          {{ t('organization.workspace.proposals.openThis') }}
        </UiButton>
      </div>
    </div>
  </article>
</template>
