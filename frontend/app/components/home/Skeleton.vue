<script setup lang="ts">
// Silhouette de l'accueil : mêmes conteneurs, classes de mise en page et points de
// rupture que `HomeShowcase`, `HomeAsidePanel`, `HomeCallSection` et
// `HomeEditionHistory`. Une classe de mise en page changée là-bas se reporte ici,
// sinon la page saute à l'arrivée des données. Les temps forts, chargés à part,
// n'ont pas de silhouette : la section apparaît quand elle est prête.

// Même témoin que `HomeEditionHistory` : la silhouette suit la vue choisie par le visiteur.
const viewCookie = useCookie<string | null>('ep-editions-vue', { readonly: true })
const isPosters = computed(() => viewCookie.value === 'posters')
</script>

<template>
  <div class="flex flex-col gap-16 lg:gap-20">
    <!-- Vitrine : citation à gauche, panneau de 400 px à droite sur `lg`, dessous en deçà. -->
    <section class="full-bleed relative -mt-8 grid overflow-hidden bg-surface-inverse sm:-mt-10">
      <div class="relative flex min-h-[min(720px,calc(100svh-var(--nav-height)-var(--topbar-height)))] flex-col justify-end lg:[grid-area:1/1]">
        <div class="mx-auto w-full max-w-[1280px] px-4 pt-10 pb-8 sm:px-6 lg:px-8">
          <div class="flex flex-col gap-4 pb-16 lg:pe-[calc(400px+3rem)] lg:pb-[6.5rem]">
            <UiSkeletonLoader tone="inverse" width="7rem" height="1.75rem" rounded="var(--radius-full)" />
            <UiSkeletonLoader tone="inverse" variant="text" :lines="3" width="min(100%, 34rem)" height="3rem" />
            <UiSkeletonLoader tone="inverse" variant="text" width="10rem" height="1.25rem" />
            <UiSkeletonLoader tone="inverse" variant="text" width="16rem" />
          </div>
        </div>
      </div>

      <div class="relative z-20 lg:[grid-area:1/1]">
        <div class="mx-auto w-full max-w-[1280px] px-4 py-6 sm:px-6 lg:flex lg:h-full lg:items-end lg:justify-end lg:px-8 lg:pt-14 lg:pb-8">
          <div class="flex flex-col gap-4 rounded-xl border border-border-on-inverse bg-surface-inverse-raised p-5 sm:p-6 lg:w-[400px]">
            <UiSkeletonLoader tone="inverse" width="14rem" height="1.6rem" />
            <div v-for="row in 3" :key="row" class="grid grid-cols-[4rem_minmax(0,1fr)] gap-3">
              <UiSkeletonLoader tone="inverse" width="3.5rem" height="1.6rem" />
              <UiSkeletonLoader tone="inverse" variant="text" :lines="2" width="100%" />
            </div>
            <UiSkeletonLoader tone="inverse" height="var(--target-min)" />
          </div>
        </div>
      </div>
    </section>

    <!-- Appel à propositions : aplat bleu nuit, photographie à droite sur `lg`, dessus en deçà. -->
    <section>
      <div class="grid overflow-hidden rounded-xl bg-surface-inverse lg:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)]">
        <div class="min-h-52 lg:order-2" />
        <div class="flex flex-col gap-4 px-5 py-7 sm:px-8 lg:py-11 lg:ps-11">
          <UiSkeletonLoader tone="inverse" width="12rem" height="1.5rem" rounded="var(--radius-full)" />
          <UiSkeletonLoader tone="inverse" width="min(100%, 24rem)" height="3rem" />
          <UiSkeletonLoader tone="inverse" variant="text" :lines="3" />
          <UiSkeletonLoader tone="inverse" width="14rem" height="var(--target-min)" />
        </div>
      </div>
    </section>

    <!-- Les éditions : en-tête, puis liste sous un trait épais ou rail d'affiches. -->
    <section class="flex flex-col" :class="{ 'min-h-[calc(100svh-var(--nav-height))]': isPosters }">
      <div class="mb-8 flex flex-wrap items-end justify-between gap-4">
        <div class="flex min-w-0 flex-1 flex-col gap-2">
          <UiSkeletonLoader width="14rem" height="2.5rem" />
          <UiSkeletonLoader variant="text" width="min(100%, 30rem)" height="1rem" />
        </div>
        <div class="flex min-h-(--target-min) min-w-0 items-center gap-6 overflow-hidden">
          <UiSkeletonLoader v-for="index in 4" :key="index" variant="text" width="4.5rem" class="shrink-0" />
        </div>
      </div>

      <div v-if="isPosters" class="-mx-1 flex min-h-0 flex-1 gap-4 overflow-hidden px-1 pb-3">
        <div v-for="index in 4" :key="index" class="flex w-[78vw] max-w-[22rem] shrink-0 lg:w-[21rem] xl:w-[23rem]">
          <div class="w-full rounded-xl bg-surface-inverse" />
        </div>
      </div>

      <div v-else class="border-t-2 border-text">
        <div
          v-for="row in 3"
          :key="row"
          class="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-x-4 gap-y-2.5 border-b border-border-subtle py-5 sm:grid-cols-[6rem_minmax(0,1fr)] lg:grid-cols-[7.5rem_12.5rem_minmax(0,1fr)_13.75rem] lg:gap-6"
        >
          <UiSkeletonLoader width="4.5rem" height="2.5rem" />
          <div class="hidden aspect-video lg:block">
            <UiSkeletonLoader height="100%" />
          </div>
          <div class="flex min-w-0 flex-col gap-2">
            <UiSkeletonLoader variant="text" width="40%" height="0.75rem" />
            <UiSkeletonLoader width="80%" height="1.5rem" />
          </div>
          <div class="col-start-2 flex flex-col gap-1.5 lg:col-start-auto">
            <UiSkeletonLoader variant="text" width="9rem" />
            <UiSkeletonLoader variant="text" width="7rem" height="0.75rem" />
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
