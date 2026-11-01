<script setup lang="ts">
import type { EditionPeriod } from '~/types/home'
import type { TabItem } from '~/types/ui'
import type { EditionStatsRow, PublicEditionRow } from '~/types/views'
import type { EventId } from '~/types/shared'

/**
 * LES ÉDITIONS : la dernière section de l'accueil, en liste éditoriale (par
 * défaut) ou en affiches (arbitré le 03/10), même habit condensé (01/11).
 *
 * `groupEditionsByYear()` reste la source de l'ORDRE et du millésime ANNONCÉ
 * (une édition à cheval sur le 1er janvier garde le sien) ; chaque ligne,
 * chaque affiche porte ce millésime.
 *
 * LE FILTRE NE COÛTE AUCUNE REQUÊTE : `api.home.screen()` rend l'historique
 * complet, et les onglets annoncent leurs décomptes calculés sur l'ensemble non
 * filtré. Son état vit dans l'URL (`?periode=a-venir`) : il se partage et
 * survit au retour arrière.
 *
 * SEUL LE RAIL DES AFFICHES DÉFILE HORIZONTALEMENT, jamais le corps de page :
 * `overflow-x-auto` sur la liste, `min-w-0` sur ses parents flex.
 * `scrollBy({ behavior: 'auto' })` délègue à `scroll-behavior`, posé sous
 * `motion-safe` seulement : moins d'animations donne un saut net.
 */

interface Props {
  /** L'historique COMPLET, le filtre est appliqué ici, pas en amont. */
  editions: PublicEditionRow[]
  /** `programme.v_edition_stats`, indexée par `event_id`. Clé absente = zéro. */
  stats: Record<EventId, EditionStatsRow>
  period: EditionPeriod
}

const props = defineProps<Props>()

const emit = defineEmits<{ 'update:period': [period: EditionPeriod] }>()

const { t } = useI18n()

const history = computed(() => buildEditionHistory(props.editions, props.stats, props.period))

/** L'ordre vient des groupes ; le rail, lui, ne connaît qu'une suite. */
const editions = computed(() => history.value.groups.flatMap((group) => group.editions))

const tabs = computed<TabItem[]>(() =>
  EDITION_PERIODS.map((period) => ({
    value: period,
    label: t(`home.history.tabs.${period}`),
    count: history.value.counts[period],
  })),
)

/** Une valeur d'onglet est une période, `UiTabs` ne connaît que des chaînes. */
function onTab(value: string): void {
  const period = EDITION_PERIODS.find((entry) => entry === value)
  if (period) emit('update:period', period)
}

// -------------------------------------------------------------------------
// La vue : liste (par défaut) ou affiches, arbitrée le 03/10
// -------------------------------------------------------------------------

type HistoryView = 'posters' | 'list'

/** Un cookie et non le stockage du navigateur : le rendu serveur sert d'emblée la bonne vue. */
const viewCookie = useCookie<string>('ep-editions-vue', {
  default: () => 'list',
  maxAge: 60 * 60 * 24 * 365,
  sameSite: 'lax',
})

const view = computed<HistoryView>({
  get: () => (viewCookie.value === 'posters' ? 'posters' : 'list'),
  set: (value) => {
    viewCookie.value = value
  },
})

const views = computed(() => [
  { value: 'list' as const, icon: 'list', label: t('home.history.view.list') },
  { value: 'posters' as const, icon: 'grid', label: t('home.history.view.posters') },
])

// -------------------------------------------------------------------------
// Le rail
// -------------------------------------------------------------------------

const rail = ref<HTMLElement | null>(null)

/**
 * TROIS ÉTATS MESURÉS, PAS DEVINÉS. Le nombre d'affiches ne dit pas si le rail
 * déborde : cinq tiennent sur un écran large et deux débordent sur un
 * téléphone. Les commandes n'apparaissent donc que sur mesure réelle, et se
 * désactivent aux extrémités plutôt que de tourner en boucle, une boucle
 * silencieuse fait croire qu'il reste des éditions à voir.
 */
const overflows = ref(false)
const atStart = ref(true)
const atEnd = ref(false)

function measure(): void {
  const element = rail.value
  if (!element) return
  const max = element.scrollWidth - element.clientWidth
  overflows.value = max > 1
  atStart.value = element.scrollLeft <= 1
  atEnd.value = element.scrollLeft >= max - 1
}

/** Un pas = une affiche et sa gouttière, mesurées sur le rendu réel. */
function step(element: HTMLElement): number {
  const card = element.firstElementChild
  if (!card) return element.clientWidth
  const gap = Number.parseFloat(getComputedStyle(element).columnGap) || 0
  return card.getBoundingClientRect().width + gap
}

function move(direction: 1 | -1): void {
  const element = rail.value
  if (!element) return
  element.scrollBy({ left: direction * step(element), behavior: 'auto' })
}

// Le rail naît et meurt avec la vue « Affiches » comme avec l'état vide : on le
// suit, plutôt que de ne l'équiper qu'au montage.
watch(
  rail,
  (element, _previous, onCleanup) => {
    if (!element) return
    measure()
    element.addEventListener('scroll', measure, { passive: true })
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    onCleanup(() => {
      element.removeEventListener('scroll', measure)
      observer.disconnect()
    })
  },
  { flush: 'post' },
)

/**
 * CHANGER D'ONGLET REVIENT AU DÉBUT. Sans cela, on filtre sur « Passées » en
 * étant déjà à mi-course et la liste s'ouvre sur une affiche du milieu, ce qui
 * se lit comme un filtre qui a mangé les premières.
 */
watch(
  () => props.period,
  async () => {
    await nextTick()
    rail.value?.scrollTo({ left: 0, behavior: 'auto' })
    measure()
  },
)
</script>

<template>
  <!-- `id="editions"` : le panneau « À venir » y renvoie, comme le pied de page renvoie à l'appel.
       En affiches, la section prend un écran : l'en-tête garde sa taille, le rail le reste. -->
  <section
    id="editions"
    aria-labelledby="historique-titre"
    class="flex scroll-mt-24 flex-col"
    :class="{ 'min-h-[calc(100svh-var(--nav-height))]': view === 'posters' }"
  >
    <div class="mb-8 flex flex-wrap items-end justify-between gap-x-8 gap-y-4">
      <div class="min-w-0">
        <h2 id="historique-titre" class="heading-condensed text-[clamp(2rem,4vw,2.75rem)]">
          {{ t('home.history.title') }}
        </h2>
        <p class="mt-2 max-w-[62ch] text-text-secondary">
          {{ t('home.history.description') }}
        </p>
      </div>

      <div class="flex min-w-0 max-w-full flex-wrap items-center gap-3">
        <!-- `min-w-0` : sans lui, les onglets élargissent la page au lieu de défiler dans leur barre. -->
        <UiTabs
          class="min-w-0 max-w-full"
          :model-value="props.period"
          :items="tabs"
          :label="t('home.history.filterLabel')"
          :panel-id="() => 'historique-panneau'"
          @update:model-value="onTab"
        >
          <template #tab="{ item, selected }">
            <span class="font-condensed text-[1.05rem] font-semibold tracking-[0.05em] uppercase">{{ item.label }}</span>
            <UiCounter v-if="item.count !== undefined" :value="item.count" :tone="selected ? 'accent' : 'neutral'" />
          </template>
        </UiTabs>

        <div
          role="group"
          class="flex shrink-0 items-center gap-0.5 rounded-lg border border-border p-0.5"
          :aria-label="t('home.history.view.label')"
          :data-bird-say="t('home.bird.view')"
        >
          <UiButton
            v-for="option in views"
            :key="option.value"
            variant="ghost"
            icon-only
            :icon="option.icon"
            :label="option.label"
            :pressed="view === option.value"
            @click="view = option.value"
          />
        </div>
        <!-- Un confort, pas le seul accès : le rail se fait au doigt, à la molette et à la tabulation. -->
        <div v-if="overflows && view === 'posters'" class="hidden shrink-0 items-center gap-2 sm:flex">
          <button
            type="button"
            class="flex size-(--target-min) cursor-pointer items-center justify-center rounded-full border border-border bg-surface-raised text-text-secondary transition-colors hover:bg-surface-sunken disabled:cursor-not-allowed disabled:opacity-40"
            :disabled="atStart"
            :aria-label="t('home.history.rail.previous')"
            @click="move(-1)"
          >
            <UiIcon name="chevron-left" size="1.25rem" />
          </button>
          <button
            type="button"
            class="flex size-(--target-min) cursor-pointer items-center justify-center rounded-full border border-border bg-surface-raised text-text-secondary transition-colors hover:bg-surface-sunken disabled:cursor-not-allowed disabled:opacity-40"
            :disabled="atEnd"
            :aria-label="t('home.history.rail.next')"
            @click="move(1)"
          >
            <UiIcon name="chevron-right" size="1.25rem" />
          </button>
        </div>
      </div>
    </div>

    <div id="historique-panneau" class="flex min-h-0 flex-1 flex-col">
      <UiEmptyState
        v-if="!editions.length"
        icon="calendar"
        filtered
        :title="t('home.history.empty.title')"
        :description="t('home.history.empty.description')"
        :action-label="t('home.history.empty.action')"
        @action="emit('update:period', 'all')"
      />

      <!-- La liste éditoriale sous un trait épais ; chaque ligne porte son millésime. -->
      <ul
        v-else-if="view === 'list'"
        class="m-0 list-none border-t-2 border-text p-0"
        :aria-label="t('home.history.list.label')"
      >
        <template v-for="group in history.groups" :key="group.year">
          <li v-for="edition in group.editions" :key="edition.id" class="border-b border-border-subtle">
            <HomeEditionRow
              :edition="edition"
              :year="group.year"
              :session-count="publishedSessionCount(history.stats, edition.id)"
            />
          </li>
        </template>
      </ul>

      <!-- `-mx-1 px-1` : l'ombre et l'anneau de focus des affiches de bord respirent sans décaler le rail. -->
      <ul
        v-else
        ref="rail"
        class="-mx-1 flex min-h-0 flex-1 snap-x snap-mandatory gap-4 overflow-x-auto scroll-px-1 px-1 pb-3 motion-safe:scroll-smooth"
        :aria-label="t('home.history.rail.label')"
      >
        <li
          v-for="edition in editions"
          :key="edition.id"
          class="flex w-[78vw] max-w-[22rem] shrink-0 snap-start lg:w-[21rem] xl:w-[23rem]"
        >
          <HomeEditionCard
            class="w-full"
            :edition="edition"
            :session-count="publishedSessionCount(history.stats, edition.id)"
          />
        </li>
      </ul>
    </div>
  </section>
</template>
