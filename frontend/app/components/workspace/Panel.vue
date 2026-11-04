<script setup lang="ts">
/**
 * La carte qui remonte sur le bandeau de l'espace organisation (direction
 * condensée, 04/11) : seule surface élevée de la page, le reste est posé sur des filets.
 */

interface Props {
  title?: string
  /** À droite du titre : un compte, un numéro de dossier, un fuseau. */
  meta?: string
  headingId?: string
}

const props = withDefaults(defineProps<Props>(), { title: '', meta: '', headingId: undefined })
</script>

<template>
  <section
    class="rounded-lg bg-surface-raised px-4 py-4.5 shadow-lg sm:p-6"
    :aria-labelledby="props.title && props.headingId ? props.headingId : undefined"
  >
    <div v-if="props.title" class="mb-4 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
      <h2 :id="props.headingId" class="heading-condensed text-[2rem] text-heading">{{ props.title }}</h2>
      <p v-if="props.meta" class="text-sm text-text-muted">{{ props.meta }}</p>
    </div>
    <slot />
  </section>
</template>
