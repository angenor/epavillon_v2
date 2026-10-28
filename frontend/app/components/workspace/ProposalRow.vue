<script setup lang="ts">
import type { ProposalTracking } from '~/types/organization-workspace'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { Intent } from '~/types/ui'
import type { ProposalActionIndex } from '~/utils/workspace-editions'

/**
 * UN DOSSIER, UNE LIGNE : couverture, titre, état, prochaine étape, « Accéder ».
 *
 * La frise de l'avancement a quitté la liste (28/10) : cinq jalons par dossier
 * noyaient la seule chose cherchée ici, qui doit agir et quand. Elle reste sur
 * la fiche du dossier.
 */

interface Props {
  tracking: ProposalTracking
  index: ProposalActionIndex
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, timeWithZone } = useDateTime()
const localePath = useLocalePath()

// `changes_requested` est jaune : un dossier qui attend son auteur n'a pas échoué.
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

const target = computed(() => localePath(`/mon-organisation/dossiers/${props.tracking.proposal.id}`))

const nextStep = computed<{ text: string; mine: boolean } | null>(() => {
  const { proposal, edition, open_change_requests, sessions } = props.tracking
  const key = 'organization.workspace.proposals.next'
  const reports = props.index.missingReports.get(proposal.id) ?? 0

  switch (proposal.status) {
    case 'changes_requested':
      return { text: t(`${key}.changes`, open_change_requests), mine: true }
    case 'draft':
      return props.index.pending.has(proposal.id)
        ? { text: t(`${key}.draft`), mine: true }
        : { text: t(`${key}.draftClosed`), mine: false }
    case 'submitted':
    case 'under_review':
      return { text: t(`${key}.review`), mine: false }
    case 'accepted': {
      if (reports > 0) return { text: t(`${key}.report`, reports), mine: true }
      const now = Date.now()
      const upcoming = sessions
        .map((tracked) => tracked.session)
        .filter((session) => Date.parse(session.ends_at) > now)
        .sort((a, b) => a.starts_at.localeCompare(b.starts_at))[0]
      if (upcoming) {
        return {
          text: t(`${key}.session`, {
            date: date(upcoming.starts_at, edition.timezone),
            time: timeWithZone(upcoming.starts_at, edition.timezone, edition.city ?? undefined),
          }),
          mine: false,
        }
      }
      return sessions.length > 0 ? { text: t(`${key}.held`), mine: false } : { text: t(`${key}.scheduling`), mine: false }
    }
    default:
      return null
  }
})
</script>

<template>
  <div class="flex flex-col gap-3 px-4 py-3 sm:flex-row sm:items-center sm:gap-4">
    <div class="flex min-w-0 flex-1 items-center gap-4">
      <UiImage
        v-if="props.tracking.cover"
        :image="props.tracking.cover"
        ratio="16 / 9"
        sizes="96px"
        rounded="rounded-md"
        class="w-24 shrink-0"
        aria-hidden="true"
      />
      <span
        v-else
        aria-hidden="true"
        class="grid aspect-video w-24 shrink-0 place-items-center rounded-md bg-surface-sunken text-text-subtle"
      >
        <UiIcon name="document" size="1.25rem" />
      </span>

      <div class="min-w-0">
        <NuxtLink :to="target" class="font-semibold text-heading no-underline hover:text-accent">
          {{ tr(props.tracking.proposal.title) }}
        </NuxtLink>
        <p class="mt-0.5 font-mono text-xs text-text-subtle">
          {{ t('organization.workspace.proposals.reference', { code: props.tracking.proposal.reference_code }) }}
        </p>
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-x-4 gap-y-2 sm:contents">
      <UiBadge
        class="shrink-0 sm:w-44 sm:justify-center"
        :intent="STATUS_TONES[props.tracking.proposal.status]"
        :label="t(`organization.workspace.proposals.timeline.${props.tracking.proposal.status}`)"
        solid
      />
      <p class="text-sm sm:w-60 sm:shrink-0" :class="nextStep?.mine ? 'font-semibold text-text' : 'text-text-muted'">
        {{ nextStep?.text }}
      </p>
    </div>

    <UiButton variant="secondary" class="w-full shrink-0 sm:w-auto" :to="target" icon-trailing="arrow-right">
      {{ t('organization.workspace.proposals.open') }}
    </UiButton>
  </div>
</template>
