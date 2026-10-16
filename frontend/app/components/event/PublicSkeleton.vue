<script setup lang="ts">
// Silhouette du cas courant : édition avec bannière et appel ouvert. Conteneurs,
// grilles et points de rupture recopiés de `EventHero`, `EventHeroCall` et de la
// page, pour que rien ne bouge à l'arrivée des données.
const FACT_TILES = 4
const MILESTONE_STEPS = 4
const SPECIAL_DAYS = 2
</script>

<template>
  <div class="flex flex-col">
    <header>
      <!-- Le bandeau garde son aplat sombre : sans photo, il reste la masse qui
           ouvre la page. -->
      <div class="full-bleed relative -mt-8 overflow-hidden bg-surface-inverse sm:-mt-10">
        <div
          class="relative mx-auto w-full max-w-[1280px] px-4 pt-16 pb-10 sm:px-6 sm:pt-24 sm:pb-14 lg:pt-28"
        >
          <div class="grid items-end gap-8 lg:grid-cols-12 lg:gap-10">
            <div class="min-w-0 lg:col-span-7 xl:col-span-8">
              <div class="flex items-center gap-2">
                <UiSkeletonLoader tone="inverse" width="7rem" height="0.85rem" variant="text" />
                <UiSkeletonLoader tone="inverse" width="4.5rem" height="1.4rem" rounded="var(--radius-full)" />
              </div>

              <div class="mt-3 flex flex-col gap-2">
                <div class="h-8 w-[92%] sm:h-10 lg:h-12">
                  <UiSkeletonLoader tone="inverse" height="100%" />
                </div>
                <div class="h-8 w-3/5 sm:h-10 lg:h-12">
                  <UiSkeletonLoader tone="inverse" height="100%" />
                </div>
              </div>

              <div class="mt-6 hidden max-w-3xl gap-3 sm:grid sm:grid-cols-2">
                <UiSkeletonLoader
                  v-for="tile in FACT_TILES"
                  :key="tile"
                  tone="inverse"
                  height="4.75rem"
                  rounded="var(--radius-lg)"
                />
              </div>
            </div>

            <div class="lg:col-span-5 xl:col-span-4">
              <div class="rounded-xl border border-border-on-inverse p-5 sm:p-6">
                <UiSkeletonLoader tone="inverse" width="5.5rem" height="1.4rem" rounded="var(--radius-full)" />
                <UiSkeletonLoader class="mt-4" tone="inverse" width="40%" height="0.75rem" variant="text" />
                <div class="mt-1 h-9 w-1/2 sm:h-12">
                  <UiSkeletonLoader tone="inverse" height="100%" />
                </div>
                <UiSkeletonLoader class="mt-4" tone="inverse" width="35%" height="0.75rem" variant="text" />
                <UiSkeletonLoader class="mt-2" tone="inverse" width="70%" height="0.9rem" variant="text" />
                <UiSkeletonLoader class="mt-1.5" tone="inverse" width="30%" height="0.75rem" variant="text" />
                <div class="mt-5 flex flex-col gap-2">
                  <UiSkeletonLoader tone="inverse" height="var(--target-min)" />
                  <UiSkeletonLoader tone="inverse" height="var(--target-min)" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Sous `sm`, les faits quittent le bandeau, comme dans `EventHero`. -->
      <div class="mt-8 grid max-w-3xl gap-3 sm:hidden">
        <UiSkeletonLoader
          v-for="tile in FACT_TILES"
          :key="tile"
          height="4.75rem"
          rounded="var(--radius-lg)"
        />
      </div>
    </header>

    <div class="mt-12 grid gap-12 lg:grid-cols-12 lg:gap-x-12">
      <aside class="lg:order-2 lg:col-span-4">
        <div class="flex flex-col gap-6">
          <div class="rounded-xl border border-border bg-surface-raised p-5 sm:p-6">
            <UiSkeletonLoader width="55%" height="1.4rem" />
            <UiSkeletonLoader class="mt-2" variant="text" :lines="2" />
            <ol class="mt-5 flex flex-col gap-5">
              <li v-for="step in MILESTONE_STEPS" :key="step" class="flex gap-3">
                <UiSkeletonLoader variant="circle" width="1.75rem" height="1.75rem" />
                <div class="min-w-0 flex-1 space-y-1.5 pt-1">
                  <UiSkeletonLoader width="60%" height="0.85rem" variant="text" />
                  <UiSkeletonLoader width="40%" height="0.75rem" variant="text" />
                </div>
              </li>
            </ol>
          </div>

          <div class="rounded-xl border border-border bg-surface-raised p-5 sm:p-6">
            <UiSkeletonLoader width="85%" height="1.4rem" />
            <UiSkeletonLoader class="mt-3" variant="text" :lines="3" />
            <div class="mt-5 space-y-4">
              <div v-for="term in 3" :key="term" class="space-y-1.5">
                <UiSkeletonLoader width="35%" height="0.75rem" variant="text" />
                <UiSkeletonLoader width="75%" height="0.9rem" variant="text" />
              </div>
            </div>
          </div>
        </div>
      </aside>

      <div class="flex flex-col gap-14 lg:order-1 lg:col-span-8">
        <section>
          <div class="h-8 w-48 sm:h-9">
            <UiSkeletonLoader height="100%" />
          </div>
          <UiSkeletonLoader class="mt-5" variant="text" height="1.25rem" :lines="6" />
        </section>

        <section>
          <div class="h-8 w-56 sm:h-9">
            <UiSkeletonLoader height="100%" />
          </div>
          <UiSkeletonLoader class="mt-3 max-w-(--measure)" variant="text" :lines="2" />
          <div class="mt-6 grid gap-4 md:grid-cols-2">
            <div
              v-for="day in SPECIAL_DAYS"
              :key="day"
              class="rounded-xl border border-border bg-surface-raised p-5 pl-6"
            >
              <UiSkeletonLoader width="70%" height="1.15rem" />
              <UiSkeletonLoader class="mt-2" width="45%" height="0.85rem" variant="text" />
              <UiSkeletonLoader class="mt-3" variant="text" :lines="3" />
              <div class="mt-4 flex flex-wrap items-center gap-3">
                <UiSkeletonLoader width="45%" height="0.85rem" variant="text" />
                <UiSkeletonLoader width="5rem" height="1.4rem" rounded="var(--radius-full)" />
              </div>
            </div>
          </div>
        </section>

        <section class="rounded-xl bg-surface-inverse p-6 sm:p-8">
          <div class="flex flex-wrap items-end justify-between gap-6">
            <div class="min-w-0 flex-1">
              <div class="h-8 w-56 max-w-full sm:h-9">
                <UiSkeletonLoader tone="inverse" height="100%" />
              </div>
              <div class="mt-4 flex items-baseline gap-3">
                <div class="h-10 w-16">
                  <UiSkeletonLoader tone="inverse" height="100%" />
                </div>
                <UiSkeletonLoader tone="inverse" width="9rem" height="0.9rem" variant="text" />
              </div>
            </div>
            <div class="w-56 max-w-full">
              <UiSkeletonLoader tone="inverse" height="var(--target-min)" />
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>
