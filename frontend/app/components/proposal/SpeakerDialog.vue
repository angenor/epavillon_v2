<script setup lang="ts">
import type { DraftSpeaker, DraftUpload, PersonLookup } from '~/types/proposal-form'
import { TEXT_LIMITS } from '~/types/proposal-form'
import type { SpeakerRole } from '~/types/programme/proposal'
import type { SelectOption } from '~/types/ui'

/**
 * SAISIE D'UN INTERVENANT — civilité, prénom, nom, adresse, fonction,
 * organisation, rôle, photo, notice.
 *
 * L'INTERVENANT EST UNE PERSONNE, pas une ligne d'activité. C'est la correction
 * de fond de la v1, dont `activity_speakers` redupliquait nom, prénom, adresse
 * et photo à chaque activité : le même expert existait en autant d'exemplaires
 * que de participations, sans moyen de consolider son historique.
 *
 * D'OÙ LA RECHERCHE PAR ADRESSE, qui est le cœur de cet écran. Dès que
 * l'adresse est complète, on demande à la plateforme si elle connaît quelqu'un
 * qui la porte (`people.primary_email`, clé de rapprochement du modèle). Trois
 * situations, et elles ne se ressemblent pas :
 *
 *  1. PERSONNE INCONNUE — on saisit tout, et l'API créera la personne à
 *     l'enregistrement du dossier.
 *  2. PERSONNE CONNUE, SANS COMPTE — elle a été intervenante ailleurs. On peut
 *     retenir son profil ET corriger ce qui a changé : personne d'autre ne
 *     l'entretient.
 *  3. PERSONNE CONNUE, AVEC UN COMPTE — son identité lui appartient. Le
 *     déposant la retient telle quelle et ne la modifie pas : corriger « Awa
 *     Sow Fall » en « A. Sowfall » pour son propre confort réécrirait une fiche
 *     visible de toutes ses autres participations. Seul le titulaire, ou un
 *     administrateur de la plateforme, y touche.
 *
 * CE QUI RESTE MODIFIABLE DANS TOUS LES CAS : le RÔLE dans l'activité, la
 * FONCTION et l'ORGANISATION. Ces deux dernières ne sont pas des attributs de la
 * personne mais des INSTANTANÉS de cette activité — `job_title_snapshot`,
 * `organization_snapshot` —, et le modèle le dit sans ambiguïté : « une personne
 * change d'employeur, l'archive de la COP28 ne doit pas être réécrite pour
 * autant ». Elles sont pré-remplies depuis le profil, jamais figées par lui.
 *
 * LA PHOTO SE RATTACHE À LA PERSONNE (rôle `avatar` de `identity.people` dans
 * `media.attachable_roles`), et elle est MONTRÉE : une photo choisie sans être
 * affichée est une photo qu'on ne peut pas vérifier — mauvais fichier, portrait
 * de travers, image d'une autre personne.
 */

interface Props {
  open: boolean
  /** Intervenant en cours de modification ; `null` pour un ajout. */
  speaker: DraftSpeaker | null
  /** Adresses déjà retenues dans ce dossier — une personne ne figure qu'une fois. */
  takenEmails: string[]
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  save: [speaker: DraftSpeaker]
}>()

const { t } = useI18n()
const api = useApi()

const CIVILITIES = ['mme', 'm', 'dr', 'pr', 'other'] as const
const ROLES: SpeakerRole[] = ['speaker', 'moderator', 'panelist', 'keynote', 'facilitator', 'interpreter']

const civilityOptions = computed<SelectOption[]>(() =>
  CIVILITIES.map((code) => ({
    value: code,
    label: t(`proposal.form.step-speakers.civilities.${code}`),
  })),
)

const roleOptions = computed<SelectOption[]>(() =>
  ROLES.map((role) => ({
    value: role,
    label: t(`proposal.form.step-speakers.roles.${role}.label`),
    description: t(`proposal.form.step-speakers.roles.${role}.hint`),
  })),
)

/** Copie de travail : on ne modifie l'intervenant qu'à la validation. */
const form = ref<DraftSpeaker>(emptyDraftSpeaker(0))
const touched = ref(false)

watch(
  () => [props.open, props.speaker] as const,
  ([open, speaker]) => {
    if (!open) return
    form.value = speaker ? { ...speaker } : emptyDraftSpeaker(props.takenEmails.length)
    touched.value = false
    lookup.value = null
    lookupState.value = 'idle'
  },
  { immediate: true },
)

// ---------------------------------------------------------------------------
// Recherche de la personne par son adresse
// ---------------------------------------------------------------------------

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/

const lookup = ref<PersonLookup | null>(null)
const lookupState = ref<'idle' | 'searching' | 'found' | 'unknown'>('idle')

/** Seule la dernière recherche écrit : même garde qu'aux organisations. */
let sequence = 0
let timer: ReturnType<typeof setTimeout> | null = null

/**
 * L'IDENTITÉ EST-ELLE VERROUILLÉE ? Seulement quand le profil retenu porte un
 * COMPTE. Une personne connue sans compte reste corrigible : c'est le dossier
 * qui l'entretient, et personne d'autre ne le fera.
 */
const isIdentityLocked = computed(() => form.value.person_id !== null && form.value.has_account)

async function runLookup(email: string): Promise<void> {
  const needle = email.trim().toLowerCase()

  // Changer d'adresse détache le profil : on ne garde pas l'identité de
  // quelqu'un sous l'adresse d'un autre.
  if (form.value.person_id !== null && needle !== form.value.email.trim().toLowerCase()) {
    detachProfile()
  }

  if (!EMAIL_PATTERN.test(needle)) {
    lookup.value = null
    lookupState.value = 'idle'
    return
  }

  const current = ++sequence
  lookupState.value = 'searching'
  const found = await api.proposals.lookupSpeaker(needle)
  if (current !== sequence) return

  lookup.value = found
  lookupState.value = found ? 'found' : 'unknown'
}

function onEmailInput(value: string): void {
  form.value.email = value
  if (timer) clearTimeout(timer)
  timer = setTimeout(() => void runLookup(value), 400)
}

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})

/**
 * Retenir le profil trouvé. Tout est repris — identité ET amorce des
 * instantanés d'activité —, et l'identité se verrouille si la personne a un
 * compte.
 */
function selectProfile(): void {
  const person = lookup.value
  if (!person) return

  form.value = {
    ...form.value,
    person_id: person.person_id,
    has_account: person.has_account,
    civility: person.civility,
    first_name: person.first_name,
    last_name: person.last_name,
    email: person.email,
    bio: person.bio ?? form.value.bio,
    // Amorces MODIFIABLES : ce sont les instantanés de cette activité.
    job_title: form.value.job_title || (person.job_title ?? ''),
    organization_name: form.value.organization_name || (person.organization_name ?? ''),
    organization_id: person.organization_id,
    // La photo appartient au profil : le déposant n'en téléverse pas une seconde.
    photo: null,
  }
}

/** Repartir d'une saisie libre — l'adresse est conservée. */
function detachProfile(): void {
  form.value.person_id = null
  form.value.has_account = false
}

// ---------------------------------------------------------------------------
// Contrôles du dialogue
// ---------------------------------------------------------------------------

const emailError = computed(() => {
  if (!touched.value) return undefined
  const email = form.value.email.trim().toLowerCase()
  if (email.length === 0) return t('validation.required')
  if (!EMAIL_PATTERN.test(email)) return t('validation.email')
  const taken = props.takenEmails.map((entry) => entry.toLowerCase())
  const current = props.speaker?.email.trim().toLowerCase()
  if (email !== current && taken.includes(email)) {
    return t('proposal.form.step-speakers.errors.emailDuplicate', { email })
  }
  return undefined
})

function requiredError(value: string | null): string | undefined {
  if (!touched.value) return undefined
  return (value ?? '').trim().length === 0 ? t('validation.required') : undefined
}

/**
 * Les quatre exigences de l'étape, arrêtées par le commanditaire le 17/08 :
 * civilité, prénom, nom, fonction et organisation. Aucune n'est `NOT NULL` en
 * base — ce sont des exigences de DOSSIER : le programme annonce « Mme Awa Sow
 * Fall, directrice exécutive, ROAC », et une ligne amputée s'y voit.
 */
const canSave = computed(
  () =>
    !emailError.value &&
    form.value.first_name.trim().length > 0 &&
    form.value.last_name.trim().length > 0 &&
    Boolean(form.value.civility) &&
    form.value.job_title.trim().length > 0 &&
    form.value.organization_name.trim().length > 0,
)

function save(): void {
  touched.value = true
  if (!canSave.value) return
  emit('save', {
    ...form.value,
    email: form.value.email.trim(),
    first_name: form.value.first_name.trim(),
    last_name: form.value.last_name.trim(),
    job_title: form.value.job_title.trim(),
    organization_name: form.value.organization_name.trim(),
  })
}

// ---------------------------------------------------------------------------
// Photo
// ---------------------------------------------------------------------------

/**
 * La photo est RETENUE, pas téléversée : le dépôt sur le stockage appartient au
 * prompt B6. On garde de quoi l'envoyer, de quoi refuser tout de suite ce que
 * `media.attachable_roles` refuserait — rôle `avatar`, images seules, cinq
 * mégaoctets — et **de quoi la montrer**, par une adresse locale à cet onglet.
 */
const AVATAR_MAX_BYTES = 5_242_880
const photoError = ref<string | null>(null)

function onPhotoChange(event: Event): void {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  photoError.value = null
  if (!file) return

  if (!file.type.startsWith('image/')) {
    photoError.value = t('proposal.form.step-speakers.photo.typeError')
    input.value = ''
    return
  }
  if (file.size > AVATAR_MAX_BYTES) {
    photoError.value = t('proposal.form.step-speakers.photo.sizeError', {
      size: formatByteSize(AVATAR_MAX_BYTES, 'fr'),
    })
    input.value = ''
    return
  }

  releasePreview()
  const upload: DraftUpload = {
    file_name: file.name,
    mime_type: file.type,
    byte_size: file.size,
    asset_id: null,
    preview_url: URL.createObjectURL(file),
  }
  form.value.photo = upload
  input.value = ''
}

/** L'adresse d'objet est libérée : elle retient le fichier en mémoire. */
function releasePreview(): void {
  const url = form.value.photo?.preview_url
  if (url) URL.revokeObjectURL(url)
}

function removePhoto(): void {
  releasePreview()
  form.value.photo = null
}
</script>

<template>
  <UiModal
    :open="props.open"
    size="lg"
    :title="props.speaker
      ? t('proposal.form.step-speakers.dialog.editTitle')
      : t('proposal.form.step-speakers.dialog.addTitle')"
    :description="t('proposal.form.step-speakers.dialog.description')"
    @update:open="emit('update:open', $event)"
  >
    <form class="grid gap-5" @submit.prevent="save">
      <!-- L'ADRESSE D'ABORD : c'est elle qui interroge l'annuaire. -->
      <UiInput
        :model-value="form.email"
        type="email"
        autocomplete="off"
        :label="t('proposal.form.step-speakers.fields.email.label')"
        :hint="t('proposal.form.step-speakers.fields.email.hint')"
        :error="emailError"
        :readonly="isIdentityLocked"
        required
        @update:model-value="onEmailInput"
        @blur="touched = true"
      >
        <template v-if="lookupState === 'searching'" #suffix>
          <UiSpinner size="1rem" />
        </template>
      </UiInput>

      <!-- PROFIL TROUVÉ, PAS ENCORE RETENU. -->
      <UiAlert
        v-if="lookup && form.person_id === null"
        intent="info"
        :title="t('proposal.form.step-speakers.dialog.foundTitle')"
      >
        <p>
          <span class="font-bold text-text">
            {{ lookup.civility ? t(`proposal.form.step-speakers.civilities.${lookup.civility}`) : '' }}
            {{ lookup.first_name }} {{ lookup.last_name }}
          </span>
          <span v-if="lookup.job_title || lookup.organization_name" class="block">
            {{ [lookup.job_title, lookup.organization_name].filter(Boolean).join(' · ') }}
          </span>
          <span class="mt-1 block">
            {{
              lookup.has_account
                ? t('proposal.form.step-speakers.dialog.foundWithAccount')
                : t('proposal.form.step-speakers.dialog.foundWithoutAccount')
            }}
          </span>
        </p>
        <template #actions>
          <UiButton
            variant="primary"
            size="sm"
            :label="t('proposal.form.step-speakers.dialog.useProfile')"
            @click="selectProfile()"
          />
        </template>
      </UiAlert>

      <!-- PROFIL RETENU. Dire ce qui est verrouillé, et pourquoi. -->
      <UiAlert
        v-else-if="form.person_id !== null"
        :intent="isIdentityLocked ? 'success' : 'info'"
        :title="isIdentityLocked
          ? t('proposal.form.step-speakers.dialog.lockedTitle')
          : t('proposal.form.step-speakers.dialog.linkedTitle')"
        :message="isIdentityLocked
          ? t('proposal.form.step-speakers.dialog.lockedDescription')
          : t('proposal.form.step-speakers.dialog.linkedDescription')"
      >
        <template #actions>
          <UiButton
            variant="ghost"
            size="sm"
            :label="t('proposal.form.step-speakers.dialog.detach')"
            @click="detachProfile()"
          />
        </template>
      </UiAlert>

      <!-- ADRESSE INCONNUE : on le dit, sans en faire une faute. -->
      <p
        v-else-if="lookupState === 'unknown'"
        class="flex items-start gap-2 rounded-md bg-surface-sunken px-3 py-2 text-sm text-text-secondary"
      >
        <UiIcon name="info" size="1.05rem" class="mt-0.5 shrink-0 text-text-muted" />
        {{ t('proposal.form.step-speakers.dialog.unknownPerson') }}
      </p>

      <div class="grid gap-4 sm:grid-cols-[9rem_1fr_1fr]">
        <UiSelect
          v-model="form.civility"
          :options="civilityOptions"
          :label="t('proposal.form.step-speakers.fields.civility.label')"
          :placeholder="t('proposal.form.step-speakers.fields.civility.placeholder')"
          :error="requiredError(form.civility)"
          :disabled="isIdentityLocked"
          required
        />
        <UiInput
          v-model="form.first_name"
          :label="t('proposal.form.step-speakers.fields.first_name.label')"
          :error="requiredError(form.first_name)"
          :readonly="isIdentityLocked"
          required
          @blur="touched = true"
        />
        <UiInput
          v-model="form.last_name"
          :label="t('proposal.form.step-speakers.fields.last_name.label')"
          :error="requiredError(form.last_name)"
          :readonly="isIdentityLocked"
          required
          @blur="touched = true"
        />
      </div>

      <!-- FONCTION ET ORGANISATION : instantanés de CETTE activité, donc
           modifiables même sur un profil verrouillé. -->
      <div class="grid gap-4 sm:grid-cols-2">
        <UiInput
          v-model="form.job_title"
          :label="t('proposal.form.step-speakers.fields.job_title.label')"
          :hint="t('proposal.form.step-speakers.fields.job_title.hint')"
          :error="requiredError(form.job_title)"
          required
          @blur="touched = true"
        />
        <UiInput
          v-model="form.organization_name"
          :label="t('proposal.form.step-speakers.fields.organization.label')"
          :hint="t('proposal.form.step-speakers.fields.organization.hint')"
          :error="requiredError(form.organization_name)"
          required
          @blur="touched = true"
        />
      </div>

      <UiRadio
        :model-value="form.role"
        :options="roleOptions"
        :label="t('proposal.form.step-speakers.fields.role.label')"
        :hint="t('proposal.form.step-speakers.fields.role.hint')"
        required
        @update:model-value="form.role = $event as SpeakerRole"
      />

      <UiTextarea
        v-model="form.bio"
        :label="t('proposal.form.step-speakers.fields.bio.label')"
        :hint="isIdentityLocked
          ? t('proposal.form.step-speakers.fields.bio.locked')
          : t('proposal.form.step-speakers.fields.bio.hint')"
        :maxlength="TEXT_LIMITS.speaker_bio"
        :readonly="isIdentityLocked"
        :rows="4"
        auto-grow
      />

      <!-- PHOTO, AVEC SON APERÇU. Rattachée à la personne, jamais au dossier. -->
      <div>
        <p class="mb-1.5 text-sm font-bold text-text">
          {{ t('proposal.form.step-speakers.photo.label') }}
          <span class="ml-1.5 text-xs font-normal text-text-subtle">{{ t('form.optional') }}</span>
        </p>

        <p v-if="isIdentityLocked" class="max-w-(--measure) text-sm text-text-muted">
          {{ t('proposal.form.step-speakers.photo.locked') }}
        </p>

        <template v-else>
          <p class="mb-2 max-w-(--measure) text-sm text-text-muted">
            {{ t('proposal.form.step-speakers.photo.hint') }}
          </p>

          <div v-if="form.photo" class="flex flex-wrap items-center gap-4">
            <!-- L'aperçu : carré, recadré, bordé — la même vignette que celle
                 qui accompagnera le nom sur la page de l'activité. -->
            <img
              v-if="form.photo.preview_url"
              :src="form.photo.preview_url"
              :alt="t('proposal.form.step-speakers.photo.previewAlt', {
                speaker: `${form.first_name} ${form.last_name}`.trim(),
              })"
              class="size-20 shrink-0 rounded-md border border-border object-cover"
            >
            <div class="min-w-0">
              <p class="font-mono text-sm break-all text-text-secondary">
                {{ form.photo.file_name }}
              </p>
              <p class="text-sm text-text-subtle">{{ formatByteSize(form.photo.byte_size, 'fr') }}</p>
              <UiButton
                class="mt-1"
                variant="ghost"
                size="sm"
                icon="trash"
                :label="t('proposal.form.step-speakers.photo.remove')"
                @click="removePhoto()"
              />
            </div>
          </div>

          <label
            v-else
            class="inline-flex min-h-(--target-min) cursor-pointer items-center gap-2 rounded-md border border-border px-4 py-2 text-sm font-bold text-text-secondary hover:bg-surface-hover"
          >
            <UiIcon name="upload" size="1rem" />
            {{ t('proposal.form.step-speakers.photo.choose') }}
            <input type="file" accept="image/*" class="sr-only" @change="onPhotoChange">
          </label>

          <p v-if="photoError" role="alert" class="mt-1.5 text-sm font-bold text-danger">
            {{ photoError }}
          </p>
        </template>
      </div>
    </form>

    <template #footer>
      <UiButton variant="ghost" :label="t('common.actions.cancel')" @click="emit('update:open', false)" />
      <UiButton
        variant="primary"
        :label="props.speaker ? t('common.actions.save') : t('common.actions.add')"
        @click="save()"
      />
    </template>
  </UiModal>
</template>
