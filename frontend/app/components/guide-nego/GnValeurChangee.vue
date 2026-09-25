<script setup lang="ts">
/**
 * La valeur d'une ligne clé-valeur (08 · 4 octies) : l'ancienne barrée au-dessus, la
 * flèche cyan devant la nouvelle ; sans changement, la valeur seule. La précision
 * — jour et fuseau, bâtiment — se lit dessous.
 *
 * Le trait barré ne s'entend pas : « avant » et « maintenant » sont dits à qui écoute.
 */
const props = withDefaults(
  defineProps<{
    valeur: string
    avant?: string | null
    precision?: string | null
    /** La salle : 20/700, la donnée qu'on cherche en courant. */
    forte?: boolean
    /** Une session annulée : tout passe en gris. */
    eteinte?: boolean
    /** En cours : l'aplat jaune du temps présent. */
    maintenant?: boolean
  }>(),
  { avant: null, precision: null, forte: false, eteinte: false, maintenant: false },
)

const { t } = useI18n()

const changee = computed(() => !!props.avant && props.avant !== props.valeur)
</script>

<template>
  <span
    class="gn-valeur-changee"
    :class="{ 'gn-valeur-changee--forte': forte, 'gn-valeur-changee--eteinte': eteinte }"
  >
    <s v-if="changee" class="gn-valeur-changee__avant">
      <span class="gn-hors-ecran">{{ t('gn-valeur-changee.avant') }}</span>{{ avant }}
    </s>
    <span class="gn-valeur-changee__valeur">
      <GnPicto v-if="changee" nom="moved" :taille="20" class="gn-valeur-changee__fleche" />
      <span v-if="changee" class="gn-hors-ecran">{{ t('gn-valeur-changee.maintenant') }}</span>
      <span class="gn-valeur-changee__texte" :class="{ 'gn-valeur-changee__texte--maintenant': maintenant && !eteinte }">
        {{ valeur }}
      </span>
    </span>
    <span v-if="precision" class="gn-valeur-changee__precision">{{ precision }}</span>
  </span>
</template>

<style>
[data-app="guide-nego"] .gn-valeur-changee {
  min-width: 0;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  text-align: end;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-valeur-changee__avant {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  text-decoration-thickness: 2px;
}

[data-app="guide-nego"] .gn-valeur-changee__valeur {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-texte);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
  font-variant-numeric: tabular-nums;
}

[data-app="guide-nego"] .gn-valeur-changee__fleche {
  color: var(--gn-etat-deplacee);
}

[data-app="guide-nego"] .gn-valeur-changee--forte .gn-valeur-changee__valeur {
  color: var(--gn-accent);
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-valeur-changee--eteinte .gn-valeur-changee__valeur {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-valeur-changee__texte--maintenant {
  padding: 2px 6px;
  background: var(--gn-attention-aplat);
  color: var(--gn-attention-aplat-texte);
}

[data-app="guide-nego"] .gn-valeur-changee__precision {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-valeur-changee__precision::first-letter {
  text-transform: uppercase;
}
</style>
