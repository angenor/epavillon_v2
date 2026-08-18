<script setup lang="ts">
import type { AdminAction } from '~/types/admin-dashboard'
import type { TimeZoneName } from '~/types/shared'

/**
 * CE QUI DEMANDE UNE ACTION — le bloc le plus haut du tableau de bord, et la
 * seule zone de l'écran qui coûte quelque chose si on ne la lit pas.
 *
 * TROIS RÈGLES, ET CHACUNE SE PAIE SI ON L'INVERSE :
 *
 *  1. VIDE, IL RESTE LISIBLE. C'est la contrainte explicite du prompt : un
 *     back-office où tout va bien ne doit pas ressembler à un écran cassé. D'où
 *     un encart calme et affirmatif — ni bordure rouge, ni glyphe d'alerte, ni
 *     zone grise laissée béante. « Rien n'attend l'équipe » est une réponse.
 *  2. UNE LIGNE PAR FAMILLE, jamais une par élément. Quarante dossiers non
 *     évalués donneraient quarante lignes, et le bloc deviendrait la liste des
 *     propositions — qui existe déjà, avec ses filtres. Le décompte, trois
 *     exemples nommés, et un lien vers l'écran DÉJÀ RÉGLÉ sur le problème.
 *  3. LE NOMBRE EST DANS LA PHRASE, PAS DANS UNE PASTILLE À PART. « 7 revues en
 *     retard » se lit d'un trait ; un « 7 » posé à côté d'un libellé oblige à
 *     deviner ce qu'il compte.
 *
 * LES COULEURS SUIVENT LA RÈGLE D'USAGE, et aucune ligne n'est rouge : rien ici
 * n'est un échec. Le jaune dit « à traiter », le cyan « à regarder ». Le rouge
 * est réservé à l'échec, à la suppression et au direct — le peindre sur une file
 * de travail en ferait une alarme permanente, qu'on cesse de lire.
 */

interface Props {
  actions: AdminAction[]
  /** Fuseau de l'édition : toute échéance affichée le porte. */
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { date } = useDateTime()
const localePath = useLocalePath()

/** Une icône par famille. Le titre porte le sens ; l'icône ne fait que trier l'œil. */
const ICONS: Record<AdminAction['kind'], string> = {
  proposals_unreviewed: 'inbox',
  reviews_overdue: 'clock',
  organization_duplicates: 'building',
  schedule_conflicts: 'calendar',
  active_incidents: 'broadcast',
}

const TONES: Record<AdminAction['severity'], string> = {
  high: 'border-l-warning bg-warning-surface/40',
  medium: 'border-l-info bg-info-surface/40',
}

const ICON_TONES: Record<AdminAction['severity'], string> = {
  high: 'text-warning',
  medium: 'text-info',
}

const total = computed(() => props.actions.reduce((sum, action) => sum + action.count, 0))
</script>

<template>
  <section aria-labelledby="admin-actions-title">
    <div class="mb-4 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
      <h2 id="admin-actions-title" class="text-xl font-semibold">
        {{ t('admin.dashboard.actions.title') }}
      </h2>
      <p v-if="props.actions.length > 0" class="text-sm text-text-subtle">
        {{ t('admin.dashboard.actions.total', total) }}
      </p>
    </div>

    <!-- L'ÉTAT VIDE N'EST PAS UN ÉTAT D'ERREUR. Un encart en retrait, une coche
         verte, une phrase qui affirme — et l'heure de la mesure, pour qu'on
         sache que l'écran a bien regardé. -->
    <UiCard v-if="props.actions.length === 0" sunken>
      <div class="flex items-start gap-3">
        <UiIcon name="check-circle" class="mt-0.5 shrink-0 text-success" size="1.5rem" />
        <div>
          <p class="font-semibold text-text">{{ t('admin.dashboard.actions.empty.title') }}</p>
          <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
            {{ t('admin.dashboard.actions.empty.description') }}
          </p>
        </div>
      </div>
    </UiCard>

    <ul v-else class="flex flex-col gap-2">
      <li v-for="action in props.actions" :key="action.kind">
        <NuxtLink
          :to="localePath(action.target)"
          class="flex min-h-(--target-min) items-start gap-3 rounded-md border border-border border-l-(length:--border-thick) px-4 py-3 no-underline transition-colors duration-(--duration-fast) hover:border-border-strong"
          :class="TONES[action.severity]"
        >
          <UiIcon
            :name="ICONS[action.kind]"
            class="mt-0.5 shrink-0"
            :class="ICON_TONES[action.severity]"
            size="1.25rem"
          />

          <span class="min-w-0 flex-1">
            <span class="block font-semibold text-text">
              {{ t(`admin.dashboard.actions.kind.${action.kind}.label`, action.count) }}
            </span>
            <span class="mt-0.5 block text-sm text-text-muted">
              {{ t(`admin.dashboard.actions.kind.${action.kind}.detail`) }}
            </span>

            <!-- LES EXEMPLES NOMMÉS. « 7 revues en retard » ne dit pas par où
                 commencer ; « Lemoine (3), Ben Amor (2) » le dit. Trois au plus :
                 au-delà, c'est l'écran concerné qu'il faut ouvrir. -->
            <span v-if="action.examples.length" class="mt-1.5 flex flex-wrap items-center gap-x-2 gap-y-1">
              <!-- LES DEUX PARTS SE TRONQUENT, et la précision est bornée à 40 %
                   de la pastille. Sans cette borne, un libellé long — le titre
                   complet d'une édition sur un incident de portée « événement » —
                   pousse la pastille hors de la carte, et TOUTE LA PAGE se met à
                   défiler horizontalement à 375 px. Mesuré, puis corrigé. -->
              <span
                v-for="example in action.examples"
                :key="example.label"
                class="inline-flex max-w-full min-w-0 items-baseline gap-1 rounded-sm bg-surface-raised px-2 py-0.5 text-xs text-text-secondary"
              >
                <span class="truncate">{{ example.label }}</span>
                <span v-if="example.hint" class="max-w-[40%] shrink-0 truncate font-mono text-text-subtle">
                  {{ example.hint }}
                </span>
              </span>
              <span v-if="action.count > action.examples.length" class="text-xs text-text-subtle">
                {{ t('admin.dashboard.actions.more', action.count - action.examples.length) }}
              </span>
            </span>
          </span>

          <span
            v-if="action.due_at"
            class="hidden shrink-0 self-center text-sm tabular-nums text-text-secondary sm:block"
          >
            {{ t('admin.dashboard.actions.dueOn', { date: date(action.due_at, props.timezone) }) }}
          </span>

          <UiIcon name="chevron-right" class="mt-0.5 shrink-0 text-text-subtle" size="1.125rem" />
        </NuxtLink>
      </li>
    </ul>
  </section>
</template>
