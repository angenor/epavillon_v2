<script setup lang="ts">
import type { IncidentScope, IncidentSeverity } from '~/types/live'
import type { I18nText, Url } from '~/types/shared'

/**
 * BANDEAU D'INCIDENT — motif transverse, publié depuis le back-office.
 *
 * Il rend une ligne de `live.active_incidents()` : la fonction remonte déjà la
 * hiérarchie (séance, journée, édition, organisation porteuse, plus le global)
 * et trie par gravité décroissante. Le front n'a donc AUCUN filtre de portée à
 * réimplémenter : il affiche ce qu'on lui donne, dans l'ordre reçu.
 *
 * DEUX RÈGLES DE PLACEMENT, qui n'appartiennent PAS à ce composant mais que
 * l'écran qui l'emploie doit tenir :
 * · UN SEUL BANDEAU AFFICHÉ À LA FOIS, quelle que soit la pile remontée par
 *   `active_incidents()` — c'est le plus grave qui parle. Deux bandeaux empilés,
 *   et plus aucun n'est lu.
 * · AU-DESSUS DE LA BARRE DE NAVIGATION, en tête de page, pleine largeur. Un
 *   incident glissé dans le flux du contenu passe sous le pli.
 * Aucun orchestrateur global n'est fourni ici volontairement : choisir QUEL
 * incident parle relève de la mise en page (`layouts/`), pas d'un composant
 * d'interface. `standalone` sert seulement à sortir du plein-fer quand le
 * bandeau est montré hors de ce rail (démonstration, aperçu de back-office).
 *
 * QUATRE NIVEAUX DE GRAVITÉ, PAS TROIS. L'ENUM `live.incident_severity` en
 * déclare quatre — `info`, `warning`, `error`, `critical` — dans cet ordre,
 * volontairement croissant côté base pour que `ORDER BY severity DESC` remonte
 * le plus grave. Le modèle fait foi : les quatre sont rendus. Le guide de style
 * n'en illustre que trois ; c'est le guide qui est en retard sur le modèle.
 *
 * MONTÉE EN INTENSITÉ, PAS QUATRE SURFACES PÂLES. Quatre fonds teintés se
 * ressemblent trop pour qu'on distingue « maintenance planifiée » de
 * « inscriptions suspendues » d'un coup d'œil :
 * · `info`               — fond teinté bordé, texte courant ;
 * · `warning`            — APLAT PLEIN jaune, texte sombre ;
 * · `error` / `critical` — APLAT PLEIN rouge, texte clair.
 * Sur un aplat, tout hérite de la couleur du bandeau, liens compris : un lien
 * cyan sur fond rouge est illisible dans les deux thèmes. Le soulignement prend
 * alors le relais pour signaler qu'il s'agit bien d'un lien.
 *
 * `info` et `warning` sont refermables si l'incident le permet ; `error` et
 * `critical` ne le sont JAMAIS, quoi qu'en dise `is_dismissible`. Un incident
 * majeur qu'on peut chasser d'un clic est un incident qui ne sera pas lu — et le
 * bandeau est le seul canal dont dispose l'équipe pendant une COP.
 *
 * `critical` partage l'aplat rouge d'`error` : la palette n'a pas de cinquième
 * famille d'état, et en inventer une désaccorderait tout le reste. Deux signaux
 * le distinguent — il ne se referme pas, et un trait de gauche épais le borde.
 *
 * `role="alert"` à partir de `error` seulement : un bandeau d'information
 * présent au chargement ne doit pas interrompre la lecture.
 *
 * LE TEXTE VIENT DE LA BASE — `title` et `message` sont des `platform.i18n_text`,
 * résolus par `resolveI18nText()`. Ils ne passent PAS par les fichiers i18n :
 * c'est une donnée saisie par un administrateur, pas un libellé d'interface.
 * Seuls les libellés du composant lui-même (nom de la gravité, « masquer ») sont
 * traduits.
 */

interface Props {
  severity: IncidentSeverity
  /** Titre multilingue venu de la base. Facultatif — le message peut suffire. */
  title?: I18nText | null
  /** Message multilingue venu de la base. Obligatoire côté modèle. */
  message: I18nText
  /** Portée déclarée, affichée en repère (« toute la plateforme », « cette activité »). */
  scope?: IncidentScope
  /**
   * SUJET NOMMÉ — « Atelier de négociation », « Journée finance », le nom légal
   * d'une organisation. Résolu par le modèle, jamais recomposé ici.
   *
   * Sans lui, « la diffusion est interrompue » ne dit pas de QUOI il s'agit sur
   * une page qui parle de trente activités. La portée seule ne suffit pas :
   * « cette activité » ne nomme rien quand le bandeau coiffe tout un programme.
   */
  targetLabel?: string | null
  actionUrl?: Url | null
  /** Libellé du lien d'action ; à défaut, un libellé générique. */
  actionLabel?: string
  /** Refermable — ignoré pour `error` et `critical`. */
  dismissible?: boolean
  /** Fin d'affichage annoncée, quand elle est connue. */
  displayUntil?: string | null
  /** Fuseau dans lequel afficher la date de fin. Requis avec `displayUntil`. */
  timezone?: string
  zoneLabel?: string
  /**
   * Le bandeau est montré HORS du rail de tête de page — aperçu, démonstration,
   * bloc de back-office. Il prend alors ses propres coins arrondis au lieu du
   * plein-fer bord à bord.
   */
  standalone?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ dismiss: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()

const isVisible = ref(true)

const ICONS: Record<IncidentSeverity, string> = {
  info: 'info',
  warning: 'warning',
  error: 'error',
  critical: 'ban',
}

/** Vrai dès que le bandeau passe en aplat plein — tout le rendu en dépend. */
const isSolid = computed(() => props.severity !== 'info')

const TONES: Record<IncidentSeverity, string> = {
  info: 'border-y border-info-border bg-info-surface text-text',
  warning: 'bg-warning-solid text-warning-contrast',
  error: 'bg-danger-solid text-danger-contrast',
  // Le trait de gauche est pris dans la couleur de CONTRASTE : sur un aplat
  // rouge, un rail rouge foncé ne se voit pas.
  critical:
    'bg-danger-solid text-danger-contrast border-l-(length:--border-thick) border-l-danger-contrast',
}

/** Un incident grave ne se referme pas, quoi qu'en dise la base. */
const canDismiss = computed(
  () => Boolean(props.dismissible) && props.severity !== 'error' && props.severity !== 'critical',
)

const until = computed(() => {
  if (!props.displayUntil || !props.timezone) return null
  return t('incident-banner.until', { date: dateTime(props.displayUntil, props.timezone) })
})

function dismiss(): void {
  isVisible.value = false
  emit('dismiss')
}
</script>

<template>
  <div
    v-if="isVisible"
    :role="props.severity === 'info' ? 'note' : 'alert'"
    class="ui-incident flex w-full gap-3 px-4 py-3 sm:px-6"
    :class="[TONES[props.severity], props.standalone ? 'rounded-md' : '']"
  >
    <!-- Aucune classe de couleur : l'icône hérite du ton du bandeau. -->
    <UiIcon :name="ICONS[props.severity]" class="mt-0.5 shrink-0" size="1.125rem" />

    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
        <!-- La gravité s'écrit en capitales dans la couleur du bandeau, plutôt
             qu'en pastille : une pastille porte ses propres couleurs et
             deviendrait illisible posée sur un aplat plein. -->
        <span class="text-xs font-bold tracking-(--tracking-caps) uppercase">
          {{ t(`incident-banner.severity.${props.severity}`) }}
        </span>
        <span v-if="props.scope" class="text-xs" :class="isSolid ? '' : 'text-text-subtle'">
          {{ t(`incident-banner.scope.${props.scope}`) }}
        </span>
        <span
          v-if="props.targetLabel"
          class="min-w-0 truncate text-xs font-medium"
          :class="isSolid ? '' : 'text-text'"
        >
          {{ props.targetLabel }}
        </span>
      </div>

      <p
        v-if="props.title"
        class="mt-1.5 font-display text-base leading-snug"
        :class="isSolid ? '' : 'text-text'"
      >
        {{ tr(props.title) }}
      </p>
      <p
        class="max-w-(--measure) text-sm"
        :class="[props.title ? 'mt-0.5' : 'mt-1.5', isSolid ? '' : 'text-text-secondary']"
      >
        {{ tr(props.message) }}
      </p>

      <div v-if="props.actionUrl || until" class="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1">
        <UiButton
          v-if="props.actionUrl"
          variant="link"
          size="sm"
          :href="props.actionUrl"
          icon-trailing="external-link"
        >
          {{ props.actionLabel ?? t('incident-banner.action') }}
        </UiButton>
        <span v-if="until" class="text-xs" :class="isSolid ? '' : 'text-text-subtle'">{{ until }}</span>
      </div>
    </div>

    <button
      v-if="canDismiss"
      type="button"
      class="ui-incident-close -mt-1 -mr-1 grid size-8 shrink-0 cursor-pointer place-items-center self-start rounded-sm text-inherit transition-colors duration-(--duration-fast)"
      @click="dismiss"
    >
      <span class="sr-only">{{ t('incident-banner.dismiss') }}</span>
      <UiIcon name="close" size="0.875rem" />
    </button>
  </div>
</template>

<style scoped>
/* Sur un aplat plein, un lien accentué est illisible : il prend la couleur du
   bandeau et c'est le soulignement qui dit que c'en est un. `:deep` parce que le
   lien est rendu par `UiButton`, hors de la portée du style scopé. */
.ui-incident:not([role="note"]) :deep(a) {
  color: inherit;
  text-decoration: underline;
}

.ui-incident-close:hover {
  background: color-mix(in srgb, currentColor 12%, transparent);
}
</style>
