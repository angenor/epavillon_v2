<script setup lang="ts">
// Silhouette de l'accueil : mêmes conteneurs, classes de mise en page et points de
// rupture que `HomeShowcase`, `HomeAsidePanel`, `HomeCallSection` et
// `HomeEditionHistory`. Une classe de mise en page changée là-bas se reporte ici,
// sinon la page saute à l'arrivée des données.

// Même témoin que `HomeEditionHistory` : la silhouette suit la vue choisie par le visiteur.
const viewCookie = useCookie<string | null>('ep-editions-vue', { readonly: true })
const isPosters = computed(() => viewCookie.value === 'posters')

const historyGroups = [2, 1] as const
</script>

<template>
  <div class="flex flex-col gap-16">
    <!-- Bandeau : un écran de haut, panneau « À venir » de 340 px à gauche sur `lg`, dessous en deçà. -->
    <section class="full-bleed relative -mt-8 overflow-hidden bg-surface-inverse sm:-mt-10">
      <div class="relative flex min-h-[calc(100svh-var(--nav-height)-var(--topbar-height))] flex-col lg:ps-[340px]">
        <div class="relative flex min-h-0 flex-1 flex-col justify-end">
          <div class="w-full pb-14 sm:pb-16">
            <div
              class="mx-auto flex w-full max-w-[1280px] flex-col gap-5 px-4 py-8 sm:px-6 sm:py-10 lg:flex-row lg:items-end lg:gap-12"
            >
              <div class="min-w-0 flex-1" :style="{ maxWidth: '52rem' }">
                <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
                  <UiSkeletonLoader tone="inverse" width="7rem" height="1.75rem" rounded="var(--radius-full)" />
                  <UiSkeletonLoader tone="inverse" variant="text" width="10rem" height="1rem" />
                </div>
                <div class="relative mt-4 ps-8 sm:ps-11">
                  <div class="absolute start-0 top-0">
                    <UiSkeletonLoader tone="inverse" width="1.25rem" height="1.25rem" />
                  </div>
                  <UiSkeletonLoader
                    tone="inverse"
                    variant="text"
                    :lines="3"
                    width="100%"
                    height="var(--font-size-display)"
                  />
                </div>
              </div>

              <div
                class="flex shrink-0 flex-col gap-2 border-t border-glass-border-strong pt-4 lg:w-72 lg:border-t-0 lg:border-s-2 lg:ps-6 lg:pt-0"
              >
                <UiSkeletonLoader tone="inverse" variant="text" width="9rem" height="1rem" />
                <UiSkeletonLoader tone="inverse" variant="text" width="80%" />
                <UiSkeletonLoader tone="inverse" variant="text" width="65%" />
                <div class="flex min-h-(--target-min) items-center">
                  <UiSkeletonLoader tone="inverse" variant="text" width="8rem" height="1rem" />
                </div>
              </div>
            </div>
          </div>

          <div class="absolute inset-x-0 bottom-0 z-10 pt-10 pb-3">
            <div class="mx-auto flex w-full max-w-[1280px] items-center gap-2 px-2 sm:gap-3 sm:px-6">
              <div class="flex min-w-0 items-center gap-2 sm:gap-3 lg:max-w-[52rem]">
                <UiSkeletonLoader tone="inverse" variant="circle" width="var(--target-min)" height="var(--target-min)" class="shrink-0" />
                <div class="flex min-w-0 items-center gap-2 overflow-hidden py-1">
                  <div v-for="index in 5" :key="index" class="h-11 w-[4.5rem] shrink-0 sm:h-14 sm:w-24">
                    <UiSkeletonLoader tone="inverse" height="100%" />
                  </div>
                </div>
                <UiSkeletonLoader tone="inverse" variant="circle" width="var(--target-min)" height="var(--target-min)" class="shrink-0" />
              </div>
              <span class="hidden flex-1 lg:block" />
              <UiSkeletonLoader
                tone="inverse"
                variant="circle"
                width="var(--target-min)"
                height="var(--target-min)"
                class="ml-auto shrink-0 lg:ml-0"
              />
              <div class="ml-1 hidden shrink-0 sm:block">
                <UiSkeletonLoader tone="inverse" width="4rem" height="2rem" rounded="var(--radius-full)" />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="lg:absolute lg:inset-y-0 lg:start-0 lg:z-20 lg:w-[340px]">
        <div class="relative flex h-full flex-col bg-surface-inverse lg:border-e lg:border-glass-border">
          <div class="border-b border-glass-border px-4 py-4 sm:px-6 lg:px-5">
            <div class="mx-auto flex h-7 w-full max-w-[1280px] items-center lg:mx-0">
              <UiSkeletonLoader tone="inverse" width="6rem" height="1.25rem" />
            </div>
          </div>

          <div class="min-h-0 flex-1 px-4 py-4 sm:px-6 lg:overflow-hidden lg:px-5 lg:pb-28">
            <div class="mx-auto flex w-full max-w-[1280px] flex-col gap-6 lg:mx-0">
              <div>
                <div class="flex items-center justify-between gap-2">
                  <UiSkeletonLoader tone="inverse" variant="text" width="9rem" height="0.75rem" />
                  <UiSkeletonLoader tone="inverse" variant="text" width="5rem" height="0.75rem" />
                </div>
                <div class="mt-3 ms-1 flex flex-col gap-4 border-s border-glass-border ps-[18px]">
                  <!-- Les cartes d'édition se taisent une fois le programme publié : la frise ouvre le panneau. -->
                  <div v-for="day in 2" :key="day" class="flex flex-col gap-2">
                    <UiSkeletonLoader tone="inverse" variant="text" width="10rem" />
                    <div
                      v-for="card in day === 1 ? 3 : 1"
                      :key="card"
                      class="flex flex-col gap-2 rounded-lg border border-glass-border p-3"
                    >
                      <UiSkeletonLoader tone="inverse" variant="text" width="45%" height="0.75rem" />
                      <UiSkeletonLoader tone="inverse" variant="text" :lines="2" width="100%" />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="relative px-4 pb-5 sm:px-6 lg:absolute lg:inset-x-0 lg:bottom-0 lg:bg-surface-inverse lg:px-5 lg:pt-4">
            <UiSkeletonLoader tone="inverse" height="var(--target-min)" />
          </div>
        </div>
      </div>
    </section>

    <!-- Appel à propositions : bloc sombre, contenu en bas à gauche, rebours à droite sur `lg`. -->
    <section>
      <div class="overflow-hidden rounded-xl bg-surface-inverse shadow-md">
        <div
          class="grid gap-8 px-5 py-8 sm:px-10 sm:py-12 lg:min-h-96 lg:grid-cols-[minmax(0,1fr)_auto] lg:items-end lg:px-12"
        >
          <div class="max-w-xl">
            <div class="flex flex-wrap items-center gap-3">
              <UiSkeletonLoader tone="inverse" width="5.5rem" height="1.5rem" rounded="var(--radius-full)" />
              <UiSkeletonLoader tone="inverse" variant="text" width="8rem" height="0.75rem" />
            </div>
            <div class="mt-4 flex flex-col gap-2">
              <div class="h-7 w-full sm:h-9">
                <UiSkeletonLoader tone="inverse" height="100%" />
              </div>
              <div class="h-7 w-3/5 sm:h-9">
                <UiSkeletonLoader tone="inverse" height="100%" />
              </div>
            </div>
            <UiSkeletonLoader tone="inverse" variant="text" :lines="3" class="mt-4" />
            <div class="mt-6 grid gap-4 sm:grid-cols-2">
              <div v-for="index in 2" :key="index" class="flex items-start gap-3">
                <UiSkeletonLoader tone="inverse" variant="circle" width="1.15rem" height="1.15rem" class="mt-0.5 shrink-0" />
                <div class="flex min-w-0 flex-1 flex-col gap-2">
                  <UiSkeletonLoader tone="inverse" variant="text" width="6rem" height="0.75rem" />
                  <UiSkeletonLoader tone="inverse" variant="text" width="80%" height="1rem" />
                </div>
              </div>
            </div>
            <div class="mt-8 flex flex-wrap items-center gap-x-6 gap-y-3">
              <UiSkeletonLoader tone="inverse" width="14rem" height="var(--target-min)" />
              <UiSkeletonLoader tone="inverse" variant="text" width="9rem" height="1rem" />
            </div>
          </div>

          <div class="flex flex-col gap-2 justify-self-start rounded-lg border border-border-on-inverse px-6 py-5 lg:justify-self-end">
            <UiSkeletonLoader tone="inverse" variant="text" width="6rem" height="0.75rem" />
            <div class="h-10 w-36 sm:h-12">
              <UiSkeletonLoader tone="inverse" height="100%" />
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Historique des éditions : en-tête souligné, puis liste par année ou rail d'affiches. -->
    <section class="flex flex-col" :class="{ 'min-h-[calc(100svh-var(--nav-height))]': isPosters }">
      <div class="flex flex-wrap items-end justify-between gap-4 border-b border-border pb-4">
        <div class="flex min-w-0 flex-1 flex-col gap-2">
          <UiSkeletonLoader width="14rem" height="1.75rem" />
          <UiSkeletonLoader variant="text" width="min(100%, 30rem)" height="1rem" />
        </div>
        <div class="flex min-w-0 max-w-full items-center gap-3">
          <div class="flex min-h-(--target-min) min-w-0 items-center gap-6 overflow-hidden px-4">
            <UiSkeletonLoader v-for="index in 4" :key="index" variant="text" width="4.5rem" class="shrink-0" />
          </div>
          <div class="flex shrink-0 items-center gap-0.5 rounded-lg border border-border p-0.5">
            <UiSkeletonLoader v-for="index in 2" :key="index" width="var(--target-min)" height="var(--target-min)" />
          </div>
        </div>
      </div>

      <div v-if="isPosters" class="mt-6 flex min-h-0 flex-1 flex-col">
        <div class="-mx-1 flex min-h-0 flex-1 gap-4 overflow-hidden px-1 pb-3">
          <div
            v-for="index in 4"
            :key="index"
            class="flex w-[78vw] max-w-[22rem] shrink-0 lg:w-[21rem] xl:w-[23rem]"
          >
            <div class="flex w-full flex-col overflow-hidden rounded-xl border border-border bg-surface-inverse">
              <div class="p-4">
                <UiSkeletonLoader tone="inverse" width="5rem" height="1.25rem" rounded="var(--radius-full)" />
              </div>
              <div class="mt-auto flex flex-col gap-3 p-4 sm:p-5">
                <UiSkeletonLoader tone="inverse" variant="text" width="45%" height="0.75rem" />
                <UiSkeletonLoader tone="inverse" width="85%" height="1.75rem" />
                <UiSkeletonLoader tone="inverse" variant="text" :lines="3" width="70%" />
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-else class="flex flex-col">
        <div
          v-for="(rows, group) in historyGroups"
          :key="group"
          class="grid border-b border-border md:grid-cols-[6.5rem_minmax(0,1fr)] md:gap-x-6"
        >
          <div class="pt-4 md:pt-5">
            <div class="h-6 w-16 md:h-9 md:w-24">
              <UiSkeletonLoader height="100%" />
            </div>
          </div>
          <div class="divide-y divide-border">
            <div
              v-for="row in rows"
              :key="row"
              class="grid grid-cols-[7rem_minmax(0,1fr)] items-start gap-x-4 gap-y-1.5 py-4 sm:grid-cols-[12rem_minmax(0,1fr)] sm:gap-x-6 lg:grid-cols-[12rem_minmax(0,1fr)_14rem_8rem_1.5rem] lg:items-center lg:gap-y-0"
            >
              <div class="row-span-3 aspect-video lg:row-span-1">
                <UiSkeletonLoader height="100%" />
              </div>
              <div class="flex min-w-0 flex-col gap-2">
                <UiSkeletonLoader variant="text" width="40%" height="0.75rem" />
                <UiSkeletonLoader width="80%" height="1.25rem" />
                <UiSkeletonLoader variant="text" width="30%" />
              </div>
              <div class="col-start-2 flex flex-col gap-1.5 lg:col-start-auto">
                <UiSkeletonLoader variant="text" width="9rem" />
                <UiSkeletonLoader variant="text" width="7rem" height="0.75rem" />
              </div>
              <div class="col-start-2 lg:col-start-auto">
                <UiSkeletonLoader width="5rem" height="1.25rem" rounded="var(--radius-full)" />
              </div>
              <div class="hidden lg:block">
                <UiSkeletonLoader width="1.25rem" height="1.25rem" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
