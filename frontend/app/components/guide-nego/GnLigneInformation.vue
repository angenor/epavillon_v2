<script setup lang="ts">
import { NuxtLink } from '#components'

/**
 * La règle du lieu, en une ligne bordée : « Canal modéré par l'IFDD… », « Seules
 * les personnes qui l'ont accepté figurent ici ». Elle remplace aussi la saisie
 * sous une annonce, là où l'on ne répond pas.
 *
 * Sa sortie est nommée pour elle-même (« Écrire à l'IFDD ») et décrite par la
 * phrase : lue hors contexte, elle reste compréhensible.
 */
defineProps<{ texte: string; sortie?: string; vers?: string }>()

const idTexte = useId()
</script>

<template>
  <div class="gn-ligne-info">
    <GnPicto nom="info" :taille="20" class="gn-ligne-info__picto" />
    <span :id="idTexte" class="gn-ligne-info__texte">{{ texte }}</span>
    <NuxtLink
      v-if="sortie && vers"
      :to="vers"
      class="gn-ligne-info__sortie"
      :aria-describedby="idTexte"
    >
      {{ sortie }}
    </NuxtLink>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-ligne-info {
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  padding-block: var(--gn-espace-8);
  padding-inline: var(--gn-espace-12);
  border: var(--gn-filet-1) solid var(--gn-filet);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-ligne-info__picto {
  color: var(--gn-picto-secondaire);
}

[data-app="guide-nego"] .gn-ligne-info__texte {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-ligne-info__sortie {
  flex: none;
  min-height: var(--gn-cible);
  display: inline-flex;
  align-items: center;
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-ligne-info__sortie:active {
  background: var(--gn-presse);
}
</style>
