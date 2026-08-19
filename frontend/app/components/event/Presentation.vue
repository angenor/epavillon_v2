<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'

/**
 * Sortie de `EventHero` le 19/08 : sous le bandeau, elle repoussait la frise
 * des échéances de deux cents pixels, alors que c'est la frise qui répond à la
 * question pour laquelle on vient. Placée après l'encart d'appel.
 */

interface Props {
  edition: EventEdition
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
</script>

<template>
  <section aria-labelledby="presentation-titre">
    <h2 id="presentation-titre" class="font-display text-xl">
      {{ t('event.public.presentation.title') }}
    </h2>

    <p class="mt-4 text-lg text-text-secondary" :style="{ maxWidth: 'var(--measure)' }">
      {{ tr(props.edition.description) }}
    </p>

    <!-- `highlights` porte les consignes pratiques — accès, badge, inscription.
         D'où le filet : ce n'est pas la suite de la description. -->
    <p
      v-if="props.edition.highlights"
      class="mt-4 rounded-md border-l-4 border-accent bg-surface-sunken px-4 py-3 text-sm text-text-secondary"
      :style="{ maxWidth: 'var(--measure)' }"
    >
      {{ tr(props.edition.highlights) }}
    </p>
  </section>
</template>
