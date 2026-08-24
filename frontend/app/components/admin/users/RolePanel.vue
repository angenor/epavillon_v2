<script setup lang="ts">
import type {
  AssignableRole,
  GrantRolePayload,
  RoleAssignmentOptions,
  RoleAssignmentView,
  ScopeChoice,
} from '~/types/admin-users'
import type { EffectivePermission, ScopeType } from '~/types/identity'
import type { SelectOption } from '~/types/ui'

/**
 * LE PANNEAU D'ATTRIBUTION — le point central de cet écran, et sa raison d'être.
 *
 * Le prompt le dit sans détour : « l'interface doit rendre évidente la différence
 * entre Administrateur, Administrateur de la COP31, Révisionniste de la COP31 et
 * Référent de l'organisation X ». Ces quatre lignes ne portent que DEUX rôles.
 * Tout ce qui les sépare est la portée — et un formulaire qui la traiterait comme
 * un champ parmi d'autres, en bas, après la date de fin, reproduirait exactement
 * le malentendu de la v1.
 *
 * D'OÙ LA FORME DE CE PANNEAU, EN TROIS TEMPS QUI SE LISENT COMME UNE PHRASE :
 *
 *   1. QUEL RÔLE ?        chaque rôle annonce les portées qu'il admet
 *   2. SUR QUOI ?         le choix se réduit à ce que le rôle admet ET à ce que
 *                         l'acteur peut accorder ; la portée globale est un choix
 *                         explicite, jamais une valeur par défaut silencieuse
 *   3. JUSQU'À QUAND, ET POURQUOI ?
 *
 * Et un quatrième temps, qui n'est pas une saisie : CE QUE ÇA CHANGE. Le panneau
 * annonce, avant d'écrire, les permissions que la personne GAGNERA — et dit
 * quand elle n'en gagne aucune, le cas d'une administratrice globale à qui l'on
 * ajoute un rôle sur une édition. Sans cette phrase, on attribue à l'aveugle.
 *
 * LA PORTÉE GLOBALE N'EST PAS COCHÉE PAR DÉFAUT. C'est la portée la plus large de
 * la plateforme ; l'obtenir par simple inattention est précisément l'accident que
 * cet écran doit rendre impossible.
 */

interface Props {
  open: boolean
  personName: string
  options: RoleAssignmentOptions | null
  /** Attributions en cours de la personne — pour détecter le doublon avant l'envoi. */
  assignments: RoleAssignmentView[]
  /** Permissions effectives de la PERSONNE VISÉE, pour calculer ce qu'elle gagne. */
  targetPermissions: EffectivePermission[]
  submitting?: boolean
  /** Refus renvoyé par l'API, déjà traduit. */
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  submit: [payload: GrantRolePayload]
}>()

const { t } = useI18n()
const { tr } = useI18nText()

const roleCode = ref<string>('')
const scopeType = ref<ScopeType | ''>('')
const scopeId = ref<string>('')
const validFrom = ref<string>('')
const validUntil = ref<string>('')
const note = ref<string>('')
const showAllPermissions = ref(false)

/** Le panneau se rouvre vierge : un motif oublié d'une attribution précédente
 *  se retrouverait attaché à la suivante, et l'historique deviendrait faux. */
watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return
    roleCode.value = ''
    scopeType.value = ''
    scopeId.value = ''
    validFrom.value = ''
    validUntil.value = ''
    note.value = ''
    showAllPermissions.value = false
  },
)

const selectedRole = computed<AssignableRole | null>(
  () => props.options?.roles.find((role) => role.code === roleCode.value) ?? null,
)

const roleOptions = computed<SelectOption[]>(() =>
  (props.options?.roles ?? []).map((role) => ({
    value: role.code,
    label: tr(role.label),
    description: role.description ? tr(role.description) : undefined,
  })),
)

/**
 * Les portées offertes : celles que le rôle admet, ET que l'acteur peut accorder.
 *
 * Une portée admise par le rôle mais interdite à l'acteur reste AFFICHÉE,
 * désactivée : la masquer ferait croire que le rôle ne l'admet pas, et
 * l'opérateur chercherait longtemps pourquoi « Administrateur » ne peut pas être
 * global.
 */
const scopeOptions = computed<{ value: ScopeType; label: string; allowed: boolean }[]>(() => {
  const role = selectedRole.value
  if (!role || !props.options) return []

  return role.allowed_scopes.map((scope) => ({
    value: scope,
    label: t(`admin.user.roles.scopeType.${scope}`),
    allowed:
      scope === 'global'
        ? props.options!.can_assign_global
        : scope === 'negotiation_space'
          ? // Aucun espace de négociation n'existe tant que le module est en
            // maintenance : la portée est admise par le modèle, pas offrable.
            props.options!.negotiation_spaces.length > 0
          : true,
  }))
})

/** Le rôle change : la portée retenue peut ne plus être admise. */
watch(roleCode, () => {
  const admitted = scopeOptions.value.filter((option) => option.allowed)
  scopeType.value = admitted.length === 1 ? admitted[0]!.value : ''
  scopeId.value = ''
})

const targets = computed<ScopeChoice[]>(() => {
  if (!props.options) return []
  if (scopeType.value === 'event') return props.options.events
  if (scopeType.value === 'organization') return props.options.organizations
  if (scopeType.value === 'negotiation_space') return props.options.negotiation_spaces
  return []
})

const targetOptions = computed<SelectOption[]>(() =>
  targets.value.map((choice) => ({
    value: choice.scope_id,
    label: choice.label,
    description: choice.hint ?? undefined,
    disabled: choice.disabled,
  })),
)

/** Une portée hors du périmètre d'administration : la dire, plutôt que la taire. */
const restrictedTargets = computed(() => targets.value.filter((choice) => choice.disabled).length)

const needsTarget = computed(() => scopeType.value !== '' && scopeType.value !== 'global')

/** `ux_role_assignments_active` — le doublon se voit AVANT l'envoi. */
const conflict = computed<RoleAssignmentView | null>(() => {
  if (!roleCode.value || scopeType.value === '') return null
  return findConflictingAssignment(
    props.assignments,
    roleCode.value,
    scopeType.value,
    needsTarget.value ? scopeId.value || null : null,
  )
})

/**
 * CE QUE CETTE ATTRIBUTION CHANGE — la phrase que le panneau doit à l'opérateur.
 *
 * Vide ne veut pas dire « erreur » : cela veut dire que la personne détient déjà
 * tout ce que ce rôle apporte sur cette portée, le plus souvent parce qu'elle a
 * une attribution GLOBALE. L'attribution reste possible et parfaitement
 * légitime — un rôle documente autant qu'il autorise —, mais il faut le savoir.
 */
const gained = computed(() => {
  const role = selectedRole.value
  if (!role || scopeType.value === '') return []
  return permissionsGainedBy(
    props.targetPermissions,
    role.permissions.map((permission) => permission.code),
    scopeType.value,
    needsTarget.value ? scopeId.value || null : null,
  )
})

const gainedLabels = computed(() => {
  const role = selectedRole.value
  if (!role) return []
  return gained.value.flatMap((code) => {
    const permission = role.permissions.find((entry) => entry.code === code)
    return permission ? [tr(permission.label)] : []
  })
})

const isValid = computed(
  () =>
    roleCode.value !== '' &&
    scopeType.value !== '' &&
    (!needsTarget.value || scopeId.value !== '') &&
    conflict.value === null &&
    (validUntil.value === '' || validFrom.value === '' || validUntil.value > validFrom.value),
)

function submit(): void {
  if (!isValid.value || scopeType.value === '') return

  emit('submit', {
    role_code: roleCode.value,
    scope_type: scopeType.value,
    scope_id: needsTarget.value ? scopeId.value : null,
    // Une date nue vaut minuit dans le fuseau du navigateur : l'API la
    // normalisera. Vide = maintenant, et c'est le cas courant.
    valid_from: validFrom.value ? `${validFrom.value}T00:00:00Z` : null,
    valid_until: validUntil.value ? `${validUntil.value}T23:59:59Z` : null,
    note: note.value.trim() || null,
  })
}
</script>

<template>
  <UiDrawer
    :open="open"
    width="34rem"
    :title="t('admin.user.roles.panel.title')"
    :description="t('admin.user.roles.panel.description', { name: personName })"
    @update:open="emit('update:open', $event)"
  >
    <form class="space-y-6" @submit.prevent="submit">
      <!-- 1. QUEL RÔLE -->
      <section class="space-y-3">
        <h3 class="font-display text-sm tracking-wide text-text-subtle uppercase">
          {{ t('admin.user.roles.panel.step.role') }}
        </h3>

        <UiFormField
          :label="t('admin.user.roles.panel.field.role')"
          required
          :hint="selectedRole?.description ? tr(selectedRole.description) : undefined"
        >
          <UiSelect
            v-model="roleCode"
            :options="roleOptions"
            :placeholder="t('admin.user.roles.panel.field.rolePlaceholder')"
            required
          />
        </UiFormField>

        <!-- CE QUE LE RÔLE APPORTE, avant même de choisir la portée. « Ça donne
             quoi ? » est la première question, et elle ne devrait jamais obliger
             à quitter le panneau. -->
        <div v-if="selectedRole" class="rounded-md border border-border bg-surface-sunken p-3 text-sm">
          <p class="text-text-muted">
            {{ t('admin.user.roles.panel.grants', { count: selectedRole.permissions.length }) }}
          </p>
          <ul class="mt-2 flex flex-wrap gap-1.5">
            <li
              v-for="permission in showAllPermissions
                ? selectedRole.permissions
                : selectedRole.permissions.slice(0, 4)"
              :key="permission.code"
            >
              <UiBadge size="sm" :label="tr(permission.label)" />
            </li>
          </ul>
          <button
            v-if="selectedRole.permissions.length > 4"
            type="button"
            class="mt-2 cursor-pointer text-sm text-accent underline underline-offset-2"
            @click="showAllPermissions = !showAllPermissions"
          >
            {{
              showAllPermissions
                ? t('admin.user.roles.panel.showLess')
                : t('admin.user.roles.panel.showAll', { count: selectedRole.permissions.length - 4 })
            }}
          </button>
        </div>
      </section>

      <!-- 2. SUR QUELLE PORTÉE — le cœur du panneau -->
      <section v-if="selectedRole" class="space-y-3">
        <h3 class="font-display text-sm tracking-wide text-text-subtle uppercase">
          {{ t('admin.user.roles.panel.step.scope') }}
        </h3>

        <fieldset class="space-y-2">
          <legend class="sr-only">{{ t('admin.user.roles.panel.field.scope') }}</legend>

          <label
            v-for="option in scopeOptions"
            :key="option.value"
            class="flex min-h-(--target-min) items-start gap-3 rounded-md border p-3 transition-colors"
            :class="[
              option.allowed ? 'cursor-pointer hover:bg-surface-hover' : 'cursor-not-allowed opacity-60',
              scopeType === option.value ? 'border-accent bg-accent-surface' : 'border-border',
            ]"
          >
            <input
              v-model="scopeType"
              type="radio"
              name="scope-type"
              class="mt-1 accent-[var(--color-accent)]"
              :value="option.value"
              :disabled="!option.allowed"
            >
            <span class="min-w-0">
              <span class="block font-medium">{{ option.label }}</span>
              <span class="block text-sm text-text-muted">
                {{ t(`admin.user.roles.scopeHelp.${option.value}`) }}
              </span>
              <span v-if="!option.allowed" class="mt-1 block text-sm text-warning">
                {{
                  option.value === 'negotiation_space'
                    ? t('admin.user.roles.panel.scopeUnavailable')
                    : t('admin.user.roles.panel.scopeForbidden')
                }}
              </span>
            </span>
          </label>
        </fieldset>

        <UiFormField
          v-if="needsTarget"
          :label="t(`admin.user.roles.panel.field.target.${scopeType}`)"
          required
        >
          <UiSelect
            v-model="scopeId"
            :options="targetOptions"
            :placeholder="t('admin.user.roles.panel.field.targetPlaceholder')"
            required
          />
        </UiFormField>

        <!-- LE PÉRIMÈTRE, DIT EN CLAIR. Des cibles grisées sans explication se
             lisent comme un bogue. -->
        <UiAlert
          v-if="needsTarget && restrictedTargets > 0"
          intent="info"
          compact
          :message="t('admin.user.roles.panel.restricted', { count: restrictedTargets })"
        />
      </section>

      <!-- 3. DURÉE ET MOTIF -->
      <section v-if="selectedRole && scopeType" class="space-y-3">
        <h3 class="font-display text-sm tracking-wide text-text-subtle uppercase">
          {{ t('admin.user.roles.panel.step.terms') }}
        </h3>

        <div class="grid gap-3 sm:grid-cols-2">
          <UiDatePicker
            v-model="validFrom"
            :label="t('admin.user.roles.panel.field.validFrom')"
            :hint="t('admin.user.roles.panel.field.validFromHint')"
          />
          <UiDatePicker
            v-model="validUntil"
            :label="t('admin.user.roles.panel.field.validUntil')"
            :hint="t('admin.user.roles.panel.field.validUntilHint')"
            :min="validFrom || undefined"
            :error="
              validUntil && validFrom && validUntil <= validFrom
                ? t('admin.user.roles.panel.field.validUntilError')
                : undefined
            "
          />
        </div>

        <UiFormField
          :label="t('admin.user.roles.panel.field.note')"
          :hint="t('admin.user.roles.panel.field.noteHint')"
        >
          <UiTextarea v-model="note" :rows="2" :maxlength="500" auto-grow />
        </UiFormField>
      </section>

      <!-- 4. CE QUE ÇA CHANGE -->
      <section
        v-if="selectedRole && scopeType && !conflict"
        class="rounded-md border border-border bg-surface-sunken p-3"
      >
        <h3 class="font-display text-sm tracking-wide text-text-subtle uppercase">
          {{ t('admin.user.roles.panel.step.effect') }}
        </h3>

        <p v-if="gainedLabels.length === 0" class="mt-2 text-sm text-text-muted">
          {{ t('admin.user.roles.panel.effect.none') }}
        </p>
        <template v-else>
          <p class="mt-2 text-sm">
            {{ t('admin.user.roles.panel.effect.gained', { count: gainedLabels.length }) }}
          </p>
          <ul class="mt-2 flex flex-wrap gap-1.5">
            <li v-for="label in gainedLabels" :key="label">
              <UiBadge size="sm" intent="success" :label="label" />
            </li>
          </ul>
        </template>
      </section>

      <!-- LE DOUBLON, ANNONCÉ AVANT L'ENVOI — `ux_role_assignments_active`. -->
      <UiAlert
        v-if="conflict"
        intent="warning"
        :title="t('admin.user.roles.panel.conflict.title')"
        :message="
          t('admin.user.roles.panel.conflict.message', {
            granted: new Date(conflict.granted_at).toLocaleDateString('fr-CA'),
          })
        "
      />

      <UiAlert v-if="error" intent="danger" :message="error" />

      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" type="button" @click="emit('update:open', false)">
          {{ t('common.actions.cancel') }}
        </UiButton>
        <UiButton type="submit" :disabled="!isValid" :loading="submitting">
          {{ t('admin.user.roles.panel.submit') }}
        </UiButton>
      </div>
    </form>
  </UiDrawer>
</template>
