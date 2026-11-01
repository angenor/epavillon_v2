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
  <section class="full-bleed -mb-8 bg-surface sm:-mb-10">
    <div class="mx-auto flex w-full max-w-[1280px] flex-col px-4 pb-14 sm:px-6 lg:pb-18">
      <header class="flex flex-wrap items-center gap-x-4 gap-y-3 border-b border-border-subtle py-3.5">
        <!-- `h-lh` et les tailles d'origine : chaque bloc prend la hauteur de ligne du texte qu'il remplace. -->
        <div class="flex h-lh w-40 items-center font-condensed text-[1.6rem] leading-[1.05]">
          <UiSkeletonLoader height="0.8em" />
        </div>
        <div class="min-w-0 flex-[1_1_280px] sm:max-w-90">
          <UiSkeletonLoader height="var(--target-min)" />
        </div>
        <div class="inline-flex shrink-0 gap-0.5 rounded-md border border-border-subtle bg-surface-sunken p-0.75">
          <div class="flex h-(--target-min) w-24 items-center justify-center px-5">
            <UiSkeletonLoader width="70%" height="0.875rem" />
          </div>
          <div class="flex h-(--target-min) w-32 items-center justify-center px-5">
            <UiSkeletonLoader width="75%" height="0.875rem" />
          </div>
        </div>
        <div class="flex h-lh basis-full items-center text-sm min-[860px]:ml-auto min-[860px]:w-64 min-[860px]:basis-auto">
          <UiSkeletonLoader variant="text" height="0.8em" />
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
        <div class="flex flex-col gap-1 border-b border-border-subtle bg-surface py-3.5 min-[960px]:hidden">
          <div class="-mx-4 flex min-w-0 items-center gap-1 overflow-hidden px-4 py-0.5 sm:mx-0 sm:px-0.5">
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
          class="grid gap-x-8 gap-y-4 border-b-2 border-text pt-9 pb-7 min-[960px]:grid-cols-[184px_minmax(0,1fr)]"
        >
          <header class="flex flex-wrap items-baseline gap-x-3.5 gap-y-1 self-start min-[960px]:block">
            <div class="flex h-[3.06rem] w-16 items-center min-[960px]:h-[4.42rem] min-[960px]:w-24">
              <UiSkeletonLoader height="85%" />
            </div>
            <div class="flex h-lh w-28 items-center font-condensed text-2xl leading-[1.05] min-[960px]:mt-2.5">
              <UiSkeletonLoader height="0.8em" />
            </div>
            <div class="flex h-lh w-36 items-center text-sm min-[960px]:mt-0.5">
              <UiSkeletonLoader variant="text" height="0.8em" />
            </div>
          </header>

          <ol>
            <li v-for="row in day.rows" :key="row" class="border-b border-border-subtle last:border-b-0">
              <div
                class="grid grid-cols-[76px_minmax(0,1fr)] gap-x-3.5 gap-y-2 py-5 min-[860px]:grid-cols-[112px_176px_minmax(0,1fr)_auto] min-[860px]:gap-x-5.5"
              >
                <div class="row-span-2 min-[860px]:row-span-1">
                  <div class="flex h-[1.76rem] w-16 items-center min-[860px]:h-[2.34rem] min-[860px]:w-24">
                    <UiSkeletonLoader height="85%" />
                  </div>
                  <div class="mt-1.5 flex h-lh w-18 items-center text-[0.82rem]">
                    <UiSkeletonLoader variant="text" height="0.8em" />
                  </div>
                </div>

                <div class="hidden aspect-video min-[860px]:block">
                  <UiSkeletonLoader height="100%" rounded="var(--radius-lg)" />
                </div>

                <div class="min-w-0">
                  <div class="font-condensed text-[1.25rem] leading-[1.12] min-[860px]:text-[1.45rem]">
                    <div class="flex h-lh items-center">
                      <UiSkeletonLoader :width="TITLE_WIDTHS[(row + dayIndex) % 4]" height="0.8em" />
                    </div>
                  </div>
                  <div class="mt-2 flex items-center gap-2.5 text-[0.9rem]">
                    <div class="size-10 shrink-0">
                      <UiSkeletonLoader height="100%" rounded="var(--radius-md)" />
                    </div>
                    <div class="flex h-lh flex-1 items-center">
                      <UiSkeletonLoader variant="text" :width="ORG_WIDTHS[(row + dayIndex) % 4]" height="0.8em" />
                    </div>
                  </div>
                </div>

                <div class="col-start-2 h-5 w-20 min-[860px]:col-start-4">
                  <UiSkeletonLoader height="100%" rounded="var(--radius-sm)" />
                </div>
              </div>
            </li>
          </ol>
        </section>
      </div>
    </div>
  </section>
</template>
