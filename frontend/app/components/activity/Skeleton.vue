<!-- Silhouette de la page d'une activité : mêmes conteneurs, débords et points de rupture que Hero, Ticket, Stage et People (billet d'abord sur téléphone), pour que rien ne saute à l'arrivée des données. -->
<template>
  <header class="relative isolate overflow-hidden bg-surface-inverse pt-6 pb-9 lg:pb-30">
    <div class="mx-auto max-w-7xl px-4 sm:px-6">
      <div class="flex min-h-11 items-center gap-x-5">
        <UiSkeletonLoader tone="inverse" variant="text" width="7rem" height="1rem" />
        <UiSkeletonLoader tone="inverse" variant="text" width="11rem" />
      </div>

      <div class="mt-5 flex flex-wrap gap-2">
        <UiSkeletonLoader tone="inverse" width="7.5rem" height="2.25rem" rounded="var(--radius-full)" />
        <UiSkeletonLoader tone="inverse" width="6rem" height="2.25rem" rounded="var(--radius-full)" />
      </div>

      <!-- Une ligne de plus sur téléphone : à 2rem sur 343 px, un titre ordinaire en prend trois. -->
      <div class="mt-5.5 flex max-w-280 flex-col">
        <div class="h-[2.2rem] py-1.5 sm:h-[2.9rem] sm:py-2 lg:h-[3.575rem] lg:py-2.5">
          <UiSkeletonLoader tone="inverse" width="96%" height="100%" />
        </div>
        <div class="h-[2.2rem] py-1.5 sm:h-[2.9rem] sm:py-2 lg:h-[3.575rem] lg:py-2.5">
          <UiSkeletonLoader tone="inverse" width="72%" height="100%" />
        </div>
        <div class="h-[2.2rem] py-1.5 sm:hidden">
          <UiSkeletonLoader tone="inverse" width="48%" height="100%" />
        </div>
      </div>

      <div class="mt-5.5 flex flex-wrap items-center gap-x-8 gap-y-3">
        <div class="flex h-6 items-center gap-2.5">
          <UiSkeletonLoader tone="inverse" variant="circle" width="1.125rem" height="1.125rem" />
          <UiSkeletonLoader tone="inverse" variant="text" width="10rem" height="1rem" />
        </div>
        <div class="flex h-6 items-center gap-2.5">
          <UiSkeletonLoader tone="inverse" variant="circle" width="1.125rem" height="1.125rem" />
          <UiSkeletonLoader tone="inverse" variant="text" width="13rem" height="1rem" />
        </div>
      </div>

      <div class="mt-6 flex items-center gap-4">
        <UiSkeletonLoader tone="inverse" width="7rem" height="4rem" rounded="var(--radius-lg)" />
        <div class="flex min-w-0 flex-1 flex-col gap-2">
          <UiSkeletonLoader tone="inverse" variant="text" width="5rem" height="0.75rem" />
          <UiSkeletonLoader tone="inverse" variant="text" width="min(14rem, 100%)" height="1.0625rem" />
        </div>
      </div>
    </div>
  </header>

  <div class="mx-auto max-w-7xl px-4 pb-16 sm:px-6">
    <div class="grid grid-cols-1 gap-7 pt-6 lg:grid-cols-[minmax(0,1fr)_380px] lg:items-start lg:gap-10 lg:pt-0">
      <div class="relative z-1 overflow-hidden rounded-lg bg-surface-raised shadow-lg lg:col-start-2 lg:row-start-1 lg:-mt-18">
        <div class="flex items-center gap-4 bg-border-subtle px-6 py-4">
          <UiSkeletonLoader width="3.5rem" height="3.2rem" />
          <div class="flex flex-col gap-1.5">
            <UiSkeletonLoader variant="text" width="6rem" height="1rem" />
            <UiSkeletonLoader variant="text" width="8rem" height="1rem" />
          </div>
        </div>
        <div class="flex flex-col px-6 pt-4.5 pb-5.5">
          <UiSkeletonLoader width="11rem" height="2.4rem" />
          <div class="mt-2.5">
            <UiSkeletonLoader variant="text" width="9rem" height="0.875rem" />
          </div>

          <div class="mt-4.5 border-t border-border-subtle pt-3.5">
            <UiSkeletonLoader variant="text" width="6rem" height="1rem" />
            <div class="mt-2 grid grid-cols-4">
              <div v-for="index in 4" :key="index" class="flex flex-col items-center gap-1.5 border-l border-border-subtle first:border-l-0">
                <UiSkeletonLoader width="2.5rem" height="2.5rem" />
                <UiSkeletonLoader variant="text" width="2rem" height="0.6875rem" />
              </div>
            </div>
          </div>

          <div class="mt-4">
            <UiSkeletonLoader height="3rem" />
          </div>
          <div class="mt-4 grid grid-cols-2 gap-2.5">
            <UiSkeletonLoader height="var(--target-min)" />
            <UiSkeletonLoader height="var(--target-min)" />
          </div>
        </div>
      </div>

      <div class="relative z-1 flex min-w-0 flex-col gap-13 lg:col-start-1 lg:row-start-1 lg:-mt-18">
        <div class="aspect-video min-w-0 overflow-hidden rounded-lg shadow-lg">
          <UiSkeletonLoader height="100%" rounded="0" />
        </div>

        <div>
          <UiSkeletonLoader width="9rem" height="2.2rem" />
          <div class="mt-4.5">
            <UiSkeletonLoader variant="text" :lines="5" height="1rem" />
          </div>
        </div>

        <div>
          <UiSkeletonLoader width="13rem" height="2.2rem" />
          <div class="mt-4.5 grid grid-cols-1 gap-3.5 sm:grid-cols-2">
            <div v-for="index in 2" :key="index" class="flex items-center gap-3.5 rounded-lg border border-border-subtle p-3.5">
              <UiSkeletonLoader variant="circle" width="3.5rem" height="3.5rem" />
              <div class="flex min-w-0 flex-1 flex-col gap-1.5">
                <UiSkeletonLoader variant="text" width="70%" height="1.125rem" />
                <UiSkeletonLoader variant="text" width="50%" />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
