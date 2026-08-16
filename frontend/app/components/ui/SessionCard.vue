<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { ThemeBadge } from '~/types/ui'
import type { Intent } from '~/types/ui'

/**
 * CARTE DE SÉANCE — le composant le plus vu de la plateforme.
 *
 * Il rend une ligne de `programme.v_public_schedule` TELLE QUELLE. C'est
 * délibéré : la vue répond à l'écran de programmation en une requête, ses
 * colonnes portent déjà les valeurs jointes (`room_name`, `organization_name`)
 * et l'état temporel CALCULÉ EN BASE. Recomposer tout cela dans le composant
 * produirait une seconde implémentation, qui divergerait sur les cas limites —
 * séance en cours, reportée, annulée.
 *
 * SIX ÉTATS AFFICHÉS, CINQ VENUS DE LA VUE. `temporal_state` en donne cinq :
 * `upcoming`, `ongoing`, `past`, `postponed`, `cancelled`. Le sixième — EN
 * DIRECT — n'en fait pas partie et ne peut pas en faire partie : il ne dépend
 * pas du temps mais de la diffusion (`sessions.status = 'live'`, un canal
 * réservé, une équipe technique). Il est rendu par `UiLiveBadge`, qui vérifie
 * lui-même la règle métier n° 4 — un seul direct à la fois, tous événements
 * confondus. La carte ne décide donc jamais seule d'afficher ce repère.
 *
 * PASTILLES THÉMATIQUES. La vue expose `theme_codes` : des CODES, pas des
 * libellés — le libellé et la couleur vivent dans `reference.taxonomy_terms` et
 * changent au back-office. L'appelant les résout et les passe par `themes` ;
 * recopier un libellé de thématique dans un fichier i18n est le défaut exact de
 * la v1. La couleur de la base est rendue en POINT, jamais en fond de texte :
 * elle est saisie par un administrateur et rien ne garantit son contraste.
 *
 * PAYS DE L'ORGANISATION. La vue ne le joint pas — écart relevé et consigné dans
 * `docs/PROGRESSION.md`. Il est donc passé à part, par `organizationCountry`, en
 * attendant que la vue l'expose.
 *
 * LE FUSEAU EST OBLIGATOIRE et vient de la séance (`sessions.timezone`), pas de
 * la machine du visiteur. Une COP se tient à Belém ou à Riyad ; afficher l'heure
 * locale du navigateur serait faux pour presque tout le monde.
 */

interface Props {
  /** Ligne de `programme.v_public_schedule`, consommée telle quelle. */
  session: PublicScheduleRow
  /**
   * Thématiques résolues depuis `session.theme_codes` contre
   * `reference.taxonomy_terms`. Libellé et couleur viennent de la BASE.
   */
  themes?: ThemeBadge[]
  /** Pays de l'organisation porteuse — absent de la vue, voir l'en-tête. */
  organizationCountry?: string | null
  /** Nom du lieu pour le libellé de fuseau — « Belém ». */
  zoneLabel?: string
  /** Destination de la fiche de la séance. */
  to?: string
  /** Motif d'annulation (`sessions.cancelled_reason`), affiché si la séance l'est. */
  cancelledReason?: string | null
  /** Version dense — vue liste, colonne de planificateur. */
  compact?: boolean
  /** Nombre d'inscrits en liste d'attente, quand il est connu. */
  waitlistCount?: number
  /** La séance ouvre-t-elle une liste d'attente (`sessions.waitlist_enabled`) ? */
  waitlistEnabled?: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

/**
 * État temporel → intention. `ongoing` prend l'accent plutôt qu'un vert de
 * succès : « en cours » n'est pas une réussite, c'est une position dans le temps.
 */
const STATE_INTENTS: Record<PublicScheduleRow['temporal_state'], Intent> = {
  upcoming: 'info',
  ongoing: 'success',
  past: 'neutral',
  postponed: 'warning',
  cancelled: 'danger',
}

/** Liseré de gauche : le seul endroit où l'état colore la carte entière. */
const STATE_EDGE: Record<PublicScheduleRow['temporal_state'], string> = {
  upcoming: 'before:bg-info-border',
  ongoing: 'before:bg-success-solid',
  past: 'before:bg-border',
  postponed: 'before:bg-warning-solid',
  cancelled: 'before:bg-danger-solid',
}

const FORMAT_ICONS: Record<PublicScheduleRow['format'], string> = {
  online: 'monitor',
  in_person: 'map-pin',
  hybrid: 'globe',
}

const state = computed(() => props.session.temporal_state)
const isCancelled = computed(() => state.value === 'cancelled')

/** Journées spéciales et fils thématiques, déjà agrégés par la vue. */
const specialDays = computed(() => props.session.tracks.filter((track) => track.kind === 'special_day'))
const otherTracks = computed(() => props.session.tracks.filter((track) => track.kind !== 'special_day'))
</script>

<template>
  <article
    class="relative overflow-hidden rounded-lg border border-border bg-surface-raised shadow-xs transition-colors
           before:absolute before:inset-y-0 before:left-0 before:w-1 before:content-['']"
    :class="[
      STATE_EDGE[state],
      props.to ? 'hover:border-border-strong focus-within:border-accent' : '',
      isCancelled ? 'bg-surface-sunken' : '',
    ]"
  >
    <div :class="props.compact ? 'py-3 pr-4 pl-5' : 'py-4 pr-4 pl-5'">
      <!-- 1. Le créneau, avec son fuseau. Toujours en tête : c'est ce qu'on
           cherche dans une programmation. -->
      <div class="flex flex-wrap items-center gap-x-3 gap-y-2">
        <UiZonedTime
          :start="props.session.starts_at"
          :end="props.session.ends_at"
          :timezone="props.session.timezone"
          :zone-label="props.zoneLabel"
          :format="props.compact ? 'short' : 'full'"
          icon
          class="text-sm font-medium text-text"
          :class="isCancelled ? 'line-through decoration-danger/60' : ''"
        />

        <!-- Le repère « en direct » vérifie lui-même la règle : une seule carte
             de toute la plateforme peut le porter. -->
        <UiLiveBadge :session-id="props.session.id" size="sm" />

        <UiBadge :intent="STATE_INTENTS[state]" size="sm">
          {{ t(`session-card.state.${state}`) }}
        </UiBadge>
      </div>

      <!-- 2. Le titre porte le sens — hiérarchie typographique franche. -->
      <h3 class="mt-2 text-lg leading-snug">
        <NuxtLink
          v-if="props.to"
          :to="props.to"
          class="ui-session-link no-underline text-text hover:text-accent"
        >
          {{ tr(props.session.title) }}
        </NuxtLink>
        <template v-else>{{ tr(props.session.title) }}</template>
      </h3>

      <!-- 3. L'organisation porteuse, avec son pays. -->
      <p v-if="props.session.organization_name" class="mt-1 flex items-center gap-1.5 text-sm text-text-muted">
        <UiIcon name="building" size="0.95rem" class="shrink-0 text-text-subtle" />
        <span class="min-w-0 truncate">
          {{ props.session.organization_name }}
          <span v-if="props.session.organization_acronym" class="text-text-subtle">
            ({{ props.session.organization_acronym }})
          </span>
          <span v-if="props.organizationCountry" class="text-text-subtle">
            — {{ props.organizationCountry }}
          </span>
        </span>
      </p>

      <p v-if="props.session.summary && !props.compact" class="mt-2 line-clamp-2 text-sm text-text-muted">
        {{ tr(props.session.summary) }}
      </p>

      <!-- 4. Format, salle, diffusion — les faits pratiques, sur une ligne. -->
      <div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1.5 text-sm text-text-muted">
        <span class="inline-flex items-center gap-1.5">
          <UiIcon :name="FORMAT_ICONS[props.session.format]" size="0.95rem" class="text-text-subtle" />
          {{ t(`session-card.format.${props.session.format}`) }}
        </span>

        <span v-if="props.session.room_name" class="inline-flex items-center gap-1.5">
          <UiIcon name="map-pin" size="0.95rem" class="text-text-subtle" />
          {{ tr(props.session.room_name) }}
        </span>

        <span v-if="props.session.is_streamed" class="inline-flex items-center gap-1.5">
          <UiIcon name="broadcast" size="0.95rem" class="text-text-subtle" />
          {{ t('session-card.streamed') }}
        </span>
      </div>

      <!-- 5. Journées spéciales : pastilles pleines, elles sont éditoriales et
           doivent se voir. La couleur vient de `programme_tracks.color_hex`. -->
      <div
        v-if="specialDays.length || otherTracks.length || props.themes?.length"
        class="mt-3 flex flex-wrap items-center gap-1.5"
      >
        <UiBadge
          v-for="track in specialDays"
          :key="track.slug"
          intent="info"
          size="sm"
          :dot-color="track.color"
        >
          {{ tr(track.title) }}
        </UiBadge>

        <UiBadge
          v-for="track in otherTracks"
          :key="track.slug"
          size="sm"
          :dot-color="track.color"
        >
          {{ tr(track.title) }}
        </UiBadge>

        <UiBadge
          v-for="theme in props.themes ?? []"
          :key="theme.code"
          size="sm"
          :dot-color="theme.color"
        >
          {{ typeof theme.label === 'string' ? theme.label : tr(theme.label) }}
        </UiBadge>
      </div>

      <!-- 6. La jauge. Sans capacité déclarée, seul le nombre d'inscrits sort. -->
      <div v-if="!isCancelled" class="mt-3">
        <UiCapacityMeter
          :registered="props.session.registered_count"
          :capacity="props.session.capacity"
          :waitlist-enabled="props.waitlistEnabled"
          :waitlist-count="props.waitlistCount"
          :compact="props.compact"
        />
      </div>

      <!-- 7. Motif d'annulation — obligatoire en base (`ck_sessions_cancelled_reason`),
           donc jamais tu à l'écran. -->
      <p
        v-if="isCancelled && props.cancelledReason"
        class="mt-3 rounded-md border border-danger-border bg-danger-surface px-3 py-2 text-sm text-text-muted"
      >
        <span class="font-semibold text-danger">{{ t('session-card.cancelledReason') }} : </span>
        {{ props.cancelledReason }}
      </p>

      <div v-if="$slots.actions" class="relative z-10 mt-3 flex flex-wrap gap-2">
        <slot name="actions" />
      </div>
    </div>
  </article>
</template>

<style scoped>
/* La zone de clic couvre la carte, un seul lien reste focalisable. Les actions
   du pied remontent au-dessus par `z-10` (voir le créneau `actions`). */
.ui-session-link::after {
  content: '';
  position: absolute;
  inset: 0;
}
</style>
