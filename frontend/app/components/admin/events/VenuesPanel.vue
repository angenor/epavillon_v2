<script setup lang="ts">
import type {
  EditionDetail,
  EditionRoom,
  EditionRoomPayload,
  EditionVenue,
  EditionVenuePayload,
} from '~/types/admin-events'
import type { VenueKind } from '~/types/event/venue'
import type { SelectOption } from '~/types/ui'

/**
 * ONGLET « LIEUX ET SALLES ».
 *
 * ── POURQUOI CET ONGLET COMPTE PLUS QU'IL N'Y PARAÎT ────────────────────────
 *
 * Les salles étaient absentes de la v1, et les conflits de créneaux se réglaient à
 * l'œil : faute de salle en base, personne ne pouvait seulement les NOMMER. Ce sont
 * elles qui permettent à `detect_conflicts()` de dire « le stand est réservé deux
 * fois » plutôt qu'« il y a deux activités à 14 h ». Ce que l'on saisit ici décide
 * de ce que le planificateur saura signaler.
 *
 * ── `is_virtual` EST LA CASE DANGEREUSE ─────────────────────────────────────
 *
 * Une salle virtuelle accepte les créneaux simultanés, et la détection des doubles
 * réservations l'écarte — sans quoi elle noierait les vrais conflits. La cocher sur
 * le stand physique ferait donc TAIRE le conflit de gravité haute que l'équipe doit
 * absolument voir (règle métier n° 3 : un seul stand, deux activités ne peuvent
 * matériellement pas s'y tenir en même temps). Le formulaire l'explique, et la
 * carte de la salle rappelle laquelle des deux règles s'applique.
 *
 * ── SUPPRIMER NE CASSE RIEN, ET CE N'EST PAS UNE RAISON DE SE TAIRE ─────────
 *
 * `xmod_fk_sessions_room ON DELETE SET NULL` : retirer une salle rend ses activités
 * au panneau du planificateur, sans les supprimer. C'est silencieux en base ; la
 * confirmation le chiffre.
 */

interface Props {
  detail: EditionDetail
  canManage: boolean
  busy?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  saveVenue: [payload: EditionVenuePayload]
  removeVenue: [venueId: string]
  saveRoom: [payload: EditionRoomPayload]
  removeRoom: [roomId: string]
}>()

const { t } = useI18n()
const { tr } = useI18nText()

const KINDS: VenueKind[] = ['pavilion', 'partner', 'plenary', 'virtual', 'other']

const kindOptions = computed<SelectOption[]>(() =>
  KINDS.map((kind) => ({ value: kind, label: t('admin.event.tabs.venuesTab.kind.' + kind) })),
)

// ---------------------------------------------------------------------------
// Lieu
// ---------------------------------------------------------------------------

const venueDraft = ref<EditionVenuePayload | null>(null)

function openVenueCreate(): void {
  venueDraft.value = {
    id: null,
    event_id: props.detail.edition.id,
    name: { fr: '' },
    kind: 'pavilion',
    address: props.detail.edition.address,
    map_url: null,
  }
}

function openVenueEdit(venue: EditionVenue): void {
  venueDraft.value = {
    id: venue.id,
    event_id: props.detail.edition.id,
    name: venue.name,
    kind: venue.kind,
    address: venue.address,
    map_url: venue.map_url,
  }
}

const doomedVenue = ref<EditionVenue | null>(null)

/** Activités qu'un retrait de lieu rendrait au panneau : la somme de ses salles. */
const venueSessionCount = computed(
  () => doomedVenue.value?.rooms.reduce((sum, room) => sum + room.session_count, 0) ?? 0,
)

// ---------------------------------------------------------------------------
// Salle
// ---------------------------------------------------------------------------

const roomDraft = ref<EditionRoomPayload | null>(null)
/** Saisie de l'équipement : une ligne par élément, converti en tableau à l'envoi. */
const equipmentText = ref('')

function openRoomCreate(venue: EditionVenue): void {
  roomDraft.value = {
    id: null,
    venue_id: venue.id,
    name: { fr: '' },
    code: '',
    capacity: null,
    // Une salle d'un lieu VIRTUEL est virtuelle par nature : on préremplit selon
    // le lieu plutôt que de laisser cocher une case qui contredirait son parent.
    is_virtual: venue.kind === 'virtual',
    has_streaming: false,
    equipment: [],
    sort_order: (venue.rooms.length + 1) * 10,
  }
  equipmentText.value = ''
}

function openRoomEdit(room: EditionRoom): void {
  roomDraft.value = { ...room, equipment: [...room.equipment] }
  equipmentText.value = room.equipment.join('\n')
}

function submitRoom(): void {
  if (!roomDraft.value) return
  emit('saveRoom', {
    ...roomDraft.value,
    equipment: equipmentText.value
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean),
  })
}

const doomedRoom = ref<EditionRoom | null>(null)

/** Fermer les formulaires une fois l'écriture acceptée — pas avant. */
watch(
  () => props.detail.venues,
  () => {
    if (props.error) return
    venueDraft.value = null
    roomDraft.value = null
  },
)
</script>

<template>
  <section>
    <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
      <div class="min-w-0">
        <h2 class="font-display text-xl font-semibold">
          {{ t('admin.event.tabs.venuesTab.title') }}
        </h2>
        <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.event.tabs.venuesTab.intro') }}
        </p>
      </div>

      <UiButton v-if="props.canManage" icon="plus" :disabled="props.busy" @click="openVenueCreate">
        {{ t('admin.event.tabs.venuesTab.addVenue') }}
      </UiButton>
    </header>

    <UiEmptyState
      v-if="props.detail.venues.length === 0"
      class="mt-5"
      icon="building"
      :title="t('admin.event.tabs.venuesTab.empty.title')"
      :description="t('admin.event.tabs.venuesTab.empty.description')"
    />

    <div v-else class="mt-5 space-y-5">
      <article
        v-for="venue in props.detail.venues"
        :key="venue.id"
        class="rounded-lg border border-border bg-surface-raised"
      >
        <header class="flex flex-wrap items-start gap-x-4 gap-y-2 border-b border-border p-4">
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <h3 class="min-w-0 truncate font-semibold text-text">{{ tr(venue.name) }}</h3>
              <UiBadge
                intent="neutral"
                size="sm"
                :label="t('admin.event.tabs.venuesTab.kind.' + venue.kind)"
              />
            </div>
            <p v-if="venue.address" class="mt-1 text-sm text-text-muted">{{ venue.address }}</p>
            <UiButton
              v-if="venue.map_url"
              variant="link"
              size="sm"
              :href="venue.map_url"
              icon="globe"
            >
              {{ t('admin.event.tabs.venuesTab.map') }}
            </UiButton>
          </div>

          <div v-if="props.canManage" class="flex shrink-0 flex-wrap gap-1">
            <UiButton
              variant="secondary"
              size="sm"
              icon="plus"
              @click="openRoomCreate(venue)"
            >
              {{ t('admin.event.tabs.venuesTab.addRoom') }}
            </UiButton>
            <UiButton
              variant="ghost"
              size="sm"
              icon="edit"
              icon-only
              :label="t('common.actions.edit')"
              @click="openVenueEdit(venue)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="trash"
              icon-only
              :label="t('admin.event.tabs.venuesTab.removeVenue.action')"
              @click="doomedVenue = venue"
            />
          </div>
        </header>

        <p v-if="venue.rooms.length === 0" class="p-4 text-sm text-text-subtle">
          {{ t('admin.event.tabs.venuesTab.noRooms') }}
        </p>

        <ul v-else class="divide-y divide-border">
          <li
            v-for="room in venue.rooms"
            :key="room.id"
            class="flex flex-wrap items-start gap-x-4 gap-y-2 p-4"
          >
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <p class="min-w-0 truncate font-medium text-text">{{ tr(room.name) }}</p>
                <code class="font-mono text-xs text-text-muted">{{ room.code }}</code>
                <UiBadge
                  v-if="room.is_virtual"
                  intent="info"
                  size="sm"
                  :label="t('admin.event.tabs.venuesTab.virtualBadge')"
                />
                <UiBadge
                  v-if="room.has_streaming"
                  intent="neutral"
                  size="sm"
                  icon="broadcast"
                  :label="t('admin.event.tabs.venuesTab.streamingBadge')"
                />
              </div>

              <p class="mt-1 text-sm text-text-muted">
                {{
                  room.capacity
                    ? t('admin.event.tabs.venuesTab.capacity', room.capacity)
                    : t('admin.event.tabs.venuesTab.capacityNone')
                }}
                ·
                {{ t('admin.event.tabs.venuesTab.sessions', room.session_count) }}
              </p>

              <!-- LAQUELLE DES DEUX RÈGLES S'APPLIQUE À CETTE SALLE. Sans cette
                   phrase, `is_virtual` n'est qu'une case parmi d'autres. -->
              <p class="mt-1 text-xs text-text-subtle">
                {{
                  room.is_virtual
                    ? t('admin.event.tabs.venuesTab.virtualExplain')
                    : t('admin.event.tabs.venuesTab.physicalExplain')
                }}
              </p>

              <ul v-if="room.equipment.length > 0" class="mt-2 flex flex-wrap gap-1.5">
                <li v-for="item in room.equipment" :key="item">
                  <UiChip :label="item" fixed />
                </li>
              </ul>
            </div>

            <div v-if="props.canManage" class="flex shrink-0 gap-1">
              <UiButton
                variant="ghost"
                size="sm"
                icon="edit"
                icon-only
                :label="t('common.actions.edit')"
                @click="openRoomEdit(room)"
              />
              <UiButton
                variant="ghost"
                size="sm"
                icon="trash"
                icon-only
                :label="t('admin.event.tabs.venuesTab.removeRoom.action')"
                @click="doomedRoom = room"
              />
            </div>
          </li>
        </ul>
      </article>
    </div>

    <!-- FORMULAIRE D'UN LIEU -------------------------------------------------->
    <UiDrawer
      :open="venueDraft !== null"
      :title="
        venueDraft?.id
          ? t('admin.event.tabs.venuesTab.venueForm.editTitle')
          : t('admin.event.tabs.venuesTab.venueForm.createTitle')
      "
      @update:open="(next: boolean) => { if (!next) venueDraft = null }"
    >
      <form v-if="venueDraft" class="space-y-5" novalidate @submit.prevent="emit('saveVenue', { ...venueDraft })">
        <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

        <AdminEventsI18nField
          :model-value="venueDraft.name"
          :label="t('admin.event.tabs.venuesTab.venueForm.nameField')"
          required
          @update:model-value="(next) => (venueDraft!.name = next ?? { fr: '' })"
        />

        <UiSelect
          :model-value="venueDraft.kind"
          :label="t('admin.event.tabs.venuesTab.venueForm.kindField')"
          :options="kindOptions"
          hide-optional
          @update:model-value="(next: string) => (venueDraft!.kind = next as VenueKind)"
        />

        <UiInput
          :model-value="venueDraft.address ?? ''"
          :label="t('admin.event.tabs.venuesTab.venueForm.addressField')"
          :maxlength="240"
          @update:model-value="(next: string) => (venueDraft!.address = next || null)"
        />

        <UiInput
          :model-value="venueDraft.map_url ?? ''"
          type="url"
          :label="t('admin.event.tabs.venuesTab.venueForm.mapUrlField')"
          @update:model-value="(next: string) => (venueDraft!.map_url = next || null)"
        />

        <div class="flex flex-wrap gap-3 border-t border-border pt-4">
          <UiButton type="submit" :loading="props.busy">{{ t('common.actions.save') }}</UiButton>
          <UiButton variant="ghost" :disabled="props.busy" @click="venueDraft = null">
            {{ t('admin.event.tabs.confirm.cancel') }}
          </UiButton>
        </div>
      </form>
    </UiDrawer>

    <!-- FORMULAIRE D'UNE SALLE ------------------------------------------------>
    <UiDrawer
      :open="roomDraft !== null"
      :title="
        roomDraft?.id
          ? t('admin.event.tabs.venuesTab.roomForm.editTitle')
          : t('admin.event.tabs.venuesTab.roomForm.createTitle')
      "
      @update:open="(next: boolean) => { if (!next) roomDraft = null }"
    >
      <form v-if="roomDraft" class="space-y-5" novalidate @submit.prevent="submitRoom">
        <UiAlert v-if="props.error" intent="danger" live :message="props.error" />

        <AdminEventsI18nField
          :model-value="roomDraft.name"
          :label="t('admin.event.tabs.venuesTab.roomForm.nameField')"
          required
          @update:model-value="(next) => (roomDraft!.name = next ?? { fr: '' })"
        />

        <div class="grid gap-4 sm:grid-cols-2">
          <UiInput
            :model-value="roomDraft.code"
            :label="t('admin.event.tabs.venuesTab.roomForm.codeField')"
            :hint="t('admin.event.tabs.venuesTab.roomForm.codeHint')"
            required
            :maxlength="32"
            @update:model-value="(next: string) => (roomDraft!.code = next)"
          />
          <UiInput
            :model-value="roomDraft.capacity ?? ''"
            type="number"
            :min="1"
            :label="t('admin.event.tabs.venuesTab.roomForm.capacityField')"
            @update:model-value="(next: string) => (roomDraft!.capacity = next ? Number(next) : null)"
          />
        </div>

        <UiSwitch
          :model-value="roomDraft.is_virtual"
          :label="t('admin.event.tabs.venuesTab.roomForm.isVirtual')"
          :hint="t('admin.event.tabs.venuesTab.roomForm.isVirtualHint')"
          @update:model-value="(next: boolean) => (roomDraft!.is_virtual = next)"
        />

        <UiSwitch
          :model-value="roomDraft.has_streaming"
          :label="t('admin.event.tabs.venuesTab.roomForm.hasStreaming')"
          @update:model-value="(next: boolean) => (roomDraft!.has_streaming = next)"
        />

        <UiTextarea
          :model-value="equipmentText"
          :label="t('admin.event.tabs.venuesTab.roomForm.equipmentField')"
          :hint="t('admin.event.tabs.venuesTab.roomForm.equipmentHint')"
          :rows="4"
          @update:model-value="(next: string) => (equipmentText = next)"
        />

        <div class="flex flex-wrap gap-3 border-t border-border pt-4">
          <UiButton type="submit" :loading="props.busy">{{ t('common.actions.save') }}</UiButton>
          <UiButton variant="ghost" :disabled="props.busy" @click="roomDraft = null">
            {{ t('admin.event.tabs.confirm.cancel') }}
          </UiButton>
        </div>
      </form>
    </UiDrawer>

    <!-- SUPPRESSIONS ---------------------------------------------------------->
    <UiModal
      :open="doomedVenue !== null"
      :title="t('admin.event.tabs.venuesTab.removeVenue.title')"
      :description="t('admin.event.tabs.venuesTab.removeVenue.description')"
      @update:open="(next: boolean) => { if (!next) doomedVenue = null }"
    >
      <UiAlert
        v-if="venueSessionCount > 0"
        intent="warning"
        :message="t('admin.event.tabs.venuesTab.removeVenue.withSessions', venueSessionCount)"
      />

      <template #footer>
        <UiButton variant="ghost" @click="doomedVenue = null">
          {{ t('admin.event.tabs.confirm.cancel') }}
        </UiButton>
        <UiButton
          variant="danger"
          :loading="props.busy"
          @click="() => { if (doomedVenue) emit('removeVenue', doomedVenue.id); doomedVenue = null }"
        >
          {{ t('admin.event.tabs.venuesTab.removeVenue.action') }}
        </UiButton>
      </template>
    </UiModal>

    <UiModal
      :open="doomedRoom !== null"
      :title="t('admin.event.tabs.venuesTab.removeRoom.title')"
      :description="t('admin.event.tabs.venuesTab.removeRoom.description')"
      @update:open="(next: boolean) => { if (!next) doomedRoom = null }"
    >
      <UiAlert
        v-if="doomedRoom && doomedRoom.session_count > 0"
        intent="warning"
        :message="t('admin.event.tabs.venuesTab.removeRoom.withSessions', doomedRoom.session_count)"
      />

      <template #footer>
        <UiButton variant="ghost" @click="doomedRoom = null">
          {{ t('admin.event.tabs.confirm.cancel') }}
        </UiButton>
        <UiButton
          variant="danger"
          :loading="props.busy"
          @click="() => { if (doomedRoom) emit('removeRoom', doomedRoom.id); doomedRoom = null }"
        >
          {{ t('admin.event.tabs.venuesTab.removeRoom.action') }}
        </UiButton>
      </template>
    </UiModal>
  </section>
</template>
