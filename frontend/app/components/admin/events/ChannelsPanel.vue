<script setup lang="ts">
import type { EditionChannel, EditionChannelPayload, EditionDetail } from '~/types/admin-events'
import type { BroadcastProvider } from '~/types/event/venue'
import type { SelectOption } from '~/types/ui'

/**
 * ONGLET « CANAL DE DIFFUSION ».
 *
 * ── RÈGLE MÉTIER N° 4 : UN SEUL DIRECT À LA FOIS ────────────────────────────
 *
 * Tous événements confondus. Une seule équipe technique, un seul flux, un seul
 * lecteur en page d'accueil. Le canal est donc modélisé comme une RESSOURCE
 * RÉSERVABLE, au même titre qu'une salle : deux séances diffusées qui l'occupent sur
 * des créneaux qui se recouvrent remontent au planificateur en gravité bloquante, et
 * `publication_readiness()` retient la publication du programme tant que le conflit
 * n'est pas arbitré.
 *
 * ── LE CANAL PAR DÉFAUT N'EST PAS UN CONFORT ────────────────────────────────
 *
 * `tg_sessions_derive_fields()` le pose d'office sur toute séance marquée comme
 * diffusée : sans lui, une séance « diffusée » n'occupe AUCUN canal et échappe
 * entièrement à la règle. C'est l'oubli qui est dangereux, pas la saisie — d'où
 * l'avertissement en tête d'onglet quand aucun canal par défaut n'existe.
 * `ux_broadcast_channels_default` n'en autorise qu'un par édition.
 *
 * ── LES CANAUX DE LA PLATEFORME SE VOIENT, NE SE MODIFIENT PAS ──────────────
 *
 * `event_id IS NULL` désigne un canal général, hors édition — celui des webinaires
 * courants. Il apparaît ici parce qu'une séance de cette édition peut l'emprunter,
 * et qu'il faut savoir qu'il existe ; il se gère ailleurs.
 */

interface Props {
  detail: EditionDetail
  canManage: boolean
  busy?: boolean
  error?: string | null
  /** Le canal a été désactivé au lieu d'être supprimé : c'est un succès, pas un refus. */
  notice?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  save: [payload: EditionChannelPayload]
  remove: [channelId: string]
}>()

const { t } = useI18n()
const { tr } = useI18nText()

const PROVIDERS: BroadcastProvider[] = [
  'youtube',
  'vimeo',
  'facebook',
  'linkedin',
  'dailymotion',
  'custom',
]

const providerOptions = computed<SelectOption[]>(() =>
  PROVIDERS.map((provider) => ({
    value: provider,
    label: t('admin.event.tabs.channelsTab.provider.' + provider),
  })),
)

const localeOptions = computed<SelectOption[]>(() => [
  { value: '', label: t('admin.event.tabs.channelsTab.form.localeNone') },
  { value: 'fr', label: t('admin.event.form.localeTab.fr') },
  { value: 'en', label: t('admin.event.form.localeTab.en') },
])

/** Les canaux de cette édition — ceux qu'on peut gérer d'ici. */
const ownChannels = computed(() => props.detail.channels.filter((c) => c.event_id !== null))
const platformChannels = computed(() => props.detail.channels.filter((c) => c.event_id === null))

/**
 * Aucun canal par défaut ACTIF sur l'édition, alors qu'elle en porte au moins un.
 * On n'avertit pas une édition sans canal du tout : l'état vide le dit déjà.
 */
const missingDefault = computed(
  () => ownChannels.value.length > 0 && !ownChannels.value.some((c) => c.is_default && c.is_active),
)

const draft = ref<EditionChannelPayload | null>(null)

function openCreate(): void {
  draft.value = {
    id: null,
    event_id: props.detail.edition.id,
    code: '',
    name: { fr: '' },
    provider: 'youtube',
    channel_ref: null,
    locale: 'fr',
    // Le premier canal d'une édition est le canal par défaut : c'est le cas normal,
    // et l'inverse laisserait une édition avec un canal que rien n'utilise.
    is_default: ownChannels.value.length === 0,
    is_active: true,
  }
}

function openEdit(channel: EditionChannel): void {
  draft.value = {
    id: channel.id,
    event_id: props.detail.edition.id,
    code: channel.code,
    name: channel.name,
    provider: channel.provider,
    channel_ref: channel.channel_ref,
    locale: channel.locale,
    is_default: channel.is_default,
    is_active: channel.is_active,
  }
}

const doomed = ref<EditionChannel | null>(null)

watch(
  () => props.detail.channels,
  () => {
    if (!props.error) draft.value = null
  },
)
</script>

<template>
  <section>
    <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
      <div class="min-w-0">
        <h2 class="font-display text-xl font-semibold">
          {{ t('admin.event.tabs.channelsTab.title') }}
        </h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.event.tabs.channelsTab.intro') }}
        </p>
      </div>

      <UiButton v-if="props.canManage" icon="plus" :disabled="props.busy" @click="openCreate">
        {{ t('admin.event.tabs.channelsTab.add') }}
      </UiButton>
    </header>

    <UiAlert v-if="props.notice" class="mt-4" intent="success" live :message="props.notice" />

    <!-- L'OUBLI EST LE DANGER, PAS LA SAISIE. Jaune : cela demande attention. -->
    <UiAlert
      v-if="missingDefault"
      class="mt-4"
      intent="warning"
      :title="t('admin.event.tabs.channelsTab.noDefault.title')"
      :message="t('admin.event.tabs.channelsTab.noDefault.description')"
    />

    <UiEmptyState
      v-if="props.detail.channels.length === 0"
      class="mt-5"
      icon="broadcast"
      :title="t('admin.event.tabs.channelsTab.empty.title')"
      :description="t('admin.event.tabs.channelsTab.empty.description')"
    />

    <ul v-else class="mt-5 grid gap-4 lg:grid-cols-2">
      <li
        v-for="channel in [...ownChannels, ...platformChannels]"
        :key="channel.id"
        class="rounded-lg border border-border bg-surface-raised p-4"
        :class="channel.is_active ? '' : 'opacity-70'"
      >
        <div class="flex items-start gap-3">
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <h3 class="min-w-0 truncate font-semibold text-text">{{ tr(channel.name) }}</h3>
              <UiBadge
                intent="neutral"
                size="sm"
                :label="t('admin.event.tabs.channelsTab.provider.' + channel.provider)"
              />
              <!-- Vert : confirmé, c'est le canal que la base posera d'office. -->
              <UiBadge
                v-if="channel.is_default"
                intent="success"
                size="sm"
                :label="t('admin.event.tabs.channelsTab.defaultBadge')"
              />
              <UiBadge
                v-if="!channel.is_active"
                intent="neutral"
                size="sm"
                :label="t('admin.event.tabs.channelsTab.inactiveBadge')"
              />
              <UiBadge
                v-if="channel.event_id === null"
                intent="info"
                size="sm"
                :label="t('admin.event.tabs.channelsTab.platformBadge')"
              />
            </div>

            <p class="mt-1 flex flex-wrap items-center gap-x-2 text-sm text-text-muted">
              <code class="font-mono text-xs">{{ channel.code }}</code>
              <span v-if="channel.channel_ref">{{ channel.channel_ref }}</span>
            </p>

            <p class="mt-1 text-sm text-text-muted">
              {{ t('admin.event.tabs.channelsTab.sessions', channel.session_count) }}
            </p>

            <p v-if="channel.event_id === null" class="mt-1 text-xs text-text-subtle">
              {{ t('admin.event.tabs.channelsTab.platformHint') }}
            </p>
          </div>

          <div v-if="props.canManage && channel.event_id !== null" class="flex shrink-0 flex-col gap-1">
            <UiButton
              variant="ghost"
              size="sm"
              icon="edit"
              icon-only
              :label="t('common.actions.edit')"
              @click="openEdit(channel)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="trash"
              icon-only
              :label="t('admin.event.tabs.channelsTab.remove.action')"
              @click="doomed = channel"
            />
          </div>
        </div>
      </li>
    </ul>

    <UiDrawer
      :open="draft !== null"
      :title="
        draft?.id
          ? t('admin.event.tabs.channelsTab.form.editTitle')
          : t('admin.event.tabs.channelsTab.form.createTitle')
      "
      @update:open="(next: boolean) => { if (!next) draft = null }"
    >
      <form v-if="draft" class="space-y-5" novalidate @submit.prevent="emit('save', { ...draft })">
        <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

        <AdminEventsI18nField
          :model-value="draft.name"
          :label="t('admin.event.tabs.channelsTab.form.nameField')"
          required
          @update:model-value="(next) => (draft!.name = next ?? { fr: '' })"
        />

        <div class="grid gap-4 sm:grid-cols-2">
          <UiInput
            :model-value="draft.code"
            :label="t('admin.event.tabs.channelsTab.form.codeField')"
            required
            :maxlength="40"
            @update:model-value="(next: string) => (draft!.code = next)"
          />
          <UiSelect
            :model-value="draft.provider"
            :label="t('admin.event.tabs.channelsTab.form.providerField')"
            :options="providerOptions"
            hide-optional
            @update:model-value="(next: string) => (draft!.provider = next as BroadcastProvider)"
          />
        </div>

        <UiInput
          :model-value="draft.channel_ref ?? ''"
          :label="t('admin.event.tabs.channelsTab.form.channelRefField')"
          :hint="t('admin.event.tabs.channelsTab.form.channelRefHint')"
          @update:model-value="(next: string) => (draft!.channel_ref = next || null)"
        />

        <UiSelect
          :model-value="draft.locale ?? ''"
          :label="t('admin.event.tabs.channelsTab.form.localeField')"
          :options="localeOptions"
          hide-optional
          @update:model-value="(next: string) => (draft!.locale = next || null)"
        />

        <UiSwitch
          :model-value="draft.is_default"
          :label="t('admin.event.tabs.channelsTab.form.isDefault')"
          :hint="t('admin.event.tabs.channelsTab.form.isDefaultHint')"
          @update:model-value="(next: boolean) => (draft!.is_default = next)"
        />

        <UiSwitch
          :model-value="draft.is_active"
          :label="t('admin.event.tabs.channelsTab.form.isActive')"
          @update:model-value="(next: boolean) => (draft!.is_active = next)"
        />

        <div class="flex flex-wrap gap-3 border-t border-border pt-4">
          <UiButton type="submit" :loading="props.busy">{{ t('common.actions.save') }}</UiButton>
          <UiButton variant="ghost" :disabled="props.busy" @click="draft = null">
            {{ t('admin.event.tabs.confirm.cancel') }}
          </UiButton>
        </div>
      </form>
    </UiDrawer>

    <UiModal
      :open="doomed !== null"
      :title="t('admin.event.tabs.channelsTab.remove.title')"
      :description="t('admin.event.tabs.channelsTab.remove.description')"
      @update:open="(next: boolean) => { if (!next) doomed = null }"
    >
      <UiAlert
        v-if="doomed && doomed.session_count > 0"
        intent="info"
        :message="t('admin.event.tabs.channelsTab.remove.willDeactivate', doomed.session_count)"
      />

      <template #footer>
        <UiButton variant="ghost" @click="doomed = null">
          {{ t('admin.event.tabs.confirm.cancel') }}
        </UiButton>
        <UiButton
          variant="danger"
          :loading="props.busy"
          @click="() => { if (doomed) emit('remove', doomed.id); doomed = null }"
        >
          {{ t('admin.event.tabs.channelsTab.remove.action') }}
        </UiButton>
      </template>
    </UiModal>
  </section>
</template>
