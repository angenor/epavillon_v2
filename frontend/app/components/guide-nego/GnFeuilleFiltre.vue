<script setup lang="ts">
const props = defineProps<{
  ouverte: boolean
  titre: string
  /** « tous les types », « toutes les thématiques » : ce que donne une feuille sans choix. */
  tous: string
  options: Array<{ valeur: string; libelle: string; compte: number }>
  /** Le choix appliqué ; la feuille travaille sur un brouillon, copié à chaque ouverture. */
  choix: string[]
  /** Le nombre de documents que donnerait ce brouillon, avant de l'appliquer. */
  compterPour: (brouillon: string[]) => number
}>()

const emit = defineEmits<{ appliquer: [string[]]; fermer: [] }>()

const { t } = useI18n()

const brouillon = ref<string[]>([])

watch(
  () => props.ouverte,
  (ouverte) => {
    if (ouverte) brouillon.value = [...props.choix]
  },
  { immediate: true },
)

function coche(valeur: string): boolean {
  return brouillon.value.includes(valeur)
}

function basculer(valeur: string, coche: boolean) {
  brouillon.value = coche
    ? [...brouillon.value, valeur]
    : brouillon.value.filter((v) => v !== valeur)
}

const nombre = computed(() => props.compterPour(brouillon.value))

function changerOuverture(ouverte: boolean) {
  if (!ouverte) emit('fermer')
}

function appliquer() {
  emit('appliquer', [...brouillon.value])
  emit('fermer')
}
</script>

<template>
  <GnFeuilleBasse
    :model-value="ouverte"
    :titre="titre"
    :sous-titre="t('gn-feuille-filtre.sous-titre', { tous })"
    @update:model-value="changerOuverture"
  >
    <ul class="gn-feuille-filtre__options">
      <li v-for="(option, rang) in options" :key="option.valeur" class="gn-feuille-filtre__option">
        <GnCase
          :model-value="coche(option.valeur)"
          :libelle="option.libelle"
          :derniere="rang === options.length - 1"
          @update:model-value="basculer(option.valeur, $event)"
        />
        <span class="gn-feuille-filtre__compte">{{ option.compte }}</span>
      </li>
    </ul>
    <GnBouton @clic="appliquer">
      {{ t('gn-feuille-filtre.afficher', { count: nombre }, nombre) }}
    </GnBouton>
  </GnFeuilleBasse>
</template>

<style>
[data-app="guide-nego"] .gn-feuille-filtre__options {
  border-top: var(--gn-filet-1) solid var(--gn-filet);
}

/* GnCase n'a pas de place en fin de ligne : le compte se pose par-dessus, et laisse le toucher à la case. */
[data-app="guide-nego"] .gn-feuille-filtre__option {
  position: relative;
}

[data-app="guide-nego"] .gn-feuille-filtre__option > .gn-case {
  padding-inline-end: var(--gn-espace-48);
}

[data-app="guide-nego"] .gn-feuille-filtre__option > .gn-case:not(.gn-case--desactive):active {
  padding-inline-end: calc(var(--gn-marge-ecran) + var(--gn-espace-48));
}

[data-app="guide-nego"] .gn-feuille-filtre__compte {
  position: absolute;
  inset-block: 0;
  inset-inline-end: 0;
  display: flex;
  align-items: center;
  pointer-events: none;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}
</style>
