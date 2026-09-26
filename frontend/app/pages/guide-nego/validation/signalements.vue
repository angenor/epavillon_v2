<script setup lang="ts">
import type { OptionDeCercle } from '~/components/guide-nego/GnCercle.vue'
import type { RejectReason, ReportQueueItem } from '~/types/negotiation-reports'
import type { NomDEtat } from '~/utils/guide-nego/etats'
import type { IssueDeDecision } from '~/composables/guide-nego/useGnValidation'
import { dayKeyInZone } from '~/utils/datetime'
import { etatDuTraite, peutSeRetirer, PRECISION_MAX, type EtatDuTraite } from '~/utils/guide-nego/signalements'

/**
 * 11 · 1a et 1f — la file des signalements, depuis le téléphone de l'administration.
 * « Valider » agit en un geste, « Annuler » six secondes ; rien n'est public avant la
 * publication, trente secondes plus tard. **En ligne seulement** (FR-013) : une décision
 * prise sur une vue périmée de la source contredirait ADR-010.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const { dayLong, time } = useDateTime()
const { momentLisible } = useGnMomentLecture()
const k = (cle: string, params: Record<string, unknown> = {}, n?: number) =>
  n === undefined ? t(`guide-nego.validation.${cle}`, params) : t(`guide-nego.validation.${cle}`, params, n)

const connexion = useGnConnexion()
const compte = useGnSession()
const acces = useGnAcces()
const edition = useGnEdition()
const thematiques = useGnThematiques()
const validation = useGnValidation()
const { etat } = validation

const accesLu = ref(false)
let horloge: ReturnType<typeof setInterval> | undefined

async function charger(): Promise<void> {
  if (connexion.etat.value.enLigne && validation.peutValider.value) await validation.charger()
}

onMounted(async () => {
  await compte.assurer()
  if (compte.connectee.value) await acces.rafraichir()
  accesLu.value = true
  void thematiques.assurerLeVocabulaire()
  await charger()
  // Une validation paraît trente secondes plus tard : la file le relit d'elle-même.
  horloge = setInterval(() => void charger(), 30_000)
})
onBeforeUnmount(() => clearInterval(horloge))

const enLigne = computed(() => connexion.etat.value.enLigne)
watch(enLigne, (oui) => {
  if (oui) void charger()
})

const peutValider = computed(() => compte.connectee.value && validation.peutValider.value)
const file = computed(() => etat.value.file)
const attente = computed(() => !accesLu.value || (peutValider.value && !file.value && etat.value.enCours))
const fuseau = computed(() => edition.edition.value?.timezone ?? 'UTC')

const sousTitre = computed(() => {
  const f = file.value
  if (!f) return undefined
  const n = f.pending.length
  const m = f.decided_today.length
  return k('sous-titre', { attente: k('a-traiter', { count: n }, n), traites: k('traites', { count: m }, m) })
})

// --- Les cartes -------------------------------------------------------------

function plage(debut: string, fin: string | null): string {
  const d = time(debut, fuseau.value)
  return fin ? `${d}–${time(fin, fuseau.value)}` : d
}

function quand(iso: string): string {
  const heure = time(iso, fuseau.value)
  return dayKeyInZone(iso, fuseau.value) === dayKeyInZone(new Date(), fuseau.value)
    ? heure
    : k('reunion-quand', { jour: dayLong(iso, fuseau.value), heure })
}

const ETAT_DU_TRAITE: Record<EtatDuTraite, NomDEtat> = {
  'non-retenu': 'non-retenu',
  'en-publication': 'valide',
  valide: 'valide',
  retire: 'terminee',
}

function carte(item: ReportQueueItem) {
  const auteur = item.author.country ? k('auteur-pays', { nom: item.author.name, pays: item.author.country }) : item.author.name
  const motif = k(`motif.${item.reason}`)

  let proposition: string | null = null
  if (item.reason === 'time' && item.proposed_start) proposition = k('propose-heure', { heure: time(item.proposed_start, fuseau.value) })
  if (item.reason === 'venue' && item.proposed_venue) proposition = k('propose-salle', { salle: item.proposed_venue })

  let rappel: string
  const s = item.session
  if (s) {
    const titre = (locale.value === 'fr' ? s.title_fr : null) ?? s.title_en
    rappel = s.venue
      ? k('session-salle', { titre, quand: quand(s.start_at), salle: s.venue })
      : k('session', { titre, quand: quand(s.start_at) })
  } else {
    const jour = item.day ? dayLong(`${item.day}T12:00:00Z`, 'UTC') : null
    const heure = item.proposed_start ? time(item.proposed_start, fuseau.value) : null
    const thematique = item.theme ? thematiques.nomDe(item.theme) : null
    rappel = [[item.proposed_venue, [jour, heure].filter(Boolean).join(', ')].filter(Boolean).join(' — '), thematique]
      .filter(Boolean)
      .join(' · ')
  }

  const src = item.source_now
  const source = src
    ? {
        titre: src.read_at ? k('source.titre', { moment: momentLisible(src.read_at) }) : k('source.titre-sans-heure'),
        etat: (src.status === 'cancelled' ? 'annulee' : 'prevue') as NomDEtat,
        libelle:
          src.status === 'cancelled'
            ? k('source.annulee')
            : src.venue
              ? k('source.prevue-salle', { plage: plage(src.start_at, src.end_at), salle: src.venue })
              : k('source.prevue', { plage: plage(src.start_at, src.end_at) }),
      }
    : null

  const traite = item.status === 'submitted' ? null : etatDuTraite(item)
  const instantDuTraite = traite === 'retire' ? item.withdrawn_at : item.decided_at

  return {
    item,
    entete: k('entete', { auteur, motif, heure: time(item.submitted_at, fuseau.value) }),
    principal: item.what ?? item.detail ?? proposition,
    proposition: item.what || item.detail ? proposition : null,
    rappel,
    source,
    horsSource: !s,
    marque: traite ? ETAT_DU_TRAITE[traite] : null,
    etat: traite && instantDuTraite ? k(`etat.${traite}`, { heure: time(instantDuTraite, fuseau.value) }) : null,
    par: item.decided_by ? k('par', { nom: item.decided_by }) : null,
    refus: item.reject_reason ? k('refus-motif', { motif: k(`refus.${item.reject_reason}`) }) : null,
    retirable: peutSeRetirer(item),
  }
}

const aTraiter = computed(() => (file.value?.pending ?? []).map(carte))
const traites = computed(() => (file.value?.decided_today ?? []).map(carte))

// --- Les gestes -------------------------------------------------------------

const enAction = ref<string | null>(null)
const alerte = ref<string | null>(null)
const message = ref<{ texte: string; action?: string; agir?: () => void; rang: number } | null>(null)
const annoncer = (texte: string, action?: string, agir?: () => void) =>
  (message.value = { texte, action, agir, rang: (message.value?.rang ?? 0) + 1 })

// L'alerte reste en tête de la file ; le message le redit là où se trouve le pouce.
function dire(issue: Extract<IssueDeDecision, { ok: false }>): void {
  alerte.value = issue.raison === 'hors-connexion' ? k('hors-connexion-geste') : (issue.message ?? k('panne'))
  annoncer(alerte.value)
}

async function decider(id: string, geste: () => Promise<IssueDeDecision>): Promise<boolean> {
  alerte.value = null
  enAction.value = id
  try {
    const issue = await geste()
    if (!issue.ok) dire(issue)
    return issue.ok
  } finally {
    enAction.value = null
    void acces.rafraichir()
  }
}

async function valider(id: string): Promise<void> {
  if (!(await decider(id, () => validation.valider(id)))) return
  annoncer(k('valide'), k('annuler'), () => void annulerLaValidation(id))
}

async function annulerLaValidation(id: string): Promise<void> {
  message.value = null
  if (await decider(id, () => validation.annuler(id))) annoncer(k('annulee'))
}

// --- « Ne pas retenir » (1f) ------------------------------------------------

const MOTIFS_DE_REFUS: readonly RejectReason[] = ['source_maintains', 'already_known', 'not_precise']
const feuilleRefus = ref(false)
const aRefuser = ref<ReportQueueItem | null>(null)
const motifDeRefus = ref<string | null>(null)
const precisionDuRefus = ref('')
const optionsDeRefus = computed<OptionDeCercle[]>(() =>
  MOTIFS_DE_REFUS.map((m) => ({ valeur: m, libelle: k(`refus.${m}`) })),
)

function ouvrirLeRefus(item: ReportQueueItem): void {
  aRefuser.value = item
  motifDeRefus.value = null
  precisionDuRefus.value = ''
  feuilleRefus.value = true
}

async function refuser(): Promise<void> {
  const item = aRefuser.value
  const reason = motifDeRefus.value as RejectReason | null
  if (!item || !reason) return
  const detail = precisionDuRefus.value.trim()
  const ok = await decider(item.id, () => validation.refuser(item.id, detail ? { reason, detail } : { reason }))
  feuilleRefus.value = false
  if (ok) annoncer(k('non-retenu'))
}

// --- « Retirer » ------------------------------------------------------------

const confirmationRetrait = ref(false)
const aRetirer = ref<string | null>(null)

function demanderLeRetrait(id: string): void {
  aRetirer.value = id
  confirmationRetrait.value = true
}

async function retirer(): Promise<void> {
  const id = aRetirer.value
  if (!id) return
  if (await decider(id, () => validation.retirer(id))) annoncer(k('retire'))
}

useHead({ title: k('titre') })
</script>

<template>
  <GnEcran
    :titre="k('titre')"
    :sous-titre="sousTitre"
    retour="/guide-nego/validation"
    :onglets="false"
    :ce-qui-se-lit="k('hors-connexion-bandeau')"
  >
    <template #connexion>
      <GnLigneConnexion :en-ligne="enLigne" :lu-a="etat.luA" />
    </template>
    <template v-if="peutValider" #action>
      <span class="gn-validation__role">
        <GnPicto nom="shield-check" :taille="20" />
        {{ k('role') }}
      </span>
    </template>

    <GnChargement v-if="attente" forme="squelette" :lignes="5" :libelle="k('chargement')" />

    <GnEtatVide
      v-else-if="!peutValider"
      picto="lock"
      :titre="k('refuse.titre')"
      :texte="k('refuse.texte')"
      :sortie="k('refuse.sortie')"
      sortie-vers="/guide-nego/ressources"
    />

    <GnEtatVide
      v-else-if="!file && !enLigne"
      picto="wifi-off"
      :titre="k('jamais-lue.titre')"
      :texte="k('jamais-lue.texte')"
    />

    <GnEtatErreur
      v-else-if="!file"
      :titre="k('erreur.titre')"
      :texte="etat.erreur ?? k('erreur.texte')"
      :sortie="k('erreur.reessayer')"
      @sortie="charger()"
    />

    <div v-else class="gn-validation">
      <p v-if="!enLigne" class="gn-validation__alerte" role="status">
        <GnPicto nom="wifi-off" :taille="20" />
        <span>{{ k('hors-connexion') }}</span>
      </p>
      <p v-if="alerte" class="gn-validation__alerte" role="alert">
        <GnPicto nom="warn" :taille="20" />
        <span>{{ alerte }}</span>
      </p>

      <GnEtatVide v-if="!aTraiter.length" picto="flag" :titre="k('vide.titre')" :texte="k('vide.texte')" />

      <ul v-else class="gn-validation__cartes" :aria-label="k('a-traiter-titre')">
        <li v-for="c in aTraiter" :key="c.item.id" class="gn-validation__carte">
          <p class="gn-validation__entete">{{ c.entete }}</p>
          <p v-if="c.principal" class="gn-validation__principal">{{ c.principal }}</p>
          <p v-if="c.proposition" class="gn-validation__detail">{{ c.proposition }}</p>
          <p v-if="c.rappel" class="gn-validation__detail">{{ c.rappel }}</p>
          <blockquote v-if="c.source" class="gn-validation__source">
            <p class="gn-validation__source-titre">{{ c.source.titre }}</p>
            <GnMarqueEtat :etat="c.source.etat" :libelle="c.source.libelle" />
          </blockquote>
          <p v-else-if="c.horsSource" class="gn-validation__detail">{{ k('source.hors-source') }}</p>
          <div class="gn-validation__gestes">
            <GnBouton
              variante="secondaire"
              largeur="demie"
              :desactive="enAction !== null"
              @clic="ouvrirLeRefus(c.item)"
            >
              {{ k('ne-pas-retenir') }}
            </GnBouton>
            <GnBouton
              largeur="demie"
              :chargement="enAction === c.item.id"
              :desactive="enAction !== null && enAction !== c.item.id"
              @clic="valider(c.item.id)"
            >
              {{ k('valider') }}
            </GnBouton>
          </div>
        </li>
      </ul>

      <template v-if="traites.length">
        <GnEnteteGroupe :titre="k('traites-titre')" :compteur="traites.length" />
        <ul class="gn-validation__cartes">
          <li v-for="c in traites" :key="c.item.id" class="gn-validation__carte">
            <p class="gn-validation__entete">{{ c.entete }}</p>
            <p v-if="c.principal" class="gn-validation__principal">{{ c.principal }}</p>
            <p v-if="c.rappel" class="gn-validation__detail">{{ c.rappel }}</p>
            <p v-if="c.refus" class="gn-validation__refus">{{ c.refus }}</p>
            <p v-if="c.item.reject_detail" class="gn-validation__detail">{{ c.item.reject_detail }}</p>
            <GnMarqueEtat v-if="c.marque && c.etat" :etat="c.marque" :libelle="c.etat" />
            <p v-if="c.par" class="gn-validation__detail">{{ c.par }}</p>
            <GnBouton
              v-if="c.retirable"
              variante="secondaire"
              picto="close"
              :chargement="enAction === c.item.id"
              :desactive="enAction !== null && enAction !== c.item.id"
              @clic="demanderLeRetrait(c.item.id)"
            >
              {{ k('retirer') }}
            </GnBouton>
          </li>
        </ul>
      </template>
    </div>

    <GnFeuilleBasse
      v-model="feuilleRefus"
      :titre="k('feuille.confirmer')"
      :sous-titre="aRefuser ? k('feuille.sous-titre', { nom: aRefuser.author.name }) : undefined"
      :fermeture="k('feuille.revenir')"
    >
      <form class="gn-validation__feuille" @submit.prevent="refuser">
        <GnCercle v-model="motifDeRefus" :options="optionsDeRefus" :libelle="k('feuille.motifs')" />
        <GnZoneTexte
          v-model="precisionDuRefus"
          :libelle="k('feuille.precision')"
          :aide="k('feuille.facultatif')"
          :maximum="PRECISION_MAX"
        />
        <GnBouton
          type="submit"
          :desactive="!motifDeRefus"
          :chargement="aRefuser !== null && enAction === aRefuser.id"
        >
          {{ k('feuille.confirmer') }}
        </GnBouton>
      </form>
    </GnFeuilleBasse>

    <GnConfirmation
      v-model="confirmationRetrait"
      :question="k('confirmer-retrait.question')"
      :phrase="k('confirmer-retrait.phrase')"
      :action="k('confirmer-retrait.action')"
      dangereuse
      @confirmer="retirer"
    />

    <GnMessageEphemere
      v-if="message"
      :key="message.rang"
      :texte="message.texte"
      :action="message.action"
      @agir="message.agir?.()"
      @fini="message = null"
    />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-validation {
  padding-bottom: var(--gn-espace-16);
}

/* Redit ici : on arrive souvent sur la file sans passer par la liste. */
[data-app="guide-nego"] .gn-validation__role {
  flex: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-succes);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-validation__alerte {
  margin-top: var(--gn-espace-12);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-validation__alerte .gn-picto {
  flex: none;
}

[data-app="guide-nego"] .gn-validation__carte {
  padding-block: var(--gn-espace-12);
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: var(--gn-espace-8);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-validation__entete,
[data-app="guide-nego"] .gn-validation__detail {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-validation__principal {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-validation__refus {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

/* La citation de la source : ce qu'elle dit à l'instant, avec son heure de lecture. */
[data-app="guide-nego"] .gn-validation__source {
  padding: var(--gn-espace-8) var(--gn-espace-12);
  display: flex;
  flex-direction: column;
  gap: 2px;
  border-inline-start: var(--gn-filet-3) solid var(--gn-filet-fort);
}

[data-app="guide-nego"] .gn-validation__source-titre {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-validation__gestes {
  display: flex;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-validation__gestes .gn-bouton {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-validation__feuille {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
  padding-bottom: var(--gn-espace-8);
}
</style>
