<script setup lang="ts">
/**
 * Section ancrée du guide de style.
 *
 * L'ancre est indispensable : le guide fait plusieurs écrans, et on y renvoie en
 * cours de développement (« voir #composants-base »). Le titre est cliquable et
 * copie son ancre — c'est ce qui rend le renvoi possible sans chercher.
 *
 * `scroll-mt` compense l'en-tête collant : sans lui, une ancre amène le titre
 * SOUS la barre de navigation, et l'on croit avoir raté la section.
 */

interface Props {
  id: string
  title: string
  /** Un paragraphe qui dit ce que la section démontre et pourquoi. */
  description?: string
}

const props = defineProps<Props>()
</script>

<template>
  <section :id="props.id" class="scroll-mt-24 border-t border-border pt-10 first:border-t-0 first:pt-0">
    <header class="max-w-prose">
      <h2 class="group text-2xl">
        <a :href="`#${props.id}`" class="text-text no-underline hover:text-accent">
          {{ props.title }}
          <span class="ml-1 text-text-subtle opacity-0 transition-opacity group-hover:opacity-100" aria-hidden="true">#</span>
        </a>
      </h2>
      <p v-if="props.description" class="mt-2 text-text-muted">{{ props.description }}</p>
    </header>

    <div class="mt-6 space-y-8">
      <slot />
    </div>
  </section>
</template>
