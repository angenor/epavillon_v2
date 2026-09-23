<script setup lang="ts">
import type { LibraryDocument } from '~/types/negotiation-documents'
import { ETAT_DE_LA_MARQUE, type MarqueDeLigne } from '~/utils/guide-nego/documents'
import { tailleLisible } from '~/utils/guide-nego/place'

const props = defineProps<{
  document: LibraryDocument
  /** Libellé du type, déjà résolu depuis le vocabulaire servi par l'API — jamais un fichier i18n. */
  type: string
  marques: MarqueDeLigne[]
  /** Chemin de la fiche. */
  vers: string
  /** Téléchargement en cours : octets reçus et total (nul si inconnu). */
  progression?: { recus: number; total: number | null } | null
  /** La recherche a trouvé le mot dans le texte, pas dans le titre : l'étiquette imprimée et l'extrait. */
  trouve?: { page: string; extrait: string } | null
}>()

const { t, locale } = useI18n()

const estLien = computed(() => props.document.source === 'link')

const marquesAffichees = computed(() =>
  props.marques.map((marque) => {
    let libelle: string | undefined
    if (marque === 'lien-reseau' || marque === 'non-telecharge') {
      libelle = t(`gn-ligne-document.marque.${marque}`)
    } else if (marque === 'remplace' && props.document.superseded_by) {
      libelle = t('gn-ligne-document.marque.remplace', { titre: props.document.superseded_by.title })
    }
    return { marque, etat: ETAT_DE_LA_MARQUE[marque], libelle }
  }),
)

/** Sans réseau, un document absent du téléphone ne s'ouvre pas : la maquette l'éteint. */
const indisponible = computed(() =>
  props.marques.includes('non-telecharge') || props.marques.includes('lien-reseau'),
)
const eteint = computed(() => indisponible.value || props.marques.includes('remplace'))

const meta = computed(() => {
  const d = props.document
  const annee = estLien.value ? null : (d.issued_on?.slice(0, 4) ?? null)
  const origine =
    d.publisher && annee
      ? t('gn-ligne-document.editeur-annee', { editeur: d.publisher, annee })
      : (d.publisher ?? annee)
  const morceaux = estLien.value
    ? [t('gn-ligne-document.page-web'), origine]
    : [
        d.page_count === null ? null : t('gn-ligne-document.pages', { count: d.page_count }, d.page_count),
        d.reading_bytes === null ? null : tailleLisible(d.reading_bytes, locale.value),
        origine,
      ]
  return morceaux.filter((m): m is string => !!m).join(' · ')
})

const pourcentage = computed(() => {
  const p = props.progression
  if (!p || p.total === null || p.total <= 0) return null
  const part = Math.min(1, Math.max(0, p.recus / p.total))
  const texte = new Intl.NumberFormat(locale.value, { style: 'percent', maximumFractionDigits: 0 }).format(part)
  return { part, texte }
})

const texteDeProgression = computed(() =>
  pourcentage.value
    ? t('gn-ligne-document.telechargement', { part: pourcentage.value.texte })
    : t('gn-ligne-document.telechargement-en-cours'),
)
</script>

<template>
  <NuxtLink :to="vers" class="gn-ligne-document" :class="{ 'gn-ligne-document--indisponible': indisponible }">
    <GnPicto :nom="estLien ? 'external' : 'doc'" :taille="24" class="gn-ligne-document__picto" />
    <span class="gn-ligne-document__corps">
      <span class="gn-ligne-document__type">{{ type }}</span>
      <span class="gn-ligne-document__titre" :class="{ 'gn-ligne-document__titre--eteint': eteint }">
        {{ document.title }}
      </span>
      <span v-if="meta" class="gn-ligne-document__meta">{{ meta }}</span>
      <span v-if="trouve" class="gn-ligne-document__trouve">
        {{ t('gn-ligne-document.trouve', { page: trouve.page, extrait: trouve.extrait }) }}
      </span>
      <span v-if="marquesAffichees.length" class="gn-ligne-document__marques">
        <GnMarqueEtat
          v-for="m in marquesAffichees"
          :key="m.marque"
          :etat="m.etat"
          :libelle="m.libelle"
        />
      </span>
      <span v-if="progression" class="gn-ligne-document__progression">
        <span class="gn-ligne-document__progression-texte">
          <GnChargement :taille="20" decoratif />
          {{ texteDeProgression }}
        </span>
        <span v-if="pourcentage" class="gn-ligne-document__piste" aria-hidden="true">
          <span class="gn-ligne-document__part" :style="{ inlineSize: `${pourcentage.part * 100}%` }" />
        </span>
      </span>
    </span>
    <GnPicto v-if="!indisponible" nom="chevron" :taille="24" class="gn-ligne-document__chevron" />
    <span v-else class="gn-ligne-document__sans-chevron" aria-hidden="true" />
  </NuxtLink>
</template>

<style>
[data-app="guide-nego"] .gn-ligne-document {
  min-height: var(--gn-cible);
  padding-block: var(--gn-ligne-air);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-ligne-document:active {
  background: var(--gn-presse);
  /* La pression déborde les marges de l'écran, comme une ligne pleine largeur. */
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
}

[data-app="guide-nego"] .gn-ligne-document:focus-visible {
  outline-offset: calc(-1 * var(--gn-focus-decalage));
}

[data-app="guide-nego"] .gn-ligne-document__picto {
  color: var(--gn-picto-document);
}

[data-app="guide-nego"] .gn-ligne-document--indisponible .gn-ligne-document__picto {
  color: var(--gn-picto-secondaire);
}

[data-app="guide-nego"] .gn-ligne-document__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-ligne-document__type {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-ligne-document__titre {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-ligne-document__titre--eteint {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-ligne-document__meta,
[data-app="guide-nego"] .gn-ligne-document__trouve {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-ligne-document__trouve {
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  overflow: hidden;
}

[data-app="guide-nego"] .gn-ligne-document__marques {
  display: flex;
  flex-wrap: wrap;
  column-gap: var(--gn-espace-12);
  row-gap: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-ligne-document__progression {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-block-start: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-ligne-document__progression-texte {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  color: var(--gn-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-ligne-document__piste {
  display: block;
  block-size: var(--gn-jauge);
  background: var(--gn-jauge-fond);
}

[data-app="guide-nego"] .gn-ligne-document__part {
  display: block;
  block-size: 100%;
  background: var(--gn-accent);
}

[data-app="guide-nego"] .gn-ligne-document__chevron {
  color: var(--gn-picto-secondaire);
}

[data-app="guide-nego"] .gn-ligne-document__sans-chevron {
  flex: none;
  inline-size: var(--gn-picto-taille);
}
</style>
