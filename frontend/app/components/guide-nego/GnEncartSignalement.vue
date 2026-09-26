<script setup lang="ts">
import type { NetworkReport } from '~/types/negotiation-sessions'
import { couleurDEtat } from '~/utils/guide-nego/etats'
import { DESSIN_DU_MOTIF } from '~/utils/guide-nego/signalements'

/**
 * L'encart d'un signalement validé (08 · 1a, 1e) : il se pose par-dessus la donnée
 * officielle, qui reste entière dessous (ADR-010). **Jamais de nom** : ni l'autrice,
 * ni la personne qui a validé — seulement l'IFDD et l'heure.
 */
const props = withDefaults(
  defineProps<{
    signalement: NetworkReport
    /** Le fuseau de la COP : les heures y sont lues. */
    fuseau: string
    ville?: string | null
  }>(),
  { ville: null },
)

const { t } = useI18n()
const { time, timeWithZone } = useDateTime()
const k = (cle: string, params: Record<string, unknown> = {}) => t(`gn-encart-signalement.${cle}`, params)

const tete = computed(() => k('tete', { heure: time(props.signalement.validated_at, props.fuseau) }))

const motif = computed(() => {
  const s = props.signalement
  const dessin = DESSIN_DU_MOTIF[s.reason]
  let texte = k(`motif.${s.reason}`)
  if (s.reason === 'time' && s.proposed_start) {
    texte = k('heure', { heure: timeWithZone(s.proposed_start, props.fuseau, props.ville ?? undefined) })
  }
  if (s.reason === 'venue' && s.proposed_venue) texte = k('salle', { salle: s.proposed_venue })
  return { texte, picto: dessin.picto, couleur: dessin.teinte ? couleurDEtat(dessin.teinte) : undefined }
})
</script>

<template>
  <section class="gn-encart-signalement" :aria-label="tete">
    <p class="gn-encart-signalement__tete">
      <GnPicto nom="diamond" :taille="20" />
      <span>{{ tete }}</span>
    </p>
    <p class="gn-encart-signalement__motif" :style="motif.couleur ? { color: motif.couleur } : undefined">
      <GnPicto :nom="motif.picto" :taille="20" />
      <span>{{ motif.texte }}</span>
    </p>
    <p v-if="signalement.detail" class="gn-encart-signalement__precision">{{ signalement.detail }}</p>
  </section>
</template>

<style>
[data-app="guide-nego"] .gn-encart-signalement {
  margin-top: var(--gn-espace-16);
  padding: var(--gn-espace-12) var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  gap: 6px;
  border: var(--gn-filet-2) solid var(--gn-reseau);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-encart-signalement__tete,
[data-app="guide-nego"] .gn-encart-signalement__motif {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-encart-signalement__tete {
  color: var(--gn-reseau);
}

[data-app="guide-nego"] .gn-encart-signalement__motif {
  color: var(--gn-picto);
}

[data-app="guide-nego"] .gn-encart-signalement .gn-picto {
  flex: none;
  margin-block-start: calc((1em * var(--gn-interligne-15) - var(--gn-picto-marque)) / 2);
}

[data-app="guide-nego"] .gn-encart-signalement__precision {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}
</style>
