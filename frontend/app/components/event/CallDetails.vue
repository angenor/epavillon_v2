<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'

/**
 * « Conditions de l'appel » : ce qu'on vient VÉRIFIER une fois décidé. Le
 * billet de la colonne de droite porte l'état, l'échéance et le dépôt ; ici la
 * description de l'appel, ses trois conditions chiffrées (dossiers par
 * organisation, intervenants, durée) et les consignes.
 *
 * L'ancre reste `appel-a-propositions` : le pied de page de chaque écran y renvoie.
 */

interface Props {
  /** `null` quand l'édition n'ouvre aucun appel : la section n'est pas rendue. */
  call: CallForProposals | null
  edition: EventEdition
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

const conditions = computed(() => {
  const call = props.call
  if (!call) return []
  return [
    {
      key: 'proposals',
      value: call.max_proposals_per_organization ?? t('event.public.call.conditions.unlimited'),
      label:
        call.max_proposals_per_organization === null
          ? t('event.public.call.conditions.proposalsUnlimited')
          : t('event.public.call.conditions.proposals', call.max_proposals_per_organization),
    },
    {
      key: 'speakers',
      value:
        call.min_speakers === call.max_speakers
          ? String(call.max_speakers)
          : t('event.public.call.conditions.range', { min: call.min_speakers, max: call.max_speakers }),
      label: t('event.public.call.conditions.speakers', call.max_speakers),
    },
    {
      key: 'duration',
      value: t('event.public.call.conditions.minutes', { count: call.default_duration_minutes }),
      label: t('event.public.call.conditions.duration'),
    },
  ]
})
</script>

<template>
  <section
    v-if="props.call"
    id="appel-a-propositions"
    class="scroll-mt-28"
    aria-labelledby="appel-titre"
  >
    <h2 id="appel-titre" class="heading-condensed text-[clamp(1.9rem,3.6vw,2.5rem)] text-heading">
      {{ t('event.public.call.conditions.title') }}
    </h2>
    <p v-if="props.call.description" class="mt-2 max-w-[64ch] text-text-secondary">
      {{ tr(props.call.description) }}
    </p>

    <dl class="mt-4 grid gap-3 sm:grid-cols-3">
      <div
        v-for="condition in conditions"
        :key="condition.key"
        class="flex flex-col-reverse justify-end rounded-lg border border-border-subtle bg-surface-raised p-3.5"
      >
        <dt class="mt-1 text-[0.82rem] text-text-secondary">{{ condition.label }}</dt>
        <dd class="font-condensed text-[1.8rem] leading-none font-bold text-heading uppercase tabular-nums">
          {{ condition.value }}
        </dd>
      </div>
    </dl>

    <a
      v-if="props.call.guidelines_url"
      :href="props.call.guidelines_url"
      target="_blank"
      rel="noopener noreferrer"
      class="mt-4 inline-flex min-h-11 items-center gap-1.5 text-sm font-semibold text-accent"
    >
      {{ t('event.public.call.guidelines') }}
      <UiIcon name="external-link" size="0.9rem" />
      <span class="sr-only">{{ t('common.a11y.externalLink') }}</span>
    </a>
  </section>
</template>
