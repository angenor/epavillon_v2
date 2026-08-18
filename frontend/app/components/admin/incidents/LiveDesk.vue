<script setup lang="ts">
import type { LiveDesk, LiveDeskSession } from '~/types/admin-incidents'
import type { TimeZoneName } from '~/types/shared'

/**
 * LE POSTE DE DIRECT — ce qui se joue maintenant, et le geste qui va avec.
 *
 * IL EXISTE PARCE QU'UN MESSAGE D'INCIDENT SE RÉDIT EN SITUATION. La salle
 * attend, l'intervenante ne s'est pas connectée, la diffusion vient de tomber :
 * demander alors de choisir une portée parmi cinq, une nature parmi neuf et une
 * cible dans une liste de trente activités, c'est demander trois décisions à
 * quelqu'un qui n'a pas trois secondes. Ici la question est posée à l'envers —
 * voici l'activité, que se passe-t-il ? — et chaque réponse ouvre le formulaire
 * DÉJÀ rempli de la portée, de la cible, de la nature et de la fenêtre.
 *
 * QUATRE CAS, PAS NEUF. Retard, débordement, panne technique, diffusion
 * interrompue : ce sont ceux qui arrivent pendant qu'une activité se tient. Les
 * cinq autres natures du vocabulaire — changement de salle, annulation,
 * information — se décident à l'avance et passent par le formulaire complet.
 *
 * « DIFFUSION INTERROMPUE » N'APPARAÎT QUE SI L'ACTIVITÉ EST DIFFUSÉE
 * (`sessions.is_streamed`). Offrir de signaler la panne d'un direct qui n'existe
 * pas produirait un bandeau que personne ne comprendrait.
 *
 * LE POSTE NE PUBLIE RIEN LUI-MÊME. Chaque bouton est un LIEN vers le
 * formulaire : le message part sous les yeux de qui le signe, avec son aperçu.
 * Un bouton qui publierait un bandeau au public en un clic, sans relecture, est
 * exactement ce qu'il ne faut pas mettre dans un écran qu'on utilise dans
 * l'urgence.
 *
 * LA DATE N'APPARAÎT QUE HORS PÉRIODE. Quand le poste montre la journée en
 * cours, l'heure suffit — on sait quel jour on est. En repli, elle est
 * indispensable : « 14:00 — 15:30 » pour une activité d'octobre 2027 se lirait
 * comme un créneau de cet après-midi.
 *
 * IL DIT AUSSI CE QUI EST DÉJÀ DIT. Une activité qui porte déjà un message actif
 * l'annonce : republier la même panne est le meilleur moyen que le public cesse
 * de lire les bandeaux.
 */

interface Props {
  desk: LiveDesk
  timezone: TimeZoneName
  zoneLabel?: string
  /** `live.incident.publish` sur cette édition — sans elle, aucun geste offert. */
  canPublish?: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date: formatDate } = useDateTime()
const localePath = useLocalePath()

/** Les quatre natures du direct, dans l'ordre où elles arrivent en salle. */
const QUICK_KINDS = ['delay', 'overrun', 'technical_issue', 'connection_issue'] as const
type QuickKind = (typeof QUICK_KINDS)[number]

/** La panne de diffusion ne se signale que sur une activité diffusée. */
function kindsFor(session: LiveDeskSession): QuickKind[] {
  return QUICK_KINDS.filter((kind) => kind !== 'connection_issue' || session.is_streamed)
}

/**
 * Le formulaire reçoit tout par l'URL : portée, cible, nature. La fenêtre
 * d'affichage, elle, est déduite du créneau de l'activité par la page de
 * publication — c'est elle qui connaît le fuseau de l'édition.
 */
function reportLink(session: LiveDeskSession, kind: QuickKind): string {
  return localePath({
    path: '/admin/incidents/nouveau',
    query: { portee: 'session', cible: session.session_id, nature: kind },
  })
}

const STATE_INTENT = {
  ongoing: 'danger',
  upcoming: 'info',
  past: 'neutral',
  cancelled: 'neutral',
  postponed: 'warning',
} as const

const dayLabel = computed(() => formatDate(props.desk.day, props.timezone))
</script>

<template>
  <section
    class="rounded-lg border border-border bg-surface-raised"
    :aria-label="t('admin.incident.list.desk.title')"
  >
    <header class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 border-b border-border px-4 py-3">
      <h2 class="font-display text-base">{{ t('admin.incident.list.desk.title') }}</h2>
      <!-- LE REPLI SE DIT. « Rien aujourd'hui » et « voici la suite » ne sont pas
           la même information, et les confondre ferait croire à un direct en
           cours hors période. -->
      <p class="text-sm text-text-muted">
        {{
          desk.is_fallback
            ? t('admin.incident.list.desk.fallback', { date: dayLabel })
            : t('admin.incident.list.desk.today', { date: dayLabel })
        }}
      </p>
    </header>

    <p v-if="desk.sessions.length === 0" class="px-4 py-6 text-sm text-text-muted">
      {{ t('admin.incident.list.desk.empty') }}
    </p>

    <ul v-else class="divide-y divide-border">
      <li
        v-for="session in desk.sessions"
        :key="session.session_id"
        class="flex flex-wrap items-start justify-between gap-x-6 gap-y-3 px-4 py-3"
      >
        <div class="min-w-0 flex-1">
          <div class="flex flex-wrap items-center gap-2">
            <!-- En cours = en rouge, comme le direct partout ailleurs dans la
                 charte. C'est le seul endroit de l'écran où le rouge ne dit pas
                 une gravité mais un état d'antenne. -->
            <UiBadge
              :intent="STATE_INTENT[session.temporal_state]"
              size="sm"
              solid
              :label="t(`admin.incident.list.desk.state.${session.temporal_state}`)"
            />
            <p class="truncate font-medium">{{ tr(session.title) }}</p>
          </div>

          <p class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-text-muted">
            <UiZonedTime
              :start="session.starts_at"
              :end="session.ends_at"
              :timezone="timezone"
              :zone-label="zoneLabel"
              :format="desk.is_fallback ? 'withDate' : 'short'"
            />
            <span v-if="session.room_name">{{ tr(session.room_name) }}</span>
            <span v-if="session.is_streamed" class="flex items-center gap-1">
              <UiIcon name="broadcast" size="0.85rem" />
              {{ t('admin.incident.list.desk.streamed') }}
            </span>
          </p>

          <p
            v-if="session.active_incident_count > 0"
            class="mt-1 flex items-center gap-1.5 text-sm text-warning"
          >
            <UiIcon name="warning" size="0.85rem" />
            {{ t('admin.incident.list.desk.alreadyPublished', { count: session.active_incident_count }) }}
          </p>
        </div>

        <div v-if="canPublish" class="flex flex-wrap items-center gap-2">
          <UiButton
            v-for="kind in kindsFor(session)"
            :key="kind"
            variant="secondary"
            size="sm"
            :to="reportLink(session, kind)"
          >
            {{ t(`admin.incident.list.desk.report.${kind}`) }}
          </UiButton>
        </div>
      </li>
    </ul>
  </section>
</template>
