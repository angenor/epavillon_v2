<script setup lang="ts">
import type { ContrastVerdict } from '~/utils/contrast'

/**
 * Section « Jetons » du guide de style — palette, typographie, espacements.
 *
 * LES RATIOS DE CONTRASTE SONT CALCULÉS, PAS RECOPIÉS. Ils sont déjà annotés
 * dans `design-tokens.css` ; les redire ici produirait deux vérités qui
 * divergeraient à la première retouche de palette. Le composant lit donc les
 * variables CSS RÉELLEMENT APPLIQUÉES et calcule le rapport WCAG à l'affichage.
 *
 * Conséquence heureuse : basculer le thème sombre recalcule tout. Les jetons de
 * rôle changent de valeur, et l'on voit immédiatement si un rôle est passé sous
 * le seuil — ce qu'un tableau figé ne montrerait jamais.
 *
 * Le calcul demande le DOM : au rendu serveur, la palette affiche ses squelettes.
 * C'est une démonstration de plus, et non un défaut à corriger.
 */

const { t, locale } = useI18n()

/** Familles de l'échelle de marque, avec le palier où vit la couleur officielle. */
const BRAND_SCALES: { name: string; labelKey: string; charter: number }[] = [
  { name: 'cyan', labelKey: 'style-guide.tokens.brand.cyan', charter: 500 },
  { name: 'rouge', labelKey: 'style-guide.tokens.brand.rouge', charter: 500 },
  { name: 'jaune', labelKey: 'style-guide.tokens.brand.jaune', charter: 400 },
  { name: 'vert', labelKey: 'style-guide.tokens.brand.vert', charter: 500 },
  { name: 'violet', labelKey: 'style-guide.tokens.brand.violet', charter: 700 },
  { name: 'bleu', labelKey: 'style-guide.tokens.brand.bleu', charter: 900 },
  { name: 'gris', labelKey: 'style-guide.tokens.brand.gris', charter: 700 },
]

const STEPS = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900]

/** Jetons de RÔLE — les seuls qu'un composant a le droit d'appeler. */
const ROLE_GROUPS: { labelKey: string; tokens: string[] }[] = [
  {
    labelKey: 'style-guide.tokens.roles.surfaces',
    tokens: [
      '--color-surface',
      '--color-surface-raised',
      '--color-surface-sunken',
      '--color-surface-overlay',
      '--color-surface-hover',
      '--color-surface-selected',
    ],
  },
  {
    labelKey: 'style-guide.tokens.roles.text',
    tokens: [
      '--color-text',
      '--color-text-muted',
      '--color-text-subtle',
      '--color-text-inverse',
      '--color-text-link',
    ],
  },
  {
    labelKey: 'style-guide.tokens.roles.borders',
    tokens: ['--color-border', '--color-border-subtle', '--color-border-strong', '--color-focus'],
  },
  {
    labelKey: 'style-guide.tokens.roles.accent',
    tokens: [
      '--color-accent',
      '--color-accent-solid',
      '--color-accent-surface',
      '--color-accent-border',
    ],
  },
  {
    labelKey: 'style-guide.tokens.roles.states',
    tokens: ['--color-success', '--color-warning', '--color-danger', '--color-info'],
  },
]

const TYPE_SCALE = [
  { token: '--font-size-3xl', class: 'text-3xl', usageKey: 'style-guide.tokens.type.usage3xl' },
  { token: '--font-size-2xl', class: 'text-2xl', usageKey: 'style-guide.tokens.type.usage2xl' },
  { token: '--font-size-xl', class: 'text-xl', usageKey: 'style-guide.tokens.type.usageXl' },
  { token: '--font-size-lg', class: 'text-lg', usageKey: 'style-guide.tokens.type.usageLg' },
  { token: '--font-size-base', class: 'text-base', usageKey: 'style-guide.tokens.type.usageBase' },
  { token: '--font-size-sm', class: 'text-sm', usageKey: 'style-guide.tokens.type.usageSm' },
  { token: '--font-size-xs', class: 'text-xs', usageKey: 'style-guide.tokens.type.usageXs' },
]

const SPACINGS = [1, 2, 3, 4, 5, 6, 8, 10, 12, 16, 20, 24]
const RADII = ['--radius-sm', '--radius-md', '--radius-lg', '--radius-xl', '--radius-full']
const SHADOWS = ['--shadow-xs', '--shadow-sm', '--shadow-md', '--shadow-lg']

interface Swatch {
  token: string
  value: string
  /** Rapport contre le fond de page ACTIF — thème compris. */
  ratio: number | null
  verdict: ContrastVerdict
}

const brandSwatches = ref<Record<string, Swatch[]>>({})
const roleSwatches = ref<Record<string, Swatch[]>>({})
const typeSizes = ref<Record<string, string>>({})
const spacingValues = ref<Record<string, string>>({})
const isMeasured = ref(false)

const preferences = usePreferencesStore()

/** Relève les valeurs appliquées et calcule les rapports. */
function measure(): void {
  if (typeof document === 'undefined') return
  const background = cssVariableValue('--color-surface')

  const brand: Record<string, Swatch[]> = {}
  for (const scale of BRAND_SCALES) {
    brand[scale.name] = STEPS.map((step) => {
      const token = `--ifdd-${scale.name}-${step}`
      const value = cssVariableValue(token)
      const ratio = contrastRatio(value, background)
      return { token, value, ratio, verdict: contrastVerdict(ratio) }
    })
  }
  brandSwatches.value = brand

  const roles: Record<string, Swatch[]> = {}
  for (const group of ROLE_GROUPS) {
    roles[group.labelKey] = group.tokens.map((token) => {
      const value = cssVariableValue(token)
      const ratio = contrastRatio(value, background)
      return { token, value, ratio, verdict: contrastVerdict(ratio) }
    })
  }
  roleSwatches.value = roles

  typeSizes.value = Object.fromEntries(
    TYPE_SCALE.map((entry) => [entry.token, cssVariableValue(entry.token)]),
  )
  spacingValues.value = Object.fromEntries(
    SPACINGS.map((step) => [`--space-${step}`, cssVariableValue(`--space-${step}`)]),
  )

  isMeasured.value = true
}

onMounted(() => {
  measure()
  // Le thème change les jetons de rôle : on remesure, et l'on voit sur-le-champ
  // si une nuance passe sous le seuil de contraste en thème sombre.
  watch(() => preferences.theme, () => nextTick(measure))
})

const VERDICT_INTENTS: Record<ContrastVerdict, 'success' | 'info' | 'warning' | 'neutral'> = {
  aaa: 'success',
  aa: 'success',
  'aa-large': 'warning',
  fail: 'neutral',
}

const ratioLabel = (swatch: Swatch): string => formatRatio(swatch.ratio, locale.value === 'en' ? 'en-GB' : 'fr-FR')
</script>

<template>
  <StyleGuideSection
    id="jetons"
    :title="t('style-guide.tokens.title')"
    :description="t('style-guide.tokens.description')"
  >
    <!-- 1. PALETTE DE MARQUE -->
    <StyleGuideDemo
      :title="t('style-guide.tokens.brand.title')"
      :note="t('style-guide.tokens.brand.note')"
    >
      <div v-if="!isMeasured" class="space-y-3">
        <UiSkeletonLoader v-for="scale in BRAND_SCALES" :key="scale.name" height="2.5rem" />
      </div>

      <div v-else class="space-y-5">
        <div v-for="scale in BRAND_SCALES" :key="scale.name">
          <p class="mb-1.5 text-sm font-medium text-text">{{ t(scale.labelKey) }}</p>
          <div class="grid grid-cols-2 gap-1 sm:grid-cols-5 lg:grid-cols-10">
            <div
              v-for="(swatch, index) in brandSwatches[scale.name] ?? []"
              :key="swatch.token"
              class="rounded-md border border-border-subtle p-1.5"
            >
              <div
                class="h-8 w-full rounded-sm ring-1 ring-ring-contrast/5"
                :style="{ backgroundColor: swatch.value }"
              />
              <p class="mt-1 font-mono text-[0.6875rem] leading-tight text-text-muted">
                {{ STEPS[index] }}
                <span v-if="STEPS[index] === scale.charter" class="text-accent" :title="t('style-guide.tokens.brand.charterMark')">◆</span>
              </p>
              <p class="font-mono text-[0.625rem] leading-tight text-text-subtle">{{ swatch.value }}</p>
              <p
                class="font-mono text-[0.625rem] leading-tight tabular-nums"
                :class="swatch.verdict === 'fail' ? 'text-text-subtle' : 'text-text-muted'"
              >
                {{ ratioLabel(swatch) }}
              </p>
            </div>
          </div>
        </div>
        <p class="text-sm text-text-subtle">{{ t('style-guide.tokens.brand.legend') }}</p>
      </div>
    </StyleGuideDemo>

    <!-- 2. JETONS DE RÔLE -->
    <StyleGuideDemo
      :title="t('style-guide.tokens.roles.title')"
      :note="t('style-guide.tokens.roles.note')"
    >
      <div v-if="!isMeasured" class="space-y-2">
        <UiSkeletonLoader v-for="group in ROLE_GROUPS" :key="group.labelKey" height="3rem" />
      </div>

      <div v-else class="space-y-5">
        <div v-for="group in ROLE_GROUPS" :key="group.labelKey">
          <p class="mb-1.5 text-sm font-medium text-text">{{ t(group.labelKey) }}</p>
          <ul class="grid gap-1.5 sm:grid-cols-2 lg:grid-cols-3">
            <li
              v-for="swatch in roleSwatches[group.labelKey] ?? []"
              :key="swatch.token"
              class="flex items-center gap-2.5 rounded-md border border-border-subtle px-2.5 py-2"
            >
              <span
                class="size-7 shrink-0 rounded-md ring-1 ring-ring-contrast/10"
                :style="{ backgroundColor: swatch.value }"
              />
              <span class="min-w-0 flex-1">
                <span class="block truncate font-mono text-xs text-text">{{ swatch.token }}</span>
                <span class="block font-mono text-[0.625rem] text-text-subtle">{{ swatch.value }}</span>
              </span>
              <UiBadge :intent="VERDICT_INTENTS[swatch.verdict]" size="sm">
                {{ ratioLabel(swatch) }}
              </UiBadge>
            </li>
          </ul>
        </div>
      </div>
    </StyleGuideDemo>

    <!-- 3. ÉCHELLE TYPOGRAPHIQUE -->
    <StyleGuideDemo
      :title="t('style-guide.tokens.type.title')"
      :note="t('style-guide.tokens.type.note')"
    >
      <dl class="space-y-4">
        <div
          v-for="entry in TYPE_SCALE"
          :key="entry.token"
          class="flex flex-col gap-1 border-b border-border-subtle pb-4 last:border-0 last:pb-0 sm:flex-row sm:items-baseline sm:gap-6"
        >
          <dt class="w-56 shrink-0 font-mono text-xs text-text-subtle">
            {{ entry.token }}
            <span class="ml-1 tabular-nums">{{ typeSizes[entry.token] ?? '' }}</span>
          </dt>
          <dd class="min-w-0 flex-1">
            <p :class="[entry.class, 'font-display leading-tight text-text']">
              {{ t('style-guide.tokens.type.sample') }}
            </p>
            <p class="mt-0.5 text-sm text-text-subtle">{{ t(entry.usageKey) }}</p>
          </dd>
        </div>
      </dl>
    </StyleGuideDemo>

    <!-- 4. ESPACEMENTS, RAYONS, OMBRES -->
    <StyleGuideDemo
      :title="t('style-guide.tokens.spacing.title')"
      :note="t('style-guide.tokens.spacing.note')"
    >
      <div class="space-y-6">
        <ul class="space-y-1.5">
          <li v-for="step in SPACINGS" :key="step" class="flex items-center gap-3">
            <span class="w-24 shrink-0 font-mono text-xs text-text-subtle">--space-{{ step }}</span>
            <span class="w-12 shrink-0 font-mono text-xs tabular-nums text-text-muted">
              {{ spacingValues[`--space-${step}`] ?? '' }}
            </span>
            <span class="h-3 rounded-sm bg-accent-solid" :style="{ width: `var(--space-${step})` }" />
          </li>
        </ul>

        <div>
          <p class="mb-2 text-sm font-medium text-text">{{ t('style-guide.tokens.radius.title') }}</p>
          <div class="flex flex-wrap gap-3">
            <div v-for="token in RADII" :key="token" class="text-center">
              <div
                class="size-16 border border-border-strong bg-surface-sunken"
                :style="{ borderRadius: `var(${token})` }"
              />
              <p class="mt-1 font-mono text-[0.625rem] text-text-subtle">{{ token.replace('--radius-', '') }}</p>
            </div>
          </div>
        </div>

        <div>
          <p class="mb-2 text-sm font-medium text-text">{{ t('style-guide.tokens.shadow.title') }}</p>
          <p class="mb-2 text-sm text-text-subtle">{{ t('style-guide.tokens.shadow.note') }}</p>
          <div class="flex flex-wrap gap-4">
            <div v-for="token in SHADOWS" :key="token" class="text-center">
              <div
                class="size-16 rounded-lg border border-border-subtle bg-surface-raised"
                :style="{ boxShadow: `var(${token})` }"
              />
              <p class="mt-1 font-mono text-[0.625rem] text-text-subtle">{{ token.replace('--shadow-', '') }}</p>
            </div>
          </div>
        </div>
      </div>
    </StyleGuideDemo>
  </StyleGuideSection>
</template>
