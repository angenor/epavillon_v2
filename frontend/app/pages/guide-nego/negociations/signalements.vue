<script setup lang="ts">
import type { NomDEtat } from '~/utils/guide-nego/etats'
import { dayKeyInZone } from '~/utils/datetime'
import { couleurDEtat } from '~/utils/guide-nego/etats'
import { DESSIN_DU_MOTIF, texteDeLEtat, type EtatSignalement, type LigneSignalement } from '~/utils/guide-nego/signalements'

/**
 * Maquette 09, 2a — « Mes signalements », le plus récent d'abord. Ce qui attend dans la
 * file s'y voit aussitôt, « Envoyé — partira au retour du réseau » (FR-006).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const { dayLong, time } = useDateTime()
const route = useRoute()
const connexion = useGnConnexion()
const compte = useGnSession()
const edition = useGnEdition()
const lecture = useGnSessions()
const signalements = useGnSignalements()
const thematiques = useGnThematiques()

const k = (cle: string, params: Record<string, unknown> = {}, n?: number) =>
  n === undefined ? t(`guide-nego.signalements.${cle}`, params) : t(`guide-nego.signalements.${cle}`, params, n)

const retour = computed(() => {
  const depuis = typeof route.query.depuis === 'string' ? route.query.depuis : ''
  if (depuis === 'sessions') return '/guide-nego/negociations'
  if (/^[0-9a-f-]{36}$/i.test(depuis)) return `/guide-nego/negociations/${depuis}`
  return '/guide-nego/ressources/reglages'
})

const tentee = ref(false)
async function relire(): Promise<void> {
  await signalements.rafraichir()
  tentee.value = true
}

onMounted(async () => {
  await compte.assurer()
  if (!compte.connectee.value) return
  signalements.assurer()
  void lecture.rafraichir()
  void thematiques.assurerLeVocabulaire()
  await relire()
})

const enLigne = computed(() => connexion.etat.value.enLigne)
const fuseau = computed(() => lecture.fuseau.value ?? edition.edition.value?.timezone ?? 'UTC')
const lignes = computed(() => signalements.lignes.value)
const attente = computed(() => compte.connectee.value && !signalements.connu.value && !tentee.value && lignes.value.length === 0)

const sousTitre = computed(() => {
  const libelle = edition.edition.value?.libelle
  if (!compte.connectee.value || !libelle || !signalements.connu.value) return undefined
  const n = lignes.value.length
  return k('sous-titre', { edition: libelle, count: n }, n)
})

function moment(iso: string): string {
  const heure = time(iso, fuseau.value)
  const aujourdhui = dayKeyInZone(new Date(), fuseau.value)
  return dayKeyInZone(iso, fuseau.value) === aujourdhui ? heure : k('le', { jour: dayLong(iso, fuseau.value), heure })
}

const MARQUE: Record<EtatSignalement, NomDEtat> = {
  'en-attente': 'envoye',
  envoye: 'envoye',
  valide: 'valide',
  'non-retenu': 'non-retenu',
}

function vue(ligne: LigneSignalement) {
  const s = ligne.signalement
  const etat = texteDeLEtat(ligne)
  const dessin = DESSIN_DU_MOTIF[s.reason]
  const session = s.session
  const titre = session ? ((locale.value === 'fr' ? session.title_fr : null) ?? session.title_en) : null

  let proposition: string | null = null
  if (s.reason === 'time' && s.proposed_start) proposition = k('propose-heure', { heure: time(s.proposed_start, fuseau.value) })
  if (s.reason === 'venue' && s.proposed_venue) proposition = k('propose-salle', { salle: s.proposed_venue })

  let rappel: string
  if (session && titre) {
    const quand = moment(session.start_at)
    rappel = session.venue ? k('rappel', { titre, quand, salle: session.venue }) : k('rappel-sans-salle', { titre, quand })
  } else {
    const lieu = [s.proposed_venue, s.proposed_start ? time(s.proposed_start, fuseau.value) : null].filter(Boolean).join(' — ')
    const thematique = s.theme ? thematiques.nomDe(s.theme) : null
    rappel = [lieu, thematique].filter(Boolean).join(' · ')
  }

  return {
    cle: s.client_ref,
    picto: dessin.picto,
    couleur: dessin.teinte ? couleurDEtat(dessin.teinte) : undefined,
    motif: k(`motif.${s.reason}`),
    principal: s.what ?? s.detail,
    proposition,
    rappel,
    marque: MARQUE[etat.cle],
    etat: k(`etat.${etat.cle}`, { moment: moment(etat.a) }),
    refus: etat.motif ? k('refus', { motif: k(`refus-motif.${etat.motif}`) }) : null,
    precisionDuRefus: etat.precision,
    vers: session
      ? `/guide-nego/negociations/${session.id}?depuis=signalements`
      : s.network_meeting_id
        ? `/guide-nego/negociations/reseau/${s.network_meeting_id}`
        : null,
  }
}

const vues = computed(() => lignes.value.map(vue))
const Lien = resolveComponent('NuxtLink')

useHead({ title: k('titre') })
</script>

<template>
  <GnEcran :titre="k('titre')" :sous-titre="sousTitre" :retour="retour" :onglets="false">
    <template #connexion>
      <GnLigneConnexion :en-ligne="enLigne" :lu-a="signalements.luA.value" />
    </template>

    <GnEtatVide
      v-if="compte.pret.value && !compte.connectee.value"
      picto="user"
      :titre="k('sans-compte.titre')"
      :texte="k('sans-compte.texte')"
      :sortie="k('sans-compte.connexion')"
      sortie-vers="/guide-nego/connexion"
    />

    <GnChargement v-else-if="!compte.pret.value || attente" forme="squelette" :lignes="5" :libelle="k('chargement')" />

    <GnEtatVide
      v-else-if="!signalements.connu.value && !lignes.length && !enLigne"
      picto="wifi-off"
      :titre="k('jamais-lu.titre')"
      :texte="k('jamais-lu.texte')"
    />

    <GnEtatErreur
      v-else-if="!signalements.connu.value && !lignes.length"
      :titre="k('erreur.titre')"
      :texte="k('erreur.texte')"
      :sortie="k('erreur.reessayer')"
      @sortie="relire()"
    />

    <GnEtatVide v-else-if="!lignes.length" picto="flag" :titre="k('vide.titre')" :texte="k('vide.texte')" />

    <template v-else>
      <ul class="gn-signalements">
        <li v-for="v in vues" :key="v.cle">
          <component :is="v.vers ? Lien : 'div'" :to="v.vers ?? undefined" class="gn-signalements__ligne">
            <GnPicto :nom="v.picto" :taille="20" class="gn-signalements__motif-picto" :style="v.couleur ? { color: v.couleur } : undefined" />
            <div class="gn-signalements__corps">
              <span class="gn-signalements__motif">{{ v.motif }}</span>
              <span v-if="v.principal" class="gn-signalements__principal">{{ v.principal }}</span>
              <span v-if="v.proposition" class="gn-signalements__detail">{{ v.proposition }}</span>
              <span v-if="v.rappel" class="gn-signalements__detail">{{ v.rappel }}</span>
              <GnMarqueEtat :etat="v.marque" :libelle="v.etat" class="gn-signalements__etat" />
              <span v-if="v.refus" class="gn-signalements__refus">{{ v.refus }}</span>
              <span v-if="v.precisionDuRefus" class="gn-signalements__detail">{{ v.precisionDuRefus }}</span>
            </div>
            <GnPicto v-if="v.vers" nom="chevron" :taille="24" class="gn-signalements__chevron" />
          </component>
        </li>
      </ul>

      <p class="gn-signalements__pied">
        <GnPicto nom="shield-check" :taille="20" />
        <span>{{ k('pied') }}</span>
      </p>
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-signalements__ligne {
  min-height: var(--gn-cible);
  padding-block: var(--gn-espace-12);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  color: var(--gn-texte);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-signalements__motif-picto,
[data-app="guide-nego"] .gn-signalements__chevron {
  flex: none;
  margin-top: 2px;
  color: var(--gn-picto);
}

[data-app="guide-nego"] .gn-signalements__chevron {
  align-self: center;
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-signalements__corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-signalements__motif,
[data-app="guide-nego"] .gn-signalements__detail,
[data-app="guide-nego"] .gn-signalements__refus {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-signalements__motif {
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-signalements__principal {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-signalements__etat {
  margin-top: 4px;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-signalements__refus {
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-signalements__pied {
  padding-block: var(--gn-espace-16);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-signalements__pied .gn-picto {
  flex: none;
}
</style>
