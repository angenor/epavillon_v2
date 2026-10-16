<script setup lang="ts">
/**
 * Silhouette de `EventProgramme`. Les conteneurs reprennent les classes de mise
 * en page de `EventProgramme`, `EventProgrammeWeek`, `EventProgrammeDayStrip` et
 * `EventProgrammeDayList` : à les modifier, modifier ici aussi, sinon la page
 * saute à l'arrivée des données.
 */

// Même règle que `EventProgramme` : la semaine sauf `?vue=liste`.
const route = useRoute()
const isList = computed(() => route.query.vue === 'liste')

const DAYS = [{ rows: 4 }, { rows: 3 }]
const TILES = 9
const TITLE_WIDTHS = ['92%', '78%', '86%', '70%']
const ORG_WIDTHS = ['55%', '40%', '62%', '48%']

const WEEK_CARDS = [3, 2, 3, 2, 3, 2]
const weekColumns = { '--week-columns': String(WEEK_CARDS.length) }
const WEEK_GRID =
  'grid grid-cols-[repeat(var(--week-columns),minmax(44%,1fr))] md:grid-cols-[repeat(var(--week-columns),minmax(30%,1fr))] lg:grid-cols-[repeat(var(--week-columns),minmax(11rem,1fr))]'
</script>

<template>
  <section class="full-bleed -mb-8 bg-surface font-sans sm:-mb-10">
    <div class="mx-auto flex w-full max-w-[1280px] flex-col px-4 pt-10 pb-14 sm:px-6 lg:pt-14 lg:pb-18">
      <header class="flex flex-wrap items-end justify-between gap-4 border-b border-border-subtle pb-4">
        <div class="min-w-0">
          <!-- `h-lh` et les tailles d'origine : chaque bloc prend la hauteur de ligne du texte qu'il remplace. -->
          <div class="flex h-lh w-44 items-center text-[30px] leading-tight">
            <UiSkeletonLoader height="0.8em" />
          </div>
          <!-- Le sous-titre réel est assez long pour renvoyer recherche et bascule sous le titre. -->
          <div class="mt-1.5 flex h-lh w-2xl max-w-full items-center text-base">
            <UiSkeletonLoader variant="text" height="0.8em" />
          </div>
        </div>
        <div class="flex w-full flex-col gap-3 sm:w-auto sm:flex-row sm:items-center sm:gap-2">
          <div class="min-w-0 sm:w-[300px]">
            <UiSkeletonLoader height="var(--target-min)" />
          </div>
          <div class="order-first grid shrink-0 grid-cols-2 gap-0.5 rounded-[10px] border border-border p-0.75 sm:order-none sm:flex">
            <div class="flex h-(--target-min) items-center justify-center px-5 sm:w-32">
              <UiSkeletonLoader width="70%" height="0.875rem" />
            </div>
            <div class="flex h-(--target-min) items-center justify-center px-5 sm:w-40">
              <UiSkeletonLoader width="75%" height="0.875rem" />
            </div>
          </div>
        </div>
      </header>

      <div v-if="!isList">
        <div class="flex flex-wrap items-center justify-between gap-4 border-b border-border-subtle py-4">
          <div class="flex flex-col sm:flex-row sm:items-center sm:gap-3.5">
            <div class="flex h-lh w-72 max-w-full items-center text-[24px] leading-tight">
              <UiSkeletonLoader height="0.8em" />
            </div>
            <div class="mt-1 flex h-lh w-44 items-center text-sm sm:mt-0">
              <UiSkeletonLoader variant="text" height="0.8em" />
            </div>
          </div>
          <div class="flex gap-2">
            <div class="size-(--target-min)">
              <UiSkeletonLoader height="100%" />
            </div>
            <div class="h-(--target-min) w-48">
              <UiSkeletonLoader height="100%" />
            </div>
          </div>
        </div>

        <div class="mt-7 overflow-hidden">
          <div :class="WEEK_GRID" :style="weekColumns">
            <div
              v-for="(cards, index) in WEEK_CARDS"
              :key="index"
              class="flex min-w-0 flex-col border-l border-border-subtle px-4 first:border-l-0"
              :class="index % 2 ? 'bg-text/3' : ''"
            >
              <div class="block flex-1 border-b-2 border-text py-3">
                <div class="flex items-end gap-2.5">
                  <div class="h-9 w-11 lg:h-[2.7rem] lg:w-13">
                    <UiSkeletonLoader height="100%" />
                  </div>
                  <div class="min-w-0 flex-1 pb-0.5">
                    <div class="flex h-5 w-16 max-w-full items-center">
                      <UiSkeletonLoader variant="text" height="0.8rem" />
                    </div>
                    <div class="flex h-4 w-12 max-w-full items-center">
                      <UiSkeletonLoader variant="text" height="0.6rem" />
                    </div>
                  </div>
                </div>
                <div class="mt-2 h-8" />
              </div>
            </div>
          </div>
        </div>

        <div class="overflow-hidden">
          <div :class="WEEK_GRID" :style="weekColumns">
            <div
              v-for="(cards, index) in WEEK_CARDS"
              :key="index"
              class="min-w-0 border-l border-border-subtle px-4 pb-3 first:border-l-0"
              :class="index % 2 ? 'bg-text/3' : ''"
            >
              <div v-for="card in cards" :key="card" class="border-b border-border-subtle py-3.5">
                <div class="aspect-video">
                  <UiSkeletonLoader height="100%" rounded="6px" />
                </div>
                <div class="mt-2.5 flex flex-wrap items-baseline gap-x-2">
                  <div class="h-6.5 w-16">
                    <UiSkeletonLoader height="85%" />
                  </div>
                  <div class="flex h-4 w-20 items-center">
                    <UiSkeletonLoader variant="text" height="0.6rem" />
                  </div>
                </div>
                <div class="mt-1.5 text-[15px] leading-[1.3]">
                  <div class="flex h-lh items-center">
                    <UiSkeletonLoader :width="TITLE_WIDTHS[(card + index) % 4]" height="0.8em" />
                  </div>
                  <div class="flex h-lh items-center">
                    <UiSkeletonLoader width="88%" height="0.8em" />
                  </div>
                  <div class="flex h-lh items-center">
                    <UiSkeletonLoader width="50%" height="0.8em" />
                  </div>
                </div>
                <div class="mt-1.5 flex h-lh items-center text-[11px]">
                  <UiSkeletonLoader variant="text" width="35%" height="0.8em" />
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="flex justify-end border-t border-border-subtle pt-5">
          <div class="h-(--target-min) w-48">
            <UiSkeletonLoader height="100%" />
          </div>
        </div>
      </div>

      <div v-else>
        <div class="flex flex-col gap-1 border-b border-border-subtle bg-surface py-3.5 lg:flex-row lg:items-center lg:gap-4">
          <div class="-mx-4 flex min-w-0 items-center gap-1 overflow-hidden px-4 py-0.5 sm:mx-0 sm:px-0.5 lg:flex-1">
            <div class="mr-3 flex h-4 w-24 shrink-0 items-center text-xs">
              <UiSkeletonLoader variant="text" height="0.8em" />
            </div>
            <div v-for="tile in TILES" :key="tile" class="h-14 w-15 shrink-0">
              <UiSkeletonLoader height="100%" rounded="var(--radius-lg)" />
            </div>
          </div>
        </div>

        <section
          v-for="(day, dayIndex) in DAYS"
          :key="dayIndex"
          class="grid gap-y-4 border-b border-text pt-8 pb-3 lg:grid-cols-[184px_minmax(0,1fr)] lg:gap-x-8"
        >
          <header class="flex flex-wrap items-baseline gap-x-3 self-start lg:block">
            <div class="flex h-[3.15rem] w-16 items-center lg:h-[4.05rem] lg:w-22">
              <UiSkeletonLoader height="85%" />
            </div>
            <div class="flex h-lh w-24 items-center text-lg lg:mt-2.5">
              <UiSkeletonLoader height="0.8em" />
            </div>
            <div class="flex h-lh basis-full items-center text-sm lg:mt-0.5">
              <UiSkeletonLoader variant="text" width="9rem" height="0.8em" />
            </div>
          </header>

          <ol>
            <li v-for="row in day.rows" :key="row">
              <div
                class="relative -ml-3 grid grid-cols-[7rem_minmax(0,1fr)] gap-x-4 gap-y-1.5 rounded-md px-3 py-4 lg:grid-cols-[112px_208px_minmax(0,1fr)_172px_24px] lg:items-center lg:gap-x-7 lg:gap-y-0 lg:py-4.5"
                :class="row % 2 === 0 ? 'bg-text/3' : ''"
              >
                <div class="col-start-2 row-start-1 flex flex-wrap items-baseline gap-x-2 self-start lg:col-start-1 lg:block lg:pt-1">
                  <div class="flex h-5 w-13 items-center lg:h-9.5 lg:w-24">
                    <UiSkeletonLoader height="85%" />
                  </div>
                  <div class="flex h-lh w-28 items-center text-[13px] lg:mt-1.5">
                    <UiSkeletonLoader variant="text" height="0.8em" />
                  </div>
                </div>

                <div class="col-start-1 row-span-3 row-start-1 aspect-video self-start lg:col-start-2 lg:row-span-1 lg:w-52 lg:self-center">
                  <UiSkeletonLoader height="100%" rounded="var(--radius-lg)" />
                </div>

                <div class="col-start-2 min-w-0 lg:col-start-3 lg:row-start-1">
                  <div class="text-base leading-[1.25] sm:text-lg lg:text-[21px]">
                    <div class="flex h-lh items-center">
                      <UiSkeletonLoader :width="TITLE_WIDTHS[(row + dayIndex) % 4]" height="0.8em" />
                    </div>
                    <div class="flex h-lh items-center">
                      <UiSkeletonLoader width="45%" height="0.8em" />
                    </div>
                  </div>
                  <div class="mt-1.5 flex h-lh items-center text-sm">
                    <UiSkeletonLoader variant="text" :width="ORG_WIDTHS[(row + dayIndex) % 4]" height="0.8em" />
                  </div>
                </div>

                <div class="col-start-2 h-5 w-24 justify-self-start lg:col-start-4 lg:row-start-1">
                  <UiSkeletonLoader height="100%" rounded="var(--radius-sm)" />
                </div>

                <div class="hidden size-5.5 lg:col-start-5 lg:row-start-1 lg:block">
                  <UiSkeletonLoader height="100%" />
                </div>
              </div>
            </li>
          </ol>
        </section>
      </div>
    </div>
  </section>
</template>
