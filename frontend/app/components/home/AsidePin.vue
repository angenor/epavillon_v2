<script setup lang="ts">
import type { ShowcaseRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'

/**
 * UNE ÉPINGLE ÉDITORIALE DU PANNEAU « À VENIR » — un `home_aside` de
 * `content.v_showcase`.
 *
 * ELLES REMPLACENT CINQ COMPOSANTS ÉCRITS EN DUR. La v1 portait cinq widgets
 * d'annonce — FPHN, PACO, Chypre… — dont le texte, les dates et l'image vivaient
 * dans des fichiers Vue. Chaque nouvelle annonce demandait un déploiement, et
 * les anciennes survivaient parce que personne ne pensait à retirer le
 * composant. Ici, l'IFDD compose depuis le back-office et la fenêtre de
 * diffusion éteint l'épingle toute seule.
 *
 * LA FENÊTRE EST DÉJÀ APPLIQUÉE PAR LA VUE : `starts_at` et `ends_at` ne servent
 * qu'à L'ANNONCER (« jusqu'au 30 septembre »), jamais à re-filtrer. Rejouer le
 * filtre ici, c'est reprendre le défaut qu'on vient de corriger.
 *
 * LA DATE DE FIN EST UNE DATE, SANS HEURE, ET C'EST DÉLIBÉRÉ. Un jour se lit
 * sans fuseau, et c'est tout ce dont le lecteur a besoin pour savoir jusqu'à
 * quand l'information vaut. Le fuseau reste indispensable pour CHOISIR ce jour :
 * une fenêtre qui s'achève le 30 septembre à 23 h 59 heure de Belém tombe le
 * 1er octobre en temps universel, et l'épingle annoncerait alors un jour de plus
 * que son propre texte. D'où l'ordre : le fuseau de l'ÉDITION de rattachement,
 * que le panneau connaît et transmet ; à défaut celui de la séance mise en
 * avant ; à défaut le temps universel, pour un contenu de plateforme qui n'est
 * situé nulle part.
 */

interface Props {
  pin: ShowcaseRow
  /** Fuseau de l'édition de rattachement, résolu par le panneau. */
  timezone?: TimeZoneName | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

const thumbnail = computed(() => showcaseThumbnail(props.pin))

const zone = computed<TimeZoneName>(
  () => props.timezone ?? props.pin.session_timezone ?? 'UTC',
)

const until = computed(() =>
  props.pin.ends_at ? date(props.pin.ends_at, zone.value) : '',
)

const linkLabel = computed(() => tr(props.pin.link_label) || t('home.aside.pins.link'))
</script>

<template>
  <article class="flex gap-3 rounded-lg border border-glass-border bg-glass-raised p-3 shadow-glass backdrop-blur-glass transition-colors hover:bg-glass-hover">
    <div
      v-if="thumbnail || props.pin.background_color_hex"
      class="size-16 shrink-0 overflow-hidden rounded-md"
      :style="
        thumbnail ? undefined : { backgroundColor: props.pin.background_color_hex ?? undefined }
      "
    >
      <UiImage :image="thumbnail" ratio="1 / 1" frame-class="size-full" sizes="64px" />
    </div>

    <div class="min-w-0 flex-1">
      <HomeNatureBadge
        :label="props.pin.nature_label"
        :color="props.pin.nature_color"
        size="sm"
        tone="inverse"
      />

      <h4 class="mt-1.5 text-sm leading-snug font-bold text-text-on-inverse">
        {{ tr(props.pin.title) }}
      </h4>

      <p v-if="props.pin.quote" class="mt-1 text-sm text-text-on-inverse-muted">
        {{ tr(props.pin.quote) }}
      </p>

      <p v-if="until" class="mt-1.5 text-xs text-text-on-inverse-muted">
        {{ t('home.aside.pins.until', { date: until }) }}
      </p>

      <a
        v-if="props.pin.link_url"
        :href="props.pin.link_url"
        target="_blank"
        rel="noopener noreferrer"
        class="mt-2 inline-flex items-center gap-1 text-sm font-medium text-text-on-inverse underline-offset-4 hover:underline"
      >
        {{ linkLabel }}
        <UiIcon name="arrow-up-right" size="0.9rem" />
        <span class="sr-only">{{ t('common.a11y.externalLink') }}</span>
      </a>
    </div>
  </article>
</template>
