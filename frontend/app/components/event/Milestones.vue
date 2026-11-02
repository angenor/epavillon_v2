<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type { TimelineStep } from '~/types/ui'

/**
 * FRISE DES JALONS: ouverture de l'appel, clôture, annonce des résultats,
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
 * et les journées spéciales, c'est-à-dire exactement au moment où l'on se
 * demande s'il reste du temps. Les quatre jalons viennent tous du
 * modèle, aucun n'est inventé :
 *
 *   ouverture   `calls_for_proposals.opens_at`
 *   clôture     `event.effective_deadline()`, prolongation comprise
 *   résultats   `calls_for_proposals.results_expected_at`
 *   événement   `events.starts_at`
 *
 * L'ÉTAT DE CHAQUE ÉTAPE VIENT DES FAITS (`utils/edition-milestones.ts`) et
 * est passé explicitement à la frise : `UiStatusTimeline` déduirait l'étape
 * courante de la dernière étape DATÉE, ce qui convient au journal d'un dossier
 * mais pas à des dates connues d'avance.
 */

interface Props {
  edition: EventEdition
  call: CallForProposals | null
}

const props = defineProps<Props>()

const { t } = useI18n()

/** Instant d'évaluation, figé au rendu : serveur et client doivent s'accorder. */
const now = Date.now()

const steps = computed<TimelineStep[]>(() =>
  editionMilestones(props.edition, props.call, now).map((step) => ({
    value: step.key,
    label: t(`event.public.milestones.${step.key}`),
    at: step.at,
    dateOnly: step.dateOnly,
    state: step.state,
  })),
)
</script>

<template>
  <section class="milestones rounded-lg border border-border-subtle bg-surface-raised px-5.5 py-5" aria-labelledby="jalons-titre">
    <h2 id="jalons-titre" class="heading-condensed text-[1.45rem] text-heading">{{ t('event.public.milestones.title') }}</h2>
    <p class="mt-1 text-sm text-text-muted">{{ t('event.public.milestones.description') }}</p>

    <UiStatusTimeline
      class="mt-4"
      :steps="steps"
      :label="t('event.public.milestones.title')"
      :timezone="props.edition.timezone"
      :zone-label="props.edition.city ?? undefined"
      orientation="vertical"
    />
  </section>
</template>

<style scoped>
/* Le titre de chaque étape prend l'habit condensé de la page. La frise reste
   celle de `UiStatusTimeline` (états, traits, dates) : la recopier pour une
   police en ferait deux à corriger. */
.milestones :deep(ol > li > div > p:first-child) {
  font-family: var(--font-condensed);
  font-size: 1.15rem;
  font-weight: 700;
  line-height: 1.1;
  letter-spacing: 0.02em;
  text-transform: uppercase;
}
.milestones :deep(ol > li > div > p:first-child + p) {
  font-size: 0.8rem;
}
</style>
