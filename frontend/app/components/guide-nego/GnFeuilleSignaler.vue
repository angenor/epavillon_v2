<script setup lang="ts">
import type { OptionDeFeuille } from '~/components/guide-nego/GnFeuilleBasse.vue'
import type { ReportPayload } from '~/types/negotiation-reports'
import type { OfficialSession } from '~/types/negotiation-sessions'
import {
  DESSIN_DU_MOTIF,
  MOTIFS_DE_CHANGEMENT,
  PRECISION_MAX,
  corpsDuChangement,
  type MotifDeChangement,
} from '~/utils/guide-nego/signalements'

/**
 * « Signaler un changement » (maquette 09, 1a et 1b) : le motif, puis la précision, dans
 * la même feuille. Trois gestes — ouvrir, choisir, « Envoyer » — et aucune boîte de
 * confirmation : la phrase du bouclier en tient lieu (FR-005). Tout est facultatif après
 * le motif, donc « Envoyer » est actif d'emblée.
 */
const props = withDefaults(
  defineProps<{
    session: OfficialSession
    fuseau: string
    ville?: string | null
    envoi?: boolean
  }>(),
  { ville: null, envoi: false },
)

const emit = defineEmits<{ envoyer: [Omit<ReportPayload, 'client_ref' | 'edition'>] }>()

const ouverte = defineModel<boolean>({ required: true })

const { t, locale } = useI18n()
const { time, zoneLabel } = useDateTime()
const k = (cle: string, params: Record<string, unknown> = {}) => t(`gn-feuille-signaler.${cle}`, params)

const motif = ref<MotifDeChangement | null>(null)
const heure = ref('')
const salle = ref('')
const precision = ref('')

watch(ouverte, (voulue) => {
  if (!voulue) return
  motif.value = null
  heure.value = ''
  salle.value = ''
  precision.value = ''
})

const rappel = computed(() => {
  const s = props.session
  const titre = (locale.value === 'fr' ? s.title_fr : null) ?? s.title_en
  const debut = time(s.start_at, props.fuseau)
  return s.venue ? k('rappel', { titre, heure: debut, salle: s.venue }) : k('rappel-sans-salle', { titre, heure: debut })
})

const options = computed<OptionDeFeuille[]>(() =>
  motif.value
    ? []
    : MOTIFS_DE_CHANGEMENT.map((m) => ({
        valeur: m,
        libelle: k(`motif.${m}`),
        picto: DESSIN_DU_MOTIF[m].picto,
        teinte: DESSIN_DU_MOTIF[m].teinte ?? undefined,
        suite: true,
      })),
)

function choisir(option: OptionDeFeuille): void {
  motif.value = option.valeur as MotifDeChangement
}

function envoyer(): void {
  if (!motif.value || props.envoi) return
  emit(
    'envoyer',
    corpsDuChangement(
      motif.value,
      { heure: heure.value, salle: salle.value, precision: precision.value },
      props.session,
      props.fuseau,
    ),
  )
}
</script>

<template>
  <GnFeuilleBasse
    v-model="ouverte"
    :titre="motif ? k(`motif.${motif}`) : k('titre')"
    :sous-titre="rappel"
    :options="options"
    @choisir="choisir"
  >
    <form v-if="motif" class="gn-feuille-signaler" @submit.prevent="envoyer">
      <GnChamp
        v-if="motif === 'time'"
        v-model="heure"
        type="time"
        :libelle="k('heure')"
        :aide="k('heure-aide', { zone: zoneLabel(fuseau, ville ?? undefined) })"
      />
      <GnChamp
        v-else-if="motif === 'venue'"
        v-model="salle"
        :libelle="k('salle')"
        :aide="k('facultatif')"
        autocomplete="off"
      />
      <GnZoneTexte
        v-model="precision"
        :libelle="k('precision')"
        :aide="k('facultatif')"
        :maximum="PRECISION_MAX"
      />
      <p class="gn-feuille-signaler__bouclier">
        <GnPicto nom="shield-check" :taille="20" />
        <span>{{ k('bouclier') }}</span>
      </p>
      <GnBouton type="submit" picto="send" :chargement="envoi">{{ k('envoyer') }}</GnBouton>
    </form>
  </GnFeuilleBasse>
</template>

<style>
[data-app="guide-nego"] .gn-feuille-signaler {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-feuille-signaler__bouclier {
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-feuille-signaler__bouclier .gn-picto {
  flex: none;
}
</style>
