<script setup lang="ts">
/**
 * Un choix unique dans une liste de lignes : cercle de réglage, cercle de partage,
 * réponse de quiz. Toute la ligne de 56 px est la cible.
 *
 * Le composant porte le groupe entier, et non une option isolée : le déplacement aux
 * flèches et l'unique arrêt de tabulation demandent de connaître les voisines, et
 * les recopier dans chaque écran finirait par donner autant de comportements que
 * d'écrans. Même mécanisme que `GnSegmente`.
 *
 * `marque` sert la correction d'un quiz : la bonne réponse et la mienne quand elle est
 * fausse. La croix rouge dit « faux » ; le triangle reste réservé à l'erreur système.
 */
export interface OptionDeCercle {
  valeur: string
  libelle: string
  detail?: string
  marque?: 'juste' | 'faux'
  desactive?: boolean
}

const props = withDefaults(
  defineProps<{
    options: OptionDeCercle[]
    /** Nom du groupe, annoncé avant les options. */
    libelle: string
    /** Le quiz corrigé, par exemple : les options restent lisibles, plus modifiables. */
    desactive?: boolean
  }>(),
  { desactive: false },
)

const choix = defineModel<string | null>({ default: null })

const { t } = useI18n()
const groupe = useTemplateRef<HTMLElement>('groupe')

const disponibles = computed(() => props.options.filter((option) => !option.desactive))

/** Le groupe est UNE étape de tabulation : l'option cochée la porte, sinon la première. */
const tabulable = computed(
  () =>
    (props.options.find((option) => option.valeur === choix.value) ??
      disponibles.value[0] ??
      props.options[0])?.valeur,
)

function choisir(option: OptionDeCercle) {
  if (props.desactive || option.desactive) return
  choix.value = option.valeur
}

function deplacer(pas: number) {
  if (props.desactive || disponibles.value.length === 0) return
  const index = disponibles.value.findIndex((option) => option.valeur === choix.value)
  const suivante = disponibles.value[(index + pas + disponibles.value.length) % disponibles.value.length]
  if (!suivante) return
  choix.value = suivante.valeur
  nextTick(() => groupe.value?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus())
}
</script>

<template>
  <div ref="groupe" class="gn-cercle" role="radiogroup" :aria-label="libelle">
    <button
      v-for="(option, index) in options"
      :key="option.valeur"
      type="button"
      role="radio"
      class="gn-cercle__option"
      :class="[
        option.marque ? `gn-cercle__option--${option.marque}` : undefined,
        {
          'gn-cercle__option--coche': option.valeur === choix,
          'gn-cercle__option--derniere': index === options.length - 1,
          'gn-cercle__option--desactive': desactive || option.desactive,
        },
      ]"
      :aria-checked="option.valeur === choix"
      :aria-disabled="desactive || option.desactive || undefined"
      :tabindex="option.valeur === tabulable ? 0 : -1"
      @click="choisir(option)"
      @keydown.left.prevent="deplacer(-1)"
      @keydown.up.prevent="deplacer(-1)"
      @keydown.right.prevent="deplacer(1)"
      @keydown.down.prevent="deplacer(1)"
    >
      <span class="gn-cercle__rond" aria-hidden="true">
        <GnPicto v-if="option.marque === 'faux'" nom="close" :taille="16" />
        <GnPicto
          v-else-if="option.marque === 'juste' || option.valeur === choix"
          nom="check"
          :taille="16"
        />
      </span>
      <span class="gn-cercle__texte">
        <span class="gn-cercle__libelle">{{ option.libelle }}</span>
        <span v-if="option.detail" class="gn-cercle__detail">{{ option.detail }}</span>
      </span>
      <span v-if="option.marque" class="gn-cercle__mot">{{ t(`gn-cercle.${option.marque}`) }}</span>
    </button>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-cercle {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-cercle__option {
  width: 100%;
  min-height: var(--gn-ligne-reglage);
  padding: var(--gn-espace-8) 0;
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border: none;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  background: none;
  color: var(--gn-texte);
  text-align: start;
}

[data-app="guide-nego"] .gn-cercle__option--derniere {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-cercle__option:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-cercle__option:not(.gn-cercle__option--desactive):active {
  background: var(--gn-presse);
  /* La pression déborde les marges de l'écran, comme une ligne pleine largeur. */
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-cercle__rond {
  inline-size: var(--gn-cercle-choix);
  block-size: var(--gn-cercle-choix);
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  border: var(--gn-filet-2) solid var(--gn-filet-fort);
  border-radius: var(--gn-rayon-24);
  color: var(--gn-accent-inv);
}

[data-app="guide-nego"] .gn-cercle__option--coche .gn-cercle__rond {
  background: var(--gn-accent);
  border-color: var(--gn-accent);
}

/* La correction passe avant mon choix : une réponse cochée ET fausse est rouge. */
[data-app="guide-nego"] .gn-cercle__option--juste .gn-cercle__rond {
  background: var(--gn-succes);
  border-color: var(--gn-succes);
}

[data-app="guide-nego"] .gn-cercle__option--faux .gn-cercle__rond {
  background: var(--gn-danger);
  border-color: var(--gn-danger);
  color: var(--gn-danger-texte);
}

[data-app="guide-nego"] .gn-cercle__texte {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-cercle__libelle {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-cercle__detail {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-cercle__mot {
  flex: none;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-cercle__option--juste .gn-cercle__mot {
  color: var(--gn-succes);
}

[data-app="guide-nego"] .gn-cercle__option--faux .gn-cercle__mot {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-cercle__option--desactive {
  cursor: not-allowed;
}

/* Une option seulement désactivée s'efface ; une option corrigée garde ses couleurs. */
[data-app="guide-nego"] .gn-cercle__option--desactive:not(.gn-cercle__option--juste):not(.gn-cercle__option--faux) {
  color: var(--gn-desactive-texte);
}

[data-app="guide-nego"] .gn-cercle__option--desactive:not(.gn-cercle__option--juste):not(.gn-cercle__option--faux) .gn-cercle__detail {
  color: var(--gn-desactive-texte);
}

[data-app="guide-nego"] .gn-cercle__option--desactive:not(.gn-cercle__option--juste):not(.gn-cercle__option--faux) .gn-cercle__rond {
  border-color: var(--gn-desactive-texte);
  color: var(--gn-desactive-texte);
}

[data-app="guide-nego"] .gn-cercle__option--desactive.gn-cercle__option--coche:not(.gn-cercle__option--juste):not(.gn-cercle__option--faux) .gn-cercle__rond {
  background: var(--gn-desactive-fond);
}
</style>
