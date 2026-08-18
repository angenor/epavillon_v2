<script setup lang="ts">
import type { IncidentPayload } from '~/types/admin-incidents'
import type { LocaleCode, TimeZoneName } from '~/types/shared'

/**
 * L'APERÇU EN DIRECT DU BANDEAU, tel que le public le lira.
 *
 * IL REND LE VRAI COMPOSANT, PAS UNE MAQUETTE. `UiIncidentBanner` est celui que
 * la programmation publique et la fiche d'activité affichent : un aperçu dessiné
 * à part divergerait au premier ajustement, et c'est précisément ce qu'on vient
 * vérifier ici — un aplat rouge ne se juge pas sur une description.
 *
 * DEUX RÈGLES DU BANDEAU SE VOIENT ICI, ET C'EST LEUR INTÉRÊT :
 * · un incident et un incident majeur NE SE REFERMENT PAS, quoi que dise la
 *   case « refermable » du formulaire ;
 * · à partir de `error`, le bandeau passe en aplat plein — l'aperçu montre donc
 *   ce que coûte le choix de gravité, au moment où on le fait.
 *
 * LA LANGUE SE BASCULE. Le message est écrit dans les deux langues et personne
 * ne relit l'anglais s'il faut changer la langue de toute l'interface pour le
 * voir. La bascule ne touche QUE l'aperçu.
 *
 * `standalone` : le bandeau sort de son rail pleine largeur et prend ses coins
 * arrondis — c'est le cas d'usage que son en-tête prévoit pour un aperçu.
 */

interface Props {
  payload: IncidentPayload
  timezone: TimeZoneName
  zoneLabel?: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { dateTime } = useDateTime()

const previewLocale = ref<LocaleCode>('fr')
const LOCALES: LocaleCode[] = ['fr', 'en']

/** Le bandeau résout lui-même l'`i18n_text` : on lui donne la langue choisie. */
const previewTitle = computed(() =>
  props.payload.title ? { fr: props.payload.title[previewLocale.value] ?? '' } : null,
)
const previewMessage = computed(() => ({ fr: props.payload.message[previewLocale.value] ?? '' }))

const hasContent = computed(() => (previewMessage.value.fr ?? '').trim().length > 0)

/** Un message programmé ne parle pas encore : l'aperçu le dit plutôt que de mentir. */
const scheduledNotice = computed(() => {
  if (Date.parse(props.payload.display_from) <= Date.now()) return null
  return t('admin.incident.form.preview.scheduledNotice', {
    date: dateTime(props.payload.display_from, props.timezone),
  })
})
</script>

<template>
  <section class="rounded-lg border border-border bg-surface-raised p-4">
    <header class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-2">
      <div>
        <h2 class="font-display text-base">{{ t('admin.incident.form.preview.title') }}</h2>
        <p class="text-sm text-text-muted">{{ t('admin.incident.form.preview.description') }}</p>
      </div>

      <div class="flex items-center gap-1" role="group" :aria-label="t('admin.incident.form.preview.locale')">
        <UiButton
          v-for="locale in LOCALES"
          :key="locale"
          :variant="previewLocale === locale ? 'secondary' : 'ghost'"
          size="sm"
          @click="previewLocale = locale"
        >
          {{ locale.toUpperCase() }}
        </UiButton>
      </div>
    </header>

    <div class="mt-4">
      <UiIncidentBanner
        v-if="hasContent"
        :key="previewLocale"
        :severity="payload.severity"
        :title="previewTitle"
        :message="previewMessage"
        :scope="payload.scope"
        :action-url="payload.action_url"
        :dismissible="payload.is_dismissible"
        :display-until="payload.display_until"
        :timezone="timezone"
        :zone-label="zoneLabel"
        standalone
      />
      <p v-else class="rounded-md border border-dashed border-border p-6 text-center text-sm text-text-subtle">
        {{ t('admin.incident.form.preview.empty') }}
      </p>
    </div>

    <p v-if="scheduledNotice && hasContent" class="mt-3 flex items-center gap-1.5 text-sm text-text-muted">
      <UiIcon name="calendar" size="0.9rem" />
      {{ scheduledNotice }}
    </p>
  </section>
</template>
