<script setup lang="ts">
import type { EditionPeriod } from '~/types/home'
import type { TabItem } from '~/types/ui'
import type { EditionStatsRow, PublicEditionRow } from '~/types/views'
import type { EventId } from '~/types/shared'

/**
 * L'HISTORIQUE DES ÉVÉNEMENTS — la troisième section de l'accueil.
 *
 * ── UNE FRISE QUI DÉFILE, PLUS UNE PILE QUI S'ALLONGE ───────────────────────
 *
 * La section empilait des rangées de cartes, une par millésime : au troisième
 * millésime, l'accueil mesurait quatre écrans et personne n'atteignait le bas.
 * Elle occupe désormais UN écran et pas davantage — en-tête fixe, rail
 * horizontal qui prend toute la hauteur restante. Ce que la pile disait par la
 * longueur, le rail le dit par le mouvement.
 *
 * SEUL LE RAIL DÉFILE HORIZONTALEMENT ; le corps de page, jamais. C'est la
 * règle des 375 px, et elle se joue à deux endroits ici : `overflow-x-auto` sur
 * la liste, et `min-w-0` sur tout parent flex qui la contient — sans quoi la
 * largeur minimale d'un élément flex vaut celle de son contenu, et ce sont
 * cinq affiches qui élargissent la page entière.
 *
 * ── LE MILLÉSIME N'EST PLUS UNE COLONNE ─────────────────────────────────────
 *
 * `groupEditionsByYear()` reste la source de l'ORDRE — années décroissantes,
 * éditions triées à l'intérieur — mais les groupes sont aplatis à l'affichage.
 * Un intertitre d'année dans un rail horizontal serait un repère qu'on dépasse
 * sans le voir ; chaque affiche porte donc son millésime, là où l'œil est déjà.
 *
 * L'utilitaire n'est pas contourné pour autant : c'est lui qui garantit qu'une
 * édition à cheval sur le 1er janvier se range sur son millésime ANNONCÉ et non
 * sur l'année de `starts_at`.
 *
 * ── LE FILTRE NE COÛTE AUCUNE REQUÊTE ───────────────────────────────────────
 *
 * `api.home.screen()` rend l'historique COMPLET : changer d'onglet ne fait que
 * recomposer ce qu'on a déjà. C'est ce qui permet aux onglets d'annoncer leurs
 * décomptes — « Passées (2) » se lit AVANT d'y aller — et ces décomptes se
 * calculent sur l'ensemble non filtré, jamais sur la sélection courante.
 *
 * ── L'ÉTAT VIT DANS L'URL ───────────────────────────────────────────────────
 *
 * `?periode=a-venir`, comme partout dans ce projet. Le composant n'en décide
 * pas : il reçoit la période et signale qu'on en demande une autre. Un état de
 * filtre enfermé dans un composant ne se partage pas, ne se recharge pas et ne
 * revient pas au retour arrière.
 *
 * ── LE DÉFILEMENT DOUX NE S'IMPOSE PAS ──────────────────────────────────────
 *
 * `scrollBy({ behavior: 'auto' })` n'est PAS l'absence d'animation : `auto`
 * délègue à la propriété CSS `scroll-behavior`, que la liste ne porte que sous
 * `motion-safe`. Une personne qui demande moins d'animations obtient donc un
 * saut net, sans que ce composant ait à interroger sa préférence — écrire
 * `behavior: 'smooth'` ici l'aurait ignorée.
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

/** L'ordre vient des groupes ; le rail, lui, ne connaît qu'une suite. */
const editions = computed(() => history.value.groups.flatMap((group) => group.editions))

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

// -------------------------------------------------------------------------
// Le rail
// -------------------------------------------------------------------------

const rail = ref<HTMLElement | null>(null)

/**
 * TROIS ÉTATS MESURÉS, PAS DEVINÉS. Le nombre d'affiches ne dit pas si le rail
 * déborde : cinq tiennent sur un écran large et deux débordent sur un
 * téléphone. Les commandes n'apparaissent donc que sur mesure réelle, et se
 * désactivent aux extrémités plutôt que de tourner en boucle — une boucle
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

onMounted(() => {
  const element = rail.value
  if (!element) return
  measure()
  element.addEventListener('scroll', measure, { passive: true })
  const observer = new ResizeObserver(measure)
  observer.observe(element)
  onBeforeUnmount(() => {
    element.removeEventListener('scroll', measure)
    observer.disconnect()
  })
})

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
  <!-- UN ÉCRAN, PAS DAVANTAGE. La section se donne la hauteur disponible sous la
       barre de navigation ; l'en-tête garde sa taille, le rail prend le reste.
       `min-h` et non `h` : si l'en-tête passe sur trois lignes à 375 px, la
       section grandit plutôt que d'écraser les affiches. -->
  <!-- `id="editions"` : le panneau « À venir » y renvoie depuis son bloc des
       prochains rendez-vous, comme le pied de page renvoie à l'appel. -->
  <section
    id="editions"
    aria-labelledby="historique-titre"
    class="flex scroll-mt-24 flex-col min-h-[calc(100svh-var(--nav-height))]"
  >
    <div class="flex flex-wrap items-end justify-between gap-4 border-b border-border pb-4">
      <div class="min-w-0">
        <h2 id="historique-titre" class="font-display text-2xl">
          {{ t('home.history.title') }}
        </h2>
        <p class="mt-1 text-text-muted" :style="{ maxWidth: 'var(--measure)' }">
          {{ t('home.history.description') }}
        </p>
      </div>

      <div class="flex min-w-0 max-w-full items-center gap-3">
        <!-- `min-w-0` : sans lui, la barre d'onglets est un élément flex dont la
             largeur minimale vaut celle de son contenu — quatre onglets et leurs
             compteurs élargissent alors la page entière au lieu de défiler dans
             leur propre `overflow-x-auto`. -->
        <UiTabs
          class="min-w-0 max-w-full"
          :model-value="props.period"
          :items="tabs"
          :label="t('home.history.filterLabel')"
          :panel-id="() => 'historique-panneau'"
          @update:model-value="onTab"
        />

        <!-- LES COMMANDES SONT UN CONFORT, PAS LE SEUL ACCÈS : le rail se fait
             au doigt, à la molette et à la tabulation — un lien qui prend le
             focus amène son affiche à l'écran de lui-même. Elles disparaissent
             donc sous `sm`, où le doigt est plus rapide qu'elles, et quand rien
             ne déborde. -->
        <div v-if="overflows" class="hidden shrink-0 items-center gap-2 sm:flex">
          <button
            type="button"
            class="flex cursor-pointer items-center justify-center rounded-full border border-border bg-surface-raised text-text-secondary transition-colors hover:bg-surface-sunken disabled:cursor-not-allowed disabled:opacity-40"
            :style="{ width: 'var(--target-min)', height: 'var(--target-min)' }"
            :disabled="atStart"
            :aria-label="t('home.history.rail.previous')"
            @click="move(-1)"
          >
            <UiIcon name="chevron-left" size="1.25rem" />
          </button>
          <button
            type="button"
            class="flex cursor-pointer items-center justify-center rounded-full border border-border bg-surface-raised text-text-secondary transition-colors hover:bg-surface-sunken disabled:cursor-not-allowed disabled:opacity-40"
            :style="{ width: 'var(--target-min)', height: 'var(--target-min)' }"
            :disabled="atEnd"
            :aria-label="t('home.history.rail.next')"
            @click="move(1)"
          >
            <UiIcon name="chevron-right" size="1.25rem" />
          </button>
        </div>
      </div>
    </div>

    <div id="historique-panneau" class="mt-6 flex min-h-0 flex-1 flex-col">
      <UiEmptyState
        v-if="!editions.length"
        icon="calendar"
        filtered
        :title="t('home.history.empty.title')"
        :description="t('home.history.empty.description')"
        :action-label="t('home.history.empty.action')"
        @action="emit('update:period', 'all')"
      />

      <!-- `-mx-1 px-1` : la gouttière qui laisse respirer l'ombre et l'anneau de
           focus des affiches de bord, sans décaler le rail par rapport au titre.
           `scroll-px-1` accorde l'accrochage à ce même retrait. -->
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
