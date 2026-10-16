<!-- Silhouette de la page d'une activité : mêmes conteneurs, débords et points de rupture que Hero, Stage, Ticket et People, pour que rien ne saute à l'arrivée des données. -->
<template>
  <header class="relative isolate overflow-hidden bg-surface-inverse pt-6 pb-28 lg:pb-30">
    <div class="mx-auto max-w-7xl px-4 sm:px-6">
      <div class="flex min-h-11 items-center gap-x-5">
        <UiSkeletonLoader tone="inverse" variant="text" width="7rem" height="1rem" />
        <UiSkeletonLoader tone="inverse" variant="text" width="11rem" />
      </div>

      <div class="mt-8 flex flex-wrap gap-2 sm:mt-10">
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

  <div class="mx-auto max-w-[1280px] px-4 pb-16 sm:px-6">
    <div class="relative -mt-12 flex flex-col gap-10 lg:-mt-[72px] lg:grid lg:grid-cols-[minmax(0,1fr)_400px] lg:items-start lg:gap-x-8">
      <div class="contents min-w-0 lg:flex lg:flex-col lg:gap-14 lg:pr-8">
        <div class="order-1 aspect-video min-w-0 overflow-hidden rounded-lg shadow-lg lg:-mr-8">
          <UiSkeletonLoader height="100%" rounded="0" />
        </div>

        <div class="order-4">
          <div class="border-b border-text pb-3">
            <UiSkeletonLoader width="9rem" height="1.75rem" />
          </div>
          <div class="mt-5">
            <UiSkeletonLoader :lines="2" height="1.25rem" />
          </div>
          <div class="mt-5">
            <UiSkeletonLoader variant="text" :lines="5" height="1rem" />
          </div>
        </div>

        <div class="order-5">
          <div class="border-b border-text pb-3">
            <UiSkeletonLoader width="11rem" height="1.75rem" />
          </div>
          <div class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 sm:grid-cols-3">
            <div v-for="index in 3" :key="index" class="flex min-w-0 flex-col items-center gap-2">
              <div class="mb-2 size-24 sm:size-28">
                <UiSkeletonLoader variant="circle" width="100%" height="100%" />
              </div>
              <UiSkeletonLoader variant="text" width="75%" height="1.125rem" />
              <UiSkeletonLoader variant="text" width="55%" />
            </div>
          </div>
        </div>
      </div>

      <div class="contents lg:flex lg:flex-col lg:gap-10">
        <div class="order-2 overflow-hidden rounded-lg bg-surface-raised shadow-lg">
          <div class="h-1 bg-border-subtle" />
          <div class="flex flex-col p-6">
            <UiSkeletonLoader variant="text" width="9rem" height="0.75rem" />
            <div class="mt-2.5 flex items-end gap-3">
              <UiSkeletonLoader width="8.5rem" height="2.925rem" />
              <UiSkeletonLoader variant="text" width="4.5rem" />
            </div>

            <div class="mt-5 border-t border-border-subtle pt-5">
              <UiSkeletonLoader variant="text" width="6rem" height="0.75rem" />
              <div class="mt-3 grid grid-cols-4">
                <div v-for="index in 4" :key="index" class="flex flex-col items-center gap-2 border-l border-border-subtle first:border-l-0">
                  <UiSkeletonLoader width="2.75rem" height="2.75rem" />
                  <UiSkeletonLoader variant="text" width="2rem" height="0.6875rem" />
                </div>
              </div>
            </div>

            <div class="mt-5">
              <UiSkeletonLoader height="3rem" />
            </div>
            <div class="mt-4">
              <UiSkeletonLoader variant="text" width="60%" height="0.8125rem" />
            </div>
            <div class="mt-4 grid grid-cols-2 gap-2">
              <UiSkeletonLoader height="var(--target-min)" />
              <UiSkeletonLoader height="var(--target-min)" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
