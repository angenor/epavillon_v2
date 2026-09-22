<script setup lang="ts">
import { NuxtLink } from '#components'
import { initialesDe } from '~/utils/guide-nego/compte'

/**
 * L'avatar du compte : 40 px, initiales 15/700, et l'image quand le compte en a une.
 *
 * **Une image qui ne se charge pas rend les initiales**, et pas seulement une image
 * absente : hors connexion, l'image du compte ne vient pas — elle n'a pas sa place
 * dans la garde de la coquille, qui ne porte rien de personnel. Sans ce repli, la
 * salle sans réseau montrerait un rond vide à la place d'un nom.
 *
 * Avec `vers`, il devient la cible de 48 px qui ouvre le profil ; l'image est alors
 * muette, le lien porte le nom.
 */
const props = withDefaults(
  defineProps<{
    prenom: string | null
    nom: string | null
    image?: string | null
    vers?: string
  }>(),
  { image: null, vers: undefined },
)

const { t } = useI18n()

const initiales = computed(() => initialesDe(props.prenom, props.nom))
const nomComplet = computed(() => [props.prenom, props.nom].filter(Boolean).join(' '))
const libelle = computed(() =>
  nomComplet.value ? t('gn-avatar.profil-de', { nom: nomComplet.value }) : t('gn-avatar.profil'),
)

const imageEnEchec = ref(false)
watch(() => props.image, () => (imageEnEchec.value = false))
const imageMontree = computed(() => Boolean(props.image) && !imageEnEchec.value)
</script>

<template>
  <component
    :is="vers ? NuxtLink : 'span'"
    :to="vers"
    class="gn-avatar"
    :class="{ 'gn-avatar--lien': vers }"
    :aria-label="vers ? libelle : undefined"
  >
    <span class="gn-avatar__rond" aria-hidden="true">
      <img v-if="imageMontree" :src="image ?? undefined" alt="" class="gn-avatar__image" @error="imageEnEchec = true">
      <template v-else-if="initiales">{{ initiales }}</template>
      <GnPicto v-else nom="user" :taille="20" />
    </span>
  </component>
</template>

<style>
[data-app="guide-nego"] .gn-avatar {
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  text-decoration: none;
}

[data-app="guide-nego"] .gn-avatar--lien {
  width: var(--gn-cible);
  height: var(--gn-cible);
  /* La cible déborde à gauche comme le bouton retour : le rond s'aligne sur la marge. */
  margin-inline-start: -4px;
}

[data-app="guide-nego"] .gn-avatar--lien:active .gn-avatar__rond {
  outline: var(--gn-filet-2) solid var(--gn-filet-fort);
  outline-offset: 2px;
}

[data-app="guide-nego"] .gn-avatar__rond {
  width: var(--gn-avatar);
  height: var(--gn-avatar);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: var(--gn-rayon-24);
  background: var(--gn-titre);
  color: var(--gn-sur-titre);
  font-size: var(--gn-taille-15);
  line-height: 1;
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-avatar__image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
</style>
