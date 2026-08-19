<script setup lang="ts">
import type { EditionPeriod } from '~/types/home'
import type { TabItem } from '~/types/ui'
import type { EditionStatsRow, PublicEditionRow } from '~/types/views'
import type { EventId } from '~/types/shared'

/**
 * L'HISTORIQUE DES ÉVÉNEMENTS — la troisième section de l'accueil.
 *
 * ── LE FILTRE NE COÛTE AUCUNE REQUÊTE ───────────────────────────────────────
 *
 * `api.home.screen()` rend l'historique COMPLET : changer d'onglet ne fait que
 * recomposer ce qu'on a déjà. C'est ce qui permet aux onglets d'annoncer leurs
 * décomptes — « Passés (2) » se lit AVANT d'y aller — et ces décomptes se
 * calculent sur l'ensemble non filtré, jamais sur la sélection courante.
 *
 * ── L'ÉTAT VIT DANS L'URL ───────────────────────────────────────────────────
 *
 * `?periode=a-venir`, comme partout dans ce projet. Le composant n'en décide
 * pas : il reçoit la période et signale qu'on en demande une autre. Un état de
 * filtre enfermé dans un composant ne se partage pas, ne se recharge pas et ne
 * revient pas au retour arrière.
 *
 * ── LE MILLÉSIME EST UN REPÈRE DE COLONNE ───────────────────────────────────
 *
 * Sur écran large, l'année se détache à gauche et suit le défilement : c'est ce
 * qui transforme une pile de cartes en une frise qu'on parcourt des yeux. En
 * dessous, elle redevient un simple intertitre — une colonne de 96 px sur un
 * téléphone ne serait qu'une perte de largeur.
 */

interface Props {
  /** L'historique COMPLET — le filtre est appliqué ici, pas en amont. */
  editions: PublicEditionRow[]
  /** `programme.v_edition_stats`, indexée par `event_id`. Clé absente = zéro. */
  stats: Record<EventId, EditionStatsRow>
  period: EditionPeriod
}

const props = defineProps<Props>()

const emit = defineEmits<{ 'update:period': [period: EditionPeriod] }>()

const { t } = useI18n()

const history = computed(() => buildEditionHistory(props.editions, props.stats, props.period))

const tabs = computed<TabItem[]>(() =>
  EDITION_PERIODS.map((period) => ({
    value: period,
    label: t(`home.history.tabs.${period}`),
    count: history.value.counts[period],
  })),
)

/** Une valeur d'onglet est une période — `UiTabs` ne connaît que des chaînes. */
function onTab(value: string): void {
  const period = EDITION_PERIODS.find((entry) => entry === value)
  if (period) emit('update:period', period)
}
</script>

<template>
  <section aria-labelledby="historique-titre">
    <div class="flex flex-wrap items-end justify-between gap-4 border-b border-border pb-4">
      <div class="min-w-0">
        <h2 id="historique-titre" class="font-display text-2xl">
          {{ t('home.history.title') }}
        </h2>
        <p class="mt-1 text-text-muted" :style="{ maxWidth: 'var(--measure)' }">
          {{ t('home.history.description') }}
        </p>
      </div>

      <!-- `min-w-0` : sans lui, la barre d'onglets est un élément flex dont la
           largeur minimale vaut celle de son contenu — quatre onglets et leurs
           compteurs élargissent alors la page entière au lieu de défiler dans
           leur propre `overflow-x-auto`. C'est la règle « le corps de page ne
           défile jamais horizontalement », et elle se joue ici, à 375 px. -->
      <UiTabs
        class="min-w-0 max-w-full"
        :model-value="props.period"
        :items="tabs"
        :label="t('home.history.filterLabel')"
        :panel-id="() => 'historique-panneau'"
        @update:model-value="onTab"
      />
    </div>

    <div id="historique-panneau" class="mt-6">
      <UiEmptyState
        v-if="!history.groups.length"
        icon="calendar"
        filtered
        :title="t('home.history.empty.title')"
        :description="t('home.history.empty.description')"
        :action-label="t('home.history.empty.action')"
        @action="emit('update:period', 'all')"
      />

      <div v-else class="flex flex-col gap-10">
        <div
          v-for="group in history.groups"
          :key="group.year"
          class="lg:grid lg:grid-cols-[5rem_minmax(0,1fr)] lg:gap-8"
        >
          <h3
            class="font-display text-xl tabular-nums text-text-subtle lg:sticky lg:top-24 lg:self-start"
          >
            {{ group.year }}
          </h3>

          <div class="mt-3 grid gap-4 sm:grid-cols-2 lg:mt-0 xl:grid-cols-3">
            <HomeEditionCard
              v-for="edition in group.editions"
              :key="edition.id"
              :edition="edition"
              :session-count="publishedSessionCount(history.stats, edition.id)"
            />
          </div>
        </div>
      </div>
    </div>
  </section>
</template>
