<script setup lang="ts">
/**
 * Le rail A–Z de la liste : toucher une lettre, ou y glisser le doigt, saute à son
 * groupe. Une lettre sans entrée est grise, la lettre en cours porte un filet.
 *
 * Le dessin fait 24 px de large ; la cible, 44, gagnée vers l'intérieur de l'écran.
 * Le doigt vise tout le rail : chaque lettre, à 20 px de haut, serait trop petite.
 */
const props = defineProps<{ presentes: readonly string[]; courante: string | null }>()
const emit = defineEmits<{ choisir: [lettre: string] }>()

const { t } = useI18n()

const LETTRES = [...'ABCDEFGHIJKLMNOPQRSTUVWXYZ']
const lettres = computed(() => (props.presentes.includes('#') ? [...LETTRES, '#'] : LETTRES))
const presentes = computed(() => new Set(props.presentes))

const rail = useTemplateRef<HTMLElement>('rail')
let glisse = false
let derniere: string | null = null
let dernierY = 0

function lettreSous(y: number): string | null {
  const boutons = rail.value?.querySelectorAll<HTMLElement>('[data-lettre]')
  if (!boutons?.length) return null
  for (const bouton of boutons) {
    if (y < bouton.getBoundingClientRect().bottom) return bouton.dataset.lettre ?? null
  }
  return boutons[boutons.length - 1]?.dataset.lettre ?? null
}

function viser(y: number): void {
  dernierY = y
  const lettre = lettreSous(y)
  if (!lettre || lettre === derniere) return
  derniere = lettre
  emit('choisir', lettre)
}

function poser(evenement: PointerEvent): void {
  glisse = true
  derniere = null
  rail.value?.setPointerCapture(evenement.pointerId)
  viser(evenement.clientY)
}

// La page qui défile fait passer le rail sous un doigt immobile : seul un doigt qui bouge vise.
function glisser(evenement: PointerEvent): void {
  if (glisse && evenement.clientY !== dernierY) viser(evenement.clientY)
}

function lever(): void {
  glisse = false
}

// Au clavier seulement : le doigt passe par le rail entier.
function auClavier(evenement: MouseEvent, lettre: string): void {
  if (evenement.detail === 0) emit('choisir', lettre)
}
</script>

<template>
  <nav
    ref="rail"
    class="gn-rail-alphabet"
    :aria-label="t('gn-rail-alphabet.libelle')"
    @pointerdown="poser"
    @pointermove="glisser"
    @pointerup="lever"
    @pointercancel="lever"
  >
    <button
      v-for="lettre in lettres"
      :key="lettre"
      type="button"
      class="gn-rail-alphabet__lettre"
      :class="{
        'gn-rail-alphabet__lettre--vide': !presentes.has(lettre),
        'gn-rail-alphabet__lettre--courante': lettre === courante,
      }"
      :data-lettre="lettre"
      :aria-disabled="presentes.has(lettre) ? undefined : 'true'"
      :aria-current="lettre === courante ? 'true' : undefined"
      :aria-label="t('gn-rail-alphabet.lettre', { lettre })"
      @click="auClavier($event, lettre)"
    >
      {{ lettre }}
    </button>
  </nav>
</template>

<style>
[data-app="guide-nego"] .gn-rail-alphabet {
  flex: none;
  inline-size: var(--gn-rail-alpha);
  display: flex;
  flex-direction: column;
  position: relative;
  touch-action: none;
  user-select: none;
  -webkit-user-select: none;
}

/* Vingt pixels de cible en plus, vers le contenu : 44 en tout. */
[data-app="guide-nego"] .gn-rail-alphabet::before {
  content: '';
  position: absolute;
  inset-block: 0;
  inset-inline-start: -20px;
  inset-inline-end: 0;
}

[data-app="guide-nego"] .gn-rail-alphabet__lettre {
  position: relative;
  block-size: var(--gn-rail-alpha-interligne);
  padding: 0;
  border: none;
  border-inline-end: var(--gn-filet-3) solid transparent;
  background: none;
  color: var(--gn-titre);
  font-family: var(--gn-police);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-rail-alpha-interligne);
  font-weight: var(--gn-graisse-gras);
  text-align: center;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-rail-alphabet__lettre--vide {
  color: var(--gn-desactive-texte);
}

[data-app="guide-nego"] .gn-rail-alphabet__lettre--courante {
  border-inline-end-color: var(--gn-filet-fort);
}

[data-app="guide-nego"] .gn-rail-alphabet__lettre:focus-visible {
  outline: var(--gn-focus-anneau) solid var(--gn-focus);
  outline-offset: calc(-1 * var(--gn-focus-anneau));
}
</style>
