<script setup lang="ts">
/**
 * Jauge d'inscription — combien de places prises sur combien.
 *
 * TROIS SITUATIONS, TROIS RENDUS. Places disponibles, salle presque pleine,
 * salle complète avec liste d'attente. La dernière n'est pas un échec : une
 * liste d'attente qui fonctionne remplit les places libérées, et
 * `promote_from_waitlist()` existe pour cela dans le modèle.
 *
 * SANS JAUGE DÉCLARÉE (`capacity` nul), on affiche le nombre d'inscrits et rien
 * d'autre : une barre de progression sans total est un mensonge graphique.
 *
 * LE DÉPASSEMENT EST POSSIBLE et doit se voir : `registered_count` compte les
 * inscrits ET les présents, et la jauge d'une salle peut être revue à la baisse
 * après coup. La barre est alors bornée à 100 % mais le chiffre reste exact —
 * l'inverse (barre qui déborde, chiffre corrigé) cacherait le problème.
 *
 * `role="meter"` avec ses bornes : c'est ce qui permet à un lecteur d'écran
 * d'annoncer « 68 sur 80 » plutôt que de décrire une barre colorée.
 */

interface Props {
  /** Inscrits — `v_public_schedule.registered_count` : inscrits et présents,
   *  annulations exclues. */
  registered: number
  /** Jauge de la salle. `null` = sans limite déclarée. */
  capacity?: number | null
  /** Liste d'attente ouverte sur cette séance (`sessions.waitlist_enabled`). */
  waitlistEnabled?: boolean
  /** Personnes en liste d'attente. */
  waitlistCount?: number
  /** Version d'une ligne, sans barre — cartes denses, cellules de tableau. */
  compact?: boolean
}

const props = defineProps<Props>()
const { t } = useI18n()

const ratio = computed(() => {
  if (!props.capacity || props.capacity <= 0) return null
  return props.registered / props.capacity
})

const percent = computed(() => (ratio.value === null ? 0 : Math.min(100, Math.round(ratio.value * 100))))

/**
 * Seuils : au-delà de 100 % la salle est pleine, au-delà de 85 % elle se remplit.
 * La couleur DISTINGUE ici un état réel de la séance — elle ne décore pas.
 */
const level = computed<'ok' | 'tight' | 'full'>(() => {
  if (ratio.value === null) return 'ok'
  if (ratio.value >= 1) return 'full'
  if (ratio.value >= 0.85) return 'tight'
  return 'ok'
})

const BAR: Record<'ok' | 'tight' | 'full', string> = {
  ok: 'bg-success-solid',
  tight: 'bg-warning-solid',
  full: 'bg-danger-solid',
}

const remaining = computed(() =>
  props.capacity === null || props.capacity === undefined
    ? null
    : Math.max(0, props.capacity - props.registered),
)

const label = computed(() => {
  if (!props.capacity) return t('session-card.capacity.registered', props.registered)
  return t('session-card.capacity.ratio', {
    registered: props.registered,
    capacity: props.capacity,
  })
})
</script>

<template>
  <div class="min-w-0">
    <div class="flex flex-wrap items-baseline gap-x-2 gap-y-0.5 text-sm">
      <UiIcon name="users" size="0.95rem" class="self-center text-text-subtle" />
      <span class="font-mono tabular-nums text-text">{{ label }}</span>

      <span v-if="level === 'full'" class="text-xs font-semibold text-danger">
        {{ t('session-card.capacity.full') }}
      </span>
      <span v-else-if="remaining !== null && level === 'tight'" class="text-xs text-warning">
        {{ t('session-card.capacity.remaining', remaining) }}
      </span>
    </div>

    <div
      v-if="ratio !== null && !props.compact"
      class="mt-1.5 h-1.5 w-full overflow-hidden rounded-full bg-surface-sunken"
      role="meter"
      :aria-valuenow="props.registered"
      :aria-valuemin="0"
      :aria-valuemax="props.capacity ?? undefined"
      :aria-valuetext="label"
      :aria-label="t('session-card.capacity.label')"
    >
      <!-- Bornée à 100 % : le chiffre au-dessus reste exact si la jauge est
           dépassée, la barre ne déborde pas de son cadre. -->
      <div class="h-full rounded-full transition-[width] duration-300" :class="BAR[level]" :style="{ width: `${percent}%` }" />
    </div>

    <p v-if="props.waitlistEnabled && level === 'full'" class="mt-1 text-xs text-text-muted">
      {{
        props.waitlistCount
          ? t('session-card.capacity.waitlistCount', props.waitlistCount)
          : t('session-card.capacity.waitlist')
      }}
    </p>
  </div>
</template>
