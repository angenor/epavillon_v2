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
 * ── LA STRUCTURE : DEUX RAILS ENCADRENT LE CONTENU ──────────────────────────
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
 * EN COLONNE DE DROITE dès que la carte est large, en bandeau supérieur sinon.
 *
 * Elle a d'abord été un bandeau à toutes les largeurs, et c'était une faute
 * d'échelle : dans la programmation, une carte occupe toute la largeur de la
 * page — un bandeau 16:9 y mesurait près de sept cents pixels de haut, pour une
 * illustration. Il repoussait sous la ligne de flottaison le titre, l'heure et
 * l'organisation, c'est-à-dire tout ce qu'on vient chercher.
 *
 * LA COLONNE FAIT 20 REM et l'image la remplit sur toute la hauteur, sans jamais
 * allonger la carte : le cadre y est en position absolue, la hauteur reste celle
 * du contenu. Le cadrage visible n'est donc pas un format fixe — il suit cette
 * hauteur. Viser un 16:9 exact demanderait 27 rem, la hauteur courante d'une
 * carte multipliée par 16/9, et cette colonne-là prend trop de place au titre.
 * La largeur se change ici, à un seul endroit.
 *
 * Elle vient de `v_public_schedule.cover`, que la base résout ainsi : couverture
 * de la séance, à défaut celle de la proposition d'origine. Ce repli est la
 * règle — une organisation joint son image AU DÉPÔT, et personne ne revient en
 * téléverser une seconde après l'acceptation.
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
 * ── DEUX ÉCARTS DU MODÈLE, RÉGLÉS DANS LA VUE (17/08) ───────────────────────
 *
 * La vue ne joignait pas le PAYS de l'organisation et n'exposait que les CODES
 * des thématiques, sans libellé ni couleur : la carte les recevait alors en
 * propriétés séparées, à charge pour chaque écran de les résoudre. Les deux
 * colonnes ont été ajoutées à `v_public_schedule` avant l'écran A3 —
 * `organization_country` et `themes` (`reference.term_badges()`). La carte les
 * lit donc sur la ligne, comme tout le reste.
 *
 * Les deux propriétés de contournement subsistent, et seulement pour cela : un
 * appelant qui doit FORCER une valeur — une démonstration, un jeu de données
 * partiel. Elles ne servent plus à réparer la vue.
 */

interface Props {
  /** Ligne de `programme.v_public_schedule`, consommée telle quelle. */
  session: PublicScheduleRow
  /**
   * Thématiques imposées par l'appelant. À défaut — le cas courant — celles que
   * porte la ligne (`v_public_schedule.themes`), libellé et couleur compris.
   */
  themes?: ThemeBadge[]
  /** Pays imposé par l'appelant ; à défaut `v_public_schedule.organization_country`. */
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

/** Thématiques : celles de la ligne, sauf si l'appelant en impose d'autres. */
const themes = computed<ThemeBadge[]>(
  () =>
    props.themes ??
    props.session.themes.map((theme) => ({
      code: theme.code,
      label: theme.label,
      color: theme.color,
    })),
)

/** Pays de l'organisation porteuse, résolu par la vue. */
const country = computed(
  () => props.organizationCountry ?? (props.session.organization_country ? tr(props.session.organization_country) : null),
)

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
  <!-- DEUX RAILS ENCADRENT LE CONTENU : le temps à gauche, l'image à droite.
       La symétrie n'est pas décorative — elle donne à chacun une largeur fixe et
       alignée d'une carte à l'autre, et laisse au titre tout ce qui reste. -->
  <article
    class="@container relative overflow-hidden rounded-lg border bg-surface-raised transition-colors
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
    <!-- LES POINTS DE RUPTURE SONT CEUX DE LA CARTE, PAS DE LA FENÊTRE. Cette
         carte se pose aussi bien pleine largeur dans la programmation que dans
         une colonne de démonstration ou de planificateur : à `lg:` près, un
         écran de 1440 px lui donnait trois colonnes même large de 591 px, et le
         titre tombait sur trois lignes dans 219 px. `@container` fait dépendre la
         disposition de la largeur reçue — la seule qui décide de ce qui tient. -->
    <div
      class="grid @lg:grid-cols-[132px_1fr]"
      :class="showCover ? '@5xl:grid-cols-[132px_1fr_20rem]' : ''"
    >
      <!-- 1. LA COUVERTURE, quand il y en a une. Sinon rien : pas de bandeau gris,
           pas de pictogramme de remplacement — la carte se referme sur deux
           colonnes et reste exactement celle du guide de référence, qui n'en
           montre aucune.

           ELLE RESTE EN TÊTE DU DOCUMENT et ne se déplace en colonne que par
           `order` : un lecteur d'écran rencontre l'illustration puis l'heure et le
           titre, dans le même ordre quelle que soit la disposition.

           LA CARTE EST UNE GRILLE À TOUTES LES LARGEURS — une seule colonne quand
           elle est étroite — et ce n'est pas un détail de mise en forme. `min-w-0`
           sur la colonne de contenu ne neutralise la largeur minimale automatique
           que d'un ÉLÉMENT DE GRILLE ; sur un bloc ordinaire il ne fait rien, et la
           plus longue ligne du contenu redevient incompressible. Repasser la carte
           en bloc la faisait déborder de l'écran d'un téléphone.

           EN COLONNE, ELLE NE PORTE AUCUNE HAUTEUR PROPRE : le cadre y passe en
           position absolue. C'est ce qui empêche la boucle — donner des
           proportions à une image qui remplit la hauteur faisait dériver la
           largeur de la hauteur, puis la hauteur de la largeur, et la carte
           gonflait à près de six cents pixels. Ici la hauteur vient du CONTENU,
           la largeur est celle de la colonne, et l'image se recadre entre les
           deux par `object-fit: cover`.

           `ratio="auto"` est délibéré : `UiImage` pose ses proportions en style
           inline, qu'aucune classe utilitaire ne peut redéfinir. Les deux cas —
           bandeau 16:9 en tête, cadre absolu en colonne — se décrivent donc sur
           le cadre, seul endroit où ils peuvent varier avec la largeur. -->
      <UiImage
        v-if="showCover"
        :image="props.session.cover"
        ratio="auto"
        sizes="(min-width: 1024px) 20rem, 100vw"
        class="border-b border-separator @lg:col-span-2 @5xl:relative @5xl:order-last @5xl:col-span-1 @5xl:h-full @5xl:border-b-0 @5xl:border-l"
        frame-class="aspect-[16/9] @5xl:absolute @5xl:inset-0 @5xl:aspect-auto @5xl:size-full"
      />

      <!-- 2. LE RAIL TEMPOREL — la date, l'heure, le fuseau, et rien d'autre. -->
      <div
        class="flex flex-wrap items-baseline gap-x-3 gap-y-1 border-b border-separator bg-surface-sunken py-3 pr-4 pl-5
               @lg:flex-col @lg:items-stretch @lg:border-r @lg:border-b-0 @lg:p-4"
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
            <span v-if="country" class="text-text-muted">
              · {{ country }}
            </span>
          </p>
        </div>

        <!-- 4. Thématiques et journées spéciales, plafonnées à trois. -->
        <div v-if="specialDays.length || otherTracks.length || themes.length" class="flex flex-wrap items-center gap-2">
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
          <UiThemeTagList :themes="themes" :max="3" size="sm" />
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
