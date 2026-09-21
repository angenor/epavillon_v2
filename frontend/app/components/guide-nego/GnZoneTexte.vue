<script setup lang="ts">
/**
 * Zone de texte de 120 px — question à un expert, rubrique de restitution, signalement.
 * Elle emprunte son libellé, son aide et son erreur à `GnChamp` : un seul dessin, une
 * seule règle « libellé au-dessus », une seule correction le jour où il faut la reprendre.
 *
 * Le compteur n'est PAS une zone vive : relu à chaque frappe, il couvrirait la dictée.
 * Il est décrit par le champ — donc lu à la prise de focus — et une annonce polie n'arrive
 * que dans les cinquante derniers caractères, au moment où la limite concerne la personne.
 */
defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    libelle: string
    aide?: string
    erreur?: string
    desactive?: boolean
    indication?: string
    maximum?: number
  }>(),
  {
    aide: undefined,
    erreur: undefined,
    desactive: false,
    indication: undefined,
    maximum: 600,
  },
)

const valeur = defineModel<string>({ default: '' })

const { t, locale } = useI18n()

const idCompteur = `${useId()}-compteur`

const SEUIL_ANNONCE = 50

const format = computed(() => new Intl.NumberFormat(locale.value))

const compteur = computed(() =>
  t('gn-zone-texte.compteur', {
    saisi: format.value.format(valeur.value.length),
    maximum: format.value.format(props.maximum),
  }),
)

const restant = computed(() => Math.max(props.maximum - valeur.value.length, 0))

const annonce = computed(() =>
  restant.value <= SEUIL_ANNONCE
    ? t('gn-zone-texte.reste', { restant: format.value.format(restant.value) })
    : '',
)
</script>

<template>
  <GnChamp
    :libelle="libelle"
    :aide="aide"
    :erreur="erreur"
    :desactive="desactive"
  >
    <template #default="{ idSaisie, decritPar, invalide }">
      <textarea
        :id="idSaisie"
        v-model="valeur"
        class="gn-champ__saisie gn-zone-texte__saisie"
        :placeholder="indication"
        :disabled="desactive"
        :maxlength="maximum"
        :aria-describedby="[decritPar, idCompteur].filter(Boolean).join(' ')"
        :aria-invalid="invalide"
        v-bind="$attrs"
      />
    </template>

    <template #pied>
      <span :id="idCompteur" class="gn-zone-texte__compteur">{{ compteur }}</span>
      <span class="gn-hors-ecran" role="status">{{ annonce }}</span>
    </template>
  </GnChamp>
</template>

<style>
[data-app="guide-nego"] .gn-zone-texte__saisie {
  height: var(--gn-zone-texte);
  padding: var(--gn-espace-12);
  resize: none;
}

/* `margin-inline-start: auto` et non `justify-content` : le compteur reste collé à droite
   même sans aide à sa gauche. Les chiffres sont tabulaires depuis la racine : il ne tremble pas. */
[data-app="guide-nego"] .gn-zone-texte__compteur {
  margin-inline-start: auto;
  white-space: nowrap;
}
</style>
