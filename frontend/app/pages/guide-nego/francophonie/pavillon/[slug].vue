<script setup lang="ts">
import type { PublicSessionSpeaker } from '~/types/programme/session'
import type { IssueInscriptionPavillon } from '~/composables/guide-nego/useGnInscriptionsPavillon'
import { dayKeyInZone } from '~/utils/datetime'
import {
  consentementRequis,
  etatDeLActivite,
  formulaireDUnGeste,
  minutesDeRediffusion,
  PREFIXE_FILE_INSCRIPTION_PAVILLON,
  reponsesPreremplies,
} from '~/utils/guide-nego/pavillon'

/**
 * Écran 10 · 1d — la fiche d'une activité du Pavillon. Le détail lu tient lieu de source ;
 * à défaut, la ligne de l'édition gardée ouvre la fiche sans réseau. Pas d'« Ajouter à mon
 * agenda » (FR-012).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const { intlLocale, dayLong, time, zoneLabel, zoneOffsetShort } = useDateTime()
const { tr } = useI18nText()
const route = useRoute()
const connexion = useGnConnexion()
const compte = useGnSession()
const edition = useGnEdition()
const pavillon = useGnPavillon()
const inscriptions = useGnInscriptionsPavillon()
const pays = useGnPays()

const k = (cle: string, params: Record<string, unknown> = {}) => t(`guide-nego.pavillon-activite.${cle}`, params)

const slug = String(route.params.slug ?? '')
const lecture = useGnActivitePavillon(slug)
const retour = '/guide-nego/francophonie?section=pavillon'

const maintenant = shallowRef(new Date())
let horloge: ReturnType<typeof setInterval> | undefined
const tentee = ref(false)
const relecture = ref(false)

async function relire(): Promise<void> {
  relecture.value = true
  try {
    await Promise.all([lecture.rafraichir(), pavillon.rafraichir()])
  } finally {
    relecture.value = false
    tentee.value = true
  }
}

onMounted(async () => {
  horloge = setInterval(() => (maintenant.value = new Date()), 60_000)
  void relire()
  await compte.assurer()
  inscriptions.assurer()
  if (compte.connectee.value) void pays.assurer()
})
onBeforeUnmount(() => {
  clearInterval(horloge)
  ficheOuverte.value = null
})

const enLigne = computed(() => connexion.etat.value.enLigne)
const attente = computed(() => !lecture.pret.value && !tentee.value)
const activite = computed(() => lecture.activite.value)
const detail = computed(() => lecture.detail.value)
const fuseau = computed(() => activite.value?.timezone ?? pavillon.fuseau.value ?? 'UTC')
const ville = computed(() => edition.edition.value?.city ?? undefined)

const etat = computed(() => (activite.value ? etatDeLActivite(activite.value, maintenant.value) : null))
const marque = computed(() => (activite.value ? inscriptions.marque(activite.value.id, maintenant.value) : null))
const annulee = computed(() => etat.value === 'annulee')
const titre = computed(() => (activite.value ? tr(activite.value.title) : k('titre')))

const majuscule = (texte: string) => texte.charAt(0).toLocaleUpperCase() + texte.slice(1)

const heure = computed(() => {
  const a = activite.value
  if (!a) return null
  const f = fuseau.value
  const debut = time(a.starts_at, f)
  const fin = time(a.ends_at, f)
  const jour = dayLong(a.starts_at, f)
  const valeur =
    dayKeyInZone(a.starts_at, f) === dayKeyInZone(a.ends_at, f)
      ? k('heure', { jour, debut, fin })
      : k('heure-deux-jours', { jour, debut, jourFin: dayLong(a.ends_at, f), fin })
  return {
    valeur: majuscule(valeur),
    precision: k('fuseau', { zone: zoneLabel(f, ville.value), decalage: zoneOffsetShort(f, a.starts_at) }),
  }
})

const lieu = computed(() => {
  const l = pavillon.lieu.value
  return l ? { nom: tr(l.name), adresse: l.address, plan: l.map_url } : null
})

const nomDOrganisation = (o: { name?: string; acronym?: string | null }) =>
  o.name && o.acronym ? `${o.name} (${o.acronym})` : (o.name ?? o.acronym ?? '')

const organisations = computed(() => {
  const liste = [...(detail.value?.organizations ?? [])].sort((a, b) => a.sort_order - b.sort_order)
  const noms = (garder: (role: string) => boolean) => liste.filter((o) => garder(o.role)).map(nomDOrganisation).filter(Boolean)
  const organisateurs = noms((r) => r === 'lead' || r === 'co_organizer')
  const a = activite.value
  if (!organisateurs.length && a?.organization_name) {
    organisateurs.push(nomDOrganisation({ name: a.organization_name, acronym: a.organization_acronym }))
  }
  return { organisateurs, partenaires: noms((r) => r === 'partner' || r === 'sponsor') }
})

const langues = computed(() => {
  const codes = activite.value?.language_codes ?? []
  if (!codes.length) return null
  const noms = new Intl.DisplayNames([intlLocale.value], { type: 'language' })
  return codes.map((c) => majuscule(noms.of(c) ?? c)).join(', ')
})

const rediffusion = computed(() => {
  const a = activite.value
  if (!a?.replay_url) return null
  const minutes = minutesDeRediffusion(a)
  return { url: a.replay_url, libelle: minutes ? k('revoir-duree', { minutes }) : k('revoir') }
})

const resume = computed(() => (activite.value?.summary ? tr(activite.value.summary) : null))

const intervenants = computed(() =>
  [...(detail.value?.speakers ?? [])]
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((s: PublicSessionSpeaker) => {
      const [prenom = '', ...reste] = s.display_name.trim().split(/\s+/)
      const fonction = [s.job_title_snapshot, s.organization_snapshot].filter(Boolean).join(', ')
      return {
        id: s.id,
        nom: s.display_name,
        prenom,
        famille: reste.join(' '),
        role: s.role === 'speaker' ? null : k(`role.${s.role}`),
        fonction,
      }
    }),
)

const libelleAttente = (position: number | null) =>
  position ? t('gn-ligne-activite.attente-position', { position }) : undefined

// --- S'inscrire (FR-006, FR-006a, FR-007) -----------------------------------

const id = computed(() => activite.value?.id ?? null)
// Le layout se tait sur un refus que cette fiche redit en place.
const ficheOuverte = useState<string | null>('gn-fiche-pavillon-ouverte', () => null)
watch(id, (valeur) => (ficheOuverte.value = valeur), { immediate: true })

const bouton = computed(() => (id.value ? inscriptions.bouton(id.value, maintenant.value) : null))
const aEnvoyer = computed(() => (id.value ? inscriptions.aEnvoyer(id.value) : false))
const lecturePendante = computed(() => compte.connectee.value && !inscriptions.pret.value)
const champs = computed(() => lecture.formulaire.value?.fields ?? [])
const paysIso2 = computed(() => pays.iso2DuPays(compte.compte.value.paysId))
const envoi = ref(false)
const refus = ref<string | null>(null)
const feuilleCompte = ref(false)
const feuilleFormulaire = ref(false)
const erreursDuFormulaire = ref<Record<string, string>>({})
const refusDuFormulaire = ref<string | null>(null)

const pasEncore = computed(() => {
  const ouvre = bouton.value?.ouvreLe
  if (!ouvre) return null
  const f = fuseau.value
  return k('inscription.pas-encore', { jour: dayLong(ouvre, f), heure: time(ouvre, f), zone: zoneLabel(f, ville.value) })
})

const message = ref<{ texte: string; action?: string; agir?: () => void; rang: number } | null>(null)
const annoncer = (texte: string, action?: string, agir?: () => void) =>
  (message.value = { texte, action, agir, rang: (message.value?.rang ?? 0) + 1 })

function dire(issue: IssueInscriptionPavillon): void {
  if (issue.issue === 'refusee') {
    message.value = null
    refus.value = issue.message ?? k('inscription.refusee')
  } else if (issue.issue === 'inscrite') annoncer(k('inscription.faite'))
  else if (issue.issue === 'liste-attente') annoncer(libelleAttente(issue.position) ?? t('gn-marque-etat.liste-attente'))
  else if (issue.issue === 'en-attente') annoncer(k('inscription.en-file'))
}

async function envoyer(geste: () => Promise<IssueInscriptionPavillon>): Promise<IssueInscriptionPavillon> {
  envoi.value = true
  refus.value = null
  try {
    return await geste()
  } finally {
    envoi.value = false
  }
}

async function inscrire(): Promise<void> {
  const seance = id.value
  if (!seance) return
  if (!compte.connectee.value) {
    feuilleCompte.value = true
    return
  }
  if (compte.compte.value.paysId && !paysIso2.value) await pays.assurer().catch(() => undefined)
  if (formulaireDUnGeste(champs.value, paysIso2.value)) {
    dire(await envoyer(() => inscriptions.inscrire(seance, reponsesPreremplies(champs.value, paysIso2.value))))
    return
  }
  erreursDuFormulaire.value = {}
  refusDuFormulaire.value = null
  feuilleFormulaire.value = true
}

async function envoyerLeFormulaire(reponses: Record<string, unknown>, consentement: boolean): Promise<void> {
  const seance = id.value
  if (!seance) return
  erreursDuFormulaire.value = {}
  refusDuFormulaire.value = null
  const issue = await envoyer(() => inscriptions.inscrire(seance, reponses, consentement))
  // Un refus qui nomme un champ se corrige dans la feuille, qui reste ouverte.
  if (issue.issue === 'refusee' && issue.champ) {
    const texte = issue.message ?? k('inscription.refusee')
    refus.value = null
    if (champs.value.some((c) => c.code === issue.champ)) erreursDuFormulaire.value = { [issue.champ]: texte }
    else refusDuFormulaire.value = texte
    return
  }
  feuilleFormulaire.value = false
  dire(issue)
}

async function desinscrire(): Promise<void> {
  const seance = id.value
  if (!seance) return
  const quitteLAttente = bouton.value?.libelle === 'liste-attente'
  const reponses = { ...(inscriptions.inscription(seance)?.answers ?? {}) }
  const issue = await envoyer(() => inscriptions.annuler(seance))
  if (issue.issue === 'refusee') return dire(issue)
  const cle = issue.issue === 'en-attente' ? 'en-file' : quitteLAttente ? 'attente-quittee' : 'faite'
  annoncer(k(`desinscription.${cle}`), k('desinscription.annuler'), () => {
    message.value = null
    void envoyer(() => inscriptions.inscrire(seance, reponses, consentementRequis(champs.value, reponses))).then(dire)
  })
}

// Un geste reparti de la file et refusé se dit ici, en mots clairs.
watch(inscriptions.refus, (avis) => {
  if (avis?.message && id.value && avis.cle === `${PREFIXE_FILE_INSCRIPTION_PAVILLON}${id.value}`) refus.value = avis.message
})

useHead({ title: titre })
</script>

<template>
  <GnEcran
    :titre="titre"
    :surtitre="activite ? k('origine') : undefined"
    :retour="retour"
    :onglets="false"
    :ce-qui-se-lit="k('hors-connexion')"
  >
    <template #connexion>
      <GnLigneConnexion :en-ligne="enLigne" :lu-a="lecture.luA.value ?? pavillon.luA.value" />
    </template>

    <GnChargement v-if="attente && !activite" forme="squelette" :lignes="6" :libelle="k('chargement')" class="gn-activite__attente" />

    <GnEtatVide
      v-else-if="!activite && pavillon.sansEdition.value"
      picto="franco"
      :titre="k('sans-edition.titre')"
      :texte="k('sans-edition.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <GnEtatVide
      v-else-if="!activite && lecture.introuvable.value"
      picto="franco"
      :titre="k('inconnue.titre')"
      :texte="k('inconnue.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <GnEtatVide
      v-else-if="!activite && !enLigne"
      picto="wifi-off"
      :titre="k('jamais-lue.titre')"
      :texte="k('jamais-lue.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <GnEtatErreur
      v-else-if="!activite"
      :titre="k('erreur.titre')"
      :texte="k('erreur.texte')"
      :sortie="relecture ? undefined : k('erreur.reessayer')"
      @sortie="relire()"
    />

    <div v-else class="gn-activite" :class="{ 'gn-activite--annulee': annulee }">
      <div class="gn-activite__marques">
        <GnMarqueEtat v-if="etat" :etat="etat" />
        <GnMarqueEtat v-if="marque?.nom === 'inscrite'" etat="inscrite" />
        <GnMarqueEtat v-else-if="marque?.nom === 'liste-attente'" etat="liste-attente" :libelle="libelleAttente(marque.position)" />
        <GnMarqueEtat v-else-if="marque?.nom === 'complet'" etat="complet" />
        <span v-else-if="marque?.nom === 'sans-inscription'" class="gn-activite__libre">
          {{ t('gn-ligne-activite.sans-inscription') }}
        </span>
      </div>

      <GnBlocLieu v-if="lieu" :nom="lieu.nom" :adresse="lieu.adresse" :plan="lieu.plan" />

      <dl class="gn-activite__lignes">
        <div v-if="heure" class="gn-activite__ligne">
          <dt>{{ k('cle.heure') }}</dt>
          <dd class="gn-activite__double">
            <span class="gn-activite__valeur">{{ heure.valeur }}</span>
            <span class="gn-activite__precision">{{ heure.precision }}</span>
          </dd>
        </div>
        <div v-if="activite.room_name" class="gn-activite__ligne">
          <dt>{{ k('cle.salle') }}</dt>
          <dd class="gn-activite__valeur">{{ tr(activite.room_name) }}</dd>
        </div>
        <div v-if="organisations.organisateurs.length" class="gn-activite__ligne">
          <dt>{{ k('cle.organisateurs') }}</dt>
          <dd class="gn-activite__double">
            <span v-for="nom in organisations.organisateurs" :key="nom" class="gn-activite__valeur">{{ nom }}</span>
          </dd>
        </div>
        <div v-if="organisations.partenaires.length" class="gn-activite__ligne">
          <dt>{{ k('cle.partenaires') }}</dt>
          <dd class="gn-activite__double">
            <span v-for="nom in organisations.partenaires" :key="nom" class="gn-activite__valeur">{{ nom }}</span>
          </dd>
        </div>
        <div v-if="langues" class="gn-activite__ligne">
          <dt>{{ k('cle.langue') }}</dt>
          <dd class="gn-activite__valeur">{{ langues }}</dd>
        </div>
        <div v-if="rediffusion" class="gn-activite__ligne">
          <dt>{{ k('cle.rediffusion') }}</dt>
          <dd>
            <a :href="rediffusion.url" class="gn-activite__revoir" target="_blank" rel="noopener">
              <GnPicto nom="play" :taille="20" />
              {{ rediffusion.libelle }}
            </a>
          </dd>
        </div>
      </dl>

      <p v-if="resume" class="gn-activite__resume">{{ resume }}</p>

      <template v-if="intervenants.length">
        <GnEnteteGroupe :titre="k('intervenants')" :compteur="intervenants.length" />
        <ul class="gn-activite__intervenants">
          <li v-for="i in intervenants" :key="i.id" class="gn-activite__intervenant">
            <GnAvatar :prenom="i.prenom" :nom="i.famille" />
            <span class="gn-activite__personne">
              <span class="gn-activite__nom">{{ i.nom }}</span>
              <span v-if="i.role" class="gn-activite__fonction">{{ i.role }}</span>
              <span v-if="i.fonction" class="gn-activite__fonction">{{ i.fonction }}</span>
            </span>
          </li>
        </ul>
      </template>

      <div v-if="bouton || aEnvoyer || refus" class="gn-activite__actions">
        <template v-if="bouton">
          <GnBouton v-if="bouton.libelle === 'inscrire'" :chargement="envoi || lecturePendante" @clic="inscrire">
            {{ k('inscription.inscrire') }}
          </GnBouton>
          <GnBouton
            v-else-if="bouton.libelle === 'inscrite'"
            variante="secondaire"
            picto="check"
            actif
            :desactive="!bouton.geste"
            :chargement="envoi"
            @clic="desinscrire"
          >
            {{ t('gn-marque-etat.inscrite') }}
          </GnBouton>
          <template v-else-if="bouton.libelle === 'rejoindre-attente'">
            <GnMarqueEtat etat="complet" />
            <GnBouton variante="secondaire" picto="clock" :chargement="envoi || lecturePendante" @clic="inscrire">
              {{ k('inscription.rejoindre-attente') }}
            </GnBouton>
          </template>
          <template v-else-if="bouton.libelle === 'liste-attente'">
            <GnMarqueEtat etat="liste-attente" :libelle="libelleAttente(bouton.position)" />
            <GnBouton v-if="bouton.geste" variante="secondaire" :chargement="envoi" @clic="desinscrire">
              {{ k('inscription.quitter-attente') }}
            </GnBouton>
          </template>
          <GnMarqueEtat v-else-if="bouton.libelle === 'complet'" etat="complet" :libelle="k('inscription.complet')" />
          <p v-else class="gn-activite__closes">
            <GnPicto :nom="bouton.libelle === 'pas-encore' ? 'clock' : 'lock'" :taille="18" />
            {{ bouton.libelle === 'pas-encore' ? pasEncore : k('inscription.closes') }}
          </p>
        </template>

        <GnMarqueEtat v-if="aEnvoyer" etat="a-envoyer" :libelle="k('inscription.a-envoyer')" />
        <p v-if="refus" class="gn-activite__refus" role="alert">
          <GnPicto nom="warn" :taille="20" />
          {{ refus }}
        </p>
      </div>
    </div>

    <GnFormulaireInscription
      v-model="feuilleFormulaire"
      :champs="champs"
      :titre="titre"
      :pays-iso2="paysIso2"
      :envoi="envoi"
      :erreurs-serveur="erreursDuFormulaire"
      :refus="refusDuFormulaire"
      :attente="bouton?.libelle === 'rejoindre-attente'"
      @envoyer="envoyerLeFormulaire"
    />

    <GnFeuilleBasse
      v-model="feuilleCompte"
      :titre="k('inscription.compte.titre')"
      :sous-titre="k('inscription.compte.texte')"
      :fermeture="k('inscription.fermer')"
    >
      <div class="gn-activite__sorties">
        <GnBouton vers="/guide-nego/connexion" picto="user">{{ t('gn-verrou.sortie.connexion') }}</GnBouton>
        <GnBouton variante="secondaire" vers="/guide-nego/compte">{{ t('gn-verrou.sortie.compte') }}</GnBouton>
      </div>
    </GnFeuilleBasse>

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
[data-app="guide-nego"] .gn-ecran:has(.gn-activite--annulee) .gn-entete__titre {
  text-decoration: line-through;
  text-decoration-thickness: 2px;
}

[data-app="guide-nego"] .gn-activite__attente {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-ecran__contenu:has(> .gn-activite) {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-activite {
  flex: 1;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-activite:not(:has(.gn-activite__actions)) {
  padding-bottom: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-activite__actions {
  position: sticky;
  bottom: 0;
  margin-top: auto;
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding: var(--gn-espace-16) var(--gn-marge-ecran) calc(var(--gn-espace-16) + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  border-top: var(--gn-filet-1) solid var(--gn-filet-doux);
  background: var(--gn-barre-fond);
}

[data-app="guide-nego"] .gn-activite__closes,
[data-app="guide-nego"] .gn-activite__refus {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-activite__closes {
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-activite__refus {
  color: var(--gn-danger);
}

[data-app="guide-nego"] .gn-activite__closes .gn-picto,
[data-app="guide-nego"] .gn-activite__refus .gn-picto {
  flex: none;
  margin-block-start: 1px;
}

[data-app="guide-nego"] .gn-activite__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-activite__marques {
  display: flex;
  flex-wrap: wrap;
  column-gap: var(--gn-espace-16);
  row-gap: var(--gn-espace-4);
  padding: var(--gn-espace-12) 0 10px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet-doux);
}

[data-app="guide-nego"] .gn-activite__marques .gn-marque-etat {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-activite__libre,
[data-app="guide-nego"] .gn-activite__precision,
[data-app="guide-nego"] .gn-activite__fonction {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-activite__lignes {
  padding: var(--gn-espace-4) var(--gn-espace-16);
  border-radius: var(--gn-rayon-24);
  background: var(--gn-fond-2);
}

[data-app="guide-nego"] .gn-activite__ligne {
  display: flex;
  justify-content: space-between;
  gap: var(--gn-espace-12);
  padding-block: var(--gn-espace-12);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet-doux);
}

[data-app="guide-nego"] .gn-activite__ligne:last-child {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-activite__ligne dt {
  flex: none;
  padding-top: 2px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-activite__ligne dd {
  min-width: 0;
  display: flex;
  justify-content: flex-end;
  text-align: end;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-activite__ligne dd.gn-activite__double {
  flex-direction: column;
  align-items: flex-end;
}

[data-app="guide-nego"] .gn-activite__valeur {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}

/* La ligne reste basse, la cible déborde en hauteur jusqu'à 48 px. */
[data-app="guide-nego"] .gn-activite__revoir {
  min-height: var(--gn-cible);
  margin-block: -13px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-etat-rediffusion);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-activite__revoir .gn-picto {
  flex: none;
}

[data-app="guide-nego"] .gn-activite__resume {
  padding-top: var(--gn-espace-12);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  white-space: pre-line;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-activite__intervenant {
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-12);
  padding-block: var(--gn-ligne-air);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet-doux);
}

[data-app="guide-nego"] .gn-activite__personne {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-activite__nom {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
}
</style>
