<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'

/**
 * L'APPEL EN COURS, MIS EN AVANT — c'est ce que le prompt demande de montrer à
 * qui n'a encore rien déposé.
 *
 * C'EST LA SEULE CHOSE UTILE À AFFICHER DANS L'ÉTAT VIDE. Une organisation qui
 * arrive dans son espace sans dossier n'a pas besoin qu'on lui explique ce
 * qu'elle ne voit pas : elle a besoin de savoir où déposer et jusqu'à quand.
 *
 * L'ÉCHÉANCE AFFICHÉE EST L'ÉCHÉANCE EFFECTIVE — `event.effective_deadline()` :
 * la prolongation si elle existe, la clôture sinon. Afficher `closes_at` quand
 * un appel a été prolongé ferait renoncer des gens qui avaient encore le temps.
 *
 * LE REBOURS SE TAIT QUAND IL N'A PLUS DE SENS. Passé l'échéance, l'encart ne
 * compte plus à rebours : il annonce qu'aucun appel n'est ouvert.
 */

interface Props {
  call: CallForProposals | null
  edition: EventEdition | null
}

const props = defineProps<Props>()

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
  <UiCard v-if="isOpen && props.call" class="border-accent-border bg-accent-surface">
    <p class="text-xs font-bold tracking-wide text-accent uppercase">
      {{ t('organization.workspace.openCall.eyebrow') }}
    </p>

    <h3 class="mt-2 text-lg leading-snug font-semibold text-heading">
      {{ props.edition ? tr(props.edition.title) : tr(props.call.title) }}
    </h3>

    <p class="mt-2 text-sm text-text-secondary">
      {{ t('organization.workspace.openCall.deadline', { date: date(deadline, timezone) }) }}
    </p>

    <p v-if="countdown" class="mt-1 text-sm font-semibold text-accent">
      {{
        countdown.days > 0
          ? t('organization.workspace.openCall.remaining', countdown.days)
          : t('organization.workspace.openCall.lastDay')
      }}
    </p>

    <div class="mt-4 flex flex-wrap items-center gap-3">
      <UiButton variant="primary" :to="localePath('/deposer-une-proposition')" icon="plus">
        {{ t('organization.workspace.openCall.action') }}
      </UiButton>
      <UiButton
        v-if="props.edition"
        variant="link"
        :to="localePath(`/evenements/${props.edition.slug}`)"
      >
        {{ t('organization.workspace.openCall.criteria') }}
      </UiButton>
    </div>
  </UiCard>

  <UiCard v-else sunken>
    <h3 class="font-semibold text-text">{{ t('organization.workspace.openCall.closed.title') }}</h3>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('organization.workspace.openCall.closed.description') }}
    </p>
  </UiCard>
</template>
