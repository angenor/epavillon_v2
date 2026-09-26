<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { EtatActivite, MarqueActivite } from '~/utils/guide-nego/pavillon'

/**
 * Une activité du Pavillon dans sa liste (10 · 1c) : l'état de l'activité, puis
 * l'inscription ou la rediffusion. Le lien couvre la ligne par son pseudo-élément ; la
 * rediffusion passe au-dessus, comme l'étiquette Pavillon d'une réunion.
 */
const props = withDefaults(
  defineProps<{
    activite: PublicScheduleRow
    etat: EtatActivite
    marque: MarqueActivite | null
    fuseau: string
    /** Le nom du lieu pour « heure d'Antalya » ; à défaut, celui du fuseau. */
    ville?: string | null
    vers: string
  }>(),
  { ville: null },
)

const { t } = useI18n()
const { time, timeRange, dayLong } = useDateTime()
const { tr } = useI18nText()

const debut = computed(() => time(props.activite.starts_at, props.fuseau))
const entendu = computed(() =>
  t('gn-ligne-activite.entendu', {
    jour: dayLong(props.activite.starts_at, props.fuseau),
    heures: timeRange(props.activite.starts_at, props.activite.ends_at, props.fuseau, props.ville ?? undefined),
  }),
)

const rediffusion = computed(() => (props.marque?.nom === 'rediffusion' ? props.marque : null))
const libelleRediffusion = computed(() =>
  rediffusion.value?.minutes
    ? t('gn-ligne-activite.rediffusion', { minutes: rediffusion.value.minutes })
    : t('gn-marque-etat.rediffusion'),
)
const attente = computed(() => {
  const m = props.marque
  if (m?.nom !== 'liste-attente') return undefined
  return m.position ? t('gn-ligne-activite.attente-position', { position: m.position }) : undefined
})
</script>

<template>
  <div
    class="gn-ligne-activite"
    :class="{ 'gn-ligne-activite--terminee': etat === 'terminee', 'gn-ligne-activite--annulee': etat === 'annulee' }"
  >
    <span class="gn-ligne-activite__heures" aria-hidden="true">
      <span
        class="gn-ligne-activite__debut"
        :class="{ 'gn-ligne-activite__debut--en-cours': etat === 'en-cours' }"
      >{{ debut }}</span>
    </span>

    <span class="gn-ligne-activite__corps">
      <span class="gn-ligne-activite__origine">{{ t('gn-ligne-activite.origine') }}</span>
      <NuxtLink :to="vers" class="gn-ligne-activite__lien">
        <span class="gn-hors-ecran">{{ entendu }}</span>
        <span class="gn-ligne-activite__titre">{{ tr(activite.title) }}</span>
      </NuxtLink>
      <GnMarqueEtat :etat="etat" />
      <a
        v-if="rediffusion"
        :href="rediffusion.url"
        class="gn-ligne-activite__rediffusion"
        target="_blank"
        rel="noopener"
      >
        <GnMarqueEtat etat="rediffusion" :libelle="libelleRediffusion" />
      </a>
      <GnMarqueEtat v-else-if="marque?.nom === 'inscrite'" etat="inscrite" />
      <GnMarqueEtat v-else-if="marque?.nom === 'liste-attente'" etat="liste-attente" :libelle="attente" />
      <GnMarqueEtat v-else-if="marque?.nom === 'complet'" etat="complet" />
      <span v-else-if="marque?.nom === 'sans-inscription'" class="gn-ligne-activite__libre">
        {{ t('gn-ligne-activite.sans-inscription') }}
      </span>
    </span>

    <GnPicto nom="chevron" :taille="24" class="gn-ligne-activite__chevron" />
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-ligne-activite {
  position: relative;
  min-height: var(--gn-cible);
  padding-block: var(--gn-ligne-air);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-ligne-activite:has(.gn-ligne-activite__lien:active) {
  background: var(--gn-presse);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-ligne-activite__lien {
  color: inherit;
  text-decoration: none;
}

/* Toute la ligne ouvre la fiche. */
[data-app="guide-nego"] .gn-ligne-activite__lien::after {
  content: '';
  position: absolute;
  inset: 0;
}

[data-app="guide-nego"] .gn-ligne-activite__lien:focus-visible {
  outline: none;
}

[data-app="guide-nego"] .gn-ligne-activite__lien:focus-visible::after {
  outline: var(--gn-focus-anneau) solid var(--gn-focus);
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-ligne-activite__heures {
  flex: none;
  width: var(--gn-colonne-heure);
  display: flex;
  align-items: flex-start;
}

[data-app="guide-nego"] .gn-ligne-activite__debut {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  font-variant-numeric: tabular-nums;
  color: var(--gn-titre);
}

/* Le jaune dit « maintenant » : il couvre le début seul, comme sur une session. */
[data-app="guide-nego"] .gn-ligne-activite__debut--en-cours {
  padding-inline: var(--gn-espace-4);
  margin-inline-start: calc(-1 * var(--gn-espace-4));
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
}

[data-app="guide-nego"] .gn-ligne-activite__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-ligne-activite__origine {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-ligne-activite__titre {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

/* La rediffusion s'ouvre d'ici : sa cible fait 48 px et passe au-dessus du lien de la ligne. */
[data-app="guide-nego"] .gn-ligne-activite__rediffusion {
  position: relative;
  z-index: 1;
  min-height: var(--gn-cible);
  margin-block: -13px;
  display: inline-flex;
  align-items: center;
  text-decoration: underline;
  text-underline-offset: 4px;
  color: var(--gn-etat-rediffusion);
}

[data-app="guide-nego"] .gn-ligne-activite__libre {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-ligne-activite__chevron {
  flex: none;
  color: var(--gn-picto-secondaire);
}

/* Terminée : tout passe en gris. */
[data-app="guide-nego"] .gn-ligne-activite--terminee .gn-ligne-activite__debut,
[data-app="guide-nego"] .gn-ligne-activite--terminee .gn-ligne-activite__titre {
  color: var(--gn-texte-2);
}

/* Annulée : elle garde sa place, son titre est barré. */
[data-app="guide-nego"] .gn-ligne-activite--annulee .gn-ligne-activite__titre {
  color: var(--gn-texte-2);
  text-decoration: line-through;
  text-decoration-thickness: 2px;
}
</style>
