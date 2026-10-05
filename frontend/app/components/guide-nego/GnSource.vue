<script setup lang="ts">
import type { KnowledgeSource } from '~/types/negotiation-savoir'
import { destinationDe } from '~/utils/guide-nego/faq'
import { NuxtLink } from '#components'

/**
 * Une source de la FAQ. Avec une citation : le passage en retrait, filet à gauche, et
 * sa référence dessous. Sans : une ligne de document. Un document s'ouvre dans le
 * lecteur à la page citée ; une référence extérieure, à son adresse si elle en a une.
 */
const props = defineProps<{ source: KnowledgeSource }>()

const { t } = useI18n()

const titre = computed(() => props.source.document_title ?? props.source.external_title ?? '')
const pages = computed(() => {
  const { page_from: de, page_to: a } = props.source
  if (de === undefined) return null
  return a !== undefined && a !== de ? t('gn-source.pages', { de, a }) : t('gn-source.page', { page: de })
})
const endroit = computed(() => [props.source.section_label, pages.value].filter(Boolean).join(', '))
const reference = computed(() => [titre.value, endroit.value].filter(Boolean).join(', '))
const destination = computed(() => destinationDe(props.source))
const interne = computed(() => (destination.value && 'interne' in destination.value ? destination.value.interne : null))
const externe = computed(() => (destination.value && 'externe' in destination.value ? destination.value.externe : null))
const balise = computed(() => (interne.value ? NuxtLink : externe.value ? 'a' : null))
const attributs = computed(() =>
  interne.value
    ? { to: interne.value }
    : externe.value
      ? { href: externe.value, target: '_blank', rel: 'noopener noreferrer' }
      : {},
)
</script>

<template>
  <figure v-if="source.quote" class="gn-source gn-source--citation">
    <blockquote class="gn-source__passage">« {{ source.quote }} »</blockquote>
    <figcaption>
      <component :is="balise" v-if="balise" v-bind="attributs" class="gn-source__reference gn-source__reference--lien">
        {{ reference }}
        <GnPicto :nom="externe ? 'external' : 'chevron'" :taille="20" />
        <span v-if="externe" class="gn-hors-ecran">{{ t('gn-source.nouvel-onglet') }}</span>
      </component>
      <span v-else class="gn-source__reference">{{ reference }}</span>
    </figcaption>
  </figure>

  <component :is="balise ?? 'div'" v-else v-bind="attributs" class="gn-source gn-source--ligne">
    <GnPicto nom="doc" :taille="20" class="gn-source__picto" />
    <span class="gn-source__corps">
      <span class="gn-source__titre">{{ titre }}</span>
      <span v-if="endroit" class="gn-source__endroit">{{ endroit }}</span>
    </span>
    <GnPicto v-if="balise" :nom="externe ? 'external' : 'chevron'" :taille="20" class="gn-source__chevron" />
    <span v-if="externe" class="gn-hors-ecran">{{ t('gn-source.nouvel-onglet') }}</span>
  </component>
</template>

<style>
/* La citation est un bloc, plus un filet en marge : le passage sur le fond relevé, sa référence dessous. */
[data-app="guide-nego"] .gn-source--citation {
  margin: var(--gn-espace-12) 0 0;
  padding: var(--gn-espace-16) var(--gn-espace-16) var(--gn-espace-4);
  border-radius: var(--gn-rayon-20);
  background: var(--gn-fond-2);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-source__passage {
  margin: 0;
  color: var(--gn-texte-lecture);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-source__reference {
  display: block;
  padding-block-end: var(--gn-espace-12);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-source__reference--lien {
  min-height: var(--gn-bouton-rond);
  padding-block-end: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--gn-espace-8);
  color: var(--gn-accent);
  text-decoration: none;
}

/* La ligne-carte de « Derniers consultés » (maquette 03). */
[data-app="guide-nego"] .gn-source--ligne {
  min-height: var(--gn-ligne-reglage);
  padding: var(--gn-espace-8) var(--gn-espace-16);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border-radius: var(--gn-rayon-16);
  background: var(--gn-fond-2);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-source--ligne + .gn-source--ligne {
  margin-block-start: 6px;
}

[data-app="guide-nego"] a.gn-source--ligne:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-source__picto {
  flex: none;
  color: var(--gn-picto-document);
}

[data-app="guide-nego"] .gn-source__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-source__titre {
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-source__endroit {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-source__chevron {
  flex: none;
  color: var(--gn-picto-secondaire);
}
</style>
