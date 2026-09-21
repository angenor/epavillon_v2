<script setup lang="ts">
import { useJetonsLus } from './PlancheSection.vue'

/**
 * Section 2 — l'échelle typographique, rendue à sa taille.
 *
 * Elle montre aussi ce qui a décidé du choix d'Atkinson Hyperlegible Next : la
 * ligature œ, les capitales accentuées, et des chiffres de largeur fixe qui alignent
 * une colonne d'horaires. Une planche qui ne le prouve pas ne sert à rien.
 */
const ECHELLE = ['32', '28', '24', '20', '17', '15', '13'] as const
const GRAISSES = ['regulier', 'demi-gras', 'gras'] as const
const LECTURE = ['--gn-lecture-normale', '--gn-lecture-grande', '--gn-lecture-tres-grande'] as const

const MESURES = [
  '--gn-police',
  ...ECHELLE.flatMap((cran) => [`--gn-taille-${cran}`, `--gn-interligne-${cran}`]),
  ...GRAISSES.map((graisse) => `--gn-graisse-${graisse}`),
  ...LECTURE,
]

const { t } = useI18n()
const racine = useTemplateRef<HTMLElement>('racine')
const lus = useJetonsLus(racine, [], MESURES)

const valeur = (jeton: string): string => lus.value[jeton] ?? '…'
const cran = (taille: string) => ({
  fontSize: `var(--gn-taille-${taille})`,
  lineHeight: `var(--gn-interligne-${taille})`,
})
const poids = (graisse: string) => ({ fontWeight: `var(--gn-graisse-${graisse})` })
</script>

<template>
  <GnPlancheSection
    numero="2"
    :titre="t('gn-planche-typographie.titre')"
    :propos="t('gn-planche-typographie.propos')"
  >
    <div ref="racine" class="gn-planche-typographie">
      <p class="gn-planche-note">{{ t('gn-planche-typographie.police') }} {{ valeur('--gn-police') }}</p>

      <GnPlancheSection
        :titre="t('gn-planche-typographie.echelle')"
        :propos="t('gn-planche-typographie.echelle-propos')"
      >
        <div class="gn-planche-liste">
          <div v-for="taille in ECHELLE" :key="taille" class="gn-planche-typographie__cran">
            <div class="gn-planche-legende gn-planche-typographie__reperes">
              <span class="gn-planche-jeton">--gn-taille-{{ taille }}</span>
              <span class="gn-planche-valeur">
                {{ valeur(`--gn-taille-${taille}`) }} · {{ t('gn-planche-typographie.interligne') }}
                {{ valeur(`--gn-interligne-${taille}`) }}
              </span>
              <span class="gn-planche-valeur">{{ t(`gn-planche-typographie.usage.${taille}`) }}</span>
            </div>
            <p class="gn-planche-typographie__exemple" :style="cran(taille)">
              {{ t('gn-planche-typographie.exemple') }}
            </p>
          </div>
        </div>
        <p class="gn-planche-note">{{ t('gn-planche-typographie.regle-13') }}</p>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-typographie.graisses')"
        :propos="t('gn-planche-typographie.graisses-propos')"
      >
        <div class="gn-planche-liste">
          <div v-for="graisse in GRAISSES" :key="graisse" class="gn-planche-ligne">
            <span class="gn-planche-ligne__nom">
              <span class="gn-planche-jeton">--gn-graisse-{{ graisse }}</span>
              <span class="gn-planche-valeur"> · {{ valeur(`--gn-graisse-${graisse}`) }}</span>
            </span>
          </div>
        </div>
        <p v-for="graisse in GRAISSES" :key="graisse" :style="poids(graisse)">
          {{ t('gn-planche-typographie.exemple') }}
        </p>
        <p class="gn-planche-typographie__italique" :style="poids('regulier')">
          {{ t('gn-planche-typographie.italique-400') }}
        </p>
        <p class="gn-planche-typographie__italique" :style="poids('demi-gras')">
          {{ t('gn-planche-typographie.italique-600') }}
        </p>
        <p class="gn-planche-note">{{ t('gn-planche-typographie.italique-regle') }}</p>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-typographie.lettres')"
        :propos="t('gn-planche-typographie.lettres-propos')"
      >
        <p class="gn-planche-typographie__glyphes" :style="cran('32')">œ Œ æ Æ ç Ç</p>
        <p class="gn-planche-typographie__glyphes" :style="cran('32')">À É È Ê Ç Î Ô Ù Û Ÿ</p>
        <p class="gn-planche-typographie__glyphes" :style="cran('24')">« » — – … ’ €</p>
        <p :style="cran('17')">{{ t('gn-planche-typographie.phrase') }}</p>
        <p class="gn-planche-typographie__glyphes" :style="cran('24')">Il1 lI1 O0 oO rn m</p>
        <p class="gn-planche-note">{{ t('gn-planche-typographie.lettres-note') }}</p>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-typographie.chiffres')"
        :propos="t('gn-planche-typographie.chiffres-propos')"
      >
        <div class="gn-planche-typographie__horaires">
          <div class="gn-planche-typographie__colonne">
            <span class="gn-planche-valeur">{{ t('gn-planche-typographie.tabulaire') }}</span>
            <span :style="cran('20')">09:00</span>
            <span :style="cran('20')">11:45</span>
            <span :style="cran('20')">14:30</span>
            <span :style="cran('20')">18:15</span>
          </div>
          <div class="gn-planche-typographie__colonne gn-planche-typographie__colonne--proportionnelle">
            <span class="gn-planche-valeur">{{ t('gn-planche-typographie.proportionnel') }}</span>
            <span :style="cran('20')">09:00</span>
            <span :style="cran('20')">11:45</span>
            <span :style="cran('20')">14:30</span>
            <span :style="cran('20')">18:15</span>
          </div>
        </div>
        <p class="gn-planche-note">{{ t('gn-planche-typographie.chiffres-note') }}</p>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-typographie.lecture')"
        :propos="t('gn-planche-typographie.lecture-propos')"
      >
        <div class="gn-planche-liste">
          <div v-for="jeton in LECTURE" :key="jeton" class="gn-planche-ligne">
            <span class="gn-planche-ligne__nom">
              <span class="gn-planche-jeton">{{ jeton }}</span>
              <span class="gn-planche-valeur"> · {{ valeur(jeton) }}</span>
            </span>
            <span :style="{ fontSize: `var(${jeton})` }">{{ t('gn-planche-typographie.aa') }}</span>
          </div>
        </div>
      </GnPlancheSection>
    </div>
  </GnPlancheSection>
</template>

<style>
[data-app="guide-nego"] .gn-planche-typographie {
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-planche-typographie__cran {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-block: var(--gn-ligne-air);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-planche-liste > .gn-planche-typographie__cran:first-child {
  border-top: none;
}

[data-app="guide-nego"] .gn-planche-typographie__reperes {
  padding: 0;
}

[data-app="guide-nego"] .gn-planche-typographie__exemple {
  color: var(--gn-titre);
  font-weight: var(--gn-graisse-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-planche-typographie__italique {
  font-style: italic;
}

[data-app="guide-nego"] .gn-planche-typographie__glyphes {
  color: var(--gn-titre);
  font-weight: var(--gn-graisse-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-planche-typographie__horaires {
  display: flex;
  gap: var(--gn-espace-24);
}

[data-app="guide-nego"] .gn-planche-typographie__colonne {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
  font-weight: var(--gn-graisse-gras);
}

/* Le seul endroit du système où les chiffres ne sont pas tabulaires : il est là pour
   montrer ce que l'application éviterait si elle s'en passait. */
[data-app="guide-nego"] .gn-planche-typographie__colonne--proportionnelle {
  font-variant-numeric: proportional-nums;
}
</style>
