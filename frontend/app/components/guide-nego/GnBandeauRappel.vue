<script setup lang="ts">
import type { OfficialSession } from '~/types/negotiation-sessions'

/**
 * Le rappel d'une session de l'agenda, en tête d'écran, dans l'application ouverte
 * (FR-034). Jaune : il concerne la personne à l'instant. Refermable d'un toucher.
 */
const props = withDefaults(
  defineProps<{ session: OfficialSession; fuseau: string; ville?: string | null }>(),
  { ville: null },
)
const emit = defineEmits<{ fermer: [] }>()

const { t, locale } = useI18n()
const { timeWithZone } = useDateTime()

const heure = computed(() => timeWithZone(props.session.start_at, props.fuseau, props.ville ?? undefined))
const titre = computed(() => (locale.value === 'fr' ? props.session.title_fr : null) ?? props.session.title_en)
</script>

<template>
  <section class="gn-rappel" role="status" :aria-label="t('gn-rappel.libelle')">
    <GnPicto nom="bell" :taille="20" class="gn-rappel__picto" />
    <div class="gn-rappel__corps">
      <p class="gn-rappel__quand">{{ t('gn-rappel.commence', { heure }) }}</p>
      <p class="gn-rappel__titre">{{ titre }}</p>
      <p v-if="session.venue" class="gn-rappel__salle">{{ session.venue }}</p>
      <NuxtLink
        :to="`/guide-nego/negociations/${session.id}?depuis=agenda`"
        class="gn-rappel__lien"
        @click="emit('fermer')"
      >
        {{ t('gn-rappel.voir') }}
      </NuxtLink>
    </div>
    <button type="button" class="gn-rappel__fermer" :aria-label="t('gn-rappel.fermer')" @click="emit('fermer')">
      <GnPicto nom="close" :taille="24" />
    </button>
  </section>
</template>

<style>
/* Au-dessus de l'écran, sous la barre d'état : il prend la place que l'écran lui laissait. */
[data-app="guide-nego"] .gn-rappel {
  position: sticky;
  top: 0;
  z-index: 5;
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  padding: var(--gn-sur-haut-barre-etat) var(--gn-espace-4) var(--gn-espace-8) var(--gn-marge-ecran);
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
}

[data-app="guide-nego"] .gn-rappel ~ .gn-ecran {
  padding-top: var(--gn-sur-haut-avant-contenu);
}

[data-app="guide-nego"] .gn-rappel__picto {
  flex: none;
  margin-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-rappel__corps {
  flex: 1;
  min-width: 0;
  padding-top: var(--gn-espace-12);
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-rappel__quand {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-rappel__titre {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-rappel__salle {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-rappel__lien {
  width: fit-content;
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  color: inherit;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-rappel__fermer {
  flex: none;
  display: inline-grid;
  place-items: center;
  min-width: var(--gn-cible);
  min-height: var(--gn-cible);
  border: none;
  background: none;
  color: inherit;
}
</style>
