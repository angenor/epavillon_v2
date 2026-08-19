<script setup lang="ts">
import type { PublicEditionRow } from '~/types/views'

/**
 * UNE ÉDITION DANS LE RAIL DE L'ACCUEIL — une AFFICHE, plus une fiche.
 *
 * ── CE QUI A CHANGÉ, ET POURQUOI ────────────────────────────────────────────
 *
 * La carte était une vignette 16/9 surmontant six lignes de texte sur fond
 * clair : lisible, mais elle ne disait rien de l'énergie d'une programmation.
 * Elle est désormais pleine hauteur, le média occupe TOUTE la carte et le texte
 * se pose dessus. C'est la posture « billetterie de festival » de la direction
 * artistique, celle qui manquait à cette section.
 *
 * ── LE DÉGRADÉ EST ICI DANS SON SEUL RÔLE AUTORISÉ ──────────────────────────
 *
 * `.scrim-fade-bottom` est l'unique dégradé de la charte, et il ne décore rien :
 * il rend lisible un texte clair sur une image dont on ignore la luminosité. Une
 * bannière d'édition est exactement ce cas — elle est téléversée par un
 * administrateur, personne ne sait si son bas est sombre ou surexposé.
 *
 * Il ne se pose donc QUE sur une image. Sur le repli, il n'y a rien à voiler :
 * l'aplat institutionnel est déjà sombre, et un dégradé posé dessus serait de
 * l'ornement — ce que la charte refuse.
 *
 * PAS DE VERRE NON PLUS. Le média est là, le verre serait autorisé ; il n'est
 * pas utile. Le voile suffit à porter le contraste, et un panneau flou sur une
 * carte de 320 px ajouterait un calcul de flou par carte pour un gain nul.
 *
 * ── LA BANNIÈRE EST SOUVENT NULLE, ET LA CARTE RESTE ENTIÈRE ────────────────
 *
 * `v_public_editions.banner` vient du rôle `banner` de `event.events` ; la
 * plupart des éditions passées n'en ont pas. Le repli n'invente AUCUNE image :
 * un aplat institutionnel portant le millésime en filigrane, ce qui reste un
 * repère là où un pictogramme générique n'en serait pas un.
 *
 * ── PAS DE PASTILLES THÉMATIQUES SUR L'AFFICHE ──────────────────────────────
 *
 * Retirées, et c'est un choix. Une affiche dit QUAND, OÙ et COMBIEN ; la
 * taxonomie est un outil de recherche, et sa place est sur la page de l'édition
 * où l'on cherche vraiment. Sur un média voilé, trois pastilles claires de plus
 * — après la pastille d'état — auraient surtout rendu le titre moins lisible.
 *
 * ── « EN COURS » EST JAUNE, PAS VERT ────────────────────────────────────────
 *
 * Le jaune signale ce qui demande attention ; le vert, ce qui est confirmé. Une
 * édition en cours n'est pas une réussite, c'est un événement qui se tient en ce
 * moment. `UiStatusBadge` porte cette table de couleurs — la carte lui passe
 * l'état temporel et le libellé, elle ne choisit aucune teinte. Son fond est
 * OPAQUE, ce qui est la raison pour laquelle elle peut se poser en haut de la
 * carte, hors du voile.
 *
 * ── LE NOMBRE D'ACTIVITÉS : ABSENT VAUT ZÉRO ────────────────────────────────
 *
 * Une édition sans programme publié n'a AUCUNE ligne dans
 * `programme.v_edition_stats`. Le compte arrive déjà résolu par
 * `publishedSessionCount()`, et la carte affiche « 0 activité publiée » plutôt
 * qu'un tiret : zéro est une information, un tiret est un aveu.
 */

interface Props {
  edition: PublicEditionRow
  /** Séances publiées — déjà résolu, l'absence de ligne valant zéro. */
  sessionCount: number
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateRange, zoneLabel } = useDateTime()
const localePath = useLocalePath()

const to = computed(() => localePath(`/evenements/${props.edition.slug}`))

const dates = computed(() =>
  dateRange(props.edition.starts_at, props.edition.ends_at, props.edition.timezone),
)

const zone = computed(() => zoneLabel(props.edition.timezone, props.edition.city ?? undefined))

const place = computed(() =>
  [props.edition.city, tr(props.edition.country_name)].filter(Boolean).join(', '),
)

/** Le repère du repli : le millésime annoncé, jamais l'année de `starts_at`. */
const stamp = computed(
  () => props.edition.edition_label ?? String(props.edition.edition_year),
)
</script>

<template>
  <!-- `rounded-xl` — 12 px, l'usage exceptionnel que ce jeton attendait. La
       charte plafonne à 8 px sur les blocs d'interface ; celui-ci n'en est pas
       un, c'est une affiche de 320 px de large sur toute la hauteur de l'écran,
       où 8 px se lisent comme un angle droit. Le rayon suit la taille du bloc,
       et aucune autre surface de la plateforme n'a cette taille-là. -->
  <article
    class="group relative flex flex-col overflow-hidden rounded-xl border border-border bg-surface-inverse text-text-on-inverse shadow-sm transition duration-200 hover:shadow-lg motion-safe:hover:-translate-y-1"
  >
    <UiImage
      v-if="props.edition.banner"
      :image="props.edition.banner"
      ratio="auto"
      frame-class="size-full"
      class="absolute inset-0"
      sizes="(min-width: 1280px) 24rem, (min-width: 1024px) 22rem, 80vw"
    />

    <!-- REPLI : aucun visuel inventé. Le millésime en filigrane repère la carte
         sans prétendre montrer l'événement. -->
    <!-- Placé HAUT, pas au centre : au centre d'une affiche de 700 px, le
         filigrane vient buter contre le surtitre et se lit alors comme un texte
         mal superposé plutôt que comme une matière de fond. -->
    <div
      v-else
      class="absolute inset-x-0 top-0 flex justify-center px-6 pt-16"
      aria-hidden="true"
    >
      <span class="font-display text-5xl tabular-nums text-text-on-inverse/10">
        {{ stamp }}
      </span>
    </div>

    <!-- LE VOILE, SEULEMENT SOUS UNE IMAGE, ET EN DEUX TEMPS.
         · L'APLAT couvre toute l'affiche. Il ne sert pas le titre — le fondu s'en
           charge — mais la pastille d'état et le SURTITRE, qui se posent sur une
           zone dont on ne sait rien : une photographie surexposée y rendait
           « COP CLIMAT (CCNUCC) » illisible en gris clair sur blanc.
         · LE FONDU porte le contraste du bloc de texte, sur les deux tiers bas.
         Deux voiles superposés restent UN voile : le second ne fait qu'épaissir
         le premier là où le texte est dense. -->
    <div
      v-if="props.edition.banner"
      class="pointer-events-none absolute inset-0 bg-scrim/30"
      aria-hidden="true"
    />
    <div
      v-if="props.edition.banner"
      class="scrim-fade-bottom pointer-events-none absolute inset-x-0 bottom-0 h-2/3"
      aria-hidden="true"
    />

    <p class="relative flex items-start p-4">
      <UiStatusBadge
        :state="props.edition.temporal_state"
        size="sm"
        :label="t(`home.history.state.${props.edition.temporal_state}`)"
      />
    </p>

    <div class="relative mt-auto flex flex-col gap-2 p-4 sm:p-5">
      <p
        v-if="props.edition.series_name"
        class="text-xs uppercase text-text-on-inverse"
        :style="{ letterSpacing: 'var(--tracking-caps)' }"
      >
        {{ tr(props.edition.series_name) }}
      </p>

      <!-- LIEN COUVRANT : `after:inset-0` étend la zone cliquable à la carte
           entière, sans dupliquer le titre dans un second lien invisible que
           les lecteurs d'écran énonceraient deux fois. L'anneau de focus, lui,
           reste dessiné autour du texte — visible, et non rogné par
           `overflow-hidden`. -->
      <h3 class="font-display text-xl leading-tight text-text-on-inverse sm:text-2xl">
        <NuxtLink
          :to="to"
          class="text-text-on-inverse no-underline after:absolute after:inset-0 hover:underline"
        >
          {{ tr(props.edition.title) }}
        </NuxtLink>
      </h3>

      <p
        v-if="props.edition.edition_label"
        class="text-sm font-medium text-text-on-inverse-muted"
      >
        {{ props.edition.edition_label }}
      </p>

      <p class="flex items-start gap-2 text-sm text-text-on-inverse-muted">
        <UiIcon name="calendar" size="1rem" class="mt-0.5 shrink-0" />
        <span>
          {{ dates }}
          <span class="block text-xs">{{ zone }}</span>
        </span>
      </p>

      <p v-if="place" class="flex items-center gap-2 text-sm text-text-on-inverse-muted">
        <UiIcon name="map-pin" size="1rem" class="shrink-0" />
        {{ place }}
      </p>

      <p class="flex items-center gap-2 text-sm text-text-on-inverse-muted">
        <UiIcon name="grid" size="1rem" class="shrink-0" />
        {{ t('home.history.sessions', { count: props.sessionCount }, props.sessionCount) }}
      </p>

      <!-- L'affordance, pas un second lien : la carte entière conduit déjà à
           l'édition, et deux liens vers la même adresse encombrent la
           tabulation autant que la lecture à voix haute. -->
      <p
        class="mt-1 inline-flex items-center gap-1 text-sm font-medium text-text-on-inverse"
        aria-hidden="true"
      >
        {{ t('home.history.card.open') }}
        <UiIcon
          name="arrow-right"
          size="0.9rem"
          class="transition-transform duration-200 motion-safe:group-hover:translate-x-1"
        />
      </p>
    </div>
  </article>
</template>
