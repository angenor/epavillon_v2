<script setup lang="ts">
import type { CallForProposals, ReviewCriterion } from '~/types/event/call'

/**
 * LES CRITÈRES D'ÉVALUATION, PUBLIÉS : une organisation doit savoir sur quoi
 * elle sera jugée. La grille vit en base, par appel (`event.review_criteria`).
 *
 * Deux choses que la section ne tait pas : la PONDÉRATION (« Pertinence »
 * compte deux fois « Innovation ») et le critère ÉLIMINATOIRE (`is_knockout`),
 * signalé avant le dépôt plutôt qu'après.
 *
 * Le total est calculé, jamais écrit : c'est `event.max_weighted_score()` côté
 * base. Changer une pondération au back-office suffit à mettre à jour la page.
 *
 * Un tableau dense, bien composé (ce public lit des documents de négociation),
 * écrit ici plutôt que par `UiTable` : en-tête bleu nuit en condensé, chiffres
 * en condensé gras, ligne de total. Sous `sm`, note et pondération se masquent,
 * les points restent.
 */

interface Props {
  criteria: ReviewCriterion[]
  call: CallForProposals | null
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()

const sorted = computed(() => [...props.criteria].sort((a, b) => a.sort_order - b.sort_order))

/** Somme de `max_score × weight`, l'équivalent de `event.max_weighted_score()`. */
const maxTotal = computed(() =>
  sorted.value.reduce((total, criterion) => total + criterion.max_score * criterion.weight, 0),
)

/** « 1,5 » en français, « 1.5 » en anglais. */
const number = (value: number) => new Intl.NumberFormat(locale.value, { maximumFractionDigits: 2 }).format(value)

const footnote = computed(() => {
  const reviews = props.call?.required_reviews ?? 0
  return [
    t('event.public.criteria.maxTotal', { total: number(maxTotal.value) }),
    reviews ? t('event.public.criteria.reviews', { count: reviews }, reviews) : '',
    sorted.value.some((criterion) => criterion.is_knockout) ? t('event.public.criteria.knockoutNote') : '',
  ]
    .filter(Boolean)
    .join(' ')
})

const headClass = 'px-2.5 py-3 font-condensed text-[0.95rem] font-semibold tracking-[0.06em] uppercase sm:px-4'
const numClass = 'px-2.5 py-3.5 text-right align-top font-condensed text-[1.35rem] leading-tight font-bold tabular-nums sm:px-4'
</script>

<template>
  <section id="criteres" class="scroll-mt-28" aria-labelledby="criteres-titre">
    <h2 id="criteres-titre" class="heading-condensed text-[clamp(1.9rem,3.6vw,2.5rem)] text-heading">
      {{ t('event.public.criteria.title') }}
    </h2>
    <p class="mt-2 max-w-[64ch] text-text-secondary">
      {{ t('event.public.criteria.description') }}
    </p>

    <UiEmptyState
      v-if="!sorted.length"
      class="mt-5"
      icon="document"
      :title="t('event.public.criteria.empty.title')"
      :description="t('event.public.criteria.empty.description')"
    />

    <template v-else>
      <div class="mt-4 overflow-hidden rounded-lg border border-border-subtle bg-surface-raised">
        <table class="w-full border-collapse text-[0.95rem]">
          <caption class="sr-only">{{ t('event.public.criteria.caption', { total: number(maxTotal) }) }}</caption>
          <thead>
            <tr class="bg-surface-inverse text-text-on-inverse">
              <th scope="col" class="text-left" :class="headClass">{{ t('event.public.criteria.columns.criterion') }}</th>
              <th scope="col" class="hidden text-right sm:table-cell" :class="headClass">
                {{ t('event.public.criteria.columns.maxScore') }}
              </th>
              <th scope="col" class="hidden text-right sm:table-cell" :class="headClass">
                {{ t('event.public.criteria.columns.weight') }}
              </th>
              <th scope="col" class="text-right" :class="headClass">{{ t('event.public.criteria.columns.total') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="criterion in sorted" :key="criterion.id" class="border-t border-border-subtle even:bg-surface-sunken">
              <th scope="row" class="px-2.5 py-3.5 text-left font-normal sm:px-4">
                <span class="font-semibold text-text">{{ tr(criterion.label) }}</span>
                <UiBadge v-if="criterion.is_knockout" intent="danger" size="sm" class="ml-2 align-middle">
                  {{ t('event.public.criteria.knockout') }}
                </UiBadge>
                <span v-if="criterion.description" class="mt-1 block text-sm text-text-muted">{{ tr(criterion.description) }}</span>
              </th>
              <td class="hidden text-heading sm:table-cell" :class="numClass">{{ number(criterion.max_score) }}</td>
              <td class="hidden text-heading sm:table-cell" :class="numClass">× {{ number(criterion.weight) }}</td>
              <td class="text-accent" :class="numClass">{{ number(criterion.max_score * criterion.weight) }}</td>
            </tr>
          </tbody>
          <tfoot>
            <tr class="border-t-2 border-text">
              <th scope="row" class="px-2.5 py-3.5 text-left font-bold sm:px-4">{{ t('event.public.criteria.columns.sum') }}</th>
              <td class="hidden sm:table-cell" />
              <td class="hidden sm:table-cell" />
              <td class="text-accent" :class="numClass">{{ number(maxTotal) }}</td>
            </tr>
          </tfoot>
        </table>
      </div>

      <p class="mt-3 max-w-[72ch] text-sm text-text-muted">{{ footnote }}</p>
    </template>
  </section>
</template>
