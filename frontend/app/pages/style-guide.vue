<script setup lang="ts">
import type { ThemeBadge } from '~/types/ui'
import type { OrganizationId } from '~/types/shared'

/**
 * GUIDE DE STYLE VIVANT — la référence visuelle du projet.
 *
 * ELLE FAIT FOI. Trois usages, et ils tiennent ensemble :
 *
 * · RÉFÉRENCE pendant tout le développement — quand un écran a besoin d'un
 *   bouton, d'un état vide ou d'une carte de séance, c'est ici qu'on regarde à
 *   quoi il ressemble et comment il s'appelle.
 * · TEST DE NON-RÉGRESSION VISUELLE — les composants sont RENDUS, pas décrits.
 *   Une retouche de jeton ou de composant se voit ici sur tous ses états à la
 *   fois, en clair comme en sombre.
 * · CONTRÔLE D'ACCESSIBILITÉ — les rapports de contraste sont calculés à
 *   l'affichage, sur les valeurs réellement appliquées. Basculer le thème les
 *   recalcule.
 *
 * LES DONNÉES SONT RÉELLES ET PASSENT PAR `useApi()`, comme dans n'importe quel
 * écran. Aucune page n'importe un mock — pas même celle-ci. C'est ce qui garantit
 * que les composants sont éprouvés sur ce qu'ils rencontreront : des titres
 * d'activité longs, des noms d'organisation à rallonge, des jauges dépassées.
 *
 * ÉTAT DES DONNÉES : la page traite ses quatre états comme n'importe quel écran.
 * C'est aussi une démonstration — un guide de style qui ne tiendrait pas ses
 * propres règles ne ferait pas foi longtemps.
 */

definePageMeta({ layout: 'public' })

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()

/** L'édition de référence des données simulées — COP31, Belém, novembre 2027. */
const EVENT_SLUG = 'cop31-belem-2027'

/**
 * Périmètre d'administration : global, pour que la liste des propositions
 * s'affiche entière. Dans un écran réel, il vient de
 * `identity.administered_events()` et n'est jamais posé à la main.
 */
const ADMIN_SCOPE = { is_global: true, event_ids: [] as string[] }

const { data, pending, error, refresh } = await useAsyncData('style-guide-data', async () => {
  const event = await api.events.bySlug(EVENT_SLUG)
  if (!event) return null

  const [schedule, themeTerms, organizations, countries, dashboard] = await Promise.all([
    api.sessions.schedule(event.id),
    api.reference.terms('activity_theme'),
    api.organizations.list(),
    api.reference.countries(),
    api.proposals.dashboard(event.id, ADMIN_SCOPE),
  ])

  return { event, schedule, themeTerms, organizations, countries, dashboard }
})

/** Nom du lieu — le libellé de fuseau préféré à la ville déduite de l'IANA. */
const zoneLabel = computed(() => data.value?.event.city ?? '')

/**
 * Thématiques indexées par code. La carte de séance n'en a plus besoin — la vue
 * porte `themes`, libellé et couleur compris —, mais la démonstration des
 * pastilles thématiques, elle, montre la taxonomie ENTIÈRE et non les seules
 * thématiques présentes dans six séances.
 */
const themesByCode = computed<Record<string, ThemeBadge>>(() => {
  const entries = (data.value?.themeTerms ?? []).map((term) => [
    term.code,
    { code: term.code, label: term.label, color: term.color_hex },
  ])
  return Object.fromEntries(entries)
})

const themeList = computed<ThemeBadge[]>(() => Object.values(themesByCode.value))

/*
 * Le pays de chaque organisation était recomposé ici, `v_public_schedule` ne le
 * joignant pas. La vue expose `organization_country` depuis le 17/08 : la carte
 * de séance le lit sur sa ligne, et cette recomposition a disparu avec l'écart.
 */

/** Six séances pour les cartes, douze dossiers pour le tableau. */
const sessions = computed(() => (data.value?.schedule ?? []).slice(0, 6))
const proposals = computed(() => data.value?.dashboard ?? [])

/** Titres réels passés aux exemples génériques — ce sont des données, pas des libellés. */
const sampleTitles = computed(() => sessions.value.map((session) => tr(session.title)))

const SECTIONS = [
  { id: 'jetons', labelKey: 'style-guide.nav.tokens' },
  { id: 'composants-base', labelKey: 'style-guide.nav.base' },
  { id: 'formulaires', labelKey: 'style-guide.nav.forms' },
  { id: 'donnees', labelKey: 'style-guide.nav.data' },
  { id: 'navigation', labelKey: 'style-guide.nav.navigation' },
  { id: 'composants-metier', labelKey: 'style-guide.nav.business' },
  { id: 'motifs', labelKey: 'style-guide.nav.patterns' },
]

useHead({ title: t('style-guide.title') })
</script>

<template>
  <div>
    <header class="max-w-prose">
      <p class="text-sm tracking-wide text-text-subtle uppercase">{{ t('style-guide.eyebrow') }}</p>
      <h1 class="mt-2 text-3xl">{{ t('style-guide.title') }}</h1>
      <p class="mt-3 text-lg text-text-muted">{{ t('style-guide.intro') }}</p>
      <p class="mt-3 text-sm text-text-muted">{{ t('style-guide.authority') }}</p>
    </header>

    <!-- Sommaire ancré : le guide fait plusieurs écrans, et l'on y renvoie. -->
    <nav :aria-label="t('style-guide.nav.label')" class="mt-8">
      <ul class="flex flex-wrap gap-2">
        <li v-for="section in SECTIONS" :key="section.id">
          <a
            :href="`#${section.id}`"
            class="inline-block rounded-md border border-border bg-surface-raised px-3 py-1.5 text-sm text-text no-underline transition-colors hover:border-border-strong hover:bg-surface-hover"
          >
            {{ t(section.labelKey) }}
          </a>
        </li>
      </ul>
    </nav>

    <UiAlert intent="info" class="mt-6" :title="t('style-guide.themeHint.title')">
      {{ t('style-guide.themeHint.body') }}
    </UiAlert>

    <!-- Les quatre états, tenus par la page elle-même. -->
    <UiErrorState
      v-if="error"
      class="mt-10"
      :description="t('style-guide.dataError')"
      :detail="error.message"
      @retry="refresh"
    />

    <UiEmptyState
      v-else-if="!pending && !data"
      class="mt-10"
      :title="t('style-guide.noEvent.title')"
      :description="t('style-guide.noEvent.description', { slug: EVENT_SLUG })"
    />

    <div v-else class="mt-12 space-y-12">
      <StyleGuideTokensSection />

      <StyleGuideBaseSection
        :themes="themeList"
        :titles="sampleTitles"
        :event-acronym="data?.event.acronym"
      />

      <StyleGuideFormsSection />

      <StyleGuideDataSection :rows="proposals" :loading="pending" />

      <StyleGuideNavigationSection />

      <StyleGuideBusinessSection
        :sessions="sessions"
        :zone-label="zoneLabel"
        :loading="pending"
      />

      <StyleGuidePatternsSection :session="sessions[0] ?? null" :zone-label="zoneLabel" />
    </div>
  </div>
</template>
