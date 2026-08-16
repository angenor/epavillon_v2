<script setup lang="ts">
import type { IsoDateTime, TimeZoneName } from '~/types/shared'

/**
 * UNE DATE AVEC SON FUSEAU — motif transverse de toute la plateforme.
 *
 * « 14:30 — 16:00 (heure de Belém, UTC−3) »
 *
 * POURQUOI C'EST UN COMPOSANT ET PAS UN APPEL DE FONCTION. La règle du projet
 * — « toute date affichée porte son fuseau » — se perd dès qu'elle repose sur la
 * discipline : il suffit d'un `{{ session.starts_at }}` pour la trahir, et rien
 * ne le signale. Un composant rend l'oubli visible à la relecture, et impose le
 * fuseau comme propriété OBLIGATOIRE : on ne peut pas l'appeler sans y penser.
 *
 * POURQUOI LE DÉCALAGE EN PLUS DU NOM DU LIEU. Une COP réunit des gens de
 * quarante pays. « heure de Belém » ne dit rien à qui ne connaît pas Belém ;
 * « UTC−3 » se calcule de partout. Les deux ensemble servent les deux publics.
 *
 * MODÈLE : les instants sont stockés en `timestamptz` et le fuseau de référence
 * vit à côté (`event.events.timezone`, `programme.sessions.timezone`). La
 * conversion se fait À L'AFFICHAGE, jamais à l'écriture — c'est écrit dans
 * `060_events.sql`, et c'est ce que fait ce composant.
 *
 * `<time datetime>` : la valeur lisible par machine est l'instant ISO complet,
 * avec son décalage. C'est ce qui permet de l'exporter vers un agenda.
 */

interface Props {
  /** Instant de début, ISO 8601 avec décalage. */
  start: IsoDateTime
  /** Instant de fin. Absent, seul le début est affiché. */
  end?: IsoDateTime | null
  /** Fuseau IANA de référence. OBLIGATOIRE, par principe. */
  timezone: TimeZoneName
  /**
   * Nom du lieu — « Belém ». À passer dès qu'il est connu : la ville déduite de
   * l'identifiant IANA n'est qu'un repli, et elle est sans accent (« Belem »).
   */
  zoneLabel?: string
  /**
   * Forme d'affichage :
   * · `full`  — « 14:30 — 16:00 (heure de Belém, UTC−3) », la forme de référence
   * · `short` — « 14:30 — 16:00, heure de Belém », quand la page rappelle déjà le fuseau
   * · `withDate` — la date complète précède le créneau
   */
  format?: 'full' | 'short' | 'withDate'
  /** Icône d'horloge en tête. */
  icon?: boolean
  /**
   * Composition à deux niveaux : les bornes en police de titrage, le fuseau en
   * texte secondaire plus petit. C'est le dessin du guide de style — il donne au
   * créneau le poids qu'il mérite dans une programmation, où l'heure est ce
   * qu'on cherche. Le rendu par défaut reste une seule ligne de texte courant,
   * pour les contextes où le créneau n'est qu'une métadonnée parmi d'autres.
   */
  stacked?: boolean
}

const props = withDefaults(defineProps<Props>(), { format: 'full' })

const { timeRange, timeRangeFull, date, zoneOffsetShort } = useDateTime()

const range = computed(() =>
  props.format === 'short'
    ? timeRange(props.start, props.end, props.timezone, props.zoneLabel)
    : timeRangeFull(props.start, props.end, props.timezone, props.zoneLabel),
)

const dayLabel = computed(() =>
  props.format === 'withDate' ? date(props.start, props.timezone) : '',
)

/** Composition à deux niveaux : les bornes seules d'un côté, le fuseau de l'autre. */
const bounds = computed(() => timeRange(props.start, props.end, props.timezone, props.zoneLabel).split(',')[0] ?? '')
const zoneSuffix = computed(
  () => `${props.zoneLabel || timeZoneCityLabel(props.timezone)}, ${zoneOffsetShort(props.timezone, props.start)}`,
)
</script>

<template>
  <!-- Composition à deux niveaux : les bornes portent le poids, le fuseau suit. -->
  <span v-if="props.stacked" class="inline-flex items-baseline gap-2 tabular-nums">
    <UiIcon v-if="props.icon" name="clock" size="0.95em" class="self-center text-text-subtle" />
    <time :datetime="props.start" class="font-display text-lg font-bold text-text">{{ bounds }}</time>
    <span class="text-xs text-text-muted">{{ zoneSuffix }}</span>
  </span>

  <span v-else class="inline-flex items-baseline gap-1.5">
    <UiIcon v-if="props.icon" name="clock" size="0.95em" class="self-center text-text-subtle" />
    <span>
      <span v-if="dayLabel" class="font-medium">{{ dayLabel }} · </span>
      <!-- L'attribut porte l'instant brut : c'est la valeur qu'une machine lit. -->
      <time :datetime="props.start" class="tabular-nums">{{ range }}</time>
    </span>
  </span>
</template>
