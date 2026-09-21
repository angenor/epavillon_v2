<script setup lang="ts">
/**
 * La planche de design, rendue par les vrais jetons et les vrais composants.
 *
 * Aucun lien de l'application n'y mène et elle est en `noindex` : c'est un outil de
 * travail, pas un écran. On y vient par son adresse.
 *
 * Les deux thèmes se montrent ensemble parce qu'un rôle ne se juge pas seul : c'est
 * l'écart entre le clair et le sombre qui révèle un jeton oublié. Chaque volet porte
 * `data-app` ET `data-theme` — les jetons sombres sont déclarés sur les deux attributs
 * du MÊME élément, un conteneur qui ne porterait que le thème ne basculerait pas.
 */
import type { SegmentDeChoix } from '~/components/guide-nego/GnSegmente.vue'

definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()

type Affichage = 'clair' | 'sombre' | 'cote-a-cote'

const affichage = ref<Affichage>('clair')

const segments = computed<SegmentDeChoix[]>(() => [
  { valeur: 'clair', libelle: t('guide-nego.composants.clair'), picto: 'sun' },
  { valeur: 'sombre', libelle: t('guide-nego.composants.sombre'), picto: 'moon' },
  { valeur: 'cote-a-cote', libelle: t('guide-nego.composants.cote-a-cote') },
])

const volets = computed<('clair' | 'sombre')[]>(() =>
  affichage.value === 'cote-a-cote' ? ['clair', 'sombre'] : [affichage.value],
)

const bascule = computed({
  get: () => affichage.value as string,
  set: (valeur: string) => (affichage.value = valeur as Affichage),
})

/** Les sept ancres, dans l'ordre de la maquette. Le numéro sert de clé et d'ancre. */
const sections = ['1', '2', '3', '4', '5', '6', '7'] as const

// Côte à côte, les deux volets portent le même contenu : seul le premier reçoit les
// ancres, sinon le document aurait sept identifiants en double et les liens ne
// mèneraient nulle part de sûr.
function ancre(numero: string, theme: string): string | undefined {
  return theme === volets.value[0] ? `gn-planche-${numero}` : undefined
}

useHead({
  title: t('guide-nego.composants.titre'),
  meta: [{ name: 'robots', content: 'noindex' }],
})
</script>

<template>
  <div class="gn-planche">
    <header class="gn-planche__entete">
      <h1 class="gn-planche__titre">{{ t('guide-nego.composants.titre') }}</h1>
      <p class="gn-planche__propos">{{ t('guide-nego.composants.propos') }}</p>
      <GnSegmente
        v-model="bascule"
        :segments="segments"
        :libelle="t('guide-nego.composants.affichage')"
      />
      <nav class="gn-planche__sommaire" :aria-label="t('guide-nego.composants.sommaire')">
        <a v-for="numero in sections" :key="numero" :href="`#gn-planche-${numero}`">
          {{ numero }} · {{ t(`guide-nego.composants.section.${numero}`) }}
        </a>
      </nav>
    </header>

    <div class="gn-planche__volets" :class="{ 'gn-planche__volets--deux': volets.length === 2 }">
      <div
        v-for="theme in volets"
        :key="theme"
        data-app="guide-nego"
        :data-theme="theme"
        class="gn-planche__volet"
      >
        <p class="gn-planche__nom-de-volet">{{ t(`guide-nego.composants.${theme}`) }}</p>
        <div :id="ancre('1', theme)"><GnPlancheCouleurs /></div>
        <div :id="ancre('2', theme)"><GnPlancheTypographie /></div>
        <div :id="ancre('3', theme)"><GnPlancheMesures /></div>
        <div :id="ancre('4', theme)"><GnPlanchePictogrammes /></div>
        <div :id="ancre('5', theme)"><GnPlancheComposants /></div>
        <div :id="ancre('6', theme)"><GnPlancheEtats /></div>
        <div :id="ancre('7', theme)"><GnPlancheMouvement /></div>
      </div>
    </div>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-planche {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: var(--gn-entre-blocs);
  padding: var(--gn-espace-24) var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-planche__entete {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-planche__titre {
  color: var(--gn-titre);
  font-size: var(--gn-taille-28);
  line-height: var(--gn-interligne-28);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-planche__propos {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-planche__sommaire {
  display: flex;
  flex-wrap: wrap;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-planche__sommaire a {
  min-height: var(--gn-cible);
  padding-inline: var(--gn-espace-12);
  display: inline-flex;
  align-items: center;
  border: var(--gn-filet-1) solid var(--gn-filet);
  border-radius: var(--gn-rayon-24);
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-planche__volets--deux {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--gn-espace-16);
}

/* Un volet est une racine de thème imbriquée : il redéclare tous les jetons. Il faut
   donc lui retirer ce que `base.css` réserve à la racine de l'application — la hauteur
   d'écran et le centrage de colonne, qui n'ont ici aucun sens. */
[data-app="guide-nego"] .gn-planche__volet {
  min-height: 0;
  padding: var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-32);
}

[data-app="guide-nego"] .gn-planche__nom-de-volet {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-gras);
  text-transform: uppercase;
  letter-spacing: 0.02em;
}
</style>
