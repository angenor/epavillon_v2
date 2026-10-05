<script setup lang="ts">
/**
 * La cloche de l'en-tête. La maquette Nuit y pose un point d'alerte, pas un compteur :
 * le nombre de non lues est dit par le libellé, que lit le lecteur d'écran.
 */
const props = withDefaults(defineProps<{ nonLues: number; vers?: string }>(), { vers: '/guide-nego/notifications' })

const { t } = useI18n()
const libelle = computed(() =>
  props.nonLues > 0 ? t('gn-cloche.libelle-non-lues', { count: props.nonLues }, props.nonLues) : t('gn-cloche.libelle'),
)
</script>

<template>
  <GnBoutonRond :vers="vers" picto="bell" :libelle="libelle" :pastille="nonLues > 0" />
</template>
