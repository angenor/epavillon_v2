<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'

/**
 * L'APPEL EN COURS, en billet à côté de « À faire » (04/11) : le rebours en
 * grand, l'échéance, et le geste. C'est aussi la seule chose utile de l'état
 * vide : qui n'a rien déposé a besoin de savoir où déposer et jusqu'à quand.
 *
 * L'échéance est l'échéance EFFECTIVE (`event.effective_deadline()`) : la
 * prolongation si elle existe. Passé l'échéance, le billet ne compte plus et
 * annonce qu'aucun appel n'est ouvert.
 */

interface Props {
  call: CallForProposals | null
  edition: EventEdition | null
  /** Rien quand l'appel est fermé : dans l'onglet d'une édition, un billet vide n'apprend rien. */
  hideWhenClosed?: boolean
}

const props = withDefaults(defineProps<Props>(), { hideWhenClosed: false })

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()
const localePath = useLocalePath()

const deadline = computed(() => (props.call ? effectiveDeadline(props.call) : null))
const countdown = useCountdown(deadline)

/** Fuseau de l'édition : une échéance de dépôt se lit à l'heure du pavillon. */
const timezone = computed(() => props.edition?.timezone ?? 'UTC')

const isOpen = computed(() => props.call !== null && !(countdown.value?.expired ?? true))
</script>

<template>
  <UiTicket v-if="isOpen && props.call" :label="t('organization.workspace.openCall.eyebrow')">
    <p class="font-condensed text-[1.05rem] font-bold tracking-[0.05em] text-text-secondary uppercase">
      {{ t('organization.workspace.openCall.eyebrow') }}
    </p>
    <p class="mt-1.5 font-semibold text-text">
      {{ props.edition ? tr(props.edition.title) : tr(props.call.title) }}
    </p>

    <p v-if="countdown && countdown.days > 0" class="mt-3.5 flex items-baseline gap-3">
      <b class="font-condensed text-[4rem] leading-[0.8] font-bold text-accent tabular-nums">{{ countdown.days }}</b>
      <span class="font-condensed text-[1.15rem] leading-[1.1] font-bold tracking-[0.04em] whitespace-pre-line text-heading uppercase">
        {{ t('organization.workspace.openCall.daysLeft', countdown.days) }}
      </span>
    </p>
    <p v-else-if="countdown" class="mt-3.5 font-condensed text-[1.6rem] leading-tight font-bold text-accent uppercase">
      {{ t('organization.workspace.openCall.lastDay') }}
    </p>

    <p class="mt-3 text-sm text-text-secondary">
      {{ t('organization.workspace.openCall.deadline', { date: date(deadline, timezone) }) }}
    </p>

    <div class="mt-4 grid gap-1">
      <UiButton variant="primary" block :to="localePath('/deposer-une-proposition')" icon="plus">
        {{ t('organization.workspace.openCall.action') }}
      </UiButton>
      <UiButton v-if="props.edition" variant="link" :to="localePath(`/evenements/${props.edition.slug}`)">
        {{
          props.call.uses_scoring_grid
            ? t('organization.workspace.openCall.criteria')
            : t('organization.workspace.openCall.edition')
        }}
      </UiButton>
    </div>
  </UiTicket>

  <UiTicket v-else-if="!props.hideWhenClosed" :label="t('organization.workspace.openCall.closed.title')" bar-class="bg-border">
    <p class="font-bold text-text">{{ t('organization.workspace.openCall.closed.title') }}</p>
    <p class="mt-1 text-sm text-text-muted">{{ t('organization.workspace.openCall.closed.description') }}</p>
  </UiTicket>
</template>
