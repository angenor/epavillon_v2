<script setup lang="ts">
import type { ReminderSlot } from '~/types/organization-workspace'
import type { TimeZoneName } from '~/types/shared'

/**
 * LE CALENDRIER DES RAPPELS D'UNE ACTIVITÉ.
 *
 * QUATRE RAPPELS PARTENT, ET ILS SONT CUMULÉS : deux jours, un jour, une heure
 * et trente minutes avant le début. Ce n'est PAS un choix parmi quatre — c'est
 * la règle du commanditaire, et le modèle l'a écrite telle quelle dans le défaut
 * de `engagement.reminder_rules.offsets`. Le composant l'énonce en toutes
 * lettres au-dessus de la liste : quatre lignes sans cette phrase se lisent
 * comme quatre options.
 *
 * PARTI OU À VENIR, ET RIEN ENTRE LES DEUX. C'est la seule question que se pose
 * une organisation en regardant cette liste, et deux repères y répondent
 * ensemble : la pastille d'état écrite en toutes lettres, et l'horodatage. La
 * couleur seule ne dit jamais un état — règle du guide, et ici elle protège d'un
 * contresens qui coûterait cher un jour de COP.
 *
 * L'HEURE EST CELLE DE L'ÉDITION, pas celle du lecteur. Un rappel qui part
 * « 30 minutes avant » part 30 minutes avant l'activité, où qu'on le lise : le
 * fuseau affiché est donc celui du pavillon, et il est nommé.
 */

interface Props {
  reminders: ReminderSlot[]
  /** Fuseau de l'édition — jamais celui du navigateur. */
  timezone: TimeZoneName
  /** Nom du fuseau tel qu'il s'annonce (« heure de Belém »). */
  zoneLabel?: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { dateTime } = useDateTime()

/** Le fuseau se nomme par sa ville, comme partout ailleurs sur la plateforme. */
const zone = computed(() => props.zoneLabel?.trim() || timeZoneCityLabel(props.timezone))

/**
 * Le décalage, dit dans l'unité qui se lit. `2880 minutes avant` est exact et
 * illisible ; « 2 jours avant » est la même chose et se comprend d'un coup.
 */
function offsetLabel(minutes: number): string {
  if (minutes % (24 * 60) === 0) {
    return t('organization.workspace.proposal.sessions.reminders.offset.days', minutes / (24 * 60))
  }
  if (minutes % 60 === 0) {
    return t('organization.workspace.proposal.sessions.reminders.offset.hours', minutes / 60)
  }
  return t('organization.workspace.proposal.sessions.reminders.offset.minutes', minutes)
}

/** Trois états seulement à l'affichage : parti, à venir, écarté. */
function stateOf(slot: ReminderSlot): 'sent' | 'pending' | 'skipped' {
  if (slot.status === 'sent') return 'sent'
  if (slot.status === 'skipped' || slot.status === 'cancelled') return 'skipped'
  return 'pending'
}
</script>

<template>
  <section class="rounded-lg border border-border-subtle bg-surface-sunken p-4">
    <h4 class="text-sm font-semibold text-text">
      {{ t('organization.workspace.proposal.sessions.reminders.title') }}
    </h4>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('organization.workspace.proposal.sessions.reminders.description', { zone }) }}
    </p>

    <p v-if="props.reminders.length === 0" class="mt-3 text-sm text-text-subtle">
      {{ t('organization.workspace.proposal.sessions.reminders.noRecipients') }}
    </p>

    <ul v-else class="mt-3 flex flex-col divide-y divide-border-subtle">
      <li
        v-for="slot in props.reminders"
        :key="`${slot.offset_before}-${slot.channel}`"
        class="flex flex-wrap items-center gap-x-4 gap-y-1 py-2.5"
      >
        <!-- Le décalage en tête, largeur fixe : la colonne se parcourt d'un
             regard, du plus lointain au plus proche. -->
        <span class="w-32 shrink-0 text-sm font-semibold text-text">{{ offsetLabel(slot.offset_before) }}</span>

        <UiBadge
          :intent="stateOf(slot) === 'sent' ? 'success' : stateOf(slot) === 'skipped' ? 'neutral' : 'info'"
          :icon="stateOf(slot) === 'sent' ? 'check' : stateOf(slot) === 'skipped' ? 'minus' : 'clock'"
          :label="t(`organization.workspace.proposal.sessions.reminders.${stateOf(slot)}`)"
          size="sm"
        />

        <time :datetime="slot.scheduled_for" class="text-sm tabular-nums text-text-secondary">
          {{ dateTime(slot.scheduled_for, props.timezone) }}
        </time>

        <span class="text-sm text-text-muted">
          {{ t('organization.workspace.proposal.sessions.reminders.recipients', slot.recipient_count) }}
          ·
          {{ t(`organization.workspace.proposal.sessions.reminders.channel.${slot.channel}`) }}
        </span>
      </li>
    </ul>
  </section>
</template>
