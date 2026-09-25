<script setup lang="ts">
import { momentDeLecture } from '~/utils/guide-nego/connexion'

/**
 * La source officielle est coupée (07 · 1c) : l'écran le dit, et ne montre aucune
 * session à la place — pas même celles de ce matin, qui ont pu changer (principe XII).
 *
 * `unreachable` : elle ne répond plus, depuis une heure qu'on donne. `disabled` : l'IFDD
 * a suspendu l'affichage. Dans les deux cas, le programme officiel reste la sortie.
 */
const props = withDefaults(
  defineProps<{
    raison: 'unreachable' | 'disabled'
    /** Depuis quand la source ne répond plus. */
    depuis?: string | null
    /** L'adresse du programme officiel de la CCNUCC ; nulle, le texte seul le nomme. */
    programme?: string | null
    enCours?: boolean
  }>(),
  { depuis: null, programme: null, enCours: false },
)

defineEmits<{ reessayer: [] }>()

const { t, locale } = useI18n()

const texte = computed(() => {
  if (props.raison === 'disabled') {
    return t('gn-lecture-impossible.suspendu')
  }
  if (!props.depuis) return t('gn-lecture-impossible.sans-reponse')
  const moment = momentDeLecture(props.depuis, new Date(), locale.value)
  const depuis = t(`gn-lecture-impossible.depuis.${moment.quand}`, { heure: moment.heure, jour: moment.quand === 'avant' ? moment.jour : '' })
  return t('gn-lecture-impossible.sans-reponse-depuis', { depuis })
})
</script>

<template>
  <div class="gn-lecture-impossible" role="alert">
    <GnPicto nom="warn" :taille="24" />
    <h2 class="gn-lecture-impossible__titre">
      {{ raison === 'disabled' ? t('gn-lecture-impossible.titre-suspendu') : t('gn-lecture-impossible.titre') }}
    </h2>
    <p class="gn-lecture-impossible__texte">{{ texte }}</p>
    <div class="gn-lecture-impossible__sorties">
      <GnBouton v-if="programme" variante="principal" picto="external" :vers="programme">
        {{ t('gn-lecture-impossible.lien-ccnucc') }}
      </GnBouton>
      <GnBouton variante="secondaire" :chargement="enCours" @clic="$emit('reessayer')">
        {{ t('gn-lecture-impossible.reessayer') }}
      </GnBouton>
    </div>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-lecture-impossible {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  padding-block: var(--gn-espace-24);
}

[data-app="guide-nego"] .gn-lecture-impossible .gn-picto {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-lecture-impossible__titre {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-lecture-impossible__texte {
  max-width: var(--gn-mesure-lecture);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-lecture-impossible__sorties {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-cibles);
  padding-block-start: var(--gn-espace-8);
}

/* Le pictogramme du bouton garde la couleur du bouton, pas le rouge de l'alerte. */
[data-app="guide-nego"] .gn-lecture-impossible__sorties .gn-picto {
  color: inherit;
}
</style>
