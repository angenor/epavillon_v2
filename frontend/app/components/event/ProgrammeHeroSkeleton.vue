<script setup lang="ts">
// Silhouette de `EventProgrammeHero` : mêmes classes de mise en page, à tenir alignées.
const FIGURES = 3
</script>

<template>
  <!-- Comme le bandeau, `-mt-8 sm:-mt-10` le colle au menu : il reste le premier enfant de la page. -->
  <div class="full-bleed relative -mt-8 overflow-hidden bg-surface-inverse sm:-mt-10" aria-hidden="true">
    <div class="mx-auto w-full max-w-[1280px] px-4 py-8 sm:px-6 sm:py-10">
      <div class="max-w-2xl">
        <div class="flex flex-wrap items-center gap-3 text-sm">
          <div class="flex h-lh w-36 items-center">
            <UiSkeletonLoader variant="text" tone="inverse" height="0.8em" />
          </div>
          <div class="h-5 w-20">
            <UiSkeletonLoader tone="inverse" height="100%" rounded="var(--radius-sm)" />
          </div>
        </div>

        <div class="mt-3 font-display text-2xl leading-tight sm:text-4xl">
          <div class="flex h-lh items-center">
            <UiSkeletonLoader tone="inverse" height="0.8em" />
          </div>
          <div class="flex h-lh items-center">
            <UiSkeletonLoader tone="inverse" width="60%" height="0.8em" />
          </div>
        </div>

        <div class="mt-2 flex h-lh w-80 max-w-full items-center text-sm">
          <UiSkeletonLoader variant="text" tone="inverse" height="0.8em" />
        </div>

        <div class="mt-6 flex flex-wrap gap-x-8 gap-y-4">
          <div v-for="figure in FIGURES" :key="figure" class="flex flex-col">
            <div class="h-7.5 w-14">
              <UiSkeletonLoader tone="inverse" height="100%" />
            </div>
            <div class="flex h-4 w-20 items-center">
              <UiSkeletonLoader variant="text" tone="inverse" height="0.6rem" />
            </div>
          </div>

          <div class="basis-full border-t border-border-on-inverse pt-4 sm:basis-auto sm:border-t-0 sm:border-l sm:pt-0 sm:pl-8">
            <div class="flex h-4 w-24 items-center">
              <UiSkeletonLoader variant="text" tone="inverse" height="0.6rem" />
            </div>
            <div class="mt-1 flex h-5 w-36 items-center">
              <UiSkeletonLoader variant="text" tone="inverse" height="0.8rem" />
            </div>
            <div class="flex h-4 w-28 items-center">
              <UiSkeletonLoader variant="text" tone="inverse" height="0.6rem" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
