<script setup lang="ts">
/**
 * Champ de saisie d'une ligne, libellé TOUJOURS au-dessus : un texte indicatif s'efface
 * à la première frappe, et la personne ne sait plus ce qu'elle est en train de remplir.
 *
 * Il sert aussi d'enveloppe à `GnZoneTexte` : le libellé, l'aide, l'erreur et les
 * identifiants `aria` qui les relient vivent ici une seule fois, et la fente par défaut
 * remplace la saisie sans rien dupliquer.
 *
 * `type="time"` emporte la largeur de 110 px. La mesure tient au contenu — « 14:30 » et
 * rien d'autre — pas au goût de l'écran : une prop de largeur de plus serait un second
 * réglage à tenir accordé, et un champ d'heure pleine largeur le jour où on l'oublie.
 */
defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    libelle: string
    aide?: string
    erreur?: string
    desactive?: boolean
    indication?: string
    type?: 'text' | 'email' | 'password' | 'tel' | 'url' | 'number' | 'time'
  }>(),
  {
    aide: undefined,
    erreur: undefined,
    desactive: false,
    indication: undefined,
    type: 'text',
  },
)

defineSlots<{
  default?: (fente: {
    idSaisie: string
    decritPar: string | undefined
    invalide: 'true' | undefined
  }) => unknown
  pied?: () => unknown
}>()

const valeur = defineModel<string>({ default: '' })

const { t } = useI18n()

const base = useId()
const idSaisie = `${base}-saisie`
const idAide = `${base}-aide`
const idErreur = `${base}-erreur`

const decritPar = computed(
  () => [props.aide ? idAide : '', props.erreur ? idErreur : ''].filter(Boolean).join(' ') || undefined,
)

const invalide = computed<'true' | undefined>(() => (props.erreur ? 'true' : undefined))
</script>

<template>
  <div
    class="gn-champ"
    :class="{
      'gn-champ--erreur': Boolean(erreur),
      'gn-champ--desactive': desactive,
      'gn-champ--heure': type === 'time',
    }"
  >
    <label class="gn-champ__libelle" :for="idSaisie">{{ libelle }}</label>

    <slot :id-saisie="idSaisie" :decrit-par="decritPar" :invalide="invalide">
      <input
        :id="idSaisie"
        v-model="valeur"
        class="gn-champ__saisie"
        :type="type"
        :placeholder="indication"
        :disabled="desactive"
        :aria-describedby="decritPar"
        :aria-invalid="invalide"
        v-bind="$attrs"
      >
    </slot>

    <div v-if="aide || $slots.pied" class="gn-champ__pied">
      <span v-if="aide" :id="idAide" class="gn-champ__aide">{{ aide }}</span>
      <slot name="pied" />
    </div>

    <p v-if="erreur" :id="idErreur" class="gn-champ__erreur" role="alert">
      <GnPicto nom="warn" :taille="20" />
      <span><span class="gn-hors-ecran">{{ t('gn-champ.erreur') }} </span>{{ erreur }}</span>
    </p>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-champ {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-champ__libelle {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-champ__saisie {
  width: 100%;
  min-height: var(--gn-champ);
  padding-inline: var(--gn-espace-12);
  border: var(--gn-filet-2) solid var(--gn-filet);
  border-radius: var(--gn-rayon-4);
  background: var(--gn-fond);
  color: var(--gn-texte);
  font-family: var(--gn-police);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-champ__saisie::placeholder {
  color: var(--gn-texte-2);
  opacity: 1;
}

/* Le focus fonce aussi le bord : au soleil, l'anneau seul se perd. L'erreur garde le
   sien, rouge — sa règle vient après. */
[data-app="guide-nego"] .gn-champ__saisie:focus-visible {
  border-color: var(--gn-filet-fort);
  outline: var(--gn-focus-anneau) solid var(--gn-focus);
  outline-offset: var(--gn-focus-decalage);
}

[data-app="guide-nego"] .gn-champ--heure .gn-champ__saisie {
  width: var(--gn-champ-heure);
}

[data-app="guide-nego"] .gn-champ--erreur .gn-champ__libelle {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-champ--erreur .gn-champ__saisie {
  border-color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-champ--desactive .gn-champ__libelle {
  color: var(--gn-texte-2);
}

/* Un champ éteint reste un aplat plein : le gris translucide des navigateurs rend le
   texte illisible au soleil, et Safari repeint la couleur malgré `color`. */
[data-app="guide-nego"] .gn-champ__saisie:disabled {
  border-color: var(--gn-desactive-fond);
  background: var(--gn-desactive-fond);
  color: var(--gn-desactive-texte);
  -webkit-text-fill-color: var(--gn-desactive-texte);
  opacity: 1;
  cursor: not-allowed;
}

[data-app="guide-nego"] .gn-champ__pied {
  display: flex;
  justify-content: space-between;
  gap: var(--gn-espace-8);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-champ__erreur {
  margin: 0;
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  color: var(--gn-danger);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}
</style>
