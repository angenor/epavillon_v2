<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'
import type { EventSeries } from '~/types/event/series'
import type { AttachedImage, EditionImageRole } from '~/types/media'

/**
 * LE BANDEAU DE LA PAGE D'UNE ÉDITION (direction condensée, 01/11) : retour
 * aux éditions, nature et état, grand titre, puis les quatre faits.
 *
 * Sur la photo de l'édition, voilée (`bg-scrim/50` et `.scrim-fade-bottom`) :
 * le contraste vient du voile, le verre des faits ne fait que séparer. Sans
 * photo, l'aplat bleu nuit et des faits sans verre (le verre n'existe que sur
 * un média).
 *
 * Le 32:9 d'abord, le 16:9 à défaut, jamais le carré : étiré sur toute la
 * largeur, il ne montrerait plus rien. La hauteur suit le contenu.
 *
 * `hasAside` : le billet de l'appel remonte de 72 px sur le bas du bandeau à
 * partir de `lg`, d'où la marge basse plus grande. Dessous, rien ne chevauche.
 */

interface Props {
  edition: EventEdition
  /** Série de rattachement : « CdP climat (CCNUCC) ». */
  series?: EventSeries | null
  /** Les trois déclinaisons, telles que `v_public_editions` les rend. */
  images?: Partial<Record<EditionImageRole, AttachedImage | null>> | null
  /** Nom du pays hôte, déjà résolu. */
  country?: string | null
  hasAside?: boolean
}

const props = withDefaults(defineProps<Props>(), { series: null, images: null, country: null, hasAside: false })

const { t } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()

const poster = computed(() => props.images?.banner ?? props.images?.cover ?? null)

/** Couleurs des états : cyan informe, jaune « en cours », gris clos, rouge échec. */
const STATUS_INTENT: Record<EventEdition['status'], 'info' | 'warning' | 'neutral' | 'danger'> = {
  draft: 'neutral',
  announced: 'info',
  ongoing: 'warning',
  completed: 'neutral',
  cancelled: 'danger',
  suspended: 'danger',
}
</script>

<template>
  <header
    class="relative isolate overflow-hidden bg-surface-inverse text-text-on-inverse [--color-focus:var(--color-accent-on-inverse)]"
  >
    <template v-if="poster">
      <!-- Chargement immédiat : l'image est au-dessus de la ligne de flottaison. -->
      <UiImage
        :image="poster"
        ratio="auto"
        frame-class="size-full"
        class="absolute inset-0 -z-10"
        loading="eager"
        sizes="100vw"
        aria-hidden="true"
      />
      <div class="absolute inset-0 -z-10 bg-scrim/50" aria-hidden="true" />
      <div class="scrim-fade-bottom absolute inset-x-0 bottom-0 -z-10 h-4/5" aria-hidden="true" />
    </template>

    <div
      class="mx-auto max-w-[1280px] px-4 pt-6 pb-9 sm:px-6 sm:pb-12"
      :class="props.hasAside ? 'lg:pb-30' : 'lg:pb-16'"
    >
      <nav :aria-label="t('nav.breadcrumb.label')">
        <NuxtLink
          :to="`${localePath('/')}#editions`"
          class="inline-flex min-h-11 items-center gap-2 text-[0.9375rem] font-semibold text-text-on-inverse-muted no-underline hover:text-text-on-inverse hover:underline"
        >
          <UiIcon name="arrow-left" size="1rem" :stroke-width="2" />
          {{ t('event.public.hero.back') }}
        </NuxtLink>
      </nav>

      <p class="mt-4 flex flex-wrap items-center gap-2.5">
        <span
          v-if="props.series"
          class="font-condensed text-[1.05rem] leading-tight font-semibold tracking-[0.06em] text-text-on-inverse-muted uppercase"
        >
          {{ tr(props.series.name) }}
        </span>
        <!-- Fond opaque : lisible sur n'importe quelle photo, même code partout. -->
        <UiBadge :intent="STATUS_INTENT[props.edition.status]" size="sm">
          {{ t(`event.public.hero.status.${props.edition.status}`) }}
        </UiBadge>
      </p>

      <h1
        class="heading-condensed mt-4 max-w-[34ch] text-[clamp(2.2rem,4.4vw,3.6rem)] text-balance text-text-on-inverse"
      >
        {{ tr(props.edition.title) }}
      </h1>

      <EventHeroFacts
        class="mt-7 max-w-[860px]"
        :edition="props.edition"
        :country="props.country"
        :tone="poster ? 'glass' : 'inverse'"
      />
    </div>
  </header>
</template>
