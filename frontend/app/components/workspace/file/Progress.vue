<script setup lang="ts">
import type { ProposalStatus } from '~/types/programme/proposal'
import type { TimeZoneName } from '~/types/shared'
import type { TimelineStep } from '~/types/ui'
import type { OrganizationEditBlockedReason } from '~/utils/proposal-edit'
import type { ProposalCancellation } from '~/utils/proposal-cancellation'

/**
 * L'AVANCEMENT D'UN DOSSIER, la carte qui remonte sur le bandeau (04/11) : la
 * frise, puis ce qu'elle entraîne. La décision rendue, ou l'annulation (qui,
 * quand, pourquoi, qui a été prévenu), et ce que l'organisation peut encore faire.
 */

interface Props {
  steps: TimelineStep[]
  timezone: TimeZoneName
  referenceCode: string
  status: ProposalStatus
  decidedAt: string | null
  decisionMessage: string
  cancellation: ProposalCancellation | null
  /** Nom de qui a annulé, quand l'historique ne le donne pas. */
  cancelledByFallback: string
  isDraft: boolean
  editBlockedReason: OrganizationEditBlockedReason | null
  draftTo: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { date } = useDateTime()

const cancelledBy = computed(() => props.cancellation?.actorLabel ?? props.cancelledByFallback)
// Le site ne sait pas si l'organisation ou l'IFDD a annulé : seule la personne qui a annulé lit que l'équipe a été prévenue.
const auth = useAuthStore()
const cancelledByViewer = computed(() => Boolean(props.cancellation?.actorId) && props.cancellation?.actorId === auth.person?.id)
</script>

<template>
  <WorkspacePanel
    heading-id="frise-title"
    :title="t('organization.workspace.proposal.tracking.progress')"
    :meta="t('organization.workspace.proposal.title', { code: props.referenceCode })"
  >
    <UiStatusTimeline
      :steps="props.steps"
      orientation="horizontal"
      condensed
      hide-details
      :timezone="props.timezone"
      :label="t('organization.workspace.proposal.tracking.timelineLabel')"
    />

    <template v-if="props.status === 'cancelled' && props.cancellation">
      <UiAlert class="mt-5" intent="warning" :title="t('organization.workspace.proposal.cancel.done.title')">
        <p>
          {{
            cancelledBy
              ? t('organization.workspace.proposal.cancel.done.byOn', { date: date(props.cancellation.at, props.timezone), actor: cancelledBy })
              : t('organization.workspace.proposal.cancel.done.on', { date: date(props.cancellation.at, props.timezone) })
          }}
          <template v-if="props.cancellation.reason">
            {{ t('organization.workspace.proposal.cancel.done.reason', { reason: props.cancellation.reason }) }}
          </template>
        </p>
        <p v-if="props.cancellation.afterAcceptance" class="mt-1">
          {{
            cancelledByViewer
              ? t('organization.workspace.proposal.cancel.done.mailsYou')
              : t('organization.workspace.proposal.cancel.done.mailsRegistrants')
          }}
        </p>
      </UiAlert>
      <p class="mt-5 flex items-start gap-2 text-[0.9rem] text-text-muted">
        <UiIcon name="lock" size="1.05rem" class="mt-0.5 shrink-0 text-text-subtle" />
        {{ t('organization.workspace.proposal.cancel.done.closed') }}
      </p>
    </template>

    <template v-else>
      <UiAlert
        v-if="props.decidedAt"
        class="mt-5"
        :intent="props.status === 'accepted' ? 'success' : 'warning'"
        :title="t(`organization.workspace.proposal.tracking.decision.${props.status}`)"
        :message="props.decisionMessage"
      />

      <div
        v-if="props.isDraft && props.editBlockedReason === null"
        class="mt-5 flex flex-wrap items-center justify-between gap-x-5 gap-y-3 rounded-md bg-surface-sunken p-4"
      >
        <p class="max-w-[52ch] text-text-secondary">{{ t('organization.workspace.proposal.draftNotice') }}</p>
        <UiButton variant="primary" class="shrink-0" :to="props.draftTo" icon="edit">
          {{ t('organization.workspace.proposal.tracking.editDraft') }}
        </UiButton>
      </div>

      <p v-else-if="props.editBlockedReason" class="mt-5 flex items-start gap-2 text-[0.9rem] text-text-muted">
        <UiIcon name="lock" size="1.05rem" class="mt-0.5 shrink-0 text-text-subtle" />
        {{ t(`organization.workspace.proposal.tracking.editBlocked.${props.editBlockedReason}`) }}
      </p>
    </template>
  </WorkspacePanel>
</template>
