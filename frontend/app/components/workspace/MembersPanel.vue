<script setup lang="ts">
import type { InviteMemberPayload, MemberEntry } from '~/types/organization-workspace'
import type { Membership } from '~/types/org'
import type { SelectOption } from '~/types/ui'
import type { PersonId } from '~/types/shared'

/**
 * LES MEMBRES DE L'ORGANISATION, ET L'INVITATION PAR ADRESSE.
 *
 * UNE ADHÉSION « EN ATTENTE » A DEUX ORIGINES OPPOSÉES, et cet écran est le seul
 * endroit où la confusion se paierait : une DEMANDE reçue s'accepte, une
 * INVITATION émise se relance. Le modèle les distingue depuis le 17/08
 * (`memberships.invited_at`) ; sans cela, un référent aurait approuvé sa propre
 * invitation et donné une adhésion active à quelqu'un qui n'a rien accepté.
 *
 * SEUL UN RÉFÉRENT INVITE ET ACCEPTE. Le formulaire n'apparaît pas aux autres, et
 * l'écran le DIT plutôt que de le cacher en silence : un membre qui cherche
 * comment inviter une collègue doit comprendre pourquoi il ne trouve pas, sans
 * quoi il écrira à l'IFDD.
 *
 * ON N'INVITE PAS DEUX FOIS. Une invitation déjà en vol se relance ; en émettre
 * une seconde se heurterait à `ux_memberships`, et l'écran l'annonce au lieu de
 * laisser la base refuser.
 */

interface Props {
  members: MemberEntry[]
  organizationName: string
  /** Personne connectée — pour marquer sa propre ligne. */
  viewerId: PersonId
  /** Le rôle du lecteur ouvre — ou non — l'invitation et la modération. */
  isManager: boolean
  busy?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  invite: [payload: Omit<InviteMemberPayload, 'organization_id'>]
  /** Décision sur une DEMANDE d'adhésion. Une invitation ne s'approuve pas. */
  decide: [membershipId: string, approved: boolean]
}>()

const { t } = useI18n()
const { date } = useDateTime()

/** Fuseau du lecteur : ces dates concernent SA gestion, pas une édition. */
const viewerTimezone = computed(
  () => Intl.DateTimeFormat().resolvedOptions().timeZone || 'Europe/Paris',
)

const active = computed(() => props.members.filter((entry) => entry.membership.status === 'active'))
const pending = computed(() => props.members.filter((entry) => entry.membership.status === 'pending'))

const roleOptions = computed<SelectOption[]>(() =>
  (['member', 'contributor', 'manager'] satisfies Membership['role'][]).map((role) => ({
    value: role,
    label: t(`organization.workspace.members.role.${role}`),
  })),
)

// --- Formulaire d'invitation ------------------------------------------------

const email = ref('')
const role = ref<Membership['role']>('member')
const jobTitle = ref('')
const emailError = ref('')

/**
 * Une seule vérification côté écran, et c'est la forme de l'adresse : le reste
 * — adresse déjà membre, invitation déjà en vol — est une RÉPONSE de l'API, pas
 * une devinette du formulaire.
 */
function submit(): void {
  const value = email.value.trim()
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/.test(value)) {
    emailError.value = t('validation.email')
    return
  }
  emailError.value = ''
  emit('invite', {
    email: value,
    role: role.value,
    job_title: jobTitle.value.trim() || null,
  })
  email.value = ''
  jobTitle.value = ''
}

/** Nom affichable : une personne invitée n'a pas encore donné le sien. */
function nameOf(entry: MemberEntry): string {
  const full = `${entry.person.first_name} ${entry.person.last_name}`.trim()
  return full.length > 0 ? full : entry.person.primary_email
}
</script>

<template>
  <section id="membres" aria-labelledby="workspace-members-title" class="scroll-mt-24">
    <div class="mb-4 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
      <h2 id="workspace-members-title" class="text-xl font-semibold">
        {{ t('organization.workspace.members.title') }}
      </h2>
      <p class="text-sm text-text-subtle">
        {{ t('organization.workspace.members.count', active.length) }}
      </p>
    </div>
    <p class="max-w-(--measure) text-sm text-text-muted">
      {{ t('organization.workspace.members.description', { organization: props.organizationName }) }}
    </p>

    <ul class="mt-4 flex flex-col divide-y divide-border-subtle rounded-lg border border-border bg-surface-raised">
      <li
        v-for="entry in [...active, ...pending]"
        :key="entry.membership.id"
        class="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3"
      >
        <span class="min-w-0 flex-1">
          <span class="flex flex-wrap items-baseline gap-x-2">
            <span class="font-semibold text-text">{{ nameOf(entry) }}</span>
            <span
              v-if="entry.person.id === props.viewerId"
              class="text-xs font-semibold tracking-wide text-accent uppercase"
            >
              {{ t('organization.workspace.members.status.you') }}
            </span>
          </span>
          <span v-if="entry.membership.job_title" class="mt-0.5 block text-sm text-text-muted">
            {{ entry.membership.job_title }}
          </span>
          <!-- La DIRECTION de l'attente, écrite en toutes lettres. « En
               attente » seul laisserait deviner qui doit bouger. -->
          <span v-if="entry.membership.status === 'pending'" class="mt-0.5 block text-sm text-text-subtle">
            {{
              entry.is_invitation
                ? t('organization.workspace.members.status.invited', {
                    date: date(entry.membership.invited_at, viewerTimezone),
                  })
                : t('organization.workspace.members.status.requested', {
                    date: date(entry.membership.created_at, viewerTimezone),
                  })
            }}
          </span>
        </span>

        <UiBadge
          :intent="entry.membership.role === 'manager' ? 'info' : 'neutral'"
          :label="t(`organization.workspace.members.role.${entry.membership.role}`)"
          size="sm"
        />

        <span v-if="entry.membership.status === 'pending'" class="flex flex-wrap gap-2">
          <template v-if="props.isManager && !entry.is_invitation">
            <UiButton
              variant="secondary"
              size="sm"
              icon="check"
              :disabled="props.busy"
              @click="emit('decide', entry.membership.id, true)"
            >
              {{ t('organization.workspace.members.accept') }}
            </UiButton>
            <UiButton
              variant="ghost"
              size="sm"
              :disabled="props.busy"
              @click="emit('decide', entry.membership.id, false)"
            >
              {{ t('organization.workspace.members.decline') }}
            </UiButton>
          </template>
          <UiButton
            v-else-if="props.isManager"
            variant="ghost"
            size="sm"
            icon="mail"
            :disabled="props.busy"
          >
            {{ t('organization.workspace.members.remind') }}
          </UiButton>
        </span>
      </li>
    </ul>

    <!-- L'invitation. Réservée au référent, et le dire quand on ne l'est pas. -->
    <UiCard v-if="props.isManager" class="mt-5" :title="t('organization.workspace.members.invite.title')">
      <p class="max-w-(--measure) text-sm text-text-muted">
        {{ t('organization.workspace.members.invite.description') }}
      </p>

      <form class="mt-4 flex flex-col gap-4 sm:flex-row sm:items-start" @submit.prevent="submit()">
        <UiInput
          id="invite-email"
          v-model="email"
          class="flex-1"
          type="email"
          autocomplete="email"
          :label="t('organization.workspace.members.invite.email')"
          :placeholder="t('organization.workspace.members.invite.emailPlaceholder')"
          :error="emailError"
          required
        />

        <UiInput
          id="invite-job-title"
          v-model="jobTitle"
          class="flex-1"
          :label="t('organization.workspace.members.invite.jobTitle')"
          :placeholder="t('organization.workspace.members.invite.jobTitlePlaceholder')"
        />

        <UiSelect
          id="invite-role"
          v-model="role"
          class="sm:w-52"
          :label="t('organization.workspace.members.invite.role')"
          :options="roleOptions"
          hide-optional
        />

        <UiButton type="submit" variant="primary" class="sm:mt-7" :loading="props.busy">
          {{ t('organization.workspace.members.invite.submit') }}
        </UiButton>
      </form>
    </UiCard>

    <p v-else class="mt-4 text-sm text-text-subtle">
      {{ t('organization.workspace.members.managerOnly') }}
    </p>
  </section>
</template>
