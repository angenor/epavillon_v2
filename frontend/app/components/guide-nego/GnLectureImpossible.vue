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
    <span class="gn-lecture-impossible__pastille" aria-hidden="true"><GnPicto nom="warn" :taille="20" /></span>
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
    <!-- Ce qui reste affiché malgré la coupure : les réunions signalées par le réseau (FR-022). -->
    <p v-if="$slots.default" class="gn-lecture-impossible__garde"><slot /></p>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-lecture-impossible {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  padding: 18px var(--gn-espace-16);
  border-radius: var(--gn-rayon-24);
  background: var(--gn-fond-2);
}

[data-app="guide-nego"] .gn-lecture-impossible__pastille {
  flex: none;
  width: var(--gn-pastille-icone);
  height: var(--gn-pastille-icone);
  margin-block-end: var(--gn-espace-4);
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--gn-rayon-14);
  background: var(--gn-bloc-releve);
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-lecture-impossible__titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-lecture-impossible__texte {
  max-width: var(--gn-mesure-lecture);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
}

[data-app="guide-nego"] .gn-lecture-impossible__sorties {
  width: 100%;
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-cibles);
  padding-block-start: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-lecture-impossible__garde {
  padding-block-start: var(--gn-espace-8);
  display: flex;
  align-items: flex-start;
  gap: 6px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}
</style>
