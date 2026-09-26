<script setup lang="ts">
import type { PathwayStep } from '~/types/negotiation-savoir'

/**
 * Une étape du parcours : la ligne à cocher de `GnCase`, alignée en haut pour les
 * libellés sur deux lignes, et sous elle la sortie « Lire : … ». Le lien vit hors du
 * libellé de la case : il ne cocherait pas, mais le lecteur d'écran l'annoncerait
 * dans le nom de la case.
 */
const props = withDefaults(
  defineProps<{
    etape: PathwayStep
    cochee: boolean
    /** Où mène « Lire : … » ; nul, l'étape n'en porte pas. */
    vers?: string | null
    /** Ce qu'on lit, quand l'IFDD n'a pas nommé le lien. */
    cible?: string | null
    derniere?: boolean
  }>(),
  { vers: null, cible: null, derniere: false },
)
const emit = defineEmits<{ basculer: [] }>()

const { t } = useI18n()

const lien = computed(() => {
  const nom = props.etape.link?.label ?? props.cible
  return props.vers && nom ? t('gn-etape-parcours.lire', { cible: nom }) : null
})
</script>

<template>
  <div class="gn-etape" :class="{ 'gn-etape--derniere': derniere }">
    <GnCase
      :model-value="cochee"
      :libelle="etape.label"
      :detail="etape.detail ?? undefined"
      :origine="etape.origin_label ?? undefined"
      attenue
      derniere
      @update:model-value="emit('basculer')"
    />
    <NuxtLink v-if="lien && vers" :to="vers" class="gn-etape__lire">{{ lien }}</NuxtLink>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-etape {
  display: flex;
  flex-direction: column;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-etape--derniere {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-etape .gn-case {
  align-items: flex-start;
  padding-block: 14px;
}

[data-app="guide-nego"] .gn-etape .gn-case__carre {
  margin-block-start: 1px;
}

/* Aligné sur le libellé, sous la case ; cible pleine hauteur malgré ses 15 px. */
[data-app="guide-nego"] .gn-etape__lire {
  align-self: flex-start;
  min-block-size: var(--gn-cible);
  margin-block-start: calc(-1 * (var(--gn-espace-12) + var(--gn-espace-8)));
  margin-inline-start: calc(var(--gn-case) + var(--gn-espace-12));
  display: inline-flex;
  align-items: center;
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}
</style>
