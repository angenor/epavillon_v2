<script setup lang="ts">
import { expressionRepliee, LONGUEUR_MINIMALE, type Passage } from '~/utils/guide-nego/lecteur'

/**
 * La recherche dans le document — maquette 04 · 05. Elle se fait sur le téléphone : la
 * même, avec ou sans réseau (FR-041). Le passage de la page en cours dit « vous êtes ici ».
 */
const props = defineProps<{
  passages: Passage[]
  /** Les pages cherchées : celles du document. */
  pages: number
  /** Le rang du passage de la page en cours ; -1 s'il n'y en a pas. */
  ici: number
}>()

defineEmits<{ aller: [rang: number] }>()

const expression = defineModel<string>('expression', { required: true })

const { t } = useI18n()

// Au-delà, la liste ne se lit plus et le téléphone peine à la dessiner.
const LISTES = 200

const pages = computed(() => t('gn-lecteur-recherche.pages', { count: props.pages }, props.pages))
const compte = computed(() => {
  const q = expression.value.trim()
  if (!q) return null
  if (expressionRepliee(q).length < LONGUEUR_MINIMALE) return t('gn-lecteur-recherche.trop-court')
  const n = props.passages.length
  return n
    ? t('gn-lecteur-recherche.compte', { passages: t('gn-lecteur-recherche.passages', { count: n }, n), pages: pages.value })
    : t('gn-lecteur-recherche.aucun', { expression: q, pages: pages.value })
})
const listes = computed(() => props.passages.slice(0, LISTES))
</script>

<template>
  <div class="gn-lecteur-recherche">
    <GnChampRecherche v-model="expression" :libelle="t('gn-lecteur-recherche.libelle')" />
    <p v-if="compte" class="gn-lecteur-recherche__compte" role="status">{{ compte }}</p>
    <ul v-if="passages.length" class="gn-lecteur-recherche__liste">
      <li v-for="(passage, rang) in listes" :key="`${passage.page}-${passage.bloc}-${passage.champ}-${passage.debut}`">
        <button type="button" class="gn-lecteur-recherche__passage" @click="$emit('aller', rang)">
          <span class="gn-lecteur-recherche__corps">
            <span class="gn-lecteur-recherche__lieu">
              {{ passage.section
                ? t('gn-lecteur-recherche.lieu', { page: passage.etiquette, section: passage.section })
                : t('gn-lecteur-recherche.page', { page: passage.etiquette }) }}
              <template v-if="rang === ici">{{ t('gn-lecteur-recherche.ici') }}</template>
            </span>
            <span class="gn-lecteur-recherche__extrait">
              {{ passage.extrait.avant }}<mark>{{ passage.extrait.trouve }}</mark>{{ passage.extrait.apres }}
            </span>
          </span>
          <GnPicto nom="chevron" :taille="24" class="gn-lecteur-recherche__chevron" />
        </button>
      </li>
    </ul>
    <p v-if="passages.length > LISTES" class="gn-lecteur-recherche__compte">
      {{ t('gn-lecteur-recherche.affiner', { n: LISTES }) }}
    </p>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-lecteur-recherche {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-lecteur-recherche__compte {
  padding-block: var(--gn-espace-8) 10px;
  border-bottom: var(--gn-filet-3) solid var(--gn-filet-fort);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-lecteur-recherche__passage {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  inline-size: 100%;
  min-height: var(--gn-cible);
  padding: 10px 0;
  border: none;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  background: none;
  color: var(--gn-texte);
  font: inherit;
  text-align: start;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-lecteur-recherche__passage:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-lecteur-recherche__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

[data-app="guide-nego"] .gn-lecteur-recherche__lieu {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-lecteur-recherche__extrait {
  font-size: var(--gn-taille-17);
  line-height: 1.4;
  overflow-wrap: break-word;
}

[data-app="guide-nego"] .gn-lecteur-recherche__extrait mark {
  padding-inline: 2px;
  background: var(--gn-fond-2);
  color: var(--gn-texte);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-lecteur-recherche__chevron {
  flex: none;
  color: var(--gn-picto-secondaire);
}
</style>
