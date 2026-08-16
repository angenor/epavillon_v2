<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import type { ThemeBadge } from '~/types/ui'
import type { SessionDisplayState } from './StatusBadge.vue'

/**
 * CARTE DE SÉANCE — le composant le plus vu de la plateforme.
 *
 * Il rend une ligne de `programme.v_public_schedule` TELLE QUELLE. La vue répond
 * à l'écran de programmation en une requête : colonnes jointes (`room_name`,
 * `organization_name`), état temporel calculé en base, couverture résolue.
 * Recomposer tout cela ici produirait une seconde implémentation, qui
 * divergerait sur les cas limites — séance en cours, reportée, annulée.
 *
 * ── LA STRUCTURE : UN RAIL TEMPOREL, PUIS LE CONTENU ────────────────────────
 *
 * La colonne de gauche ne porte que la date, l'heure et le fuseau, sur fond en
 * retrait. C'est le dessin du guide de style de référence, et il est juste :
 * dans une programmation, l'heure est ce qu'on CHERCHE. Elle mérite sa colonne,
 * alignée d'une carte à l'autre, et non une ligne de métadonnées parmi d'autres.
 * Sous 640 px, le rail devient un bandeau horizontal — une colonne de 132 px sur
 * un téléphone ne laisse plus rien au titre.
 *
 * ── L'IMAGE DE COUVERTURE ───────────────────────────────────────────────────
 *
 * En bandeau supérieur, 16:9, pleine largeur au-dessus du rail. Elle vient de
 * `v_public_schedule.cover`, que la base résout ainsi : couverture de la séance,
 * à défaut celle de la proposition d'origine. Ce repli est la règle — une
 * organisation joint son image AU DÉPÔT, et personne ne revient en téléverser
 * une seconde après l'acceptation.
 *
 * SANS IMAGE, LA CARTE RESTE ENTIÈRE. Aucun bandeau gris, aucun pictogramme de
 * remplacement, aucun dégradé : le bloc disparaît, et la carte est exactement
 * celle du guide de référence, qui n'en montre aucune. Une image inventée pour
 * « remplir » coûte de la place et n'apprend rien.
 *
 * ── SIX ÉTATS, CINQ VENUS DE LA VUE ─────────────────────────────────────────
 *
 * `temporal_state` en donne cinq. Le sixième — EN DIRECT — n'en fait pas partie
 * et ne peut pas en faire partie : il ne dépend pas du temps mais de la
 * diffusion. Il est rendu par `UiLiveBadge`, qui vérifie lui-même la règle
 * métier n° 4 — un seul direct à la fois, tous événements confondus. La carte ne
 * décide donc jamais seule d'afficher ce repère.
 *
 * ── DEUX ÉCARTS DU MODÈLE, CONTOURNÉS ICI ───────────────────────────────────
 *
 * La vue ne joint pas le PAYS de l'organisation et n'expose que les CODES des
 * thématiques — sans leurs libellés ni leurs couleurs. Les deux arrivent donc en
 * propriétés séparées, résolues par l'appelant. Écarts consignés dans
 * `docs/PROGRESSION.md` ; ils se corrigent dans la vue, pas ici.
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
  /** Version dense — vue liste, colonne de planificateur. Masque l'image. */
  compact?: boolean
  /** La séance ouvre-t-elle une liste d'attente (`sessions.waitlist_enabled`) ? */
  waitlistEnabled?: boolean
  waitlistCount?: number
  /** Masque le bandeau de couverture même quand la séance en a un. */
  hideCover?: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, timeRange, zoneOffsetShort } = useDateTime()
const { isLive } = useLiveSession()

const state = computed(() => props.session.temporal_state)
const isCancelled = computed(() => state.value === 'cancelled')
const isPast = computed(() => state.value === 'past')

/**
 * L'état AFFICHÉ : le direct l'emporte sur l'état temporel, puisqu'il dit autre
 * chose. Le registre partagé tranche — pas le statut brut de la séance.
 */
const displayState = computed<SessionDisplayState>(() =>
  isLive(props.session.id) ? 'live' : state.value,
)

/** Liseré de gauche : le seul endroit où l'état colore la carte entière. */
const STATE_EDGE: Record<SessionDisplayState, string> = {
  upcoming: 'before:bg-info-border',
  ongoing: 'before:bg-warning-solid',
  past: 'before:bg-border',
  postponed: 'before:bg-postponed',
  cancelled: 'before:bg-danger-solid',
  live: 'before:bg-live',
}

const FORMAT_ICONS: Record<PublicScheduleRow['format'], string> = {
  online: 'monitor',
  in_person: 'map-pin',
  hybrid: 'globe',
}

/** Journées spéciales et fils, déjà agrégés par la vue. */
const specialDays = computed(() => props.session.tracks.filter((track) => track.kind === 'special_day'))
const otherTracks = computed(() => props.session.tracks.filter((track) => track.kind !== 'special_day'))

const showCover = computed(() => Boolean(props.session.cover) && !props.hideCover && !props.compact)

/** Rail : la date en tête, l'heure ensuite, le fuseau en dernier. */
const dayLabel = computed(() => date(props.session.starts_at, props.session.timezone))
const hoursLabel = computed(() =>
  timeRange(props.session.starts_at, props.session.ends_at, props.session.timezone, props.zoneLabel),
)
const zoneShort = computed(
  () =>
    `${props.zoneLabel || ''}${props.zoneLabel ? ', ' : ''}${zoneOffsetShort(props.session.timezone, props.session.starts_at)}`,
)
</script>

<template>
  <article
    class="relative overflow-hidden rounded-lg border bg-surface-raised transition-colors
           before:absolute before:inset-y-0 before:left-0 before:z-10 before:w-1 before:content-['']"
    :class="[
      STATE_EDGE[displayState],
      // Le direct est le seul état qui borde la carte entière : il doit se
      // repérer dans une grille de vingt cartes sans avoir à lire.
      displayState === 'live' ? 'border-live border-(length:--border-medium)' : 'border-border',
      props.to ? 'hover:border-border-strong hover:shadow-sm focus-within:border-accent' : '',
      isPast || isCancelled ? 'bg-surface-sunken' : 'shadow-xs',
    ]"
  >
    <!-- 1. La couverture, quand il y en a une. Sinon rien : pas de bandeau gris. -->
    <UiImage
      v-if="showCover"
      :image="props.session.cover"
      ratio="16 / 9"
      sizes="(min-width: 1024px) 33vw, 100vw"
      class="border-b border-separator"
    />

    <div class="grid sm:grid-cols-[132px_1fr]">
      <!-- 2. LE RAIL TEMPOREL — la date, l'heure, le fuseau, et rien d'autre. -->
      <div
        class="flex flex-wrap items-baseline gap-x-3 gap-y-1 border-b border-separator bg-surface-sunken py-3 pr-4 pl-5
               sm:flex-col sm:items-stretch sm:border-r sm:border-b-0 sm:p-4"
      >
        <p class="font-display text-lg leading-tight font-bold text-text">{{ dayLabel }}</p>
        <p class="text-sm font-bold tabular-nums text-text" :class="isCancelled ? 'line-through' : ''">
          {{ hoursLabel }}
        </p>
        <p class="text-xs text-text-muted">{{ zoneShort }}</p>
      </div>

      <!-- 3. LE CONTENU -->
      <div class="flex min-w-0 flex-col gap-3 px-5 py-4">
        <div class="flex flex-wrap items-center gap-2">
          <UiLiveBadge :session-id="props.session.id" size="sm" />
          <UiStatusBadge
            v-if="displayState !== 'live'"
            :state="displayState"
            :label="t(`session-card.state.${state}`)"
            size="sm"
          />
        </div>

        <div class="min-w-0">
          <!-- Le titre porte le sens : hiérarchie typographique franche. -->
          <h3 class="font-display text-lg leading-snug">
            <NuxtLink
              v-if="props.to"
              :to="props.to"
              class="ui-session-link text-text no-underline hover:text-accent"
              :class="isCancelled ? 'line-through decoration-danger/60' : ''"
            >
              {{ tr(props.session.title) }}
            </NuxtLink>
            <span v-else :class="isCancelled ? 'line-through decoration-danger/60' : ''">
              {{ tr(props.session.title) }}
            </span>
          </h3>

          <p
            v-if="props.session.organization_name"
            class="mt-1 truncate text-sm text-text-secondary"
            :class="isCancelled ? 'line-through' : ''"
          >
            <b class="font-semibold">{{ props.session.organization_name }}</b>
            <span v-if="props.session.organization_acronym" class="text-text-muted">
              ({{ props.session.organization_acronym }})
            </span>
            <span v-if="props.organizationCountry" class="text-text-muted">
              · {{ props.organizationCountry }}
            </span>
          </p>
        </div>

        <!-- 4. Thématiques et journées spéciales, plafonnées à trois. -->
        <div v-if="specialDays.length || otherTracks.length || props.themes?.length" class="flex flex-wrap items-center gap-2">
          <UiBadge
            v-for="track in specialDays"
            :key="track.slug"
            intent="info"
            size="sm"
            :dot-color="track.color"
          >
            {{ tr(track.title) }}
          </UiBadge>
          <UiBadge v-for="track in otherTracks" :key="track.slug" size="sm" :dot-color="track.color">
            {{ tr(track.title) }}
          </UiBadge>
          <UiThemeTagList :themes="props.themes ?? []" :max="3" size="sm" />
        </div>

        <!-- 5. Les faits pratiques, sur une ligne. -->
        <div class="flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-text-muted">
          <span class="inline-flex items-center gap-1.5">
            <UiIcon :name="FORMAT_ICONS[props.session.format]" size="0.9rem" />
            {{ t(`session-card.format.${props.session.format}`) }}
          </span>
          <span v-if="props.session.room_name" class="inline-flex items-center gap-1.5">
            <UiIcon name="map-pin" size="0.9rem" />
            {{ tr(props.session.room_name) }}
          </span>
          <span v-if="props.session.is_streamed" class="inline-flex items-center gap-1.5">
            <UiIcon name="video" size="0.9rem" />
            {{ t('session-card.streamed') }}
          </span>
        </div>

        <!-- 6. Le pied : la jauge à gauche, les actions à droite, séparés par un filet.
             Une séance annulée, reportée ou passée n'a plus de jauge à montrer :
             le nombre d'inscrits d'une séance qui n'aura pas lieu n'informe personne. -->
        <div
          v-if="!isCancelled && state !== 'postponed' || $slots.actions"
          class="flex flex-wrap items-center justify-between gap-4 border-t border-separator pt-3"
        >
          <UiCapacityMeter
            v-if="!isCancelled && state !== 'postponed' && !isPast"
            :registered="props.session.registered_count"
            :capacity="props.session.capacity"
            :waitlist-enabled="props.waitlistEnabled"
            :waitlist-count="props.waitlistCount"
            :compact="props.compact"
            class="min-w-40 flex-1"
          />
          <div v-if="$slots.actions" class="relative z-20 ml-auto flex flex-wrap gap-2">
            <slot name="actions" />
          </div>
        </div>

        <!-- 7. Motif d'annulation ou de report — obligatoire en base pour une
             annulation (`ck_sessions_cancelled_reason`), donc jamais tu à l'écran. -->
        <p
          v-if="(isCancelled || state === 'postponed') && props.cancelledReason"
          class="rounded-md border px-3 py-2 text-sm text-text-secondary"
          :class="
            isCancelled
              ? 'border-danger-border bg-danger-surface'
              : 'border-postponed-border bg-postponed-surface'
          "
        >
          <span class="font-semibold" :class="isCancelled ? 'text-danger' : 'text-postponed'">
            {{ t(isCancelled ? 'session-card.cancelledReason' : 'session-card.postponedReason') }} :
          </span>
          {{ props.cancelledReason }}
        </p>
      </div>
    </div>
  </article>
</template>

<style scoped>
/* La zone de clic couvre la carte, un seul lien reste focalisable. Les actions
   du pied remontent au-dessus par `z-20` (voir le créneau `actions`). */
.ui-session-link::after {
  content: '';
  position: absolute;
  inset: 0;
  z-index: 1;
}
</style>
