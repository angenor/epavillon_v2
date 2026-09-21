<script setup lang="ts">
import { useJetonsLus } from './PlancheSection.vue'

/**
 * Section 7 — les durées et les courbes, rejouables.
 *
 * Lues à l'exécution, elles prouvent aussi la règle : sous « réduire les animations »,
 * les durées tombent à 0 et les boucles se figent — ce que cette planche affiche en
 * toutes lettres plutôt que de le promettre.
 */
const DUREES = ['bref', 'standard', 'long'] as const
const COURBES = ['sortie', 'standard'] as const
const MESURES = [
  '--gn-duree-bref',
  '--gn-duree-standard',
  '--gn-duree-long',
  '--gn-duree-arc',
  '--gn-duree-squelette',
  '--gn-duree-ephemere',
  '--gn-courbe-sortie',
  '--gn-courbe-standard',
]

const { t } = useI18n()
const racine = useTemplateRef<HTMLElement>('racine')
const lus = useJetonsLus(racine, [], MESURES)

const valeur = (jeton: string): string => lus.value[jeton] || '…'

const auBout = reactive<Record<string, boolean>>({ bref: false, standard: false, long: false })
const courbesAuBout = ref(false)

const duree = (cle: string) => ({ transitionDuration: `var(--gn-duree-${cle})` })
const courbe = (cle: string) => ({
  transitionDuration: 'var(--gn-duree-long)',
  transitionTimingFunction: `var(--gn-courbe-${cle})`,
})

function enMs(brut: string): number {
  const nombre = Number.parseFloat(brut)
  if (Number.isNaN(nombre)) return 0
  return brut.trim().endsWith('ms') ? nombre : nombre * 1000
}

const messageVisible = ref(false)
const compteEcoule = ref(false)
const envoi = ref(0)
let minuterie: ReturnType<typeof setTimeout> | undefined

/**
 * Le message tient exactement ce que dit le jeton : la valeur est relue à chaque envoi.
 * Deux trames avant de vider la jauge — sans cela le navigateur n'a jamais peint la
 * jauge pleine, et la voit passer de 0 à 0.
 */
function jouerEphemere() {
  clearTimeout(minuterie)
  envoi.value += 1
  messageVisible.value = true
  compteEcoule.value = false
  requestAnimationFrame(() => requestAnimationFrame(() => (compteEcoule.value = true)))
  const tenue = enMs(valeur('--gn-duree-ephemere'))
  if (tenue > 0) minuterie = setTimeout(() => (messageVisible.value = false), tenue)
}

const reduit = ref(false)
let requete: MediaQueryList | undefined
const suivreLeReglage = (evenement: MediaQueryListEvent) => (reduit.value = evenement.matches)

onMounted(() => {
  requete = window.matchMedia('(prefers-reduced-motion: reduce)')
  reduit.value = requete.matches
  requete.addEventListener('change', suivreLeReglage)
})

onBeforeUnmount(() => {
  clearTimeout(minuterie)
  requete?.removeEventListener('change', suivreLeReglage)
})
</script>

<template>
  <GnPlancheSection
    numero="7"
    :titre="t('gn-planche-mouvement.titre')"
    :propos="t('gn-planche-mouvement.propos')"
  >
    <div ref="racine" class="gn-planche-mouvement">
      <p class="gn-planche-note">{{ t('gn-planche-mouvement.principe') }}</p>

      <GnPlancheSection
        :titre="t('gn-planche-mouvement.durees')"
        :propos="t('gn-planche-mouvement.durees-propos')"
      >
        <div class="gn-planche-liste">
          <div v-for="jeton in MESURES" :key="jeton" class="gn-planche-ligne">
            <span class="gn-planche-ligne__nom gn-planche-jeton">{{ jeton }}</span>
            <span class="gn-planche-valeur">{{ valeur(jeton) }}</span>
          </div>
        </div>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-mouvement.rejouables')"
        :propos="t('gn-planche-mouvement.rejouables-propos')"
      >
        <div
          v-for="cle in DUREES"
          :key="cle"
          class="gn-planche-mouvement__carte"
        >
          <span class="gn-planche-jeton">--gn-duree-{{ cle }}</span>
          <span class="gn-planche-valeur">
            {{ valeur(`--gn-duree-${cle}`) }} — {{ t(`gn-planche-mouvement.usage.${cle}`) }}
          </span>
          <div class="gn-planche-mouvement__piste">
            <div
              class="gn-planche-mouvement__pion"
              :class="{ 'gn-planche-mouvement__pion--bout': auBout[cle] }"
              :style="duree(cle)"
            />
          </div>
          <GnBouton
            variante="secondaire"
            :aria-label="t('gn-planche-mouvement.rejouer-nom', { jeton: `--gn-duree-${cle}` })"
            @clic="auBout[cle] = !auBout[cle]"
          >
            {{ t('gn-planche-mouvement.rejouer') }}
          </GnBouton>
        </div>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-mouvement.courbes')"
        :propos="t('gn-planche-mouvement.courbes-propos')"
      >
        <div class="gn-planche-mouvement__carte">
          <div v-for="cle in COURBES" :key="cle" class="gn-planche-mouvement__voie">
            <span class="gn-planche-jeton">--gn-courbe-{{ cle }}</span>
            <span class="gn-planche-valeur">{{ valeur(`--gn-courbe-${cle}`) }}</span>
            <span class="gn-planche-valeur">{{ t(`gn-planche-mouvement.usage-courbe.${cle}`) }}</span>
            <div class="gn-planche-mouvement__piste">
              <div
                class="gn-planche-mouvement__pion"
                :class="{ 'gn-planche-mouvement__pion--bout': courbesAuBout }"
                :style="courbe(cle)"
              />
            </div>
          </div>
          <GnBouton variante="secondaire" @clic="courbesAuBout = !courbesAuBout">
            {{ t('gn-planche-mouvement.rejouer-ensemble') }}
          </GnBouton>
        </div>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-mouvement.boucles')"
        :propos="t('gn-planche-mouvement.boucles-propos')"
      >
        <div class="gn-planche-mouvement__carte">
          <span class="gn-planche-jeton">--gn-duree-arc</span>
          <span class="gn-planche-valeur">
            {{ valeur('--gn-duree-arc') }} — {{ t('gn-planche-mouvement.usage.arc') }}
          </span>
          <div class="gn-planche-mouvement__arc" />
        </div>
        <div class="gn-planche-mouvement__carte">
          <span class="gn-planche-jeton">--gn-duree-squelette</span>
          <span class="gn-planche-valeur">
            {{ valeur('--gn-duree-squelette') }} — {{ t('gn-planche-mouvement.usage.squelette') }}
          </span>
          <div class="gn-planche-mouvement__squelette" />
          <div class="gn-planche-mouvement__squelette gn-planche-mouvement__squelette--courte" />
        </div>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-mouvement.ephemere')"
        :propos="t('gn-planche-mouvement.ephemere-propos')"
      >
        <div class="gn-planche-mouvement__carte">
          <span class="gn-planche-jeton">--gn-duree-ephemere</span>
          <span class="gn-planche-valeur">
            {{ valeur('--gn-duree-ephemere') }} — {{ t('gn-planche-mouvement.usage.ephemere') }}
          </span>
          <GnBouton variante="secondaire" @clic="jouerEphemere">
            {{ t('gn-planche-mouvement.envoyer') }}
          </GnBouton>
          <template v-if="messageVisible">
            <p :key="envoi" class="gn-planche-mouvement__message" role="status">
              {{ t('gn-planche-mouvement.ephemere-message') }}
            </p>
            <div :key="`jauge-${envoi}`" class="gn-planche-mouvement__compte">
              <div
                class="gn-planche-mouvement__compte-reste"
                :class="{ 'gn-planche-mouvement__compte-reste--ecoule': compteEcoule }"
              />
            </div>
          </template>
        </div>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-mouvement.reduit')"
        :propos="t('gn-planche-mouvement.reduit-propos')"
      >
        <p class="gn-planche-mouvement__verdict" role="status">
          <GnPicto :nom="reduit ? 'check' : 'info'" :taille="20" />
          {{ reduit ? t('gn-planche-mouvement.reduit-actif') : t('gn-planche-mouvement.reduit-inactif') }}
        </p>
        <p class="gn-planche-note">{{ t('gn-planche-mouvement.reduit-preuve') }}</p>
        <p class="gn-planche-note">{{ t('gn-planche-mouvement.immobile') }}</p>
      </GnPlancheSection>
    </div>
  </GnPlancheSection>
</template>

<style>
[data-app="guide-nego"] .gn-planche-mouvement {
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-planche-mouvement__carte {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding: var(--gn-espace-12);
  border: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-mouvement__voie {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
}

/* Le pion se déplace par `left` et non par `transform` : la course doit s'arrêter au
   bord de la piste, dont la largeur n'est connue qu'à l'affichage. */
[data-app="guide-nego"] .gn-planche-mouvement__piste {
  position: relative;
  height: var(--gn-case);
  background: var(--gn-fond-2);
}

[data-app="guide-nego"] .gn-planche-mouvement__pion {
  position: absolute;
  top: 0;
  left: 0;
  width: var(--gn-case);
  height: var(--gn-case);
  background: var(--gn-accent);
  transition-property: left;
}

[data-app="guide-nego"] .gn-planche-mouvement__pion--bout {
  left: calc(100% - var(--gn-case));
}

[data-app="guide-nego"] .gn-planche-mouvement__arc {
  width: var(--gn-picto-taille);
  height: var(--gn-picto-taille);
  border: var(--gn-filet-2) solid var(--gn-accent);
  border-top-color: transparent;
  border-radius: var(--gn-rayon-24);
  animation: gn-spin var(--gn-duree-arc) linear infinite;
}

[data-app="guide-nego"] .gn-planche-mouvement__squelette {
  height: var(--gn-espace-16);
  background: var(--gn-squelette);
  animation: gn-pulse var(--gn-duree-squelette) ease-in-out infinite;
}

[data-app="guide-nego"] .gn-planche-mouvement__squelette--courte {
  width: 60%;
}

[data-app="guide-nego"] .gn-planche-mouvement__message {
  padding: var(--gn-espace-12);
  background: var(--gn-titre);
  color: var(--gn-sur-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-planche-mouvement__compte {
  height: var(--gn-jauge);
  background: var(--gn-jauge-fond);
}

[data-app="guide-nego"] .gn-planche-mouvement__compte-reste {
  width: 100%;
  height: 100%;
  background: var(--gn-accent);
  transition-property: width;
  transition-duration: var(--gn-duree-ephemere);
  transition-timing-function: linear;
}

[data-app="guide-nego"] .gn-planche-mouvement__compte-reste--ecoule {
  width: 0;
}

[data-app="guide-nego"] .gn-planche-mouvement__verdict {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  min-height: var(--gn-cible);
  color: var(--gn-texte);
  font-size: var(--gn-taille-17);
  font-weight: var(--gn-graisse-demi-gras);
  line-height: var(--gn-interligne-17);
}
</style>
