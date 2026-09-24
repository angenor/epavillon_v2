<script setup lang="ts">
import { tailleLisible } from '~/utils/guide-nego/place'

/**
 * L'attente de la première page lue en ligne (FR-009 bis) : ce qui est reçu sur ce qui
 * est demandé, puis, au bout de 3 s, deux sorties. En `ligne`, pendant qu'on lit le
 * texte en attendant : la page arrive, et on peut choisir d'y rester.
 */
const props = withDefaults(
  defineProps<{
    variante?: 'plein' | 'ligne'
    recu: number
    demande: number
    sorties?: boolean
    /** « Texte agrandi » est offert : sans lui, pas de lecture en attendant. */
    texteOffert?: boolean
  }>(),
  { variante: 'plein', sorties: false, texteOffert: false },
)

const emit = defineEmits<{ lireLeTexte: []; telecharger: []; rester: [] }>()

const { t, locale } = useI18n()

const part = computed(() => (props.demande > 0 ? props.recu / props.demande : null))
const recus = computed(() =>
  props.demande > 0
    ? t('gn-attente-pages.recus', {
        recu: tailleLisible(props.recu, locale.value),
        demande: tailleLisible(props.demande, locale.value),
      })
    : t('gn-attente-pages.demande'),
)
</script>

<template>
  <div v-if="props.variante === 'plein'" class="gn-attente-pages">
    <GnChargement decoratif />
    <p class="gn-attente-pages__titre" role="status">{{ t('gn-attente-pages.titre') }}</p>
    <GnProgression :part="part" :libelle="recus" />
    <p class="gn-attente-pages__recus">{{ recus }}</p>
    <div v-if="props.sorties" class="gn-attente-pages__sorties">
      <p class="gn-attente-pages__lent">{{ t('gn-attente-pages.lent') }}</p>
      <GnBouton v-if="props.texteOffert" picto="text-size" @clic="emit('lireLeTexte')">
        {{ t('gn-attente-pages.lire-le-texte') }}
      </GnBouton>
      <GnBouton variante="secondaire" picto="download" @clic="emit('telecharger')">
        {{ t('gn-attente-pages.telecharger') }}
      </GnBouton>
    </div>
  </div>

  <p v-else class="gn-attente-pages__ligne" role="status">
    <GnChargement :taille="20" decoratif />
    <span class="gn-attente-pages__ligne-texte">{{ t('gn-attente-pages.page-arrive') }}</span>
    <button type="button" class="gn-attente-pages__rester" @click="emit('rester')">
      {{ t('gn-attente-pages.rester') }}
    </button>
  </p>
</template>

<style>
[data-app="guide-nego"] .gn-attente-pages {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
  padding-top: var(--gn-espace-32);
}

[data-app="guide-nego"] .gn-attente-pages__titre {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-attente-pages__recus,
[data-app="guide-nego"] .gn-attente-pages__lent {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-attente-pages__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-16);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
}

/* Au-dessus de la barre repliée, comme la ligne de reprise (04 · 03). */
[data-app="guide-nego"] .gn-attente-pages__ligne {
  position: fixed;
  bottom: calc(var(--gn-barre-lecture-repliee) + var(--gn-jauge) + var(--gn-espace-12) + env(safe-area-inset-bottom));
  left: 50%;
  transform: translateX(-50%);
  z-index: 5;
  width: calc(min(100%, var(--gn-colonne-largeur)) - 2 * var(--gn-marge-ecran));
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  min-height: var(--gn-cible);
  padding: 0 var(--gn-espace-8) 0 var(--gn-espace-16);
  background: var(--gn-titre);
  color: var(--gn-sur-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-attente-pages__ligne .gn-chargement {
  color: var(--gn-sur-titre);
}

[data-app="guide-nego"] .gn-attente-pages__ligne-texte {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-attente-pages__rester {
  flex: none;
  min-height: var(--gn-cible);
  padding-inline: var(--gn-espace-8);
  border: none;
  background: none;
  color: var(--gn-action-sur-titre);
  font: inherit;
  font-weight: var(--gn-graisse-gras);
  text-decoration-line: var(--gn-action-sur-titre-trait);
  text-underline-offset: var(--gn-espace-4);
  cursor: pointer;
}
</style>
