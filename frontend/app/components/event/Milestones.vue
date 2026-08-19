<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type { TimelineStep } from '~/types/ui'

/**
 * FRISE DES JALONS — ouverture de l'appel, clôture, annonce des résultats,
 * tenue de l'événement.
 *
 * Elle répond à « où en est-on ? » d'un seul coup d'œil, ce que quatre dates
 * dispersées dans la page ne font pas.
 *
 * ── ELLE EST PASSÉE À LA VERTICALE, EN COLONNE LATÉRALE (19/08) ─────────────
 *
 * Horizontale et pleine largeur, elle occupait cent pixels de haut pour quatre
 * dates, et écrasait ses libellés sous 640 px. Verticale dans la colonne de
 * droite, elle tient dans le champ de vision pendant qu'on lit la présentation
 * et les journées spéciales — c'est-à-dire exactement au moment où l'on se
 * demande s'il reste du temps. Les quatre jalons viennent tous du
 * modèle, aucun n'est inventé :
 *
 *   ouverture   `calls_for_proposals.opens_at`
 *   clôture     `event.effective_deadline()` — prolongation comprise
 *   résultats   `calls_for_proposals.results_expected_at`
 *   événement   `events.starts_at`
 *
 * L'ÉTAT DE CHAQUE ÉTAPE EST CALCULÉ ICI, et passé explicitement à la frise.
 * `UiStatusTimeline` sait déduire l'étape courante de la dernière étape DATÉE —
 * ce qui convient au journal d'un dossier, où une date signifie « c'est
 * arrivé ». Ici toutes les dates sont connues d'avance : la déduction placerait
 * le curseur sur la dernière, et la frise annoncerait la conférence terminée
 * avant qu'elle ait commencé.
 */

interface Props {
  edition: EventEdition
  call: CallForProposals | null
}

const props = defineProps<Props>()

const { t } = useI18n()

/** Instant d'évaluation, figé au rendu : serveur et client doivent s'accorder. */
const now = Date.now()

const steps = computed<TimelineStep[]>(() => {
  const raw: Array<{ value: string; at: string | null; dateOnly?: boolean }> = []

  if (props.call) {
    raw.push({ value: 'callOpens', at: props.call.opens_at })
    raw.push({ value: 'callCloses', at: effectiveDeadline(props.call) })
    raw.push({
      value: 'results',
      // `results_expected_at` est une DATE, sans heure : on la ramène à midi
      // UTC pour qu'aucune conversion de fuseau ne la fasse changer de jour, et
      // on demande à la frise de n'afficher QUE le jour — l'heure qu'on lirait
      // ne serait pas une saisie mais le résidu de cette conversion.
      at: props.call.results_expected_at ? `${props.call.results_expected_at}T12:00:00Z` : null,
      dateOnly: true,
    })
  }
  raw.push({ value: 'event', at: props.edition.starts_at })

  /** Première étape non franchie : c'est elle, et elle seule, qui est courante. */
  const currentIndex = raw.findIndex((step) => !step.at || Date.parse(step.at) > now)

  return raw.map((step, index) => ({
    value: step.value,
    label: t(`event.public.milestones.${step.value}`),
    at: step.at,
    dateOnly: step.dateOnly,
    state:
      index === currentIndex ? 'current' : index < currentIndex || currentIndex === -1 ? 'done' : 'upcoming',
  }))
})
</script>

<template>
  <section
    class="rounded-xl border border-border bg-surface-raised p-5 sm:p-6"
    aria-labelledby="jalons-titre"
  >
    <h2 id="jalons-titre" class="font-display text-xl">{{ t('event.public.milestones.title') }}</h2>
    <p class="mt-1 text-sm text-text-muted">{{ t('event.public.milestones.description') }}</p>

    <UiStatusTimeline
      class="mt-5"
      :steps="steps"
      :label="t('event.public.milestones.title')"
      :timezone="props.edition.timezone"
      :zone-label="props.edition.city ?? undefined"
      orientation="vertical"
    />
  </section>
</template>
