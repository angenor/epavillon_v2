<script setup lang="ts">
import type { InviteMemberPayload, WorkspaceMember } from '~/types/organization-workspace'
import type { Membership } from '~/types/org'
import type { Intent, SelectOption } from '~/types/ui'
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
  members: WorkspaceMember[]
  organizationName: string
  /** Personne connectée, pour marquer sa propre ligne. */
  viewerId: PersonId
  /** Le rôle du lecteur ouvre (ou non) l'invitation et la modération. */
  isManager: boolean
  /** Fiche pas encore validée par l'équipe : le référent est provisoire, il
   *  n'invite ni n'accepte personne (31/10). L'API le refuse aussi. */
  pendingValidation?: boolean
  busy?: boolean
  /** Résultat de la dernière invitation ou décision, affiché sous le formulaire. */
  message?: { intent: Intent; text: string } | null
}

const props = withDefaults(defineProps<Props>(), { pendingValidation: false, busy: false, message: null })
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
// Sans référent actif, personne ici ne peut accepter : c'est l'équipe de l'ePavillon qui tranche (31/10).
const hasManager = computed(() => active.value.some((entry) => entry.membership.role === 'manager'))
const canModerate = computed(() => props.isManager && !props.pendingValidation)

function roleLabel(role: Membership['role']): string {
  return t(
    role === 'manager' && props.pendingValidation
      ? 'organization.workspace.members.role.provisionalManager'
      : `organization.workspace.members.role.${role}`,
  )
}

function pendingLabel(entry: WorkspaceMember): string {
  if (entry.is_invitation) {
    return t('organization.workspace.members.status.invited', { date: date(entry.membership.invited_at, viewerTimezone.value) })
  }
  // Sa propre demande : « reçue » se lirait comme une demande qu'on aurait soi-même reçue.
  const key = entry.person.id === props.viewerId ? 'awaiting' : 'requested'
  return t(`organization.workspace.members.status.${key}`, { date: date(entry.membership.created_at, viewerTimezone.value) })
}

// Seule l'équipe nomme un référent (31/10) : une invitation n'en fait pas un.
const roleOptions = computed<SelectOption[]>(() =>
  (['member', 'contributor'] satisfies Membership['role'][]).map((role) => ({
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
 * Une seule vérification côté écran, la forme de l'adresse : le reste (adresse
 * déjà membre, invitation déjà en vol) est une RÉPONSE de l'API, pas une
 * devinette du formulaire.
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

/** Aucun repli sur l'adresse : les deux colonnes de nom sont non vides en base,
 *  et une personne invitée porte le libellé neutre que l'API lui écrit. */
function nameOf(entry: WorkspaceMember): string {
  return `${entry.person.first_name} ${entry.person.last_name}`.trim()
}
</script>

<template>
  <section id="membres" aria-labelledby="workspace-members-title" class="scroll-mt-24">
    <WorkspaceRule heading-id="workspace-members-title" :label="t('organization.workspace.members.title')" :count="active.length" />
    <p class="mt-3.5 max-w-(--measure) text-[0.92rem] text-text-muted">
      {{ t('organization.workspace.members.description', { organization: props.organizationName }) }}
    </p>

    <ul class="mt-5 border-t border-border-subtle">
      <li
        v-for="entry in [...active, ...pending]"
        :key="entry.membership.id"
        class="grid grid-cols-[44px_minmax(0,1fr)] items-center gap-x-4 gap-y-2 border-b border-border-subtle py-3.5 md:grid-cols-[48px_minmax(0,1fr)_auto_auto]"
      >
        <span
          aria-hidden="true"
          class="grid size-11 place-items-center rounded-full border border-border font-condensed text-[1.1rem] font-bold text-text-secondary md:size-12"
          :class="entry.membership.status === 'pending' ? 'border-dashed' : 'bg-surface-sunken'"
        >
          {{ initialsOf(nameOf(entry)) }}
        </span>
        <span class="min-w-0">
          <span class="font-bold text-text">{{ nameOf(entry) }}</span>
          <span
            v-if="entry.person.id === props.viewerId"
            class="ml-2 text-[0.72rem] font-bold tracking-[0.05em] text-accent uppercase"
          >
            {{ t('organization.workspace.members.status.you') }}
          </span>
          <span v-if="entry.membership.job_title" class="block text-[0.85rem] text-text-muted">
            {{ entry.membership.job_title }}
          </span>
          <!-- La DIRECTION de l'attente, écrite en toutes lettres : « En attente »
               seul laisserait deviner qui doit bouger. -->
          <span v-if="entry.membership.status === 'pending'" class="block text-[0.85rem] text-text-subtle">
            {{ pendingLabel(entry) }}
          </span>
        </span>

        <UiBadge
          class="col-start-2 justify-self-start md:col-start-3"
          :intent="entry.membership.role === 'manager' ? 'info' : 'neutral'"
          :label="roleLabel(entry.membership.role)"
        />

        <span class="col-start-2 flex flex-wrap gap-2 md:col-start-4">
          <template v-if="entry.membership.status === 'pending' && canModerate && !entry.is_invitation">
            <UiButton
              variant="secondary"
              icon="check"
              :disabled="props.busy"
              @click="emit('decide', entry.membership.id, true)"
            >
              {{ t('organization.workspace.members.accept') }}
            </UiButton>
            <UiButton
              variant="ghost"
              :disabled="props.busy"
              @click="emit('decide', entry.membership.id, false)"
            >
              {{ t('organization.workspace.members.decline') }}
            </UiButton>
          </template>
          <UiButton
            v-else-if="entry.membership.status === 'pending' && canModerate && entry.is_invitation"
            variant="ghost"
            icon="mail"
            :disabled="props.busy"
          >
            {{ t('organization.workspace.members.remind') }}
          </UiButton>
        </span>
      </li>
    </ul>

    <UiAlert v-if="props.message" class="mt-6" :intent="props.message.intent" :message="props.message.text" live />

    <!-- L'invitation. Réservée au référent, et le dire quand on ne l'est pas. -->
    <UiAlert
      v-if="props.isManager && props.pendingValidation"
      class="mt-6"
      :title="t('organization.workspace.members.pendingValidation.title')"
      :message="t('organization.workspace.members.pendingValidation.message')"
    />
    <div v-else-if="props.isManager" class="mt-7 rounded-lg border border-border bg-surface-raised p-5">
      <h3 class="font-condensed text-[1.45rem] leading-tight font-semibold text-heading">
        {{ t('organization.workspace.members.invite.title') }}
      </h3>
      <p class="mt-1 max-w-[64ch] text-sm text-text-muted">
        {{ t('organization.workspace.members.invite.description') }}
      </p>

      <form
        class="mt-4.5 grid grid-cols-1 items-start gap-4 lg:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)_200px_auto]"
        @submit.prevent="submit()"
      >
        <UiInput
          id="invite-email"
          v-model="email"
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
          :label="t('organization.workspace.members.invite.jobTitle')"
          :placeholder="t('organization.workspace.members.invite.jobTitlePlaceholder')"
        />
        <UiSelect
          id="invite-role"
          v-model="role"
          :label="t('organization.workspace.members.invite.role')"
          :options="roleOptions"
          hide-optional
        />
        <UiButton type="submit" variant="primary" class="max-lg:w-full lg:mt-7" :loading="props.busy">
          {{ t('organization.workspace.members.invite.submit') }}
        </UiButton>
      </form>
    </div>

    <UiAlert
      v-else-if="!hasManager"
      class="mt-6"
      :title="t('organization.workspace.members.noManager.title')"
      :message="t('organization.workspace.members.noManager.message')"
    />
    <p v-else class="mt-4 text-sm text-text-subtle">
      {{ t('organization.workspace.members.managerOnly') }}
    </p>
  </section>
</template>
