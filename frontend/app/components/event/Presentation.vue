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
    <h2 id="presentation-titre" class="font-display text-2xl sm:text-3xl">
      {{ t('event.public.presentation.title') }}
    </h2>

    <!-- La description est saisie dans l'éditeur riche du back-office : titres,
         listes, liens. Rendue comme du texte, elle affichait ses balises. -->
    <UiRichContent class="mt-5 text-lg leading-relaxed" :html="tr(props.edition.description)" />

    <!-- `highlights` porte les consignes pratiques — accès, badge, inscription.
         D'où le cadre et l'icône : ce n'est pas la suite de la description, c'est
         ce qu'il faut avoir lu avant de venir. -->
    <aside
      v-if="props.edition.highlights"
      class="mt-6 flex items-start gap-3 rounded-xl border border-border border-l-4 border-l-accent bg-surface-sunken px-5 py-4"
      :style="{ maxWidth: 'var(--measure)' }"
    >
      <UiIcon name="info" size="1.1rem" class="mt-0.5 shrink-0 text-accent" />
      <p class="text-sm text-text-secondary">{{ tr(props.edition.highlights) }}</p>
    </aside>
  </section>
</template>
