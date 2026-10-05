<script setup lang="ts">
import { useJetonsLus } from './PlancheSection.vue'

/**
 * Section 2 : l'échelle typographique de la maquette Nuit, rendue à sa taille, dans sa
 * famille (Sora pour les titres et les chiffres qui portent le sens, Manrope pour le
 * reste) et sa graisse. Elle montre aussi la ligature œ, les capitales accentuées et
 * pourquoi les chiffres restent proportionnels.
 */
type Famille = 'titre' | 'texte'
const ECHELLE: readonly { taille: string; famille: Famille; graisse: string; approche?: string }[] = [
  { taille: '76', famille: 'titre', graisse: 'extra-gras', approche: '76' },
  { taille: '44', famille: 'titre', graisse: 'extra-gras', approche: '44' },
  { taille: '28', famille: 'titre', graisse: 'gras', approche: '28' },
  { taille: '22', famille: 'titre', graisse: 'gras' },
  { taille: '20', famille: 'titre', graisse: 'demi-gras' },
  { taille: '18', famille: 'titre', graisse: 'gras' },
  { taille: '17', famille: 'texte', graisse: 'regulier' },
  { taille: '16', famille: 'texte', graisse: 'gras' },
  { taille: '15', famille: 'texte', graisse: 'demi-gras' },
  { taille: '14', famille: 'texte', graisse: 'regulier' },
  { taille: '13', famille: 'texte', graisse: 'regulier' },
  { taille: '12', famille: 'texte', graisse: 'extra-gras', approche: '12' },
  { taille: '11', famille: 'texte', graisse: 'extra-gras', approche: '11' },
]
const GRAISSES = ['regulier', 'moyen', 'demi-gras', 'gras', 'extra-gras'] as const
const LECTURE = ['--gn-lecture-normale', '--gn-lecture-grande', '--gn-lecture-tres-grande'] as const

const MESURES = [
  '--gn-police',
  '--gn-police-titre',
  ...ECHELLE.flatMap(({ taille }) => [`--gn-taille-${taille}`, `--gn-interligne-${taille}`]),
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
const dessinDu = (c: (typeof ECHELLE)[number]) => ({
  ...cran(c.taille),
  fontFamily: `var(${c.famille === 'titre' ? '--gn-police-titre' : '--gn-police'})`,
  fontWeight: `var(--gn-graisse-${c.graisse})`,
  letterSpacing: c.approche ? `var(--gn-approche-${c.approche})` : undefined,
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
      <p class="gn-planche-note">{{ t('gn-planche-typographie.police-titre') }} {{ valeur('--gn-police-titre') }}</p>
      <p class="gn-planche-note">{{ t('gn-planche-typographie.police') }} {{ valeur('--gn-police') }}</p>

      <GnPlancheSection
        :titre="t('gn-planche-typographie.echelle')"
        :propos="t('gn-planche-typographie.echelle-propos')"
      >
        <div class="gn-planche-liste">
          <div v-for="c in ECHELLE" :key="c.taille" class="gn-planche-typographie__cran">
            <div class="gn-planche-legende gn-planche-typographie__reperes">
              <span class="gn-planche-jeton">--gn-taille-{{ c.taille }}</span>
              <span class="gn-planche-valeur">
                {{ t(`gn-planche-typographie.famille.${c.famille}`) }} · {{ valeur(`--gn-taille-${c.taille}`) }} ·
                {{ t('gn-planche-typographie.interligne') }} {{ valeur(`--gn-interligne-${c.taille}`) }}
              </span>
              <span class="gn-planche-valeur">{{ t(`gn-planche-typographie.usage.${c.taille}`) }}</span>
            </div>
            <p class="gn-planche-typographie__exemple" :style="dessinDu(c)">
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
        <p class="gn-planche-typographie__glyphes" :style="cran('24')">« » – … ’ €</p>
        <p :style="cran('17')">{{ t('gn-planche-typographie.phrase') }}</p>
        <p class="gn-planche-typographie__glyphes" :style="cran('24')">Il1 lI1 O0 oO rn m</p>
        <p class="gn-planche-note">{{ t('gn-planche-typographie.lettres-note') }}</p>
      </GnPlancheSection>

      <GnPlancheSection
        :titre="t('gn-planche-typographie.chiffres')"
        :propos="t('gn-planche-typographie.chiffres-propos')"
      >
        <div class="gn-planche-typographie__horaires">
          <div class="gn-planche-typographie__colonne gn-planche-typographie__colonne--tabulaire">
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
  border-top: var(--gn-filet-1) solid var(--gn-filet-doux);
}

[data-app="guide-nego"] .gn-planche-liste > .gn-planche-typographie__cran:first-child {
  border-top: none;
}

[data-app="guide-nego"] .gn-planche-typographie__reperes {
  padding: 0;
}

[data-app="guide-nego"] .gn-planche-typographie__exemple {
  color: var(--gn-titre);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-planche-typographie__italique {
  font-style: italic;
}

[data-app="guide-nego"] .gn-planche-typographie__glyphes {
  color: var(--gn-titre);
  font-family: var(--gn-police-titre);
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

/* Le seul endroit du système où les chiffres sont tabulaires : il montre le blanc que
   Manrope laisse autour du 1, que la maquette n'a pas. */
[data-app="guide-nego"] .gn-planche-typographie__colonne--tabulaire {
  font-variant-numeric: tabular-nums;
}
</style>
