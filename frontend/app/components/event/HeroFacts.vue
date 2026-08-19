<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'

/**
 * LES QUATRE FAITS D'UNE ÉDITION — dates, lieu, mode, pavillon.
 *
 * ── POURQUOI CE COMPOSANT EXISTE ────────────────────────────────────────────
 *
 * `EventHero` a DEUX rendus : un bandeau plein cadre quand l'édition porte un
 * visuel, l'en-tête sobre sinon. Les faits sont les mêmes dans les deux, seule
 * la MATIÈRE change — du verre sur la photographie, une surface de page sinon.
 * Écrits deux fois, ils auraient divergé au premier ajout, et c'est justement là
 * qu'une divergence se voit le moins : un seul des deux rendus est à l'écran.
 *
 * ── DES TUILES, ET PLUS UNE LISTE EN DEUX COLONNES (19/08) ──────────────────
 *
 * La liste de définitions posait ses quatre entrées à même le fond. Sur une
 * photographie, chacune flottait sans attache et l'ensemble se lisait comme un
 * paragraphe éclaté. Chaque fait tient désormais dans sa propre tuile : un cadre
 * léger, une icône, un intitulé, une valeur. Le contraste ne dépend plus de ce
 * que montre l'image derrière — c'est la tuile qui porte le fond.
 *
 * Cela reste une `<dl>` : ce sont des données, pas une accroche. L'icône n'a
 * jamais le sens à sa charge, l'intitulé est toujours écrit.
 *
 * ── TOUTE DATE PORTE SON FUSEAU ─────────────────────────────────────────────
 *
 * Celui de l'édition (`event.events.timezone`), jamais celui du visiteur :
 * « du 9 au 20 novembre 2027, heure de Belém ».
 */

interface Props {
  edition: EventEdition
  /** Nom du pays hôte, déjà résolu depuis `reference.countries`. */
  country?: string | null
  /** `glass` : posées sur un média voilé. `surface` : sur un fond de page. */
  tone?: 'glass' | 'surface'
}

const props = withDefaults(defineProps<Props>(), { tone: 'surface' })

const { t } = useI18n()
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

interface Fact {
  key: string
  icon: string
  label: string
  value: string
  detail?: string
  /** Le vert du pavillon reste le vert : c'est une information CONFIRMÉE. */
  iconClass?: string
}

const facts = computed<Fact[]>(() => {
  const list: Fact[] = [
    {
      key: 'dates',
      icon: 'calendar',
      label: t('event.public.hero.dates'),
      value: dates.value,
      detail: zone.value,
    },
  ]

  if (place.value) {
    list.push({
      key: 'place',
      icon: 'map-pin',
      label: t('event.public.hero.place'),
      value: place.value,
      detail: props.edition.address ?? undefined,
    })
  }

  list.push({
    key: 'mode',
    icon: FORMAT_ICONS[props.edition.participation_mode],
    label: t('event.public.hero.mode'),
    value: t(`session-card.format.${props.edition.participation_mode}`),
  })

  if (props.edition.has_pavilion) {
    list.push({
      key: 'pavilion',
      icon: 'check',
      label: t('event.public.hero.pavilion'),
      value: t('event.public.hero.pavilionHeld'),
      iconClass: 'text-success',
    })
  }

  return list
})

const glass = computed(() => props.tone === 'glass')

const tileClass = computed(() =>
  glass.value
    ? 'border-glass-border bg-glass-raised backdrop-blur-glass'
    : 'border-border bg-surface-raised',
)
const labelClass = computed(() => (glass.value ? 'text-text-on-inverse-muted' : 'text-text-subtle'))
const valueClass = computed(() => (glass.value ? 'text-text-on-inverse' : 'text-text'))
const detailClass = computed(() =>
  glass.value ? 'text-text-on-inverse-muted' : 'text-text-muted',
)
</script>

<template>
  <dl class="grid gap-3 sm:grid-cols-2">
    <div
      v-for="fact in facts"
      :key="fact.key"
      class="flex items-start gap-3 rounded-lg border px-4 py-3"
      :class="tileClass"
    >
      <UiIcon
        :name="fact.icon"
        size="1.05rem"
        class="mt-0.5 shrink-0"
        :class="fact.iconClass ?? detailClass"
      />
      <div class="min-w-0">
        <dt
          class="text-xs uppercase"
          :class="labelClass"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{ fact.label }}
        </dt>
        <dd class="mt-0.5 text-sm" :class="valueClass">
          {{ fact.value }}
          <span v-if="fact.detail" class="block text-xs" :class="detailClass">{{ fact.detail }}</span>
        </dd>
      </div>
    </div>
  </dl>
</template>
