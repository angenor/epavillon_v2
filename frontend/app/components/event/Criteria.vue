<script setup lang="ts">
import type { CallForProposals, ReviewCriterion } from '~/types/event/call'
import type { TableColumn } from '~/types/ui'

/**
 * LES CRITÈRES D'ÉVALUATION, PUBLIÉS.
 *
 * « Une organisation doit savoir sur quoi elle sera jugée. » La v1 se contentait
 * d'une note libre sur 20 : impossible de dire à une organisation pourquoi son
 * dossier n'avait pas été retenu, ni de le lui montrer avant qu'elle écrive. La
 * grille vit maintenant en base, par appel (`event.review_criteria`), et rien
 * n'empêche de la publier — c'est même ce qui rend l'évaluation opposable.
 *
 * DEUX CHOSES QUE CETTE SECTION NE DOIT PAS TAIRE :
 *
 *   · LA PONDÉRATION. « Pertinence » compte deux fois « Innovation » ; l'ignorer
 *     conduit une organisation à soigner le mauvais paragraphe.
 *   · LE CRITÈRE ÉLIMINATOIRE (`is_knockout`). Une note nulle sur la pertinence
 *     disqualifie quelle que soit la moyenne. Le dire APRÈS coup serait un
 *     piège ; il est signalé ici, et sur la fiche d'évaluation (A8) avant que le
 *     comité pose la note.
 *
 * LE TOTAL EST CALCULÉ, JAMAIS ÉCRIT EN DUR — c'est `event.max_weighted_score()`
 * côté base. Changer une pondération au back-office doit suffire à mettre à jour
 * ce que le public lit.
 */

interface Props {
  criteria: ReviewCriterion[]
  call: CallForProposals | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

const sorted = computed(() => [...props.criteria].sort((a, b) => a.sort_order - b.sort_order))

/** Somme de `max_score × weight` — l'équivalent de `event.max_weighted_score()`. */
const maxTotal = computed(() =>
  sorted.value.reduce((total, criterion) => total + criterion.max_score * criterion.weight, 0),
)

const columns = computed<TableColumn[]>(() => [
  { key: 'label', label: t('event.public.criteria.columns.criterion') },
  { key: 'max_score', label: t('event.public.criteria.columns.maxScore'), numeric: true, align: 'end', width: '7rem' },
  { key: 'weight', label: t('event.public.criteria.columns.weight'), numeric: true, align: 'end', width: '7rem' },
  {
    key: 'total',
    label: t('event.public.criteria.columns.total'),
    numeric: true,
    align: 'end',
    width: '8rem',
    hideOnMobile: true,
  },
])
</script>

<template>
  <section id="criteres" class="scroll-mt-24" aria-labelledby="criteres-titre">
    <h2 id="criteres-titre" class="font-display text-xl">{{ t('event.public.criteria.title') }}</h2>
    <p class="mt-1 text-sm text-text-muted" :style="{ maxWidth: 'var(--measure)' }">
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
      <UiTable
        class="mt-5"
        :columns="columns"
        :rows="sorted"
        row-key="id"
        :caption="t('event.public.criteria.caption', { total: maxTotal })"
        :hoverable="false"
      >
        <template #cell-label="{ row }">
          <div class="min-w-0">
            <p class="font-semibold text-text">
              {{ tr(row.label) }}
              <UiBadge v-if="row.is_knockout" intent="danger" size="sm" class="ml-2 align-middle">
                {{ t('event.public.criteria.knockout') }}
              </UiBadge>
            </p>
            <p v-if="row.description" class="mt-1 text-sm text-text-muted">{{ tr(row.description) }}</p>
          </div>
        </template>
        <template #cell-max_score="{ row }">{{ row.max_score }}</template>
        <template #cell-weight="{ row }">× {{ row.weight }}</template>
        <template #cell-total="{ row }">{{ row.max_score * row.weight }}</template>
      </UiTable>

      <p class="mt-3 text-sm text-text-secondary">
        {{ t('event.public.criteria.maxTotal', { total: maxTotal }) }}
        <span v-if="props.call?.required_reviews">
          · {{ t('event.public.criteria.reviews', { count: props.call.required_reviews }) }}
        </span>
      </p>
      <p v-if="sorted.some((criterion) => criterion.is_knockout)" class="mt-2 text-sm text-text-secondary">
        {{ t('event.public.criteria.knockoutNote') }}
      </p>
    </template>
  </section>
</template>
