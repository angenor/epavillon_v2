<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'

/**
 * « À propos de cette édition », en tête de la colonne principale. Sortie du
 * bandeau le 19/08 : sur la photo, un long paragraphe se lit mal même voilé.
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
    <h2 id="presentation-titre" class="heading-condensed text-[clamp(1.9rem,3.6vw,2.5rem)] text-heading">
      {{ t('event.public.presentation.title') }}
    </h2>

    <!-- Saisie dans l'éditeur riche du back-office : titres, listes, liens. -->
    <UiRichContent class="mt-4 max-w-[68ch] text-[1.05rem] leading-[1.75]" :html="tr(props.edition.description)" />

    <!-- `highlights` porte les consignes pratiques (accès, badge, inscription) :
         ce qu'il faut avoir lu avant de venir, d'où le cadre. -->
    <aside
      v-if="props.edition.highlights"
      class="mt-6 flex max-w-(--measure) items-start gap-3 rounded-lg border border-border border-l-4 border-l-accent bg-surface-sunken px-5 py-4"
    >
      <UiIcon name="info" size="1.1rem" class="mt-0.5 shrink-0 text-accent" />
      <p class="text-sm text-text-secondary">{{ tr(props.edition.highlights) }}</p>
    </aside>
  </section>
</template>
