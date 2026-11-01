<script setup lang="ts">
// Silhouette de `EventProgrammeHero` : mêmes classes de mise en page, à tenir alignées.
const FIGURES = 3
</script>

<template>
  <!-- Comme le bandeau, `-mt-8 sm:-mt-10` le colle au menu : il reste le premier enfant de la page. -->
  <div class="full-bleed relative -mt-8 overflow-hidden bg-surface-inverse sm:-mt-10" aria-hidden="true">
    <div class="mx-auto w-full max-w-[1280px] px-4 pt-10 pb-9 sm:px-6">
      <div class="max-w-3xl">
        <div class="flex flex-wrap items-center gap-3 text-sm">
          <div class="flex h-lh w-36 items-center">
            <UiSkeletonLoader variant="text" tone="inverse" height="0.8em" />
          </div>
          <div class="h-5 w-20">
            <UiSkeletonLoader tone="inverse" height="100%" rounded="var(--radius-sm)" />
          </div>
        </div>

        <div class="mt-3.5 max-w-[34ch] font-condensed text-[clamp(2.1rem,3.6vw,3.1rem)] leading-none">
          <div class="flex h-lh items-center">
            <UiSkeletonLoader tone="inverse" height="0.8em" />
          </div>
          <div class="flex h-lh items-center">
            <UiSkeletonLoader tone="inverse" width="60%" height="0.8em" />
          </div>
        </div>

        <div class="mt-3.5 flex h-lh w-96 max-w-full items-center text-[0.95rem]">
          <UiSkeletonLoader variant="text" tone="inverse" height="0.8em" />
        </div>

        <div class="mt-5.5 flex flex-wrap items-end gap-x-8 gap-y-3">
          <div v-for="figure in FIGURES" :key="figure" class="flex flex-col">
            <div class="h-[2.4rem] w-14">
              <UiSkeletonLoader tone="inverse" height="100%" />
            </div>
            <div class="flex h-5 w-20 items-center">
              <UiSkeletonLoader variant="text" tone="inverse" height="0.6rem" />
            </div>
          </div>

          <div class="basis-full text-[0.82rem] sm:basis-auto sm:border-l sm:border-border-on-inverse sm:pl-6">
            <div class="flex h-lh w-44 items-center">
              <UiSkeletonLoader variant="text" tone="inverse" height="0.7em" />
            </div>
            <div class="flex h-lh w-24 items-center">
              <UiSkeletonLoader variant="text" tone="inverse" height="0.7em" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
