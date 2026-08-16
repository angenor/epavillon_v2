<script setup lang="ts">
import type { Intent, ThemeBadge } from '~/types/ui'

/**
 * Section « Composants de base » — boutons, pastilles, jetons de filtre,
 * alertes, cartes.
 *
 * CHAQUE COMPOSANT EST MONTRÉ AVEC TOUS SES ÉTATS, côte à côte. C'est le seul
 * moyen de vérifier qu'un état désactivé reste lisible, qu'un bouton en
 * chargement ne change pas de largeur, et qu'une alerte se distingue autrement
 * que par sa couleur.
 *
 * LES THÉMATIQUES VIENNENT DE LA BASE, chargées par la page via `useApi()` et
 * passées ici : leurs libellés et leurs couleurs se modifient au back-office, et
 * les figer dans le guide reproduirait le défaut de la v1 — des libellés de
 * thématiques recopiés dans le front, désynchronisés de la base.
 */

interface Props {
  /** Thématiques réelles (`reference.taxonomy_terms`, taxonomie `activity_theme`). */
  themes?: ThemeBadge[]
  /**
   * Titres d'activité réels, déjà résolus. Ce sont des DONNÉES, pas des libellés
   * d'interface : ils ne passent donc jamais par i18n — les recopier dans un
   * fichier de traduction est le défaut exact de la v1.
   */
  titles?: string[]
  /** Sigle de l'édition en cours (`event.events.acronym`) — donnée, elle aussi. */
  eventAcronym?: string | null
}

const props = defineProps<Props>()
const { t } = useI18n()
const { tr } = useI18nText()

const themeLabel = (theme: ThemeBadge): string =>
  typeof theme.label === 'string' ? theme.label : tr(theme.label)

/** Trois thématiques suffisent à démontrer le point coloré. */
const sampleThemes = computed(() => (props.themes ?? []).slice(0, 3))

const titleAt = (index: number): string => props.titles?.[index] ?? '—'

const BUTTON_VARIANTS = ['primary', 'secondary', 'ghost', 'danger', 'link'] as const
const INTENTS: Intent[] = ['neutral', 'info', 'success', 'warning', 'danger']

/** Bascule de chargement : le bouton travaille trois secondes, comme en vrai. */
const busyVariant = ref<string | null>(null)
function simulateWork(variant: string): void {
  busyVariant.value = variant
  setTimeout(() => (busyVariant.value = null), 2400)
}

/**
 * Jetons de filtre, retirables — l'état d'une liste filtrée. Les deux premiers
 * portent une thématique venue de la base, les deux autres une valeur d'ENUM
 * traduite : les deux sortes de libellés cohabitent dans une barre de filtres,
 * et c'est exactement le piège à ne pas confondre.
 */
const filters = computed(() => [
  ...sampleThemes.value.slice(0, 2).map((theme) => ({
    id: `theme-${theme.code}`,
    facetKey: 'style-guide.base.chips.facetTheme',
    label: themeLabel(theme),
    color: theme.color ?? null,
  })),
  {
    id: 'format',
    facetKey: 'style-guide.base.chips.facetFormat',
    label: t('session-card.format.hybrid'),
    color: null,
  },
  {
    id: 'status',
    facetKey: 'style-guide.base.chips.facetStatus',
    label: t('style-guide.business.status.under_review'),
    color: null,
  },
])
const removed = ref<string[]>([])
function removeFilter(id: string): void {
  removed.value = [...removed.value, id]
}
function resetFilters(): void {
  removed.value = []
}
const visibleFilters = computed(() => filters.value.filter((filter) => !removed.value.includes(filter.id)))
</script>

<template>
  <StyleGuideSection
    id="composants-base"
    :title="t('style-guide.base.title')"
    :description="t('style-guide.base.description')"
  >
    <!-- BOUTONS — variantes × états -->
    <StyleGuideDemo
      :title="t('style-guide.base.buttons.title')"
      :note="t('style-guide.base.buttons.note')"
    >
      <div class="space-y-5">
        <div v-for="variant in BUTTON_VARIANTS" :key="variant">
          <p class="mb-2 font-mono text-xs text-text-subtle">variant="{{ variant }}"</p>
          <div class="flex flex-wrap items-center gap-3">
            <UiButton :variant="variant">{{ t('style-guide.base.buttons.rest') }}</UiButton>
            <UiButton :variant="variant" icon="plus">{{ t('style-guide.base.buttons.withIcon') }}</UiButton>
            <UiButton :variant="variant" icon-trailing="chevron-right">
              {{ t('style-guide.base.buttons.trailing') }}
            </UiButton>
            <UiButton
              :variant="variant"
              :loading="busyVariant === variant"
              @click="simulateWork(variant)"
            >
              {{ t('style-guide.base.buttons.loading') }}
            </UiButton>
            <UiButton :variant="variant" disabled>{{ t('style-guide.base.buttons.disabled') }}</UiButton>
          </div>
        </div>

        <div class="border-t border-border-subtle pt-4">
          <p class="mb-2 font-mono text-xs text-text-subtle">size · icon-only · block</p>
          <div class="flex flex-wrap items-center gap-3">
            <UiButton size="sm">{{ t('style-guide.base.buttons.compact') }}</UiButton>
            <UiButton size="md">{{ t('style-guide.base.buttons.regular') }}</UiButton>
            <UiButton size="lg">{{ t('style-guide.base.buttons.large') }}</UiButton>
            <UiButton variant="secondary" icon-only icon="edit" :label="t('common.actions.edit')" />
            <UiButton variant="ghost" icon-only icon="more-horizontal" :label="t('common.actions.seeMore')" />
            <UiButton variant="danger" icon-only icon="trash" :label="t('common.actions.delete')" />
            <UiButton variant="secondary" pressed icon="filter">
              {{ t('style-guide.base.buttons.pressed') }}
            </UiButton>
          </div>
          <div class="mt-3 max-w-sm">
            <UiButton block icon="upload">{{ t('style-guide.base.buttons.block') }}</UiButton>
          </div>
        </div>
      </div>
    </StyleGuideDemo>

    <!-- PASTILLES -->
    <StyleGuideDemo
      :title="t('style-guide.base.badges.title')"
      :note="t('style-guide.base.badges.note')"
    >
      <div class="space-y-4">
        <div class="flex flex-wrap items-center gap-2">
          <UiBadge v-for="intent in INTENTS" :key="intent" :intent="intent">
            {{ t(`style-guide.base.badges.intents.${intent}`) }}
          </UiBadge>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <UiBadge v-for="intent in INTENTS" :key="`solid-${intent}`" :intent="intent" solid>
            {{ t(`style-guide.base.badges.intents.${intent}`) }}
          </UiBadge>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <UiBadge icon="check-circle" intent="success">{{ t('style-guide.base.badges.withIcon') }}</UiBadge>
          <UiBadge v-for="theme in sampleThemes" :key="theme.code" :dot-color="theme.color">
            {{ themeLabel(theme) }}
          </UiBadge>
          <UiBadge size="sm">{{ t('style-guide.base.badges.small') }}</UiBadge>
        </div>
        <p class="text-sm text-text-subtle">{{ t('style-guide.base.badges.dotNote') }}</p>
      </div>
    </StyleGuideDemo>

    <!-- JETONS DE FILTRE -->
    <StyleGuideDemo
      :title="t('style-guide.base.chips.title')"
      :note="t('style-guide.base.chips.note')"
    >
      <div class="flex flex-wrap items-center gap-2">
        <UiChip
          v-for="filter in visibleFilters"
          :key="filter.id"
          :facet="t(filter.facetKey)"
          :label="filter.label"
          :dot-color="filter.color"
          clickable
          @remove="removeFilter(filter.id)"
        />
        <UiChip
          :facet="t('style-guide.base.chips.facetEvent')"
          :label="props.eventAcronym ?? '—'"
          fixed
        />
        <UiChip :label="t('style-guide.base.chips.disabled')" disabled />

        <UiButton
          v-if="removed.length"
          variant="link"
          size="sm"
          icon="refresh"
          @click="resetFilters"
        >
          {{ t('style-guide.base.chips.reset') }}
        </UiButton>
      </div>
      <p class="mt-3 text-sm text-text-subtle">{{ t('style-guide.base.chips.fixedNote') }}</p>
    </StyleGuideDemo>

    <!-- ALERTES -->
    <StyleGuideDemo
      :title="t('style-guide.base.alerts.title')"
      :note="t('style-guide.base.alerts.note')"
    >
      <div class="space-y-3">
        <UiAlert intent="info" :title="t('style-guide.base.alerts.info.title')">
          {{ t('style-guide.base.alerts.info.body') }}
        </UiAlert>

        <UiAlert intent="success" :title="t('style-guide.base.alerts.success.title')" live>
          {{ t('style-guide.base.alerts.success.body') }}
        </UiAlert>

        <UiAlert intent="warning" :title="t('style-guide.base.alerts.warning.title')" dismissible>
          {{ t('style-guide.base.alerts.warning.body') }}
          <template #actions>
            <UiButton variant="secondary" size="sm">{{ t('style-guide.base.alerts.warning.action') }}</UiButton>
          </template>
        </UiAlert>

        <UiAlert intent="danger" :title="t('style-guide.base.alerts.danger.title')" live>
          {{ t('style-guide.base.alerts.danger.body') }}
        </UiAlert>

        <UiAlert intent="neutral" compact>
          {{ t('style-guide.base.alerts.compact') }}
        </UiAlert>
      </div>
    </StyleGuideDemo>

    <!-- CARTES -->
    <StyleGuideDemo
      :title="t('style-guide.base.cards.title')"
      :note="t('style-guide.base.cards.note')"
      surface
    >
      <div class="grid gap-4 md:grid-cols-3">
        <UiCard :eyebrow="t('style-guide.base.cards.eyebrow')" :title="titleAt(0)">
          <p class="text-sm text-text-muted">{{ t('style-guide.base.cards.body') }}</p>
          <template #footer>
            <UiButton variant="link" size="sm" icon-trailing="arrow-right">
              {{ t('common.actions.seeMore') }}
            </UiButton>
          </template>
        </UiCard>

        <UiCard
          :eyebrow="t('style-guide.base.cards.eyebrowSelected')"
          :title="titleAt(1)"
          selected
        >
          <p class="text-sm text-text-muted">{{ t('style-guide.base.cards.selected') }}</p>
        </UiCard>

        <UiCard :title="t('style-guide.base.cards.sunkenTitle')" sunken>
          <p class="text-sm text-text-muted">{{ t('style-guide.base.cards.sunken') }}</p>
        </UiCard>
      </div>
    </StyleGuideDemo>
  </StyleGuideSection>
</template>
