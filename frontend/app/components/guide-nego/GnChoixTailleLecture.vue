<script setup lang="ts">
import type { SegmentDeChoix } from '~/components/guide-nego/GnSegmente.vue'
import type { TailleDuTexte } from '~/utils/guide-nego/appareil-lecture'

/** La taille du texte des documents — un seul réglage, dans le lecteur comme dans le profil. */
defineProps<{ libelle: string }>()

const { t } = useI18n()
const { taille, choisir, TAILLES } = useGnTailleDeLecture()

const NOMS: Record<TailleDuTexte, string> = { 17: 'normale', 20: 'grande', 24: 'tres-grande' }

const segments = computed<SegmentDeChoix[]>(() =>
  TAILLES.map((valeur) => ({ valeur: String(valeur), libelle: t(`gn-choix-taille-lecture.${NOMS[valeur]}`) })),
)
const choisie = computed({
  get: () => String(taille.value),
  set: (valeur: string) => choisir(Number(valeur) as TailleDuTexte),
})
</script>

<template>
  <GnSegmente v-model="choisie" :segments="segments" :libelle="libelle" />
</template>
