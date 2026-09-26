<script setup lang="ts">
import type { Notification } from '~/types/engagement'
import { couleurDEtat } from '~/utils/guide-nego/etats'
import { dessinDeNotification } from '~/utils/guide-nego/signalements'

/**
 * Une ligne du centre (maquette 02 · 10). Non lue : texte en 600 et carré jaune — jamais
 * un fond jaune. Le toucher est rendu à l'écran, qui marque lu avant d'ouvrir la fiche.
 */
const props = defineProps<{ notification: Notification; fuseau: string; derniere?: boolean }>()
const emit = defineEmits<{ ouvrir: [notification: Notification] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time } = useDateTime()

const nonLue = computed(() => !props.notification.read_at)
const dessin = computed(() => dessinDeNotification(props.notification))
const titre = computed(() => tr(props.notification.title) || tr(props.notification.body))
const detail = computed(() => (props.notification.title ? tr(props.notification.body) : '') || t('gn-ligne-notification.origine'))
const heure = computed(() => time(props.notification.created_at, props.fuseau))
const Lien = resolveComponent('NuxtLink')

function ouvrir(evenement: Event): void {
  evenement.preventDefault()
  emit('ouvrir', props.notification)
}
</script>

<template>
  <component
    :is="notification.link_path ? Lien : 'button'"
    :to="notification.link_path ?? undefined"
    :type="notification.link_path ? undefined : 'button'"
    class="gn-ligne-notif"
    :class="{ 'gn-ligne-notif--non-lue': nonLue, 'gn-ligne-notif--derniere': derniere }"
    @click="ouvrir"
  >
    <GnPicto
      :nom="dessin.picto"
      :taille="20"
      class="gn-ligne-notif__picto"
      :style="dessin.teinte ? { color: couleurDEtat(dessin.teinte) } : undefined"
    />
    <span class="gn-ligne-notif__corps">
      <span class="gn-ligne-notif__titre">{{ titre }}</span>
      <span class="gn-ligne-notif__meta">{{ t('gn-ligne-notification.meta', { detail, heure }) }}</span>
    </span>
    <span v-if="nonLue" class="gn-ligne-notif__non-lu" role="img" :aria-label="t('gn-ligne-notification.non-lu')" />
  </component>
</template>

<style>
[data-app="guide-nego"] .gn-ligne-notif {
  width: 100%;
  min-height: var(--gn-cible);
  padding: 10px 0;
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  border: none;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  background: none;
  color: var(--gn-texte);
  font: inherit;
  text-align: start;
  text-decoration: none;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-ligne-notif--derniere {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-ligne-notif:active {
  background: var(--gn-presse);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-marge-ecran);
  width: calc(100% + 2 * var(--gn-marge-ecran));
}

[data-app="guide-nego"] .gn-ligne-notif__picto {
  flex: none;
  margin-top: 2px;
  color: var(--gn-picto);
}

[data-app="guide-nego"] .gn-ligne-notif__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-ligne-notif__titre {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-regulier);
}

[data-app="guide-nego"] .gn-ligne-notif--non-lue .gn-ligne-notif__titre {
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-ligne-notif__meta {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-ligne-notif__non-lu {
  flex: none;
  width: var(--gn-carre-non-lu);
  height: var(--gn-carre-non-lu);
  margin-top: 6px;
  background: var(--gn-attention-aplat);
}
</style>
