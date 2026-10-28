import { defineStore } from 'pinia'
import type { Uuid } from '~/types/shared'

/**
 * La fenêtre de rédaction d'un courriel (28/10), ouverte depuis n'importe quel
 * écran du back-office : l'icône enveloppe de la barre du haut, « Nouveau
 * courriel » de l'historique, un brouillon de la liste. Une seule fenêtre à la
 * fois ; `version` avance à chaque fermeture pour que l'historique se relise.
 */
export const useMailComposerStore = defineStore('mail-composer', () => {
  const isOpen = ref(false)
  /** Le brouillon ouvert ; nul : un nouveau, créé à l'ouverture. */
  const mailingId = ref<Uuid | null>(null)
  const version = ref(0)

  function open(id: Uuid | null = null): void {
    mailingId.value = id
    isOpen.value = true
  }

  function close(): void {
    isOpen.value = false
    mailingId.value = null
    version.value += 1
  }

  return { isOpen, mailingId, version, open, close }
})
