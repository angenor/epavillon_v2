<script setup lang="ts">
import type { FrancophoneMeeting } from '~/types/negotiation-meetings'
import type { EtatReunion } from '~/utils/guide-nego/reunions'

/**
 * Une réunion de la Francophonie dans sa liste (10 · 1a) : une seule marque d'état.
 *
 * Le lien couvre toute la ligne par son pseudo-élément, et l'étiquette Pavillon passe
 * au-dessus : un lien dans un lien n'est pas du HTML, et un lecteur d'écran s'y perd.
 */
const props = withDefaults(
  defineProps<{
    reunion: FrancophoneMeeting
    etat: EtatReunion
    fuseau: string
    /** Le nom du lieu pour « heure d'Antalya » ; à défaut, celui du fuseau. */
    ville?: string | null
    vers: string
  }>(),
  { ville: null },
)

const { t } = useI18n()
const { intlLocale, time, timeRange, dayLong } = useDateTime()
const { tr } = useI18nText()

const jourCourt = computed(() =>
  new Intl.DateTimeFormat(intlLocale.value, { weekday: 'short', day: 'numeric', timeZone: props.fuseau }).format(
    new Date(props.reunion.start_at),
  ),
)
const debut = computed(() => time(props.reunion.start_at, props.fuseau))
const fin = computed(() => time(props.reunion.end_at, props.fuseau))
const entendu = computed(() =>
  t('gn-ligne-reunion.entendu', {
    jour: dayLong(props.reunion.start_at, props.fuseau),
    heures: timeRange(props.reunion.start_at, props.reunion.end_at, props.fuseau, props.ville ?? undefined),
  }),
)

const lieu = computed(() => {
  const r = props.reunion
  if (r.format === 'online') return t('gn-ligne-reunion.en-ligne')
  if (!r.venue) return r.format === 'hybrid' ? t('gn-ligne-reunion.en-ligne') : t('gn-ligne-reunion.lieu-inconnu')
  return r.format === 'hybrid' ? t('gn-ligne-reunion.et-en-ligne', { lieu: r.venue }) : r.venue
})

const accesLimite = computed(() => {
  const r = props.reunion
  if (r.open_access) return null
  const public_ = r.access_audience ? tr(r.access_audience) : ''
  return public_ ? t('gn-ligne-reunion.acces-limite', { public: public_ }) : t('gn-marque-etat.acces-limite')
})
</script>

<template>
  <div
    class="gn-ligne-reunion"
    :class="{ 'gn-ligne-reunion--annulee': etat === 'annulee', 'gn-ligne-reunion--terminee': etat === 'terminee' }"
  >
    <span class="gn-ligne-reunion__heures" aria-hidden="true">
      <span class="gn-ligne-reunion__jour">{{ jourCourt }}</span>
      <span class="gn-ligne-reunion__debut">{{ debut }}</span>
      <span class="gn-ligne-reunion__fin">{{ fin }}</span>
    </span>

    <span class="gn-ligne-reunion__corps">
      <span class="gn-ligne-reunion__origine">{{ t('gn-ligne-reunion.origine') }}</span>
      <NuxtLink :to="vers" class="gn-ligne-reunion__lien">
        <span class="gn-hors-ecran">{{ entendu }}</span>
        <span class="gn-ligne-reunion__titre">{{ tr(reunion.title) }}</span>
      </NuxtLink>
      <span class="gn-ligne-reunion__lieu">
        <GnPicto nom="pin" :taille="18" />
        {{ lieu }}
      </span>
      <GnMarqueEtat v-if="accesLimite" etat="acces-limite" :libelle="accesLimite" />
      <GnMarqueEtat :etat="etat" />
      <GnEtiquettePavillon v-if="reunion.pavilion_session_id" class="gn-ligne-reunion__pavillon" />
    </span>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-ligne-reunion {
  position: relative;
  min-height: var(--gn-cible);
  padding-block: var(--gn-espace-12);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-ligne-reunion:has(.gn-ligne-reunion__lien:active) {
  background: var(--gn-presse);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-ligne-reunion__lien {
  color: inherit;
  text-decoration: none;
}

/* Toute la ligne ouvre la fiche. */
[data-app="guide-nego"] .gn-ligne-reunion__lien::after {
  content: '';
  position: absolute;
  inset: 0;
}

[data-app="guide-nego"] .gn-ligne-reunion__lien:focus-visible {
  outline: none;
}

[data-app="guide-nego"] .gn-ligne-reunion__lien:focus-visible::after {
  outline: var(--gn-focus-anneau) solid var(--gn-focus);
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-ligne-reunion__pavillon {
  position: relative;
  z-index: 1;
}

[data-app="guide-nego"] .gn-ligne-reunion__heures {
  flex: none;
  width: var(--gn-colonne-heure);
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  font-variant-numeric: tabular-nums;
}

[data-app="guide-nego"] .gn-ligne-reunion__jour,
[data-app="guide-nego"] .gn-ligne-reunion__origine {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-ligne-reunion__debut {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-ligne-reunion__fin {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-ligne-reunion__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--gn-espace-4);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-ligne-reunion__titre {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-ligne-reunion__lieu {
  display: inline-flex;
  align-items: flex-start;
  gap: 6px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-ligne-reunion__lieu .gn-picto {
  flex: none;
  margin-block-start: 2px;
}

/* Terminée : tout passe en gris. */
[data-app="guide-nego"] .gn-ligne-reunion--terminee .gn-ligne-reunion__debut,
[data-app="guide-nego"] .gn-ligne-reunion--terminee .gn-ligne-reunion__titre {
  color: var(--gn-texte-2);
}

/* Annulée : elle garde sa place, son titre est barré. */
[data-app="guide-nego"] .gn-ligne-reunion--annulee .gn-ligne-reunion__titre {
  color: var(--gn-texte-2);
  text-decoration: line-through;
  text-decoration-thickness: 2px;
}
</style>
