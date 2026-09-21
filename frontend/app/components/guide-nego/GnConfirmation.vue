<script setup lang="ts">
import type { NomDePicto } from '~/utils/guide-nego/pictogrammes'
/**
 * La boîte qui demande avant d'agir. Toute action dangereuse — Retirer mon message,
 * Quitter le canal, Bloquer — passe par elle : le bouton rouge ne s'actionne jamais seul.
 *
 * `alertdialog` et non `dialog` : elle interrompt et n'attend qu'une décision, ce qui
 * fait lire sa question et sa phrase d'un coup au lieu d'un titre puis d'un contenu.
 *
 * L'action principale est à DROITE, le retour à gauche — c'est la règle du système, pas
 * un réglage. Le focus se pose sur le retour : d'une touche Entrée trop vite venue, on
 * ne détruit rien.
 */
withDefaults(
  defineProps<{
    question: string
    phrase?: string
    /** Le verbe du bouton de droite : « Envoyer », « Retirer mon message ». */
    action: string
    picto?: NomDePicto
    /** Le bouton de droite devient rouge, pour ce qui ne se défait pas. */
    dangereuse?: boolean
    /** Remplace « Revenir » quand la sortie porte un autre nom. */
    retour?: string
  }>(),
  { phrase: undefined, picto: undefined, dangereuse: false, retour: undefined },
)

const emit = defineEmits<{ confirmer: [] }>()

const ouverte = defineModel<boolean>({ required: true })

const { t } = useI18n()
const idQuestion = useId()
const idPhrase = useId()
const boite = useTemplateRef<HTMLElement>('boite')

useGnPiegeFocus(ouverte, () => boite.value, {
  fermer,
  focusInitial: () => boite.value?.querySelector<HTMLElement>('.gn-confirmation__retour'),
})

function fermer() {
  ouverte.value = false
}

function confirmer() {
  emit('confirmer')
  fermer()
}
</script>

<template>
  <Teleport to="#gn-portail">
    <Transition name="gn-confirmation">
      <div v-if="ouverte" class="gn-confirmation">
        <!-- Le voile ne ferme pas : une question qui interrompt attend sa réponse. -->
        <div class="gn-confirmation__voile" aria-hidden="true" />
        <div
          ref="boite"
          class="gn-confirmation__boite"
          role="alertdialog"
          aria-modal="true"
          :aria-labelledby="idQuestion"
          :aria-describedby="phrase ? idPhrase : undefined"
        >
          <h2 :id="idQuestion" class="gn-confirmation__question">{{ question }}</h2>
          <p v-if="phrase" :id="idPhrase" class="gn-confirmation__phrase">{{ phrase }}</p>
          <div class="gn-confirmation__actions">
            <GnBouton
              class="gn-confirmation__retour"
              variante="secondaire"
              largeur="demie"
              @clic="fermer"
            >
              {{ retour ?? t('gn-confirmation.revenir') }}
            </GnBouton>
            <GnBouton
              :variante="dangereuse ? 'dangereux' : 'principal'"
              largeur="demie"
              :picto="picto"
              @clic="confirmer"
            >
              {{ action }}
            </GnBouton>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style>
[data-app="guide-nego"] .gn-confirmation {
  position: absolute;
  inset: 0;
  padding: var(--gn-marge-ecran);
  display: flex;
  align-items: center;
  justify-content: center;
}

[data-app="guide-nego"] .gn-confirmation__voile {
  position: absolute;
  inset: 0;
  background: var(--gn-voile);
  opacity: var(--gn-voile-opacite);
}

[data-app="guide-nego"] .gn-confirmation__boite {
  position: relative;
  width: 100%;
  /* Le cadre de référence moins ses deux marges : sur un grand écran, la boîte ne s'étire pas. */
  max-width: calc(var(--gn-cadre-largeur) - 2 * var(--gn-marge-ecran));
  max-height: 100%;
  overflow-y: auto;
  padding: var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  border: var(--gn-filet-2) solid var(--gn-filet-fort);
  border-radius: var(--gn-rayon-4);
  background: var(--gn-fond);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-confirmation__question {
  color: var(--gn-titre);
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-confirmation__phrase {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-confirmation__actions {
  margin-top: var(--gn-espace-4);
  display: flex;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-confirmation-enter-active,
[data-app="guide-nego"] .gn-confirmation-leave-active {
  transition: opacity var(--gn-duree-standard) var(--gn-courbe-sortie);
}

[data-app="guide-nego"] .gn-confirmation-enter-active .gn-confirmation__boite,
[data-app="guide-nego"] .gn-confirmation-leave-active .gn-confirmation__boite {
  transition: transform var(--gn-duree-standard) var(--gn-courbe-sortie);
}

[data-app="guide-nego"] .gn-confirmation-enter-from,
[data-app="guide-nego"] .gn-confirmation-leave-to {
  opacity: 0;
}

[data-app="guide-nego"] .gn-confirmation-enter-from .gn-confirmation__boite,
[data-app="guide-nego"] .gn-confirmation-leave-to .gn-confirmation__boite {
  transform: translateY(var(--gn-espace-8));
}
</style>
