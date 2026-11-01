<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'

/**
 * LES QUATRE FAITS D'UNE ÉDITION, dans le bandeau : dates, lieu, participation,
 * pavillon. Une carte chacun : libellé en petites capitales, valeur en
 * condensé gras, précision dessous. Une `<dl>`, ce sont des données.
 *
 * `glass` sur la photo de l'édition (jetons du verre) ; `inverse` sur l'aplat
 * bleu nuit, où le verre n'a pas de média à séparer.
 *
 * Toute date porte le fuseau de l'édition, jamais celui du visiteur.
 */

interface Props {
  edition: EventEdition
  /** Nom du pays hôte, déjà résolu. */
  country?: string | null
  tone?: 'glass' | 'inverse'
}

const props = withDefaults(defineProps<Props>(), { country: null, tone: 'glass' })

const { t } = useI18n()
const { dateRange, zoneLabel } = useDateTime()

const FORMAT_ICONS: Record<EventEdition['participation_mode'], string> = {
  online: 'monitor',
  in_person: 'map-pin',
  hybrid: 'users',
}

interface Fact {
  key: string
  icon: string
  label: string
  value: string
  detail?: string
}

const facts = computed<Fact[]>(() => {
  const edition = props.edition
  const list: Fact[] = [
    {
      key: 'dates',
      icon: 'calendar',
      label: t('event.public.hero.dates'),
      value: dateRange(edition.starts_at, edition.ends_at, edition.timezone),
      detail: zoneLabel(edition.timezone, edition.city ?? undefined),
    },
  ]

  // La ville en grand, le pays dessous ; sans ville, le pays seul.
  const place = edition.city ?? props.country
  if (place) {
    const detail = [edition.city ? props.country : null, edition.address].filter((part): part is string => Boolean(part))
    list.push({
      key: 'place',
      icon: 'map-pin',
      label: t('event.public.hero.place'),
      value: place,
      detail: detail.join(', ') || undefined,
    })
  }

  list.push({
    key: 'mode',
    icon: FORMAT_ICONS[edition.participation_mode],
    label: t('event.public.hero.mode'),
    value: t(`session-card.format.${edition.participation_mode}`),
    detail: t(`event.public.hero.modeDetail.${edition.participation_mode}`),
  })

  if (edition.has_pavilion) {
    list.push({
      key: 'pavilion',
      icon: 'building',
      label: t('event.public.hero.pavilion'),
      value: t('event.public.hero.pavilionValue'),
      detail: t('event.public.hero.pavilionHeld'),
    })
  }

  return list
})

const tileClass = computed(() =>
  props.tone === 'glass'
    ? 'border-glass-border bg-glass backdrop-blur-glass'
    : 'border-border-on-inverse bg-surface-inverse-raised',
)
</script>

<template>
  <dl class="grid grid-cols-2 gap-2.5 lg:grid-cols-4">
    <div v-for="fact in facts" :key="fact.key" class="min-w-0 rounded-lg border px-3.5 py-3" :class="tileClass">
      <dt class="flex items-center gap-1.5 text-[0.7rem] font-bold tracking-[0.07em] text-text-on-inverse-muted uppercase">
        <UiIcon :name="fact.icon" size="0.95rem" class="shrink-0" />
        {{ fact.label }}
      </dt>
      <dd class="mt-1">
        <span class="block font-condensed text-[1.3rem] leading-[1.05] font-bold tracking-[0.02em] text-text-on-inverse uppercase [overflow-wrap:anywhere]">
          {{ fact.value }}
        </span>
        <span v-if="fact.detail" class="mt-0.5 block text-[0.78rem] leading-snug text-text-on-inverse-muted">{{ fact.detail }}</span>
      </dd>
    </div>
  </dl>
</template>
