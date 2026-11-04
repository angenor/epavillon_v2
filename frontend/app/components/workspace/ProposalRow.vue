<script setup lang="ts">
import type { ProposalTracking } from '~/types/organization-workspace'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { Intent } from '~/types/ui'
import type { ProposalActionIndex } from '~/utils/workspace-editions'

/**
 * UN DOSSIER, UNE LIGNE : couverture, titre condensé, état, prochaine étape,
 * « Accéder ». Une séance programmée se lit à son heure, en grand (04/11).
 *
 * La frise de l'avancement reste sur la fiche du dossier (28/10) : cinq jalons
 * par dossier noyaient la seule chose cherchée ici, qui doit agir et quand.
 */

interface Props {
  tracking: ProposalTracking
  index: ProposalActionIndex
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dayLong, time, zoneLabel } = useDateTime()
const localePath = useLocalePath()

// « En évaluation » garde le cyan de l'information (arbitré le 04/11) ; « Corrections demandées » est jaune.
const STATUS_TONES: Record<ProposalStatus, Intent> = {
  draft: 'neutral',
  submitted: 'info',
  under_review: 'info',
  changes_requested: 'warning',
  accepted: 'success',
  rejected: 'danger',
  withdrawn: 'neutral',
  cancelled: 'neutral',
}

const CLOSED: ProposalStatus[] = ['rejected', 'withdrawn', 'cancelled']

const target = computed(() => localePath(`/mon-organisation/dossiers/${props.tracking.proposal.id}`))
const closed = computed(() => CLOSED.includes(props.tracking.proposal.status))

type NextStep = { kind: 'text'; text: string; mine: boolean } | { kind: 'session'; time: string; when: string }

const nextStep = computed<NextStep | null>(() => {
  const { proposal, edition, open_change_requests, sessions } = props.tracking
  const key = 'organization.workspace.proposals.next'
  const reports = props.index.missingReports.get(proposal.id) ?? 0
  const text = (value: string, mine = false): NextStep => ({ kind: 'text', text: value, mine })

  switch (proposal.status) {
    case 'changes_requested':
      return text(t(`${key}.changes`, open_change_requests), true)
    case 'draft':
      return props.index.pending.has(proposal.id) ? text(t(`${key}.draft`), true) : text(t(`${key}.draftClosed`))
    case 'submitted':
    case 'under_review':
      return text(t(`${key}.review`))
    case 'accepted': {
      if (reports > 0) return text(t(`${key}.report`, reports), true)
      const now = Date.now()
      const upcoming = sessions
        .map((tracked) => tracked.session)
        .filter((session) => session.status !== 'cancelled' && Date.parse(session.ends_at) > now)
        .sort((a, b) => a.starts_at.localeCompare(b.starts_at))[0]
      if (upcoming) {
        return {
          kind: 'session',
          time: time(upcoming.starts_at, edition.timezone),
          when: t(`${key}.sessionWhen`, {
            day: dayLong(upcoming.starts_at, edition.timezone),
            zone: zoneLabel(edition.timezone, edition.city ?? undefined),
          }),
        }
      }
      return sessions.length > 0 ? text(t(`${key}.held`)) : text(t(`${key}.scheduling`))
    }
    default:
      return null
  }
})
</script>

<template>
  <div
    class="grid grid-cols-[96px_minmax(0,1fr)] items-start gap-x-4 gap-y-3 py-5 lg:grid-cols-[152px_minmax(0,1fr)_236px_auto] lg:items-center lg:gap-5.5"
  >
    <div :class="closed ? 'opacity-75 grayscale' : ''" aria-hidden="true">
      <UiImage
        v-if="props.tracking.cover"
        :image="props.tracking.cover"
        ratio="16 / 9"
        sizes="(min-width: 1024px) 152px, 96px"
        rounded="rounded-md"
      />
      <span
        v-else
        class="grid aspect-video place-items-center rounded-md border border-dashed border-border bg-surface-sunken text-text-subtle"
      >
        <UiIcon name="document" size="1.4rem" />
      </span>
    </div>

    <div class="min-w-0">
      <h3 class="font-condensed text-[1.25rem] leading-[1.12] font-semibold tracking-[0.005em] text-pretty text-heading lg:text-[1.45rem]">
        <NuxtLink
          :to="target"
          class="text-heading no-underline decoration-2 underline-offset-4 hover:text-accent hover:underline"
        >
          {{ tr(props.tracking.proposal.title) }}
        </NuxtLink>
      </h3>
      <p class="mt-1.5 text-[0.8rem] text-text-subtle tabular-nums">
        {{ t('organization.workspace.proposals.reference', { code: props.tracking.proposal.reference_code }) }}
      </p>
      <p v-if="props.tracking.unread_comments > 0" class="mt-1.5 flex items-center gap-1.5 text-[0.85rem] font-bold text-accent">
        <UiIcon name="chat" size="1rem" />
        {{ t('organization.workspace.proposals.unread', props.tracking.unread_comments) }}
      </p>
    </div>

    <div class="col-span-2 flex flex-wrap items-center gap-x-3.5 gap-y-2 lg:col-span-1 lg:grid lg:justify-items-start lg:gap-2">
      <UiBadge
        :intent="STATUS_TONES[props.tracking.proposal.status]"
        :label="t(`organization.workspace.proposals.timeline.${props.tracking.proposal.status}`)"
      />
      <p v-if="nextStep?.kind === 'session'" class="flex items-baseline gap-2 text-sm text-text-secondary">
        <b class="font-condensed text-[1.9rem] leading-[0.9] font-bold text-heading tabular-nums">{{ nextStep.time }}</b>
        <span class="first-letter:uppercase">{{ nextStep.when }}</span>
      </p>
      <p v-else-if="nextStep" class="text-sm leading-snug" :class="nextStep.mine ? 'font-bold text-text' : 'text-text-muted'">
        {{ nextStep.text }}
      </p>
    </div>

    <UiButton variant="secondary" class="col-span-2 w-full lg:col-span-1 lg:w-auto" :to="target" icon-trailing="arrow-right">
      {{ t('organization.workspace.proposals.open') }}
    </UiButton>
  </div>
</template>
