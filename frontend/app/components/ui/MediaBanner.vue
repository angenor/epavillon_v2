<script setup lang="ts">
import type { AttachedImage } from '~/types/media'

/**
 * Le bandeau d'une page de la programmation (activité, journée spéciale) : photo
 * voilée, ou aplat bleu nuit sans photo, fil d'Ariane, puis le contenu de la page.
 * Le bas garde 72 px de marge de plus : le billet et l'image de la page remontent dessus.
 *
 * Il dit à ses pastilles (`UiBannerPill`) s'il porte une photo : le verre n'est
 * permis que sur un média.
 */

interface Props {
  image?: AttachedImage | null
  backTo: string
  backLabel: string
  /** Suite du fil d'Ariane, en texte. */
  trail?: string
}

const props = withDefaults(defineProps<Props>(), { image: null, trail: '' })

const { t } = useI18n()

provide(MEDIA_BANNER_ON_MEDIA, computed(() => props.image !== null))
</script>

<template>
  <header
    class="relative isolate overflow-hidden bg-surface-inverse pt-6 pb-28 text-text-on-inverse [--color-focus:var(--color-accent-on-inverse)] lg:pb-30"
  >
    <template v-if="props.image">
      <UiImage
        :image="props.image"
        ratio="auto"
        loading="eager"
        sizes="100vw"
        frame-class="size-full"
        class="absolute inset-0 -z-10"
        aria-hidden="true"
      />
      <div class="absolute inset-0 -z-10 bg-scrim/50" aria-hidden="true" />
      <div class="scrim-fade-top absolute inset-0 -z-10" aria-hidden="true" />
    </template>

    <div class="mx-auto max-w-7xl px-4 sm:px-6">
      <nav :aria-label="t('nav.breadcrumb.label')" class="flex flex-wrap items-center gap-x-5 text-sm">
        <NuxtLink
          :to="props.backTo"
          class="inline-flex min-h-11 items-center gap-2 text-[0.9375rem] font-bold text-text-on-inverse hover:text-text-on-inverse hover:underline"
        >
          <UiIcon name="arrow-left" size="1rem" :stroke-width="2" />
          {{ props.backLabel }}
        </NuxtLink>
        <span v-if="props.trail" class="min-w-0 text-text-on-inverse-muted">{{ props.trail }}</span>
      </nav>

      <slot />
    </div>
  </header>
</template>
