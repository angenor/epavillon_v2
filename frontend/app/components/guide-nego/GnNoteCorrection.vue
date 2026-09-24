<script setup lang="ts">
import type { CorrectionNote } from '~/types/negotiation-documents'

/**
 * La note de correction d'un expert — maquette 04 · 11a et 11b. Un filet rouge dans la
 * marge borde le passage visé, puis une ligne repliée par note. Le passage arrive par
 * l'emplacement, **tel quel** : la note se pose par-dessus le texte, jamais dedans.
 */
const props = withDefaults(
  defineProps<{
    notes: Pick<CorrectionNote, 'id' | 'body' | 'author_name' | 'posted_at'>[]
    /** Toutes dépliées à l'ouverture — la planche montre les deux états. */
    depliee?: boolean
  }>(),
  { depliee: false },
)

const { t } = useI18n()
const { date } = useDateTime()

// La date d'une pose, pas un créneau : le fuseau du téléphone, sans heure.
const fuseau = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'

const ouvertes = ref(new Set<string>(props.depliee ? props.notes.map((n) => n.id) : []))

function basculer(id: string): void {
  const suivantes = new Set(ouvertes.value)
  if (!suivantes.delete(id)) suivantes.add(id)
  ouvertes.value = suivantes
}

const signature = (note: Pick<CorrectionNote, 'author_name' | 'posted_at'>) =>
  t('gn-note-correction.signature', { nom: note.author_name, date: date(note.posted_at, fuseau) })
</script>

<template>
  <div class="gn-note-correction">
    <slot />
    <div v-for="note in notes" :key="note.id">
      <button
        type="button"
        class="gn-note-correction__ligne"
        :aria-expanded="ouvertes.has(note.id)"
        :aria-controls="`gn-note-${note.id}`"
        @click.stop="basculer(note.id)"
      >
        <GnPicto nom="warn" :taille="20" class="gn-note-correction__triangle" />
        <span class="gn-note-correction__libelle">{{ t('gn-note-correction.ligne') }}</span>
        <GnPicto :nom="ouvertes.has(note.id) ? 'chev-up' : 'chev-down'" :taille="24" class="gn-note-correction__chevron" />
      </button>
      <div v-if="ouvertes.has(note.id)" :id="`gn-note-${note.id}`" class="gn-note-correction__boite">
        <p class="gn-note-correction__texte">{{ note.body }}</p>
        <p class="gn-note-correction__signature">
          <GnPicto nom="shield-check" :taille="16" class="gn-note-correction__bouclier" />
          {{ signature(note) }}
        </p>
      </div>
    </div>
  </div>
</template>

<style>
/* Le filet tient dans la marge de l'écran : le texte du passage ne bouge pas d'un pixel. */
[data-app="guide-nego"] .gn-note-correction {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  margin-inline-start: calc(-1 * var(--gn-marge-ecran));
  padding-inline-start: calc(var(--gn-marge-ecran) - var(--gn-filet-3));
  border-inline-start: var(--gn-filet-3) solid var(--gn-etat-depasse);
}

[data-app="guide-nego"] .gn-note-correction__ligne {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  inline-size: 100%;
  min-height: var(--gn-cible);
  padding: 0;
  border: none;
  background: none;
  color: var(--gn-etat-depasse);
  font: inherit;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-align: start;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-note-correction__triangle,
[data-app="guide-nego"] .gn-note-correction__chevron {
  flex: none;
}

[data-app="guide-nego"] .gn-note-correction__libelle {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-note-correction__chevron {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-note-correction__boite {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
  padding: var(--gn-espace-8) var(--gn-espace-12);
  border: var(--gn-filet-1) solid var(--gn-filet);
}

/* La boîte ne suit pas la taille de lecture : c'est une glose, pas le texte du guide. */
[data-app="guide-nego"] .gn-note-correction__texte {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  color: var(--gn-texte);
  white-space: pre-line;
  overflow-wrap: break-word;
}

[data-app="guide-nego"] .gn-note-correction__signature {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-note-correction__bouclier {
  flex: none;
  color: var(--gn-etat-valide);
}
</style>
