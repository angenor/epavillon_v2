<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'

/**
 * UNE SÉANCE DU PANNEAU « À VENIR ».
 *
 * POURQUOI PAS `UiSessionCard`. Celle-ci porte l'image de couverture, le pays,
 * les thématiques, la jauge de places et le motif d'annulation — tout ce qu'il
 * faut dans une programmation. Dans une colonne de 340 px qui en aligne six,
 * elle noie l'unique chose qu'on vient y chercher : QUAND, et QUOI. La carte du
 * panneau ne retient donc que le créneau, le titre et l'organisation.
 *
 * TOUTE HEURE PORTE SON FUSEAU, et c'est celui de la SÉANCE : le panneau mêle
 * les éditions — une séance de Belém et un webinaire à l'heure de Dakar s'y
 * suivent. Sans le fuseau, deux lignes voisines seraient incomparables.
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
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { isLive } = useLiveSession()

const title = computed(() => tr(props.session.title))
</script>

<template>
  <!-- CARTE DE VERRE. Le panneau qui la porte est sombre — sur la photographie
       à partir de `lg`, sur l'aplat institutionnel en dessous : dans les deux
       cas, une carte claire y ferait un trou. La matière vient des jetons
       (`--color-glass-raised`, `--color-glass-border`), jamais d'un `bg-white/10`
       écrit ici — c'est ainsi que la v1 s'est retrouvée avec treize opacités. -->
  <article class="rounded-lg border border-glass-border bg-glass-raised p-3 shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover">
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
        format="withDate"
        class="text-xs text-text-on-inverse-muted"
      />
    </div>

    <h4 class="mt-1.5 text-sm leading-snug font-bold text-text-on-inverse">
      <NuxtLink v-if="props.to" :to="props.to" class="text-text-on-inverse no-underline hover:underline">
        {{ title }}
      </NuxtLink>
      <template v-else>{{ title }}</template>
    </h4>

    <p v-if="props.session.organization_name" class="mt-1 truncate text-xs text-text-on-inverse-muted">
      {{ props.session.organization_acronym ?? props.session.organization_name }}
    </p>
  </article>
</template>
