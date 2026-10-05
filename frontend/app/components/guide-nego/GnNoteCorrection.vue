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

/** En marge de la page du PDF, replier la note ferme le panneau qui la porte. */
const emit = defineEmits<{ basculer: [id: string, ouverte: boolean] }>()

const { t } = useI18n()
const { date } = useDateTime()

// La date d'une pose, pas un créneau : le fuseau du téléphone, sans heure.
const fuseau = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'

const ouvertes = ref(new Set<string>(props.depliee ? props.notes.map((n) => n.id) : []))

function basculer(id: string): void {
  const suivantes = new Set(ouvertes.value)
  if (!suivantes.delete(id)) suivantes.add(id)
  ouvertes.value = suivantes
  emit('basculer', id, suivantes.has(id))
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
        <GnPicto :nom="ouvertes.has(note.id) ? 'chev-up' : 'chev-down'" :taille="20" class="gn-note-correction__chevron" />
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
/* Le trait tient dans la marge de l'écran : le texte du passage ne bouge pas d'un pixel. */
[data-app="guide-nego"] .gn-note-correction {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  margin-inline-start: calc(-1 * var(--gn-marge-ecran));
  padding-inline-start: var(--gn-marge-ecran);
}

/* Le trait du repère « maintenant » de la maquette : 2 px aux bouts arrondis, à mi-marge. */
[data-app="guide-nego"] .gn-note-correction::before {
  content: '';
  position: absolute;
  inset-block: 0;
  inset-inline-start: calc(var(--gn-marge-ecran) / 2);
  width: 2px;
  border-radius: 1px;
  background: var(--gn-etat-depasse);
}

[data-app="guide-nego"] .gn-note-correction__ligne {
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  inline-size: 100%;
  min-height: var(--gn-bouton-rond);
  padding: 0;
  border: none;
  background: none;
  color: var(--gn-etat-depasse);
  font: inherit;
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  font-weight: var(--gn-graisse-extra-gras);
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
  color: var(--gn-picto-secondaire);
}

[data-app="guide-nego"] .gn-note-correction__boite {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding: var(--gn-espace-12) var(--gn-espace-16);
  border-radius: var(--gn-rayon-16);
  background: var(--gn-fond-2);
}

/* La boîte ne suit pas la taille de lecture : c'est une glose, pas le texte du guide. */
[data-app="guide-nego"] .gn-note-correction__texte {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-lecture);
  white-space: pre-line;
  overflow-wrap: break-word;
}

[data-app="guide-nego"] .gn-note-correction__signature {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-note-correction__bouclier {
  flex: none;
  color: var(--gn-etat-valide);
}
</style>
