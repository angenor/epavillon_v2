<script setup lang="ts">
import { NuxtLink } from '#components'
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'

/**
 * Le bouton rond de 44 de la maquette : loupe, cloche, filtre, retour, « Aa ».
 * Un lien avec `vers`, un bouton sinon. Le libellé est celui que lit le lecteur d'écran ;
 * le contenu visible est le pictogramme, ou l'emplacement par défaut (« Aa »).
 */
const props = withDefaults(
  defineProps<{
    libelle: string
    picto?: NomDePicto
    vers?: string
    /** L'écran courant (`aria-current`) ou un filtre posé : le bouton passe à l'accent. */
    actif?: boolean
    /** Le point d'alerte de la cloche : quelque chose de nouveau, le nombre est dans le libellé. */
    pastille?: boolean
    /** Bordé plutôt que plein : sur un bloc, le fond du bouton s'y confondrait. */
    borde?: boolean
  }>(),
  { picto: undefined, vers: undefined, actif: false, pastille: false, borde: false },
)

const emit = defineEmits<{ clic: [] }>()
</script>

<template>
  <component
    :is="props.vers ? NuxtLink : 'button'"
    :to="props.vers"
    :type="props.vers ? undefined : 'button'"
    class="gn-bouton-rond"
    :class="{ 'gn-bouton-rond--actif': actif, 'gn-bouton-rond--borde': borde }"
    :aria-label="libelle"
    :aria-current="props.vers && actif ? 'page' : undefined"
    :aria-pressed="!props.vers && actif ? true : undefined"
    @click="props.vers ? undefined : emit('clic')"
  >
    <slot><GnPicto v-if="picto" :nom="picto" :taille="20" /></slot>
    <span v-if="pastille" class="gn-bouton-rond__pastille" aria-hidden="true" />
  </component>
</template>

<style>
[data-app="guide-nego"] .gn-bouton-rond {
  position: relative;
  flex: none;
  width: var(--gn-bouton-rond);
  height: var(--gn-bouton-rond);
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--gn-rayon-22);
  background: var(--gn-fond-2);
  color: var(--gn-texte);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-15);
  font-weight: var(--gn-graisse-gras);
  line-height: 1;
  text-decoration: none;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-bouton-rond:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-bouton-rond--borde {
  background: transparent;
  border: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-bouton-rond--actif {
  background: var(--gn-accent);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"] .gn-bouton-rond__pastille {
  position: absolute;
  top: 10px;
  right: 11px;
  width: 8px;
  height: 8px;
  border-radius: 4px;
  background: var(--gn-danger);
}
</style>
