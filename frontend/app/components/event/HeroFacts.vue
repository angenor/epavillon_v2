<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'

/**
 * LES QUATRE FAITS D'UNE ÉDITION — dates, lieu, mode, pavillon.
 *
 * ── POURQUOI CE COMPOSANT EXISTE ────────────────────────────────────────────
 *
 * `EventHero` a DEUX rendus depuis le 19/08 : un bandeau plein cadre quand
 * l'édition porte un visuel, l'en-tête sobre sinon. Les faits sont les mêmes
 * dans les deux, seules les couleurs changent. Écrits deux fois, ils auraient
 * divergé au premier ajout — et c'est justement là qu'une divergence se voit le
 * moins, puisqu'un seul des deux rendus est à l'écran à la fois.
 *
 * ── UNE LISTE DE DÉFINITIONS, PAS UNE ACCROCHE ──────────────────────────────
 *
 * Ce sont des données. Chacune porte son icône, mais le SENS est dans le texte :
 * l'icône ne remplace jamais l'intitulé.
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
  /** `inverse` : posés sur un média voilé. `surface` : sur un fond de page. */
  tone?: 'inverse' | 'surface'
}

const props = withDefaults(defineProps<Props>(), { tone: 'surface' })

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

const inverse = computed(() => props.tone === 'inverse')

const labelClass = computed(() =>
  inverse.value ? 'text-text-on-inverse-muted' : 'text-text-subtle',
)
const valueClass = computed(() => (inverse.value ? 'text-text-on-inverse' : 'text-text'))
const detailClass = computed(() =>
  inverse.value ? 'text-text-on-inverse-muted' : 'text-text-muted',
)
</script>

<template>
  <dl class="grid gap-4 sm:grid-cols-2">
    <div>
      <dt class="text-xs uppercase" :class="labelClass" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('event.public.hero.dates') }}
      </dt>
      <dd class="mt-1 flex items-start gap-2" :class="valueClass">
        <UiIcon name="calendar" size="1.05rem" class="mt-0.5 shrink-0" :class="detailClass" />
        <span>
          {{ dates }}
          <span class="block text-sm" :class="detailClass">{{ zone }}</span>
        </span>
      </dd>
    </div>

    <div v-if="place">
      <dt class="text-xs uppercase" :class="labelClass" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('event.public.hero.place') }}
      </dt>
      <dd class="mt-1 flex items-start gap-2" :class="valueClass">
        <UiIcon name="map-pin" size="1.05rem" class="mt-0.5 shrink-0" :class="detailClass" />
        <span>
          {{ place }}
          <span v-if="props.edition.address" class="block text-sm" :class="detailClass">
            {{ props.edition.address }}
          </span>
        </span>
      </dd>
    </div>

    <div>
      <dt class="text-xs uppercase" :class="labelClass" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('event.public.hero.mode') }}
      </dt>
      <dd class="mt-1 flex items-start gap-2" :class="valueClass">
        <UiIcon
          :name="FORMAT_ICONS[props.edition.participation_mode]"
          size="1.05rem"
          class="mt-0.5 shrink-0"
          :class="detailClass"
        />
        <span>{{ t(`session-card.format.${props.edition.participation_mode}`) }}</span>
      </dd>
    </div>

    <div v-if="props.edition.has_pavilion">
      <dt class="text-xs uppercase" :class="labelClass" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('event.public.hero.pavilion') }}
      </dt>
      <dd class="mt-1 flex items-start gap-2" :class="valueClass">
        <!-- LE VERT RESTE LE VERT, sur photographie comme sur fond de page : un
             pavillon tenu est une information CONFIRMÉE, et la couleur d'état ne
             change pas parce que le fond a changé. -->
        <UiIcon name="check" size="1.05rem" class="mt-0.5 shrink-0 text-success" />
        <span>{{ t('event.public.hero.pavilionHeld') }}</span>
      </dd>
    </div>
  </dl>
</template>
