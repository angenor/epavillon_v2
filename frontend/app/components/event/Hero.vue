<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'
import type { EventSeries } from '~/types/event/series'
import type { AttachedImage } from '~/types/media'

/**
 * EN-TÊTE DE LA PAGE PUBLIQUE D'UNE ÉDITION — la première des quatre questions
 * auxquelles cette page répond : DE QUOI S'AGIT-IL ?
 *
 * Titre, dates, lieu, mode de participation, visuel. Rien d'autre : les
 * échéances viennent juste après (l'encart d'appel), et le programme plus bas.
 * Un en-tête qui répond à tout ne répond à rien.
 *
 * LE VISUEL N'EST PAS UNE BANNIÈRE DE FOND, ET IL RESTE À CÔTÉ DU TEXTE.
 * Le back-office, lui, a reçu le 19/08 un bandeau plein cadre : c'est un écran
 * de travail, on y reconnaît une édition d'un coup d'œil parmi vingt. Ici on
 * la LIT — un titre posé sur une photographie perd son contraste dès qu'elle
 * change, et elle change à chaque édition. Sans visuel — c'est le cas de la
 * COP29 dans les données simulées — l'en-tête reste entier.
 *
 * TOUTE DATE PORTE SON FUSEAU. Celui de l'édition (`event.events.timezone`),
 * jamais celui du visiteur : « du 9 au 20 novembre 2027, heure de Belém ».
 */

interface Props {
  edition: EventEdition
  /** Série de rattachement, quand l'édition en a une : « COP Climat (CCNUCC) ». */
  series?: EventSeries | null
  /**
   * LA DÉCLINAISON 16:9 (rôle `cover`), et pas le bandeau 32:9.
   *
   * Depuis le 19/08, une édition porte trois recadrages téléversés à la main.
   * Celui-ci est posé À CÔTÉ du texte, dans une colonne de 40 % de la page :
   * un 32:9 y ferait une meurtrière de 120 px de haut. L'appelant se rabat sur
   * `banner` quand le 16:9 manque — recadré par `object-cover`, ce qui reste
   * préférable à un trou dans la mise en page.
   */
  cover?: AttachedImage | null
  /** Nom du pays hôte, déjà résolu depuis `reference.countries`. */
  country?: string | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateRange, zoneLabel } = useDateTime()

/** « du 9 au 20 novembre 2027 » dans le fuseau de l'édition. */
const dates = computed(() =>
  dateRange(props.edition.starts_at, props.edition.ends_at, props.edition.timezone),
)

/** « heure de Belém » — le lieu prime sur la ville déduite de l'identifiant IANA. */
const zone = computed(() => zoneLabel(props.edition.timezone, props.edition.city ?? undefined))

/** « Belém, Brésil » ; rien du tout pour une édition entièrement en ligne. */
const place = computed(() => {
  const parts = [props.edition.city, props.country].filter((part): part is string => Boolean(part))
  return parts.join(', ')
})

const FORMAT_ICONS: Record<EventEdition['participation_mode'], string> = {
  online: 'monitor',
  in_person: 'map-pin',
  hybrid: 'globe',
}

/**
 * L'état de l'édition, dit avec les couleurs des états et non celles des
 * humeurs : le cyan informe, le jaune demande attention (« en cours »), le gris
 * clôt. Une édition annulée est un échec de calendrier, donc rouge.
 */
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
  <header class="grid gap-8 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)] lg:items-start">
    <div class="min-w-0">
      <p class="flex flex-wrap items-center gap-2 text-sm text-text-subtle">
        <span v-if="props.series" class="uppercase" :style="{ letterSpacing: 'var(--tracking-caps)' }">
          {{ tr(props.series.name) }}
        </span>
        <UiBadge :intent="STATUS_INTENT[props.edition.status]" size="sm">
          {{ t(`event.public.hero.status.${props.edition.status}`) }}
        </UiBadge>
      </p>

      <h1 class="mt-3 font-display text-3xl leading-tight sm:text-4xl">
        {{ tr(props.edition.title) }}
      </h1>

      <!-- Les faits, sur une liste de définitions : ce sont des données, pas une
           accroche. Chacune porte son icône, mais le sens est dans le texte. -->
      <dl class="mt-6 grid gap-4 sm:grid-cols-2">
        <div>
          <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
            {{ t('event.public.hero.dates') }}
          </dt>
          <dd class="mt-1 flex items-start gap-2 text-text">
            <UiIcon name="calendar" size="1.05rem" class="mt-0.5 shrink-0 text-text-muted" />
            <span>
              {{ dates }}
              <span class="block text-sm text-text-muted">{{ zone }}</span>
            </span>
          </dd>
        </div>

        <div v-if="place">
          <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
            {{ t('event.public.hero.place') }}
          </dt>
          <dd class="mt-1 flex items-start gap-2 text-text">
            <UiIcon name="map-pin" size="1.05rem" class="mt-0.5 shrink-0 text-text-muted" />
            <span>
              {{ place }}
              <span v-if="props.edition.address" class="block text-sm text-text-muted">
                {{ props.edition.address }}
              </span>
            </span>
          </dd>
        </div>

        <div>
          <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
            {{ t('event.public.hero.mode') }}
          </dt>
          <dd class="mt-1 flex items-start gap-2 text-text">
            <UiIcon :name="FORMAT_ICONS[props.edition.participation_mode]" size="1.05rem" class="mt-0.5 shrink-0 text-text-muted" />
            <span>{{ t(`session-card.format.${props.edition.participation_mode}`) }}</span>
          </dd>
        </div>

        <div v-if="props.edition.has_pavilion">
          <dt class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
            {{ t('event.public.hero.pavilion') }}
          </dt>
          <dd class="mt-1 flex items-start gap-2 text-text">
            <UiIcon name="check" size="1.05rem" class="mt-0.5 shrink-0 text-success" />
            <span>{{ t('event.public.hero.pavilionHeld') }}</span>
          </dd>
        </div>
      </dl>

      <p class="mt-6 text-lg text-text-secondary" :style="{ maxWidth: 'var(--measure)' }">
        {{ tr(props.edition.description) }}
      </p>

      <p
        v-if="props.edition.highlights"
        class="mt-4 rounded-md border-l-4 border-accent bg-surface-sunken px-4 py-3 text-sm text-text-secondary"
        :style="{ maxWidth: 'var(--measure)' }"
      >
        {{ tr(props.edition.highlights) }}
      </p>
    </div>

    <!-- Le visuel, quand il existe. Chargement immédiat : il est au-dessus de la
         ligne de flottaison, le différer ferait sauter la mise en page. -->
    <UiImage
      v-if="props.cover"
      :image="props.cover"
      ratio="16 / 9"
      loading="eager"
      sizes="(min-width: 1024px) 40vw, 100vw"
      rounded="rounded-lg"
      class="border border-border"
    />
  </header>
</template>
