<script setup lang="ts">
/**
 * Six secondes, puis il s'efface : il ne porte donc JAMAIS seul une information —
 * ce qu'il annonce se lit aussi ailleurs, sur l'écran ou dans l'en-tête.
 *
 * Il se téléporte dans `#gn-portail`, qui vit sous la borne du design : dans `<body>`,
 * il n'aurait ni jetons, ni thème.
 */
withDefaults(defineProps<{ texte: string; action?: string }>(), { action: undefined })
const emit = defineEmits<{ agir: []; fini: [] }>()

const visible = ref(true)
let minuterie: ReturnType<typeof setTimeout> | undefined

const SECONDES_PAR_DEFAUT = 6

/** La durée est un jeton : elle se lit sous la borne, seul endroit où elle est définie. */
function dureeEnMs(): number {
  const portail = document.getElementById('gn-portail')
  const jeton = portail && getComputedStyle(portail).getPropertyValue('--gn-duree-ephemere')
  const secondes = Number.parseFloat(jeton || '')
  return (Number.isFinite(secondes) && secondes > 0 ? secondes : SECONDES_PAR_DEFAUT) * 1000
}

onMounted(() => {
  minuterie = setTimeout(fermer, dureeEnMs())
})

onBeforeUnmount(() => clearTimeout(minuterie))

function fermer() {
  visible.value = false
  emit('fini')
}

function agir() {
  emit('agir')
  fermer()
}
</script>

<template>
  <Teleport to="#gn-portail">
    <p v-if="visible" class="gn-ephemere" role="status">
      <span class="gn-ephemere__texte">{{ texte }}</span>
      <button v-if="action" type="button" class="gn-ephemere__action" @click="agir">{{ action }}</button>
    </p>
  </Teleport>
</template>

<style>
[data-app="guide-nego"] .gn-ephemere {
  position: absolute;
  left: var(--gn-marge-ecran);
  right: var(--gn-marge-ecran);
  bottom: calc(var(--gn-sur-bas-barre-onglets) + var(--gn-sur-bas-geste) + env(safe-area-inset-bottom));
  min-height: var(--gn-cible);
  padding: var(--gn-espace-12) var(--gn-espace-16);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--gn-espace-16);
  background: var(--gn-titre);
  color: var(--gn-sur-titre);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  animation: gn-ephemere-entre var(--gn-duree-standard) var(--gn-courbe-sortie);
}

[data-app="guide-nego"] .gn-ephemere__action {
  flex: none;
  min-height: var(--gn-cible);
  padding-inline: var(--gn-espace-8);
  border: none;
  background: none;
  color: var(--gn-action-sur-titre);
  font-weight: var(--gn-graisse-gras);
  text-decoration-line: var(--gn-action-sur-titre-trait);
  text-underline-offset: var(--gn-espace-4);
}

@keyframes gn-ephemere-entre {
  from { opacity: 0; transform: translateY(var(--gn-espace-8)); }
}
</style>
