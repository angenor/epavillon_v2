<script setup lang="ts">
/**
 * AFFICHAGE D'UN CONTENU RICHE — le pendant en lecture de `UiRichText`.
 *
 * MÊME FEUILLE DE STYLE, la classe `.rich-text` de `main.css` : un contenu qui
 * change d'allure entre la saisie et la publication est un contenu qu'on
 * relit deux fois. Les couleurs et la police viennent de la charte, jamais du
 * contenu — les styles en ligne qui auraient survécu à un copier-coller sont
 * neutralisés par la feuille.
 *
 * `v-html` EST ICI UN CHOIX EXPLICITE, ET IL A UNE CONDITION. Le contenu est
 * rédigé par un tiers : il n'est de confiance qu'une fois **assaini côté API**,
 * avec la liste blanche de balises de l'éditeur. Tant que l'API n'existe pas,
 * le HTML rendu ne peut venir que de `UiRichText`, qui ne produit pas de script.
 * C'est écrit ici pour que personne ne branche ce composant sur une source
 * quelconque en croyant qu'il filtre quoi que ce soit : il ne filtre rien.
 * L'assainissement est une obligation du prompt B4.
 *
 * VIDE, IL NE REND RIEN — pas un cadre vide, pas un blanc : l'appelant décide de
 * ce qu'il affiche à la place.
 */

interface Props {
  /** Fragment HTML restreint, tel que `UiRichText` le produit. */
  html: string | null | undefined
}

const props = defineProps<Props>()

// Un document vierge de ProseMirror vaut `<p></p>` : c'est du vide.
const isEmpty = computed(() => richTextToPlain(props.html).length === 0)
</script>

<template>
  <!-- eslint-disable-next-line vue/no-v-html -- voir l'en-tête : assainissement à l'API -->
  <div v-if="!isEmpty" class="rich-text" v-html="props.html" />
</template>
