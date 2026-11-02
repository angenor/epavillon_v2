<script setup lang="ts">
import type { JournalSummary } from '~/types/admin-journal'
import type { RouteLocationRaw } from 'vue-router'

/**
 * Les quatre chiffres de l'en-tête. Chacun est un lien vers la liste qui le
 * détaille : un chiffre qu'on ne peut pas ouvrir oblige à refaire le filtre.
 *
 * Le quatrième compte ce qui attend une relance (travaux et événements en
 * échec, lus en direct) plutôt que les essais échoués des 24 heures : une tâche
 * qui a réussi au deuxième essai n'a rien à faire dans l'onglet qu'il ouvre.
 */

defineProps<{ summary: JournalSummary | null; loading?: boolean }>()

const { t } = useI18n()

interface Figure {
  key: 'http_error' | 'http_refused' | 'http_slow' | 'failing'
  value: (summary: JournalSummary) => number
  to: RouteLocationRaw
  tone: string
}

const FIGURES: Figure[] = [
  { key: 'http_error', value: (s) => s.http_error, to: { query: { onglet: 'erreurs', gravite: 'panne' } }, tone: 'text-danger' },
  { key: 'http_refused', value: (s) => s.http_refused, to: { query: { onglet: 'erreurs', gravite: 'refus' } }, tone: 'text-warning' },
  { key: 'http_slow', value: (s) => s.http_slow, to: { query: { onglet: 'erreurs', gravite: 'lent' } }, tone: 'text-text' },
  { key: 'failing', value: (s) => s.failing_jobs + s.failing_events, to: { query: { onglet: 'taches' } }, tone: 'text-danger' },
]
</script>

<template>
  <ul class="grid grid-cols-2 gap-3 lg:grid-cols-4" :aria-label="t('admin.journal.figures.label')">
    <li v-for="figure in FIGURES" :key="figure.key">
      <NuxtLink
        :to="figure.to"
        class="flex min-h-(--target-min) h-full flex-col rounded-lg border border-border bg-surface-raised px-4 py-3 text-text no-underline transition-colors duration-(--duration-fast) hover:bg-surface-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus"
      >
        <UiSkeletonLoader v-if="loading || !summary" width="3rem" height="2rem" />
        <span
          v-else
          class="text-2xl font-semibold tabular-nums"
          :class="figure.value(summary) > 0 ? figure.tone : 'text-text-muted'"
        >
          {{ figure.value(summary) }}
        </span>
        <span class="mt-1 text-sm text-text-secondary">
          {{ t(`admin.journal.figures.${figure.key}`, summary ? figure.value(summary) : 2) }}
        </span>
      </NuxtLink>
    </li>
  </ul>
</template>
