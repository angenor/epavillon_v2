<script setup lang="ts">
// Silhouette du cas courant (bannière et appel) : conteneurs, grilles et points
// de rupture recopiés de `EventHero` et de la page, pour que rien ne bouge à
// l'arrivée des données.
const FACT_TILES = 4
const MILESTONE_STEPS = 4
const SPECIAL_DAYS = 3
</script>

<template>
  <div>
    <header class="bg-surface-inverse">
      <div class="mx-auto max-w-[1280px] px-4 pt-6 pb-9 sm:px-6 sm:pb-12 lg:pb-30">
        <div class="flex min-h-11 items-center">
          <UiSkeletonLoader tone="inverse" width="7rem" height="0.9rem" variant="text" />
        </div>
        <div class="mt-4 flex items-center gap-2.5">
          <UiSkeletonLoader tone="inverse" width="9rem" height="1rem" variant="text" />
          <UiSkeletonLoader tone="inverse" width="4.5rem" height="1.4rem" rounded="var(--radius-full)" />
        </div>
        <div class="mt-4 flex max-w-[34ch] flex-col gap-2 text-[clamp(2.2rem,4.4vw,3.6rem)]">
          <div class="h-[1em] w-[92%]"><UiSkeletonLoader tone="inverse" height="100%" /></div>
          <div class="h-[1em] w-3/5"><UiSkeletonLoader tone="inverse" height="100%" /></div>
        </div>
        <div class="mt-7 grid max-w-[860px] grid-cols-2 gap-2.5 lg:grid-cols-4">
          <UiSkeletonLoader v-for="tile in FACT_TILES" :key="tile" tone="inverse" height="5.25rem" rounded="var(--radius-lg)" />
        </div>
      </div>
    </header>

    <div class="mx-auto max-w-[1280px] px-4 pb-16 sm:px-6 lg:pb-22">
      <div class="grid gap-10 pt-6 lg:grid-cols-[minmax(0,1fr)_380px] lg:items-start lg:gap-12 lg:pt-0">
        <aside class="relative z-10 flex flex-col gap-4 lg:col-start-2 lg:row-start-1 lg:-mt-[72px]">
          <div class="overflow-hidden rounded-lg bg-surface-raised shadow-lg">
            <div class="bg-surface-inverse px-5.5 py-4">
              <UiSkeletonLoader tone="inverse" width="40%" height="0.7rem" variant="text" />
              <UiSkeletonLoader class="mt-2" tone="inverse" width="80%" height="1.4rem" />
            </div>
            <div class="p-6">
              <UiSkeletonLoader width="5.5rem" height="1.4rem" rounded="var(--radius-full)" />
              <UiSkeletonLoader class="mt-4" width="40%" height="0.75rem" variant="text" />
              <UiSkeletonLoader class="mt-2" width="70%" height="2rem" />
              <UiSkeletonLoader class="mt-2" width="55%" height="0.85rem" variant="text" />
              <div class="mt-5 flex flex-col gap-2.5">
                <UiSkeletonLoader height="var(--target-min)" />
                <UiSkeletonLoader height="var(--target-min)" />
              </div>
            </div>
          </div>

          <div class="rounded-lg border border-border-subtle bg-surface-raised px-5.5 py-5">
            <UiSkeletonLoader width="55%" height="1.4rem" />
            <UiSkeletonLoader class="mt-2" variant="text" width="80%" />
            <ol class="mt-4 flex flex-col gap-5">
              <li v-for="step in MILESTONE_STEPS" :key="step" class="flex gap-3">
                <UiSkeletonLoader variant="circle" width="1.75rem" height="1.75rem" />
                <div class="min-w-0 flex-1 space-y-1.5 pt-1">
                  <UiSkeletonLoader width="60%" height="0.95rem" variant="text" />
                  <UiSkeletonLoader width="40%" height="0.75rem" variant="text" />
                </div>
              </li>
            </ol>
          </div>
        </aside>

        <div class="flex min-w-0 flex-col gap-14 lg:col-start-1 lg:row-start-1 lg:pt-14">
          <section>
            <div class="h-9 w-72 max-w-full"><UiSkeletonLoader height="100%" /></div>
            <UiSkeletonLoader class="mt-4 max-w-[68ch]" variant="text" height="1.1rem" :lines="5" />
          </section>

          <section class="rounded-lg bg-surface-inverse p-6 sm:p-7">
            <div class="flex flex-wrap items-end justify-between gap-6">
              <div class="min-w-0 flex-1">
                <div class="h-9 w-64 max-w-full"><UiSkeletonLoader tone="inverse" height="100%" /></div>
                <div class="mt-3.5 flex gap-7">
                  <div v-for="figure in 3" :key="figure" class="h-12 w-14"><UiSkeletonLoader tone="inverse" height="100%" /></div>
                </div>
              </div>
              <div class="w-60 max-w-full"><UiSkeletonLoader tone="inverse" height="var(--target-min)" /></div>
            </div>
          </section>

          <section>
            <div class="h-9 w-64 max-w-full"><UiSkeletonLoader height="100%" /></div>
            <UiSkeletonLoader class="mt-2 max-w-[64ch]" variant="text" :lines="2" />
            <div class="mt-5 grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
              <UiSkeletonLoader v-for="day in SPECIAL_DAYS" :key="day" height="300px" rounded="var(--radius-lg)" />
            </div>
          </section>
        </div>
      </div>
    </div>
  </div>
</template>
