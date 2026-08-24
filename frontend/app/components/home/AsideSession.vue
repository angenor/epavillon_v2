<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'

/**
 * UNE ACTIVITÉ DE LA FRISE DU PANNEAU « À VENIR ».
 *
 * POURQUOI PAS `UiSessionCard`. Celle-ci porte l'image de couverture, le pays,
 * la jauge de places et le motif d'annulation — tout ce qu'il faut dans une
 * programmation, et bien trop pour une colonne de 340 px qui en aligne six. La
 * carte du panneau retient le créneau, le titre, qui l'organise, où cela se
 * tient, et les thématiques : de quoi décider si l'on y va.
 *
 * LA DATE N'EST PAS ICI, elle est dans l'en-tête de journée de la frise. D'où
 * `format="short"` : les bornes et le fuseau, sans répéter le jour sur chaque
 * carte.
 *
 * TOUTE HEURE PORTE SON FUSEAU, et c'est celui de la SÉANCE : le panneau mêle
 * les éditions — une séance de Belém et un webinaire à l'heure de Montréal s'y
 * suivent. Sans le fuseau, deux cartes voisines seraient incomparables.
 *
 * UN SEUL DIRECT À LA FOIS (règle métier n° 4) : `UiLiveBadge` ne s'affiche que
 * pour la séance déclarée en direct par `useLiveSession()`. Les autres séances
 * en cours portent l'état temporel ordinaire — « En cours », en jaune, parce
 * qu'« en cours » demande attention et n'est pas une réussite.
 */

interface Props {
  session: PublicScheduleRow
  /** Destination — la programmation de l'édition concernée. */
  to?: string
  /**
   * Ville de l'édition — « Belém ». À passer dès qu'elle est connue : la ville
   * déduite de l'identifiant IANA n'est qu'un repli, et elle est sans accent
   * (« Belem »).
   */
  zoneLabel?: string
  /**
   * Sigle de l'édition, à ne passer QUE lorsque la frise en mêle plusieurs :
   * répété sous chaque carte d'une frise qui n'en montre qu'une, il n'apprend
   * rien et vole la place de l'organisation.
   */
  editionLabel?: string | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { isLive } = useLiveSession()

const title = computed(() => tr(props.session.title))

/**
 * Où cela se tient. La salle quand elle est attribuée — un dossier retenu peut
 * attendre son affectation —, à défaut le mode de participation, qui reste une
 * information utile : « En ligne » suffit à savoir qu'on peut suivre de loin.
 */
const place = computed(() => {
  const room = props.session.room_name ? tr(props.session.room_name) : ''
  return room || t(`home.aside.programme.format.${props.session.format}`)
})

const organization = computed(
  () => props.session.organization_acronym ?? props.session.organization_name ?? '',
)
</script>

<template>
  <!-- CARTE DE VERRE. Le panneau qui la porte est sombre — sur la photographie
       à partir de `lg`, sur l'aplat institutionnel en dessous : dans les deux
       cas, une carte claire y ferait un trou. La matière vient des jetons
       (`--color-glass-raised`, `--color-glass-border`), jamais d'un `bg-white/10`
       écrit ici — c'est ainsi que la v1 s'est retrouvée avec treize opacités.

       LA SÉANCE EN DIRECT PREND LE VERRE RELEVÉ ET LE TRAIT FRANC : c'est la
       seule de toute la plateforme à cet instant, elle doit se repérer sans
       lire. -->
  <article
    class="rounded-lg border p-3 shadow-glass backdrop-blur-glass transition-colors"
    :class="
      isLive(props.session.id)
        ? 'border-glass-border-strong bg-glass-hover'
        : 'border-glass-border bg-glass-raised hover:bg-glass-hover'
    "
  >
    <div class="flex flex-wrap items-center gap-2">
      <UiLiveBadge :session-id="props.session.id" size="sm" />
      <UiStatusBadge
        v-if="!isLive(props.session.id) && props.session.temporal_state === 'ongoing'"
        state="ongoing"
        size="sm"
        :label="t('home.aside.sessions.ongoing')"
      />
      <UiZonedTime
        :start="props.session.starts_at"
        :end="props.session.ends_at"
        :timezone="props.session.timezone"
        :zone-label="props.zoneLabel"
        format="short"
        class="text-xs text-text-on-inverse-muted"
      />
    </div>

    <h4 class="mt-1.5 text-sm leading-snug font-bold text-text-on-inverse">
      <NuxtLink v-if="props.to" :to="props.to" class="text-text-on-inverse no-underline hover:underline">
        {{ title }}
      </NuxtLink>
      <template v-else>{{ title }}</template>
    </h4>

    <p class="mt-1 truncate text-xs text-text-on-inverse-muted">
      <span v-if="props.editionLabel" class="font-bold text-text-on-inverse">{{ props.editionLabel }} · </span>
      <template v-if="organization">{{ organization }} · </template>{{ place }}
    </p>

    <HomeAsideThemeTags v-if="props.session.themes.length" :themes="props.session.themes" class="mt-2" />
  </article>
</template>
